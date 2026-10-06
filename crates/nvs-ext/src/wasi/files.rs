//! `wasi:filesystem` over an instance's preopens: the roots of its effective set
//! (`rule:security/extension-grants-are-an-intersection`), each a directory descriptor named by its
//! canonical absolute path.
//!
//! **A path a guest writes is resolved one name at a time** under the directory it is relative
//! to, and refused the moment a step leaves that directory's root
//! (`rule:security/path-scope-canonicalise-then-prefix`). Each name is canonicalised by
//! [`nvs_config::capability::resolved`], the one implementation the rule allows, so a link is
//! followed to its target and that target is what the prefix check compares; a link planted under
//! a root does not carry the root's grant out of it. `..` pops a name off a path that is already
//! canonical, so it is the physical parent. A step that is still a link after resolution leads
//! nowhere, and is refused rather than followed: opening it to create a file would create its
//! target, wherever that is. An absolute path is refused, and so is a name with a `\` or a NUL in
//! it, and on Windows one with a `:`.
//!
//! The last name of a path an operation acts on itself — `unlink-file-at`, `remove-directory-at`,
//! `rename-at`, `create-directory-at` and `stat-at` without `symlink-follow` — is not resolved, so
//! removing a link removes the link. A root itself is never removed or renamed.
//!
//! **A `read` root is read-only.** Every call that would write under it — opening for writing,
//! creating or truncating a file, making, removing or renaming an entry — answers `read-only`. A
//! descriptor opened without `write` answers `bad-descriptor` to a write.
//!
//! Decisions ADR 0246 § 7 left to this module, under the priority ordering:
//!
//! - **Links are never made or read.** `symlink-at`, `link-at` and `readlink-at` answer
//!   `not-permitted`: a guest that could make a link could aim it outside its root, and the check
//!   above would refuse it on every use anyway. `set-times` and `set-times-at` answer the same.
//! - **The check is a path check, as every `Core\IO` door's is.** A directory renamed between the
//!   check and the open is the race those doors have too, and is closed for all of them or none.
//! - **Every call runs to its end on the request's core**, as a `Core\IO` call does, and a read
//!   returns at most [`PERMIT`] bytes at once.
//!
//! Cost: a few paths per descriptor and one open file per file descriptor, freed when the guest
//! drops it or the instance goes. A directory listing is read whole when the guest asks for it, and
//! kept until the guest drops its stream.

use std::collections::hash_map::DefaultHasher;
use std::fs::{self, File, Metadata, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use nvs_config::resolve::Disk;
use wasmtime::component::Resource;
use wasmtime_wasi_io::async_trait;
use wasmtime_wasi_io::bytes::Bytes;
use wasmtime_wasi_io::poll::Pollable;
use wasmtime_wasi_io::streams::{
    DynInputStream, DynOutputStream, InputStream, OutputStream, StreamError, StreamResult,
};

use super::bindings::wasi::clocks::wall_clock::Datetime;
use super::bindings::wasi::filesystem::{preopens, types};
use super::{Context, PERMIT};
use crate::grants::Effective;

use types::{
    Advice, DescriptorFlags, DescriptorStat, DescriptorType, DirectoryEntry, ErrorCode,
    MetadataHashValue, NewTimestamp, OpenFlags, PathFlags,
};

/// One root a guest may open: its canonical path, and whether it may write under it.
#[derive(Debug)]
pub(crate) struct Root {
    path: PathBuf,
    writable: bool,
}

/// `reach`'s roots, each `read` root read-only and each `write` root read-write.
pub(super) fn roots(reach: &Effective) -> Vec<Arc<Root>> {
    let read = reach.read.iter().map(|path| (path, false));
    let write = reach.write.iter().map(|path| (path, true));
    read.chain(write)
        .map(|(path, writable)| {
            Arc::new(Root {
                path: path.clone(),
                writable,
            })
        })
        .collect()
}

/// A descriptor a guest holds: a directory or an open file, under the root it was reached from.
#[derive(Debug)]
#[allow(
    unreachable_pub,
    reason = "`bindgen!` re-exports it as `wasi:filesystem/types.descriptor`, which a type only the crate sees cannot be"
)]
pub struct Descriptor {
    root: Arc<Root>,
    path: PathBuf,
    flags: DescriptorFlags,
    file: Option<Arc<File>>,
}

