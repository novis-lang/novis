//! `rule:config/ownership-is-the-trust-boundary`'s trust boundary: whether any account but this one can write a file the
//! configuration reads.
//!
//! The configuration grants capabilities, so whoever can write **any** file in the tree can grant
//! themselves every one of them. `rule:config/ownership-is-the-trust-boundary` answers that with one rule and no exceptions: every file must
//! be owned by the account the runtime runs as or by root, and writable by nobody else — and so
//! must the directory that contains it, because a directory its owner can write is a standing slot
//! that owner may fill later. That is why the containing directory is checked for its **owner** as
//! well as its mode, and why an absent `optional` include is checked at all rather than skipped:
//! [`resolve`](crate::resolve)'s `include_targets` puts the check on the directory the file would
//! appear in, which is the only place a promise about a file that does not exist yet can be kept.
//!
//! **Unix is the mode bits. Windows is the DACL**, and `rule:config/ownership-is-the-trust-boundary`'s answer for which ACEs it accepts is
//! implemented here: the owner must be this account,
//! `BUILTIN\Administrators` or `NT AUTHORITY\SYSTEM`, and no *effective* write right may reach
//! `Everyone`, `NT AUTHORITY\Authenticated Users`, `BUILTIN\Users`, `BUILTIN\Guests` or
//! `ANONYMOUS LOGON`. Effective rather than by ACE, so a deny entry counts, an inherited grant is
//! seen, and the question asked is the one that matters — can that principal write this file —
//! rather than how the ACL happens to be spelled. Those SIDs are the Windows spelling of "the
//! group and the world"; the write rights are every one that changes the bytes, the name or the
//! ACL itself, `WRITE_DAC` and `WRITE_OWNER` included, because either of those buys the rest.
//! `platform::effective_rights` computes that in one pass over the DACL rather than through
//! `GetEffectiveRightsFromAclW`, and its own doc owns why — the answer is the same one, and the
//! call it stands in for is orders of magnitude dearer per principal.
//!
//! **Canonicalization is part of the check, not a side effect of it.** [`check`] returns the
//! canonical path, and that is what makes the resolver's cycle test compare files rather than
//! spellings — a cycle assembled out of symlinks is invisible to a lexical comparison. It is also
//! the one path comparison `rule:config/every-matching-app-block-applies-least-specific-first`'s `[[app]]` matching is built on, so it is written once
//! here rather than at each site that needs it.
//!
//! **[`exposure`] is the same question asked about reading, and it only ever advises.** `rule:config/a-secret-file-is-checked-for-integrity-and-advised-on-exposure` refuses
//! a secret file another account can write and warns about one another account can read, because a
//! Compose secret is mounted `0444` and a Kubernetes secret volume defaults to `0644`: the mode
//! that is a mistake on a shared host is the norm inside a container, and nothing readable from
//! here says which one this is. Integrity is enforced; confidentiality is advised.
//!
//! Cost: two `stat`s per file on Unix, two security-descriptor reads and one walk of each DACL on
//! Windows, at boot and again at each reload, plus one more of either for each secret
//! file's advisory. Nothing here runs per request. `rule:packaging/an-artifact-is-one-immutable-content-addressed-file`'s artifact cache is the one caller
//! outside boot — it checks its own directory once per process, which is why the Windows half is
//! kept cheap enough to disappear beside the work around it.
//!

use std::path::{Path, PathBuf};

/// Why a path the configuration names is not trusted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Untrusted {
    /// It could not be examined at all: it is absent, or something above it refuses to be read.
    /// The resolver reports this as `E0605`, exactly as it reports any other file it cannot read,
    /// and the message is the underlying reader's own.
    Unreadable(String),
    /// It was examined and it fails `rule:config/ownership-is-the-trust-boundary`.
    Breach {
        /// The path and how it fails, in the platform's own terms: a mode and a gid on Unix, a
        /// principal and a right mask on Windows.
        message: String,
        /// The command that repairs this one failure, with the path, the owner or the group's SID
        /// filled in, so an operator can copy it. Built where the failure is found, because only
        /// there is it known which of the repairs applies.
        help: String,
    },
}

impl Untrusted {
    /// The message, whichever half this is.
    #[must_use]
    pub fn message(&self) -> &str {
        match self {
            Self::Unreadable(message) | Self::Breach { message, .. } => message,
        }
    }

    /// The repair a [`Breach`](Self::Breach) prints, and `None` for a path that could not be
    /// examined: a repair answers a DACL or a mode that was read.
    #[must_use]
    pub fn help(&self) -> Option<&str> {
        match self {
            Self::Breach { help, .. } => Some(help),
            Self::Unreadable(_) => None,
        }
    }

    /// A breach of `message` that `help` repairs.
    pub(crate) fn breach(message: String, help: String) -> Self {
        Self::Breach { message, help }
    }

    /// The path a [`Breach`](Self::Breach) is about, prefixed onto its message.
    ///
    /// Only a breach gains it: the platform check is handed a path and describes what it found as a
    /// predicate (`is group-writable (mode 0775, gid 1000)`), so the caller decides which path the
    /// sentence is about — the file, or the directory holding it. An [`Unreadable`](Self::Unreadable)
    /// is left alone because the resolver already names the path around it.
    fn about(self, path: &Path) -> Self {
        match self {
            Self::Breach { message, help } => Self::Breach {
                message: format!("`{}` {message}", path.display()),
                help,
            },
            unreadable => unreadable,
        }
    }

