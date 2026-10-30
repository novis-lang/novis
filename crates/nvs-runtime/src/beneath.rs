//! `rule:security/writes-open-beneath-a-handle`'s walk: a resolved path opened one level at a time,
//! each level from the handle of the one above it, following no link on the way.
//!
//! The read and write doors in [`crate::capability`] resolve a path once, check the grant on that
//! resolved path, and hand it here. A resolved path names no link, so a link met on the walk is one that was
//! planted after the check, and it fails the call with [`link_refused`]. Nothing is opened by its
//! whole path a second time, which is the window a check-then-open pair leaves.
//!
//! **Unix** opens each folder with `openat` and `O_NOFOLLOW | O_DIRECTORY`, creates one with
//! `mkdirat`, opens the file with `openat` and `O_NOFOLLOW`, and renames with `renameat` between
//! two walked folders. A removal is `unlinkat`, a listing is `fdopendir` on the walked folder, and
//! metadata is read from the entry opened without following it: `O_PATH` on Linux, `O_EVTONLY |
//! O_SYMLINK` on macOS. On Linux a folder is opened `O_PATH`, so a folder the process may search
//! but not list is still walked.
//!
//! **Windows** opens each level with `NtCreateFile`, its `RootDirectory` set to the folder above and
//! `FILE_OPEN_REPARSE_POINT` set, then reads the level's reparse tag. A name surrogate — a symbolic
//! link or a junction — is refused. Any other reparse point, such as a cloud placeholder, is a real
//! folder and is walked. A rename opens the source the same way and renames its handle with
//! `NtSetInformationFile`, the destination folder's handle as the new name's root. A removal opens
//! the name the same way and sets its delete disposition, metadata is read from the entry's handle,
//! and a listing reopens the walked folder for listing and reads it with
//! `GetFileInformationByHandleEx`.
//!
//! **Cost:** one open per level of the path, the same count the operating system's own lookup
//! makes, plus one attribute read per level on Windows. Every handle but the deepest is closed as
//! soon as the next one is open.

use std::fs::File;
use std::io;
use std::path::{Component, Path};

/// The error a link on the walk fails with, the same sentence on every platform.
fn link_refused() -> io::Error {
    io::Error::other(
        "a part of this path was replaced by a link after the path was checked, so it was not opened",
    )
}

/// The error a listing stops with when the request's memory ceiling refused its next entry. The
/// breach is already recorded, so [`crate::capability::read_dir`] reports the memory `FATAL`.
#[derive(Debug)]
pub(crate) struct ListingRefused;

impl std::fmt::Display for ListingRefused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the listing does not fit in the request's memory limit")
    }
}

impl std::error::Error for ListingRefused {}

/// Whether the running request can afford one more entry named in `name` bytes, asked before the
/// entry is made. Where `entries` is full, the ask also covers the block it grows into, which is
/// held with the old one while the entries move.
///
/// **What it spends:** one comparison per entry.
fn affords_entry(entries: &Vec<Entry>, name: usize) -> io::Result<()> {
    let grows = if entries.len() == entries.capacity() {
        entries
            .capacity()
            .max(4)
            .saturating_mul(2)
            .saturating_mul(std::mem::size_of::<Entry>())
    } else {
        0
    };
    if crate::budget::affords(grows.saturating_add(name)) {
        Ok(())
    } else {
        Err(io::Error::other(ListingRefused))
    }
}

/// How [`open_file`] opens the file at the end of the walk. Every mode but [`Mode::Read`] creates
/// the file when it is not there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    /// Reading only, and the file must be there.
    Read,
    /// Writing only, and the file must not be there yet.
    New,
    /// Writing only, emptying what was there.
    Truncate,
    /// Writing only, emptying nothing.
    Write,
    /// Writing only, at the end of what was there.
    Append,
    /// Reading and writing, emptying nothing.
    ReadWrite,
}

impl Mode {
    /// The flags `mode` opens with.
    const fn open(self) -> Open {
        let none = Open {
            read: false,
            write: false,
            append: false,
            create: false,
            exclusive: false,
            truncate: false,
        };
        match self {
            Self::Read => Open { read: true, ..none },
            Self::New => Open {
                write: true,
                create: true,
                exclusive: true,
                ..none
            },
            Self::Truncate => Open {
                write: true,
                create: true,
                truncate: true,
                ..none
            },
            Self::Write => Open {
                write: true,
                create: true,
                ..none
            },
            Self::Append => Open {
                append: true,
                create: true,
                ..none
            },
            Self::ReadWrite => Open {
                read: true,
                write: true,
                create: true,
                ..none
            },
        }
    }
}

/// How [`open_with`] opens the file at the end of the walk, flag by flag, as
/// `std::fs::OpenOptions` names them. [`Mode`] is the doors' fixed set of these.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "one flag per open option, the shape `std::fs::OpenOptions` has"
)]
pub struct Open {
    /// The file may be read.
    pub read: bool,
    /// The file may be written anywhere.
    pub write: bool,
    /// The file is written only at its end. Set it without `write`.
    pub append: bool,
    /// A missing file is created.
    pub create: bool,
    /// The file is created, and a name already there fails with `AlreadyExists`.
    pub exclusive: bool,
    /// What was there is emptied. It needs `write`.
    pub truncate: bool,
}

/// The file at `path`, opened as `mode` says.
///
/// `path` is a resolved, absolute path. Its folders must exist.
///
/// # Errors
///
/// [`link_refused`] when a level of `path`, the file included, is a link. Otherwise the operating
/// system's error, which is `AlreadyExists` for [`Mode::New`] on a name already there and
/// `NotFound` for [`Mode::Read`] on a name that is not.
pub(crate) fn open_file(path: &Path, mode: Mode) -> io::Result<File> {
    open_with(path, mode.open())
}

/// [`open_file`] with each flag chosen on its own.
///
/// # Errors
///
/// [`open_file`]'s, and `AlreadyExists` for `exclusive` on a name already there.
pub fn open_with(path: &Path, how: Open) -> io::Result<File> {
    let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
        return Err(io::Error::from(io::ErrorKind::InvalidInput));
    };
    let dir = walk(parent, false)?;
    sys::open_at_end(&dir, name, how)
}

/// The one folder `path` names created. Its parent must exist.
///
/// # Errors
///
/// [`link_refused`] when a folder above it is a link, and `AlreadyExists` when the name is
/// there, a link included. Otherwise the operating system's error.
pub fn create_dir(path: &Path) -> io::Result<()> {
    let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
        return Err(io::Error::from(io::ErrorKind::InvalidInput));
    };
    let dir = walk(parent, false)?;
    sys::make_dir(&dir, name)
}

/// [`open_file`] for a door that creates: exclusively when `overwrite` is false, or emptying what
/// was there when it is true.
///
/// # Errors
///
/// [`open_file`]'s.
pub(crate) fn create_file(path: &Path, overwrite: bool) -> io::Result<File> {
    open_file(path, if overwrite { Mode::Truncate } else { Mode::New })
}

