//! The cost of the mechanism Novis's in-process script isolates replace.
//!
//! [ADR 0006](../../../docs/adr/0006-isolated-script-execution.md) argues that a
//! script needing to run *another* script under its own heap, its own limits and
//! its own globals should get an in-process isolate rather than a child process,
//! which is PHP's only answer. The argument is quantitative: an isolate reuses an
//! already-compiled unit, an arena and a coroutine, while a child process pays
//! for a `CreateProcess`/`fork`+`exec`, an image load, a language runtime boot and
//! a pipe round trip.
//!
//! This module measures the **floor** of the thing being replaced: the cheapest
//! do-nothing process the platform can start, waited to completion. It is a floor
//! in two directions, deliberately —
//!
//! * a real replacement spawns an *interpreter*, not `/bin/true`, so the honest
//!   comparison is strictly worse than what is measured here, and
//! * nothing Novis does can make this number smaller.
//!
//! So a guard test built on it can only ever understate the gap, which is the
//! safe direction for an assertion that justifies a feature. The understatement
//! is large: on `x86_64-pc-windows-msvc` this floor is 5.95 ms, while PHP 8.5.8
//! — the runtime a PHP script would actually be starting — takes 35.9 ms to boot
//! and exit.

use std::process::{Command, Stdio};

/// The cheapest process the host can start and have exit immediately.
///
/// `/bin/true` exists for exactly this purpose on Unix. Windows has no
/// equivalent binary, so the shell's built-in `exit` is the closest thing;
/// `cmd.exe` is heavier than a minimal image, which makes the Windows figure a
/// more generous floor than the Unix one rather than a less generous one.
#[must_use]
pub fn noop_command() -> Command {
    let mut cmd = if cfg!(windows) {
        let mut c = Command::new("cmd.exe");
        c.args(["/d", "/c", "exit"]);
        c
    } else {
        Command::new("/bin/true")
    };
    // No inherited handles: writing to a captured pipe would measure the pipe.
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    cmd
}

/// Starts a do-nothing process and waits for it to exit.
///
/// This is the whole round trip a script pays for today when PHP's only
/// isolation boundary — another process — is used to run a job: create, load,
/// exit, reap.
///
/// # Panics
///
/// Panics if the process cannot be started or reaped, which means the host is
/// missing `/bin/true` or `cmd.exe` and the probe cannot say anything useful.
pub fn spawn_noop() {
    let status = noop_command()
        .status()
        .expect("the host must be able to start a do-nothing process");
    debug_assert!(status.success(), "a do-nothing process must exit cleanly");
}