/// A directory's entries, read when the guest asked for them, in name order.
#[derive(Debug)]
#[allow(
    unreachable_pub,
    reason = "`bindgen!` re-exports it as `wasi:filesystem/types.directory-entry-stream`, as `Descriptor`"
)]
pub struct Entries(std::vec::IntoIter<DirectoryEntry>);

/// What a descriptor call answers.
type Answer<T> = wasmtime::Result<Result<T, ErrorCode>>;

/// How the last name of a path is treated.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Last {
    /// Resolved like every other name.
    Follow,
    /// Kept as written, because the operation acts on that entry itself.
    Keep,
}

impl Descriptor {
    /// The directory `path` under `root`.
    fn dir(root: Arc<Root>, path: PathBuf) -> Self {
        let flags = if root.writable {
            DescriptorFlags::READ | DescriptorFlags::MUTATE_DIRECTORY
        } else {
            DescriptorFlags::READ
        };
        Self {
            root,
            path,
            flags,
            file: None,
        }
    }

    /// The open file, or `bad-descriptor` for a directory or a file not opened as `need` asks.
    fn file(&self, need: DescriptorFlags) -> Result<&File, ErrorCode> {
        match &self.file {
            Some(file) if self.flags.contains(need) => Ok(file),
            Some(_) => Err(ErrorCode::BadDescriptor),
            None => Err(ErrorCode::IsDirectory),
        }
    }

    /// `read-only` unless this descriptor's root may be written under.
    fn writable(&self) -> Result<(), ErrorCode> {
        if self.root.writable {
            Ok(())
        } else {
            Err(ErrorCode::ReadOnly)
        }
    }

    fn metadata(&self) -> io::Result<Metadata> {
        match &self.file {
            Some(file) => file.metadata(),
            None => fs::metadata(&self.path),
        }
    }

    /// Where `path`, written relative to this directory, leads.
    fn reach(&self, path: &str, last: Last) -> Result<PathBuf, ErrorCode> {
        if self.file.is_some() {
            return Err(ErrorCode::NotDirectory);
        }
        if path.is_empty() {
            return Err(ErrorCode::NoEntry);
        }
        if path.starts_with('/') {
            return Err(ErrorCode::NotPermitted);
        }
        let mut names: Vec<&str> = path
            .split('/')
            .filter(|name| !name.is_empty() && *name != ".")
            .collect();
        if names.iter().any(|name| refused_name(name)) {
            return Err(ErrorCode::Invalid);
        }
        let kept = match names.last() {
            Some(&name) if last == Last::Keep && name != ".." => names.pop(),
            _ => None,
        };
        let mut at = self.path.clone();
        for name in names {
            if name == ".." {
                at.pop();
            } else {
                at = step(&at.join(name))?;
            }
            if !at.starts_with(&self.root.path) {
                return Err(ErrorCode::NotPermitted);
            }
        }
        if let Some(name) = kept {
            at.push(name);
        }
        Ok(at)
    }

    /// Where `path` leads, for an operation that changes the entry it names: under a root that
    /// may be written, and never the root itself.
    fn reach_entry(&self, path: &str) -> Result<PathBuf, ErrorCode> {
        self.writable()?;
        let at = self.reach(path, Last::Keep)?;
        if at == self.root.path {
            return Err(ErrorCode::NotPermitted);
        }
        Ok(at)
    }

