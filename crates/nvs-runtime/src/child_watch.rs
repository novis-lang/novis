//! The watcher: a second copy of the `nvs` binary that ends this process's
//! children on Linux and macOS when this process is killed or crashes.
//!
//! [`crate::child_start`]'s module doc owns why a child has to end with this
//! process and what each platform does about it. This module is the Linux and
//! macOS half of that: the watcher itself, and the one message this process
//! sends it for every child it starts.
//!
//! # How it runs
//!
//! The first child this process starts also starts the watcher, through the
//! same `std` spawn path every child takes, with no `pre_exec` hook, so a child
//! start stays a `posix_spawn` and no start waits on another thread. The
//! watcher is this binary run again with `WATCHER` in an otherwise empty
//! environment, in `/`, with null standard output and error. Its standard input
//! is one end of a Unix socket pair, and this process keeps the other end. `std`
//! opens both ends close-on-exec and `posix_spawn` duplicates the watcher's end
//! onto descriptor 0 of the watcher alone, so no other child ever holds either
//! end. A binary's `main` calls [`main_if_asked`] before anything else, which
//! runs the watcher in that copy and turns watching on in every other run.
//! A process whose `main` does not call it, such as a test binary, starts no
//! watcher, because running it again would not reach the watcher.
//!
//! The watcher ignores `SIGINT`, `SIGTERM`, `SIGHUP` and `SIGQUIT`, so a key
//! press that signals the terminal's whole process group ends `nvs` and leaves
//! the watcher to end the children. It reads its socket until end of file,
//! which comes when the last descriptor of this process's end closes: when this
//! process ends, however it ends. It then sends `SIGKILL` to every child it
//! still tracks, and exits. A watcher whose parent is already gone when it
//! starts reads what was sent, ends those children and exits. A watcher started
//! with anything but a socket on its standard input exits at once, so a program
//! that sets `WATCHER` for a child of its own gets no watcher out of it.
//!
//! The watcher works in batches. After each turn it sleeps for `BATCH` before
//! it waits again, and every message and child exit that arrived meanwhile is
//! handled in the next turn. A send while the watcher sleeps wakes no process,
//! so it costs the sending thread only the `sendmsg` call. A watcher that
//! waited again at once would be woken by every send and every child exit,
//! and the thread that sends pays for that wakeup while it holds the mutex
//! below, so every other thread starting a child waits behind it. A watcher
//! with nothing to handle waits with no timeout, so an idle `nvs` does not
//! wake it. The end of `nvs` is seen up to `BATCH` late.
//!
//! A child is sent to the watcher right after `spawn` returns, while its
//! `Child` is not yet waited for, so its process id cannot have been reused.
//! One `sendmsg` per child, under a mutex held only for that call. The first
//! child also waits for the watcher to start. A child that exits before
//! `nvs` is killed is dropped from the watcher's set when it exits, so the set
//! is the children that are still running.
//!
//! **Linux:** the message carries a pidfd for the child (`pidfd_open`) as
//! `SCM_RIGHTS`, tagged with the child's process id. The watcher waits on every
//! pidfd with `epoll`, closes one when it becomes readable (its child exited),
//! and at the end signals each one still open with `pidfd_send_signal`. A pidfd
//! names one process and never another, so no kill can reach a reused id.
//!
//! **macOS:** there is no pidfd, so the message is the child's process id. The
//! watcher registers `EVFILT_PROC` with `NOTE_EXIT` for it in a `kqueue` and
//! drops it when that fires. A registration that fails means the child already
//! exited. The watcher also registers this process's own exit, because `std`
//! sets close-on-exec on a socket after creating it on macOS, and a child
//! started by another thread in between holds this process's end open past its
//! death. At the end it reads every exit that is already queued, then sends
//! `SIGKILL` to each process id left. One window remains: a child that exits
//! at the moment `nvs` dies is reaped by `launchd`, and its id could be given
//! to a new process before the watcher's kill reaches it. Process ids are
//! handed out in sequence, so that needs the whole id space used up in that
//! moment.
//!
//! A child start never fails because of the watcher. When the watcher cannot
//! be started or reached, the child runs unwatched and `track` returns a
//! warning, which the caller writes to the log. When a send finds the watcher
//! gone, a new watcher is started and the child is sent to it. The children
//! the old watcher tracked are no longer ended when `nvs` is killed.
//!
//! **What it spends:** one watcher process per `nvs` process that has started
//! a child, until that process ends; one socket pair; one message per child
//! start, which waits in the socket for up to `BATCH`; one watcher wakeup per
//! `BATCH` while children start or exit, and none while nothing happens. A
//! send that finds the socket's buffer full waits for the watcher's next turn.
//! On Linux, one descriptor in the watcher per child that is still running or
//! exited in the last `BATCH`.

