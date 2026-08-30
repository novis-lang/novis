//! [ADR 0103] § 6's trust boundary: whether any account but this one can write a file the
//! configuration reads.
//!
//! The configuration grants capabilities, so whoever can write **any** file in the tree can grant
//! themselves every one of them. § 6 answers that with one rule and no exceptions: every file must
//! be owned by the account the runtime runs as or by root, and writable by nobody else — and so
//! must the directory that contains it, because a directory its owner can write is a standing slot
//! that owner may fill later. That is why the containing directory is checked for its **owner** as
//! well as its mode, and why an absent `optional` include is checked at all rather than skipped:
//! [`resolve`](crate::resolve)'s `include_targets` puts the check on the directory the file would
//! appear in, which is the only place a promise about a file that does not exist yet can be kept.
//!
//! **Unix is the mode bits. Windows is the DACL, and § 6 left which ACEs it accepts to M6** — the
//! answer, which that ADR's body now states, is implemented here: the owner must be this account,
//! `BUILTIN\Administrators` or `NT AUTHORITY\SYSTEM`, and no *effective* write right may reach
//! `Everyone`, `NT AUTHORITY\Authenticated Users`, `BUILTIN\Users`, `BUILTIN\Guests` or
//! `ANONYMOUS LOGON`. Effective rather than by ACE, so a deny entry counts, an inherited grant is
//! seen, and the question asked is the one that matters — can that principal write this file —
//! rather than how the ACL happens to be spelled. Those five SIDs are the Windows spelling of "the
//! group and the world"; the write rights are every one that changes the bytes, the name or the
//! ACL itself, `WRITE_DAC` and `WRITE_OWNER` included, because either of those buys the rest.
//!
//! **Canonicalization is part of the check, not a side effect of it.** [`check`] returns the
//! canonical path, and that is what makes the resolver's cycle test compare files rather than
//! spellings — a cycle assembled out of symlinks is invisible to a lexical comparison. It is also
//! the one path comparison ADR 0104 § 2's `[[app]]` matching is built on, so it is written once
//! here rather than three times.
//!
//! Cost: two `stat`s per file on Unix, two security-descriptor reads on Windows, at boot and again
//! at each `nvs ctl reload`. Nothing here runs per request.
//!
//! [ADR 0103]: ../../../docs/adr/0103-configuration-is-a-tree-of-files.md

use std::path::{Path, PathBuf};

/// Why a path the configuration names is not trusted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Untrusted {
    /// It could not be examined at all: it is absent, or something above it refuses to be read.
    /// The resolver reports this as `E0605`, exactly as it reports any other file it cannot read,
    /// and the message is the underlying reader's own.
    Unreadable(String),
    /// It was examined and it fails § 6. The message names the path and says how, in the
    /// platform's own terms — a mode and a gid on Unix, a principal and a right mask on Windows.
    Breach(String),
}

impl Untrusted {
    /// The message, whichever half this is.
    #[must_use]
    pub fn message(&self) -> &str {
        match self {
            Self::Unreadable(message) | Self::Breach(message) => message,
        }
    }

    /// The path a [`Breach`](Self::Breach) is about, prefixed onto its message.
    ///
    /// Only a breach gains it: the platform check is handed a path and describes what it found as a
    /// predicate (`is group-writable (mode 0775, gid 1000)`), so the caller decides which path the
    /// sentence is about — the file, or the directory holding it. An [`Unreadable`](Self::Unreadable)
    /// is left alone because the resolver already names the path around it.
    fn about(self, path: &Path) -> Self {
        match self {
            Self::Breach(message) => Self::Breach(format!("`{}` {message}", path.display())),
            unreadable => unreadable,
        }
    }
}

/// What a [`Untrusted::Breach`] asks the operator to do, in the platform's own spelling.
pub const REMEDY: &str = platform::REMEDY;