    /// The file or directory `path` names, opened as `how` and `flags` ask.
    fn open(&self, path: &str, how: OpenFlags, flags: DescriptorFlags) -> Result<Self, ErrorCode> {
        let creates = how.intersects(OpenFlags::CREATE | OpenFlags::TRUNCATE);
        let writes = flags.contains(DescriptorFlags::WRITE);
        if creates || writes || flags.contains(DescriptorFlags::MUTATE_DIRECTORY) {
            self.writable()?;
        }
        let at = self.reach(path, Last::Follow)?;
        match fs::metadata(&at) {
            Ok(meta) if meta.is_dir() => {
                if how.contains(OpenFlags::CREATE | OpenFlags::EXCLUSIVE) {
                    return Err(ErrorCode::Exist);
                }
                if writes {
                    return Err(ErrorCode::IsDirectory);
                }
                Ok(Self::dir(Arc::clone(&self.root), at))
            }
            Ok(_) if how.contains(OpenFlags::DIRECTORY) => Err(ErrorCode::NotDirectory),
            Err(err) if how.contains(OpenFlags::DIRECTORY) => Err(code(&err)),
            _ => {
                let file = OpenOptions::new()
                    .read(true)
                    .write(writes || creates)
                    .create(how.contains(OpenFlags::CREATE) && !how.contains(OpenFlags::EXCLUSIVE))
                    .create_new(how.contains(OpenFlags::CREATE | OpenFlags::EXCLUSIVE))
                    .truncate(how.contains(OpenFlags::TRUNCATE))
                    .open(&at)
                    .map_err(|err| code(&err))?;
                let kept = DescriptorFlags::READ | DescriptorFlags::WRITE;
                Ok(Self {
                    root: Arc::clone(&self.root),
                    path: at,
                    flags: (flags & kept) | DescriptorFlags::READ,
                    file: Some(Arc::new(file)),
                })
            }
        }
    }
}

/// A name no path a guest writes may hold.
fn refused_name(name: &str) -> bool {
    name.contains(['\\', '\0']) || (cfg!(windows) && name.contains(':'))
}

/// `path` canonicalised, or its deepest existing ancestor canonicalised with the rest re-appended.
/// A result that is a link is one that leads nowhere, and is refused.
fn step(path: &Path) -> Result<PathBuf, ErrorCode> {
    let at = nvs_config::capability::resolved(path, &Disk).ok_or(ErrorCode::NoEntry)?;
    if fs::symlink_metadata(&at).is_ok_and(|meta| meta.file_type().is_symlink()) {
        return Err(ErrorCode::NotPermitted);
    }
    Ok(at)
}

/// The WASI error an operating-system error is.
fn code(err: &io::Error) -> ErrorCode {
    match err.kind() {
        io::ErrorKind::NotFound => ErrorCode::NoEntry,
        io::ErrorKind::PermissionDenied => ErrorCode::Access,
        io::ErrorKind::AlreadyExists => ErrorCode::Exist,
        io::ErrorKind::NotADirectory => ErrorCode::NotDirectory,
        io::ErrorKind::IsADirectory => ErrorCode::IsDirectory,
        io::ErrorKind::DirectoryNotEmpty => ErrorCode::NotEmpty,
        io::ErrorKind::InvalidInput | io::ErrorKind::InvalidFilename => ErrorCode::Invalid,
        io::ErrorKind::ReadOnlyFilesystem => ErrorCode::ReadOnly,
        io::ErrorKind::StorageFull => ErrorCode::InsufficientSpace,
        io::ErrorKind::Unsupported => ErrorCode::Unsupported,
        _ => ErrorCode::Io,
    }
}

fn kind(kind: fs::FileType) -> DescriptorType {
    if kind.is_dir() {
        DescriptorType::Directory
    } else if kind.is_file() {
        DescriptorType::RegularFile
    } else if kind.is_symlink() {
        DescriptorType::SymbolicLink
    } else {
        DescriptorType::Unknown
    }
}

fn datetime(time: io::Result<SystemTime>) -> Option<Datetime> {
    let since = time.ok()?.duration_since(UNIX_EPOCH).ok()?;
    Some(Datetime {
        seconds: since.as_secs(),
        nanoseconds: since.subsec_nanos(),
    })
}

