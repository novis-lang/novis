//! The controlling terminal: the profile resolved once for the process — which
//! standard streams are terminals, how wide and tall one is, how much colour it
//! can show — and how many of its columns a given string will occupy.
//!
//! `rule:tooling/the-terminal-profile-resolves-once`
//! specifies the answers and `crates/nvs-stdlib/src/cli.rs` is the surface that
//! hands them to a program. What lives here is the *reaching*: an `ioctl` on
//! Unix, two console calls on Windows, and the environment variables that
//! decide colour. `rule:security/capability-check-at-the-door`
//! is why the split is at this crate's edge rather than inside `Core\Cli` —
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
//! The environment variables read below (`NO_COLOR`, `CLICOLOR_FORCE`,
//! `FORCE_COLOR`, `TERM`, `COLORTERM`) are read *here* and never handed back as
//! values. That is deliberate and is the reason `Core\Cli` needs no
//! `Core\Env`-shaped grant either: what crosses this boundary is a column count
//! and an enum, never the environment itself.
//!
//! # Once, not per call
//!
//! [`profile`] fills a [`OnceLock`] and every later call reads it. `rule:tooling/the-terminal-profile-resolves-once`
//! requires that — two reads of the width in one run are the same number *by
//! construction*, so a program that measures at the top and draws at the bottom
//! cannot tear a frame — and the trade it makes is staleness: a window the user
//! resizes is not noticed until the process restarts. The ADR's own answer to
//! "the terminal changed" is that a program writes `Cli\Text` and the sink
//! decides at write time, so there is nothing in the surface that freshness
//! here could serve.
//!
//! # The column count is here, and so are its callers
//!
//! [`display_width`] answers `rule:tooling/the-terminal-profile-resolves-once`'s `Core\Cli::displayWidth` and
//! [`clamp`] cuts a region's row to the same unit. They read one table on
//! purpose: a row cut against a different answer than the one the
//! program was handed is a frame that wraps, which is the one failure `rule:tooling/the-terminal-is-restored-on-every-exit-path`
//! 's clamp exists to prevent. The unit itself — UAX #11 columns over the
//! string *as the sink would write it* — and the code points that are
//! not a column at all are [`display_width`]'s own doc comment, which is their
//! only home.
//!
//! It sits in this module rather than beside `Core\Str`'s units because a
//! column count is a property of the renderer, not of the string
//! (`rule:types/string-is-utf8` fixed the units
//! that are properties of the string, and `rule:tooling/the-terminal-profile-resolves-once`'s last paragraph is why
//! this one is not a `Core\Str` member).
//!
//! # The prompts read the terminal, never `Stream::In`
//!
//! `rule:tooling/a-prompt-is-a-core-member`'s prompts are the second half of this module: [`prompt`] opens
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
//! # And they answer within [`ANSWER_DEADLINE`], terminal or no terminal
//!
//! `rule:tooling/a-prompt-is-a-core-member`'s "it never blocks" is two rules, and the profile above only
//! settles the first: with no terminal there is nothing to wait on. The second
//! is that a terminal *nobody is sitting at* — a CI job that allocated a pty,
//! a `docker run -t` with no keyboard behind it — must not hold the program
//! either, which is `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`'s "no spelling for an unbounded wait" reaching
//! this surface. So the read is under a clock, and there is no argument
//! anywhere in the surface that lengthens it.
//!
//! The bound is a thread and a channel rather than a timed read, because a
//! terminal in cooked mode has no portable one. `poll`/`select` on the device
//! answers *a byte is available*, which on Windows is any console input record
//! at all — a key release, a focus change — while the `ReadFile` behind a
//! cooked line read still waits for `Enter`, so a wait that returns says
//! nothing about whether the read after it will. [`ask`] therefore runs whole
//! on its own thread, owning everything it touches, and [`answer_within`]
//! waits on the channel: the same bound on every platform, with no `unsafe`
//! anywhere in it.
//!
//! # A live region ends in a `Drop`, because every other ending can be skipped
//!
//! `rule:tooling/in-place-output-is-a-scoped-live-region`'s in-place output is the module's third half: [`Region`] owns
//! the cursor between the two ends of one `Core\Cli::live` call. § 8 makes
//! putting the terminal back an obligation on **every** exit path — a throw, a
//! fatal, an internal panic (`rule:errors/panics-bypass-user-code`), a signal — and the only construct in Rust that runs on all of them is
//! a destructor. So restoration is [`Region`]'s `Drop` and lives nowhere else:
//! there is no `close()` a caller can forget, no `finally` for a Novis program
//! to write, and no second copy of the escape sequence that shows the cursor
//! again. `nvs_stdlib::cli` holds the open regions on a stack and drops back to
//! a depth, which is how the *scope* half is enforced above this line.
//!
//! Memory: one [`Profile`] — scalar fields only, no allocation — for the
//! life of the process, charged to no request, plus one answer's bytes for the
//! length of a [`prompt`] call, bounded by `MAX_ANSWER`. A prompt that reaches
//! its deadline leaves that thread parked in its read until the terminal ends
//! the line or the process exits — one stack, only on the path where nobody
//! answered, which is the price of a bound that holds on every platform. A
//! [`Region`] holds two frames — the one on screen and the one waiting for the
//! timer — each bounded by the terminal's own width times its height, for as
//! long as the `live` call it belongs to.