use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};

/// The environment variable that starts this binary as the watcher. Its value
/// is the process id of the `nvs` process that started it.
#[cfg(any(target_os = "linux", target_os = "macos"))]
const WATCHER: &str = "NVS_INTERNAL_CHILD_WATCHER";

/// Whether this process's `main` called [`main_if_asked`], which is what
/// makes running this binary again reach the watcher.
static ENABLED: AtomicBool = AtomicBool::new(false);

/// Runs the watcher when this process was started as one, and returns its exit
/// status. Otherwise turns watching on for every child this process starts,
/// and returns `None`.
///
/// A binary's `main` calls this first, before it reads its arguments, so a
/// bundled application's arguments never reach it and nothing else runs in
/// the watcher. On other platforms than Linux and macOS it returns `None` and
/// changes nothing.
#[must_use]
pub fn main_if_asked() -> Option<ExitCode> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    if let Some(parent) = std::env::var_os(WATCHER)
        .and_then(|value| value.to_str().and_then(|value| value.parse::<u32>().ok()))
    {
        return Some(unix::watch(parent));
    }
    ENABLED.store(true, Ordering::Relaxed);
    None
}

/// Sends `child` to the watcher, starting the watcher first if it is not
/// running. Returns a warning for the log when the child is not watched, or
/// when the watcher had to be started again. Windows has a job and no
/// watcher, so it never calls this.
#[cfg(not(windows))]
pub(crate) fn track(child: &std::process::Child) -> Option<String> {
    if !ENABLED.load(Ordering::Relaxed) {
        return None;
    }
    tracked(child)
}

/// [`track`] once watching is on.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn tracked(child: &std::process::Child) -> Option<String> {
    unix::track(child).err()
}

/// [`track`] on the Unix systems that have no watcher.
#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn tracked(_child: &std::process::Child) -> Option<String> {
    None
}