fn stat(meta: &Metadata) -> DescriptorStat {
    DescriptorStat {
        type_: kind(meta.file_type()),
        link_count: 1,
        size: meta.len(),
        data_access_timestamp: datetime(meta.accessed()),
        data_modification_timestamp: datetime(meta.modified()),
        status_change_timestamp: None,
    }
}

/// A value that is the same for one file while it is unchanged: its path, size and modification
/// time hashed.
fn hash(path: &Path, meta: &Metadata) -> MetadataHashValue {
    let mut lower = DefaultHasher::new();
    path.hash(&mut lower);
    meta.len().hash(&mut lower);
    meta.modified().ok().hash(&mut lower);
    let lower = lower.finish();
    let mut upper = DefaultHasher::new();
    lower.hash(&mut upper);
    path.hash(&mut upper);
    MetadataHashValue {
        lower,
        upper: upper.finish(),
    }
}

#[cfg(unix)]
fn read_at(file: &File, out: &mut [u8], offset: u64) -> io::Result<usize> {
    std::os::unix::fs::FileExt::read_at(file, out, offset)
}

#[cfg(windows)]
fn read_at(file: &File, out: &mut [u8], offset: u64) -> io::Result<usize> {
    std::os::windows::fs::FileExt::seek_read(file, out, offset)
}

#[cfg(unix)]
fn write_at(file: &File, bytes: &[u8], offset: u64) -> io::Result<usize> {
    std::os::unix::fs::FileExt::write_at(file, bytes, offset)
}

#[cfg(windows)]
fn write_at(file: &File, bytes: &[u8], offset: u64) -> io::Result<usize> {
    std::os::windows::fs::FileExt::seek_write(file, bytes, offset)
}

/// Writes all of `bytes` at `offset`.
fn write_all_at(file: &File, mut bytes: &[u8], mut offset: u64) -> io::Result<()> {
    while !bytes.is_empty() {
        match write_at(file, bytes, offset)? {
            0 => return Err(io::ErrorKind::WriteZero.into()),
            written => {
                bytes = &bytes[written..];
                offset += written as u64;
            }
        }
    }
    Ok(())
}

/// At most `len` bytes of `file` from `offset`, and whether the read reached its end.
fn read_some(file: &File, len: u64, offset: u64) -> io::Result<(Vec<u8>, bool)> {
    let len = usize::try_from(len).unwrap_or(usize::MAX).min(PERMIT);
    let mut out = vec![0; len];
    let read = read_at(file, &mut out, offset)?;
    out.truncate(read);
    Ok((out, read < len))
}

/// A file read from `offset` on.
struct Reader {
    file: Arc<File>,
    offset: u64,
}

#[async_trait]
impl Pollable for Reader {
    async fn ready(&mut self) {}
}

impl InputStream for Reader {
    fn read(&mut self, size: usize) -> StreamResult<Bytes> {
        let (bytes, _) = read_some(&self.file, size as u64, self.offset)
            .map_err(|err| StreamError::LastOperationFailed(err.into()))?;
        if bytes.is_empty() && size > 0 {
            return Err(StreamError::Closed);
        }
        self.offset += bytes.len() as u64;
        Ok(Bytes::from(bytes))
    }
}

/// A file written from `offset` on, or at its end each time for `None`.
struct Writer {
    file: Arc<File>,
    offset: Option<u64>,
}

#[async_trait]
impl Pollable for Writer {
    async fn ready(&mut self) {}
}

impl OutputStream for Writer {
    fn write(&mut self, bytes: Bytes) -> StreamResult<()> {
        let failed = |err: io::Error| StreamError::LastOperationFailed(err.into());
        let offset = match self.offset {
            Some(offset) => offset,
            None => self.file.metadata().map_err(failed)?.len(),
        };
        write_all_at(&self.file, &bytes, offset).map_err(failed)?;
        if let Some(offset) = &mut self.offset {
            *offset += bytes.len() as u64;
        }
        Ok(())
    }

