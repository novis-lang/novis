//! What an `nvs` process was seen to use, written down for observed selection.
//!
//! `bun nv` runs a check again only when something it used has changed. A check's Rust functions
//! come from coverage; what coverage cannot show is recorded here: the `Core` classes a program
//! resolved through the registry, which it reaches by name rather than by calling code, and the
//! files, directories and paths it read. With `NVS_FOOTPRINT_LOG=<file>` in the environment, each
//! of those is appended to that file as one line, `<kind>` and a tab and the value:
//!
//! | line | written when |
//! |---|---|
//! | `class\tCore\Math` | the compiler or the runtime looked `Core\Math` up in the registry, found or not |
//! | `class\t*` | a reader listed every class, so any change to the registry changes what it printed |
//! | `card\tCore\Math` | `Core\Math`'s reference card or intro was printed or served |
//! | `card\t*` | every class's card was printed or served, as `nvs meta` does |
//! | `file\t<path>` | a file was read whole |
//! | `dir\t<path>` | a directory was listed |
//! | `exists\t<path>` | a path was tested for existence, whether or not something was there |
//! | `tree\t<path>` | a test asked for a file or a directory through `nvs_repo`: everything beneath it |
//! | `named\t<name>` | a test read every file called `<name>` anywhere in the tree, through `nvs_repo` |
//!
//! A card is a line of its own because a card is documentation: an edit to one changes what the
//! readers of cards print, and no program's behaviour.
//!
//! A path is absolute, `/`-separated on every platform and without Windows' `\\?\` prefix, and it
//! is not canonicalized: a link reads as the path that was named. Each distinct line is written
//! once per process. Several processes may append to one file, as an `nvs test` run and the case
//! processes it starts do, because each line goes out as one write to a file opened for appending.
//!
//! Without the variable nothing is written, and each call costs one atomic load. A log that is
//! named and cannot be opened or written stops the process: a footprint with a line missing is one
//! that a later change is not selected by, which is the one failure this crate exists to prevent.
//!
//! [`capture`] collects the lines one thread writes while a closure runs, with or without the
//! variable, which is how a crate's own unit tests check the lines its readers write.

use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock, PoisonError};

/// The environment variable naming the file every line is appended to.
pub const LOG_ENV: &str = "NVS_FOOTPRINT_LOG";

/// What one line records.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// A `Core` class looked up by name, or `*` for the whole roster.
    Class,
    /// A `Core` class's card printed or served, or `*` for every card.
    Card,
    /// A file read whole.
    File,
    /// A directory listed.
    Dir,
    /// A path tested for existence.
    Exists,
    /// A file or a directory a test asked for, standing for everything beneath it.
    Tree,
    /// Every file with this name, anywhere in the tree.
    Named,
}

impl Kind {
    /// The word a line starts with.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Class => "class",
            Self::Card => "card",
            Self::File => "file",
            Self::Dir => "dir",
            Self::Exists => "exists",
            Self::Tree => "tree",
            Self::Named => "named",
        }
    }
}

/// An open log: the file, and the lines this process has written to it already.
#[derive(Debug)]
pub struct Log {
    file: File,
    path: PathBuf,
    seen: HashSet<(Kind, String)>,
}

impl Log {
    /// Opens `path` for appending, creating it when it is not there.
    ///
    /// # Errors
    ///
    /// Whatever opening the file reports.
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self {
            file,
            path: path.to_path_buf(),
            seen: HashSet::new(),
        })
    }

    /// Appends the line for `kind` and `value`, unless this log has written it already.
    ///
    /// # Errors
    ///
    /// Whatever writing to the file reports.
    pub fn record(&mut self, kind: Kind, value: &str) -> io::Result<()> {
        if self.seen.contains(&(kind, value.to_string())) {
            return Ok(());
        }
        self.file.write_all(line(kind, value).as_bytes())?;
        self.seen.insert((kind, value.to_string()));
        Ok(())
    }
}

