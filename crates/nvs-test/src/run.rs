//! Running one [`Case`] and deciding whether it passed.
//!
//! ## Why a subprocess
//!
//! Every program a case names is run by spawning the `nvs` binary, not by
//! calling the compiler in-process. It costs a process launch per case, and
//! buys what is worth more than that: a case that hits a contained
//! engine failure (`rule:errors/escalation-ladder`)
//! reports as one failure instead of taking the runner down with it; the exit
//! status and the two output streams are the same ones a user sees.
//!
//! ## Why the program is always called `case.nvs`
//!
//! A diagnostic renders the path it was given, so an absolute temporary path
//! would put a machine-specific string into every expectation that covers a
//! compile error. The program is written into a fresh directory as
//! `case.nvs`, and the child's working directory is that directory, so the
//! rendered path is exactly `case.nvs` on both CI legs.
//!
//! ## Known gap
//!
//! Each gap is a record, and `bun nv gaps --module crates/nvs-test/src/run.rs` lists them.

use std::ffi::OsStr;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use crate::case::{Case, Subcommand};
use crate::expect::{matches, normalize, shown};

/// How long one case's process may run before the runner gives up on it.
///
/// Not a performance budget. It is the line past which a case has stopped
/// running and started being wedged, and it is set far above what the slowest
/// healthy case costs rather than close to it.
///
/// The headroom is for the handful of cases that spawn child processes, whose
/// cost is set by how contended the machine is and not by what they compute: a
/// pooled suite sharing a machine with a build makes every process creation a
/// scheduling question, and a deadline drawn just above what those cases take
/// on an idle machine reports them as wedged on a busy one. That failure reads
/// as a defect in whatever the case was about, which is the most expensive
/// wrong answer this constant can give.
///
/// It exists because without it one wedged case takes the suite with it, and a
/// suite that hangs reports nothing at all. On a hosted runner that is the
/// difference between a red build and one that burns the whole job budget
/// before anything says why — which this still buys, a wedged case being one
/// that would not have finished at any deadline.
pub const CASE_TIMEOUT: Duration = Duration::from_secs(300);

/// How the runner reaches the binary it drives, and which cases it runs.
#[derive(Debug, Clone)]
pub struct Options {
    /// The `nvs` binary that runs each case.
    pub nvs: PathBuf,
    /// Run only cases whose path or title contains this.
    pub filter: Option<String>,
    /// How many cases are in flight at once. Every case is its own process
    /// in its own directory already, so this only says how many of those
    /// run side by side; [`crate::run()`] owns what the pool buys.
    pub jobs: usize,
    /// How long one case's process may run before it is killed and reported as
    /// a failure. [`CASE_TIMEOUT`] is the default and owns the reasoning.
    pub timeout: Duration,
    /// Run only these case files of the trees, compared by path with either
    /// slash. `None` runs every case the trees hold; [`crate::run()`] refuses a
    /// listed file no tree holds.
    pub only: Option<Vec<PathBuf>>,
    /// Record each case into this directory: every `nvs` process the case
    /// starts writes its coverage counters and its footprint log there, under
    /// the case's [`record_name`], and compiles with no artifact cache.
    pub record: Option<PathBuf>,
    /// The directory each case's working directory is made under. The run
    /// creates it when it is missing and deletes it, whole, when it finishes,
    /// so it is a directory this run owns: `nvs test` passes a fresh one under
    /// the runtime's temporary root.
    pub root: PathBuf,
}

impl Options {
    /// The defaults: this very binary, and as many cases
    /// at once as the machine has hardware threads, writing under `root`.
    ///
    /// # Errors
    ///
    /// Fails when the running executable's own path cannot be determined.
    pub fn from_current_exe(root: PathBuf) -> io::Result<Self> {
        Ok(Self {
            nvs: std::env::current_exe()?,
            filter: None,
            jobs: std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get),
            timeout: CASE_TIMEOUT,
            only: None,
            record: None,
            root,
        })
    }
}

