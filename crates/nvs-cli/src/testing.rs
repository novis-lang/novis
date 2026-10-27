//! The fixtures more than one module's `mod tests` needs, in the platform's own spelling.
//!
//! A helper lives here when two modules' unit tests manufacture the same state and neither owns
//! it: `crate::cache`'s tests and `crate::config`'s tests both have to produce the directory
//! `rule:config/ownership-is-the-trust-boundary` refuses, and a copy in each is two spellings of
//! one platform question that drift apart the first time a platform's answer changes. Everything
//! else stays in the `mod tests` that uses it — this is what is shared, not a drawer for every
//! fixture.

use std::path::Path;

/// Refuse `dir` the creation of a new file, without touching who owns it — the other half of what
/// a write into a working directory can fail on. Ownership is left alone deliberately: a directory
/// that also failed `rule:config/ownership-is-the-trust-boundary` would be answered by the check in
/// front of the write, and the case would never reach the filesystem it is about.
///
/// Unix drops the owner's write bit. Windows has no bit that means this — the read-only attribute
/// does not stop a file being created in a directory — so it denies `Everyone` the two rights that
/// create one, and a deny entry reaches this account along with every other.
pub(crate) fn refuse_new_files(dir: &Path) {
    #[cfg(unix)]
    {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(dir, fs::Permissions::from_mode(0o555)).expect("a read-only mode");
    }
    #[cfg(windows)]
    {
        let denied = nvs_repo::spawn("icacls", &[])
            .arg(dir)
            .arg("/deny")
            .arg("*S-1-1-0:(OI)(CI)(WD,AD)")
            .output()
            .expect("`icacls` ships with every supported Windows");
        assert!(
            denied.status.success(),
            "`Everyone` could not be denied file creation on {}: {}",
            dir.display(),
            String::from_utf8_lossy(&denied.stderr),
        );
    }
}

/// Make `dir` writable by every local account — the state
/// `rule:config/ownership-is-the-trust-boundary` refuses — in the platform's own spelling, because
/// there is no portable one. Unix is a mode; Windows is an ACE for `Everyone` (`S-1-1-0`), added
/// through `icacls` rather than through `windows-sys` so that a test helper does not cost this
/// crate a dependency and an `unsafe` block. `icacls` is the same command a breach's help tells an
/// operator to undo such a grant with.
pub(crate) fn open_to_the_world(dir: &Path) {
    #[cfg(unix)]
    {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        fs::set_permissions(dir, fs::Permissions::from_mode(0o777)).expect("a world-writable mode");
    }
    #[cfg(windows)]
    {
        let granted = nvs_repo::spawn("icacls", &[])
            .arg(dir)
            .arg("/grant")
            .arg("*S-1-1-0:(OI)(CI)(M)")
            .output()
            .expect("`icacls` ships with every supported Windows");
        assert!(
            granted.status.success(),
            "`Everyone` could not be granted write on {}: {}",
            dir.display(),
            String::from_utf8_lossy(&granted.stderr),
        );
    }
}

/// Held for the whole of every `…_by_the_margin_this_test_names` test, so no two of them measure at
/// once. libtest runs the tests one filter selects in parallel, and a margin holds only on an idle
/// machine: `crate::cache`'s warm-start margin compiles on one busy thread, and beside it on a
/// four-CPU runner one of `crate::serve`'s four pinned workers shares its CPU with the compiler,
/// which roughly doubles the four-core arm and fails the scaling margin.
pub(crate) fn measuring_alone() -> std::sync::MutexGuard<'static, ()> {
    static MEASURING: std::sync::Mutex<()> = std::sync::Mutex::new(());
    MEASURING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// How many times a margin guard measures before its hard bound fails it. A ratio under the hard
/// bound is measured again, and only the last attempt is judged: a burst of load that lands on one
/// attempt passes on the next, and a change of kind is under the bound on every attempt.
const ATTEMPTS: usize = 3;

/// How far the two [`yardstick`] readings around one attempt may differ, as a fraction of the
/// faster one, before that attempt is labelled noisy.
const LOAD_TOLERANCE: f64 = 0.25;

/// The two bounds a `…_by_the_margin_this_test_names` ratio is judged against, as
/// `rule:testing/perf-two-mechanisms` sets them. Under `soft` the guard warns and passes. Under
/// `hard` it fails, and `hard` sits just over the ratio the change of kind the guard exists for
/// reaches.
#[derive(Clone, Copy)]
pub(crate) struct Margin {
    pub(crate) soft: f64,
    pub(crate) hard: f64,
}

