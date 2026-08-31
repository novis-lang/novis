//! The controlling terminal's profile — which standard streams are terminals,
//! how wide and tall one is, and how much colour it can show — resolved once
//! for the process.
//!
//! [ADR 0086](../../../../docs/adr/0086-core-cli-terminal-is-a-sink.md) § 3
//! specifies the answers and `crates/nvs-stdlib/src/cli.rs` is the surface that
//! hands them to a program. What lives here is the *reaching*: an `ioctl` on
//! Unix, two console calls on Windows, and the environment variables that
//! decide colour. [ADR 0118](../../../../docs/adr/0118-capabilities-are-configured-not-requested.md)
//! § 2 is why the split is at this crate's edge rather than inside `Core\Cli` —
//! a `Core` member may not reach the operating system directly, and
//! `crates/nvs-stdlib/tests/capability.rs` holds that shut by name.
//!
//! # No capability gates it, and that is a decision
//!
//! Every other door in this crate is in [`crate::capability`] and asks a
//! `Cap` first, because each reaches something the program was not already
//! holding — a path, a program to start. Nothing here does. All three standard
//! streams are open before the process runs a line, and `echo` already writes
//! to one of them under no capability at all, so reporting how wide that stream
//! is grants no authority a program did not arrive with. It is a module beside
//! `capability` rather than a member of it for exactly that reason: a door that
//! asks nothing is not a door, and filing it as one would make the roster of
//! real doors harder to read.
//!
//! The four environment variables read below (`NO_COLOR`, `CLICOLOR_FORCE`,
//! `FORCE_COLOR`, `TERM`, `COLORTERM`) are read *here* and never handed back as
//! values. That is deliberate and is the reason `Core\Cli` needs no
//! `Core\Env`-shaped grant either: what crosses this boundary is a column count
//! and an enum, never the environment itself.
//!
//! # Once, not per call
//!
//! [`profile`] fills a [`OnceLock`] and every later call reads it. ADR 0086 § 3
//! requires that — two reads of the width in one run are the same number *by
//! construction*, so a program that measures at the top and draws at the bottom
//! cannot tear a frame — and the trade it makes is staleness: a window the user
//! resizes is not noticed until the process restarts. The ADR's own answer to
//! "the terminal changed" is that a program writes `Cli\Text` and the sink
//! decides at write time, so there is nothing in the surface that freshness
//! here could serve.
//!
//! Memory: one [`Profile`] — three `bool`s, two `u32`s and an enum — for the
//! life of the process, charged to no request.

use std::io::IsTerminal;
use std::sync::OnceLock;

/// The default width, for a process with no controlling terminal — ADR 0086
/// § 3 names it.
pub const FALLBACK_WIDTH: u32 = 80;

/// The default height, for [`FALLBACK_WIDTH`]'s reason and named by the same
/// sentence.
pub const FALLBACK_HEIGHT: u32 = 24;

/// Which standard stream a question is about — ADR 0086 § 3's `Cli\Stream`,
/// as the Rust half of it.
///
/// A stream rather than one process-wide answer because "colour on stdout while
/// stdin is a pipe" is the common case, and a single `is_tty()` cannot say it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stream {
    /// Standard input.
    In,
    /// Standard output.
    Out,
    /// Standard error.
    Err,
}

/// How much colour standard output can show — ADR 0086 § 3's `Cli\ColorDepth`.
///
/// Ordered least to most, so the sink's `truecolor → 256 → 16 → none`
/// degradation is a comparison rather than a table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ColorDepth {
    /// No colour at all — a pipe, a file, `NO_COLOR`, or `TERM=dumb`.
    None,
    /// The eight ANSI colours and their bright halves.
    Ansi16,
    /// The 256-entry indexed palette.
    Ansi256,
    /// 24-bit colour.
    TrueColor,
}

/// ADR 0086 § 3's terminal facts, resolved once — see the module docs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Profile {
    /// Whether each stream is a terminal, indexed as [`Stream`] is ordered.
    tty: [bool; 3],
    /// The terminal's width in columns, never `0`.
    width: u32,
    /// The terminal's height in rows, never `0`.
    height: u32,
    /// The colour depth standard output can show.
    color: ColorDepth,
}

impl Profile {
    /// Whether `stream` is attached to a terminal.
    #[must_use]
    pub fn is_tty(&self, stream: Stream) -> bool {
        self.tty[stream as usize]
    }

