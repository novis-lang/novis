//! `rule:config/one-local-control-socket`'s control endpoint: the one local address `[control] socket` names, created so
//! that no account but this one can reach it.
//!
//! **This lives beside [`mod@crate::trust`] and not in the server, because it is the same question
//! asked in the other direction.** That module answers "can any account but this one *write* a file
//! the configuration reads"; this one creates an object with the answer "no" already built in. Both
//! answers are spelled per platform — a mode on Unix, a DACL on Windows — and putting the second
//! anywhere else would mean a second crate learning how a Windows security descriptor is built, for
//! one object. `nvs-server`'s own `control` module owns everything above this line: which endpoint
//! the directive named, whether the directory holding it passes § 6, and what a connected client is
//! allowed to ask for.
//!
//! **The two platforms are two objects, not one abstraction.** Unix is a `AF_UNIX` socket at the
//! path, mode `0600`; Windows is a named pipe under `\\.\pipe\`, whose kernel namespace has no
//! directory and no mode, so the same guarantee is written as a DACL naming this account,
//! `NT AUTHORITY\SYSTEM` and `BUILTIN\Administrators` and protected from inheritance. That is § 3's
//! "created mode `0600`, owned by the runtime's account" in each platform's own terms, and it is
//! the same substitution `rule:config/ownership-is-the-trust-boundary` already makes for the trust boundary.
//!
//! **The Unix path has a window and it is bounded rather than closed.** `bind` creates the socket
//! with the process umask and only then does the `chmod`, because `std` offers no way to set the
//! mode before the name appears. What bounds it is § 3's other rule, which `nvs-server` enforces
//! before calling here: the directory holding the socket is refused if any other account can write
//! it, so the only principal who could connect inside that window is one who could have replaced
//! the socket outright. Closing it entirely needs a `fchmod` on the listener before `bind`, which is
//! `libc` and an `unsafe` block for a window bounded by a check that already ran.
//!
//! Cost, as `rule:programs/memory-priority` requires: one kernel object per running server, created at boot and closed
//! when the process ends. Nothing per request and nothing per reload.
//!

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use nvs_diagnostics::{Diagnostic, code};

use crate::snapshot::{Current, Snapshot};
use crate::tree::{Config, Setting};
use crate::trust::Untrusted;

/// What `[control] socket` named — `rule:config/one-local-control-socket`.
///
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Address {
    /// `socket = false`, and the state of a tree that wrote no `[control]` block at all.
    Disabled,
    /// A local endpoint: a filesystem path on Unix, a `\\.\pipe\…` name on Windows.
    Local(PathBuf),
}

impl Address {
    /// § 3's directive, read into what a boot would create — or the refusal that stops the boot.
    ///
    /// **Absent is [`Disabled`](Self::Disabled)**, which the ADR does not spell and this decides:
    /// § 3's own sentence is that the socket exists only where a long-running server does, so the
    /// fail-closed reading is that a tree which never asked for a control surface does not get one.
    /// The alternative — defaulting to the `/run/nvs/control.sock` § 3 uses as its example — creates
    /// a control endpoint on every `nvs serve` on a developer's laptop, which is a surface nobody
    /// asked for in exchange for saving one line in the deployments that want it.
    ///
    /// # Errors
    ///
    /// `E0629` for a value that does not name a local endpoint, of which the shape worth refusing
    /// by name is one that would be reachable over a network. That code's own doc is why.
    pub fn of(config: &Config) -> Result<Self, Diagnostic> {
        let Some(written) = config
            .control
            .as_ref()
            .and_then(|block| block.socket.as_ref())
        else {
            return Ok(Self::Disabled);
        };
        match written {
            Setting::Bool(false) => Ok(Self::Disabled),
            Setting::Text(text) if !networked(text) => Ok(Self::Local(PathBuf::from(text))),
            Setting::Text(text) => Err(refused(
                &format!("\"{text}\""),
                "`rule:config/no-network-control-surface` has no network-reachable control surface in it, in either direction \
                 of configuration: the socket's owner and mode are the authentication, and a host \
                 and a port carry neither",
                "name a local endpoint — `socket = \"/run/nvs/control.sock\"`, or \
                 `socket = \"\\\\\\\\.\\\\pipe\\\\nvs-control\"` on Windows",
            )),
            Setting::Integer(port) => Err(refused(
                &port.to_string(),
                "a bare number is a port, and `rule:config/no-network-control-surface` has no listener for one to be bound on",
                "name a local endpoint, or write `socket = false` to have none",
            )),
            other => Err(refused(
                &format!("{other:?}"),
                "`rule:config/one-local-control-socket`'s value is one local endpoint or `false`",
                "write the path as a string, or `socket = false` to have no control surface",
            )),
        }
    }
}