/// The name `from` moved to the name `to`, replacing what `to` names.
///
/// Both are resolved, absolute paths. Each parent is walked as [`open_file`] walks one, and the
/// rename is made between the two folder handles: `renameat` on Unix, and on Windows a rename of
/// the source's handle with the destination folder's handle as its root. The last name of either
/// end is never followed, so a link there is renamed or replaced as the entry it is.
///
/// # Errors
///
/// [`link_refused`] when a folder above either name is a link. Otherwise the operating system's
/// error, such as nothing at `from` or the two names on different filesystems.
pub fn rename(from: &Path, to: &Path) -> io::Result<()> {
    let (Some(from_parent), Some(from_name), Some(to_parent), Some(to_name)) =
        (from.parent(), from.file_name(), to.parent(), to.file_name())
    else {
        return Err(io::Error::from(io::ErrorKind::InvalidInput));
    };
    let from_dir = walk(from_parent, false)?;
    let to_dir = walk(to_parent, false)?;
    sys::rename_at(&from_dir, from_name, &to_dir, to_name)
}

/// The file `path` names removed. Its folder is walked as [`open_file`] walks one, and the name is
/// removed from that folder's handle: `unlinkat` on Unix, and on Windows the name opened beneath
/// the handle and marked for deletion. A link there is the entry removed, never its target.
///
/// # Errors
///
/// [`link_refused`] when a folder above the name is a link. Otherwise the operating system's
/// error, such as nothing at `path` or a folder there.
pub fn remove_file(path: &Path) -> io::Result<()> {
    let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
        return Err(io::Error::from(io::ErrorKind::InvalidInput));
    };
    let dir = walk(parent, false)?;
    sys::remove_at(&dir, name, false)
}

/// The empty folder `path` names removed, as [`remove_file`] removes a file.
///
/// # Errors
///
/// [`link_refused`] when a folder above the name is a link. Otherwise the operating system's
/// error, such as nothing at `path`, a file there, or a folder that is not empty.
pub fn remove_dir(path: &Path) -> io::Result<()> {
    let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
        return Err(io::Error::from(io::ErrorKind::InvalidInput));
    };
    let dir = walk(parent, false)?;
    sys::remove_at(&dir, name, true)
}

/// What the file system knows about the file or folder at `path`, read from a handle opened
/// beneath its folder.
///
/// `path` is resolved, so it names no link. A link at its last name was planted after the check,
/// or led nowhere when the path was resolved, and either way it is not followed: the call fails
/// with `NotFound`, the answer a link that leads nowhere gives.
///
/// # Errors
///
/// [`link_refused`] when a folder above the name is a link, `NotFound` for a link at the name, and
/// otherwise the operating system's error.
pub fn metadata(path: &Path) -> io::Result<std::fs::Metadata> {
    let found = entry_metadata(path)?;
    if found.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "the last name of this path is a link, and a link is not followed after the path \
             was checked",
        ));
    }
    Ok(found)
}

/// [`metadata`] of the entry itself: a link at the last name is read as the link it is.
///
/// # Errors
///
/// [`link_refused`] when a folder above the name is a link, and otherwise the operating system's
/// error.
pub fn entry_metadata(path: &Path) -> io::Result<std::fs::Metadata> {
    match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) => sys::stat_at(&walk(parent, false)?, name),
        // A root: the walk opens it, and its own handle is what is read.
        _ => File::from(walk(path, false)?).metadata(),
    }
}

/// What one name in a listed folder is, read without following a link.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A folder.
    Dir,
    /// A regular file.
    File,
    /// A symbolic link, or on Windows a junction.
    Link,
    /// Anything else: a device, a socket, a pipe.
    Other,
}

/// One name in a folder [`list`] read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The name, without the folder's path.
    pub name: std::ffi::OsString,
    /// What the name is.
    pub kind: Kind,
}

/// Every name in the folder at `path` but `.` and `..`, in the order the file system gives them.
/// The folder is walked as [`open_file`] walks one, and listed from its own handle.
///
/// **Memory:** the whole listing, one name and one kind per entry, until the caller drops it. Each
/// entry is asked of the request's memory ceiling before it is made ([`affords_entry`]).
///
/// # Errors
///
/// [`link_refused`] when a level of `path` is a link, and [`ListingRefused`] when the request's
/// memory ceiling refused an entry. Otherwise the operating system's error, such as nothing at
/// `path` or a file there.
pub fn list(path: &Path) -> io::Result<Vec<Entry>> {
    sys::list(&walk(path, false)?)
}

/// Whether two open files are one file: the same device and file number, so a second spelling and
/// a hard link both count.
///
/// # Errors
///
/// The operating system's error when either handle cannot be asked.
pub(crate) fn same_file(a: &File, b: &File) -> io::Result<bool> {
    Ok(sys::identity(a)? == sys::identity(b)?)
}

/// Every missing folder of `path` created, the way `create_dir_all` does, from the root down.
///
/// # Errors
///
/// [`link_refused`] when a level of `path` is a link, or the operating system's error when a level
/// could not be opened or created, such as a file where a folder was expected.
pub(crate) fn create_dirs(path: &Path) -> io::Result<()> {
    walk(path, true).map(drop)
}

/// The folder at `path`, opened from the root one level at a time, creating a missing level when
/// `create` is set.
fn walk(path: &Path, create: bool) -> io::Result<sys::Dir> {
    let mut components = path.components().peekable();
    let mut root = std::ffi::OsString::new();
    while let Some(component @ (Component::Prefix(_) | Component::RootDir)) =
        components.peek().copied()
    {
        root.push(component.as_os_str());
        components.next();
    }
    if !Path::new(&root).has_root() {
        return Err(io::Error::from(io::ErrorKind::InvalidInput));
    }
    let mut dir = sys::open_root(Path::new(&root))?;
    for component in components {
        let Component::Normal(name) = component else {
            return Err(io::Error::from(io::ErrorKind::InvalidInput));
        };
        dir = sys::child(&dir, name, create)?;
    }
    Ok(dir)
}

#[cfg(unix)]
mod sys {
    use std::ffi::{CString, OsStr};
    use std::fs::File;
    use std::io;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    use std::os::unix::ffi::OsStrExt;
    use std::path::Path;

    pub(super) type Dir = OwnedFd;

    #[cfg(any(target_os = "linux", target_os = "android"))]
    const DIR_FLAGS: libc::c_int =
        libc::O_PATH | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    const DIR_FLAGS: libc::c_int =
        libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;

    pub(super) fn open_root(root: &Path) -> io::Result<Dir> {
        let name = c_name(root.as_os_str())?;
        open_at(libc::AT_FDCWD, &name, DIR_FLAGS, 0)
    }

    pub(super) fn child(dir: &Dir, name: &OsStr, create: bool) -> io::Result<Dir> {
        let name = c_name(name)?;
        match open_at(dir.as_raw_fd(), &name, DIR_FLAGS, 0) {
            Err(err) if create && err.kind() == io::ErrorKind::NotFound => {
                match make_at(dir, &name) {
                    Err(err) if err.kind() != io::ErrorKind::AlreadyExists => return Err(err),
                    _ => {}
                }
                open_at(dir.as_raw_fd(), &name, DIR_FLAGS, 0)
            }
            other => other,
        }
        .map_err(|err| explain(dir, &name, err))
    }

