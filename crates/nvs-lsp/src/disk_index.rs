//! The symbol index on disk: one file per workspace root, written when the
//! server is idle and read once when it starts.
//!
//! A start that finds a valid file trusts every entry whose file did not
//! change, and analyses only the entries that are stale
//! ([`crate::SymbolIndex::load_from`]). So opening the editor on a project it
//! saw before costs the analysis of the open document's graph and of the
//! files that changed, and not one analysis of every file under the root.
//!
//! # The format
//!
//! Written by hand over the index's own types, with no serialisation crate
//! (`rule:ide/dependencies-are-allowlisted`). Every integer is little-endian.
//!
//! ```text
//! "NVSIDX"   u32 FORMAT   32-byte build stamp   root
//! u32 paths, each a string
//! u32 files, each a path index and one entry
//! u32 reads, each an entry's path index, then u32 n and n path indices
//! 32-byte BLAKE3 of everything before it
//! ```
//!
//! A string is a `u32` length and its UTF-8 bytes. Every path is written once,
//! in the path table, and referred to by its index, because one file's sites
//! all name the same path. An entry carries the file's content hash, its size
//! and modification time when they can be trusted ([`Stamp`]), its surface
//! hash, and every field of the entry.
//!
//! # A file is untrusted input
//!
//! [`decode`] checks every length against the bytes that remain before it
//! allocates, checks every path index against the table, and gives `None` for
//! any error at all, which discards the whole file. A wrong magic, format
//! version, build stamp, root or checksum discards it too. A file in the
//! directory can come from another build, another server or a torn write, and
//! nothing it says changes what is compiled: it only feeds editor answers,
//! and those are replaced as soon as an entry is analysed again.
//!
//! # What it costs
//!
//! One file per workspace root, about the size of the index's entries in
//! memory without the postings. A write is one encode and one rename, in one
//! idle turn. A load is one read, one BLAKE3 pass over the file, and one
//! `metadata` call per indexed file. A file whose size or modification time
//! moved, or has none recorded, is read and hashed as well.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nvs_config::cache::{Digest, content_hash};
use nvs_diagnostics::BytePos;
use rustc_hash::FxHashMap;

use crate::index::{
    Construction, DeclKind, Declaration, Import, Indexed, Occurrence, Parameter, Site, Visibility,
};

/// The first six bytes of every index file.
const MAGIC: &[u8; 6] = b"NVSIDX";

/// The layout's version. A change to what is written raises it, and a file of
/// another version is discarded.
pub(crate) const FORMAT: u32 = 1;

/// The length of a digest on disk.
const DIGEST: usize = 32;

/// How close to the start of an analysis a file's modification time may be
/// and still be trusted. A file written that close to its read may have been
/// written after it, inside one tick of a coarse clock, so its time says
/// nothing about the text the analysis read. Two seconds covers the coarsest
/// file system clock in use.
const SETTLE: Duration = Duration::from_secs(2);

/// A file's size and modification time, recorded only when they identify the
/// text an analysis read. A load trusts an entry whose file still has them,
/// and reads and hashes the file of every other entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Stamp {
    /// The file's length in bytes.
    pub(crate) size: u64,
    /// Its modification time, in nanoseconds since the Unix epoch.
    pub(crate) modified: u64,
}

impl Stamp {
    /// The stamp of the file `metadata` describes, whose text an analysis
    /// that started at `started` read as `len` bytes. `None` when the size
    /// differs or the file changed too close to `started` to tell the two
    /// apart.
    pub(crate) fn of(
        metadata: &std::fs::Metadata,
        len: usize,
        started: SystemTime,
    ) -> Option<Self> {
        let modified = metadata.modified().ok()?;
        if modified + SETTLE >= started || metadata.len() != len as u64 {
            return None;
        }
        Some(Self {
            size: metadata.len(),
            modified: nanos(modified)?,
        })
    }

