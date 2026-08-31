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
//! # The prompts read the terminal, never `Stream::In`
//!
//! ADR 0086 § 4's prompts are the second half of this module: [`prompt`] opens
//! the controlling terminal *by name* and reads a line from that, which is why
//! `cat data.csv | myprog` can still ask a question. [`is_interactive`] is the
//! question asked first, and its doc comment owns why it reads the profile
//! rather than trying the open.
//!
//! What is *not* here is the decision: `nvs_stdlib::cli` asks whether a person
//! is watching this program's output at all before it prompts, because a
//! question written into an HTTP response body or a `Core\Out::capture` buffer
//! reaches nobody. This module answers "is there a terminal, and what did it
//! say"; the surface answers "should this program be asking".
//!
//! Memory: one [`Profile`] — three `bool`s, two `u32`s and an enum — for the
//! life of the process, charged to no request, plus one answer's bytes for the
//! length of a [`prompt`] call, bounded by `MAX_ANSWER`.

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

// ------------------------------------------------------------------ the prompts

/// Whether what a prompt reads is echoed as it is typed.
///
/// ADR 0086 § 4's `secret` is the one member that asks for [`Self::Hidden`],
/// and the suppression is the terminal's own — the bytes are never written
/// back — rather than an overwrite after the fact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Echo {
    /// The terminal shows what is typed, as it does for every other prompt.
    Shown,
    /// The terminal shows nothing at all: a password.
    Hidden,
}

/// How many bytes of one answer are read before the rest of the line is
/// abandoned.
///
/// A prompt's answer is a person's, so this is three orders of magnitude past
/// any real one; it exists because the alternative is an unbounded allocation
/// driven by whatever is on the other end of a terminal that may not be a
/// person at all (`AGENTS.md`'s priority 5, and the cap that makes the
/// footprint attributable).
const MAX_ANSWER: usize = 4096;

/// Whether this process has a terminal to prompt at, which is not the same
/// question as whether standard input is one.
///
/// ADR 0086 § 4: a prompt reads the *controlling terminal*, so
/// `cat data.csv | myprog` can still ask. Any one of the three streams being a
/// terminal says the process was started from one; all three redirected — a CI
/// job, a `cron` entry, a `.nvst` case, which runs as a child with its output
/// piped and its input closed — says it was not, and that is the state § 4
/// answers with a default or with `Core\Cli\NotInteractive` rather than by
/// blocking on input nobody can give.
///
/// Asking the profile rather than trying the open is what makes that
/// deterministic: on Unix a fully-redirected child of a terminal session can
/// still open `/dev/tty`, and on Windows it inherits the console — so an open
/// that succeeds proves a terminal exists *somewhere*, never that anyone is
/// watching this program's output.
#[must_use]
pub fn is_interactive() -> bool {
    let profile = profile();
    profile.is_tty(Stream::Out) || profile.is_tty(Stream::Err) || profile.is_tty(Stream::In)
}

/// Writes `question` to the controlling terminal and reads one line back from
/// it, or `None` when there is no terminal or it gave nothing.
///
/// **`Stream::In` is never read here**, and that is the whole point of the
/// function: the terminal is opened by name — `/dev/tty` on Unix, `CONIN$` and
/// `CONOUT$` on Windows — so a program whose standard input is a pipe still
/// asks its question of the person who started it.
/// `nvs_stdlib::cli`'s `a_prompt_reads_the_controlling_terminal_and_not_stdin`
/// holds that shut.
///
/// The answer is returned without its line ending and never with the
/// question's own bytes; a caller that wants the question neutralized has done
/// it already (`nvs_render::text::substitute`), because this function writes
/// what it is handed.
#[must_use]
pub fn prompt(question: &str, echo: Echo) -> Option<String> {
    if !is_interactive() {
        return None;
    }
    ask(question, echo).ok().flatten()
}

/// One line off `file`, or `None` for a terminal that answered end-of-input.
///
/// A terminal in its ordinary cooked mode hands back one line per read, so the
/// loop below usually runs once; it is a loop for the terminal that splits a
/// long line across two, and it stops at [`MAX_ANSWER`] with what it has.
fn read_answer(mut file: &std::fs::File) -> std::io::Result<Option<String>> {
    use std::io::Read;

    let mut answer: Vec<u8> = Vec::new();
    let mut chunk = [0_u8; 256];
    loop {
        let read = file.read(&mut chunk)?;
        if read == 0 {
            if answer.is_empty() {
                return Ok(None);
            }
            break;
        }
        if let Some(at) = chunk[..read].iter().position(|&byte| byte == b'\n') {
            answer.extend_from_slice(&chunk[..at]);
            break;
        }
        answer.extend_from_slice(&chunk[..read]);
        if answer.len() >= MAX_ANSWER {
            answer.truncate(MAX_ANSWER);
            break;
        }
    }
    if answer.last() == Some(&b'\r') {
        answer.pop();
    }
    // Lossy rather than a failure: a terminal's bytes are whatever encoding it
    // was configured with, and ADR 0009 § 1 says a `string` is UTF-8 — so the
    // replacement character is the honest answer for a byte that is neither,
    // and refusing the whole line would lose an answer over one keystroke.
    Ok(Some(String::from_utf8_lossy(&answer).into_owned()))
}

