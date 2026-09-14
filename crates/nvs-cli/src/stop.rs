//! What asks this process to stop: the operating system's terminating signals,
//! turned into the one drain
//! `rule:concurrency/a-drain-closes-a-connection-cleanly` already has.
//!
//! `SIGTERM` is what a service manager sends, `SIGINT` is a console's Ctrl-C,
//! and `SIGHUP` is the terminal this process was started from going away. All
//! three mean the same thing here, because the only other thing a signal could
//! have meant is a reload — and a reload is `nvs ctl reload` over the control
//! socket (`rule:config/one-local-control-socket`), never a signal. Windows has
//! no signals; its console control events are the same set of questions and get
//! the same answer, and a service's stop arrives through the SCM instead
//! ([`crate::service`] owns that half, which is not built yet).
//!
//! # A delivery is one call, and it is not made in the signal handler
//!
//! [`deliver`] is the whole of what a stop does: `Draining::process().begin()`,
//! the drain every other reader already has, and `STOPPING=1` to whatever
//! started this process ([`crate::service`]). From there the accept loops stop
//! accepting, `Core\Server::isDraining()` answers `true`, the health probe
//! answers `503`, and a program's `Core\Signal::onShutdown` handler runs at its
//! request's next safepoint (`nvs_stdlib::signal` owns that path).
//!
//! **On Unix it runs on a thread of this process's own, not in the handler.**
//! Beginning a drain takes a lock and fires the wakes registered under it, and
//! a signal delivered to a thread that already holds that lock would deadlock
//! the shutdown it is asking for. So the handler does one thing a signal
//! context may do — write a byte to a pipe — and the thread parked on the far
//! end of that pipe makes the call. What it spends is one thread and one pipe
//! for the life of the process, neither of which grows with anything.
//!
//! The handler leaves `errno` as `write` left it rather than saving and
//! restoring it. Preserving it needs a per-platform `errno` symbol, and what it
//! would buy is one accurate error message in the case where a thread was
//! between a failed syscall and reading it at the instant the process was told
//! to stop, once per process.
//!
//! Windows needs neither the thread nor the pipe: a console control handler is
//! called on an ordinary thread of its own, so it calls [`deliver`] itself.

use std::sync::OnceLock;

/// Installed once, and the answer kept so that a second call is the first
/// call's — a process has one disposition per signal, and installing twice
/// would replace a handler that is already correct.
static INSTALLED: OnceLock<Result<(), String>> = OnceLock::new();

/// Arms this process's terminating signals, so that one begins the drain.
///
/// Idempotent: the second call answers what the first one did and touches
/// nothing.
///
/// # Errors
///
/// The platform refused to install the handler, or the pipe or thread this
/// needs could not be made. A server that cannot be stopped gracefully is one
/// an operator's `systemctl stop` would have to kill mid-request, so it is a
/// refusal to start rather than a note.
pub(crate) fn on_termination() -> Result<(), String> {
    INSTALLED.get_or_init(platform::install).clone()
}

/// What a delivery ends in, and the whole of it.
///
/// The process's drain and not a handle handed down from the boot, because this
/// is called from a signal handler's thread which was given nothing: it is the
/// same bit either way (`nvs_runtime::drain`), and taking it here is what makes
/// that true by construction. The process's service manager is taken the same
/// way and for the same reason ([`crate::service::Notify::process`]).
///
/// The drain is begun before the manager is told, so `STOPPING=1` is a state
/// this process is already in rather than one it is about to enter.
pub(crate) fn deliver() {
    deliver_to(&nvs_server::Draining::process());
}

/// The stop itself, over the drain it begins.
///
/// The process's drain is [`deliver`]'s and is begun once for the life of a
/// process, so a case about what a stop *reports* takes a detached one: the
/// report is the same call either way, and the bit a signal sets stays a bit
/// only the case about signals has set.
pub(crate) fn deliver_to(draining: &nvs_server::Draining) {
    draining.begin();
    crate::service::Notify::process().state(crate::service::State::Stopping);
}

#[cfg(unix)]
mod platform {
    use std::ffi::c_int;
    use std::sync::atomic::{AtomicI32, Ordering};

