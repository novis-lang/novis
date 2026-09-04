//! Running one [`Case`] and deciding whether it passed.
//!
//! ## Why a subprocess
//!
//! Every program a case names is run by spawning the `nvs` binary, not by
//! calling the compiler in-process. It costs a process launch per case, and
//! buys three things worth more than that: a case that hits a contained
//! engine failure ([ADR 0020](/docs/adr/0020-error-escalation-ladder.md))
//! reports as one failure instead of taking the runner down with it; the exit
//! status and the two output streams are the same ones a user sees; and the
//! PHP oracle is reached exactly the same way, so the differential leg is not
//! a second mechanism.
//!
//! ## Why the program is always called `case.nvs`
//!
//! A diagnostic renders the path it was given, so an absolute temporary path
//! would put a machine-specific string into every expectation that covers a
//! compile error. The program is written into a fresh directory as
//! `case.nvs`, and the child's working directory is that directory, so the
//! rendered path is exactly `case.nvs` on both CI legs.

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use crate::case::{Case, Oracle, Subcommand};
use crate::expect::{matches, normalize, shown};

/// How the runner reaches the two binaries it drives.
#[derive(Debug, Clone)]
pub struct Options {
    /// The `nvs` binary that runs each case.
    pub nvs: PathBuf,
    /// The PHP binary a `--ORACLE--` case is compared against.
    pub php: PathBuf,
    /// Run only cases whose path or title contains this.
    pub filter: Option<String>,
}