    /// The terminal's width in columns, or [`FALLBACK_WIDTH`] when there is
    /// none. Never `0`, so a caller may subtract a margin without checking.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// The terminal's height in rows, or [`FALLBACK_HEIGHT`] when there is
    /// none. Never `0`, for [`Self::width`]'s reason.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// How much colour standard output can show.
    #[must_use]
    pub fn color_depth(&self) -> ColorDepth {
        self.color
    }
}

/// The one profile, filled by whichever caller asks first.
static PROFILE: OnceLock<Profile> = OnceLock::new();

/// The process's terminal profile, resolving it on the first call.
///
/// A process-wide `OnceLock` rather than a per-core cache: the answer is a
/// property of the process's own standard streams, which every core shares, and
/// two cores that resolved separately could disagree for the same reason two
/// calls on one core could.
#[must_use]
pub fn profile() -> &'static Profile {
    PROFILE.get_or_init(resolve)
}

/// Reads the profile out of the operating system. Called at most once per
/// process through [`profile`]; `pub` so that a test can compute a second one
/// and assert it is *discarded*.
#[must_use]
pub fn resolve() -> Profile {
    let tty = [
        std::io::stdin().is_terminal(),
        std::io::stdout().is_terminal(),
        std::io::stderr().is_terminal(),
    ];
    let (width, height) = window_size().unwrap_or((FALLBACK_WIDTH, FALLBACK_HEIGHT));
    Profile {
        tty,
        // A terminal reporting a zero dimension is reporting that it does not
        // know, and every caller of `width` is about to subtract from it.
        width: if width == 0 { FALLBACK_WIDTH } else { width },
        height: if height == 0 { FALLBACK_HEIGHT } else { height },
        color: color_depth(tty[Stream::Out as usize]),
    }
}

/// Whether an environment variable is set to anything but the empty string —
/// the convention `NO_COLOR` states and the other three follow.
fn flag(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|value| !value.is_empty())
}

/// One environment variable as text, or the empty string for one that is unset
/// or is not UTF-8 — neither of which any of the values below can be.
fn setting(name: &str) -> String {
    std::env::var_os(name)
        .and_then(|value| value.into_string().ok())
        .unwrap_or_default()
}

/// The colour depth for standard output.
///
/// The order is ADR 0086 § 3's: the refusals first, because `NO_COLOR` outranks
/// a terminal that could show colour; then the forcing variables, which are how
/// a CI runner that pipes its output still gets colour; then what the terminal
/// itself claims.
fn color_depth(out_is_tty: bool) -> ColorDepth {
    if flag("NO_COLOR") {
        return ColorDepth::None;
    }
    let term = setting("TERM");
    if term == "dumb" {
        return ColorDepth::None;
    }
    if !out_is_tty && !flag("CLICOLOR_FORCE") && !flag("FORCE_COLOR") {
        return ColorDepth::None;
    }
    // A Windows console has to be told it may interpret escape sequences at
    // all, and one that refuses shows them as text — which is worse than plain
    // output, so a refusal is `None` rather than `Ansi16`.
    if !enable_virtual_terminal() {
        return ColorDepth::None;
    }
    let colorterm = setting("COLORTERM");
    if colorterm == "truecolor" || colorterm == "24bit" {
        return ColorDepth::TrueColor;
    }
    if term.contains("256color") {
        return ColorDepth::Ansi256;
    }
    if term.is_empty() {
        // A terminal with no `TERM` at all: a Windows console that accepted
        // virtual terminal processing above, which every supported version
        // renders in 24-bit.
        return if cfg!(windows) {
            ColorDepth::TrueColor
        } else {
            ColorDepth::Ansi16
        };
    }
    ColorDepth::Ansi16
}

/// The controlling terminal's `(columns, rows)`, or `None` when there is no
/// terminal to ask.
///
/// Standard output first, then standard error, then standard input: a program
/// whose output is piped into `less` is still drawing for the terminal its
/// diagnostics go to, and asking only one stream would answer the fallback for
/// it.
#[cfg(unix)]
fn window_size() -> Option<(u32, u32)> {
    for fd in [libc::STDOUT_FILENO, libc::STDERR_FILENO, libc::STDIN_FILENO] {
        // Written out field by field rather than zeroed, so the `unsafe` below
        // covers the `ioctl` call and nothing else.
        let mut size = libc::winsize {
            ws_row: 0,
            ws_col: 0,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        #[expect(
            unsafe_code,
            reason = "`ioctl` is the only way to ask a terminal for its size; `std` has no \
                      equivalent. The descriptor is one of the three the process began with, \
                      and the out-parameter is a stack `winsize` this call owns exclusively, \
                      so the kernel writes a fixed-size struct into a live, correctly-sized \
                      allocation and nothing here outlives the call."
        )]
        let answered = unsafe { libc::ioctl(fd, libc::TIOCGWINSZ, &raw mut size) } == 0;
        if answered && size.ws_col > 0 {
            return Some((u32::from(size.ws_col), u32::from(size.ws_row)));
        }
    }
    None
}

