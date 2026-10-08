//! The data folder: the one directory Novis keeps its own files in.
//!
//! It is `.nvsdata` beside the running binary — [`std::env::current_exe`], canonicalized through
//! [`trust::canonical`], its parent — unless the command line names another one, and a relative
//! name resolves against the working directory like every other command-line path. Inside it are
//! [`Folder::config_file`], [`Folder::cache`], [`Folder::tmp`] and [`Folder::lsp`]; each of those
//! has a configuration key that moves it, and this module only owns the default.
//!
//! **One value per process.** The CLI names the folder once at startup through [`set`], and every
//! reader asks [`current`]. A process that never set one — an in-process test, a library caller —
//! gets the folder beside its own binary, computed the first time anything asks. A failed [`prepare`]
//! withdraws it: from then on [`current`] is `None`, so nothing writes into a folder that could not
//! be made or failed `rule:config/ownership-is-the-trust-boundary`'s check.
//!
//! **Private creation is written once, here**, and the runtime's temporary directories use it too
//! ([`create_private_root`], [`create_private_dir`]). On Unix every directory it creates is `0700`
//! from the moment it exists. On Windows the topmost directory a [`create_private_root`] call creates
//! gets a protected DACL — full control for this account, `SYSTEM` and `Administrators`, inherited by
//! everything below it — and every directory under it inherits that. Only the top one carries its
//! own DACL because a grant added to the root later, such as the service installer's grant to the
//! service account, has to reach the subfolders, and a protected DACL on each of them would block
//! it. A directory that already exists is never re-permissioned; [`prepare`] checks it instead.
//!
//! Cost: one `current_exe` and one canonicalization per process, and at [`prepare`] one create
//! attempt per folder plus the trust check's two reads. Nothing here runs per request except
//! [`create_private_dir`], which is the one create a temporary directory always cost.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::trust;

/// The data folder's name beside the binary.
pub const NAME: &str = ".nvsdata";

/// One data folder, and the paths inside it. Nothing here touches the disk except [`Folder::prepare`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Folder {
    root: PathBuf,
}

impl Folder {
    /// The data folder at `root`, as given.
    #[must_use]
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// The folder the command line's `flag` names, resolved against `cwd` when it is relative, or
    /// [`Folder::beside_exe`] when there is no flag.
    ///
    /// # Errors
    ///
    /// [`Folder::beside_exe`]'s, and only when `flag` is `None`.
    pub fn locate(flag: Option<&Path>, cwd: &Path) -> std::io::Result<Self> {
        match flag {
            Some(flag) => Ok(Self::new(cwd.join(flag))),
            None => Self::beside_exe(),
        }
    }

    /// `.nvsdata` in the directory of the running binary, with symlinks resolved, so a binary
    /// reached through a link in `/usr/local/bin` uses the folder beside the file it really is.
    ///
    /// # Errors
    ///
    /// When the running binary's path cannot be read or canonicalized.
    pub fn beside_exe() -> std::io::Result<Self> {
        let exe = trust::canonical(&std::env::current_exe()?)?;
        let dir = exe.parent().ok_or_else(|| {
            std::io::Error::other(format!("`{}` has no parent folder", exe.display()))
        })?;
        Ok(Self::new(dir.join(NAME)))
    }

    /// The folder itself.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `nvs.toml`: the configuration file read when the working directory has none.
    #[must_use]
    pub fn config_file(&self) -> PathBuf {
        self.root.join(crate::resolve::LOCAL_FILE)
    }

    /// `cache/`: the compiled-code cache when `[opcache] file_cache_dir` names none.
    #[must_use]
    pub fn cache(&self) -> PathBuf {
        self.root.join("cache")
    }

    /// `tmp/`: the temporary-directory root when `[io] temp_root` names none.
    #[must_use]
    pub fn tmp(&self) -> PathBuf {
        self.root.join("tmp")
    }

    /// `lsp/<version>/`: the language server's stub tree when the client names none. One per
    /// server version, so two installed versions never read each other's stubs; `version` is the
    /// caller's own `CARGO_PKG_VERSION`.
    #[must_use]
    pub fn lsp(&self, version: &str) -> PathBuf {
        self.lsp_root().join(version)
    }

    /// `lsp/`, which [`Folder::prepare`] creates; the versioned tree under it is the server's to write.
    fn lsp_root(&self) -> PathBuf {
        self.root.join("lsp")
    }