    fn flush(&mut self) -> StreamResult<()> {
        Ok(())
    }

    fn check_write(&mut self) -> StreamResult<usize> {
        Ok(PERMIT)
    }
}

impl preopens::Host for Context {
    fn get_directories(&mut self) -> wasmtime::Result<Vec<(Resource<Descriptor>, String)>> {
        let mut out = Vec::new();
        for root in &self.preopens {
            if !fs::metadata(&root.path).is_ok_and(|meta| meta.is_dir()) {
                continue;
            }
            let name = root.path.to_string_lossy().into_owned();
            let dir = Descriptor::dir(Arc::clone(root), root.path.clone());
            out.push((self.table.push(dir)?, name));
        }
        Ok(out)
    }
}

impl types::Host for Context {
    fn filesystem_error_code(
        &mut self,
        err: Resource<wasmtime_wasi_io::streams::Error>,
    ) -> wasmtime::Result<Option<ErrorCode>> {
        Ok(self.table.get(&err)?.downcast_ref::<io::Error>().map(code))
    }
}

impl Context {
    fn descriptor(&self, fd: &Resource<Descriptor>) -> wasmtime::Result<&Descriptor> {
        Ok(self.table.get(fd)?)
    }

    /// `fd`'s open file, as [`Descriptor::file`] checks it, shared so a stream can keep it.
    fn shared(
        &self,
        fd: &Resource<Descriptor>,
        need: DescriptorFlags,
    ) -> wasmtime::Result<Result<Arc<File>, ErrorCode>> {
        let descriptor = self.descriptor(fd)?;
        Ok(descriptor
            .file(need)
            .map(|_| Arc::clone(descriptor.file.as_ref().expect("checked by `file`"))))
    }

    fn push_input(&mut self, stream: Reader) -> Answer<Resource<DynInputStream>> {
        let stream: DynInputStream = Box::new(stream);
        Ok(Ok(self.table.push(stream)?))
    }

    fn push_output(&mut self, stream: Writer) -> Answer<Resource<DynOutputStream>> {
        let stream: DynOutputStream = Box::new(stream);
        Ok(Ok(self.table.push(stream)?))
    }
}

/// `$answer`'s error, returned as the call's `error-code`.
macro_rules! check {
    ($answer:expr) => {
        match $answer {
            Ok(value) => value,
            Err(err) => return Ok(Err(err)),
        }
    };
}

impl types::HostDescriptor for Context {
    fn read_via_stream(
        &mut self,
        fd: Resource<Descriptor>,
        offset: u64,
    ) -> Answer<Resource<DynInputStream>> {
        let file = check!(self.shared(&fd, DescriptorFlags::READ)?);
        self.push_input(Reader { file, offset })
    }

    fn write_via_stream(
        &mut self,
        fd: Resource<Descriptor>,
        offset: u64,
    ) -> Answer<Resource<DynOutputStream>> {
        let file = check!(self.shared(&fd, DescriptorFlags::WRITE)?);
        self.push_output(Writer {
            file,
            offset: Some(offset),
        })
    }

    fn append_via_stream(&mut self, fd: Resource<Descriptor>) -> Answer<Resource<DynOutputStream>> {
        let file = check!(self.shared(&fd, DescriptorFlags::WRITE)?);
        self.push_output(Writer { file, offset: None })
    }

    fn advise(&mut self, fd: Resource<Descriptor>, _: u64, _: u64, _: Advice) -> Answer<()> {
        check!(self.descriptor(&fd)?.file(DescriptorFlags::empty()));
        Ok(Ok(()))
    }

    fn sync_data(&mut self, fd: Resource<Descriptor>) -> Answer<()> {
        let descriptor = self.descriptor(&fd)?;
        Ok(match &descriptor.file {
            Some(file) => file.sync_data().map_err(|err| code(&err)),
            None => Ok(()),
        })
    }