/// The half shared by Linux and macOS: starting the watcher, sending to it,
/// and its main loop. [`platform`] is what differs.
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod unix {
    use std::io;
    use std::os::fd::{AsFd, OwnedFd};
    use std::os::unix::fs::FileTypeExt;
    use std::os::unix::net::UnixStream;
    use std::path::PathBuf;
    use std::process::{Child, Command, ExitCode, Stdio};
    use std::sync::{Mutex, PoisonError};
    use std::time::Duration;

    #[cfg(target_os = "linux")]
    pub(super) use super::linux as platform;
    #[cfg(target_os = "macos")]
    pub(super) use super::macos as platform;

    /// This process's end of the socket, and the watcher on the other end.
    #[derive(Debug)]
    struct Link {
        socket: UnixStream,
        watcher: Child,
    }

    /// The running watcher, or `None` before the first child and after one
    /// was found gone and could not be started again.
    static LINK: Mutex<Option<Link>> = Mutex::new(None);

    /// [`super::track`] on Linux and macOS.
    pub(super) fn track(child: &Child) -> Result<(), String> {
        let note = libc::pid_t::try_from(child.id())
            .map_err(io::Error::other)
            .and_then(platform::note)
            .map_err(|err| unwatched(&err))?;
        let mut link = LINK.lock().unwrap_or_else(PoisonError::into_inner);
        let mut restarted = false;
        loop {
            if link.is_none() {
                *link = Some(start().map_err(|err| unwatched(&err))?);
            }
            let Some(current) = link.as_ref() else {
                return Err(unwatched(&io::Error::other("the watcher was not kept")));
            };
            match platform::send(&current.socket, &note) {
                Ok(()) if restarted => return Err(RESTARTED.to_owned()),
                Ok(()) => return Ok(()),
                Err(err) if gone(&err) && !restarted => {
                    if let Some(mut old) = link.take() {
                        // Its end of the socket is closed, so it has exited
                        // or is exiting, and this reaps it. It is killed
                        // before this end closes, so a watcher still running
                        // never reads that end of file and kills live
                        // children.
                        let _ = old.watcher.kill();
                        let _ = old.watcher.wait();
                    }
                    restarted = true;
                }
                Err(err) => return Err(unwatched(&err)),
            }
        }
    }

    /// The warning for a child sent to a new watcher because the old one was
    /// gone.
    const RESTARTED: &str = "the process that stops child processes when `nvs` is killed had \
                             exited, so a new one was started. Child processes started before \
                             this are not stopped if `nvs` is killed.";

    /// The warning for a child the watcher does not know about.
    fn unwatched(err: &io::Error) -> String {
        format!("a child process was started, but it is not stopped if `nvs` is killed: {err}")
    }

    /// Whether a send failed because the watcher's end of the socket is
    /// closed.
    fn gone(err: &io::Error) -> bool {
        matches!(
            err.kind(),
            io::ErrorKind::BrokenPipe
                | io::ErrorKind::ConnectionReset
                | io::ErrorKind::NotConnected
        )
    }

    /// Starts a watcher and connects it to this process.
    fn start() -> io::Result<Link> {
        let (socket, theirs) = UnixStream::pair()?;
        let watcher = Command::new(this_binary()?)
            .env_clear()
            .env(super::WATCHER, std::process::id().to_string())
            .current_dir("/")
            .stdin(Stdio::from(OwnedFd::from(theirs)))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        Ok(Link { socket, watcher })
    }

    /// The file this process runs. On Linux, a binary that was replaced on
    /// disk while this process ran is still reachable as `/proc/self/exe`.
    fn this_binary() -> io::Result<PathBuf> {
        let exe = std::env::current_exe()?;
        #[cfg(target_os = "linux")]
        if !exe.exists() {
            return Ok(PathBuf::from("/proc/self/exe"));
        }
        Ok(exe)
    }

    /// The watcher's whole run: [`super::main_if_asked`] returns what this
    /// returns as the process's exit status.
    pub(super) fn watch(parent: u32) -> ExitCode {
        for signal in [libc::SIGINT, libc::SIGTERM, libc::SIGHUP, libc::SIGQUIT] {
            #[expect(
                unsafe_code,
                reason = "`signal` with `SIG_IGN` installs no handler, so no code of this \
                          process runs in signal context. This runs before any thread is started."
            )]
            unsafe {
                libc::signal(signal, libc::SIG_IGN);
            }
        }
        let Some(socket) = socket_on_standard_input() else {
            return ExitCode::FAILURE;
        };
        let Ok(mut watch) = platform::Watch::new(socket, parent) else {
            return ExitCode::FAILURE;
        };
        // After `Watch::new`, which on macOS has registered for `parent`'s exit:
        // a parent that exits from here on is seen by the watch.
        let served = if std::os::unix::process::parent_id() == parent {
            serve(&mut watch)
        } else {
            watch.drain()
        };
        match served {
            Ok(()) => {
                watch.end();
                ExitCode::SUCCESS
            }
            // The watch cannot go on. Ending the children here would end
            // ones `nvs` is still using, so they are left, and `nvs` finds
            // this watcher gone at its next child start.
            Err(_) => ExitCode::FAILURE,
        }
    }

    /// How long the watcher sleeps after a turn before it waits again. What
    /// arrives meanwhile is handled together in the next turn.
    const BATCH: Duration = Duration::from_millis(2);

    /// How many events one turn takes from the kernel. The socket's messages
    /// are one event however many are waiting; each child exit is one.
    pub(super) const EVENTS: usize = 256;

    /// Turns `watch` until `nvs` is gone, sleeping [`BATCH`] after each turn.
    fn serve(watch: &mut platform::Watch) -> io::Result<()> {
        while watch.turn(None)? {
            std::thread::sleep(BATCH);
        }
        Ok(())
    }

    /// Descriptor 0 as a Unix socket, or `None` when it is something else.
    fn socket_on_standard_input() -> Option<UnixStream> {
        let fd = io::stdin().as_fd().try_clone_to_owned().ok()?;
        let file = std::fs::File::from(fd);
        if !file.metadata().ok()?.file_type().is_socket() {
            return None;
        }
        Some(UnixStream::from(OwnedFd::from(file)))
    }

    /// The last OS error as an `io::Error` when `result` is negative.
    pub(super) fn checked<T: PartialOrd + Default>(result: T) -> io::Result<T> {
        if result < T::default() {
            Err(io::Error::last_os_error())
        } else {
            Ok(result)
        }
    }
}

/// The Linux watcher: one pidfd per child, waited on with `epoll`.
#[cfg(target_os = "linux")]
mod linux {
    use std::collections::HashMap;
    use std::io;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    use super::unix::{EVENTS, checked};

    /// What this process sends for one child: its process id as the tag, and
    /// a pidfd for it.
    #[derive(Debug)]
    pub(crate) struct Note {
        pid: libc::pid_t,
        pidfd: OwnedFd,
    }