/// One line as it is written: the kind's word, a tab, the value and a newline. A tab or a newline
/// inside the value is written as a space, so a line always has exactly two fields.
#[must_use]
pub fn line(kind: Kind, value: &str) -> String {
    let value = value.replace(['\t', '\n', '\r'], " ");
    format!("{}\t{value}\n", kind.word())
}

/// The log `variable` names, or `None` when it names none.
///
/// # Errors
///
/// Whatever opening the named file reports.
pub fn from_variable(variable: Option<OsString>) -> io::Result<Option<Log>> {
    match variable {
        Some(path) if !path.is_empty() => Log::open(Path::new(&path)).map(Some),
        _ => Ok(None),
    }
}

/// `path` as a line spells it: absolute against the working directory, `/`-separated, and without
/// the `\\?\` a canonical Windows path carries. An empty path is the working directory, which is
/// what a relative path with no directory part is relative to.
#[must_use]
pub fn shown(path: &Path) -> String {
    let absolute = if path.as_os_str().is_empty() {
        std::env::current_dir().unwrap_or_default()
    } else {
        std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
    };
    let text = absolute.to_string_lossy();
    if cfg!(windows) {
        let text = text.replace('\\', "/");
        if let Some(unc) = text.strip_prefix("//?/UNC/") {
            return format!("//{unc}");
        }
        return text.strip_prefix("//?/").unwrap_or(&text).to_string();
    }
    text.into_owned()
}

static LOG: OnceLock<Option<Mutex<Log>>> = OnceLock::new();

/// The process's log, opened on first use from [`LOG_ENV`].
fn log() -> Option<&'static Mutex<Log>> {
    LOG.get_or_init(|| match from_variable(std::env::var_os(LOG_ENV)) {
        Ok(log) => log.map(Mutex::new),
        Err(error) => stop(&format!("could not open the log {LOG_ENV} names: {error}")),
    })
    .as_ref()
}

/// Ends the process after saying why, for a log that cannot take a line.
fn stop(why: &str) -> ! {
    let _ = writeln!(io::stderr(), "nvs: {why}");
    std::process::abort();
}

thread_local! {
    static CAPTURED: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}

/// Whether this thread is inside [`capture`].
fn capturing() -> bool {
    CAPTURED.with(|captured| captured.borrow().is_some())
}

/// Runs `body` and returns what it returned beside every line this thread wrote meanwhile, in
/// order and without the newline. Nothing goes to the log while it runs, and a line is kept each
/// time it is written. The lines are written whether or not `NVS_FOOTPRINT_LOG` is set, so a unit
/// test sees exactly what a recorded run would.
pub fn capture<R>(body: impl FnOnce() -> R) -> (R, Vec<String>) {
    let outer = CAPTURED.with(|captured| captured.borrow_mut().replace(Vec::new()));
    let result = body();
    let lines = CAPTURED.with(|captured| {
        let mut captured = captured.borrow_mut();
        let lines = captured.take().unwrap_or_default();
        *captured = outer;
        lines
    });
    (result, lines)
}

fn write(kind: Kind, value: &str) {
    let captured = CAPTURED.with(|captured| {
        captured.borrow_mut().as_mut().is_some_and(|lines| {
            lines.push(line(kind, value).trim_end_matches('\n').to_string());
            true
        })
    });
    if captured {
        return;
    }
    let Some(log) = log() else { return };
    let mut log = log.lock().unwrap_or_else(PoisonError::into_inner);
    if let Err(error) = log.record(kind, value) {
        let path = log.path.display().to_string();
        stop(&format!("could not write to {path}: {error}"));
    }
}

/// Whether a log is being written, for a caller that would otherwise build a value for nothing.
#[must_use]
pub fn enabled() -> bool {
    capturing() || log().is_some()
}

/// Records that the class `name` was looked up in the registry. A name outside the `Core`
/// namespace is not recorded: the registry holds none, so its answer for one cannot change.
pub fn class(name: &str) {
    if enabled() && !quiet() && is_core(name) {
        write(Kind::Class, name);
    }
}

