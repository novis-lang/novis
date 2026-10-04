//! `rule:security/writes-open-beneath-a-handle`'s walk: a resolved path opened one level at a time,
//! each level from the handle of the one above it, following no link on the way.
//!
//! The write doors in [`crate::capability`] resolve a path once, check the grant on that resolved
//! path, and hand it here. A resolved path names no link, so a link met on the walk is one that was
//! planted after the check, and it fails the call with [`link_refused`]. Nothing is opened by its
//! whole path a second time, which is the window a check-then-open pair leaves.
//!
//! **Unix** opens each folder with `openat` and `O_NOFOLLOW | O_DIRECTORY`, creates one with
//! `mkdirat`, opens the file with `openat` and `O_NOFOLLOW`, and renames with `renameat` between
//! two walked folders. On Linux a folder is opened `O_PATH`, so a folder the process may search but
//! not list is still walked.
//!
//! **Windows** opens each level with `NtCreateFile`, its `RootDirectory` set to the folder above and
//! `FILE_OPEN_REPARSE_POINT` set, then reads the level's reparse tag. A name surrogate — a symbolic
//! link or a junction — is refused. Any other reparse point, such as a cloud placeholder, is a real
//! folder and is walked. A rename opens the source the same way and renames its handle with
//! `NtSetInformationFile`, the destination folder's handle as the new name's root.
//!
//! **Cost:** one open per level of the path, the same count the operating system's own lookup
//! makes, plus one attribute read per level on Windows. Every handle but the deepest is closed as
//! soon as the next one is open.

use std::fs::File;
use std::io;
use std::path::{Component, Path};

/// The error a link on the walk fails with, the same sentence on every platform.
fn link_refused() -> io::Error {
    io::Error::other("a folder in this path was replaced by a link, and a write follows no link")
}

/// How [`open_file`] opens the file at the end of the walk. Every mode creates the file when it is
/// not there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
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

