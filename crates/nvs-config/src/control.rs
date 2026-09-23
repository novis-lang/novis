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
//! **One connection at a time, which is where `rule:config/one-local-control-socket`'s
//! "operations serialize" is actually enforced.** [`Endpoint::accept`] blocks until a client is on
//! the endpoint and hands back the duplex stream to answer it on; the next accept happens only
//! once that stream is dropped. On Unix a second client waits in the listen backlog, and on Windows
//! [`connect`] waits on the busy instance for the same bounded moment, so the two platforms make a
//! waiting client wait rather than one of them refusing it.
//!
//! **The accept blocks and the stream it hands back does not**, which is not an inconsistency: the
//! server has nothing to do until a client arrives, and once one has, the connection above is
//! `hyper`'s, which asks to read the next request before it writes the answer to the one it holds.
//! A read that waited there would wait for a client that is waiting for that answer. So a read with
//! nothing behind it answers [`io::ErrorKind::WouldBlock`] — a socket flag on Unix, a
//! `PeekNamedPipe` on Windows — and the loop driving the connection is what decides how long to
//! wait, which is `nvs_server::control::answer_connection`.
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
//! when the process ends, and the pipe's own in and out buffers on Windows. A connected client adds
//! an accepted socket on Unix and nothing at all on Windows, where the stream is a borrow of the one
//! instance; either way it is one at a time for the whole process. Nothing per request and nothing
//! per reload.
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

    /// Blocks until a client is connected, handing back the duplex stream to answer it on.
    ///
    /// The stream borrows the endpoint, which is what makes "one operation at a time" a fact about
    /// the types rather than a discipline the accept loop has to keep: there is no second stream to
    /// be had while one is alive, and on Windows the borrow is literal — the stream *is* the one
    /// pipe instance, handed back to the next accept when it is dropped.
    ///
    /// # Errors
    ///
    /// The OS's own error, for an endpoint that can no longer be accepted on.
    pub fn accept(&self) -> io::Result<platform::Stream<'_>> {
        self.bound.accept()
    }

    /// The platform object underneath [`accept`](Self::accept), for a caller that has to ask the OS
    /// something about the endpoint itself.
    #[must_use]
    pub fn bound(&self) -> &platform::Bound {
        &self.bound
    }
}