    /// Whether `metadata` still has this size and modification time.
    pub(crate) fn matches(self, metadata: &std::fs::Metadata) -> bool {
        metadata.len() == self.size
            && metadata
                .modified()
                .ok()
                .and_then(nanos)
                .is_some_and(|at| at == self.modified)
    }
}

/// `at` in nanoseconds since the Unix epoch, where that fits a `u64`.
fn nanos(at: SystemTime) -> Option<u64> {
    u64::try_from(at.duration_since(UNIX_EPOCH).ok()?.as_nanos()).ok()
}

/// The index file for the workspace `root` in the directory `dir`: the first
/// 16 hex digits of the BLAKE3 hash of the canonical root, and `.idx`.
#[must_use]
pub(crate) fn file_for(dir: &Path, root: &Path) -> PathBuf {
    let root = nvs_diagnostics::canonical_key(root);
    let hash = content_hash(root.as_os_str().as_encoded_bytes()).to_string();
    dir.join(format!("{}.idx", &hash[..16]))
}

/// Writes `bytes` to `path` through a temporary file beside it, which is
/// renamed over `path`. Every way out of a failed write deletes the
/// temporary file.
///
/// The temporary name has the process id and a counter, so two writers never
/// share one, in one process or in two.
pub(crate) fn write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let (Some(dir), Some(name)) = (path.parent(), path.file_name()) else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "an index file needs a directory and a name",
        ));
    };
    let temp = dir.join(format!(
        "{}.{}.{}.tmp",
        name.to_string_lossy(),
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let written = std::fs::write(&temp, bytes).and_then(|()| std::fs::rename(&temp, path));
    if written.is_err() {
        drop(std::fs::remove_file(&temp));
    }
    written
}

/// What a valid index file holds.
pub(crate) struct Saved {
    /// Every entry, under its file's canonical key.
    pub(crate) files: Vec<(PathBuf, Indexed)>,
    /// Every entry point's reads.
    pub(crate) reads: Vec<(PathBuf, Vec<PathBuf>)>,
}

/// The bytes of an index file over `files` and `reads`, for the build `stamp`
/// and the canonical workspace `root`. `None` when a path is not UTF-8, which
/// the format cannot write.
pub(crate) fn encode<'a>(
    files: impl Iterator<Item = (&'a PathBuf, &'a Indexed)>,
    reads: impl Iterator<Item = (&'a PathBuf, &'a Vec<PathBuf>)>,
    stamp: Digest,
    root: &Path,
) -> Option<Vec<u8>> {
    let mut body = Writer::default();
    let files: Vec<_> = files.collect();
    body.u32(u32::try_from(files.len()).ok()?);
    for (path, indexed) in files {
        body.path(path)?;
        body.indexed(indexed)?;
    }
    let reads: Vec<_> = reads.collect();
    body.u32(u32::try_from(reads.len()).ok()?);
    for (entry, read) in reads {
        body.path(entry)?;
        body.u32(u32::try_from(read.len()).ok()?);
        for path in read {
            body.path(path)?;
        }
    }

    let mut out = Writer::default();
    out.out.extend_from_slice(MAGIC);
    out.u32(FORMAT);
    out.out.extend_from_slice(stamp.as_bytes());
    out.str(root.to_str()?)?;
    out.u32(u32::try_from(body.paths.len()).ok()?);
    for path in &body.table {
        out.str(path)?;
    }
    out.out.extend_from_slice(&body.out);
    let checksum = content_hash(&out.out);
    out.out.extend_from_slice(checksum.as_bytes());
    Some(out.out)
}