    /// [`about`](Self::about) for the directory `holder`, naming the `link` in it the path goes
    /// through, so the operator sees why a directory off the file's own path was examined.
    fn holding(self, holder: &Path, link: &Path) -> Self {
        match self {
            Self::Breach { message, help } => Self::Breach {
                message: format!(
                    "`{}`, which holds the link `{}`, {message}",
                    holder.display(),
                    link.display()
                ),
                help,
            },
            unreadable => unreadable,
        }
    }
}

/// The canonical path `path` names: symlinks resolved, `.` and `..` removed, in the platform's
/// plainest spelling.
///
/// **This is the only canonicalization in the configuration**, and that is a rule rather than a
/// convenience. Two separate questions rest on it — whether a file the tree reads is the one the
/// boundary was checked against (`rule:config/ownership-is-the-trust-boundary`, [`check`] below) and whether an entry file is inside an
/// `[[app]]` root (`rule:config/an-application-is-its-entry-file-path`, [`mod@crate::app`]) — and both are decided by comparing paths
/// afterwards. A second implementation is how one of them ends up accepting a `..` or a symlink
/// that the other refuses, so the comparison's first half is written once and shared.
///
/// # Errors
///
/// Whatever `std::fs::canonicalize` says, which for a path that does not exist is a "not found"
/// every caller reports as `E0605`.
///
pub fn canonical(path: &Path) -> std::io::Result<PathBuf> {
    Ok(platform::simplified(std::fs::canonicalize(path)?))
}

/// `rule:config/ownership-is-the-trust-boundary`'s check on `path` and on the directory that contains it, and the canonical path it names.
///
/// The directory gets the same check as the file rather than only the mode half of it: an entry can
/// be replaced by anyone who can write the directory, whatever the file's own bits say.
///
/// **So does the directory that holds each link on `path` as given.** A reload reads `path` through
/// its links again, so whoever can write the directory a link sits in chooses the file the next
/// reload reads. Every prefix of `path` that is a link has its parent checked: a `current` link
/// above the file as well as the file's own link. A path with no link is checked exactly as
/// before. A link's directory has one waiver more than the file's ([`platform::check_link_holder`]):
/// a root-owned directory on a read-only mount skips the mode test, because that is how the
/// kubelet mounts a ConfigMap, at `0777`.
///
/// Cost: one `lstat` per component of `path`, and one check per link found.
///
/// # Errors
///
/// [`Untrusted`], which [`resolve`](crate::resolve) turns into `E0605` when the path could not be
/// examined and `E0607` when it was and failed.
pub fn check(path: &Path) -> Result<PathBuf, Untrusted> {
    let canonical = check_alone(path)?;
    let parent = canonical.parent();
    if let Some(parent) = parent {
        platform::check(parent).map_err(|why| why.about(parent))?;
    }
    for link in path.ancestors().filter(|prefix| {
        std::fs::symlink_metadata(prefix).is_ok_and(|meta| meta.file_type().is_symlink())
    }) {
        let Some(holder) = link.parent().filter(|holder| Some(*holder) != parent) else {
            continue;
        };
        platform::check_link_holder(holder).map_err(|why| why.holding(holder, link))?;
    }
    Ok(canonical)
}

/// [`check`] on `path` alone, with the directory that contains it left unexamined, and the
/// canonical path it names.
///
/// One caller has a reason for this, and every other path the configuration names takes [`check`]:
/// [`mod@crate::data`]'s default folder, `.nvsdata` beside the binary. Its parent is the binary's
/// own directory, and an account that can write there can already replace the binary, so checking
/// that directory guards nothing the binary itself does not — and on a Windows drive's stock ACL,
/// which lets `Authenticated Users` modify, it would make the default folder unusable everywhere.
///
/// # Errors
///
/// As [`check`], about `path` only.
pub fn check_alone(path: &Path) -> Result<PathBuf, Untrusted> {
    let canonical = canonical(path).map_err(|err| Untrusted::Unreadable(err.to_string()))?;
    platform::check(&canonical).map_err(|why| why.about(&canonical))?;
    Ok(canonical)
}

/// `rule:config/a-secret-file-is-checked-for-integrity-and-advised-on-exposure`'s advisory half: how an account other than this one may **read** `path`, or `None`.
///
/// Integrity is enforced and confidentiality is only advised, and the reason is that the two
/// questions have different answers in a container: Compose mounts a secret `0444` and a Kubernetes
/// secret volume defaults to `0644`, so refusing a readable secret file would refuse the normal
/// deployment of every one of them. A path this cannot examine answers `None` — an advisory that
/// cannot be established is not raised, and [`check`] has already refused anything worth refusing
/// about a path that cannot be examined.
#[must_use]
pub fn exposure(path: &Path) -> Option<String> {
    platform::exposure(path)
}

/// This account's SID as SDDL writes it, or `None` if the token cannot be read.
///
/// [`mod@crate::data`] needs it to *build* a security descriptor for the data folder, which is the
/// one thing this module does not otherwise do; the SID lives here because the call that reads it
/// is already here, under the same `#[expect]` and for the same reason. It is the SID and never the
/// display name for `rule:config/ownership-is-the-trust-boundary`'s reason: a name is localized and a rename does not move it.
#[cfg(windows)]
#[must_use]
pub(crate) fn owner_sid() -> Option<String> {
    platform::owner_sid()
}

#[cfg(unix)]
mod platform {
    use std::path::{Path, PathBuf};

    use super::Untrusted;

    /// Nothing to simplify: `fs::canonicalize` already returns an ordinary absolute path here.
    pub(super) fn simplified(path: PathBuf) -> PathBuf {
        path
    }