/// The file name a recorded case's files start with, from the path the report
/// names it by: `/` and `\` become `~`, every other byte that is not an ASCII
/// letter, digit, `.`, `_` or `-` becomes `@` and two hex digits, so two paths
/// never share a name and none holds a `%` the profile runtime would expand.
/// `tests/conformance/a b.nvst` is `tests~conformance~a@20b.nvst`.
///
/// A name longer than [`RECORD_NAME_MAX`] bytes keeps its first
/// [`RECORD_NAME_KEEP`] and ends in `@` and the 64-bit FNV-1a hash of the whole
/// name as sixteen hex digits, so the path a profile is written to stays inside
/// Windows' 260-character limit however long a case's name is.
/// `tools/nv/proofs/run.ts`'s `recordName` follows the same rule.
#[must_use]
pub fn record_name(label: &str) -> String {
    let mut name = String::with_capacity(label.len());
    for byte in label.bytes() {
        match byte {
            b'/' | b'\\' => name.push('~'),
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'.' | b'_' | b'-' => {
                name.push(char::from(byte));
            }
            other => name.push_str(&format!("@{other:02x}")),
        }
    }
    if name.len() <= RECORD_NAME_MAX {
        return name;
    }
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in name.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{}@{hash:016x}", &name[..RECORD_NAME_KEEP])
}

/// The longest name [`record_name`] leaves whole.
pub const RECORD_NAME_MAX: usize = 96;
/// How much of a longer name it keeps before the hash.
pub const RECORD_NAME_KEEP: usize = 64;

/// The environment every `nvs` process of `case` gets when [`Options::record`]
/// names a directory: `LLVM_PROFILE_FILE` as `<dir>/<name>-%p.profraw`, one
/// file per process, `NVS_FOOTPRINT_LOG` as `<dir>/<name>.log`, which they
/// share, and `NOVIS_NO_FILE_CACHE`, so each compile runs whole. Empty when
/// nothing is recorded.
#[must_use]
pub fn recording(case: &Case, opts: &Options) -> Vec<(String, String)> {
    let Some(dir) = &opts.record else {
        return Vec::new();
    };
    let name = record_name(&case.path.display().to_string());
    vec![
        (
            "LLVM_PROFILE_FILE".to_owned(),
            dir.join(format!("{name}-%p.profraw")).display().to_string(),
        ),
        (
            "NVS_FOOTPRINT_LOG".to_owned(),
            dir.join(format!("{name}.log")).display().to_string(),
        ),
        ("NOVIS_NO_FILE_CACHE".to_owned(), "1".to_owned()),
    ]
}

/// What running one case came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Every expectation the case stated held.
    Pass,
    /// `--SKIPIF--` said not to run it, and why.
    Skip(String),
    /// One or more expectations did not hold; the lines are the report, in
    /// the order they should be printed.
    Fail(Vec<String>),
}

/// Runs `case` in `workdir`, which must already exist and be empty.
#[must_use]
pub fn run_case(case: &Case, opts: &Options, workdir: &Path) -> Outcome {
    // Auxiliary files go down before anything runs, including `--SKIPIF--`:
    // they are part of the tree the case is written against, not part of one
    // program's input.
    for aux in &case.aux {
        if let Err(error) = write_aux(workdir, &aux.path, &aux.body) {
            return Outcome::Fail(vec![format!("--FILE {}--: {error}", aux.path)]);
        }
    }

    let record = recording(case, opts);
    if let Some(source) = &case.skipif {
        match run_nvs(
            opts,
            workdir,
            &Invocation {
                name: "skipif.nvs",
                source,
                sub: Subcommand::Run,
                // Scaffolding, not the thing under test: the request is the
                // case's, and what decides whether to run the case at all must
                // not be answering it.
                request: None,
                args: &[],
                env: &case.env,
                record: &record,
            },
        ) {
            Err(error) => return Outcome::Fail(vec![format!("--SKIPIF--: {error}")]),
            Ok(output) => {
                if let Some(outcome) = skipif_verdict(&output) {
                    return outcome;
                }
            }
        }
    }

    let outcome = judge(case, opts, workdir, &record);

    if let Some(source) = &case.clean {
        // `--CLEAN--`'s whole job is tidying up after the case; its own
        // output is not an expectation and a failure in it must not turn a
        // passing case red.
        let _ = run_nvs(
            opts,
            workdir,
            &Invocation {
                name: "clean.nvs",
                source,
                sub: Subcommand::Run,
                request: None,
                args: &[],
                env: &case.env,
                record: &record,
            },
        );
    }
    outcome
}