/// See the `unix` arm. `GetConsoleScreenBufferInfo` reports the *window* as
/// well as the buffer, and the window is the visible rectangle a program draws
/// into; the buffer behind it is usually far taller.
#[cfg(windows)]
fn window_size() -> Option<(u32, u32)> {
    use windows_sys::Win32::System::Console::{
        CONSOLE_SCREEN_BUFFER_INFO, GetConsoleScreenBufferInfo, GetStdHandle, STD_ERROR_HANDLE,
        STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
    };

    for which in [STD_OUTPUT_HANDLE, STD_ERROR_HANDLE, STD_INPUT_HANDLE] {
        #[expect(
            unsafe_code,
            reason = "the two console calls are the only way to ask a Windows terminal for its \
                      size. `GetStdHandle` answers a handle this process already owns, and the \
                      out-parameter is a stack `CONSOLE_SCREEN_BUFFER_INFO` this call owns \
                      exclusively; a handle that is not a console makes the second call fail, \
                      which is the `!= 0` test rather than undefined behaviour."
        )]
        let info = unsafe {
            let handle = GetStdHandle(which);
            let mut info: CONSOLE_SCREEN_BUFFER_INFO = std::mem::zeroed();
            (GetConsoleScreenBufferInfo(handle, &raw mut info) != 0).then_some(info)
        };
        let Some(info) = info else { continue };
        let columns = i32::from(info.srWindow.Right) - i32::from(info.srWindow.Left) + 1;
        let rows = i32::from(info.srWindow.Bottom) - i32::from(info.srWindow.Top) + 1;
        if let (Ok(columns @ 1..), Ok(rows)) = (u32::try_from(columns), u32::try_from(rows)) {
            return Some((columns, rows));
        }
    }
    None
}

/// Neither Unix nor Windows: there is no terminal to measure, so the fallback
/// is the whole answer.
#[cfg(not(any(unix, windows)))]
fn window_size() -> Option<(u32, u32)> {
    None
}

/// Turns escape-sequence interpretation on for the standard output console,
/// answering whether colour may be written at all.
///
/// Always `true` off Windows, where a terminal interprets sequences without
/// being asked.
#[cfg(not(windows))]
fn enable_virtual_terminal() -> bool {
    true
}

/// See the non-Windows arm. Reached from [`color_depth`] only once something
/// has already asked for colour, so the mode is never changed for a program
/// that would not have used it.
#[cfg(windows)]
fn enable_virtual_terminal() -> bool {
    use windows_sys::Win32::System::Console::{
        CONSOLE_MODE, ENABLE_VIRTUAL_TERMINAL_PROCESSING, GetConsoleMode, GetStdHandle,
        STD_OUTPUT_HANDLE, SetConsoleMode,
    };

    #[expect(
        unsafe_code,
        reason = "there is no `std` spelling of the console mode. The handle is one this \
                  process already owns and the out-parameter is a stack `CONSOLE_MODE` this \
                  call owns exclusively; a redirected handle makes `GetConsoleMode` fail, \
                  which is the `== 0` test."
    )]
    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        let mut mode: CONSOLE_MODE = 0;
        if GetConsoleMode(handle, &raw mut mode) == 0 {
            return false;
        }
        if mode & ENABLE_VIRTUAL_TERMINAL_PROCESSING != 0 {
            return true;
        }
        SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Neither dimension is ever `0`: `width` is documented as safe to subtract
    /// a margin from, and a terminal reporting a zero column count is reporting
    /// that it does not know rather than that it is zero wide.
    #[test]
    fn a_terminal_dimension_is_never_zero() {
        assert!(profile().width() >= 1);
        assert!(profile().height() >= 1);
    }

    /// The depths ascend, which is what lets a sink degrade by comparison
    /// rather than by a table.
    #[test]
    fn the_colour_depths_ascend() {
        assert!(ColorDepth::None < ColorDepth::Ansi16);
        assert!(ColorDepth::Ansi16 < ColorDepth::Ansi256);
        assert!(ColorDepth::Ansi256 < ColorDepth::TrueColor);
    }
}