    /// Opens a pidfd for the child `pid`, which this process has not waited
    /// for yet.
    pub(crate) fn note(pid: libc::pid_t) -> io::Result<Note> {
        #[expect(
            unsafe_code,
            reason = "`pidfd_open` takes a process id and flags by value and returns a new \
                      descriptor or `-1`. It has no `libc` wrapper, so it is called by number."
        )]
        let raw = checked(unsafe {
            libc::syscall(libc::SYS_pidfd_open, libc::c_long::from(pid), NO_FLAGS)
        })?;
        let raw = RawFd::try_from(raw).map_err(io::Error::other)?;
        #[expect(
            unsafe_code,
            reason = "`pidfd_open` succeeded, so `raw` is a new descriptor that nothing else \
                      owns. The kernel opens it close-on-exec."
        )]
        let pidfd = unsafe { OwnedFd::from_raw_fd(raw) };
        Ok(Note { pid, pidfd })
    }

    /// The flags argument of the two pidfd calls.
    const NO_FLAGS: libc::c_long = 0;

    /// The size of one descriptor, as the `CMSG_*` functions take it. A
    /// `RawFd` is an `i32` on every target.
    const FD_SIZE: libc::c_uint = 4;

    /// One `sendmsg` of `note`: four bytes of process id, with its pidfd
    /// attached.
    pub(crate) fn send(socket: &UnixStream, note: &Note) -> io::Result<()> {
        let bytes = note.pid.to_ne_bytes();
        let mut iov = libc::iovec {
            iov_base: bytes.as_ptr().cast_mut().cast(),
            iov_len: bytes.len(),
        };
        // `u64`s, so the buffer has the alignment a `cmsghdr` needs.
        let mut control = [0_u64; 4];
        #[expect(
            unsafe_code,
            reason = "`CMSG_SPACE` and `CMSG_LEN` only compute sizes from their argument."
        )]
        let (space, length) = unsafe { (libc::CMSG_SPACE(FD_SIZE), libc::CMSG_LEN(FD_SIZE)) };
        if usize::try_from(space).map_err(io::Error::other)? > size_of_val(&control) {
            return Err(io::Error::other(
                "a descriptor does not fit the control buffer",
            ));
        }
        #[expect(
            unsafe_code,
            reason = "an all-zero `msghdr` is a valid one with nothing in it: null pointers and \
                      zero lengths."
        )]
        let mut message: libc::msghdr = unsafe { std::mem::zeroed() };
        message.msg_iov = &raw mut iov;
        message.msg_iovlen = 1;
        message.msg_control = control.as_mut_ptr().cast();
        message.msg_controllen = widen(space)?;
        #[expect(
            unsafe_code,
            reason = "`message` points at `control`, which is aligned for a `cmsghdr` and at \
                      least `CMSG_SPACE` of one descriptor long, so `CMSG_FIRSTHDR` is a header \
                      inside it and `CMSG_DATA` has room for the descriptor, written unaligned."
        )]
        unsafe {
            let header = libc::CMSG_FIRSTHDR(&raw const message);
            (*header).cmsg_level = libc::SOL_SOCKET;
            (*header).cmsg_type = libc::SCM_RIGHTS;
            (*header).cmsg_len = widen(length)?;
            std::ptr::write_unaligned(
                libc::CMSG_DATA(header).cast::<RawFd>(),
                note.pidfd.as_raw_fd(),
            );
        }
        loop {
            #[expect(
                unsafe_code,
                reason = "`message` and everything it points at, `bytes`, `iov` and `control`, \
                          live on this stack frame for the whole call, which only reads them."
            )]
            let sent = unsafe {
                libc::sendmsg(socket.as_raw_fd(), &raw const message, libc::MSG_NOSIGNAL)
            };
            match checked(sent) {
                Ok(sent) if usize::try_from(sent).ok() == Some(bytes.len()) => return Ok(()),
                Ok(_) => return Err(io::Error::other("the message to the watcher was cut short")),
                Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
                Err(err) => return Err(err),
            }
        }
    }

    /// `value` as the platform's type for a length field, which is `usize`
    /// with glibc and `u32` with musl.
    fn widen<T: TryFrom<libc::c_uint>>(value: libc::c_uint) -> io::Result<T> {
        T::try_from(value).map_err(|_| io::Error::other("a length does not fit"))
    }

    /// The `epoll` key of the socket. Every other key is a pidfd's number.
    const SOCKET: u64 = u64::MAX;

    /// The watcher's state: the socket from `nvs`, and a pidfd for every
    /// child that has not exited.
    #[derive(Debug)]
    pub(crate) struct Watch {
        poll: OwnedFd,
        socket: UnixStream,
        live: HashMap<RawFd, OwnedFd>,
    }

    impl Watch {
        /// A watch over `socket`. `parent` is not needed on Linux, where the
        /// socket's end of file is the only way `nvs` is seen to end.
        pub(crate) fn new(socket: UnixStream, parent: u32) -> io::Result<Self> {
            let _ = parent;
            socket.set_nonblocking(true)?;
            #[expect(
                unsafe_code,
                reason = "`epoll_create1` takes flags by value and returns a new descriptor or \
                          `-1`."
            )]
            let raw = checked(unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) })?;
            #[expect(
                unsafe_code,
                reason = "`epoll_create1` succeeded, so `raw` is a new descriptor that nothing \
                          else owns."
            )]
            let poll = unsafe { OwnedFd::from_raw_fd(raw) };
            let watch = Self {
                poll,
                socket,
                live: HashMap::new(),
            };
            watch.listen(watch.socket.as_raw_fd(), SOCKET)?;
            Ok(watch)
        }

        /// Adds `fd` to the `epoll` set under `key`.
        fn listen(&self, fd: RawFd, key: u64) -> io::Result<()> {
            let mut event = libc::epoll_event {
                events: libc::EPOLLIN.cast_unsigned(),
                u64: key,
            };
            #[expect(
                unsafe_code,
                reason = "both descriptors are open for the call, and `event` is a stack value \
                          the call reads and does not keep."
            )]
            checked(unsafe {
                libc::epoll_ctl(
                    self.poll.as_raw_fd(),
                    libc::EPOLL_CTL_ADD,
                    fd,
                    &raw mut event,
                )
            })?;
            Ok(())
        }

        /// Waits up to `timeout`, or with no limit for `None`, and handles what
        /// arrived. Returns `false` once `nvs` is gone.
        pub(crate) fn turn(&mut self, timeout: Option<Duration>) -> io::Result<bool> {
            let mut events = [libc::epoll_event { events: 0, u64: 0 }; EVENTS];
            let wait = timeout.map_or(-1, |timeout| {
                libc::c_int::try_from(timeout.as_millis()).unwrap_or(libc::c_int::MAX)
            });
            #[expect(
                unsafe_code,
                reason = "`events` is a stack array of the length passed, which the call fills \
                          and does not keep."
            )]
            let ready = unsafe {
                libc::epoll_wait(
                    self.poll.as_raw_fd(),
                    events.as_mut_ptr(),
                    libc::c_int::try_from(events.len()).unwrap_or(1),
                    wait,
                )
            };
            let ready = match checked(ready) {
                Ok(ready) => usize::try_from(ready).unwrap_or(0),
                Err(err) if err.kind() == io::ErrorKind::Interrupted => return Ok(true),
                Err(err) => return Err(err),
            };
            let mut open = true;
            for event in &events[..ready] {
                let key = event.u64;
                if key == SOCKET {
                    open &= self.read()?;
                } else if let Ok(fd) = RawFd::try_from(key) {
                    // Its child exited. Closing the pidfd also takes it out
                    // of the `epoll` set.
                    self.live.remove(&fd);
                }
            }
            Ok(open)
        }

        /// Takes every pidfd waiting on the socket. Returns `false` at end of
        /// file.
        fn read(&mut self) -> io::Result<bool> {
            loop {
                match receive(&self.socket) {
                    Ok(None) => return Ok(false),
                    Ok(Some(pidfds)) => {
                        for pidfd in pidfds {
                            let raw = pidfd.as_raw_fd();
                            self.listen(raw, u64::try_from(raw).map_err(io::Error::other)?)?;
                            self.live.insert(raw, pidfd);
                        }
                    }
                    Err(err) if err.kind() == io::ErrorKind::WouldBlock => return Ok(true),
                    Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
                    Err(err) => return Err(err),
                }
            }
        }

        /// Takes what is waiting on the socket, for a watcher whose parent is
        /// already gone.
        pub(crate) fn drain(&mut self) -> io::Result<()> {
            self.read().map(drop)
        }

        /// How many children are tracked.
        #[cfg(test)]
        pub(crate) fn tracked(&self) -> usize {
            self.live.len()
        }

        /// Sends `SIGKILL` to every child still tracked.
        pub(crate) fn end(self) {
            for pidfd in self.live.values() {
                #[expect(
                    unsafe_code,
                    reason = "`pidfd_send_signal` takes an open pidfd, a signal number, a null \
                              `siginfo_t` pointer and flags. It has no `libc` wrapper, so it is \
                              called by number."
                )]
                unsafe {
                    libc::syscall(
                        libc::SYS_pidfd_send_signal,
                        libc::c_long::from(pidfd.as_raw_fd()),
                        libc::c_long::from(libc::SIGKILL),
                        std::ptr::null::<libc::siginfo_t>(),
                        NO_FLAGS,
                    );
                }
            }
        }
    }

    /// One `recvmsg` on `socket`: `Ok(None)` at end of file, and otherwise
    /// the pidfds that came with what was read. The kernel never joins two
    /// messages that carry descriptors into one read.
    fn receive(socket: &UnixStream) -> io::Result<Option<Vec<OwnedFd>>> {
        let mut bytes = [0_u8; 64];
        let mut iov = libc::iovec {
            iov_base: bytes.as_mut_ptr().cast(),
            iov_len: bytes.len(),
        };
        let mut control = [0_u64; 16];
        #[expect(
            unsafe_code,
            reason = "an all-zero `msghdr` is a valid one with nothing in it: null pointers and \
                      zero lengths."
        )]
        let mut message: libc::msghdr = unsafe { std::mem::zeroed() };
        message.msg_iov = &raw mut iov;
        message.msg_iovlen = 1;
        message.msg_control = control.as_mut_ptr().cast();
        message.msg_controllen =
            widen(libc::c_uint::try_from(size_of_val(&control)).map_err(io::Error::other)?)?;
        #[expect(
            unsafe_code,
            reason = "`message` and everything it points at live on this stack frame for the \
                      whole call, which writes into `bytes` and `control` within their lengths."
        )]
        let read = checked(unsafe {
            libc::recvmsg(
                socket.as_raw_fd(),
                &raw mut message,
                libc::MSG_CMSG_CLOEXEC | libc::MSG_DONTWAIT,
            )
        })?;
        let mut pidfds = Vec::new();
        #[expect(
            unsafe_code,
            reason = "`recvmsg` filled `control` and set `msg_controllen`, so `CMSG_FIRSTHDR` \
                      and `CMSG_NXTHDR` walk headers inside it and stop at its end. An \
                      `SCM_RIGHTS` header's data is `cmsg_len - CMSG_LEN(0)` bytes of \
                      descriptors this process now owns, read unaligned."
        )]
        unsafe {
            let empty = length(libc::CMSG_LEN(0));
            let mut header = libc::CMSG_FIRSTHDR(&raw const message);
            while !header.is_null() {
                if (*header).cmsg_level == libc::SOL_SOCKET
                    && (*header).cmsg_type == libc::SCM_RIGHTS
                {
                    let data = libc::CMSG_DATA(header).cast::<RawFd>();
                    let count =
                        length((*header).cmsg_len).saturating_sub(empty) / size_of::<RawFd>();
                    for index in 0..count {
                        let raw = std::ptr::read_unaligned(data.add(index));
                        pidfds.push(OwnedFd::from_raw_fd(raw));
                    }
                }
                header = libc::CMSG_NXTHDR(&raw const message, header);
            }
        }
        if read == 0 && pidfds.is_empty() {
            return Ok(None);
        }
        Ok(Some(pidfds))
    }

    /// A length field as a `usize`, whichever type the platform gives it.
    fn length<T>(value: T) -> usize
    where
        usize: TryFrom<T>,
    {
        usize::try_from(value).unwrap_or(0)
    }
}