/// What a `--SKIPIF--` program's run says about its case, or `None` to run it.
///
/// A SKIPIF that exits non-zero fails the case with its stderr, whatever it
/// printed. One that does not compile prints nothing, so reading its stdout
/// alone would run a case whose author meant to guard it — green on a machine
/// where the guarded service is up, red everywhere else.
fn skipif_verdict(output: &Output) -> Option<Outcome> {
    if !output.status.success() {
        let failed = match output.status.code() {
            Some(code) => format!("--SKIPIF-- failed; it exited {code}"),
            None => "--SKIPIF-- failed".to_owned(),
        };
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Some(Outcome::Fail(vec![failed, indented("stderr", &stderr)]));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let reason = text.trim().strip_prefix("skip")?.trim();
    Some(Outcome::Skip(if reason.is_empty() {
        "--SKIPIF--".to_owned()
    } else {
        reason.to_owned()
    }))
}

fn judge(case: &Case, opts: &Options, workdir: &Path, record: &[(String, String)]) -> Outcome {
    // The child is a separate process, so a request reaches it as a file it is
    // pointed at — `crate::request` owns that format and why it is a file
    // rather than something in the environment.
    let request = match &case.request {
        None => None,
        Some(request) => {
            let at = workdir.join(crate::request::FILE_NAME);
            if let Err(error) = fs::write(at, crate::request::render(request)) {
                return Outcome::Fail(vec![format!("could not write the request: {error}")]);
            }
            Some(crate::request::FILE_NAME)
        }
    };
    let output = match run_nvs(
        opts,
        workdir,
        &Invocation {
            name: "case.nvs",
            source: &case.file,
            sub: case.run,
            request,
            args: &case.args,
            env: &case.env,
            record,
        },
    ) {
        Ok(output) => output,
        Err(error) => return Outcome::Fail(vec![format!("could not run the case: {error}")]),
    };
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let mut report = Vec::new();

    // One rule for the exit status, so a case cannot pass by printing the
    // right thing on its way to an unexpected failure: a case that states an
    // error expectation must fail, and every other case must succeed.
    let ok = output.status.success();
    if ok == case.expects_failure() {
        let wanted = if case.expects_failure() {
            "expected the run to fail, and it succeeded"
        } else {
            "expected the run to succeed"
        };
        report.push(match output.status.code() {
            Some(code) if !ok => format!("{wanted}; it exited {code}"),
            _ => wanted.to_owned(),
        });
        if !ok && !stderr.trim().is_empty() {
            report.push(indented("stderr", &stderr));
        }
    }

    if let Some(expected) = &case.expect
        && !matches(expected, &stdout)
    {
        report.push("standard output does not match".to_owned());
        report.push(indented("expected", &shown(expected)));
        report.push(indented("actual", &stdout));
    }
    if let Some(expected) = &case.expect_error
        && !matches(expected, &stderr)
    {
        report.push("standard error does not match".to_owned());
        report.push(indented("expected", &shown(expected)));
        report.push(indented("actual", &stderr));
    }

    if report.is_empty() {
        Outcome::Pass
    } else {
        Outcome::Fail(report)
    }
}

/// Writes one auxiliary file, creating the directories its path names.
///
/// `relative` has already been checked by the parser to be a `/`-separated
/// relative path with no `.` or `..` segment, so joining it onto `workdir`
/// cannot reach outside it.
fn write_aux(workdir: &Path, relative: &str, body: &str) -> io::Result<()> {
    let target = relative
        .split('/')
        .fold(workdir.to_path_buf(), |path, segment| path.join(segment));
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(target, body)
}

/// Writes `source` into `workdir` as `name` and runs `nvs <sub>` on it.
///
/// `sub` is [`Subcommand::args`]'s own list, so the two things that decide it
/// — a case's `--RUN--` section and the scaffolding's fixed `run` — cannot
/// spell a command line this binary does not have. `name` is written either
/// way; whether it is also *named* on that command line is
/// [`Subcommand::takes_file`]'s, since `nvs config dump` reads the working
/// directory rather than a file on argv.
///
/// [`Invocation::request`] names a file already written into `workdir`, and
/// only the case's own program is ever given one — [`crate::case::parse`] is
/// what makes sure the subcommand asked for can take it.
struct Invocation<'a> {
    /// What the source is written into the working directory as, and what is
    /// named on the command line where the subcommand takes a file.
    name: &'a str,
    /// The program itself.
    source: &'a str,
    /// Which `nvs` subcommand it goes through.
    sub: Subcommand,
    /// The request file already written beside it, for the one subcommand that
    /// reads one.
    request: Option<&'a str>,
    /// The program's own arguments, which go past the file.
    args: &'a [String],
    /// `--ENV--`, added to the environment the runner already has.
    env: &'a [(String, String)],
    /// [`recording`]'s variables, which only the `nvs` half gets.
    record: &'a [(String, String)],
}