/// § 6's check on `path` and on the directory that contains it, and the canonical path it names.
///
/// The directory gets the same check as the file rather than only the mode half of it: an entry can
/// be replaced by anyone who can write the directory, whatever the file's own bits say.
///
/// # Errors
///
/// [`Untrusted`], which [`resolve`](crate::resolve) turns into `E0605` when the path could not be
/// examined and `E0607` when it was and failed.
pub fn check(path: &Path) -> Result<PathBuf, Untrusted> {
    let canonical = platform::simplified(
        std::fs::canonicalize(path).map_err(|err| Untrusted::Unreadable(err.to_string()))?,
    );
    platform::check(&canonical).map_err(|why| why.about(&canonical))?;
    if let Some(parent) = canonical.parent() {
        platform::check(parent).map_err(|why| why.about(parent))?;
    }
    Ok(canonical)
}

#[cfg(unix)]
mod platform {
    use std::path::{Path, PathBuf};

    use super::Untrusted;

    pub(super) const REMEDY: &str = "make the path owned by this account or by root and writable \
                                     by neither its group nor anyone else: `chmod go-w <path>`";

    /// Nothing to simplify: `fs::canonicalize` already returns an ordinary absolute path here.
    pub(super) fn simplified(path: PathBuf) -> PathBuf {
        path
    }

    /// The owner is this account or root, and neither the group nor the world may write.
    #[expect(
        unsafe_code,
        reason = "`geteuid` has no safe spelling in `std`; it takes no argument, cannot fail, and \
                  returns a plain `uid_t`, so the call is unsafe only because it is `extern`"
    )]
    pub(super) fn check(path: &Path) -> Result<(), Untrusted> {
        use std::os::unix::fs::MetadataExt;

        let meta = std::fs::metadata(path).map_err(|err| Untrusted::Unreadable(err.to_string()))?;
        let owner = meta.uid();
        let me = unsafe { libc::geteuid() };
        if owner != me && owner != 0 {
            return Err(Untrusted::Breach(format!(
                "is owned by uid {owner}, which is neither this account (uid {me}) nor root"
            )));
        }
        let mode = meta.mode() & 0o7777;
        let who = match (mode & 0o020, mode & 0o002) {
            (0, 0) => return Ok(()),
            (0, _) => "world-writable",
            (_, 0) => "group-writable",
            _ => "group- and world-writable",
        };
        Err(Untrusted::Breach(format!(
            "is {who} (mode {mode:04o}, gid {})",
            meta.gid()
        )))
    }
}

#[cfg(windows)]
mod platform {
    use std::os::windows::ffi::OsStrExt;
    use std::path::{Path, PathBuf};