    fn get_flags(&mut self, fd: Resource<Descriptor>) -> Answer<DescriptorFlags> {
        Ok(Ok(self.descriptor(&fd)?.flags))
    }

    fn get_type(&mut self, fd: Resource<Descriptor>) -> Answer<DescriptorType> {
        let descriptor = self.descriptor(&fd)?;
        Ok(descriptor
            .metadata()
            .map(|meta| kind(meta.file_type()))
            .map_err(|err| code(&err)))
    }

    fn set_size(&mut self, fd: Resource<Descriptor>, size: u64) -> Answer<()> {
        let file = check!(self.descriptor(&fd)?.file(DescriptorFlags::WRITE));
        Ok(file.set_len(size).map_err(|err| code(&err)))
    }

    fn set_times(
        &mut self,
        _: Resource<Descriptor>,
        _: NewTimestamp,
        _: NewTimestamp,
    ) -> Answer<()> {
        Ok(Err(ErrorCode::NotPermitted))
    }

    fn read(&mut self, fd: Resource<Descriptor>, len: u64, offset: u64) -> Answer<(Vec<u8>, bool)> {
        let file = check!(self.descriptor(&fd)?.file(DescriptorFlags::READ));
        Ok(read_some(file, len, offset).map_err(|err| code(&err)))
    }

    fn write(&mut self, fd: Resource<Descriptor>, bytes: Vec<u8>, offset: u64) -> Answer<u64> {
        let file = check!(self.descriptor(&fd)?.file(DescriptorFlags::WRITE));
        Ok(write_all_at(file, &bytes, offset)
            .map(|()| bytes.len() as u64)
            .map_err(|err| code(&err)))
    }

    fn read_directory(&mut self, fd: Resource<Descriptor>) -> Answer<Resource<Entries>> {
        let descriptor = self.descriptor(&fd)?;
        if descriptor.file.is_some() {
            return Ok(Err(ErrorCode::NotDirectory));
        }
        let listing = check!(fs::read_dir(&descriptor.path).map_err(|err| code(&err)));
        let mut entries = Vec::new();
        for entry in listing {
            let entry = check!(entry.map_err(|err| code(&err)));
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            let type_ = entry.file_type().map_or(DescriptorType::Unknown, kind);
            entries.push(DirectoryEntry { type_, name });
        }
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(Ok(self.table.push(Entries(entries.into_iter()))?))
    }

    fn sync(&mut self, fd: Resource<Descriptor>) -> Answer<()> {
        let descriptor = self.descriptor(&fd)?;
        Ok(match &descriptor.file {
            Some(file) => file.sync_all().map_err(|err| code(&err)),
            None => Ok(()),
        })
    }

    fn create_directory_at(&mut self, fd: Resource<Descriptor>, path: String) -> Answer<()> {
        let at = check!(self.descriptor(&fd)?.reach_entry(&path));
        Ok(fs::create_dir(at).map_err(|err| code(&err)))
    }

    fn stat(&mut self, fd: Resource<Descriptor>) -> Answer<DescriptorStat> {
        let descriptor = self.descriptor(&fd)?;
        Ok(descriptor
            .metadata()
            .map(|meta| stat(&meta))
            .map_err(|err| code(&err)))
    }

    fn stat_at(
        &mut self,
        fd: Resource<Descriptor>,
        flags: PathFlags,
        path: String,
    ) -> Answer<DescriptorStat> {
        let descriptor = self.descriptor(&fd)?;
        let (last, read): (Last, fn(&Path) -> io::Result<Metadata>) =
            if flags.contains(PathFlags::SYMLINK_FOLLOW) {
                (Last::Follow, |at| fs::metadata(at))
            } else {
                (Last::Keep, |at| fs::symlink_metadata(at))
            };
        let at = check!(descriptor.reach(&path, last));
        Ok(read(&at).map(|meta| stat(&meta)).map_err(|err| code(&err)))
    }