/// The file at `path`, opened as `mode` says.
///
/// `path` is a resolved, absolute path. Its folders must exist.
///
/// # Errors
///
/// [`link_refused`] when a level of `path`, the file included, is a link. Otherwise the operating
/// system's error, which is `AlreadyExists` for [`Mode::New`] on a name already there.
pub(crate) fn open_file(path: &Path, mode: Mode) -> io::Result<File> {
    let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
        return Err(io::Error::from(io::ErrorKind::InvalidInput));
    };
    let dir = walk(parent, false)?;
    sys::open_at_end(&dir, name, mode)
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
pub(crate) fn rename(from: &Path, to: &Path) -> io::Result<()> {
    let (Some(from_parent), Some(from_name), Some(to_parent), Some(to_name)) =
        (from.parent(), from.file_name(), to.parent(), to.file_name())
    else {
        return Err(io::Error::from(io::ErrorKind::InvalidInput));
    };
    let from_dir = walk(from_parent, false)?;
    let to_dir = walk(to_parent, false)?;
    sys::rename_at(&from_dir, from_name, &to_dir, to_name)
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
                make_at(dir, &name)?;
                open_at(dir.as_raw_fd(), &name, DIR_FLAGS, 0)
            }
            other => other,
        }
        .map_err(|err| explain(dir, &name, err))
    }

    pub(super) fn open_at_end(dir: &Dir, name: &OsStr, mode: super::Mode) -> io::Result<File> {
        let name = c_name(name)?;
        let mode = match mode {
            super::Mode::New => libc::O_WRONLY | libc::O_EXCL,
            super::Mode::Truncate => libc::O_WRONLY | libc::O_TRUNC,
            super::Mode::Write => libc::O_WRONLY,
            super::Mode::Append => libc::O_WRONLY | libc::O_APPEND,
            super::Mode::ReadWrite => libc::O_RDWR,
        };
        let flags = libc::O_CREAT | libc::O_NOFOLLOW | libc::O_CLOEXEC | mode;
        open_at(dir.as_raw_fd(), &name, flags, 0o666)
            .map(File::from)
            .map_err(|err| explain(dir, &name, err))
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
            return Ok(());
        }
        let err = io::Error::last_os_error();
        if err.kind() == io::ErrorKind::AlreadyExists {
            Ok(())
        } else {
            Err(err)
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
    use std::os::windows::ffi::OsStrExt;
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
        ERROR_INVALID_FUNCTION, ERROR_INVALID_PARAMETER, ERROR_NOT_SUPPORTED, HANDLE,
        OBJ_CASE_INSENSITIVE, RtlNtStatusToDosError, UNICODE_STRING,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, DELETE, FILE_ATTRIBUTE_NORMAL, FILE_ATTRIBUTE_REPARSE_POINT,
        FILE_ATTRIBUTE_TAG_INFO, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
        FILE_GENERIC_READ, FILE_GENERIC_WRITE, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE,
        FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_TRAVERSE, FILE_WRITE_DATA, FileAttributeTagInfo,
        GetFileInformationByHandle, GetFileInformationByHandleEx, SYNCHRONIZE,
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

    pub(super) fn open_at_end(dir: &Dir, name: &OsStr, mode: super::Mode) -> io::Result<File> {
        // `FILE_CREATE` fails on any name already there, a link included. Every other mode opens
        // what is there without emptying it, so a link is refused before it loses anything, and a
        // truncate empties the file afterwards. A handle without `FILE_WRITE_DATA` writes only at
        // the end, which is how `std` appends too.
        let (access, disposition) = match mode {
            super::Mode::New => (FILE_GENERIC_WRITE, FILE_CREATE),
            super::Mode::Truncate | super::Mode::Write => (FILE_GENERIC_WRITE, FILE_OPEN_IF),
            super::Mode::Append => (FILE_GENERIC_WRITE & !FILE_WRITE_DATA, FILE_OPEN_IF),
            super::Mode::ReadWrite => (FILE_GENERIC_READ | FILE_GENERIC_WRITE, FILE_OPEN_IF),
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
        if mode == super::Mode::Truncate {
            file.set_len(0)?;
        }
        Ok(file)
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

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("nvs-beneath-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch folder");
        std::fs::canonicalize(&dir).expect("canonical scratch folder")
    }

    #[test]
    fn creates_every_missing_folder_and_then_the_file() {
        let root = scratch("create");
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
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn append_writes_at_the_end_and_read_write_empties_nothing() {
        let root = scratch("modes");
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
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn rename_moves_the_name_and_replaces_the_destination() {
        let root = scratch("rename");
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
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn write_mode_empties_nothing_and_same_file_sees_one_file() {
        let root = scratch("same");
        let path = root.join("s.txt");
        std::fs::write(&path, b"keep").expect("file");
        let written = open_file(&path, Mode::Write).expect("write");
        assert_eq!(std::fs::read(&path).expect("read"), b"keep");
        let read = File::open(root.join(".").join("s.txt")).expect("another spelling");
        assert!(same_file(&read, &written).expect("identity"));
        let other = open_file(&root.join("t.txt"), Mode::Write).expect("another file");
        assert!(!same_file(&read, &other).expect("identity"));
        drop((written, read, other));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_file_where_a_folder_is_needed_is_an_error() {
        let root = scratch("file");
        std::fs::write(root.join("f"), b"").expect("file");
        assert!(create_dirs(&root.join("f").join("g")).is_err());
        assert!(create_file(&root.join("f").join("g"), true).is_err());
        let _ = std::fs::remove_dir_all(&root);
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
    #[test]
    fn a_planted_link_is_never_followed() {
        let root = scratch("link");
        let outside = scratch("link-outside");
        let planted = plant_dir_link(&outside, &root.join("d"));
        assert!(planted, "a symbolic link on Unix, or a junction on Windows");
        if planted {
            let err = create_file(&root.join("d").join("x.txt"), true).expect_err("through a link");
            assert_eq!(err.to_string(), link_refused().to_string());
            let err = create_dirs(&root.join("d").join("e")).expect_err("through a link");
            assert_eq!(err.to_string(), link_refused().to_string());
            // The copy door's destination.
            let err = open_file(&root.join("d").join("c.txt"), Mode::Write)
                .expect_err("a copy through a link");
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
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&outside);
    }
}