    /// Creates the folder and `cache/`, `tmp/` and `lsp/` in it, privately, and checks the folder
    /// with [`trust::check`]. Anything that already exists is left as it is.
    ///
    /// The check runs after the root exists and before anything is created inside it, so a folder
    /// another account can write never gains subfolders from this call. It does not write
    /// `nvs.toml`.
    ///
    /// # Errors
    ///
    /// [`Unusable`] naming this folder, when a create failed or the check did.
    pub fn prepare(&self) -> Result<(), Unusable> {
        let unusable = |reason: String| Unusable {
            folder: Some(self.root.clone()),
            reason,
        };
        create_private_root(&self.root).map_err(|err| unusable(err.to_string()))?;
        trust::check(&self.root).map_err(|why| unusable(why.message().to_owned()))?;
        for sub in [self.cache(), self.tmp(), self.lsp_root()] {
            create_private_root(&sub)
                .map_err(|err| unusable(format!("{}: {err}", sub.display())))?;
        }
        Ok(())
    }
}

/// Why the data folder cannot be used. The caller prints [`Unusable::warning`] once and carries on:
/// an unusable data folder never stops a command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unusable {
    /// `None` when the folder could not even be located.
    folder: Option<PathBuf>,
    reason: String,
}

impl Unusable {
    /// The folder, when it was located.
    #[must_use]
    pub fn folder(&self) -> Option<&Path> {
        self.folder.as_deref()
    }

    /// What went wrong, in the words of the call that failed.
    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }

    /// The one warning a command prints for this, with no trailing newline.
    #[must_use]
    pub fn warning(&self) -> String {
        let head = match &self.folder {
            Some(folder) => format!(
                "warning: Novis cannot use its data folder `{}`: {}",
                folder.display(),
                self.reason
            ),
            None => format!(
                "warning: Novis cannot find its data folder: {}",
                self.reason
            ),
        };
        format!(
            "{head}\n  \
             note: programs run without the compile cache, and `Core\\IO::temporaryDir` throws an \
             error.\n  \
             help: pass `--data <folder>` to use a folder that only this account can change."
        )
    }
}

/// The process's data folder, or the reason there is none.
static CURRENT: OnceLock<Result<Folder, String>> = OnceLock::new();

/// Set by a failed [`prepare`]: the folder exists in [`CURRENT`] but must not be used.
static WITHDRAWN: AtomicBool = AtomicBool::new(false);

/// Names this process's data folder. The CLI calls it once, at startup, before anything reads it.
///
/// Returns `false` and changes nothing when a folder was already set, or already read through
/// [`current`] or [`prepare`]: a folder some code has used is the folder for the whole process.
#[must_use]
pub fn set(folder: Folder) -> bool {
    CURRENT.set(Ok(folder)).is_ok()
}

/// This process's data folder: the one [`set`] named, else [`Folder::beside_exe`].
///
/// `None` when the binary's own path could not be read, or after [`prepare`] found the folder
/// unusable.
#[must_use]
pub fn current() -> Option<&'static Folder> {
    if WITHDRAWN.load(Ordering::Relaxed) {
        return None;
    }
    located().as_ref().ok()
}

/// [`CURRENT`], computed on first use.
fn located() -> &'static Result<Folder, String> {
    CURRENT.get_or_init(|| Folder::beside_exe().map_err(|err| err.to_string()))
}

/// [`Folder::prepare`] on this process's data folder. A failure withdraws the folder, so [`current`]
/// is `None` from then on.
///
/// # Errors
///
/// [`Unusable`], for the caller to print as one warning.
pub fn prepare() -> Result<&'static Folder, Unusable> {
    let folder = located().as_ref().map_err(|reason| Unusable {
        folder: None,
        reason: reason.clone(),
    })?;
    folder.prepare().map(|()| folder).inspect_err(|_| {
        WITHDRAWN.store(true, Ordering::Relaxed);
    })
}

/// Creates `path` and every missing directory above it, privately; an existing directory is
/// success and is left as it is.
///
/// On Unix each directory this creates is `0700` from the moment it exists. On Windows the topmost
/// one it creates gets the protected DACL the module doc describes, and the rest inherit it.
///
/// # Errors
///
/// The error of the create that failed, including a path that exists and is not a directory.
pub fn create_private_root(path: &Path) -> std::io::Result<()> {
    let mut missing = Vec::new();
    let mut at = Some(path);
    while let Some(dir) = at {
        if dir.as_os_str().is_empty() || dir.is_dir() {
            break;
        }
        missing.push(dir);
        at = dir.parent();
    }
    for (index, dir) in missing.iter().rev().enumerate() {
        match platform::create(dir, index == 0) {
            Ok(()) => {}
            // Another process made it between the walk and this create.
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists && dir.is_dir() => {}
            Err(err) => return Err(err),
        }
    }
    Ok(())
}