/// Whether `text` describes something that would be reached over a network rather than opened
/// locally — a URL, or a host and a port.
///
/// The port half is decided on the last colon-separated segment being digits *and* the part before
/// it carrying no path separator, which is what keeps `C:\tmp\control.sock` and
/// `\\.\pipe\nvs-control` out of it while catching `localhost:9000` and `example.com:9000`. An
/// address with no host at all — `:9000`, `[::1]:9000` — is caught by the parse instead.
fn networked(text: &str) -> bool {
    if text.contains("://") || text.parse::<std::net::SocketAddr>().is_ok() {
        return true;
    }
    let Some((host, port)) = text.rsplit_once(':') else {
        return false;
    };
    !port.is_empty()
        && port.bytes().all(|byte| byte.is_ascii_digit())
        && !host.contains(['/', '\\'])
}

/// [`Address::of`]'s one refusal, under the one code § 6's rule is worth spending.
fn refused(written: &str, note: &str, help: &str) -> Diagnostic {
    Diagnostic::error(
        code::E_NETWORK_CONTROL_SOCKET,
        format!("`[control] socket = {written}` names no local endpoint"),
    )
    .with_note(note.to_string())
    .with_help(help.to_string())
}

/// A created control endpoint, released when it is dropped.
///
/// It carries the name it was created under so that a caller reporting on it — `nvs info`, the boot
/// log line, a test asking what the platform did — never has to re-derive it.
#[derive(Debug)]
pub struct Endpoint {
    name: PathBuf,
    bound: platform::Bound,
}

impl Endpoint {
    /// Creates the endpoint `name` addresses, reachable by this account and no other.
    ///
    /// A stale endpoint left by a process that died is replaced rather than treated as a refusal:
    /// on Unix the socket file outlives the process that bound it, and a server that would not
    /// start after a crash is an outage caused by the previous outage. Windows needs no such step —
    /// the pipe namespace is kernel-managed and a dead process's pipe is already gone.
    ///
    /// # Errors
    ///
    /// The OS's own error, for a name it will not create — a directory that is not there, a mode
    /// this account may not set, or a name already in use by a *live* server.
    pub fn create(name: &Path) -> io::Result<Self> {
        Ok(Self {
            name: name.to_path_buf(),
            bound: platform::create(name)?,
        })
    }

    /// The name it was created under.
    #[must_use]
    pub fn name(&self) -> &Path {
        &self.name
    }

    /// The platform object, for the accept loop that will drive it.
    #[must_use]
    pub fn bound(&self) -> &platform::Bound {
        &self.bound
    }
}

/// Why a control endpoint was not created.
#[derive(Debug)]
pub enum Refusal {
    /// § 3's other rule: the directory that would hold the socket fails `rule:config/ownership-is-the-trust-boundary`, so anyone
    /// who can write it can replace the socket and speak for the server.
    ///
    Untrusted(Untrusted),
    /// The OS refused the name: it is already in use by a live server, its directory is not there,
    /// or this account may not create it.
    Unavailable(String),
}

impl Refusal {
    /// The refusal as one sentence, whichever half it is.
    #[must_use]
    pub fn message(&self) -> &str {
        match self {
            Self::Untrusted(why) => why.message(),
            Self::Unavailable(message) => message,
        }
    }
}

/// § 3's directory rule for the endpoint `name`: the server refuses to start if any other account
/// could write the place the socket lives.
///
/// On Unix that is [`trust::check`](crate::trust::check) on the directory holding the socket — the
/// same check every configuration file gets, for the same reason, because a directory another
/// account can write is a standing slot they may fill with a socket of their own.
///
/// **On Windows there is nothing to check and that is not a gap.** A named pipe lives in the
/// kernel's `\Device\NamedPipe` namespace, which has no directory an account can be granted write
/// on and which no principal may create an entry in on another's behalf; the whole of the guarantee
/// is the DACL [`Endpoint::create`] applies, which is why that is where it is written.
///
/// # Errors
///
/// [`Untrusted`], which the caller reports as it reports the same verdict about a configuration
/// file.
#[cfg(unix)]
pub fn boundary(name: &Path) -> Result<(), Untrusted> {
    let parent = name.parent().filter(|at| !at.as_os_str().is_empty());
    let Some(parent) = parent else {
        return Err(Untrusted::Unreadable(format!(
            "`{}` names no directory for a control socket to be created in",
            name.display()
        )));
    };
    crate::trust::check(parent).map(drop)
}