fn run_nvs(opts: &Options, workdir: &Path, run: &Invocation<'_>) -> io::Result<Output> {
    fs::write(workdir.join(run.name), run.source)?;
    let mut args: Vec<&OsStr> = run.sub.args().iter().map(AsRef::as_ref).collect();
    // Ahead of the file, because everything past the file is the program's
    // own: a `--request` written the other side of it would be handed to the
    // program as one of its arguments instead of read by `nvs run`.
    if let Some(file) = run.request {
        args.push("--request".as_ref());
        args.push(file.as_ref());
    }
    if run.sub.takes_file() {
        args.push(run.name.as_ref());
    }
    // Past the file, so `nvs run` reads none of them as its own — the same
    // boundary `nvs run`'s trailing arguments have on a real command line, and
    // the reason a case's arguments can name a `--dryRun` of their own.
    args.extend(run.args.iter().map(|arg| arg.as_ref() as &OsStr));
    spawn(
        &opts.nvs,
        &args,
        workdir,
        &[run.env, run.record],
        opts.timeout,
    )
}

/// Runs `program` in `workdir` and collects its output, for at most `timeout`.
///
/// Like [`Command::output`] but with a deadline, which `std` has no version of.
/// `env` is each list of variables to add, in order, so a later list wins.
fn spawn(
    program: &Path,
    args: &[&OsStr],
    workdir: &Path,
    env: &[&[(String, String)]],
    timeout: Duration,
) -> io::Result<Output> {
    let mut child = Command::new(program)
        .args(args)
        .current_dir(workdir)
        // `--ENV--`, added to the environment this process already has rather
        // than replacing it — [`nvs_test::case::Case::env`] owns why, and the
        // `NO_COLOR` below is set after it on purpose: a case that wanted
        // coloured output would be pinning escape codes, which no case is
        // about.
        .envs(
            env.iter()
                .flat_map(|list| list.iter())
                .map(|(name, value)| (name, value)),
        )
        // A diagnostic decides on colour by asking whether stderr is a
        // terminal, which a captured pipe is not — but a CI runner that sets
        // `CLICOLOR_FORCE` would still colour it, and escape codes in an
        // expectation are not what any of these cases are about.
        .env("NO_COLOR", "1")
        // What `output` would set for us, and it has to be said here because
        // the wait below is this function's own: a child inheriting this
        // process's standard input could block on a read nobody will answer,
        // and the two pipes are what the readers in `wait_within` drain.
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    wait_within(&mut child, timeout, program)
}