/// The macOS watcher: one `EVFILT_PROC` registration per child, in a `kqueue`.
#[cfg(target_os = "macos")]
mod macos {
    use std::collections::HashSet;
    use std::io::{self, Read, Write};
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    use super::unix::checked;

    /// What this process sends for one child: its process id.
    pub(crate) type Note = libc::pid_t;

    /// The note for the child `pid`. It cannot fail here; the Linux arm can.
    pub(crate) fn note(pid: libc::pid_t) -> io::Result<Note> {
        Ok(pid)
    }

    /// One write of `note`'s four bytes. `std` sets `SO_NOSIGPIPE` on the
    /// socket, so a watcher that is gone is an error here and not a signal.
    pub(crate) fn send(mut socket: &UnixStream, note: &Note) -> io::Result<()> {
        socket.write_all(&note.to_ne_bytes())
    }

    /// The watcher's state: the socket from `nvs`, the parent it watches, and
    /// the process id of every child that has not exited.
    #[derive(Debug)]
    pub(crate) struct Watch {
        queue: OwnedFd,
        socket: UnixStream,
        parent: libc::pid_t,
        live: HashSet<libc::pid_t>,
        partial: Vec<u8>,
    }

    /// A `kevent` change or an empty slot for one.
    fn event(ident: usize, filter: i16, flags: u16, fflags: u32) -> libc::kevent {
        libc::kevent {
            ident,
            filter,
            flags,
            fflags,
            data: 0,
            udata: std::ptr::null_mut(),
        }
    }