/// § 3's directory rule, which on Windows has no directory to be about — see the Unix half's doc.
///
/// # Errors
///
/// Never: the answer is the DACL [`Endpoint::create`] applies, and it is not conditional.
#[cfg(windows)]
pub fn boundary(_name: &Path) -> Result<(), Untrusted> {
    Ok(())
}

/// § 3's endpoint: the directory rule first, then the object.
///
/// `guard` is the check rather than a call to [`boundary`] so that the *order* — refuse before
/// creating anything — is assertable without a filesystem that can be put in the refused state on
/// every platform. Production passes [`boundary`]; a test passes a verdict and asserts that nothing
/// was created.
///
/// # Errors
///
/// [`Refusal`], whose two halves are the boundary and the OS.
pub fn bind(
    name: &Path,
    guard: impl FnOnce(&Path) -> Result<(), Untrusted>,
) -> Result<Endpoint, Refusal> {
    guard(name).map_err(Refusal::Untrusted)?;
    Endpoint::create(name).map_err(|err| Refusal::Unavailable(err.to_string()))
}

/// `rule:config/a-reload-names-what-it-could-not-apply`'s answer to a reload: what it did, and what it could not do.
///
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// The dotted keys the swap changed and which are now in force, in dotted-key order.
    pub applied: Vec<String>,
    /// The `Boot` keys whose written value changed and which therefore did **not** take effect,
    /// each named individually. § 5's own paragraph is why they are named rather than counted: a
    /// deployment that silently ignores a changed listen address believes it applied a change it
    /// did not.
    pub ignored: Vec<&'static str>,
    /// How many compiled units the swap invalidated, so an operator knows a recompile wave is
    /// coming. § 4's rule is what decides it: a changed `env_hash` rekeys **every** unit and an
    /// unchanged one rekeys none, because a unit key carries the hash whole.
    pub invalidated: usize,
}

/// Publishes `next` over what `current` is serving and reports § 5's three answers.
///
/// `held` is how many compiled units the caller has, which is the only half of § 5's third answer
/// this module cannot know: the unit cache is `nvs-cli`'s.
///
/// # Errors
///
/// Whatever [`Current::publish`] refuses — `E0601` for a tree that does not deserialize once the
/// running `Boot` values are carried into it. The previous snapshot is still serving in that case,
/// because publishing is the last step and it never ran.
pub fn reload(current: &Current, next: Snapshot, held: usize) -> Result<Report, Diagnostic> {
    let before = current.load();
    let published = current.publish(next)?;
    // The comparison is against the *published* table and not the submitted one, which is what
    // makes a changed `Boot` key absent from `applied` rather than present in both lists: publishing
    // carries the running value back over it, so by this line the two tables agree about it again.
    let (was, now) = (leaves(&before.table), leaves(&published.snapshot.table));
    let mut applied: Vec<String> = now
        .iter()
        .filter(|(key, value)| was.get(*key) != Some(*value))
        .map(|(key, _)| key.clone())
        .chain(
            was.keys()
                .filter(|key| !now.contains_key(*key))
                .map(Clone::clone),
        )
        .collect();
    applied.sort();
    applied.dedup();
    let (was_env, now_env) = (
        crate::cache::env_hash(&before.config),
        crate::cache::env_hash(&published.snapshot.config),
    );
    let invalidated =
        usize::from(was_env.digest().as_bytes() != now_env.digest().as_bytes()) * held;
    Ok(Report {
        applied,
        ignored: published.boot.iter().map(|row| row.key).collect(),
        invalidated,
    })
}