/// What `bytes` holds, when it is a whole index file of this format, for the
/// build `stamp` and the canonical workspace `root`. `None` for anything else.
pub(crate) fn decode(bytes: &[u8], stamp: Digest, root: &Path) -> Option<Saved> {
    let (body, checksum) = bytes.split_at_checked(bytes.len().checked_sub(DIGEST)?)?;
    if content_hash(body).as_bytes() != checksum {
        return None;
    }
    let mut reader = Reader {
        bytes: body,
        paths: Vec::new(),
    };
    if reader.take(MAGIC.len())? != MAGIC
        || reader.u32()? != FORMAT
        || reader.digest()? != stamp
        || Path::new(reader.str()?) != root
    {
        return None;
    }
    let paths = reader.count()?;
    let mut table = Vec::with_capacity(paths);
    for _ in 0..paths {
        table.push(PathBuf::from(reader.str()?));
    }
    reader.paths = table;

    let count = reader.count()?;
    let mut files = Vec::with_capacity(count);
    for _ in 0..count {
        let path = reader.path()?;
        files.push((path, reader.indexed()?));
    }
    let count = reader.count()?;
    let mut reads = Vec::with_capacity(count);
    for _ in 0..count {
        let entry = reader.path()?;
        let n = reader.count()?;
        let mut read = Vec::with_capacity(n);
        for _ in 0..n {
            read.push(reader.path()?);
        }
        reads.push((entry, read));
    }
    reader.bytes.is_empty().then_some(Saved { files, reads })
}

/// The bytes being written, and the path table they refer to.
#[derive(Default)]
struct Writer {
    out: Vec<u8>,
    /// Each path's index in `table`.
    paths: FxHashMap<PathBuf, u32>,
    /// Every path written, in the order it was first written.
    table: Vec<String>,
}

impl Writer {
    fn u8(&mut self, value: u8) {
        self.out.push(value);
    }