/// Opens the endpoint `name` addresses as a client — the half `nvs ctl` speaks HTTP over.
///
/// It lives here rather than in the client because the platform split is the same one this module
/// already owns in the other direction, and a client that spelled it a second time would be a
/// second place to get `\\.\pipe\` wrong.
///
/// **What comes back answers [`io::ErrorKind::WouldBlock`] rather than waiting**, for the mirror
/// of the reason `platform::Stream` does: the client's half of the connection is `hyper`'s too,
/// and its dispatcher reads the transport before it has written anything, so a read that waited
/// there would be waiting for a server that is waiting for the request. `nvs-cli`'s `ctl` module
/// is the loop that turns it back into a wait.
///
/// # Errors
///
/// The OS's own error, for an endpoint that is not there — which is what a host with no running
/// server looks like — or that this account may not open, which is § 3's DACL and mode doing their
/// job.
pub fn connect(name: &Path) -> io::Result<platform::Client> {
    platform::connect(name)
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
    /// did not. A reloadable key whose new resource could not be built is named here too
    /// ([`reload`]'s `keep`).
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
/// `keep` names the reloadable keys whose new resource the caller could not build. Each keeps
/// its running value and is reported in [`Report::ignored`], as [`Current::publish_keeping`]
/// says.
///
/// # Errors
///
/// Whatever [`Current::publish`] refuses — `E0601` for a tree that does not deserialize once the
/// running `Boot` values are carried into it. The previous snapshot is still serving in that case,
/// because publishing is the last step and it never ran.
pub fn reload(
    current: &Current,
    next: Snapshot,
    held: usize,
    keep: &[&str],
) -> Result<Report, Diagnostic> {
    let before = current.load();
    let published = current.publish_keeping(next, keep)?;
    // The comparison is against the *published* table and not the submitted one, which is what
    // makes a changed `Boot` key absent from `applied` rather than present in both lists: publishing
    // carries the running value back over it, so by this line the two tables agree about it again.
    let (was, now) = (leaves(&before), leaves(&published.snapshot));
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

/// Every leaf in `snapshot`'s table by its dotted key — the shape a key-by-key diff needs and the
/// typed tree cannot be turned back into, which is the same reason [`Snapshot::table`] is kept at
/// all. The `[[app]]` roster is one more leaf, `app`, because the table does not carry it: an array
/// is a leaf wherever it is written, so this is the key the roster would have had there.
fn leaves(snapshot: &Snapshot) -> BTreeMap<String, toml::Value> {
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
    walk(&snapshot.table, "", &mut out);
    if let Some(roster) = &snapshot.roster {
        out.insert("app".to_string(), roster.clone());
    }
    out
}

#[cfg(unix)]
pub mod platform {
    //! The Unix endpoint: an `AF_UNIX` socket at the path, mode `0600`.

    use std::fs::Permissions;
    use std::io::{self, Read, Write};
    use std::marker::PhantomData;
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::Path;

    /// What a created endpoint holds here: the listening socket itself.
    #[derive(Debug)]
    pub struct Bound(UnixListener);

    impl Bound {
        /// The next client, waited for on this thread.
        ///
        /// # Errors
        ///
        /// The OS's own error, for a listener that can no longer accept.
        pub fn accept(&self) -> io::Result<Stream<'_>> {
            let (stream, _peer) = self.0.accept()?;
            // The peer address of an `AF_UNIX` client is unnamed and says nothing about who it is;
            // who may connect at all is the socket's mode, decided before this call ever runs.
            //
            // Non-blocking because the connection above this is `hyper`'s, and `hyper` asks to read
            // again before it writes the answer it already has: a read that waited there would be
            // waiting for a client that is waiting for the answer. [`Stream`]'s doc is the whole of
            // that contract, and the Windows half spells it with a peek.
            stream.set_nonblocking(true)?;
            Ok(Stream {
                stream,
                endpoint: PhantomData,
            })
        }
    }

    /// One connected control client: a duplex byte stream, closed when it is dropped.
    ///
    /// **A read that would wait answers [`io::ErrorKind::WouldBlock`] instead**, because the
    /// connection driven over this is `hyper`'s and `hyper` polls for the next request before it
    /// writes the answer to the one it has: a stream that waited there would deadlock against a
    /// client waiting for that answer. The drive loop is what turns that into a wait —
    /// `nvs_server::io::Nonblocking` and `nvs_server::control::answer_connection`.
    ///
    /// The accepted socket outlives the listener perfectly well here, so the borrow is not what
    /// keeps it valid — it is what makes this the same signature Windows has, where the stream
    /// really is the endpoint's one instance. One accept loop over both is worth a lifetime that
    /// one of the two platforms could do without.
    #[derive(Debug)]
    pub struct Stream<'a> {
        stream: UnixStream,
        endpoint: PhantomData<&'a Bound>,
    }

    impl Read for Stream<'_> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.stream.read(buf)
        }
    }

    impl Write for Stream<'_> {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.stream.write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.stream.flush()
        }
    }

    /// A client's end of the endpoint: the connected socket, non-blocking for the reason
    /// [`super::connect`] gives.
    #[derive(Debug)]
    pub struct Client(UnixStream);

    impl Read for Client {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.0.read(buf)
        }
    }

    impl Write for Client {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.0.flush()
        }
    }

    /// § 3's socket, and the `chmod` whose window this module's doc bounds.
    pub(super) fn create(name: &Path) -> io::Result<Bound> {
        // A socket file the last process left behind is not a live server, and `bind` refuses an
        // existing name. The directory check that ran before this call is what makes removing it
        // safe: nobody else could have put it there.
        drop(std::fs::remove_file(name));
        let listener = UnixListener::bind(name)?;
        std::fs::set_permissions(name, Permissions::from_mode(0o600))?;
        Ok(Bound(listener))
    }

    /// The client's connect, which on Unix is the whole of it: a client arriving while another is
    /// being answered waits in the listen backlog.
    pub(super) fn connect(name: &Path) -> io::Result<Client> {
        let stream = UnixStream::connect(name)?;
        stream.set_nonblocking(true)?;
        Ok(Client(stream))
    }
}

#[cfg(windows)]
pub mod platform {
    //! The Windows endpoint: a named pipe whose DACL names this account, `SYSTEM` and
    //! `Administrators`, and is protected so that nothing is inherited into it.

