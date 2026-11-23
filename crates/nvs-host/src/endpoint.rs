//! The reload endpoint: the one private way into a running `nvs serve`
//! (`rule:config/a-reload-command-checks-now-and-waits`).
//!
//! Each serving process binds one [`Endpoint`] at `<data>/run/serve-<pid>-<start>`, and
//! `nvs reload` reaches every live one there through [`reach_all`]. It is a local channel and
//! nothing else: no TCP, no HTTP, and nothing a request can name, because the data folder is
//! outside every grant.
//!
//! **Only the serving account or an administrator gets a connection.**
//! - **Unix:** the path is a Unix socket in the private `run/` folder, and [`Endpoint::accept`]
//!   closes every peer whose uid is neither this process's effective uid nor `0` (`SO_PEERCRED`
//!   on Linux, `getpeereid` on macOS). The folder's mode keeps others out, and the uid check
//!   still holds where somebody loosened it.
//! - **Windows:** the path is a small file holding the name of a named pipe, `\\.\pipe\` and 128
//!   random bits. The pipe's DACL grants this account, `Administrators` and `SYSTEM` and nobody
//!   else. It rejects remote clients, and its first instance is created with
//!   `FILE_FLAG_FIRST_PIPE_INSTANCE`, so no other process can own the name first.
//!
//! **The path appears only once it can be reached.** [`Endpoint::bind`] makes the socket or the
//! pipe under a `.new` name and renames it into place, so a reader that finds the path and cannot
//! connect knows that nobody listens there. [`reach_all`] removes such a path: it belonged to a
//! process that ended without its drain.
//!
//! The endpoint blocks, on a thread of its own, and never on a core: a reload is rare, and a
//! blocked accept costs nothing between them. Cost: one listening socket or pipe instance per
//! process and one blocked thread, and nothing on the request path.

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[path = "endpoint/unix.rs"]
mod sys;
#[cfg(windows)]
#[path = "endpoint/windows.rs"]
mod sys;
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
mod sys {
    //! A platform with no endpoint: binding and connecting both return `Unsupported`.
    use std::io;
    use std::path::Path;

    #[derive(Debug)]
    pub(super) struct Listener;
    #[derive(Debug)]
    pub(super) struct Stream;
    #[derive(Debug)]
    pub(super) struct Breaker;

    fn unsupported() -> io::Error {
        io::Error::new(
            io::ErrorKind::Unsupported,
            "no reload endpoint on this platform",
        )
    }

    pub(super) fn bind(_path: &Path, _staged: &Path) -> io::Result<Listener> {
        Err(unsupported())
    }

    pub(super) fn accept(_listener: &Listener) -> io::Result<Stream> {
        Err(unsupported())
    }

    pub(super) fn connect(_path: &Path) -> io::Result<Stream> {
        Err(unsupported())
    }

    pub(super) fn breaker(_stream: &Stream) -> io::Result<Breaker> {
        Err(unsupported())
    }

    pub(super) fn break_now(_breaker: &Breaker) {}

    impl io::Read for Stream {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            Err(unsupported())
        }
    }

    impl io::Write for Stream {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Err(unsupported())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
}

/// The prefix of every endpoint's file name in `run/`.
pub const PREFIX: &str = "serve-";

/// The file name of this process's endpoint: `serve-<pid>-<start>`, with `start` the process's
/// start in milliseconds since the Unix epoch, so a restarted process with the same pid never
/// reuses a name.
#[must_use]
pub fn name(pid: u32, start_ms: u128) -> String {
    format!("{PREFIX}{pid}-{start_ms}")
}

/// Whether `file` is an endpoint's file name, `serve-<digits>-<digits>`, and not a `.new` file
/// a bind is still making.
#[must_use]
pub fn is_name(file: &str) -> bool {
    let Some(rest) = file.strip_prefix(PREFIX) else {
        return false;
    };
    let mut parts = rest.split('-');
    let digits = |part: Option<&str>| {
        part.is_some_and(|text| !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()))
    };
    digits(parts.next()) && digits(parts.next()) && parts.next().is_none()
}

/// The pid in an endpoint's file name, `serve-<pid>-<start>`, or `None` where the name is not one.
#[must_use]
pub fn pid_of(path: &Path) -> Option<u32> {
    let file = path.file_name()?.to_str()?;
    if !is_name(file) {
        return None;
    }
    file.strip_prefix(PREFIX)?.split('-').next()?.parse().ok()
}

/// One serving process's endpoint, bound and reachable.
#[derive(Debug)]
pub struct Endpoint {
    path: PathBuf,
    listener: sys::Listener,
}