    impl Watch {
        /// A watch over `socket`, registered for the exit of `parent`.
        pub(crate) fn new(socket: UnixStream, parent: u32) -> io::Result<Self> {
            socket.set_nonblocking(true)?;
            #[expect(
                unsafe_code,
                reason = "`kqueue` takes nothing and returns a new descriptor or `-1`."
            )]
            let raw = checked(unsafe { libc::kqueue() })?;
            #[expect(
                unsafe_code,
                reason = "`kqueue` succeeded, so `raw` is a new descriptor that nothing else \
                          owns. A `kqueue` is not inherited by a child."
            )]
            let queue = unsafe { OwnedFd::from_raw_fd(raw) };
            let watch = Self {
                queue,
                socket,
                parent: libc::pid_t::try_from(parent).map_err(io::Error::other)?,
                live: HashSet::new(),
                partial: Vec::new(),
            };
            let socket = usize::try_from(watch.socket.as_raw_fd()).map_err(io::Error::other)?;
            watch.change(event(socket, libc::EVFILT_READ, libc::EV_ADD, 0))?;
            // A parent that is already gone is seen by the caller's
            // `parent_id` check, so a failure here changes nothing.
            let _ = watch.exit_of(watch.parent);
            Ok(watch)
        }

        /// Applies one change to the queue.
        fn change(&self, change: libc::kevent) -> io::Result<()> {
            #[expect(
                unsafe_code,
                reason = "`change` is one stack value the call reads, and the event list is \
                          empty, so an error is returned as `-1`."
            )]
            checked(unsafe {
                libc::kevent(
                    self.queue.as_raw_fd(),
                    &raw const change,
                    1,
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null(),
                )
            })?;
            Ok(())
        }

        /// Registers for the exit of `pid`. Fails with `ESRCH` for a process
        /// that has already exited.
        fn exit_of(&self, pid: libc::pid_t) -> io::Result<()> {
            let ident = usize::try_from(pid).map_err(io::Error::other)?;
            self.change(event(
                ident,
                libc::EVFILT_PROC,
                libc::EV_ADD | libc::EV_ONESHOT,
                libc::NOTE_EXIT,
            ))
        }

        /// Waits up to `timeout`, or with no limit for `None`, and handles what
        /// arrived. Returns `false` once `nvs` is gone.
        pub(crate) fn turn(&mut self, timeout: Option<Duration>) -> io::Result<bool> {
            let mut events = [event(0, 0, 0, 0); super::unix::EVENTS];
            let limit = timeout.map(|timeout| libc::timespec {
                tv_sec: libc::time_t::try_from(timeout.as_secs()).unwrap_or(libc::time_t::MAX),
                tv_nsec: libc::c_long::from(timeout.subsec_nanos()),
            });
            let limit_ptr = limit.as_ref().map_or(std::ptr::null(), std::ptr::from_ref);
            #[expect(
                unsafe_code,
                reason = "`events` is a stack array of the length passed, which the call fills \
                          and does not keep, and `limit_ptr` is null or points at `limit`."
            )]
            let ready = unsafe {
                libc::kevent(
                    self.queue.as_raw_fd(),
                    std::ptr::null(),
                    0,
                    events.as_mut_ptr(),
                    libc::c_int::try_from(events.len()).unwrap_or(1),
                    limit_ptr,
                )
            };
            let ready = match checked(ready) {
                Ok(ready) => usize::try_from(ready).unwrap_or(0),
                Err(err) if err.kind() == io::ErrorKind::Interrupted => return Ok(true),
                Err(err) => return Err(err),
            };
            let mut open = true;
            for event in &events[..ready] {
                if event.flags & libc::EV_ERROR != 0 {
                    continue;
                }
                if event.filter == libc::EVFILT_READ {
                    open &= self.read()?;
                } else if event.filter == libc::EVFILT_PROC {
                    let Ok(pid) = libc::pid_t::try_from(event.ident) else {
                        continue;
                    };
                    if pid == self.parent {
                        open = false;
                    } else {
                        self.live.remove(&pid);
                    }
                }
            }
            Ok(open)
        }

        /// Takes every process id waiting on the socket. Returns `false` at
        /// end of file.
        fn read(&mut self) -> io::Result<bool> {
            let mut chunk = [0_u8; 256];
            let open = loop {
                match (&self.socket).read(&mut chunk) {
                    Ok(0) => break false,
                    Ok(read) => self.partial.extend_from_slice(&chunk[..read]),
                    Err(err) if err.kind() == io::ErrorKind::WouldBlock => break true,
                    Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
                    Err(err) => return Err(err),
                }
            };
            let whole = self.partial.len() / 4 * 4;
            let pids: Vec<libc::pid_t> = self.partial[..whole]
                .chunks_exact(4)
                .filter_map(|chunk| chunk.try_into().ok().map(libc::pid_t::from_ne_bytes))
                .collect();
            self.partial.drain(..whole);
            for pid in pids {
                // A child that has already exited has nothing left to end.
                if self.exit_of(pid).is_ok() {
                    self.live.insert(pid);
                }
            }
            Ok(open)
        }

        /// Takes what is waiting on the socket, for a watcher whose parent is
        /// already gone.
        pub(crate) fn drain(&mut self) -> io::Result<()> {
            self.read().map(drop)
        }

        /// How many children are tracked.
        #[cfg(test)]
        pub(crate) fn tracked(&self) -> usize {
            self.live.len()
        }

        /// Drops every child whose exit is already queued, then sends
        /// `SIGKILL` to each one left.
        pub(crate) fn end(mut self) {
            let _ = self.turn(Some(Duration::ZERO));
            for &pid in &self.live {
                #[expect(
                    unsafe_code,
                    reason = "`kill` takes a process id and a signal number by value."
                )]
                unsafe {
                    libc::kill(pid, libc::SIGKILL);
                }
            }
        }
    }
}