    fn set_times_at(
        &mut self,
        _: Resource<Descriptor>,
        _: PathFlags,
        _: String,
        _: NewTimestamp,
        _: NewTimestamp,
    ) -> Answer<()> {
        Ok(Err(ErrorCode::NotPermitted))
    }

    fn link_at(
        &mut self,
        _: Resource<Descriptor>,
        _: PathFlags,
        _: String,
        _: Resource<Descriptor>,
        _: String,
    ) -> Answer<()> {
        Ok(Err(ErrorCode::NotPermitted))
    }

    fn open_at(
        &mut self,
        fd: Resource<Descriptor>,
        _: PathFlags,
        path: String,
        how: OpenFlags,
        flags: DescriptorFlags,
    ) -> Answer<Resource<Descriptor>> {
        let opened = check!(self.descriptor(&fd)?.open(&path, how, flags));
        Ok(Ok(self.table.push(opened)?))
    }

    fn readlink_at(&mut self, _: Resource<Descriptor>, _: String) -> Answer<String> {
        Ok(Err(ErrorCode::NotPermitted))
    }

    fn remove_directory_at(&mut self, fd: Resource<Descriptor>, path: String) -> Answer<()> {
        let at = check!(self.descriptor(&fd)?.reach_entry(&path));
        Ok(fs::remove_dir(at).map_err(|err| code(&err)))
    }

    fn rename_at(
        &mut self,
        fd: Resource<Descriptor>,
        from: String,
        to_fd: Resource<Descriptor>,
        to: String,
    ) -> Answer<()> {
        let from = check!(self.descriptor(&fd)?.reach_entry(&from));
        let to = check!(self.descriptor(&to_fd)?.reach_entry(&to));
        Ok(fs::rename(from, to).map_err(|err| code(&err)))
    }

    fn symlink_at(&mut self, _: Resource<Descriptor>, _: String, _: String) -> Answer<()> {
        Ok(Err(ErrorCode::NotPermitted))
    }

    fn unlink_file_at(&mut self, fd: Resource<Descriptor>, path: String) -> Answer<()> {
        let at = check!(self.descriptor(&fd)?.reach_entry(&path));
        Ok(fs::remove_file(at).map_err(|err| code(&err)))
    }

    fn is_same_object(
        &mut self,
        a: Resource<Descriptor>,
        b: Resource<Descriptor>,
    ) -> wasmtime::Result<bool> {
        Ok(self.descriptor(&a)?.path == self.descriptor(&b)?.path)
    }

    fn metadata_hash(&mut self, fd: Resource<Descriptor>) -> Answer<MetadataHashValue> {
        let descriptor = self.descriptor(&fd)?;
        Ok(descriptor
            .metadata()
            .map(|meta| hash(&descriptor.path, &meta))
            .map_err(|err| code(&err)))
    }

    fn metadata_hash_at(
        &mut self,
        fd: Resource<Descriptor>,
        flags: PathFlags,
        path: String,
    ) -> Answer<MetadataHashValue> {
        let descriptor = self.descriptor(&fd)?;
        let follow = flags.contains(PathFlags::SYMLINK_FOLLOW);
        let at = check!(descriptor.reach(&path, if follow { Last::Follow } else { Last::Keep }));
        let meta = if follow {
            fs::metadata(&at)
        } else {
            fs::symlink_metadata(&at)
        };
        Ok(meta.map(|meta| hash(&at, &meta)).map_err(|err| code(&err)))
    }

    fn drop(&mut self, fd: Resource<Descriptor>) -> wasmtime::Result<()> {
        self.table.delete(fd)?;
        Ok(())
    }
}

impl types::HostDirectoryEntryStream for Context {
    fn read_directory_entry(
        &mut self,
        entries: Resource<Entries>,
    ) -> Answer<Option<DirectoryEntry>> {
        Ok(Ok(self.table.get_mut(&entries)?.0.next()))
    }

    fn drop(&mut self, entries: Resource<Entries>) -> wasmtime::Result<()> {
        self.table.delete(entries)?;
        Ok(())
    }
}