/// Measures a margin guard's ratio and judges it against `margin`, the way
/// `benches/abi-probe/tests/perf_guards.rs`'s `judge` judges a timing: up to [`ATTEMPTS`]
/// attempts, each between two readings of a fixed loop, and only the last one can fail.
///
/// `measure` is one attempt, and returns the ratio and the words printed after it. `why` is the
/// sentence a failure adds after the figure: what the change of kind means. Returns the judged
/// ratio.
#[expect(
    clippy::print_stdout,
    reason = "the measured ratio is the guard's output: a CI log that shows only a pass says \
              nothing about how far over its bounds the ratio was"
)]
pub(crate) fn judge_margin(
    what: &str,
    margin: Margin,
    why: &str,
    mut measure: impl FnMut() -> (f64, String),
) -> f64 {
    let Margin { soft, hard } = margin;
    let mut attempt = 1;
    loop {
        let before = yardstick();
        let (ratio, detail) = measure();
        let after = yardstick();
        let drift = before.max(after) / before.min(after) - 1.0;
        let load = if drift > LOAD_TOLERANCE {
            "noisy"
        } else {
            "steady"
        };
        println!(
            "{what}: {ratio:.2}x{detail} [soft {soft}x, hard {hard}x; attempt {attempt} of \
             {ATTEMPTS}, {load}: yardstick {before:.0} ns then {after:.0} ns]"
        );
        // Written as `>` so that a NaN is under the hard bound and fails.
        if ratio > hard {
            if ratio < soft {
                warn(
                    what,
                    &format!(
                        "{ratio:.2}x is under the soft bound of {soft}x on a {load} run. The \
                         hard bound of {hard}x holds, so the test passes."
                    ),
                );
            }
            return ratio;
        }
        assert!(
            attempt < ATTEMPTS,
            "{what}: {ratio:.2}x{detail} on the last of {ATTEMPTS} attempts, a {load} run, is \
             under the hard bound of {hard}x. {why}"
        );
        attempt += 1;
    }
}

/// A fixed scalar loop, timed before and after every attempt, in nanoseconds per run of the loop.
/// It does the same work every time, so its two readings differ only when the machine does:
/// another process on the core, or a clock that dropped under heat. The fastest of five batches.
fn yardstick() -> f64 {
    use std::hint::black_box;
    use std::time::{Duration, Instant};

    const STEPS: u32 = 256;
    const RUNS: u32 = 2_000;
    let mut state = 0x9e37_79b9_7f4a_7c15_u64;
    let mut best = Duration::MAX;
    for _ in 0..5 {
        let at = Instant::now();
        for _ in 0..RUNS {
            for _ in 0..STEPS {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                state = black_box(state);
            }
        }
        best = best.min(at.elapsed());
    }
    best.as_secs_f64() * 1e9 / f64::from(RUNS)
}

/// Reports a soft-bound miss where a reader of the run sees it, although the test passes.
///
/// libtest holds back what a passing test prints through `println!`, but not what is written to
/// the standard output handle directly, which reaches the console and the CI log. On GitHub
/// Actions the line is a `::warning` workflow command, which lists it among the run's
/// annotations, and a line is appended to the step summary too. Elsewhere it is a plain line.
fn warn(what: &str, message: &str) {
    use std::io::Write as _;

    let line = if std::env::var_os("GITHUB_ACTIONS").is_some() {
        let title = format!("perf guard: {what}");
        format!(
            "::warning title={}::{}",
            workflow_escape(&title, true),
            workflow_escape(message, false)
        )
    } else {
        format!("warning: perf guard: {what}: {message}")
    };
    // Its own line: libtest may have printed a test's name without a newline. A warning that
    // cannot be written must not fail a guard that passed.
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "\n{line}").and_then(|()| out.flush());
    if let Some(summary) = std::env::var_os("GITHUB_STEP_SUMMARY") {
        let _ = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(summary)
            .and_then(|mut file| writeln!(file, "- **perf guard: {what}**: {message}"));
    }
}

/// `text` escaped for a GitHub Actions workflow command: its message, or one of its `key=value`
/// properties, which also escape `:` and `,`.
fn workflow_escape(text: &str, property: bool) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '%' => escaped.push_str("%25"),
            '\r' => escaped.push_str("%0D"),
            '\n' => escaped.push_str("%0A"),
            ':' if property => escaped.push_str("%3A"),
            ',' if property => escaped.push_str("%2C"),
            _ => escaped.push(c),
        }
    }
    escaped
}