    use windows_sys::Win32::Foundation::{CloseHandle, ERROR_SUCCESS, HANDLE, LocalFree};
    use windows_sys::Win32::Security::Authorization::{
        ConvertSidToStringSidW, GetEffectiveRightsFromAclW, GetNamedSecurityInfoW,
        NO_MULTIPLE_TRUSTEE, SE_FILE_OBJECT, TRUSTEE_IS_SID, TRUSTEE_IS_UNKNOWN, TRUSTEE_W,
    };
    use windows_sys::Win32::Security::{
        ACL, CreateWellKnownSid, DACL_SECURITY_INFORMATION, EqualSid, GetLengthSid,
        GetTokenInformation, OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID,
        SECURITY_MAX_SID_SIZE, TOKEN_QUERY, TOKEN_USER, TokenUser, WELL_KNOWN_SID_TYPE,
        WinAnonymousSid, WinAuthenticatedUserSid, WinBuiltinAdministratorsSid, WinBuiltinGuestsSid,
        WinBuiltinUsersSid, WinLocalSystemSid, WinWorldSid,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        DELETE, FILE_APPEND_DATA, FILE_WRITE_ATTRIBUTES, FILE_WRITE_DATA, FILE_WRITE_EA, WRITE_DAC,
        WRITE_OWNER,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    use super::Untrusted;

    pub(super) const REMEDY: &str = "the path must be owned by this account, `BUILTIN\\Administrators` or \
         `NT AUTHORITY\\SYSTEM` and grant write to no one else: `icacls <path> /inheritance:d` \
         then `icacls <path> /remove:g \"Authenticated Users\" \"Users\" \"Everyone\"`";

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

    /// The Windows spelling of "the group and the world" — the principals § 6 refuses write to,
    /// with the name a refusal prints.
    const UNTRUSTED: &[(WELL_KNOWN_SID_TYPE, &str)] = &[
        (WinWorldSid, "Everyone"),
        (WinAuthenticatedUserSid, "NT AUTHORITY\\Authenticated Users"),
        (WinBuiltinUsersSid, "BUILTIN\\Users"),
        (WinBuiltinGuestsSid, "BUILTIN\\Guests"),
        (WinAnonymousSid, "NT AUTHORITY\\ANONYMOUS LOGON"),
    ];

    /// `fs::canonicalize` returns a verbatim `\\?\` path, and the rest of the tree — a diagnostic,
    /// the boot log, ADR 0104 § 2's prefix match — wants the ordinary one. A UNC path keeps its
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

    /// The owner is trusted and no untrusted principal has an effective write right.
    #[expect(
        unsafe_code,
        reason = "reading a security descriptor is four `advapi32` calls and a `LocalFree`; the \
                  descriptor is freed on both paths and nothing borrowed from it outlives this \
                  function, since `examine` copies every SID it keeps"
    )]
    pub(super) fn check(path: &Path) -> Result<(), Untrusted> {
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
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
        let verdict = examine(owner, dacl);
        unsafe { LocalFree(descriptor) };
        verdict
    }

    /// § 6 against one descriptor: the owner first, then each untrusted principal's rights.
    fn examine(owner: PSID, dacl: *const ACL) -> Result<(), Untrusted> {
        if owner.is_null() {
            return Err(Untrusted::Breach(
                "has no owner in its security descriptor".to_string(),
            ));
        }
        let trusted = [
            token_user()?,
            well_known(WinBuiltinAdministratorsSid)?,
            well_known(WinLocalSystemSid)?,
        ];
        if !trusted.iter().any(|sid| equal(owner, sid)) {
            return Err(Untrusted::Breach(format!(
                "is owned by {}, which is neither this account nor an administrative one",
                sid_text(owner)
            )));
        }
        if dacl.is_null() {
            // A null DACL is not an empty one: it grants every account every right.
            return Err(Untrusted::Breach(
                "has a null DACL, which grants every account every right".to_string(),
            ));
        }
        for (kind, name) in UNTRUSTED {
            let rights = effective_rights(dacl, &well_known(*kind)?)?;
            if rights & WRITE_RIGHTS != 0 {
                return Err(Untrusted::Breach(format!(
                    "grants write access to `{name}` (rights {rights:#010x})"
                )));
            }
        }
        Ok(())
    }

    /// What `sid` may actually do to the object this ACL protects — deny entries and inherited
    /// grants both accounted for, which is why this asks for effective rights rather than walking
    /// the ACEs itself.
    #[expect(
        unsafe_code,
        reason = "one `advapi32` call over a borrowed ACL and a trustee built on this stack frame; \
                  neither pointer outlives the call"
    )]
    fn effective_rights(dacl: *const ACL, sid: &Sid) -> Result<u32, Untrusted> {
        let trustee = TRUSTEE_W {
            pMultipleTrustee: std::ptr::null_mut(),
            MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
            TrusteeForm: TRUSTEE_IS_SID,
            TrusteeType: TRUSTEE_IS_UNKNOWN,
            ptstrName: as_psid(sid).cast::<u16>(),
        };
        let mut rights = 0u32;
        let status = unsafe { GetEffectiveRightsFromAclW(dacl, &trustee, &mut rights) };
        if status != ERROR_SUCCESS {
            return Err(Untrusted::Unreadable(format!(
                "the effective rights of a well-known account could not be read \
                 (win32 error {status})"
            )));
        }
        Ok(rights)
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

    /// A SID buffer as the pointer every one of these calls takes.
    fn as_psid(sid: &Sid) -> PSID {
        sid.as_ptr().cast::<core::ffi::c_void>().cast_mut()
    }

    /// The `u32`s it takes to hold `bytes` bytes.
    fn words(bytes: usize) -> usize {
        bytes.div_ceil(std::mem::size_of::<u32>())
    }
}