#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
mod tests {
    use std::os::unix::net::UnixStream;
    use std::os::unix::process::ExitStatusExt;
    use std::process::{Child, Stdio};
    use std::time::{Duration, Instant};

    use super::unix::platform::{Watch, note, send};

    /// A socket pair, and a watch over one end of it as the watcher has.
    fn watched() -> (UnixStream, Watch) {
        let (ours, theirs) = UnixStream::pair().expect("a socket pair opens");
        let watch = Watch::new(theirs, std::process::id()).expect("the watch starts");
        (ours, watch)
    }

    /// Sends `child` over `socket` as `nvs` does.
    fn register(socket: &UnixStream, child: &Child) {
        let pid = libc::pid_t::try_from(child.id()).expect("a process id fits");
        send(
            socket,
            &note(pid).expect("the child has not been waited for"),
        )
        .expect("the watch's end is open");
    }

    /// Turns `watch` until `done` holds, for at most twenty seconds.
    fn turn_until(watch: &mut Watch, done: impl Fn(&Watch) -> bool) {
        let started = Instant::now();
        while !done(watch) {
            assert!(
                started.elapsed() < Duration::from_secs(20),
                "the watch got there in time"
            );
            assert!(
                watch
                    .turn(Some(Duration::from_millis(100)))
                    .expect("the watch turns"),
                "the socket is still open"
            );
        }
    }