    pub(super) fn open_at_end(dir: &Dir, name: &OsStr, how: super::Open) -> io::Result<File> {
        let name = c_name(name)?;
        let mut flags = match (how.read, how.write || how.append) {
            (true, true) => libc::O_RDWR,
            (false, true) => libc::O_WRONLY,
            _ => libc::O_RDONLY,
        };
        for (set, flag) in [
            (how.append, libc::O_APPEND),
            (how.create, libc::O_CREAT),
            (how.exclusive, libc::O_CREAT | libc::O_EXCL),
            (how.truncate, libc::O_TRUNC),
        ] {
            if set {
                flags |= flag;
            }
        }
        let flags = libc::O_NOFOLLOW | libc::O_CLOEXEC | flags;
        open_at(dir.as_raw_fd(), &name, flags, 0o666)
            .map(File::from)
            .map_err(|err| explain(dir, &name, err))
    }

    pub(super) fn make_dir(dir: &Dir, name: &OsStr) -> io::Result<()> {
        let name = c_name(name)?;
        make_at(dir, &name)
    }

    pub(super) fn rename_at(
        from_dir: &Dir,
        from: &OsStr,
        to_dir: &Dir,
        to: &OsStr,
    ) -> io::Result<()> {
        let (from, to) = (c_name(from)?, c_name(to)?);
        #[expect(
            unsafe_code,
            reason = "`renameat` is the only way to rename between two folder handles. Both \
                      descriptors are ones this module opened and still owns, and both names are \
                      NUL-terminated `CString`s that outlive the call."
        )]
        let status = unsafe {
            libc::renameat(
                from_dir.as_raw_fd(),
                from.as_ptr(),
                to_dir.as_raw_fd(),
                to.as_ptr(),
            )
        };
        if status == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    pub(super) fn identity(file: &File) -> io::Result<(u64, u64)> {
        use std::os::unix::fs::MetadataExt;
        let meta = file.metadata()?;
        Ok((meta.dev(), meta.ino()))
    }

    /// The flags that open an entry itself, a link included, for its metadata alone. Elsewhere a
    /// link fails the open with `ELOOP`, which [`explain`] reports as a link.
    #[cfg(any(target_os = "linux", target_os = "android"))]
    const ENTRY_FLAGS: libc::c_int = libc::O_PATH | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    #[cfg(target_vendor = "apple")]
    const ENTRY_FLAGS: libc::c_int =
        libc::O_EVTONLY | libc::O_SYMLINK | libc::O_NONBLOCK | libc::O_CLOEXEC;
    #[cfg(not(any(target_os = "linux", target_os = "android", target_vendor = "apple")))]
    const ENTRY_FLAGS: libc::c_int =
        libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC;

    pub(super) fn stat_at(dir: &Dir, name: &OsStr) -> io::Result<std::fs::Metadata> {
        let name = c_name(name)?;
        let entry = open_at(dir.as_raw_fd(), &name, ENTRY_FLAGS, 0)
            .map_err(|err| explain(dir, &name, err))?;
        File::from(entry).metadata()
    }

    pub(super) fn remove_at(dir: &Dir, name: &OsStr, folder: bool) -> io::Result<()> {
        let name = c_name(name)?;
        let flags = if folder { libc::AT_REMOVEDIR } else { 0 };
        #[expect(
            unsafe_code,
            reason = "`unlinkat` is the only way to remove a name under a folder handle. The \
                      descriptor is one this module opened and still owns, and the name is a \
                      NUL-terminated `CString` that outlives the call."
        )]
        let status = unsafe { libc::unlinkat(dir.as_raw_fd(), name.as_ptr(), flags) };
        if status == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    pub(super) fn list(dir: &Dir) -> io::Result<Vec<super::Entry>> {
        use std::os::fd::IntoRawFd;
        // The walked folder may be an `O_PATH` handle, which cannot be read, so `.` beneath it is
        // opened for reading. `.` is the folder itself and never a link.
        let here = c_name(OsStr::new("."))?;
        let readable = open_at(
            dir.as_raw_fd(),
            &here,
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
            0,
        )?;
        #[expect(
            unsafe_code,
            reason = "`fdopendir`, `readdir` and `closedir` are the only way to list a folder from \
                      a handle; `std::fs::read_dir` takes a path. The stream takes the descriptor \
                      only when `fdopendir` succeeds, and is closed exactly once below. Each entry \
                      `readdir` returns is read before the next call, its name as the \
                      NUL-terminated string `d_name` is."
        )]
        unsafe {
            let stream = libc::fdopendir(readable.as_raw_fd());
            if stream.is_null() {
                return Err(io::Error::last_os_error());
            }
            let _owned_by_stream = readable.into_raw_fd();
            let mut entries = Vec::new();
            let result = loop {
                clear_errno();
                let entry = libc::readdir(stream);
                if entry.is_null() {
                    let err = io::Error::last_os_error();
                    break match err.raw_os_error() {
                        Some(0) | None => Ok(()),
                        Some(_) => Err(err),
                    };
                }
                let name = std::ffi::CStr::from_ptr((*entry).d_name.as_ptr());
                if matches!(name.to_bytes(), b"." | b"..") {
                    continue;
                }
                let kind = match (*entry).d_type {
                    libc::DT_DIR => super::Kind::Dir,
                    libc::DT_REG => super::Kind::File,
                    libc::DT_LNK => super::Kind::Link,
                    libc::DT_UNKNOWN => kind_at(libc::dirfd(stream), name),
                    _ => super::Kind::Other,
                };
                if let Err(refused) = super::affords_entry(&entries, name.to_bytes().len()) {
                    break Err(refused);
                }
                entries.push(super::Entry {
                    name: OsStr::from_bytes(name.to_bytes()).to_owned(),
                    kind,
                });
            };
            libc::closedir(stream);
            result.map(|()| entries)
        }
    }

    /// What `name` under `dir` is, for a file system whose listing does not say. A name that
    /// cannot be read is [`super::Kind::Other`].
    fn kind_at(dir: libc::c_int, name: &std::ffi::CStr) -> super::Kind {
        #[expect(
            unsafe_code,
            reason = "`fstatat` without following the last name reads what a listed name is. The \
                      descriptor is the open listing's own, the name outlives the call, and the \
                      out-parameter is a zeroed stack `stat` this call owns exclusively."
        )]
        let (status, stat) = unsafe {
            let mut stat: libc::stat = std::mem::zeroed();
            let status =
                libc::fstatat(dir, name.as_ptr(), &raw mut stat, libc::AT_SYMLINK_NOFOLLOW);
            (status, stat)
        };
        if status != 0 {
            return super::Kind::Other;
        }
        match stat.st_mode & libc::S_IFMT {
            libc::S_IFDIR => super::Kind::Dir,
            libc::S_IFREG => super::Kind::File,
            libc::S_IFLNK => super::Kind::Link,
            _ => super::Kind::Other,
        }
    }

    /// `errno` set to zero, so a `readdir` that returns nothing tells the end from an error.
    fn clear_errno() {
        #[expect(
            unsafe_code,
            reason = "the thread's `errno` is a thread-local `int` the C library hands out a \
                      pointer to, valid for the thread's whole life"
        )]
        unsafe {
            #[cfg(target_os = "linux")]
            {
                *libc::__errno_location() = 0;
            }
            #[cfg(target_os = "android")]
            {
                *libc::__errno() = 0;
            }
            #[cfg(target_vendor = "apple")]
            {
                *libc::__error() = 0;
            }
        }
    }

    fn c_name(name: &OsStr) -> io::Result<CString> {
        CString::new(name.as_bytes()).map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))
    }

    fn open_at(
        dir: libc::c_int,
        name: &CString,
        flags: libc::c_int,
        mode: libc::c_uint,
    ) -> io::Result<OwnedFd> {
        #[expect(
            unsafe_code,
            reason = "`openat` is the only way to open a name relative to a folder handle, and \
                      `std` has no equivalent. The descriptor is `AT_FDCWD` or one this module \
                      opened and still owns, the name is a NUL-terminated `CString` that outlives \
                      the call, and a descriptor the call returns is owned by nothing else, so \
                      `OwnedFd` takes it exactly once."
        )]
        unsafe {
            let fd = libc::openat(dir, name.as_ptr(), flags, mode);
            if fd < 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(OwnedFd::from_raw_fd(fd))
            }
        }
    }

    fn make_at(dir: &Dir, name: &CString) -> io::Result<()> {
        #[expect(
            unsafe_code,
            reason = "`mkdirat` is the only way to create a folder relative to a folder handle. \
                      The descriptor is one this module opened and still owns, and the name is a \
                      NUL-terminated `CString` that outlives the call."
        )]
        let status = unsafe { libc::mkdirat(dir.as_raw_fd(), name.as_ptr(), 0o777) };
        if status == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    /// `err`, or [`super::link_refused`] when the name it failed on is a link. A link fails
    /// `O_NOFOLLOW` with `ELOOP`, and an `O_PATH | O_DIRECTORY` open with `ENOTDIR`, which a
    /// plain file also gives, so the name is looked at to tell the two apart.
    fn explain(dir: &Dir, name: &CString, err: io::Error) -> io::Error {
        if !matches!(err.raw_os_error(), Some(libc::ELOOP | libc::ENOTDIR)) {
            return err;
        }
        #[expect(
            unsafe_code,
            reason = "`fstatat` without following the last name is the only way to ask whether a \
                      name under a folder handle is a link. The descriptor is one this module \
                      owns, the name outlives the call, and the out-parameter is a zeroed stack \
                      `stat` this call owns exclusively."
        )]
        let is_link = unsafe {
            let mut stat: libc::stat = std::mem::zeroed();
            libc::fstatat(
                dir.as_raw_fd(),
                name.as_ptr(),
                &raw mut stat,
                libc::AT_SYMLINK_NOFOLLOW,
            ) == 0
                && stat.st_mode & libc::S_IFMT == libc::S_IFLNK
        };
        if is_link { super::link_refused() } else { err }
    }
}

