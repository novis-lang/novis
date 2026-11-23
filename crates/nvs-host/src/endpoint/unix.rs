//! The reload endpoint on Linux and macOS: a blocking Unix socket in the private `run/` folder.
//!
//! [`bind`] listens under the staged name, sets the socket file to `0600` and renames it into
//! place, so the path exists only once it accepts. [`accept`] reads each peer's uid from the
//! kernel (`SO_PEERCRED` on Linux, `getpeereid` on macOS) and closes every peer that is neither
//! this process's effective uid nor `0`; a uid it cannot read closes the peer too. A silent peer
//! is ended by the caller's deadline through a [`Breaker`], which shuts the socket down. Cost: one
//! socket and two syscalls per connection, nothing on the request path.

use std::io;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::io::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;

#[derive(Debug)]
pub(super) struct Listener(UnixListener);

#[derive(Debug)]
pub(super) struct Stream(UnixStream);

/// A second handle to a connection's socket, which shuts it down.
#[derive(Debug)]
pub(super) struct Breaker(UnixStream);

/// A second handle to `stream`'s socket.
pub(super) fn breaker(stream: &Stream) -> io::Result<Breaker> {
    stream.0.try_clone().map(Breaker)
}

/// Shuts the socket down both ways: a blocked read returns the end, a blocked write fails, and
/// every later one does the same at once.
pub(super) fn break_now(breaker: &Breaker) {
    let _ = breaker.0.shutdown(std::net::Shutdown::Both);
}

/// Listens at `staged`, then renames it to `path`. A `path` that already exists is
/// `AlreadyExists`, and every failure after the bind removes `staged`. A path longer than
/// `sun_path` fails with the platform's error.
pub(super) fn bind(path: &Path, staged: &Path) -> io::Result<Listener> {
    let listener = UnixListener::bind(staged)?;
    let placed = (|| {
        std::fs::set_permissions(staged, std::fs::Permissions::from_mode(0o600))?;
        // The check and the rename are two steps; the pid-and-start name keeps another
        // process off this path between them.
        if path.symlink_metadata().is_ok() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "the reload endpoint's path already exists",
            ));
        }
        std::fs::rename(staged, path)
    })();
    if let Err(err) = placed {
        let _ = std::fs::remove_file(staged);
        return Err(err);
    }
    Ok(Listener(listener))
}

/// Waits for the next peer that is this account or root, closing every other one.
pub(super) fn accept(listener: &Listener) -> io::Result<Stream> {
    let own = current_euid();
    loop {
        let stream = match listener.0.accept() {
            Ok((stream, _)) => stream,
            Err(err)
                if matches!(
                    err.kind(),
                    io::ErrorKind::Interrupted | io::ErrorKind::ConnectionAborted
                ) =>
            {
                continue;
            }
            Err(err) => return Err(err),
        };
        let allowed = peer_uid(&stream).is_ok_and(|uid| uid == own || uid == 0);
        if !allowed {
            continue;
        }
        return Ok(Stream(stream));
    }
}

/// Connects to the socket at `path`, with no timeout: the reply waits for the compiles.
///
/// A path that is a regular file gives `ECONNREFUSED` on Linux and `ENOTSOCK` on macOS; both
/// come back as `ConnectionRefused`, so `is_stale` reads either one.
pub(super) fn connect(path: &Path) -> io::Result<Stream> {
    match UnixStream::connect(path) {
        Ok(stream) => Ok(Stream(stream)),
        Err(err) if err.raw_os_error() == Some(libc::ENOTSOCK) => Err(io::Error::new(
            io::ErrorKind::ConnectionRefused,
            "the reload endpoint's path is not a socket",
        )),
        Err(err) => Err(err),
    }
}

impl io::Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.0.read(buf)
    }
}

impl io::Write for Stream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

#[expect(
    unsafe_code,
    reason = "`geteuid` has no `std` spelling. It takes no arguments, touches no memory and \
              cannot fail."
)]
fn current_euid() -> libc::uid_t {
    // SAFETY: `geteuid` reads the calling process's effective uid and has no preconditions.
    unsafe { libc::geteuid() }
}

#[cfg(target_os = "linux")]
fn peer_uid(stream: &UnixStream) -> io::Result<libc::uid_t> {
    let mut cred = libc::ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut len = libc::socklen_t::try_from(std::mem::size_of::<libc::ucred>())
        .map_err(|_| io::Error::other("ucred does not fit a socklen_t"))?;
    #[expect(
        unsafe_code,
        reason = "`SO_PEERCRED` is the kernel's record of the peer's uid, and `std` exposes it \
                  only on nightly. The descriptor is the caller's live socket, the out-parameter \
                  is a stack `ucred` of exactly the length passed, and nothing outlives the call."
    )]
    // SAFETY: `cred` and `len` are live, exclusively borrowed stack values; `len` is the size of
    // `cred`, so the kernel writes within it.
    let got = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&raw mut cred).cast::<libc::c_void>(),
            &raw mut len,
        )
    };
    if got != 0 {
        return Err(io::Error::last_os_error());
    }
    if usize::try_from(len).ok() != Some(std::mem::size_of::<libc::ucred>()) {
        return Err(io::Error::other("SO_PEERCRED returned a short ucred"));
    }
    Ok(cred.uid)
}

#[cfg(target_os = "macos")]
fn peer_uid(stream: &UnixStream) -> io::Result<libc::uid_t> {
    let mut uid: libc::uid_t = 0;
    let mut gid: libc::gid_t = 0;
    #[expect(
        unsafe_code,
        reason = "`getpeereid` is the kernel's record of the peer's uid, and `std` exposes it \
                  only on nightly. The descriptor is the caller's live socket, the out-parameters \
                  are stack values, and nothing outlives the call."
    )]
    // SAFETY: `uid` and `gid` are live, exclusively borrowed stack values of the types the call
    // writes.
    let got = unsafe { libc::getpeereid(stream.as_raw_fd(), &raw mut uid, &raw mut gid) };
    if got != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(uid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};

    #[test]
    fn a_line_goes_both_ways_through_the_socket() {
        // `nvs_repo::socket` stops the test when the path is too long for `sun_path`.
        let (dir, path) = nvs_repo::socket("s");
        let listener = bind(&path, &dir.join("s.new")).unwrap();
        assert!(!dir.join("s.new").exists());
        let mode = path.symlink_metadata().unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        let server = std::thread::spawn(move || {
            let stream = accept(&listener).unwrap();
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            assert_eq!(line, "ping\n");
            reader.into_inner().write_all(b"pong\n").unwrap();
        });
        let mut client = connect(&path).unwrap();
        client.write_all(b"ping\n").unwrap();
        let mut reader = BufReader::new(client);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        assert_eq!(line, "pong\n");
        server.join().unwrap();
    }

    #[test]
    fn binding_over_an_existing_path_is_already_exists() {
        let (dir, path) = nvs_repo::socket("s");
        std::fs::write(&path, b"").unwrap();
        let err = bind(&path, &dir.join("s.new")).err().unwrap();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert!(!dir.join("s.new").exists());
    }

    #[test]
    fn connecting_to_a_regular_file_is_stale() {
        let (_dir, path) = nvs_repo::socket("s");
        std::fs::write(&path, b"").unwrap();
        let err = connect(&path).err().unwrap();
        assert!(super::super::is_stale(&err), "{err:?}");
    }
}