    use std::io::{self, Read, Write};
    use std::marker::PhantomData;
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::AsRawHandle;
    use std::path::Path;
    use std::time::{Duration, Instant};

    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_BROKEN_PIPE, ERROR_PIPE_BUSY, ERROR_PIPE_CONNECTED, HANDLE, HLOCAL,
        INVALID_HANDLE_VALUE, LocalFree,
    };
    use windows_sys::Win32::Security::Authorization::{
        ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
    };
    use windows_sys::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_FIRST_PIPE_INSTANCE, FlushFileBuffers, PIPE_ACCESS_DUPLEX, ReadFile, WriteFile,
    };
    use windows_sys::Win32::System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_TYPE_BYTE,
        PIPE_UNLIMITED_INSTANCES, PeekNamedPipe, WaitNamedPipeW,
    };

    /// The in and out buffer a pipe instance is created with. A control message is a short HTTP
    /// request and its response; 64 KiB is what every named-pipe example in the platform's own
    /// documentation uses, and the buffer is an advisory hint rather than a limit on either.
    const BUFFER: u32 = 64 * 1024;

    /// How long a client waits in total for an instance another client is being answered on, before
    /// the busy error is what it gets. The operation holding it is a configuration re-read and not
    /// a request, so this is longer than one takes and short enough that a wedged server reports as
    /// one rather than as a client that never returns.
    const BUSY_WAIT: Duration = Duration::from_secs(5);

    /// What a created endpoint holds here: the first instance's handle, closed on drop.
    pub struct Bound(HANDLE);

    /// A pipe handle is a process-wide kernel object that any thread may use, and this type is the
    /// only owner of this one — so the endpoint moves to the thread that accepts on it, which is
    /// what `rule:concurrency/one-scheduler`'s control thread needs of it. The raw pointer inside
    /// `HANDLE` is what withholds this by default; nothing about the object does.
    #[expect(
        unsafe_code,
        reason = "the handle is owned solely by this type and is valid on any thread of this process"
    )]
    unsafe impl Send for Bound {}

    impl Bound {
        /// The next client, waited for on this thread.
        ///
        /// # Errors
        ///
        /// The OS's own error, for an instance that can no longer be connected on.
        #[expect(
            unsafe_code,
            reason = "one `kernel32` call over the handle this type owns; the overlapped pointer is \
                      null because the instance is synchronous, so the call returns when a client \
                      is on it and not before"
        )]
        pub fn accept(&self) -> io::Result<Stream<'_>> {
            let connected = unsafe { ConnectNamedPipe(self.0, std::ptr::null_mut()) };
            if connected == 0 {
                let failed = io::Error::last_os_error();
                // A client that opened the instance between its creation and this call is already
                // on it, which is this call having succeeded early rather than having failed.
                if os_error(&failed) != Some(ERROR_PIPE_CONNECTED) {
                    return Err(failed);
                }
            }
            Ok(Stream {
                handle: self.0,
                endpoint: PhantomData,
            })
        }
    }

    /// One connected control client: the endpoint's own instance, given back to the next accept
    /// when it is dropped.
    ///
    /// There is no second handle and nothing is duplicated, so the borrow is what keeps this sound:
    /// the instance is the endpoint's, and this type may only read and write it while the endpoint
    /// is alive and no other stream exists.
    pub struct Stream<'a> {
        handle: HANDLE,
        endpoint: PhantomData<&'a Bound>,
    }

    impl std::fmt::Debug for Stream<'_> {
        /// The handle value is the endpoint's own and says nothing more here than it does there.
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("Stream(a connected named pipe instance)")
        }
    }

    impl Drop for Stream<'_> {
        #[expect(
            unsafe_code,
            reason = "two `kernel32` calls over the borrowed instance; the flush is what lets the \
                      client read the last of an answer before the instance is taken back, and \
                      neither call closes a handle this type does not own"
        )]
        fn drop(&mut self) {
            // Disconnecting discards whatever the client has not read yet, which is why the flush
            // is not optional: `rule:concurrency/a-drain-closes-a-connection-cleanly`'s clean close
            // is a client that got the whole answer, not one that got a truncated one.
            unsafe {
                FlushFileBuffers(self.handle);
                DisconnectNamedPipe(self.handle);
            }
        }
    }

    impl Read for Stream<'_> {
        /// Non-blocking, which is [`peeked`]'s doc and this type's own.
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            peeked(self.handle, buf)
        }
    }

    /// The peek that makes a read on a synchronous pipe non-blocking, and then the read it
    /// licenses.
    ///
    /// A synchronous pipe handle has no read timeout and no non-blocking mode worth having —
    /// `PIPE_NOWAIT` is documented as existing for 16-bit compatibility and not to be used — so
    /// "is there anything to read" is asked with [`PeekNamedPipe`] and answered before any byte is
    /// committed to. That is this platform's spelling of the `WouldBlock` the Unix half gets from
    /// the socket, and **both ends of a control connection need it**: [`Stream`]'s own doc is why
    /// the server's does, and [`super::connect`]'s is why the client's does.
    #[expect(
        unsafe_code,
        reason = "two `kernel32` calls over a handle the caller owns and keeps open across them; \
                  the peek writes one `u32` this frame owns, and the read fills a slice it holds \
                  mutably for at most its own length"
    )]
    fn peeked(handle: HANDLE, buf: &mut [u8]) -> io::Result<usize> {
        let mut waiting: u32 = 0;
        let peeked = unsafe {
            PeekNamedPipe(
                handle,
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                &mut waiting,
                std::ptr::null_mut(),
            )
        };
        if peeked == 0 {
            return closed(io::Error::last_os_error());
        }
        if waiting == 0 {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        let mut read: u32 = 0;
        let want = u32::try_from(buf.len()).unwrap_or(u32::MAX).min(waiting);
        let ok = unsafe {
            ReadFile(
                handle,
                buf.as_mut_ptr(),
                want,
                &mut read,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 {
            return closed(io::Error::last_os_error());
        }
        Ok(usize::try_from(read).expect("a read of at most this buffer's own length"))
    }

    /// `failed` as the end of input where that is what it is.
    ///
    /// A client that has closed its end arrives as a broken pipe on whichever call reaches for it
    /// next. The framing above this reads an end of input as a read of zero, the way every other
    /// transport spells it, so that is what a closed pipe is handed back as.
    fn closed(failed: io::Error) -> io::Result<usize> {
        match os_error(&failed) {
            Some(ERROR_BROKEN_PIPE) => Ok(0),
            _ => Err(failed),
        }
    }

    impl Write for Stream<'_> {
        #[expect(
            unsafe_code,
            reason = "one `kernel32` call reading a slice this frame holds, for at most its own \
                      length"
        )]
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            let mut wrote: u32 = 0;
            let want = u32::try_from(buf.len()).unwrap_or(u32::MAX);
            let ok = unsafe {
                WriteFile(
                    self.handle,
                    buf.as_ptr(),
                    want,
                    &mut wrote,
                    std::ptr::null_mut(),
                )
            };
            if ok == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(usize::try_from(wrote).expect("a write of at most this buffer's own length"))
        }

        /// Nothing is held back: a write has reached the pipe by the time it returns. The flush
        /// that waits for the *client* to have read it is [`Stream`]'s drop, because that is where
        /// waiting for a peer belongs rather than in the middle of a response being written.
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    /// A client's end of the endpoint: the pipe instance opened for reading and writing, read
    /// through the same peek the server's stream is and non-blocking for the reason
    /// [`super::connect`] gives.
    #[derive(Debug)]
    pub struct Client(std::fs::File);

    impl Read for Client {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            peeked(self.0.as_raw_handle().cast(), buf)
        }
    }

    impl Write for Client {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.write(buf)
        }

        /// Nothing is held back: a write has reached the pipe by the time it returns, which is
        /// [`Stream`]'s note from the other end of the same object.
        fn flush(&mut self) -> io::Result<()> {
            self.0.flush()
        }
    }

    /// Whichever `WIN32_ERROR` `failed` carries, in the type the constants naming one are.
    fn os_error(failed: &io::Error) -> Option<u32> {
        failed
            .raw_os_error()
            .and_then(|raw| u32::try_from(raw).ok())
    }

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

    /// The client's connect: open the instance, and wait for it if another client is being answered
    /// on it.
    ///
    /// The wait is what makes this the Unix half's listen backlog rather than a refusal a client
    /// would have to retry for itself — the endpoint holds one instance on purpose, so "busy" is
    /// the ordinary state of a second operation arriving and not an error about the server. It is a
    /// loop because the wait answers that an instance is *free* and not that this client has it:
    /// the server disconnecting and this client opening are two calls, and anything may arrive
    /// between them. [`BUSY_WAIT`] bounds the whole loop rather than one turn of it.
    #[expect(
        unsafe_code,
        reason = "one `kernel32` call over a NUL-terminated name this frame owns, which returns \
                  before the buffer does"
    )]
    pub(super) fn connect(name: &Path) -> io::Result<Client> {
        let wide: Vec<u16> = name.as_os_str().encode_wide().chain(Some(0)).collect();
        let until = Instant::now() + BUSY_WAIT;
        loop {
            match std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(name)
            {
                Err(busy) if os_error(&busy) == Some(ERROR_PIPE_BUSY) => {
                    let left =
                        u32::try_from(until.saturating_duration_since(Instant::now()).as_millis())
                            .unwrap_or(u32::MAX);
                    if left == 0 || unsafe { WaitNamedPipeW(wide.as_ptr(), left) } == 0 {
                        return Err(busy);
                    }
                }
                other => return other.map(Client),
            }
        }
    }
}