impl Options {
    /// The defaults: this very binary, and `php` from `PATH`.
    ///
    /// # Errors
    ///
    /// Fails when the running executable's own path cannot be determined.
    pub fn from_current_exe() -> io::Result<Self> {
        Ok(Self {
            nvs: std::env::current_exe()?,
            php: PathBuf::from("php"),
            filter: None,
        })
    }
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

/// True when [`Options::php`] names something that can actually be run.
///
/// Probed once per suite rather than per case: a `--ORACLE--` case without an
/// oracle is unrunnable, not failing, and reporting sixty identical failures
/// on a machine that simply has no PHP installed would drown the one line
/// that says so.
#[must_use]
pub fn php_available(opts: &Options) -> bool {
    Command::new(&opts.php).arg("--version").output().is_ok()
}

/// Runs `case` in `workdir`, which must already exist and be empty.
///
/// `php_available` is [`php_available`]'s answer for this suite run; a
/// `--ORACLE--` case is skipped, with the reason, when it is false.
#[must_use]
pub fn run_case(case: &Case, opts: &Options, workdir: &Path, php_available: bool) -> Outcome {
    if let Some(why) = &case.unsupported {
        return Outcome::Fail(vec![format!("unsupported: {why}")]);
    }
    if !php_available && matches!(case.oracle, Some(Oracle::Php(_))) {
        return Outcome::Skip(format!(
            "no PHP oracle: `{}` could not be run (pass --php)",
            opts.php.display()
        ));
    }

    // Auxiliary files go down before anything runs, including `--SKIPIF--`:
    // they are part of the tree the case is written against, not part of one
    // program's input.
    for aux in &case.aux {
        if let Err(error) = write_aux(workdir, &aux.path, &aux.body) {
            return Outcome::Fail(vec![format!("--FILE {}--: {error}", aux.path)]);
        }
    }

    if let Some(source) = &case.skipif {
        match run_nvs(
            opts,
            workdir,
            "skipif.nvs",
            source,
            Subcommand::Run,
            &[],
            &case.env,
        ) {
            Err(error) => return Outcome::Fail(vec![format!("--SKIPIF--: {error}")]),
            Ok(output) => {
                let text = String::from_utf8_lossy(&output.stdout);
                let text = text.trim();
                if let Some(reason) = text.strip_prefix("skip") {
                    let reason = reason.trim();
                    return Outcome::Skip(if reason.is_empty() {
                        "--SKIPIF--".to_owned()
                    } else {
                        reason.to_owned()
                    });
                }
            }
        }
    }

    let outcome = judge(case, opts, workdir);

    if let Some(source) = &case.clean {
        // `--CLEAN--`'s whole job is tidying up after the case; its own
        // output is not an expectation and a failure in it must not turn a
        // passing case red.
        let _ = run_nvs(
            opts,
            workdir,
            "clean.nvs",
            source,
            Subcommand::Run,
            &[],
            &case.env,
        );
    }
    outcome
}

fn judge(case: &Case, opts: &Options, workdir: &Path) -> Outcome {
    let output = match run_nvs(
        opts, workdir, "case.nvs", &case.file, case.run, &case.args, &case.env,
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

    if let Some(Oracle::Php(twin)) = &case.oracle {
        match run_php(opts, workdir, twin, &case.env) {
            Err(error) => report.push(format!("--ORACLE--: could not run PHP: {error}")),
            Ok(oracle) => {
                let theirs = String::from_utf8_lossy(&oracle.stdout).into_owned();
                if !oracle.status.success() {
                    report.push(format!(
                        "--ORACLE--: PHP exited {}",
                        oracle.status.code().unwrap_or(-1)
                    ));
                    report.push(indented(
                        "php stderr",
                        &String::from_utf8_lossy(&oracle.stderr),
                    ));
                } else if normalize(&theirs) != normalize(&stdout) {
                    report.push("Novis and its PHP twin printed different things".to_owned());
                    report.push(indented("php", &theirs));
                    report.push(indented("nvs", &stdout));
                }
            }
        }
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
fn run_nvs(
    opts: &Options,
    workdir: &Path,
    name: &str,
    source: &str,
    sub: Subcommand,
    program_args: &[String],
    env: &[(String, String)],
) -> io::Result<Output> {
    fs::write(workdir.join(name), source)?;
    let mut args: Vec<&OsStr> = sub.args().iter().map(AsRef::as_ref).collect();
    if sub.takes_file() {
        args.push(name.as_ref());
    }
    // Past the file, so `nvs run` reads none of them as its own — the same
    // boundary `nvs run`'s trailing arguments have on a real command line, and
    // the reason a case's arguments can name a `--dryRun` of their own.
    args.extend(program_args.iter().map(|arg| arg.as_ref() as &OsStr));
    spawn(&opts.nvs, &args, workdir, env)
}

/// Writes `source` into `workdir` as `oracle.php` and runs PHP on it.
///
/// Under the case's own `--ENV--` as well: a differential case is only a
/// comparison if both halves were asked the same question, and `getenv` is a
/// question about the environment.
fn run_php(
    opts: &Options,
    workdir: &Path,
    source: &str,
    env: &[(String, String)],
) -> io::Result<Output> {
    fs::write(workdir.join("oracle.php"), source)?;
    spawn(&opts.php, &["oracle.php".as_ref()], workdir, env)
}

fn spawn(
    program: &Path,
    args: &[&OsStr],
    workdir: &Path,
    env: &[(String, String)],
) -> io::Result<Output> {
    Command::new(program)
        .args(args)
        .current_dir(workdir)
        // `--ENV--`, added to the environment this process already has rather
        // than replacing it — [`nvs_test::case::Case::env`] owns why, and the
        // `NO_COLOR` below is set after it on purpose: a case that wanted
        // coloured output would be pinning escape codes, which no case is
        // about.
        .envs(env.iter().map(|(name, value)| (name, value)))
        // A diagnostic decides on colour by asking whether stderr is a
        // terminal, which a captured pipe is not — but a CI runner that sets
        // `CLICOLOR_FORCE` would still colour it, and escape codes in an
        // expectation are not what any of these cases are about.
        .env("NO_COLOR", "1")
        .output()
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
        let root = std::env::temp_dir().join(format!("nvs-test-aux-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("a fresh working directory");

        write_aux(&root, "src/App/Greeter.nvs", "<?nvs\n").expect("it writes");
        let written = fs::read_to_string(root.join("src").join("App").join("Greeter.nvs"))
            .expect("it is where the path said");
        assert_eq!(written, "<?nvs\n");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_empty_block_says_so_rather_than_showing_nothing() {
        assert_eq!(indented("actual", "\n\n"), "  actual: <empty>");
    }

    #[test]
    fn a_block_indents_every_line_under_its_label() {
        assert_eq!(indented("php", "a\nb\n"), "  php:\n    a\n    b");
    }
}