    /// The write end of the pipe the handler pokes, or `-1` before there is
    /// one.
    ///
    /// An atomic rather than a `OnceLock` because a signal handler may read it:
    /// what a handler may touch is a lock-free load, and `-1` is the answer for
    /// a delivery that somehow arrived before the install finished.
    static WAKE: AtomicI32 = AtomicI32::new(-1);

    /// Every signal that means *stop*, and the whole list — the module docs own
    /// why a reload is not among them.
    const TERMINATING: [c_int; 3] = [libc::SIGTERM, libc::SIGINT, libc::SIGHUP];

    /// The pipe, the thread parked on it, then the handlers — in that order, so
    /// that a signal delivered the instant the first one is armed already has
    /// somewhere to go.
    pub(super) fn install() -> Result<(), String> {
        let (read, write) = pipe()?;
        WAKE.store(write, Ordering::Relaxed);
        std::thread::Builder::new()
            .name("nvs-stop".to_owned())
            .spawn(move || {
                if asked(read) {
                    super::deliver();
                }
            })
            .map_err(|error| {
                format!("could not start the thread that answers a terminating signal: {error}")
            })?;
        for signal in TERMINATING {
            arm(signal)?;
        }
        Ok(())
    }

    /// A pipe whose ends are both closed by an `exec` and whose write end never
    /// blocks: the handler writes one byte and the reader takes one, so a
    /// second delivery finding the pipe full is a stop that is already under
    /// way.
    #[expect(
        unsafe_code,
        reason = "`pipe` writes two descriptors into a two-element array, and \
                  `fcntl` sets a flag on one of them — neither has a safe \
                  spelling in `std`"
    )]
    fn pipe() -> Result<(c_int, c_int), String> {
        let mut ends = [0 as c_int; 2];
        // SAFETY: the platform writes exactly two descriptors into an array of
        // two, which this frame owns.
        if unsafe { libc::pipe(ends.as_mut_ptr()) } != 0 {
            return Err(format!(
                "could not make the pipe a terminating signal is answered over: {}",
                std::io::Error::last_os_error()
            ));
        }
        let [read, write] = ends;
        for (end, added) in [(read, libc::FD_CLOEXEC), (write, libc::FD_CLOEXEC)] {
            // SAFETY: `end` is a descriptor this process just made, and
            // `F_SETFD` takes the int this passes.
            let set = unsafe { libc::fcntl(end, libc::F_SETFD, added) };
            if set == -1 {
                return Err(format!(
                    "could not close-on-exec the pipe a terminating signal is answered over: {}",
                    std::io::Error::last_os_error()
                ));
            }
        }
        // SAFETY: as above, on the write end alone — the read is the blocking
        // one and stays so.
        if unsafe { libc::fcntl(write, libc::F_SETFL, libc::O_NONBLOCK) } == -1 {
            return Err(format!(
                "could not make the terminating signal's pipe non-blocking: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok((read, write))
    }

    /// Blocks until a delivery writes its byte, and says whether one did.
    ///
    /// `false` is the pipe failing or its write end being gone, and the caller
    /// then begins no drain: a read that ended for its own reason is not this
    /// process being asked to stop, and stopping a server on one would be the
    /// fail-open direction of exactly the wrong bit.
    #[expect(
        unsafe_code,
        reason = "a blocking read of one byte from a raw descriptor, which is \
                  the half of the self-pipe that is not in a signal context"
    )]
    fn asked(read: c_int) -> bool {
        let mut byte = [0_u8; 1];
        loop {
            // SAFETY: one byte, into a buffer of one this frame owns, from a
            // descriptor nothing else reads.
            let got = unsafe { libc::read(read, byte.as_mut_ptr().cast(), 1) };
            if got == 1 {
                return true;
            }
            // An interrupted read is this thread having taken a signal of its
            // own, which says nothing about the byte that is still coming.
            if got != -1
                || std::io::Error::last_os_error().kind() != std::io::ErrorKind::Interrupted
            {
                return false;
            }
        }
    }

    /// Points `signal` at the handler, keeping every other disposition as it
    /// was.
    ///
    /// `SA_RESTART`, so that a delivery does not turn every blocking call in
    /// this process into an `EINTR` its caller has to understand: what makes a
    /// parked wait notice this shutdown is the drain's own wake and never the
    /// interruption.
    #[expect(
        unsafe_code,
        reason = "`sigaction` is the platform's only way to install a handler, \
                  and the struct it takes has no safe constructor"
    )]
    fn arm(signal: c_int) -> Result<(), String> {
        // SAFETY: `sigaction` is a plain C struct whose all-zero value is the
        // documented starting point — every field this cares about is set
        // below, and `sigemptyset` initialises the mask the platform's way.
        let mut action: libc::sigaction = unsafe { std::mem::zeroed() };
        // SAFETY: the mask belongs to the local above.
        unsafe { libc::sigemptyset(&raw mut action.sa_mask) };
        // Through a pointer rather than straight to an integer: a function item
        // is a zero-sized value of its own type, and the direct cast is the one
        // `function_casts_as_integer` names — it reads as an address without
        // there being a pointer anywhere in it.
        action.sa_sigaction = delivered as *const () as usize;
        action.sa_flags = libc::SA_RESTART;
        // SAFETY: the action is a live local for the whole call, and a null
        // third argument is "do not report the old disposition".
        if unsafe { libc::sigaction(signal, &raw const action, std::ptr::null_mut()) } != 0 {
            return Err(format!(
                "could not install the handler for signal {signal}: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(())
    }

    /// The signal handler, and everything it is allowed to do.
    ///
    /// One non-blocking `write` of one byte, whose result is ignored: a full
    /// pipe is a stop already asked for, and a failed write is a process with
    /// no thread left to answer. Which signal arrived is not read, because the
    /// module docs' three mean one thing.
    #[expect(
        unsafe_code,
        reason = "a signal handler is an `extern \"C\"` the kernel calls, and \
                  the one call it makes is a raw `write`"
    )]
    extern "C" fn delivered(_signal: c_int) {
        let byte = 1_u8;
        // SAFETY: one byte from a local that outlives the call, onto a
        // descriptor this process owns for its whole life.
        let _ignored =
            unsafe { libc::write(WAKE.load(Ordering::Relaxed), (&raw const byte).cast(), 1) };
    }
}