/// Asks the controlling terminal itself, through `/dev/tty` — the device that
/// is this process's terminal whatever its standard streams were redirected
/// to.
#[cfg(unix)]
fn ask(question: &str, echo: Echo) -> std::io::Result<Option<String>> {
    use std::io::Write;

    let tty = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")?;
    (&tty).write_all(question.as_bytes())?;
    (&tty).flush()?;
    let restore = match echo {
        Echo::Shown => None,
        Echo::Hidden => hide_echo(&tty),
    };
    let answer = read_answer(&tty);
    if let Some(saved) = restore {
        show_echo(&tty, saved);
        // The terminal echoed no newline because it echoed nothing, so the
        // next line of output would otherwise start beside the question.
        let _ = (&tty).write_all(b"\n");
    }
    answer
}

/// The terminal's attributes with `ECHO` cleared, answering what they were so
/// that [`show_echo`] can put them back — `None` for a device that has no
/// attributes to change, where the answer is simply visible.
#[cfg(unix)]
fn hide_echo(tty: &std::fs::File) -> Option<libc::termios> {
    use std::os::unix::io::AsRawFd;

    let fd = tty.as_raw_fd();
    #[expect(
        unsafe_code,
        reason = "`tcgetattr`/`tcsetattr` are the only way to turn a terminal's echo off, and \
                  `std` has no equivalent. The descriptor is one this function opened and still \
                  owns, and the out-parameter is a stack `termios` this call owns exclusively, \
                  so the library writes a fixed-size struct into a live allocation that outlives \
                  the call by nothing."
    )]
    unsafe {
        let mut saved: libc::termios = std::mem::zeroed();
        if libc::tcgetattr(fd, &raw mut saved) != 0 {
            return None;
        }
        let mut hidden = saved;
        hidden.c_lflag &= !libc::ECHO;
        (libc::tcsetattr(fd, libc::TCSANOW, &raw const hidden) == 0).then_some(saved)
    }
}

/// Puts back what [`hide_echo`] answered.
#[cfg(unix)]
fn show_echo(tty: &std::fs::File, saved: libc::termios) {
    use std::os::unix::io::AsRawFd;

    #[expect(
        unsafe_code,
        reason = "the restoring half of `hide_echo`, over the same descriptor and a `termios` \
                  that call read out of it; a failure here leaves the terminal as it is, which \
                  is why the status is dropped."
    )]
    unsafe {
        libc::tcsetattr(tty.as_raw_fd(), libc::TCSANOW, &raw const saved);
    }
}

/// See the `unix` arm. Windows names the two halves of the console separately,
/// so the question and the answer are two handles rather than one: `CONOUT$`
/// is the console this process would draw on and `CONIN$` the keyboard behind
/// it, and neither is affected by a redirection of the standard streams.
#[cfg(windows)]
fn ask(question: &str, echo: Echo) -> std::io::Result<Option<String>> {
    use std::io::Write;

    let mut screen = std::fs::OpenOptions::new().write(true).open("CONOUT$")?;
    let keyboard = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("CONIN$")?;
    screen.write_all(question.as_bytes())?;
    screen.flush()?;
    let restore = match echo {
        Echo::Shown => None,
        Echo::Hidden => hide_echo(&keyboard),
    };
    let answer = read_answer(&keyboard);
    if let Some(saved) = restore {
        show_echo(&keyboard, saved);
        let _ = screen.write_all(b"\n");
    }
    answer
}

/// See the `unix` arm. The console's mode is one word, and echo is one bit of
/// it.
#[cfg(windows)]
fn hide_echo(keyboard: &std::fs::File) -> Option<u32> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::System::Console::{
        CONSOLE_MODE, ENABLE_ECHO_INPUT, GetConsoleMode, SetConsoleMode,
    };

    let handle = keyboard.as_raw_handle().cast();
    #[expect(
        unsafe_code,
        reason = "there is no `std` spelling of the console mode. The handle belongs to a file \
                  this function's caller opened and still owns, and the out-parameter is a stack \
                  `CONSOLE_MODE` this call owns exclusively; a handle that is not a console makes \
                  `GetConsoleMode` fail, which is the `== 0` test."
    )]
    unsafe {
        let mut mode: CONSOLE_MODE = 0;
        if GetConsoleMode(handle, &raw mut mode) == 0 {
            return None;
        }
        (SetConsoleMode(handle, mode & !ENABLE_ECHO_INPUT) != 0).then_some(mode)
    }
}

/// See the `unix` arm.
#[cfg(windows)]
fn show_echo(keyboard: &std::fs::File, saved: u32) {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::System::Console::SetConsoleMode;

    #[expect(
        unsafe_code,
        reason = "the restoring half of `hide_echo`, over the same handle and the mode that call \
                  read off it; a failure leaves the console as it is, which is why the status is \
                  dropped."
    )]
    unsafe {
        SetConsoleMode(keyboard.as_raw_handle().cast(), saved);
    }
}

/// Neither Unix nor Windows: there is no terminal to open, so there is no
/// prompt to answer — [`is_interactive`] has already said so, and this arm
/// exists so that the platform still builds.
#[cfg(not(any(unix, windows)))]
fn ask(_question: &str, _echo: Echo) -> std::io::Result<Option<String>> {
    Ok(None)
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