/// Every leaf in `table` by its dotted key — the shape a key-by-key diff needs and the typed tree
/// cannot be turned back into, which is the same reason [`Snapshot::table`] is kept at all.
fn leaves(table: &toml::Table) -> BTreeMap<String, toml::Value> {
    fn walk(table: &toml::Table, prefix: &str, into: &mut BTreeMap<String, toml::Value>) {
        for (key, value) in table {
            let dotted = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{prefix}.{key}")
            };
            match value.as_table() {
                Some(inner) => walk(inner, &dotted, into),
                None => drop(into.insert(dotted, value.clone())),
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(table, "", &mut out);
    out
}

#[cfg(unix)]
pub mod platform {
    //! The Unix endpoint: an `AF_UNIX` socket at the path, mode `0600`.

    use std::fs::Permissions;
    use std::io;
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::UnixListener;
    use std::path::Path;

    /// What a created endpoint holds here: the listening socket itself.
    pub type Bound = UnixListener;

    /// § 3's socket, and the `chmod` whose window this module's doc bounds.
    pub(super) fn create(name: &Path) -> io::Result<Bound> {
        // A socket file the last process left behind is not a live server, and `bind` refuses an
        // existing name. The directory check that ran before this call is what makes removing it
        // safe: nobody else could have put it there.
        drop(std::fs::remove_file(name));
        let listener = UnixListener::bind(name)?;
        std::fs::set_permissions(name, Permissions::from_mode(0o600))?;
        Ok(listener)
    }
}

#[cfg(windows)]
pub mod platform {
    //! The Windows endpoint: a named pipe whose DACL names this account, `SYSTEM` and
    //! `Administrators`, and is protected so that nothing is inherited into it.

    use std::io;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use windows_sys::Win32::Foundation::{
        CloseHandle, HANDLE, HLOCAL, INVALID_HANDLE_VALUE, LocalFree,
    };
    use windows_sys::Win32::Security::Authorization::{
        ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
    };
    use windows_sys::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_DUPLEX,
    };
    use windows_sys::Win32::System::Pipes::{
        CreateNamedPipeW, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES,
    };

    /// The in and out buffer a pipe instance is created with. A control message is a short HTTP
    /// request and its response; 64 KiB is what every named-pipe example in the platform's own
    /// documentation uses, and the buffer is an advisory hint rather than a limit on either.
    const BUFFER: u32 = 64 * 1024;

    /// What a created endpoint holds here: the first instance's handle, closed on drop.
    pub struct Bound(HANDLE);

    impl std::fmt::Debug for Bound {
        /// The handle value is a process-local number that says nothing to a reader of a log line,
        /// so the shape is named and the number is not.
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("Bound(a named pipe instance)")
        }
    }

    impl Drop for Bound {
        #[expect(
            unsafe_code,
            reason = "one `kernel32` call over a handle this type owns and is giving up"
        )]
        fn drop(&mut self) {
            unsafe { CloseHandle(self.0) };
        }
    }

    /// A security descriptor built from SDDL, freed with `LocalFree` on every path out.
    struct Descriptor(PSECURITY_DESCRIPTOR);

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

    /// § 3's endpoint: a named pipe no account but this one, `SYSTEM` or an administrator may open.
    ///
    /// The descriptor is built rather than read, and it is `D:P` — protected — because a DACL that
    /// merely *adds* these three to whatever the namespace would have handed down is not the same
    /// guarantee. `GA` is the whole generic-access mask: a control connection is duplex, so a
    /// principal that may reach it at all may do everything on it.
    #[expect(
        unsafe_code,
        reason = "two Win32 calls; the descriptor is owned by a guard that frees it on both paths, \
                  the name is a NUL-terminated buffer this frame owns, and neither pointer outlives \
                  the `CreateNamedPipeW` that reads them"
    )]
    pub(super) fn create(name: &Path) -> io::Result<Bound> {
        let sid = crate::trust::owner_sid().ok_or_else(|| {
            io::Error::other(
                "this process's own account could not be read out of its token, so a control \
                 endpoint only it can reach cannot be described",
            )
        })?;
        let sddl: Vec<u16> = format!("D:P(A;;GA;;;{sid})(A;;GA;;;SY)(A;;GA;;;BA)")
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let mut raw: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        let built = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                SDDL_REVISION_1,
                &mut raw,
                std::ptr::null_mut(),
            )
        };
        if built == 0 {
            return Err(io::Error::last_os_error());
        }
        let descriptor = Descriptor(raw);
        let attributes = SECURITY_ATTRIBUTES {
            nLength: u32::try_from(std::mem::size_of::<SECURITY_ATTRIBUTES>())
                .expect("a Win32 struct is far smaller than a `u32` counts"),
            lpSecurityDescriptor: descriptor.0,
            bInheritHandle: 0,
        };
        let wide: Vec<u16> = name.as_os_str().encode_wide().chain(Some(0)).collect();
        let handle = unsafe {
            CreateNamedPipeW(
                wide.as_ptr(),
                PIPE_ACCESS_DUPLEX | FILE_FLAG_FIRST_PIPE_INSTANCE,
                PIPE_TYPE_BYTE,
                PIPE_UNLIMITED_INSTANCES,
                BUFFER,
                BUFFER,
                0,
                &attributes,
            )
        };
        drop(descriptor);
        if handle == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        Ok(Bound(handle))
    }
}