    /// The owner is this account or root, and neither the group nor the world may write.
    ///
    /// **The owner test is waived on a read-only mount** ([`read_only_mount`]): no account on this
    /// host can change the path through that mount, which is what the owner test guards against.
    /// A container's `-v ./app:/app:ro` is that case, and its files belong to the host's account,
    /// not the container's. The mode test still runs there, so a group- or world-writable mode is
    /// refused on a read-only mount too: it says the files are meant to be shared, and a later
    /// read-write mount of the same tree would serve them.
    #[expect(
        unsafe_code,
        reason = "`geteuid` has no safe spelling in `std`; it takes no argument, cannot fail, and \
                  returns a plain `uid_t`, so the call is unsafe only because it is `extern`"
    )]
    pub(super) fn check(path: &Path) -> Result<(), Untrusted> {
        use std::os::unix::fs::MetadataExt;

        let meta = std::fs::metadata(path).map_err(|err| Untrusted::Unreadable(err.to_string()))?;
        let me = unsafe { libc::geteuid() };
        let stat = Stat {
            owner: meta.uid(),
            gid: meta.gid(),
            mode: meta.mode() & 0o7777,
        };
        judged(path, &stat, me, || read_only_mount(path))
    }

    /// [`check`] on a directory that holds a link the configuration's path goes through, with one
    /// waiver more: **a root-owned directory on a read-only mount skips the mode test too.**
    ///
    /// That is the kubelet's ConfigMap volume. It mounts the directory `0777 root:root` on a
    /// read-only file system, with each key a link into a timestamped folder it swaps. No account
    /// in the pod can write it, and only the kubelet, as root on the node, changes it. A directory
    /// another account owns, or one on a writable mount, gets [`check`] unchanged.
    #[expect(
        unsafe_code,
        reason = "`geteuid` has no safe spelling in `std`; it takes no argument, cannot fail, and \
                  returns a plain `uid_t`, so the call is unsafe only because it is `extern`"
    )]
    pub(super) fn check_link_holder(path: &Path) -> Result<(), Untrusted> {
        use std::os::unix::fs::MetadataExt;

        let meta = std::fs::metadata(path).map_err(|err| Untrusted::Unreadable(err.to_string()))?;
        let me = unsafe { libc::geteuid() };
        let stat = Stat {
            owner: meta.uid(),
            gid: meta.gid(),
            mode: meta.mode() & 0o7777,
        };
        judged_link_holder(path, &stat, me, || read_only_mount(path))
    }

    /// [`check_link_holder`]'s verdict on `stat`, read from `path`, for the account `me`.
    /// `read_only` is asked only of a directory root owns, or, as [`judged`] asks it, of one
    /// another account owns.
    pub(super) fn judged_link_holder(
        path: &Path,
        stat: &Stat,
        me: u32,
        read_only: impl FnOnce() -> bool,
    ) -> Result<(), Untrusted> {
        if stat.owner != 0 {
            return judged(path, stat, me, read_only);
        }
        if read_only() {
            return Ok(());
        }
        judged(path, stat, me, || false)
    }

    /// What [`check`] reads of a path: its owner, its group and its permission bits.
    pub(super) struct Stat {
        pub(super) owner: u32,
        pub(super) gid: u32,
        pub(super) mode: u32,
    }

    /// [`check`]'s verdict on `stat`, read from `path`, for the account `me`. `read_only` is
    /// asked only of a path another account owns.
    ///
    /// The help is the one command that repairs what was found. `chown` comes first because a
    /// mode is only this account's to change once the path is its own or root's, and `chmod` is
    /// prefixed with `sudo` where root owns the path and this is not root.
    pub(super) fn judged(
        path: &Path,
        stat: &Stat,
        me: u32,
        read_only: impl FnOnce() -> bool,
    ) -> Result<(), Untrusted> {
        let Stat { owner, gid, mode } = *stat;
        if owner != me && owner != 0 && !read_only() {
            let quoted = quoted(path);
            let mut help = format!("run `sudo chown root {quoted}`.");
            if me != 0 {
                help.push_str(&format!(
                    " The account that runs `nvs` (uid {me}) may own it instead: \
                     `sudo chown {me} {quoted}`."
                ));
            }
            return Err(Untrusted::breach(
                format!(
                    "is owned by uid {owner}, which is neither this account (uid {me}) nor root"
                ),
                help,
            ));
        }
        let who = match (mode & 0o020, mode & 0o002) {
            (0, 0) => return Ok(()),
            (0, _) => "world-writable",
            (_, 0) => "group-writable",
            _ => "group- and world-writable",
        };
        let sudo = if owner == me { "" } else { "sudo " };
        Err(Untrusted::breach(
            format!("is {who} (mode {mode:04o}, gid {gid})"),
            format!("run `{sudo}chmod go-w {}`.", quoted(path)),
        ))
    }

    /// `path` as a POSIX shell reads it back: bare when every byte is one no shell treats
    /// specially, and in single quotes otherwise, with each `'` closed, escaped and reopened.
    fn quoted(path: &Path) -> String {
        let text = path.to_string_lossy();
        let plain = text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"/._-+,:@%".contains(&byte));
        if plain && !text.is_empty() {
            return text.into_owned();
        }
        format!("'{}'", text.replace('\'', r"'\''"))
    }

    /// Whether the file system `path` is on is mounted read-only (`statvfs` reports `ST_RDONLY`).
    /// `false` when it cannot be asked, so a path that cannot be asked keeps the owner test.
    ///
    /// Cost: one `statvfs`, asked only of a path another account owns.
    #[expect(
        unsafe_code,
        reason = "`statvfs` has no safe spelling in `std`; it reads a NUL-terminated path this \
                  function owns and fills a `statvfs` it reads only after the call returned 0"
    )]
    pub(super) fn read_only_mount(path: &Path) -> bool {
        use std::os::unix::ffi::OsStrExt;

        let Ok(name) = std::ffi::CString::new(path.as_os_str().as_bytes()) else {
            return false;
        };
        let mut stat = std::mem::MaybeUninit::<libc::statvfs>::uninit();
        if unsafe { libc::statvfs(name.as_ptr(), stat.as_mut_ptr()) } != 0 {
            return false;
        }
        let stat = unsafe { stat.assume_init() };
        stat.f_flag & libc::ST_RDONLY != 0
    }

    /// The read half of the same mode bits, as a predicate for the warning to prefix a path onto.
    pub(super) fn exposure(path: &Path) -> Option<String> {
        use std::os::unix::fs::MetadataExt;

        let meta = std::fs::metadata(path).ok()?;
        let mode = meta.mode() & 0o7777;
        let who = match (mode & 0o040, mode & 0o004) {
            (0, 0) => return None,
            (0, _) => "world-readable",
            (_, 0) => "group-readable",
            _ => "group- and world-readable",
        };
        Some(format!("is {who} (mode {mode:04o}, gid {})", meta.gid()))
    }
}