/// Waits for `child` for at most `limit`, killing it if it runs over.
///
/// [`Command::output`] with a deadline, which is what `std` has no version of.
/// A thread per pipe, because a child that filled the pipe buffer would block
/// on the write and never reach the exit this is waiting for; the deadline then
/// belongs to the *reads* rather than to the child, which is what keeps this
/// off a clock. The alternative — a `try_wait` loop with a sleep in it — puts
/// the whole suite on the host's timer granularity, since `thread::sleep` on
/// Windows rounds up to the system timer's resolution and a poll asking for a
/// hundred microseconds can get fifteen milliseconds. Nothing here sleeps, so
/// the deadline costs the conformance tree nothing measurable.
///
/// End of file on both pipes means the process has let go of them, which for
/// the `nvs` process this runs means it is on its way
/// out, so the `wait` after them returns at once. A child that closed its own
/// output and then kept running would be waited on past the deadline, and that
/// is [`Command::output`]'s behaviour too; what this rules out is the case that
/// actually happens, a program wedged with its pipes still open.
///
/// The kill is likewise what lets this return at all when the deadline passes:
/// the pipes close with the process, and that is what ends the two readers,
/// which the scope cannot be left without.
///
/// # Errors
///
/// [`io::ErrorKind::TimedOut`] when the deadline passed, naming the program, so
/// a wedged case reports as one failure with a reason on it rather than as a
/// suite that never finished. Otherwise as [`Child::wait`].
fn wait_within(child: &mut Child, limit: Duration, program: &Path) -> io::Result<Output> {
    let mut out = child
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("the child's standard output was not a pipe"))?;
    let mut err = child
        .stderr
        .take()
        .ok_or_else(|| io::Error::other("the child's standard error was not a pipe"))?;
    let deadline = Instant::now() + limit;

    thread::scope(|scope| {
        // Each reader reports through the channel as well as through its join
        // handle: the handle is how the bytes come back, and the channel is the
        // only one of the two that can be waited on with a deadline.
        let (report, reported) = mpsc::channel::<()>();
        let reading_out = scope.spawn({
            let report = report.clone();
            move || {
                let mut bytes = Vec::new();
                let read = out.read_to_end(&mut bytes);
                drop(report);
                read.map(|_| bytes)
            }
        });
        let reading_err = scope.spawn(move || {
            let mut bytes = Vec::new();
            let read = err.read_to_end(&mut bytes);
            drop(report);
            read.map(|_| bytes)
        });

        // Both senders dropped is both pipes at end of file. `RecvTimeoutError`
        // either way — a timeout, or a disconnect once the second one goes —
        // so what separates them is which came first, and the deadline is the
        // only question being asked.
        let mut overran = false;
        while !overran {
            match reported.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => overran = true,
                Ok(()) => {}
            }
        }
        if overran {
            let _ = child.kill();
        }

        let panicked = |_| io::Error::other("the thread reading the child's output panicked");
        let stdout = reading_out.join().map_err(panicked)??;
        let stderr = reading_err.join().map_err(panicked)??;
        let status = child.wait()?;
        if overran {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!(
                    "`{}` did not finish within {}s and was killed",
                    program.display(),
                    limit.as_secs()
                ),
            ));
        }
        Ok(Output {
            status,
            stdout,
            stderr,
        })
    })
}