#[cfg(windows)]
mod sys {
    use std::ffi::OsStr;
    use std::fs::File;
    use std::io;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::os::windows::fs::OpenOptionsExt;
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    use std::path::Path;

    use windows_sys::Wdk::Foundation::OBJECT_ATTRIBUTES;
    use windows_sys::Wdk::Storage::FileSystem::{
        FILE_CREATE, FILE_DIRECTORY_FILE, FILE_NON_DIRECTORY_FILE, FILE_OPEN, FILE_OPEN_IF,
        FILE_OPEN_REPARSE_POINT, FILE_RENAME_INFORMATION, FILE_RENAME_INFORMATION_0,
        FILE_RENAME_POSIX_SEMANTICS, FILE_RENAME_REPLACE_IF_EXISTS, FILE_SYNCHRONOUS_IO_NONALERT,
        FileRenameInformation, FileRenameInformationEx, NtCreateFile, NtSetInformationFile,
    };
    use windows_sys::Win32::Foundation::{
        ERROR_INVALID_FUNCTION, ERROR_INVALID_PARAMETER, ERROR_NO_MORE_FILES, ERROR_NOT_SUPPORTED,
        HANDLE, OBJ_CASE_INSENSITIVE, RtlNtStatusToDosError, UNICODE_STRING,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, DELETE, FILE_ATTRIBUTE_DEVICE, FILE_ATTRIBUTE_DIRECTORY,
        FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_REPARSE_POINT, FILE_ATTRIBUTE_TAG_INFO,
        FILE_DISPOSITION_FLAG_DELETE, FILE_DISPOSITION_FLAG_POSIX_SEMANTICS, FILE_DISPOSITION_INFO,
        FILE_DISPOSITION_INFO_EX, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
        FILE_FULL_DIR_INFO, FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_LIST_DIRECTORY,
        FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_TRAVERSE,
        FILE_WRITE_DATA, FileAttributeTagInfo, FileDispositionInfo, FileDispositionInfoEx,
        FileFullDirectoryInfo, FileFullDirectoryRestartInfo, GetFileInformationByHandle,
        GetFileInformationByHandleEx, SYNCHRONIZE, SetFileInformationByHandle,
    };
    use windows_sys::Win32::System::IO::IO_STATUS_BLOCK;

    pub(super) type Dir = OwnedHandle;

    /// The bit `IsReparseTagNameSurrogate` tests: a reparse point that stands for another name,
    /// which is what a symbolic link and a junction are.
    const NAME_SURROGATE: u32 = 0x2000_0000;

    const DIR_ACCESS: u32 = FILE_TRAVERSE | FILE_READ_ATTRIBUTES | SYNCHRONIZE;

    pub(super) fn open_root(root: &Path) -> io::Result<Dir> {
        let file = std::fs::OpenOptions::new()
            .access_mode(DIR_ACCESS)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
            .open(root)?;
        Ok(OwnedHandle::from(file))
    }

    pub(super) fn child(dir: &Dir, name: &OsStr, create: bool) -> io::Result<Dir> {
        let disposition = if create { FILE_OPEN_IF } else { FILE_OPEN };
        let handle = open_at(
            dir,
            name,
            DIR_ACCESS,
            disposition,
            FILE_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT,
        )?;
        refuse_surrogate(&handle)?;
        Ok(handle)
    }

    pub(super) fn open_at_end(dir: &Dir, name: &OsStr, how: super::Open) -> io::Result<File> {
        // `FILE_CREATE` fails on any name already there, a link included. Every other open takes
        // what is there without emptying it, so a link is refused before it loses anything, and a
        // truncate empties the file afterwards. A handle without `FILE_WRITE_DATA` writes only at
        // the end, which is how `std` appends too. An open that does not create creates nothing.
        let mut access = 0;
        if how.read {
            access |= FILE_GENERIC_READ;
        }
        if how.write {
            access |= FILE_GENERIC_WRITE;
        } else if how.append {
            access |= FILE_GENERIC_WRITE & !FILE_WRITE_DATA;
        }
        let disposition = if how.exclusive {
            FILE_CREATE
        } else if how.create {
            FILE_OPEN_IF
        } else {
            FILE_OPEN
        };
        let handle = open_at(
            dir,
            name,
            access | FILE_READ_ATTRIBUTES | SYNCHRONIZE,
            disposition,
            FILE_NON_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT,
        )?;
        refuse_surrogate(&handle)?;
        let file = File::from(handle);
        if how.truncate {
            file.set_len(0)?;
        }
        Ok(file)
    }