    /// A shell that runs until its standard input closes.
    fn until_closed() -> Child {
        nvs_repo::spawn("/bin/sh", &[])
            .args(["-c", "read line; exit 0"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .expect("`/bin/sh` starts")
    }

    /// A child that exits while `nvs` runs is dropped from the watcher's set,
    /// so the end of `nvs` sends it nothing.
    #[test]
    fn a_child_that_exits_is_no_longer_tracked() {
        let (ours, mut watch) = watched();
        let mut child = until_closed();
        register(&ours, &child);
        turn_until(&mut watch, |watch| watch.tracked() == 1);
        drop(child.stdin.take());
        turn_until(&mut watch, |watch| watch.tracked() == 0);
        let status = child.wait().expect("the child is reaped");
        assert!(status.success(), "the child exited on its own: {status:?}");
    }

    /// When the socket from `nvs` closes, the watch ends, and a child it still
    /// tracks is killed.
    #[test]
    fn a_closed_socket_ends_the_watch_and_kills_a_running_child() {
        let (ours, mut watch) = watched();
        let mut child = until_closed();
        register(&ours, &child);
        turn_until(&mut watch, |watch| watch.tracked() == 1);
        drop(ours);
        let started = Instant::now();
        while watch
            .turn(Some(Duration::from_millis(100)))
            .expect("the watch turns")
        {
            assert!(
                started.elapsed() < Duration::from_secs(20),
                "the watch sees the socket close"
            );
        }
        watch.end();
        let status = child.wait().expect("the child is reaped");
        assert_eq!(status.signal(), Some(libc::SIGKILL), "{status:?}");
    }

    /// The watcher is only started by a binary whose `main` asked for it.
    #[test]
    fn a_binary_that_did_not_ask_starts_no_watcher() {
        let mut child = until_closed();
        assert_eq!(super::track(&child), None);
        drop(child.stdin.take());
        child.wait().expect("the child is reaped");
    }
}