/// One report block: a label, then the text indented under it.
fn indented(label: &str, text: &str) -> String {
    let text = normalize(text);
    if text.is_empty() {
        return format!("  {label}: <empty>");
    }
    let mut out = format!("  {label}:\n");
    for line in text.lines() {
        out.push_str("    ");
        out.push_str(line);
        out.push('\n');
    }
    out.pop();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_auxiliary_file_lands_under_the_directories_its_path_names() {
        let root = nvs_repo::scratch("test-aux");

        write_aux(&root, "src/App/Greeter.nvs", "<?nvs\n").expect("it writes");
        let written = fs::read_to_string(root.join("src").join("App").join("Greeter.nvs"))
            .expect("it is where the path said");
        assert_eq!(written, "<?nvs\n");
    }

    #[test]
    fn a_record_name_keeps_the_path_readable_and_two_paths_apart() {
        assert_eq!(
            record_name("tests/conformance/a b.nvst"),
            "tests~conformance~a@20b.nvst"
        );
        assert_eq!(
            record_name(r"tests\conformance\a b.nvst"),
            "tests~conformance~a@20b.nvst",
            "either slash names the same case"
        );
        assert_eq!(record_name("50%.nvst"), "50@25.nvst");
        assert_ne!(record_name("a~b.nvst"), record_name("a/b.nvst"));
        assert_eq!(record_name("D:/x.nvst"), "D@3a~x.nvst");
    }

    #[test]
    fn a_long_record_name_is_cut_and_ends_in_the_hash_of_the_whole() {
        let long = "tests/conformance/reject/a-binding-refuses-a-second-declaration-an-undeclared-assignment-a-changed-type.nvst";
        // `tools/nv/test/proof-record.test.ts` pins the same name for the same path.
        assert_eq!(
            record_name(long),
            "tests~conformance~reject~a-binding-refuses-a-second-declaration-@43cc62f86b7c4528"
        );
        assert_eq!(
            record_name(&"x".repeat(RECORD_NAME_MAX)).len(),
            RECORD_NAME_MAX
        );
        assert_ne!(
            record_name(&format!("{long}a")),
            record_name(&format!("{long}b"))
        );
    }

    #[test]
    fn a_recorded_case_names_its_profile_its_log_and_a_cold_compile() {
        let case = crate::case::parse(
            Path::new("tests/conformance/math.nvst"),
            "--TEST--\nmath\n--FILE--\n<?nvs\necho 1;\n--EXPECT--\n1\n",
        )
        .expect("the case parses");
        let scratch = nvs_repo::scratch("test-records");
        let mut opts =
            Options::from_current_exe(scratch.join("cases")).expect("this binary has a path");
        assert!(
            recording(&case, &opts).is_empty(),
            "nothing is recorded unless asked"
        );

        let dir = scratch.join("records");
        opts.record = Some(dir.clone());
        let vars = recording(&case, &opts);
        let value = |name: &str| {
            vars.iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| v.clone())
                .unwrap_or_else(|| panic!("{name} is set: {vars:?}"))
        };
        let stem = "tests~conformance~math.nvst";
        assert_eq!(
            value("LLVM_PROFILE_FILE"),
            dir.join(format!("{stem}-%p.profraw")).display().to_string()
        );
        assert_eq!(
            value("NVS_FOOTPRINT_LOG"),
            dir.join(format!("{stem}.log")).display().to_string()
        );
        assert_eq!(value("NOVIS_NO_FILE_CACHE"), "1");
    }

    /// A finished process's output, with `code` as its exit status.
    fn exited(code: i32, stdout: &str, stderr: &str) -> Output {
        #[cfg(unix)]
        let status = std::os::unix::process::ExitStatusExt::from_raw(code << 8);
        #[cfg(windows)]
        let status = std::os::windows::process::ExitStatusExt::from_raw(code.cast_unsigned());
        Output {
            status,
            stdout: stdout.as_bytes().to_vec(),
            stderr: stderr.as_bytes().to_vec(),
        }
    }

    #[test]
    fn skipif_that_exits_nonzero_fails_the_case() {
        let verdict = skipif_verdict(&exited(1, "", "error[E0605]: not granted\n"));
        assert_eq!(
            verdict,
            Some(Outcome::Fail(vec![
                "--SKIPIF-- failed; it exited 1".to_owned(),
                "  stderr:\n    error[E0605]: not granted".to_owned(),
            ]))
        );
        assert!(
            matches!(
                skipif_verdict(&exited(1, "skip no store\n", "")),
                Some(Outcome::Fail(_))
            ),
            "a SKIPIF that failed is not believed when it says skip"
        );
        assert_eq!(
            skipif_verdict(&exited(0, "skip no store\n", "")),
            Some(Outcome::Skip("no store".to_owned()))
        );
        assert_eq!(skipif_verdict(&exited(0, "", "")), None);
    }

    #[test]
    fn an_empty_block_says_so_rather_than_showing_nothing() {
        assert_eq!(indented("actual", "\n\n"), "  actual: <empty>");
    }

    #[test]
    fn a_block_indents_every_line_under_its_label() {
        assert_eq!(indented("actual", "a\nb\n"), "  actual:\n    a\n    b");
    }

    /// Neither a check nor a case: the process the test below spawns, so that
    /// it has something which really does hang. `#[ignore]` keeps it out of an
    /// ordinary run — nothing in `.github/workflows/ci.yml` or `tools/` passes
    /// `--ignored` — and the test asks for it by its exact path.
    ///
    /// A minute rather than forever, so that a hand-typed `--ignored` costs a
    /// minute instead of a wedged terminal. The wait under test is 200ms.
    #[test]
    #[ignore = "spawned by name by the case below; it exists in order to be killed"]
    fn a_process_that_never_finishes_on_purpose() {
        thread::sleep(Duration::from_secs(60));
    }

    #[test]
    fn a_case_whose_process_never_finishes_is_killed_and_reported() {
        let exe = std::env::current_exe().expect("this test binary has a path");
        let cwd = nvs_repo::scratch("test-killed");
        let began = Instant::now();
        let error = spawn(
            &exe,
            &[
                "--ignored".as_ref(),
                "--exact".as_ref(),
                "run::tests::a_process_that_never_finishes_on_purpose".as_ref(),
            ],
            &cwd,
            &[],
            Duration::from_millis(200),
        )
        .expect_err("a process that never exits was waited on all the way to its end");

        assert_eq!(
            error.kind(),
            io::ErrorKind::TimedOut,
            "the wait ended for the wrong reason: {error}"
        );
        // Generous by two orders of magnitude over the deadline: what would
        // fail here is the wait never ending, and the reader threads holding
        // the scope open past the kill is the way that happens.
        assert!(
            began.elapsed() < Duration::from_secs(20),
            "the deadline passed but the wait did not end: {:?}",
            began.elapsed()
        );
    }
}