    pub(super) fn make_dir(dir: &Dir, name: &OsStr) -> io::Result<()> {
        open_at(
            dir,
            name,
            DIR_ACCESS,
            FILE_CREATE,
            FILE_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT,
        )
        .map(drop)
    }

    pub(super) fn rename_at(
        from_dir: &Dir,
        from: &OsStr,
        to_dir: &Dir,
        to: &OsStr,
    ) -> io::Result<()> {
        let source = open_at(
            from_dir,
            from,
            DELETE | SYNCHRONIZE,
            FILE_OPEN,
            FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT,
        )?;
        let wide: Vec<u16> = to.encode_wide().collect();
        let name_at = std::mem::offset_of!(FILE_RENAME_INFORMATION, FileName);
        // Never less than the struct itself, which a one-letter name would be.
        let size = (name_at + wide.len() * 2).max(size_of::<FILE_RENAME_INFORMATION>());
        let invalid = || io::Error::from(io::ErrorKind::InvalidFilename);
        let name_bytes = u32::try_from(wide.len() * 2).map_err(|_| invalid())?;
        let size_arg = u32::try_from(size).map_err(|_| invalid())?;
        // `u64` cells, so the buffer is aligned for the struct's handle field.
        let mut buffer = vec![0_u64; size.div_ceil(8)];
        let info: *mut FILE_RENAME_INFORMATION = buffer.as_mut_ptr().cast();
        #[expect(
            unsafe_code,
            reason = "`NtSetInformationFile` with a rename class is the only Windows call that \
                      renames into a folder named by a handle. `FILE_RENAME_INFORMATION` ends in \
                      a variable-length name, so it is written field by field into a zeroed, \
                      8-byte-aligned buffer of exactly the size passed, which this call owns and \
                      which outlives both calls. Both handles are ones this module owns."
        )]
        unsafe {
            (&raw mut (*info).Anonymous).write(FILE_RENAME_INFORMATION_0 {
                Flags: FILE_RENAME_REPLACE_IF_EXISTS | FILE_RENAME_POSIX_SEMANTICS,
            });
            (&raw mut (*info).RootDirectory).write(to_dir.as_raw_handle() as HANDLE);
            (&raw mut (*info).FileNameLength).write(name_bytes);
            std::ptr::copy_nonoverlapping(
                wide.as_ptr(),
                (&raw mut (*info).FileName).cast::<u16>(),
                wide.len(),
            );
            let handle = source.as_raw_handle() as HANDLE;
            let mut status_block = IO_STATUS_BLOCK::default();
            let mut status = NtSetInformationFile(
                handle,
                &raw mut status_block,
                info.cast(),
                size_arg,
                FileRenameInformationEx,
            );
            // A file system with no POSIX rename, such as FAT, refuses the extended class, and
            // the plain one replaces the destination when nothing holds it open.
            if status < 0
                && matches!(
                    RtlNtStatusToDosError(status),
                    ERROR_INVALID_PARAMETER | ERROR_NOT_SUPPORTED | ERROR_INVALID_FUNCTION
                )
            {
                (&raw mut (*info).Anonymous).write(FILE_RENAME_INFORMATION_0 {
                    Flags: FILE_RENAME_REPLACE_IF_EXISTS,
                });
                status = NtSetInformationFile(
                    handle,
                    &raw mut status_block,
                    info.cast(),
                    size_arg,
                    FileRenameInformation,
                );
            }
            if status < 0 {
                let code = RtlNtStatusToDosError(status);
                return Err(io::Error::from_raw_os_error(
                    i32::try_from(code).unwrap_or(i32::MAX),
                ));
            }
            Ok(())
        }
    }

    pub(super) fn identity(file: &File) -> io::Result<(u32, u32, u32)> {
        let mut info = BY_HANDLE_FILE_INFORMATION::default();
        #[expect(
            unsafe_code,
            reason = "`GetFileInformationByHandle` is the stable way to read a file's volume and \
                      file number off a handle; `std` keeps both behind an unstable feature. The \
                      handle is borrowed from a live `File`, and the out-parameter is a stack \
                      struct this call owns exclusively."
        )]
        let ok =
            unsafe { GetFileInformationByHandle(file.as_raw_handle() as HANDLE, &raw mut info) };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok((
            info.dwVolumeSerialNumber,
            info.nFileIndexHigh,
            info.nFileIndexLow,
        ))
    }

    fn open_at(
        dir: &Dir,
        name: &OsStr,
        access: u32,
        disposition: u32,
        options: u32,
    ) -> io::Result<OwnedHandle> {
        let mut wide: Vec<u16> = name.encode_wide().collect();
        let bytes = u16::try_from(wide.len() * 2)
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidFilename))?;
        let unicode = UNICODE_STRING {
            Length: bytes,
            MaximumLength: bytes,
            Buffer: wide.as_mut_ptr(),
        };
        let attributes = OBJECT_ATTRIBUTES {
            Length: u32::try_from(size_of::<OBJECT_ATTRIBUTES>()).unwrap_or(u32::MAX),
            RootDirectory: dir.as_raw_handle() as HANDLE,
            ObjectName: &raw const unicode,
            Attributes: OBJ_CASE_INSENSITIVE,
            SecurityDescriptor: std::ptr::null(),
            SecurityQualityOfService: std::ptr::null(),
        };
        let mut handle: HANDLE = std::ptr::null_mut();
        let mut status_block = IO_STATUS_BLOCK::default();
        #[expect(
            unsafe_code,
            reason = "`NtCreateFile` is the only Windows call that opens a name relative to a \
                      folder handle; `CreateFileW` takes a whole path. The root handle is one this \
                      module opened and still owns, the name buffer and both structs are locals \
                      that outlive the call, and a handle the call returns on success is owned by \
                      nothing else, so `OwnedHandle` takes it exactly once."
        )]
        unsafe {
            let status = NtCreateFile(
                &raw mut handle,
                access,
                &raw const attributes,
                &raw mut status_block,
                std::ptr::null(),
                FILE_ATTRIBUTE_NORMAL,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                disposition,
                options,
                std::ptr::null(),
                0,
            );
            if status < 0 {
                let code = RtlNtStatusToDosError(status);
                return Err(io::Error::from_raw_os_error(
                    i32::try_from(code).unwrap_or(i32::MAX),
                ));
            }
            Ok(OwnedHandle::from_raw_handle(handle.cast()))
        }
    }

    pub(super) fn stat_at(dir: &Dir, name: &OsStr) -> io::Result<std::fs::Metadata> {
        let entry = open_at(
            dir,
            name,
            FILE_READ_ATTRIBUTES | SYNCHRONIZE,
            FILE_OPEN,
            FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT,
        )?;
        File::from(entry).metadata()
    }

    pub(super) fn remove_at(dir: &Dir, name: &OsStr, folder: bool) -> io::Result<()> {
        let kind = if folder {
            FILE_DIRECTORY_FILE
        } else {
            FILE_NON_DIRECTORY_FILE
        };
        let entry = open_at(
            dir,
            name,
            DELETE | FILE_READ_ATTRIBUTES | SYNCHRONIZE,
            FILE_OPEN,
            kind | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT,
        )?;
        let handle = entry.as_raw_handle() as HANDLE;
        let posix = FILE_DISPOSITION_INFO_EX {
            Flags: FILE_DISPOSITION_FLAG_DELETE | FILE_DISPOSITION_FLAG_POSIX_SEMANTICS,
        };
        #[expect(
            unsafe_code,
            reason = "`SetFileInformationByHandle` with a disposition class is how a name opened \
                      beneath a handle is deleted. The handle is one this module owns, and each \
                      struct is a stack local whose size is passed beside it."
        )]
        unsafe {
            let mut ok = SetFileInformationByHandle(
                handle,
                FileDispositionInfoEx,
                (&raw const posix).cast(),
                u32::try_from(size_of::<FILE_DISPOSITION_INFO_EX>()).unwrap_or(u32::MAX),
            );
            // A file system with no POSIX delete, such as FAT, refuses the extended class, and the
            // plain one deletes the name once the handle closes.
            if ok == 0
                && matches!(
                    u32::try_from(io::Error::last_os_error().raw_os_error().unwrap_or(0)),
                    Ok(ERROR_INVALID_PARAMETER | ERROR_NOT_SUPPORTED | ERROR_INVALID_FUNCTION)
                )
            {
                let plain = FILE_DISPOSITION_INFO { DeleteFile: true };
                ok = SetFileInformationByHandle(
                    handle,
                    FileDispositionInfo,
                    (&raw const plain).cast(),
                    u32::try_from(size_of::<FILE_DISPOSITION_INFO>()).unwrap_or(u32::MAX),
                );
            }
            if ok == 0 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(())
    }

    pub(super) fn list(dir: &Dir) -> io::Result<Vec<super::Entry>> {
        // An empty name beneath a handle opens the handle's own folder again, here for listing.
        let listing = open_at(
            dir,
            OsStr::new(""),
            FILE_LIST_DIRECTORY | SYNCHRONIZE,
            FILE_OPEN,
            FILE_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT,
        )?;
        let handle = listing.as_raw_handle() as HANDLE;
        // `u64` cells, so every record is aligned for its 8-byte fields. 64 KiB per batch.
        let mut buffer = vec![0_u64; 8 * 1024];
        let bytes = u32::try_from(buffer.len() * 8).unwrap_or(u32::MAX);
        let mut class = FileFullDirectoryRestartInfo;
        let mut entries = Vec::new();
        loop {
            #[expect(
                unsafe_code,
                reason = "`GetFileInformationByHandleEx` with a directory class is how a folder is \
                          listed from a handle. The handle is one this module owns, and the buffer \
                          is a live allocation of exactly the size passed."
            )]
            let ok = unsafe {
                GetFileInformationByHandleEx(handle, class, buffer.as_mut_ptr().cast(), bytes)
            };
            if ok == 0 {
                let err = io::Error::last_os_error();
                if err.raw_os_error() == i32::try_from(ERROR_NO_MORE_FILES).ok() {
                    return Ok(entries);
                }
                return Err(err);
            }
            class = FileFullDirectoryInfo;
            let base: *const u8 = buffer.as_ptr().cast();
            let mut at = 0_usize;
            loop {
                #[expect(
                    unsafe_code,
                    reason = "each record starts inside the buffer the call filled, 8-byte \
                              aligned, at the offset the record before it names, and its name is \
                              `FileNameLength` bytes inside the same buffer. Every field is read \
                              unaligned all the same."
                )]
                let (next, attributes, name) = unsafe {
                    let record = base.add(at);
                    let field =
                        |offset: usize| std::ptr::read_unaligned(record.add(offset).cast::<u32>());
                    let next = field(std::mem::offset_of!(FILE_FULL_DIR_INFO, NextEntryOffset));
                    let attributes =
                        field(std::mem::offset_of!(FILE_FULL_DIR_INFO, FileAttributes));
                    let length = field(std::mem::offset_of!(FILE_FULL_DIR_INFO, FileNameLength));
                    let start = record
                        .add(std::mem::offset_of!(FILE_FULL_DIR_INFO, FileName))
                        .cast::<u16>();
                    let name: Vec<u16> = (0..length as usize / 2)
                        .map(|unit| std::ptr::read_unaligned(start.add(unit)))
                        .collect();
                    (next, attributes, name)
                };
                let name = std::ffi::OsString::from_wide(&name);
                if name != "." && name != ".." {
                    super::affords_entry(&entries, name.len())?;
                    let kind = kind_of(dir, &name, attributes);
                    entries.push(super::Entry { name, kind });
                }
                if next == 0 {
                    break;
                }
                at += next as usize;
            }
        }
    }

    /// What a listed name is. A reparse point is opened beneath the folder to read its tag, the
    /// way the walk reads one, and an entry that cannot be opened is a link only by its tag.
    fn kind_of(dir: &Dir, name: &OsStr, attributes: u32) -> super::Kind {
        let folder = attributes & FILE_ATTRIBUTE_DIRECTORY != 0;
        if attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            let surrogate = open_at(
                dir,
                name,
                FILE_READ_ATTRIBUTES | SYNCHRONIZE,
                FILE_OPEN,
                FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT,
            )
            .and_then(|entry| refuse_surrogate(&entry));
            if surrogate.is_err() {
                return super::Kind::Link;
            }
        }
        if folder {
            super::Kind::Dir
        } else if attributes & FILE_ATTRIBUTE_DEVICE != 0 {
            super::Kind::Other
        } else {
            super::Kind::File
        }
    }

    /// [`super::link_refused`] when `handle` is a symbolic link or a junction.
    fn refuse_surrogate(handle: &OwnedHandle) -> io::Result<()> {
        let mut info = FILE_ATTRIBUTE_TAG_INFO {
            FileAttributes: 0,
            ReparseTag: 0,
        };
        #[expect(
            unsafe_code,
            reason = "`GetFileInformationByHandleEx` is the only way to read a reparse tag off an \
                      open handle. The handle is one this module owns, and the out-parameter is a \
                      stack `FILE_ATTRIBUTE_TAG_INFO` this call owns exclusively, its size passed \
                      beside it."
        )]
        let ok = unsafe {
            GetFileInformationByHandleEx(
                handle.as_raw_handle() as HANDLE,
                FileAttributeTagInfo,
                (&raw mut info).cast(),
                u32::try_from(size_of::<FILE_ATTRIBUTE_TAG_INFO>()).unwrap_or(u32::MAX),
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        if info.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
            && info.ReparseTag & NAME_SURROGATE != 0
        {
            return Err(super::link_refused());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch folder, and its path as `fs::canonicalize` spells it. The folder is deleted when
    /// the guard drops, so a test keeps it alive until its last step.
    fn scratch(name: &str) -> (nvs_repo::Scratch, std::path::PathBuf) {
        let dir = nvs_repo::scratch(&format!("beneath-{name}"));
        let canonical = std::fs::canonicalize(&dir).expect("canonical scratch folder");
        (dir, canonical)
    }

    #[cfg_attr(
        miri,
        ignore = "opens a file with `openat`, which Miri does not emulate"
    )]
    #[test]
    fn creates_every_missing_folder_and_then_the_file() {
        let (_dir, root) = scratch("create");
        create_dirs(&root.join("a").join("b")).expect("folders");
        create_dirs(&root.join("a").join("b")).expect("folders that already exist");
        let mut file = create_file(&root.join("a").join("b").join("x.txt"), false).expect("file");
        io::Write::write_all(&mut file, b"one").expect("write");
        drop(file);
        let again = create_file(&root.join("a").join("b").join("x.txt"), false)
            .expect_err("an exclusive create of a name already there");
        assert_eq!(again.kind(), io::ErrorKind::AlreadyExists);
        drop(create_file(&root.join("a").join("b").join("x.txt"), true).expect("overwrite"));
        assert_eq!(
            std::fs::read(root.join("a").join("b").join("x.txt")).expect("read"),
            b""
        );
    }

    #[cfg_attr(
        miri,
        ignore = "opens a file with `openat`, which Miri does not emulate"
    )]
    #[test]
    fn append_writes_at_the_end_and_read_write_empties_nothing() {
        let (_dir, root) = scratch("modes");
        let path = root.join("log.txt");
        for line in [&b"one\n"[..], b"two\n"] {
            let mut file = open_file(&path, Mode::Append).expect("append");
            io::Write::write_all(&mut file, line).expect("write");
        }
        assert_eq!(std::fs::read(&path).expect("read"), b"one\ntwo\n");
        let mut file = open_file(&path, Mode::ReadWrite).expect("read and write");
        let mut text = String::new();
        io::Read::read_to_string(&mut file, &mut text).expect("read back");
        assert_eq!(text, "one\ntwo\n");
        drop(file);
    }

    #[cfg_attr(
        miri,
        ignore = "opens a file with `openat`, which Miri does not emulate"
    )]
    #[test]
    fn rename_moves_the_name_and_replaces_the_destination() {
        let (_dir, root) = scratch("rename");
        std::fs::create_dir_all(root.join("a")).expect("folder");
        std::fs::write(root.join("x.txt"), b"new").expect("source");
        std::fs::write(root.join("a").join("y.txt"), b"old").expect("destination");
        rename(&root.join("x.txt"), &root.join("a").join("y.txt")).expect("rename");
        assert!(!root.join("x.txt").exists());
        assert_eq!(
            std::fs::read(root.join("a").join("y.txt")).expect("read"),
            b"new"
        );
        rename(&root.join("a"), &root.join("b")).expect("a folder");
        assert!(root.join("b").join("y.txt").exists());
        let err = rename(&root.join("gone.txt"), &root.join("z.txt")).expect_err("no source");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[cfg_attr(
        miri,
        ignore = "opens a file with `openat`, which Miri does not emulate"
    )]
    #[test]
    fn write_mode_empties_nothing_and_same_file_sees_one_file() {
        let (_dir, root) = scratch("same");
        let path = root.join("s.txt");
        std::fs::write(&path, b"keep").expect("file");
        let written = open_file(&path, Mode::Write).expect("write");
        assert_eq!(std::fs::read(&path).expect("read"), b"keep");
        let read = File::open(root.join(".").join("s.txt")).expect("another spelling");
        assert!(same_file(&read, &written).expect("identity"));
        let other = open_file(&root.join("t.txt"), Mode::Write).expect("another file");
        assert!(!same_file(&read, &other).expect("identity"));
        drop((written, read, other));
    }

    #[cfg_attr(
        miri,
        ignore = "opens a file with `openat`, which Miri does not emulate"
    )]
    #[test]
    fn a_file_where_a_folder_is_needed_is_an_error() {
        let (_dir, root) = scratch("file");
        std::fs::write(root.join("f"), b"").expect("file");
        assert!(create_dirs(&root.join("f").join("g")).is_err());
        assert!(create_file(&root.join("f").join("g"), true).is_err());
    }

    /// A read opens what is there and creates nothing.
    #[cfg_attr(
        miri,
        ignore = "opens a file with `openat`, which Miri does not emulate"
    )]
    #[test]
    fn a_read_opens_what_is_there_and_creates_nothing() {
        use std::io::Read;
        let (_dir, root) = scratch("read");
        std::fs::create_dir(root.join("d")).expect("folder");
        std::fs::write(root.join("d").join("r.txt"), b"there").expect("file");
        let mut text = String::new();
        open_file(&root.join("d").join("r.txt"), Mode::Read)
            .expect("a read")
            .read_to_string(&mut text)
            .expect("the bytes");
        assert_eq!(text, "there");
        let err = open_file(&root.join("d").join("none.txt"), Mode::Read).expect_err("missing");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        assert!(!root.join("d").join("none.txt").exists());
    }

    #[cfg_attr(
        miri,
        ignore = "opens a file with `openat`, which Miri does not emulate"
    )]
    #[test]
    fn a_guest_open_takes_each_flag_and_a_folder_is_made_once() {
        let (_dir, root) = scratch("guest");
        let existing = Open {
            read: true,
            write: true,
            ..Open::default()
        };
        let err = open_with(&root.join("f.txt"), existing).expect_err("nothing there");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        let create = Open {
            create: true,
            ..existing
        };
        let mut file = open_with(&root.join("f.txt"), create).expect("created");
        std::io::Write::write_all(&mut file, b"abc").expect("write");
        drop(file);
        let truncate = Open {
            truncate: true,
            ..existing
        };
        drop(open_with(&root.join("f.txt"), truncate).expect("emptied"));
        assert_eq!(std::fs::read(root.join("f.txt")).expect("read"), b"");
        create_dir(&root.join("g")).expect("made");
        assert!(root.join("g").is_dir());
        let err = create_dir(&root.join("g")).expect_err("made twice");
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        let err = create_dir(&root.join("h").join("i")).expect_err("no parent");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[cfg_attr(
        miri,
        ignore = "opens a file with `openat`, which Miri does not emulate"
    )]
    #[test]
    fn removes_reads_and_lists_what_is_there() {
        let (_dir, root) = scratch("entries");
        std::fs::create_dir(root.join("d")).expect("folder");
        std::fs::write(root.join("d").join("f.txt"), b"four").expect("file");
        let mut listed = list(&root.join("d")).expect("a listing");
        listed.push(list(&root).expect("the root's listing").remove(0));
        assert_eq!(
            listed,
            [
                Entry {
                    name: "f.txt".into(),
                    kind: Kind::File
                },
                Entry {
                    name: "d".into(),
                    kind: Kind::Dir
                },
            ]
        );
        assert_eq!(
            metadata(&root.join("d").join("f.txt")).expect("stat").len(),
            4
        );
        assert!(metadata(&root.join("d")).expect("stat").is_dir());
        let err = metadata(&root.join("none")).expect_err("missing");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        assert!(
            remove_dir(&root.join("d")).is_err(),
            "a folder that is not empty"
        );
        assert!(
            remove_file(&root.join("d")).is_err(),
            "a folder is not a file"
        );
        remove_file(&root.join("d").join("f.txt")).expect("removed");
        assert!(list(&root.join("d")).expect("a listing").is_empty());
        remove_dir(&root.join("d")).expect("removed");
        assert!(!root.join("d").exists());
        let err = remove_file(&root.join("d")).expect_err("gone");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    /// CVE-2: a removal, a stat and a listing each resolve their path once and act beneath a
    /// handle, so a folder swapped for a link between the check and the act is not followed.
    #[cfg_attr(
        miri,
        ignore = "opens a file with `openat`, which Miri does not emulate"
    )]
    #[test]
    fn a_removal_a_stat_and_a_listing_never_follow_a_planted_link() {
        let (_dir, root) = scratch("swap");
        let (_outside_dir, outside) = scratch("swap-outside");
        std::fs::create_dir(outside.join("e")).expect("outside folder");
        std::fs::write(outside.join("secret.txt"), b"secret").expect("outside file");
        // The check saw a real folder at `d`, and then `d` became a link.
        std::fs::create_dir(root.join("d")).expect("checked folder");
        std::fs::remove_dir(root.join("d")).expect("swapped out");
        let planted = plant_dir_link(&outside, &root.join("d"));
        assert!(planted, "a symbolic link on Unix, or a junction on Windows");
        if planted {
            let through = root.join("d").join("secret.txt");
            let err = remove_file(&through).expect_err("a removal through a link");
            assert_eq!(err.to_string(), link_refused().to_string());
            let err = remove_dir(&root.join("d").join("e")).expect_err("a removal through a link");
            assert_eq!(err.to_string(), link_refused().to_string());
            let err = metadata(&through).expect_err("a stat through a link");
            assert_eq!(err.to_string(), link_refused().to_string());
            let err = entry_metadata(&through).expect_err("a stat through a link");
            assert_eq!(err.to_string(), link_refused().to_string());
            let err = list(&root.join("d")).expect_err("a listing of a link");
            assert_eq!(err.to_string(), link_refused().to_string());
            assert!(outside.join("secret.txt").exists());
            assert!(outside.join("e").exists());
            // The link at the last name: a stat does not follow it, a stat of the entry reads it,
            // a listing names it, and a removal takes the link and leaves its target.
            let err = metadata(&root.join("d")).expect_err("a link at the last name");
            assert_eq!(err.kind(), io::ErrorKind::NotFound);
            assert!(
                entry_metadata(&root.join("d"))
                    .expect("the link")
                    .is_symlink()
            );
            assert_eq!(
                list(&root).expect("the root's listing"),
                [Entry {
                    name: "d".into(),
                    kind: Kind::Link
                }]
            );
            #[cfg(unix)]
            remove_file(&root.join("d")).expect("the link removed");
            #[cfg(windows)]
            remove_dir(&root.join("d")).expect("the junction removed");
            assert!(!root.join("d").exists());
            assert!(outside.join("secret.txt").exists());
        }
    }

    /// A link to `target` at `at`. On Windows a symbolic link needs a right most accounts lack, and
    /// a junction, which needs none, is planted in its place.
    fn plant_dir_link(target: &Path, at: &Path) -> bool {
        #[cfg(unix)]
        return std::os::unix::fs::symlink(target, at).is_ok();
        #[cfg(windows)]
        return std::os::windows::fs::symlink_dir(target, at).is_ok()
            || nvs_repo::spawn("cmd", &[])
                .arg("/C")
                .arg("mklink")
                .arg("/J")
                .arg(at)
                .arg(target)
                .output()
                .is_ok_and(|out| out.status.success());
    }

    /// A link is planted where a folder of the path was.
    #[cfg_attr(
        miri,
        ignore = "opens a file with `openat`, which Miri does not emulate"
    )]
    #[test]
    fn a_planted_link_is_never_followed() {
        let (_dir, root) = scratch("link");
        let (_outside_dir, outside) = scratch("link-outside");
        let planted = plant_dir_link(&outside, &root.join("d"));
        assert!(planted, "a symbolic link on Unix, or a junction on Windows");
        if planted {
            let err = create_file(&root.join("d").join("x.txt"), true).expect_err("through a link");
            assert_eq!(err.to_string(), link_refused().to_string());
            let err = create_dirs(&root.join("d").join("e")).expect_err("through a link");
            assert_eq!(err.to_string(), link_refused().to_string());
            // A guest's folder and its exclusive read-write create.
            let err = create_dir(&root.join("d").join("g")).expect_err("through a link");
            assert_eq!(err.to_string(), link_refused().to_string());
            let how = Open {
                read: true,
                write: true,
                create: true,
                exclusive: true,
                ..Open::default()
            };
            let err = open_with(&root.join("d").join("w.txt"), how).expect_err("through a link");
            assert_eq!(err.to_string(), link_refused().to_string());
            assert!(!outside.join("g").exists());
            assert!(!outside.join("w.txt").exists());
            // The link itself is a name already there.
            let err = create_dir(&root.join("d")).expect_err("a link as the folder");
            assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
            // The copy door's destination.
            let err = open_file(&root.join("d").join("c.txt"), Mode::Write)
                .expect_err("a copy through a link");
            assert_eq!(err.to_string(), link_refused().to_string());
            // The read doors, on a file that is there beyond the link.
            std::fs::write(outside.join("r.txt"), b"secret").expect("outside file");
            let err = open_file(&root.join("d").join("r.txt"), Mode::Read)
                .expect_err("a read through a link");
            assert_eq!(err.to_string(), link_refused().to_string());
            // The rename door, with the link above either end.
            std::fs::write(root.join("m.txt"), b"move").expect("source");
            let err = rename(&root.join("m.txt"), &root.join("d").join("m.txt"))
                .expect_err("a rename into a link");
            assert_eq!(err.to_string(), link_refused().to_string());
            std::fs::write(outside.join("n.txt"), b"stay").expect("outside source");
            let err = rename(&root.join("d").join("n.txt"), &root.join("n.txt"))
                .expect_err("a rename out of a link");
            assert_eq!(err.to_string(), link_refused().to_string());
            assert!(!outside.join("x.txt").exists());
            assert!(!outside.join("e").exists());
            assert!(!outside.join("c.txt").exists());
            assert!(!outside.join("m.txt").exists());
            assert!(outside.join("n.txt").exists());
            assert!(root.join("m.txt").exists());

            #[cfg(unix)]
            let file_link = std::os::unix::fs::symlink(outside.join("y.txt"), root.join("y.txt"));
            #[cfg(windows)]
            let file_link =
                std::os::windows::fs::symlink_file(outside.join("y.txt"), root.join("y.txt"));
            std::fs::write(outside.join("y.txt"), b"keep").expect("target");
            if file_link.is_ok() {
                let err = create_file(&root.join("y.txt"), true).expect_err("a link as the file");
                assert_eq!(err.to_string(), link_refused().to_string());
                assert_eq!(std::fs::read(outside.join("y.txt")).expect("read"), b"keep");
            }
        }
    }
}