#[cfg(windows)]
mod platform {
    use std::os::windows::ffi::OsStrExt;
    use std::path::{Path, PathBuf};

    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_SUCCESS, GENERIC_ALL, GENERIC_READ, GENERIC_WRITE, HANDLE, LocalFree,
    };
    use windows_sys::Win32::Security::Authorization::{
        ConvertSidToStringSidW, GetNamedSecurityInfoW, SE_FILE_OBJECT,
    };
    use windows_sys::Win32::Security::{
        ACCESS_ALLOWED_ACE, ACE_HEADER, ACL, CreateWellKnownSid, DACL_SECURITY_INFORMATION,
        EqualSid, GetAce, GetLengthSid, GetTokenInformation, INHERIT_ONLY_ACE,
        OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID, SECURITY_MAX_SID_SIZE, TOKEN_QUERY,
        TOKEN_USER, TokenUser, WELL_KNOWN_SID_TYPE, WinAnonymousSid, WinAuthenticatedUserSid,
        WinBuiltinAdministratorsSid, WinBuiltinGuestsSid, WinBuiltinUsersSid, WinLocalSystemSid,
        WinWorldSid,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        DELETE, FILE_ALL_ACCESS, FILE_APPEND_DATA, FILE_GENERIC_READ, FILE_GENERIC_WRITE,
        FILE_READ_DATA, FILE_WRITE_ATTRIBUTES, FILE_WRITE_DATA, FILE_WRITE_EA, WRITE_DAC,
        WRITE_OWNER,
    };
    use windows_sys::Win32::System::SystemServices::{
        ACCESS_ALLOWED_ACE_TYPE, ACCESS_DENIED_ACE_TYPE,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    use super::Untrusted;

    /// The `Administrators` group as `icacls` takes it.
    const ADMINISTRATORS: &str = "*S-1-5-32-544";

    /// The help for an owner outside the boundary: make `Administrators` the owner.
    ///
    /// That owner passes whichever account runs `nvs`, a service's included. Only an elevated
    /// prompt may give a path to a group the caller's filtered token holds as deny-only, so the
    /// help says which prompt. Its last sentence is the case that sends people here: a file an
    /// administrator creates in a normal prompt is owned by their own account, which passes only
    /// while that account is the one running `nvs`.
    fn owner_help(path: &Path) -> String {
        let quoted = quoted(path);
        format!(
            "run `icacls {quoted} /setowner {ADMINISTRATORS}` in a prompt started with \
             \"Run as administrator\". `{ADMINISTRATORS}` is the `Administrators` group. \
             A file you create in a normal prompt is owned by your own account. That owner is \
             allowed only while your account runs `nvs`."
        )
    }

    /// The help for a write right that reaches `sid`: remove the group the breach named.
    ///
    /// By the SID the breach printed, because `icacls` resolves a name in the language Windows is
    /// installed in, so the English one maps to no account anywhere else. `/inheritance:d` comes
    /// first because an inherited entry is not the path's to remove until it is copied. And
    /// `/remove:g` takes every right that group was granted, read included, which is why the grant
    /// that gives reading back is the last thing this says: a directory holding the binary has to
    /// stay runnable by the accounts that were only ever refused *write*.
    fn group_help(path: &Path, sid: &str) -> String {
        let quoted = quoted(path);
        let read = if is_directory(path) {
            "(OI)(CI)RX"
        } else {
            "RX"
        };
        format!(
            "run `icacls {quoted} /inheritance:d`, then `icacls {quoted} /remove:g {sid}`. \
             The second command also removes the right to read. To let every account read again, \
             run `icacls {quoted} /grant *S-1-5-32-545:{read}`."
        )
    }

    /// The help for a null DACL: replace it with one that gives full control to `Administrators`,
    /// `SYSTEM` and `me`, and to nobody else.
    fn null_dacl_help(path: &Path, me: &str) -> String {
        let quoted = quoted(path);
        let full = if is_directory(path) { "(OI)(CI)F" } else { "F" };
        format!(
            "run `icacls {quoted} /inheritance:r /grant:r {ADMINISTRATORS}:{full} \
             *S-1-5-18:{full} *{me}:{full}` in a prompt started with \"Run as administrator\". \
             This gives full control to administrators, `SYSTEM` and the account that runs `nvs`."
        )
    }

    /// `path` as `cmd` and PowerShell both read it back as one argument. A Windows path cannot
    /// contain `"`, so the quotes need no escape.
    fn quoted(path: &Path) -> String {
        format!("\"{}\"", path.display())
    }

    /// Whether `path` is a directory, which takes the inheritance flags `(OI)(CI)` a file does not.
    fn is_directory(path: &Path) -> bool {
        std::fs::metadata(path).is_ok_and(|meta| meta.is_dir())
    }

    /// A SID, held in a `u32` buffer: a `SID` is `DWORD`-aligned and a `Vec<u8>` promises nothing
    /// about alignment, so every one of these is allocated as words and cast at the call.
    type Sid = Vec<u32>;

    /// Every right that lets a principal change the bytes, the name or the ACL. `WRITE_DAC` and
    /// `WRITE_OWNER` are in it because either one buys all the others in a second step.
    const WRITE_RIGHTS: u32 = FILE_WRITE_DATA
        | FILE_APPEND_DATA
        | FILE_WRITE_EA
        | FILE_WRITE_ATTRIBUTES
        | DELETE
        | WRITE_DAC
        | WRITE_OWNER;

    /// The Windows spelling of "the group and the world" — the principals `rule:config/ownership-is-the-trust-boundary` refuses write to,
    /// with the name a refusal prints and the SID it prints beside it. The name is the English one
    /// on every host, so it identifies the group to a reader; the SID is what [`group_help`]'s
    /// `icacls` takes, because that command resolves a name in the installed language.
    const UNTRUSTED: &[(WELL_KNOWN_SID_TYPE, &str, &str)] = &[
        (WinWorldSid, "Everyone", "*S-1-1-0"),
        (
            WinAuthenticatedUserSid,
            "NT AUTHORITY\\Authenticated Users",
            "*S-1-5-11",
        ),
        (WinBuiltinUsersSid, "BUILTIN\\Users", "*S-1-5-32-545"),
        (WinBuiltinGuestsSid, "BUILTIN\\Guests", "*S-1-5-32-546"),
        (WinAnonymousSid, "NT AUTHORITY\\ANONYMOUS LOGON", "*S-1-5-7"),
    ];

    /// `fs::canonicalize` returns a verbatim `\\?\` path, and the rest of the tree — a diagnostic,
    /// the boot log, `rule:config/every-matching-app-block-applies-least-specific-first`'s prefix match — wants the ordinary one. A UNC path keeps its
    /// prefix, where dropping it would name a different thing.
    pub(super) fn simplified(path: PathBuf) -> PathBuf {
        let Some(rest) = path.to_str().and_then(|text| text.strip_prefix(r"\\?\")) else {
            return path;
        };
        if rest.starts_with("UNC\\") {
            return path;
        }
        PathBuf::from(rest)
    }

    /// [`check`] alone: Windows has no read-only mount, so a directory that holds a link has no
    /// waiver the file's own directory lacks.
    pub(super) fn check_link_holder(path: &Path) -> Result<(), Untrusted> {
        check(path)
    }

    /// The owner is trusted and no untrusted principal has an effective write right.
    #[expect(
        unsafe_code,
        reason = "reading a security descriptor is four `advapi32` calls and a `LocalFree`; the \
                  descriptor is freed on both paths and nothing borrowed from it outlives this \
                  function, since `examine` copies every SID it keeps"
    )]
    pub(super) fn check(path: &Path) -> Result<(), Untrusted> {
        let wide = wide(path);
        let mut owner: PSID = std::ptr::null_mut();
        let mut dacl: *mut ACL = std::ptr::null_mut();
        let mut descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        let status = unsafe {
            GetNamedSecurityInfoW(
                wide.as_ptr(),
                SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
                &mut owner,
                std::ptr::null_mut(),
                &mut dacl,
                std::ptr::null_mut(),
                &mut descriptor,
            )
        };
        if status != ERROR_SUCCESS {
            return Err(Untrusted::Unreadable(format!(
                "its security descriptor could not be read (win32 error {status})"
            )));
        }
        let verdict = examine(path, owner, dacl);
        unsafe { LocalFree(descriptor) };
        verdict
    }

    /// The path as the null-terminated wide string every `advapi32` entry point takes.
    fn wide(path: &Path) -> Vec<u16> {
        path.as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    /// `rule:config/a-secret-file-is-checked-for-integrity-and-advised-on-exposure`'s advisory: the first untrusted principal with an effective *read* right, if any.
    ///
    /// The DACL alone, since the owner is not the question here — an owner outside the boundary has
    /// already failed [`check`], and nothing is advised about a path this cannot examine.
    #[expect(
        unsafe_code,
        reason = "the same descriptor read as `check`, freed on both paths, with nothing borrowed \
                  from it outliving the call"
    )]
    pub(super) fn exposure(path: &Path) -> Option<String> {
        let wide = wide(path);
        let mut dacl: *mut ACL = std::ptr::null_mut();
        let mut descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        let status = unsafe {
            GetNamedSecurityInfoW(
                wide.as_ptr(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut dacl,
                std::ptr::null_mut(),
                &mut descriptor,
            )
        };
        if status != ERROR_SUCCESS {
            return None;
        }
        let found = readable_by_others(dacl);
        unsafe { LocalFree(descriptor) };
        found
    }

    /// Each of `rule:config/ownership-is-the-trust-boundary`'s untrusted principals against `FILE_READ_DATA`, which is the one right that
    /// hands over the credential itself.
    fn readable_by_others(dacl: *const ACL) -> Option<String> {
        if dacl.is_null() {
            return Some("has a null DACL, which grants every account every right".to_string());
        }
        for (kind, name, sid) in UNTRUSTED {
            let rights = effective_rights(dacl, &well_known(*kind).ok()?).ok()?;
            if rights & FILE_READ_DATA != 0 {
                return Some(format!(
                    "grants read access to `{name}` (`{sid}`, rights {rights:#010x})"
                ));
            }
        }
        None
    }

    /// `rule:config/ownership-is-the-trust-boundary` against one descriptor of `path`: the owner
    /// first, then each untrusted principal's rights. Each breach carries the `icacls` command
    /// that repairs it, with `path` filled in. The help is built only for a breach, so a path that
    /// passes costs nothing more than the check.
    fn examine(path: &Path, owner: PSID, dacl: *const ACL) -> Result<(), Untrusted> {
        if owner.is_null() {
            return Err(Untrusted::breach(
                "has no owner in its security descriptor".to_string(),
                owner_help(path),
            ));
        }
        let me = token_user()?;
        let trusted = [
            well_known(WinBuiltinAdministratorsSid)?,
            well_known(WinLocalSystemSid)?,
        ];
        if !equal(owner, &me) && !trusted.iter().any(|sid| equal(owner, sid)) {
            return Err(Untrusted::breach(
                format!(
                    "is owned by {}, which is neither this account nor an administrative one",
                    sid_text(owner)
                ),
                owner_help(path),
            ));
        }
        if dacl.is_null() {
            // A null DACL is not an empty one: it grants every account every right.
            let me = sid_text(as_psid(&me));
            return Err(Untrusted::breach(
                "has a null DACL, which grants every account every right".to_string(),
                null_dacl_help(path, me.trim_matches('`')),
            ));
        }
        for (kind, name, sid) in UNTRUSTED {
            let rights = effective_rights(dacl, &well_known(*kind)?)?;
            if rights & WRITE_RIGHTS != 0 {
                return Err(Untrusted::breach(
                    format!("grants write access to `{name}` (`{sid}`, rights {rights:#010x})"),
                    group_help(path, sid),
                ));
            }
        }
        Ok(())
    }

    /// What `sid` may actually do to the object this ACL protects — every deny entry ahead of a
    /// grant subtracted, and every inherited entry seen, because the DACL a security descriptor
    /// hands over already carries them materialized.
    ///
    /// **This is the answer `GetEffectiveRightsFromAclW` gives, computed here instead, and the
    /// reason is the clock.** That call costs on the order of a millisecond per principal — `rule:config/ownership-is-the-trust-boundary`
    /// asks about each untrusted one, on the path and on its parent, so a [`check`](super::check)
    /// built on it is milliseconds of pure overhead, and `rule:packaging/an-artifact-is-one-immutable-content-addressed-file`'s artifact cache pays it on every `nvs run` before it may look
    /// at a single artifact. The walk is orders of magnitude cheaper and answers the same question,
    /// because the rules that make the answer *effective* rather than a spelling are all in it: the
    /// entries are evaluated in order, so a `DENY` removes the bits it names from anything a later
    /// `ALLOW` grants, and an inherited entry is an ordinary entry in this ACL by the time anyone
    /// reads it. An `INHERIT_ONLY` entry is skipped — it describes what children get and not this
    /// object — and a generic bit is mapped to the file rights it stands for. Group membership is
    /// considered by neither: the trustee is always one of `rule:config/ownership-is-the-trust-boundary`'s own well-known SIDs, and it is
    /// that SID's own entries that are being asked about.
    ///
    #[expect(
        unsafe_code,
        reason = "one `GetAce` per entry of an ACL that outlives this call, each read through the \
                  header layout every access-allowed and access-denied entry shares"
    )]
    fn effective_rights(dacl: *const ACL, sid: &Sid) -> Result<u32, Untrusted> {
        let count = unsafe { (*dacl).AceCount };
        let mut allowed = 0u32;
        let mut denied = 0u32;
        for index in 0..u32::from(count) {
            let mut ace: *mut core::ffi::c_void = std::ptr::null_mut();
            if unsafe { GetAce(dacl, index, &mut ace) } == 0 {
                return Err(Untrusted::Unreadable(format!(
                    "entry {index} of its DACL could not be read"
                )));
            }
            let header = unsafe { *ace.cast::<ACE_HEADER>() };
            if u32::from(header.AceFlags) & INHERIT_ONLY_ACE != 0 {
                continue;
            }
            let allow = match u32::from(header.AceType) {
                ACCESS_ALLOWED_ACE_TYPE => true,
                ACCESS_DENIED_ACE_TYPE => false,
                // An audit entry grants nothing, and a callback or object entry cannot appear in a
                // file's DACL — neither is an access this has to account for.
                _ => continue,
            };
            // An `ACCESS_DENIED_ACE` is the same three fields in the same order, which is why one
            // type reads both. `addr_of!` rather than a reference: the SID runs past the end of
            // the struct that names its first word.
            let entry = ace.cast::<ACCESS_ALLOWED_ACE>();
            let trustee = unsafe { std::ptr::addr_of!((*entry).SidStart) };
            if !equal(trustee.cast::<core::ffi::c_void>().cast_mut(), sid) {
                continue;
            }
            let mask = specific(unsafe { (*entry).Mask });
            if allow {
                allowed |= mask & !denied;
            } else {
                denied |= mask & !allowed;
            }
        }
        Ok(allowed)
    }

    /// A right mask with every generic bit replaced by the file rights it stands for.
    ///
    /// Most ways of writing an ACE map them when it is written, but nothing requires it, and a
    /// `GENERIC_WRITE` left unmapped would read as granting nothing at all — the one direction a
    /// trust check may not be wrong in.
    fn specific(mask: u32) -> u32 {
        let mut mapped = mask;
        if mask & GENERIC_ALL != 0 {
            mapped |= FILE_ALL_ACCESS;
        }
        if mask & GENERIC_WRITE != 0 {
            mapped |= FILE_GENERIC_WRITE;
        }
        if mask & GENERIC_READ != 0 {
            mapped |= FILE_GENERIC_READ;
        }
        mapped
    }

    /// This process's own user SID, copied out of its token.
    #[expect(
        unsafe_code,
        reason = "the two-call size-then-read shape `GetTokenInformation` requires, with the \
                  handle closed on both paths and the SID copied before the buffer is dropped"
    )]
    fn token_user() -> Result<Sid, Untrusted> {
        let mut token: HANDLE = std::ptr::null_mut();
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
            return Err(Untrusted::Unreadable(
                "this process's own token could not be opened".to_string(),
            ));
        }
        // The first call is the size query and is expected to fail with a too-small buffer.
        let mut needed = 0u32;
        unsafe { GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut needed) };
        let mut buffer: Vec<u32> = vec![0; words(needed as usize)];
        let read = unsafe {
            GetTokenInformation(
                token,
                TokenUser,
                buffer.as_mut_ptr().cast(),
                needed,
                &mut needed,
            )
        };
        unsafe { CloseHandle(token) };
        if read == 0 {
            return Err(Untrusted::Unreadable(
                "this process's own user could not be read out of its token".to_string(),
            ));
        }
        let user = unsafe { &*buffer.as_ptr().cast::<TOKEN_USER>() };
        Ok(copy_sid(user.User.Sid))
    }

    /// One of the SIDs every Windows installation defines, built rather than parsed so that a
    /// localized account name never enters the comparison.
    #[expect(
        unsafe_code,
        reason = "`CreateWellKnownSid` writes into a buffer this function owns and sized at the \
                  documented maximum"
    )]
    fn well_known(kind: WELL_KNOWN_SID_TYPE) -> Result<Sid, Untrusted> {
        let mut sid: Sid = vec![0; words(SECURITY_MAX_SID_SIZE as usize)];
        let mut len = SECURITY_MAX_SID_SIZE;
        let built = unsafe {
            CreateWellKnownSid(
                kind,
                std::ptr::null_mut(),
                sid.as_mut_ptr().cast(),
                &mut len,
            )
        };
        if built == 0 {
            return Err(Untrusted::Unreadable(
                "a well-known SID could not be built".to_string(),
            ));
        }
        sid.truncate(words(len as usize));
        Ok(sid)
    }

    /// A SID out of a buffer this crate does not own, into one it does.
    #[expect(
        unsafe_code,
        reason = "`GetLengthSid` gives the exact byte count of a SID the caller has just been \
                  handed by the OS, and the copy is that many bytes into a buffer sized for them"
    )]
    fn copy_sid(sid: PSID) -> Sid {
        let len = unsafe { GetLengthSid(sid) } as usize;
        let mut out: Sid = vec![0; words(len)];
        unsafe {
            std::ptr::copy_nonoverlapping(sid.cast::<u8>(), out.as_mut_ptr().cast::<u8>(), len)
        };
        out
    }

    /// Whether two SIDs name the same account.
    #[expect(
        unsafe_code,
        reason = "one `advapi32` call over two SIDs that are live for the length of it"
    )]
    fn equal(left: PSID, right: &Sid) -> bool {
        unsafe { EqualSid(left, as_psid(right)) != 0 }
    }

    /// A SID as `S-1-5-…`, which is what a refusal names: the display name is localized and a
    /// rename does not change the SID.
    #[expect(
        unsafe_code,
        reason = "`ConvertSidToStringSidW` allocates the text with `LocalAlloc`, so it is measured, \
                  copied and freed here"
    )]
    fn sid_text(sid: PSID) -> String {
        let mut text: *mut u16 = std::ptr::null_mut();
        if unsafe { ConvertSidToStringSidW(sid, &mut text) } == 0 {
            return "an account it could not name".to_string();
        }
        let mut len = 0;
        while unsafe { *text.add(len) } != 0 {
            len += 1;
        }
        let out = String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(text, len) });
        unsafe { LocalFree(text.cast()) };
        format!("`{out}`")
    }

    /// This process's own user SID as SDDL writes it — [`super::owner_sid`]'s result.
    ///
    /// [`sid_text`] is the shared half and it brackets its result in backticks, because every other
    /// caller is writing a sentence for an operator. A security descriptor is not a sentence, so
    /// the brackets come off here rather than being made optional there.
    pub(super) fn owner_sid() -> Option<String> {
        let user = token_user().ok()?;
        let text = sid_text(as_psid(&user));
        let bare = text.trim_matches('`');
        bare.starts_with("S-").then(|| bare.to_string())
    }

    /// A SID buffer as the pointer every one of these calls takes.
    fn as_psid(sid: &Sid) -> PSID {
        sid.as_ptr().cast::<core::ffi::c_void>().cast_mut()
    }

    /// The `u32`s it takes to hold `bytes` bytes.
    fn words(bytes: usize) -> usize {
        bytes.div_ceil(std::mem::size_of::<u32>())
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::path::Path;

    use super::Untrusted;
    use super::platform::{Stat, judged, judged_link_holder, read_only_mount};

    /// The account a container runs as, and the host account whose checkout it mounts.
    const SERVER: u32 = 65532;
    const HOST: u32 = 1000;

    /// The path every verdict here is about. Nothing reads it: the verdict only names it.
    fn config() -> &'static Path {
        Path::new("/etc/novis/nvs.toml")
    }

    /// A file another account owns passes on a read-only mount, and the same file on a writable
    /// mount is a breach that names the owner. Mounting a file system needs root, so the mount's
    /// answer is handed to the verdict here, and the Docker smoke test's `:ro` mount asks the
    /// real `statvfs`.
    #[test]
    fn another_accounts_file_passes_on_a_read_only_mount_and_not_on_a_writable_one() {
        let stat = Stat {
            owner: HOST,
            gid: HOST,
            mode: 0o644,
        };
        judged(config(), &stat, SERVER, || true)
            .expect("nobody on this host can write through the mount");
        let why =
            judged(config(), &stat, SERVER, || false).expect_err("the owner can rewrite this file");
        assert!(
            matches!(&why, Untrusted::Breach { message, .. } if message.contains("uid 1000")),
            "the refusal names the owner: {why:?}"
        );
        let help = why.help().expect("a breach carries its repair");
        assert!(
            help.contains("`sudo chown root /etc/novis/nvs.toml`")
                && help.contains("`sudo chown 65532 /etc/novis/nvs.toml`"),
            "the repair is the `chown` to root or to this account, with the path: {help}"
        );
    }

    /// A writable mode is repaired by `chmod go-w` on the named path, with `sudo` only where
    /// another account owns it.
    #[test]
    fn a_writable_mode_is_repaired_by_chmod_with_sudo_only_for_another_owner() {
        let mine = Stat {
            owner: SERVER,
            gid: SERVER,
            mode: 0o664,
        };
        let why = judged(config(), &mine, SERVER, || false).expect_err("the group may write");
        assert_eq!(why.help(), Some("run `chmod go-w /etc/novis/nvs.toml`."));
        let roots = Stat { owner: 0, ..mine };
        let why = judged(config(), &roots, SERVER, || false).expect_err("the group may write");
        assert_eq!(
            why.help(),
            Some("run `sudo chmod go-w /etc/novis/nvs.toml`.")
        );
    }

    /// A path with a space or a quote in it is quoted, so the printed command runs as printed.
    #[test]
    fn a_path_a_shell_would_split_is_quoted_in_the_repair() {
        let stat = Stat {
            owner: SERVER,
            gid: SERVER,
            mode: 0o666,
        };
        let why = judged(Path::new("/srv/my app/it's.toml"), &stat, SERVER, || false)
            .expect_err("the world may write");
        assert_eq!(
            why.help(),
            Some(r"run `chmod go-w '/srv/my app/it'\''s.toml'`.")
        );
    }

    /// The waiver covers the owner alone. A group- or world-writable mode on a read-only mount is
    /// still a breach, and a file this account owns never asks the mount at all.
    #[test]
    fn a_writable_mode_on_a_read_only_mount_is_still_a_breach() {
        for (mode, who) in [
            (0o664, "group-writable"),
            (0o646, "world-writable"),
            (0o777, "group- and world-writable"),
        ] {
            let stat = Stat {
                owner: HOST,
                gid: HOST,
                mode,
            };
            let why = judged(config(), &stat, SERVER, || true)
                .expect_err("the mode says others may write");
            assert!(
                why.message().contains(who),
                "mode {mode:o} on a read-only mount: {why:?}"
            );
        }
        let mine = Stat {
            owner: SERVER,
            gid: SERVER,
            mode: 0o600,
        };
        judged(config(), &mine, SERVER, || {
            panic!("the mount is asked only of another account's file")
        })
        .expect("this account's own private file");
    }

    /// A ConfigMap volume's directory, `0777 root:root` on a read-only mount, passes as the holder
    /// of a link. The same mode on a writable mount is a breach, and so is another account's
    /// `0777` directory on a read-only mount.
    #[test]
    fn a_root_owned_link_holder_on_a_read_only_mount_skips_the_mode_test() {
        let volume = Stat {
            owner: 0,
            gid: 0,
            mode: 0o777,
        };
        let dir = Path::new("/etc/novis");
        judged_link_holder(dir, &volume, SERVER, || true).expect("the kubelet's ConfigMap volume");
        let why = judged_link_holder(dir, &volume, SERVER, || false)
            .expect_err("anyone on this host could swap the link");
        assert!(
            why.message().contains("group- and world-writable"),
            "{why:?}"
        );
        let hosts = Stat {
            owner: HOST,
            ..volume
        };
        let why = judged_link_holder(dir, &hosts, SERVER, || true)
            .expect_err("the waiver is root's alone");
        assert!(why.message().contains("mode 0777"), "{why:?}");
        judged(dir, &volume, SERVER, || true)
            .expect_err("the file's own directory keeps the mode test");
    }

    /// A scratch directory under `target/` is on a writable file system, and a path that is not
    /// there cannot be asked, so neither is waived.
    #[test]
    fn a_writable_file_system_and_a_missing_path_are_not_read_only() {
        let dir = nvs_repo::scratch_private("trust-read-only-mount");
        assert!(!read_only_mount(&dir), "a scratch directory is writable");
        assert!(!read_only_mount(&dir.join("not-there")));
    }
}