    fn u32(&mut self, value: u32) {
        self.out.extend_from_slice(&value.to_le_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.out.extend_from_slice(&value.to_le_bytes());
    }

    fn bool(&mut self, value: bool) {
        self.u8(u8::from(value));
    }

    fn digest(&mut self, value: Digest) {
        self.out.extend_from_slice(value.as_bytes());
    }

    fn len(&mut self, len: usize) -> Option<()> {
        self.u32(u32::try_from(len).ok()?);
        Some(())
    }

    fn str(&mut self, value: &str) -> Option<()> {
        self.len(value.len())?;
        self.out.extend_from_slice(value.as_bytes());
        Some(())
    }

    fn strs(&mut self, values: &[String]) -> Option<()> {
        self.len(values.len())?;
        for value in values {
            self.str(value)?;
        }
        Some(())
    }

    fn opt_str(&mut self, value: Option<&str>) -> Option<()> {
        match value {
            Some(value) => {
                self.u8(1);
                self.str(value)
            }
            None => {
                self.u8(0);
                Some(())
            }
        }
    }

    /// Writes `path`'s index in the path table, adding it there first.
    fn path(&mut self, path: &Path) -> Option<()> {
        let at = match self.paths.get(path) {
            Some(at) => *at,
            None => {
                let at = u32::try_from(self.table.len()).ok()?;
                self.table.push(path.to_str()?.to_owned());
                self.paths.insert(path.to_path_buf(), at);
                at
            }
        };
        self.u32(at);
        Some(())
    }

    fn span(&mut self, (start, end): (BytePos, BytePos)) {
        self.u32(start);
        self.u32(end);
    }

    fn site(&mut self, site: &Site) -> Option<()> {
        self.path(&site.path)?;
        self.span((site.start, site.end));
        Some(())
    }

    fn occurrence(&mut self, used: &Occurrence) -> Option<()> {
        self.str(&used.symbol)?;
        self.site(&used.site)?;
        self.opt_str(used.caller.as_deref())
    }

    fn occurrences(&mut self, values: &[Occurrence]) -> Option<()> {
        self.len(values.len())?;
        for used in values {
            self.occurrence(used)?;
        }
        Some(())
    }

    fn declaration(&mut self, declared: &Declaration) -> Option<()> {
        self.str(&declared.symbol)?;
        self.u8(kind_tag(declared.kind));
        self.site(&declared.site)?;
        self.u8(match declared.visibility {
            Visibility::Public => 0,
            Visibility::Protected => 1,
            Visibility::Private => 2,
        });
        self.strs(&declared.supertypes)?;
        match declared.construction {
            Some(construction) => {
                self.u8(1);
                self.bool(construction.is_abstract);
                match construction.parameters {
                    Some(n) => {
                        self.u8(1);
                        self.u64(u64::try_from(n).ok()?);
                    }
                    None => self.u8(0),
                }
            }
            None => self.u8(0),
        }
        self.len(declared.parameters.len())?;
        for parameter in &declared.parameters {
            self.str(&parameter.name)?;
            self.bool(parameter.takes_a_string);
        }
        self.strs(&declared.routes)?;
        self.bool(declared.test);
        match declared.extent {
            Some(extent) => {
                self.u8(1);
                self.span(extent);
            }
            None => self.u8(0),
        }
        Some(())
    }

    fn indexed(&mut self, indexed: &Indexed) -> Option<()> {
        self.digest(indexed.content);
        match indexed.stamp {
            Some(stamp) => {
                self.u8(1);
                self.u64(stamp.size);
                self.u64(stamp.modified);
            }
            None => self.u8(0),
        }
        self.digest(indexed.surface);
        self.path(&indexed.entry)?;
        self.len(indexed.decls.len())?;
        for declared in &indexed.decls {
            self.declaration(declared)?;
        }
        self.occurrences(&indexed.occurrences)?;
        self.len(indexed.imports.len())?;
        for import in &indexed.imports {
            self.str(&import.symbol)?;
            self.site(&import.site)?;
        }
        self.occurrences(&indexed.requests)?;
        self.strs(&indexed.unfollowed)?;
        self.len(indexed.queries.len())?;
        for query in &indexed.queries {
            self.span(*query);
        }
        self.u8(indexed.sole.map_or(0, |kind| kind_tag(kind) + 1));
        self.bool(indexed.errors);
        self.bool(indexed.broken);
        Some(())
    }
}

/// The byte a [`DeclKind`] is written as.
const fn kind_tag(kind: DeclKind) -> u8 {
    match kind {
        DeclKind::Class => 0,
        DeclKind::Interface => 1,
        DeclKind::Enum => 2,
        DeclKind::TypeAlias => 3,
        DeclKind::Method => 4,
        DeclKind::Property => 5,
        DeclKind::Const => 6,
        DeclKind::EnumCase => 7,
    }
}

/// The [`DeclKind`] written as `tag`.
const fn kind_of(tag: u8) -> Option<DeclKind> {
    Some(match tag {
        0 => DeclKind::Class,
        1 => DeclKind::Interface,
        2 => DeclKind::Enum,
        3 => DeclKind::TypeAlias,
        4 => DeclKind::Method,
        5 => DeclKind::Property,
        6 => DeclKind::Const,
        7 => DeclKind::EnumCase,
        _ => return None,
    })
}

/// The bytes not read yet, and the path table read so far.
struct Reader<'a> {
    bytes: &'a [u8],
    paths: Vec<PathBuf>,
}