#[cfg(windows)]
mod platform {
    use windows_sys::Win32::Foundation::{FALSE, TRUE};
    use windows_sys::Win32::System::Console::{
        CTRL_BREAK_EVENT, CTRL_C_EVENT, CTRL_CLOSE_EVENT, CTRL_LOGOFF_EVENT, CTRL_SHUTDOWN_EVENT,
        SetConsoleCtrlHandler,
    };
    use windows_sys::core::BOOL;

    /// Adds the handler below to this process's console control handlers.
    #[expect(
        unsafe_code,
        reason = "`SetConsoleCtrlHandler` is the platform's only way to be told \
                  a console asked this process to stop"
    )]
    pub(super) fn install() -> Result<(), String> {
        // SAFETY: a function pointer with the signature the platform declares,
        // added rather than removed.
        if unsafe { SetConsoleCtrlHandler(Some(delivered), TRUE) } == FALSE {
            return Err(format!(
                "could not install the console's control handler: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(())
    }

    /// Windows' half of a terminating signal, called on a thread of the
    /// platform's own making — so the drain is begun here rather than handed to
    /// a thread of ours.
    ///
    /// `TRUE` says this process handled the event, which for Ctrl-C and
    /// Ctrl-Break means it keeps running: the drain is what ends it, once the
    /// connections it is serving are done. For the three that are a console,
    /// session or machine going away, Windows gives a handled process a few
    /// seconds and then ends it regardless — the drain is what uses them.
    ///
    /// `FALSE` for anything else, which leaves an event this does not
    /// understand to the next handler and to the default.
    #[expect(
        unsafe_code,
        reason = "the platform calls this, so its signature is the platform's"
    )]
    unsafe extern "system" fn delivered(event: u32) -> BOOL {
        match event {
            CTRL_C_EVENT | CTRL_BREAK_EVENT | CTRL_CLOSE_EVENT | CTRL_LOGOFF_EVENT
            | CTRL_SHUTDOWN_EVENT => {
                super::deliver();
                TRUE
            }
            _ => FALSE,
        }
    }
}
