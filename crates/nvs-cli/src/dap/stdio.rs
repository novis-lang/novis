//! The process's standard streams, taken from the program the adapter runs and kept for the
//! protocol (`rule:ide/stdout-belongs-to-the-protocol`).
//!
//! The launched program runs in the adapter's own process, so every write it makes to standard
//! output would land between two frames: an `echo`, a diagnostic, a panic message, a child process
//! that inherited the stream. A sink wired into the program's context would cover the first of
//! those and none of the rest. So the streams themselves are moved: [`Streams::take`] keeps a
//! private copy of the real stdin and stdout for the protocol, and points the process's stdout and
//! stderr at two pipes and its stdin at the null device. Whatever writes to "standard output" from
//! then on writes into a pipe, and the adapter turns what arrives there into `output` events. The
//! program cannot read the client's frames off stdin either.
//!
//! **What it spends:** two pipes and two threads that read them, for the life of the adapter.

use std::fs::{File, OpenOptions};
use std::io::{self, PipeReader, PipeWriter, Write};
use std::sync::OnceLock;

/// The protocol's own copies of the real streams, and the pipes the process's streams now point
/// into.
pub(crate) struct Streams {
    /// The client's frames.
    pub(crate) input: File,
    /// The adapter's frames, and nothing else.
    pub(crate) output: File,
    /// The adapter's own log: the real stderr, which the client shows as the adapter's log.
    pub(crate) log: File,
    /// What the program writes to stdout and to stderr, in that order.
    pub(crate) program_out: PipeReader,
    pub(crate) program_err: PipeReader,
    /// The write ends the process's stdout and stderr point at, closed by [`Pipes::close`].
    pub(crate) pipes: Pipes,
}

/// The write ends of the program's pipes, and the null device the streams move to when they
/// close.
pub(crate) struct Pipes {
    out: PipeWriter,
    err: PipeWriter,
}

/// The null device the streams point at once they are taken from the program. Process-wide and
/// never closed, because Windows keeps the raw handle a standard stream is set to, so the device
/// stays open for as long as the process may write to it.
static NULL: OnceLock<File> = OnceLock::new();

fn null() -> io::Result<&'static File> {
    if let Some(null) = NULL.get() {
        return Ok(null);
    }
    let opened = OpenOptions::new()
        .read(true)
        .write(true)
        .open(if cfg!(windows) { "NUL" } else { "/dev/null" })?;
    Ok(NULL.get_or_init(|| opened))
}

impl Streams {
    /// Copies the real streams and points the process's streams away from them.
    ///
    /// # Errors
    ///
    /// A stream this process was started without, or a pipe or device the OS did not give.
    pub(crate) fn take() -> io::Result<Self> {
        let input = copy::stdin()?;
        let output = copy::stdout()?;
        let log = copy::stderr()?;
        let null = null()?;
        let (program_out, out) = io::pipe()?;
        let (program_err, err) = io::pipe()?;
        io::stdout().flush()?;
        io::stderr().flush()?;
        point::stdin(null)?;
        point::stdout(&out)?;
        point::stderr(&err)?;
        Ok(Self {
            input,
            output,
            log,
            program_out,
            program_err,
            pipes: Pipes { out, err },
        })
    }
}

impl Pipes {
    /// Points the process's stdout and stderr at the null device and closes the pipes, so their
    /// readers reach the end once every child process that inherited them has ended too.
    pub(crate) fn close(self) {
        let _ = io::stdout().flush();
        let _ = io::stderr().flush();
        // A stream that cannot be moved keeps its pipe open, and its reader then waits for the
        // process to end. That is a later `exited` event and never a byte on the protocol.
        let moved = null().and_then(|null| point::stdout(null).and_then(|()| point::stderr(null)));
        if moved.is_err() {
            std::mem::forget(self.out);
            std::mem::forget(self.err);
        }
    }
}

/// A private, close-on-exec copy of each real stream.
mod copy {
    use std::fs::File;
    use std::io;

    #[cfg(unix)]
    use std::os::fd::AsFd as AsStream;
    #[cfg(windows)]
    use std::os::windows::io::AsHandle as AsStream;

    fn of(stream: &impl AsStream) -> io::Result<File> {
        #[cfg(unix)]
        let owned = stream.as_fd().try_clone_to_owned()?;
        #[cfg(windows)]
        let owned = stream.as_handle().try_clone_to_owned()?;
        Ok(File::from(owned))
    }

    pub(super) fn stdin() -> io::Result<File> {
        of(&io::stdin())
    }

    pub(super) fn stdout() -> io::Result<File> {
        of(&io::stdout())
    }

    pub(super) fn stderr() -> io::Result<File> {
        of(&io::stderr())
    }
}

/// Points one of the process's standard streams at `to`.
#[cfg(unix)]
mod point {
    use std::io;
    use std::os::fd::{AsFd, AsRawFd};

    fn point(stream: i32, to: &impl AsFd) -> io::Result<()> {
        #[expect(
            unsafe_code,
            reason = "`dup2` has no `std` spelling. Both descriptors are open for the whole call: \
                      `to` is borrowed, and the standard stream is replaced atomically, so a \
                      write racing it lands on the old or the new file and on nothing else."
        )]
        let done = unsafe { libc::dup2(to.as_fd().as_raw_fd(), stream) };
        if done == -1 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    pub(super) fn stdin(to: &impl AsFd) -> io::Result<()> {
        point(libc::STDIN_FILENO, to)
    }

    pub(super) fn stdout(to: &impl AsFd) -> io::Result<()> {
        point(libc::STDOUT_FILENO, to)
    }

    pub(super) fn stderr(to: &impl AsFd) -> io::Result<()> {
        point(libc::STDERR_FILENO, to)
    }
}

/// Points one of the process's standard streams at `to`. Rust's standard streams ask the OS for
/// the handle on every write, so the next write already goes to `to`.
#[cfg(windows)]
mod point {
    use std::io;
    use std::os::windows::io::{AsHandle, AsRawHandle};

    use windows_sys::Win32::System::Console::{
        STD_ERROR_HANDLE, STD_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, SetStdHandle,
    };

    fn point(stream: STD_HANDLE, to: &impl AsHandle) -> io::Result<()> {
        #[expect(
            unsafe_code,
            reason = "`SetStdHandle` has no `std` spelling. It stores the raw handle and takes no \
                      ownership; every caller passes a handle that stays open for as long as the \
                      stream points at it (`Pipes` holds both pipes and the null device)."
        )]
        let done = unsafe { SetStdHandle(stream, to.as_handle().as_raw_handle()) };
        if done == 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    pub(super) fn stdin(to: &impl AsHandle) -> io::Result<()> {
        point(STD_INPUT_HANDLE, to)
    }

    pub(super) fn stdout(to: &impl AsHandle) -> io::Result<()> {
        point(STD_OUTPUT_HANDLE, to)
    }

    pub(super) fn stderr(to: &impl AsHandle) -> io::Result<()> {
        point(STD_ERROR_HANDLE, to)
    }
}