use std::io::IsTerminal;
use std::sync::OnceLock;

use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// The default width, for a process with no controlling terminal — `rule:tooling/the-terminal-profile-resolves-once`
/// names it.
pub const FALLBACK_WIDTH: u32 = 80;

/// The default height, for [`FALLBACK_WIDTH`]'s reason and named by the same
/// sentence.
pub const FALLBACK_HEIGHT: u32 = 24;

/// Which standard stream a question is about — `rule:tooling/the-terminal-profile-resolves-once`'s `Cli\Stream`,
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

/// How much colour standard output can show — `rule:tooling/the-terminal-profile-resolves-once`'s `Cli\ColorDepth`.
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

/// `rule:tooling/the-terminal-profile-resolves-once`'s terminal facts, resolved once — see the module docs.
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
/// The order is `rule:tooling/the-terminal-profile-resolves-once`'s: the refusals first, because `NO_COLOR` outranks
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
    if !enable_virtual_terminal(Stream::Out) {
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

/// Whether an escape sequence written to `stream` is interpreted, not shown as
/// text: the stream is a terminal, and on Windows its console accepted virtual
/// terminal processing.
///
/// For a caller that styles what it writes itself instead of through
/// [`profile`], whose depth is standard output's. `nvs` renders its diagnostics
/// to standard error, and that stream's console is one a redirected standard
/// output never reaches. The environment variables stay the caller's to honour.
/// Always `false` for [`Stream::In`], which nothing is written to.
#[must_use]
pub fn interprets_escapes(stream: Stream) -> bool {
    let is_terminal = match stream {
        Stream::In => false,
        Stream::Out => std::io::stdout().is_terminal(),
        Stream::Err => std::io::stderr().is_terminal(),
    };
    is_terminal && enable_virtual_terminal(stream)
}

/// Turns escape-sequence interpretation on for `stream`'s console, answering
/// whether colour may be written to it at all.
///
/// Always `true` off Windows, where a terminal interprets sequences without
/// being asked.
#[cfg(not(windows))]
fn enable_virtual_terminal(_stream: Stream) -> bool {
    true
}

/// See the non-Windows arm. Reached only once something has already asked for
/// colour on `stream`, so the mode is never changed for a program that would
/// not have used it.
#[cfg(windows)]
fn enable_virtual_terminal(stream: Stream) -> bool {
    use windows_sys::Win32::System::Console::{
        CONSOLE_MODE, ENABLE_VIRTUAL_TERMINAL_PROCESSING, GetConsoleMode, GetStdHandle,
        STD_ERROR_HANDLE, STD_OUTPUT_HANDLE, SetConsoleMode,
    };

    let id = match stream {
        // An input handle has no output mode to set.
        Stream::In => return false,
        Stream::Out => STD_OUTPUT_HANDLE,
        Stream::Err => STD_ERROR_HANDLE,
    };

    #[expect(
        unsafe_code,
        reason = "there is no `std` spelling of the console mode. The handle is one this \
                  process already owns and the out-parameter is a stack `CONSOLE_MODE` this \
                  call owns exclusively; a redirected handle makes `GetConsoleMode` fail, \
                  which is the `== 0` test."
    )]
    unsafe {
        let handle = GetStdHandle(id);
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
/// `rule:tooling/a-prompt-is-a-core-member`'s `secret` is the one member that asks for [`Self::Hidden`],
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
/// person at all (`rule:programs/memory-priority`, and the cap that makes the
/// footprint attributable).
const MAX_ANSWER: usize = 4096;

/// How long a prompt waits for an answer before it gives up.
///
/// `rule:tooling/a-prompt-is-a-core-member` says a prompt never blocks, and
/// `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect` says
/// no wait may be spelled unbounded. Five minutes is what those two come to
/// here: two orders of magnitude past the seconds a person spends answering a
/// one-line question, and still short enough that a CI job holding a pty
/// nobody is watching fails while its own log is being read rather than at
/// whatever kill the runner eventually applies.
///
/// There is deliberately **no argument that lengthens it**. A `timeout` option
/// on `ask` would be a spelling for "wait longer", and the value of the rule is
/// that no program has one — the same reason `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect` gives for the outbound
/// deadline it mirrors.
pub const ANSWER_DEADLINE: std::time::Duration = std::time::Duration::from_secs(300);

/// What one prompt got back.
///
/// Silence has its own case apart from *nobody to ask*, because the surface
/// says a different sentence for it: [`Self::Ended`] is what `rule:tooling/a-prompt-is-a-core-member` answers
/// with the `default` or with `Core\Cli\NotInteractive`, while
/// [`Self::TimedOut`] is a terminal that was opened, written to and then said
/// nothing for [`ANSWER_DEADLINE`]. Both are silence and both take the same
/// fallback; only the message a program is handed distinguishes them, and a
/// message naming a terminal that does not exist would be a lie about a
/// terminal that answered nothing.
#[derive(Debug)]
pub enum Answer {
    /// The line typed, without its ending.
    Line(String),
    /// There is no terminal, its input ended, or it could not be opened.
    Ended,
    /// A terminal that answered nothing within [`ANSWER_DEADLINE`].
    TimedOut,
}

/// Whether this process has a terminal to prompt at, which is not the same
/// question as whether standard input is one.
///
/// `rule:tooling/a-prompt-is-a-core-member`: a prompt reads the *controlling terminal*, so
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
///
/// The whole of [`ask`] runs on its own thread and this one waits on a channel
/// for [`ANSWER_DEADLINE`] — the module doc's *And they answer within
/// `ANSWER_DEADLINE`* owns why the bound is shaped that way rather than as a
/// timed read. A thread that cannot be spawned is [`Answer::Ended`]: the
/// program is out of a resource this module will not block for.
#[must_use]
pub fn prompt(question: &str, echo: Echo) -> Answer {
    if !is_interactive() {
        return Answer::Ended;
    }
    let question = question.to_owned();
    let (answers, from_terminal) = std::sync::mpsc::channel();
    let asking = std::thread::Builder::new()
        .name("nvs-prompt".to_owned())
        .spawn(move || {
            let answer = match ask(&question, echo) {
                Ok(Some(line)) => Answer::Line(line),
                Ok(None) | Err(_) => Answer::Ended,
            };
            // The receiver is gone whenever the deadline came first, and
            // nothing here can act on that: the question has been asked and
            // this thread's only remaining job is to let the terminal finish
            // its line and put the echo back.
            let _ = answers.send(answer);
        });
    if asking.is_err() {
        return Answer::Ended;
    }
    answer_within(&from_terminal, ANSWER_DEADLINE)
}

/// The bounded wait itself: whatever the asking thread sent, or
/// [`Answer::TimedOut`] once `within` has elapsed.
///
/// Split out of [`prompt`] because it is the half that can be *tested* — a
/// channel nobody sends on is an unattended terminal, with no terminal and no
/// person required, and `nvs_stdlib::cli`'s `no_prompt_blocks_without_a_deadline`
/// drives it that way. A sender dropped without a send is [`Answer::Ended`]
/// rather than a timeout, because the asking thread died rather than the clock.
#[must_use]
pub fn answer_within(
    from_terminal: &std::sync::mpsc::Receiver<Answer>,
    within: std::time::Duration,
) -> Answer {
    match from_terminal.recv_timeout(within) {
        Ok(answer) => answer,
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Answer::TimedOut,
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => Answer::Ended,
    }
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
    // was configured with, and `rule:types/bytes` says a `string` is UTF-8 — so the
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

// -------------------------------------------------------------- the live region

/// How long a [`Region`] holds a frame before it paints — `rule:tooling/in-place-output-is-a-scoped-live-region`'s
/// "coalesces frames on a timer rather than repainting per `set`".
///
/// A loop that calls `set` once per file processed calls it thousands of times
/// a second, and a terminal repainted that often is both slower than the work
/// and unreadable. Twenty frames a second is past what a reader resolves and
/// well under what a terminal costs to repaint.
pub const FRAME_INTERVAL: std::time::Duration = std::time::Duration::from_millis(50);

/// Hide the cursor, and show it again. The pair § 8 says a region must leave
/// balanced, written once so that no path can hold one without the other.
const HIDE_CURSOR: &str = "\x1b[?25l";

/// See [`HIDE_CURSOR`].
const SHOW_CURSOR: &str = "\x1b[?25h";

/// The terminal rows one `Core\Cli::live` call owns — `rule:tooling/in-place-output-is-a-scoped-live-region`'s live
/// region, and § 8's restoration obligation as a destructor.
///
/// Most of the section's properties are here: the cursor is hidden while
/// the region is open and shown again when it closes, a frame is coalesced onto
/// [`FRAME_INTERVAL`] rather than painted per [`set`](Self::set), and a paint
/// diffs against the frame already on screen so an unchanged row costs a line
/// feed rather than a repaint. So is the one that says a region with no terminal
/// **renders nothing at all**: [`open`](Self::open) answers an inert region
/// where [`is_interactive`] is false, so a piped run produces clean output
/// rather than a smear of escape sequences, and every method below is then a
/// no-op rather than a write nobody reads.
///
/// Repainting on a resize is not here yet, and the reason is § 3:
/// the profile is resolved once for the process, so the width a region clamps
/// to is fixed for its life and a window the reader resizes is not noticed. The
/// signal that would say so is `Core\Signal`'s, which is not built.
///
/// **What is left on screen is the last frame.** A region that erased itself
/// would take the program's own output with it, which is the opposite of what
/// the closing `set` was for; what it restores is the *cursor*, and the row
/// below the region is where the next `echo` lands.
pub struct Region {
    /// Where a frame is painted, or `None` for a region that renders nothing —
    /// which is both the no-terminal case and every path after
    /// [`close`](Self::close).
    screen: Option<Box<dyn std::io::Write>>,
    /// The frame on screen now, clamped, one entry per row.
    painted: Vec<String>,
    /// The frame [`set`](Self::set) was last handed and the timer has not yet
    /// let through. Painted by [`close`](Self::close) whatever the timer says,
    /// because a coalesced final frame that never landed is the one bug this
    /// shape could have.
    pending: Option<Vec<String>>,
    /// When the next paint is allowed. Starts in the past, so the first `set`
    /// lands at once — a program that draws once and then works for a minute
    /// must still be on screen for it.
    due: std::time::Instant,
    /// The column count rows are clamped to. A wrapped row would make the line
    /// arithmetic below wrong and leave the region unrecoverable, which is why
    /// this is a clamp and not a courtesy.
    width: usize,
}

impl Region {
    /// A region on the controlling terminal, or an inert one where there is no
    /// terminal to own.
    ///
    /// The device is opened **by name**, exactly as [`prompt`] opens it and for
    /// the same reason: `myprog > log` still has a person in front of it, and
    /// the escape sequences belong on their screen rather than in their file.
    #[must_use]
    pub fn open() -> Self {
        Self::over(if is_interactive() { screen() } else { None })
    }

    /// A region painting on `screen` — what a test hands one, since the
    /// alternative is a test that needs a terminal and a person.
    #[must_use]
    pub fn painting_on(screen: Box<dyn std::io::Write>) -> Self {
        Self::over(Some(screen))
    }

    /// Both constructors' body: the cursor is hidden on the way in, and from
    /// here on every path out runs [`close`](Self::close).
    fn over(screen: Option<Box<dyn std::io::Write>>) -> Self {
        let mut region = Self {
            screen,
            painted: Vec::new(),
            pending: None,
            due: std::time::Instant::now(),
            width: profile().width() as usize,
        };
        region.write(HIDE_CURSOR);
        region
    }

    /// Hands the region its next frame, painting it if the timer allows.
    ///
    /// Nothing is written for an inert region, and the frame is dropped rather
    /// than held: § 5's "renders nothing at all" is a property of this method
    /// and not only of what reaches the screen.
    pub fn set(&mut self, lines: Vec<String>) {
        if self.screen.is_none() {
            return;
        }
        self.pending = Some(lines);
        if std::time::Instant::now() >= self.due {
            self.paint();
        }
    }

    /// Paints whatever is pending, diffing against what is on screen.
    ///
    /// The cursor sits on the row below the region between paints, so the walk
    /// is: up by however many rows are painted, then one line feed per row —
    /// preceded by an erase and the row's own bytes only where the row changed.
    /// A frame with fewer rows than the last erases the extras and comes back
    /// up to sit below the new region.
    fn paint(&mut self) {
        let Some(frame) = self.pending.take() else {
            return;
        };
        let frame: Vec<String> = frame.iter().map(|row| clamp(row, self.width)).collect();
        let mut out = String::new();
        if !self.painted.is_empty() {
            out.push_str(&format!("\x1b[{}A", self.painted.len()));
        }
        for at in 0..frame.len().max(self.painted.len()) {
            match frame.get(at) {
                Some(row) if self.painted.get(at) != Some(row) => {
                    out.push_str("\r\x1b[2K");
                    out.push_str(row);
                }
                Some(_) => {}
                None => out.push_str("\r\x1b[2K"),
            }
            out.push('\n');
        }
        let erased = self.painted.len().saturating_sub(frame.len());
        if erased > 0 {
            out.push_str(&format!("\x1b[{erased}A"));
        }
        self.write(&out);
        self.painted = frame;
    }

    /// Puts the terminal back: the frame the timer was still holding lands, the
    /// cursor comes back, and the region renders nothing after this.
    ///
    /// Idempotent, because [`Drop`] calls it and so may anything else.
    fn close(&mut self) {
        self.paint();
        self.write(SHOW_CURSOR);
        self.screen = None;
    }

    /// One write and one flush, or nothing at all for an inert region.
    ///
    /// Every failure is dropped: a region is decoration over a device the
    /// program does not own, and a `Core\Cli::live` body that failed because
    /// the terminal went away mid-frame would turn a cosmetic problem into the
    /// program's outcome.
    fn write(&mut self, bytes: &str) {
        if let Some(screen) = self.screen.as_mut() {
            let _ = screen.write_all(bytes.as_bytes());
            let _ = screen.flush();
        }
    }
}

impl Drop for Region {
    /// `rule:tooling/the-terminal-is-restored-on-every-exit-path`'s restoration, and its only home — see the module docs.
    fn drop(&mut self) {
        self.close();
    }
}

impl std::fmt::Debug for Region {
    /// By hand because the screen is a `dyn Write` and has none. What it prints
    /// is what a reader of a region would ask: whether it renders at all, and
    /// how much it is holding.
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.debug_struct("Region")
            .field("renders", &self.screen.is_some())
            .field("painted", &self.painted.len())
            .field("pending", &self.pending.is_some())
            .finish()
    }
}

/// How far a `TAB` moves the cursor: on to the next multiple of eight columns.
///
/// Eight is not a preference. It is where every terminal Novis can write to has
/// its stops, because the only way to move one is an escape sequence, and
/// [`display_width`] measures a string `rule:tooling/terminal-output-is-a-sink` has already replaced every
/// escape in with a picture — so a program cannot have moved the stops in the
/// text this counts, and a program that moved them by writing a `Cli\Text` it
/// built from `styled` moved them for a run in which nothing else it wrote is
/// measurable either.
const TAB_STOP: usize = 8;

/// `text`'s width in terminal columns — `rule:tooling/the-terminal-profile-resolves-once`'s `Core\Cli::displayWidth`,
/// whose paragraph beside that table states the rule and is its only home:
/// UAX #11 widths, over grapheme clusters, measured on the string § 1's
/// substitution will actually put on the screen, with `TAB` reaching the next
/// [`TAB_STOP`] and `LF` ending a row rather than filling one.
///
/// What is decided *here* is the order those compose in, which is the one way
/// this can be built wrong: substitute, then split rows, then walk clusters.
/// Substituting first is what makes a control byte cost its Control Picture
/// rather than nothing, and walking clusters rather than code points is what
/// keeps a combining mark and an emoji ZWJ sequence attached to the glyph they
/// are drawn as part of.
///
/// A `Cli\Text` is the one input this must not be handed: it carries the SGR
/// its `Cli\Style` put there, and this function would measure that as the
/// pictures a substitution would make of it. [`clamp`] is the caller that has
/// one, and it does its own escape scan for exactly that reason.
#[must_use]
pub fn display_width(text: &str) -> usize {
    let shown = nvs_render::text::substitute(text);
    shown.split('\n').map(row_width).max().unwrap_or(0)
}

/// The column one row — no `LF` in it — leaves the cursor at, from zero.
fn row_width(row: &str) -> usize {
    row.graphemes(true)
        .fold(0, |column, cluster| advance(cluster, column))
}

/// The column `cluster` leaves the cursor at, having entered it at `at`.
///
/// The one place the tab stop is applied, so [`display_width`] and [`clamp`]
/// cannot come to disagree about where a tabbed row ends.
fn advance(cluster: &str, at: usize) -> usize {
    if cluster == "\t" {
        return at + TAB_STOP - at % TAB_STOP;
    }
    at + cluster.width()
}

/// `row`, cut to `width` terminal columns.
///
/// Escape sequences are copied through and cost no width, because a styled
/// `Cli\Text` carries the SGR its `Cli\Style` put there and those bytes occupy
/// no column. A row that was cut while styled is closed with a reset, so the
/// colour cannot leak onto the rest of the screen.
///
/// The count is [`display_width`]'s, one grapheme cluster at a time, and a
/// cluster that would cross the edge is left off entirely rather than half
/// written — so a row of wide glyphs stops one column short of the edge rather
/// than one past it, which is the side of the rounding that cannot wrap.
///
/// A row holds no newline of its own: `rule:tooling/terminal-output-is-a-sink`'s substitution has already
/// replaced every control byte a `Cli\Text` was built from with a visible
/// glyph, so one `Cli\Text` is one terminal row by construction.
fn clamp(row: &str, width: usize) -> String {
    let mut out = String::with_capacity(row.len());
    let mut column = 0usize;
    let mut styled = false;
    let mut clusters = row.graphemes(true);
    while let Some(cluster) = clusters.next() {
        if cluster == "\x1b" {
            styled = true;
            out.push_str(cluster);
            if let Some(next) = clusters.next() {
                out.push_str(next);
                if next == "[" {
                    for part in clusters.by_ref() {
                        out.push_str(part);
                        if matches!(part.chars().next(), Some('\x40'..='\x7e')) {
                            break;
                        }
                    }
                }
            }
            continue;
        }
        let reaches = advance(cluster, column);
        if reaches > width {
            if styled {
                out.push_str("\x1b[0m");
            }
            return out;
        }
        out.push_str(cluster);
        column = reaches;
    }
    out
}

/// The controlling terminal's screen, opened by name — `/dev/tty` on Unix.
#[cfg(unix)]
fn screen() -> Option<Box<dyn std::io::Write>> {
    std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/tty")
        .ok()
        .map(|tty| Box::new(tty) as Box<dyn std::io::Write>)
}

/// See the `unix` arm. `CONOUT$` is the console this process would draw on,
/// whatever its standard streams were redirected to.
#[cfg(windows)]
fn screen() -> Option<Box<dyn std::io::Write>> {
    std::fs::OpenOptions::new()
        .write(true)
        .open("CONOUT$")
        .ok()
        .map(|console| Box::new(console) as Box<dyn std::io::Write>)
}

/// Neither Unix nor Windows: there is no terminal to draw on, so every region
/// is inert — the same answer [`ask`]'s own fallback arm gives.
#[cfg(not(any(unix, windows)))]
fn screen() -> Option<Box<dyn std::io::Write>> {
    None
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

    /// A stream that is not a terminal never gets an escape sequence, whatever
    /// its console would accept: the terminal test comes first, so a redirected
    /// stream's console mode is never touched. Standard input is the one stream
    /// whose answer does not depend on how the test run was started.
    #[test]
    fn a_stream_nothing_is_written_to_interprets_no_escapes() {
        assert!(!interprets_escapes(Stream::In));
        if !std::io::stderr().is_terminal() {
            assert!(!interprets_escapes(Stream::Err));
        }
    }

    /// UAX #11's two-column classes, against the count of the thing that is not
    /// the column count: `"日本語"` is three grapheme clusters and six columns,
    /// and a fullwidth Latin letter is the same trap in the alphabet a caller
    /// is least expecting it in.
    #[test]
    fn a_wide_glyph_is_two_columns() {
        assert_eq!(display_width("日本語"), 6);
        assert_eq!("日本語".chars().count(), 3);
        assert_eq!(display_width("Ｈｉ"), 4);
    }

    /// The other half of counting clusters: a mark occupies no column of its
    /// own, and the sequence is one column however many code points it took.
    #[test]
    fn a_combining_mark_is_no_columns() {
        assert_eq!(display_width("e\u{301}"), 1);
        assert_eq!("e\u{301}".chars().count(), 2);
    }

    /// A control byte reaches the screen as a picture, so it is one column and
    /// not zero — and an SGR sequence cannot make a string measure short,
    /// because § 1 replaces it before the count sees it.
    #[test]
    fn a_control_byte_costs_what_its_picture_costs() {
        assert_eq!(display_width("a\u{1b}b"), 3);
        assert_eq!(display_width("\u{7f}"), 1);
        assert_eq!(display_width("\u{9b}"), 1);
        assert_eq!(display_width("\u{1b}[31mred\u{1b}[0m"), 12);
    }

    /// The rows § 1 passes through, each asserted where a fixed count for
    /// it would answer plausibly: the tab lands on the stop wherever it stands,
    /// and the newline ends a row rather than filling one.
    #[test]
    fn a_tab_reaches_the_stop_and_a_newline_ends_the_row() {
        assert_eq!(display_width("a\tb"), 9);
        assert_eq!(display_width("\tb"), 9);
        assert_eq!(display_width("abcdefgh\ti"), 17);
        assert_eq!(display_width("ab\n日本語"), 6);
        assert_eq!(display_width(""), 0);
    }

    /// The rounding [`clamp`]'s doc comment names, asserted on both sides: the
    /// glyph that fits is kept, the one that would cross the edge is left off
    /// whole, and the row that comes back is never wider than it was asked for.
    #[test]
    fn a_clamped_row_stops_short_of_the_edge_rather_than_past_it() {
        assert_eq!(clamp("日本語", 4), "日本");
        assert_eq!(clamp("日本語", 3), "日");
        assert!(display_width(&clamp("日本語", 3)) <= 3);
        assert_eq!(clamp("abc", 3), "abc");
    }

    /// A cut row closes the style it was cut inside, and the SGR it copied
    /// through cost it no columns — the whole reason `clamp` scans escapes
    /// itself rather than calling [`display_width`].
    #[test]
    fn a_row_cut_while_styled_is_closed() {
        assert_eq!(clamp("\u{1b}[31mabcd", 2), "\u{1b}[31mab\u{1b}[0m");
        assert_eq!(clamp("\u{1b}[31mab\u{1b}[0m", 2), "\u{1b}[31mab\u{1b}[0m");
    }
}