impl Endpoint {
    /// Binds the endpoint at `path`, in a folder that exists and is private.
    ///
    /// # Errors
    ///
    /// The platform's error, including a path that already exists, a path too long for a Unix
    /// socket, and a platform with no endpoint (`Unsupported`).
    pub fn bind(path: &Path) -> io::Result<Self> {
        let mut staged = path.as_os_str().to_owned();
        staged.push(".new");
        let staged = PathBuf::from(staged);
        let _ = std::fs::remove_file(&staged);
        let listener = sys::bind(path, &staged)?;
        Ok(Self {
            path: path.to_path_buf(),
            listener,
        })
    }

    /// Where the endpoint is.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Waits for the next connection from the serving account or an administrator. A
    /// connection from any other account is closed and the wait goes on.
    ///
    /// # Errors
    ///
    /// The platform's error for the listening socket or pipe.
    pub fn accept(&self) -> io::Result<Connection> {
        sys::accept(&self.listener).map(Connection)
    }

    /// Removes the endpoint's path, so no command finds it again. A path that is already gone
    /// is success. The listening socket or pipe stays open until the process ends.
    ///
    /// # Errors
    ///
    /// The platform's error for the remove.
    pub fn remove(&self) -> io::Result<()> {
        match std::fs::remove_file(&self.path) {
            Err(err) if err.kind() != io::ErrorKind::NotFound => Err(err),
            _ => Ok(()),
        }
    }
}

/// One connection to an endpoint, on either side.
#[derive(Debug)]
pub struct Connection(sys::Stream);

impl Read for Connection {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

impl Write for Connection {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl Connection {
    /// A [`Breaker`] for this connection.
    ///
    /// # Errors
    ///
    /// The platform's error for duplicating the socket or the pipe handle.
    pub fn breaker(&self) -> io::Result<Breaker> {
        sys::breaker(&self.0).map(Breaker)
    }
}

/// Ends a connection's blocked read or write from another thread.
///
/// On Unix, [`Breaker::break_now`] shuts the socket down both ways, so the read or write that
/// waits now ends and every later one ends at once. On Windows it cancels the read or write that
/// waits now, and nothing more: a caller that cannot know whether the other thread waits yet calls
/// it again until that thread is done.
#[derive(Debug)]
pub struct Breaker(sys::Breaker);

impl Breaker {
    /// Ends the connection's blocked read or write. A connection that is closed already is no
    /// error.
    pub fn break_now(&self) {
        sys::break_now(&self.0);
    }
}

/// Connects to the endpoint at `path`.
///
/// On Windows, the process that serves the pipe must be the one whose pid is in the file's name
/// ([`pid_of`]). Any other process is `PermissionDenied`, which is not stale, so [`reach_all`]
/// reports it and leaves its file alone.
///
/// # Errors
///
/// [`is_stale`] is true of the error when the path exists and nobody listens there. Any other
/// error is the platform's.
pub fn connect(path: &Path) -> io::Result<Connection> {
    sys::connect(path).map(Connection)
}

/// Whether a [`connect`] error means that nobody listens at the path: a socket with no listener,
/// a file that is not a socket, or a name file whose pipe does not exist.
#[must_use]
pub fn is_stale(err: &io::Error) -> bool {
    matches!(
        err.kind(),
        io::ErrorKind::ConnectionRefused | io::ErrorKind::NotFound | io::ErrorKind::InvalidData
    )
}

/// Connects to every live endpoint in `run`, in name order, and removes every stale one.
///
/// An endpoint whose connect failed for another reason is returned with its error, so a caller
/// can report it. A folder that does not exist has no endpoint.
///
/// # Errors
///
/// The error of listing `run`.
pub fn reach_all(run: &Path) -> io::Result<Vec<(PathBuf, io::Result<Connection>)>> {
    let entries = match std::fs::read_dir(run) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err),
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_str().is_some_and(is_name))
        .map(|entry| entry.path())
        .collect();
    paths.sort();
    let mut reached = Vec::new();
    for path in paths {
        match connect(&path) {
            Err(err) if is_stale(&err) => {
                let _ = std::fs::remove_file(&path);
            }
            outcome => reached.push((path, outcome)),
        }
    }
    Ok(reached)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_endpoint_name_is_serve_pid_start() {
        assert_eq!(name(42, 1_700_000_000_000), "serve-42-1700000000000");
        assert!(is_name("serve-42-1700000000000"));
        assert!(!is_name("serve-42-1700000000000.new"));
        assert!(!is_name("serve-42"));
        assert!(!is_name("serve--1"));
        assert!(!is_name("other-42-1"));
    }

    #[test]
    fn the_pid_is_read_from_an_endpoint_name() {
        assert_eq!(pid_of(Path::new("run/serve-42-1700000000000")), Some(42));
        assert_eq!(pid_of(Path::new("run/serve-42-1.new")), None);
        assert_eq!(pid_of(Path::new("run/other-42-1")), None);
    }
}