impl<'a> Reader<'a> {
    /// The next `n` bytes, when that many remain.
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let (taken, rest) = self.bytes.split_at_checked(n)?;
        self.bytes = rest;
        Some(taken)
    }

    fn u8(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }

    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }

    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }

    fn bool(&mut self) -> Option<bool> {
        match self.u8()? {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        }
    }

    /// A tag that says whether an optional value follows.
    fn present(&mut self) -> Option<bool> {
        self.bool()
    }

    fn digest(&mut self) -> Option<Digest> {
        Some(Digest::from_bytes(self.take(DIGEST)?.try_into().ok()?))
    }

    /// A count of items that follow. Every item takes at least one byte, so a
    /// count above the bytes that remain is an error before anything is
    /// allocated for it.
    fn count(&mut self) -> Option<usize> {
        let n = usize::try_from(self.u32()?).ok()?;
        (n <= self.bytes.len()).then_some(n)
    }

    fn str(&mut self) -> Option<&'a str> {
        let len = usize::try_from(self.u32()?).ok()?;
        std::str::from_utf8(self.take(len)?).ok()
    }

    fn string(&mut self) -> Option<String> {
        self.str().map(str::to_owned)
    }

    fn strings(&mut self) -> Option<Vec<String>> {
        let n = self.count()?;
        let mut values = Vec::with_capacity(n);
        for _ in 0..n {
            values.push(self.string()?);
        }
        Some(values)
    }

    fn path(&mut self) -> Option<PathBuf> {
        let at = usize::try_from(self.u32()?).ok()?;
        self.paths.get(at).cloned()
    }

    fn span(&mut self) -> Option<(BytePos, BytePos)> {
        let start = self.u32()?;
        let end = self.u32()?;
        (start <= end).then_some((start, end))
    }

    fn site(&mut self) -> Option<Site> {
        let path = self.path()?;
        let (start, end) = self.span()?;
        Some(Site { path, start, end })
    }

    fn occurrence(&mut self) -> Option<Occurrence> {
        let symbol = self.string()?;
        let site = self.site()?;
        let caller = if self.present()? {
            Some(self.string()?)
        } else {
            None
        };
        Some(Occurrence {
            symbol,
            site,
            caller,
        })
    }

    fn occurrences(&mut self) -> Option<Vec<Occurrence>> {
        let n = self.count()?;
        let mut values = Vec::with_capacity(n);
        for _ in 0..n {
            values.push(self.occurrence()?);
        }
        Some(values)
    }

    fn declaration(&mut self) -> Option<Declaration> {
        let symbol = self.string()?;
        let kind = kind_of(self.u8()?)?;
        let site = self.site()?;
        let visibility = match self.u8()? {
            0 => Visibility::Public,
            1 => Visibility::Protected,
            2 => Visibility::Private,
            _ => return None,
        };
        let supertypes = self.strings()?;
        let construction = if self.present()? {
            let is_abstract = self.bool()?;
            let parameters = if self.present()? {
                Some(usize::try_from(self.u64()?).ok()?)
            } else {
                None
            };
            Some(Construction {
                is_abstract,
                parameters,
            })
        } else {
            None
        };
        let n = self.count()?;
        let mut parameters = Vec::with_capacity(n);
        for _ in 0..n {
            parameters.push(Parameter {
                name: self.string()?,
                takes_a_string: self.bool()?,
            });
        }
        let routes = self.strings()?;
        let test = self.bool()?;
        let extent = if self.present()? {
            Some(self.span()?)
        } else {
            None
        };
        Some(Declaration {
            symbol,
            kind,
            site,
            visibility,
            supertypes,
            construction,
            parameters,
            routes,
            test,
            extent,
        })
    }

    fn indexed(&mut self) -> Option<Indexed> {
        let content = self.digest()?;
        let stamp = if self.present()? {
            Some(Stamp {
                size: self.u64()?,
                modified: self.u64()?,
            })
        } else {
            None
        };
        let surface = self.digest()?;
        let entry = self.path()?;
        let n = self.count()?;
        let mut decls = Vec::with_capacity(n);
        for _ in 0..n {
            decls.push(self.declaration()?);
        }
        let occurrences = self.occurrences()?;
        let n = self.count()?;
        let mut imports = Vec::with_capacity(n);
        for _ in 0..n {
            imports.push(Import {
                symbol: self.string()?,
                site: self.site()?,
            });
        }
        let requests = self.occurrences()?;
        let unfollowed = self.strings()?;
        let n = self.count()?;
        let mut queries = Vec::with_capacity(n);
        for _ in 0..n {
            queries.push(self.span()?);
        }
        let sole = match self.u8()? {
            0 => None,
            tag => Some(kind_of(tag - 1)?),
        };
        Some(Indexed {
            decls,
            occurrences,
            imports,
            requests,
            unfollowed,
            queries,
            sole,
            errors: self.bool()?,
            broken: self.bool()?,
            surface,
            content,
            stamp,
            entry,
        })
    }
}