/// Whether `name` is in the `Core` namespace, compared without regard to ASCII case as the
/// compiler compares a namespace.
fn is_core(name: &str) -> bool {
    name.get(..5)
        .is_some_and(|head| head.eq_ignore_ascii_case("core\\"))
}

/// Records that every class was read, as a list of the whole registry is.
pub fn every_class() {
    if enabled() && !quiet() {
        write(Kind::Class, "*");
    }
}

/// Records that the card of the class or enum `name` was printed or served: its reference card,
/// a member's card or its intro. A name outside `Core` is not recorded, as for [`class`].
pub fn card(name: &str) {
    if enabled() && is_core(name) {
        write(Kind::Card, name);
    }
}

/// Records that every card was printed or served, as a document of the whole registry does.
pub fn every_card() {
    if enabled() {
        write(Kind::Card, "*");
    }
}

/// Records that a test asked for the file or directory at `path`, which stands for everything
/// beneath it.
pub fn tree(path: &Path) {
    if enabled() {
        write(Kind::Tree, &shown(path));
    }
}

/// Records that a test read every file called `name`, wherever in the tree it is.
pub fn named(name: &str) {
    if enabled() {
        write(Kind::Named, name);
    }
}

thread_local! {
    static QUIET: Cell<u32> = const { Cell::new(0) };
}

fn quiet() -> bool {
    QUIET.with(|depth| depth.get() > 0)
}

/// While one is held, the thread records no class: for a table built from the whole registry
/// before any program is looked at, such as the type checker's seeded signatures or the runtime's
/// class descriptors. What a program uses is recorded where it looks an entry of that table up,
/// and a table every program builds says nothing about any one of them.
#[derive(Debug)]
#[must_use = "classes are recorded again as soon as the guard is dropped"]
pub struct Quiet(());

impl Quiet {
    /// Stops recording classes on this thread until the guard is dropped.
    pub fn new() -> Self {
        QUIET.with(|depth| depth.set(depth.get() + 1));
        Self(())
    }
}