/// Creates the one directory `path`, whose parent exists. On Unix it is `0700` from the moment it
/// exists; on Windows it inherits its parent's DACL.
///
/// # Errors
///
/// The create's error, and `AlreadyExists` when anything is at `path` already, which is what lets
/// a caller choosing a fresh name retry with another.
pub fn create_private_dir(path: &Path) -> std::io::Result<()> {
    platform::create(path, false)
}

#[cfg(unix)]
mod platform {
    use std::path::Path;

    /// One directory, `0700` at creation. `_top` is a Windows distinction.
    pub(super) fn create(path: &Path, _top: bool) -> std::io::Result<()> {
        use std::os::unix::fs::DirBuilderExt;

        std::fs::DirBuilder::new().mode(0o700).create(path)
    }
}

#[cfg(windows)]
mod platform {
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use windows_sys::Win32::Foundation::{HLOCAL, LocalFree};
    use windows_sys::Win32::Security::Authorization::{
        ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
    };
    use windows_sys::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
    use windows_sys::Win32::Storage::FileSystem::CreateDirectoryW;

    /// A security descriptor built from SDDL, freed with `LocalFree` on every path out.
    #[derive(Debug)]
    struct Descriptor(PSECURITY_DESCRIPTOR);

    impl Descriptor {
        /// The descriptor `sddl` describes.
        ///
        /// # Errors
        ///
        /// The OS error when `sddl` does not parse.
        #[expect(
            unsafe_code,
            reason = "one `advapi32` call over a NUL-terminated buffer this frame owns; the \
                      descriptor it allocates is owned by the returned guard, which frees it"
        )]
        fn from_sddl(sddl: &str) -> std::io::Result<Self> {
            let wide: Vec<u16> = sddl.encode_utf16().chain(Some(0)).collect();
            let mut raw: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
            let built = unsafe {
                ConvertStringSecurityDescriptorToSecurityDescriptorW(
                    wide.as_ptr(),
                    SDDL_REVISION_1,
                    &mut raw,
                    std::ptr::null_mut(),
                )
            };
            if built == 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(Self(raw))
        }

        /// The attributes a create call takes, pointing at this descriptor. They must not outlive it.
        fn attributes(&self) -> SECURITY_ATTRIBUTES {
            SECURITY_ATTRIBUTES {
                nLength: u32::try_from(std::mem::size_of::<SECURITY_ATTRIBUTES>())
                    .expect("a Win32 struct is far smaller than a `u32` counts"),
                lpSecurityDescriptor: self.0,
                bInheritHandle: 0,
            }
        }
    }

    impl Drop for Descriptor {
        #[expect(
            unsafe_code,
            reason = "`ConvertStringSecurityDescriptorToSecurityDescriptorW` allocates with \
                      `LocalAlloc`, so this is the documented way to release it"
        )]
        fn drop(&mut self) {
            unsafe { LocalFree(self.0 as HLOCAL) };
        }
    }

    /// One directory: with the protected, inheritable DACL when it is the `top` of what a call
    /// creates, else inheriting its parent's.
    ///
    /// The DACL is set by the create itself, so there is no moment in which the directory exists
    /// with the DACL it would have inherited. The SID and never an account name, because a name
    /// is localized.
    #[expect(
        unsafe_code,
        reason = "one `CreateDirectoryW` over a NUL-terminated name this frame owns and attributes \
                  whose descriptor is owned by a guard that outlives the call"
    )]
    pub(super) fn create(path: &Path, top: bool) -> std::io::Result<()> {
        if !top {
            return std::fs::create_dir(path);
        }
        let sid = crate::trust::owner_sid().ok_or_else(|| {
            std::io::Error::other(
                "this process's own account could not be read from its token, so a folder only \
                 it can reach cannot be described",
            )
        })?;
        let descriptor = Descriptor::from_sddl(&format!(
            "D:P(A;OICI;FA;;;{sid})(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)"
        ))?;
        let attributes = descriptor.attributes();
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let made = unsafe { CreateDirectoryW(wide.as_ptr(), &attributes) };
        drop(descriptor);
        if made == 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{Folder, NAME};
    use crate::trust;

    fn scratch(name: &str) -> nvs_repo::Scratch {
        nvs_repo::scratch(&format!("data-{name}"))
    }

    #[test]
    fn a_relative_flag_resolves_against_the_working_directory() {
        let cwd = scratch("relative");
        let folder = Folder::locate(Some(Path::new("data")), &cwd).expect("a flag never fails");
        assert_eq!(folder.root(), cwd.join("data"));
    }

    #[test]
    fn an_absolute_flag_is_used_as_it_is() {
        let cwd = scratch("cwd");
        let elsewhere = scratch("absolute");
        let folder = Folder::locate(Some(&elsewhere), &cwd).expect("a flag never fails");
        assert_eq!(folder.root(), elsewhere.path());
    }

    #[test]
    fn with_no_flag_the_folder_is_beside_the_running_binary() {
        let exe = trust::canonical(&std::env::current_exe().unwrap()).unwrap();
        let beside = exe.parent().unwrap().join(NAME);
        let cwd = scratch("no-flag");
        assert_eq!(Folder::locate(None, &cwd).unwrap().root(), beside);
        assert_eq!(
            super::current().map(Folder::root),
            Some(beside.as_path()),
            "a process that never set a folder reads the one beside its binary"
        );
    }

    #[test]
    fn the_paths_are_inside_the_folder() {
        let folder = Folder::new(PathBuf::from("data"));
        let root = Path::new("data");
        assert_eq!(folder.config_file(), root.join("nvs.toml"));
        assert_eq!(folder.cache(), root.join("cache"));
        assert_eq!(folder.tmp(), root.join("tmp"));
        assert_eq!(folder.lsp("1.2.3"), root.join("lsp").join("1.2.3"));
    }

    /// The folder and the directory above it are both created here, because the trust check reads
    /// the parent too and a scratch directory under `target/` may inherit a group's write grant.
    #[test]
    fn prepare_creates_private_folders_that_pass_the_trust_check() {
        let dir = scratch("prepare");
        let folder = Folder::new(dir.join("bin").join(NAME));

        folder.prepare().expect("a fresh folder is usable");
        folder
            .prepare()
            .expect("and an existing one is left as it is");

        let made = [
            dir.join("bin"),
            folder.root().to_path_buf(),
            folder.cache(),
            folder.tmp(),
            folder.root().join("lsp"),
        ];
        for path in &made {
            assert!(path.is_dir(), "{} was created", path.display());
        }
        trust::check(folder.root()).expect("the folder and its parent pass the trust check");
        trust::check(&folder.cache()).expect("and so does a subfolder");
        #[cfg(unix)]
        for path in &made {
            use std::os::unix::fs::PermissionsExt;

            let mode = std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o700, "{} is private", path.display());
        }
        // The DACL read back: no group or world principal may read what the folder contains,
        // which an inherited DACL on a drive's ordinary folders would allow.
        #[cfg(windows)]
        for path in &made {
            assert_eq!(trust::exposure(path), None, "{} is private", path.display());
        }
    }

    #[test]
    fn a_folder_that_cannot_be_created_gives_one_warning() {
        let dir = scratch("blocked");
        let blocker = dir.join("file");
        std::fs::write(&blocker, "").unwrap();
        let folder = Folder::new(blocker.join(NAME));

        let unusable = folder.prepare().expect_err("a file is in the way");

        assert_eq!(unusable.folder(), Some(folder.root()));
        let warning = unusable.warning();
        assert!(warning.starts_with("warning: "), "{warning}");
        assert!(
            warning.contains(&folder.root().display().to_string()),
            "{warning}"
        );
        assert!(warning.contains("--data <folder>"), "{warning}");
        assert!(!blocker.join(NAME).exists());
    }

    /// An existing folder is checked and never re-permissioned.
    #[cfg(unix)]
    #[test]
    fn an_existing_folder_others_can_write_is_unusable_and_left_alone() {
        use std::os::unix::fs::PermissionsExt;

        let dir = scratch("shared");
        let parent = dir.join("bin");
        super::create_private_root(&parent).unwrap();
        let folder = Folder::new(parent.join(NAME));
        std::fs::create_dir(folder.root()).unwrap();
        std::fs::set_permissions(folder.root(), std::fs::Permissions::from_mode(0o777)).unwrap();

        let unusable = folder
            .prepare()
            .expect_err("a world-writable folder fails the check");

        assert!(
            unusable.reason().contains("writable"),
            "{}",
            unusable.reason()
        );
        let mode = std::fs::metadata(folder.root())
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o777, "the folder's mode is not changed");
        assert!(!folder.cache().exists(), "nothing is created inside it");
    }
}