impl Default for Quiet {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Quiet {
    fn drop(&mut self) {
        QUIET.with(|depth| depth.set(depth.get().saturating_sub(1)));
    }
}

/// Records that the file at `path` was read whole.
pub fn file(path: &Path) {
    if enabled() {
        write(Kind::File, &shown(path));
    }
}

/// Records that the directory at `path` was listed.
pub fn dir(path: &Path) {
    if enabled() {
        write(Kind::Dir, &shown(path));
    }
}

/// Records that `path` was tested for existence.
pub fn exists(path: &Path) {
    if enabled() {
        write(Kind::Exists, &shown(path));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A log file in the temporary directory, removed when dropped.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir()
                .join(format!("nvs-footprint-{name}-{}.log", std::process::id()));
            let _ = std::fs::remove_file(&path);
            Self(path)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    #[test]
    fn a_line_is_the_kind_a_tab_and_the_value() {
        assert_eq!(line(Kind::Class, r"Core\Math"), "class\tCore\\Math\n");
        assert_eq!(line(Kind::File, "/srv/a\tb\nc"), "file\t/srv/a b c\n");
        assert_eq!(line(Kind::Dir, "/srv"), "dir\t/srv\n");
        assert_eq!(line(Kind::Exists, "/srv/x"), "exists\t/srv/x\n");
        assert_eq!(line(Kind::Card, "*"), "card\t*\n");
        assert_eq!(line(Kind::Tree, "/srv/tests"), "tree\t/srv/tests\n");
        assert_eq!(line(Kind::Named, "nvs.toml"), "named\tnvs.toml\n");
    }

    #[test]
    fn a_log_writes_each_line_once_and_two_logs_share_one_file() {
        let scratch = Scratch::new("shared");
        let mut first = Log::open(&scratch.0).expect("the log opens");
        let mut second = Log::open(&scratch.0).expect("a second writer opens it too");
        first.record(Kind::Class, r"Core\Math").expect("written");
        first.record(Kind::Class, r"Core\Math").expect("written");
        second
            .record(Kind::File, "/srv/app/data.txt")
            .expect("written");
        first
            .record(Kind::Exists, "/srv/app/missing")
            .expect("written");
        let text = std::fs::read_to_string(&scratch.0).expect("the log is there");
        assert_eq!(
            text,
            "class\tCore\\Math\nfile\t/srv/app/data.txt\nexists\t/srv/app/missing\n"
        );
    }

    #[test]
    fn no_variable_or_an_empty_one_names_no_log() {
        assert!(from_variable(None).expect("nothing to open").is_none());
        assert!(
            from_variable(Some(OsString::new()))
                .expect("nothing to open")
                .is_none()
        );
        let scratch = Scratch::new("named");
        let named = from_variable(Some(scratch.0.clone().into_os_string())).expect("it opens");
        assert!(named.is_some());
    }

    #[test]
    fn a_path_is_shown_absolute_and_slash_separated() {
        let shown = shown(Path::new("some/relative/file.txt"));
        assert!(Path::new(&shown).is_absolute(), "{shown}");
        assert!(shown.ends_with("some/relative/file.txt"), "{shown}");
        assert!(!shown.contains('\\'), "{shown}");
    }

    #[test]
    fn an_empty_path_is_the_working_directory() {
        let shown = shown(Path::new(""));
        assert!(Path::new(&shown).is_absolute(), "{shown:?}");
        assert!(shown.ends_with(&shown_tail()), "{shown:?}");
    }

    /// The last component of the path `shown` gives for `.`, which names the same directory.
    fn shown_tail() -> String {
        let dot = shown(Path::new("."));
        let dot = dot.strip_suffix("/.").unwrap_or(&dot);
        dot.rsplit('/').next().unwrap_or_default().to_string()
    }

    #[test]
    fn a_capture_keeps_this_threads_lines_in_order() {
        let ((), lines) = capture(|| {
            class(r"Core\Math");
            card(r"Core\Math");
            card(r"App\Helper");
            every_card();
            named("Cargo.toml");
            let _quiet = Quiet::new();
            class(r"Core\Str");
        });
        assert_eq!(
            lines,
            [
                "class\tCore\\Math",
                "card\tCore\\Math",
                "card\t*",
                "named\tCargo.toml"
            ]
        );
        let ((), outside) = capture(|| ());
        assert!(outside.is_empty(), "a capture starts empty: {outside:?}");
    }

    #[test]
    fn a_nested_capture_gives_the_outer_one_back() {
        let ((), outer) = capture(|| {
            card(r"Core\Arr");
            let ((), inner) = capture(|| card(r"Core\Str"));
            assert_eq!(inner, ["card\tCore\\Str"]);
            card(r"Core\Uri");
        });
        assert_eq!(outer, ["card\tCore\\Arr", "card\tCore\\Uri"]);
    }

    #[test]
    fn only_a_core_name_is_a_class_worth_recording() {
        assert!(is_core(r"Core\Math"));
        assert!(is_core(r"core\math"));
        assert!(!is_core(r"App\Helper"));
        assert!(!is_core("Core"));
        assert!(!is_core(r"Corex\A"));
    }

    #[test]
    fn a_quiet_guard_holds_until_the_last_one_is_dropped() {
        assert!(!quiet());
        let outer = Quiet::new();
        {
            let _inner = Quiet::new();
            assert!(quiet());
        }
        assert!(quiet(), "the outer guard still holds");
        drop(outer);
        assert!(!quiet());
        let other = std::thread::spawn(|| {
            let _held = Quiet::new();
            quiet()
        });
        assert!(other.join().expect("the thread ends"));
        assert!(
            !quiet(),
            "a guard on another thread leaves this one recording"
        );
    }

    #[cfg(windows)]
    #[test]
    fn a_verbatim_windows_path_loses_its_prefix() {
        assert_eq!(shown(Path::new(r"\\?\D:\srv\app")), "D:/srv/app");
        assert_eq!(shown(Path::new(r"\\?\UNC\host\share\a")), "//host/share/a");
    }
}
