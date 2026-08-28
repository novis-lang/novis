//! [ADR 0079](../../../docs/adr/0079-testing-is-a-language-feature.md)'s
//! runner: the `#[Test]` table a compile already built (§ 1), constructed and
//! called (§ 20), and reported in § 22's human format.
//!
//! # Why the runner is here
//!
//! It is the one place that can reach both halves at once. The roster is
//! `nvs_types::ExprTypeTable::tests`, which is a *front-end* table, and the
//! instance it constructs is a compiled unit's, reached through a `Ctx` — and
//! `nvs-cli` is the only crate that holds a checked program and a runtime
//! context in one scope. Neither half is discovered at startup: § 1's table
//! was settled while the program was compiled, so there is no directory scan
//! and no reflection here, only a walk of what the compile handed over.
//!
//! # The entry file's own statements do not run
//!
//! `nvs run` calls the script frame; this does not. A `#[Test]` table is
//! discovered at compile time and a program's top-level statements are the
//! *program* — running them would run the thing under test as a side effect of
//! testing it, once per suite and in whatever order the entry point happened
//! to be written. Every file the `require`/`autoload` graph reached is still
//! compiled, because that is what declares the classes and what the checker
//! judged; only the execution of a file body is left out. (A `require`d file's
//! body therefore does not run either, its one caller being the frame that is
//! not called.)
//!
//! # What is owed
//!
//! § 2's isolate-per-test and parallelism are M5, so this runs every test in
//! one process on one `Ctx`; § 20's `retries:`/`FLAKY` section and §§ 8-9's
//! `#[Fixture]` injection are not built, and a constructor that declares
//! parameters is reported as that test failing rather than pretended past
//! (`nvs_runtime::construct_and_call`). Class order is the roster's own sorted
//! order rather than § 20's declaration order: `ExprTypeTable::tests` is keyed
//! by class label and records no sequence, so declaration order across a
//! program's files is a fact only that table can grow. Within a class the
//! order *is* declaration order, which is what a reader of one file sees.

use std::process::ExitCode;
use std::time::Instant;

use nvs_types::defaults::ConstArg;

/// How one test came out.
enum Outcome {
    Passed,
    /// § 20's "a skip states a reason", carrying it.
    Skipped(String),
    /// The message the failure left on the context.
    Failed(String),
    /// The test called `exit(n)`, which ends the whole run.
    Exited(i64),
}

/// Compiles `checked` and runs every `#[Test]` it declares.
pub(crate) fn run(checked: &crate::Checked) -> ExitCode {
    let program = nvs_ir::lower::lower_program(
        crate::SCRIPT,
        &checked.program_files(),
        &checked.exprs,
        &checked.interner,
        &checked.enums,
        &checked.layouts,
    );
    let unit = match nvs_codegen::compile(&program) {
        Ok(unit) => unit,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };

    let mut ctx = nvs_runtime::Ctx::stdout();
    unit.install_in(&mut ctx);

    let (mut passed, mut failed, mut skipped) = (0_usize, 0_usize, 0_usize);
    let mut exited = None;
    let started = Instant::now();
    for class in checked.exprs.test_classes() {
        let cases = checked.exprs.tests(class).unwrap_or_default();
        println!("  {class}");
        for case in cases {
            let began = Instant::now();
            let outcome = run_case(&unit, &mut ctx, class, case);
            let elapsed = began.elapsed();
            match &outcome {
                Outcome::Passed => passed += 1,
                Outcome::Skipped(_) => skipped += 1,
                Outcome::Failed(_) | Outcome::Exited(_) => failed += 1,
            }
            report(&case.method, &outcome, elapsed);
            if let Outcome::Exited(code) = outcome {
                exited = Some(code);
                break;
            }
        }
        if exited.is_some() {
            break;
        }
    }
    // Flushed before the summary: a test's own `echo` is buffered on the
    // context, and printing the counts over the top of it would report a run
    // whose output had not been written yet.
    if let Err(error) = ctx.flush_output() {
        eprintln!("error: could not flush output: {error}");
        return ExitCode::FAILURE;
    }
    println!(
        "\n  {failed} failed, {passed} passed, {skipped} skipped in {:.0} ms",
        started.elapsed().as_secs_f64() * 1000.0
    );

    if let Some(code) = exited {
        // `exit(n)` ended the program the way `nvs run` reports one, and the
        // suite is over whatever the counts say.
        return ExitCode::from(u8::try_from(code & 0xFF).unwrap_or(0));
    }
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// One `#[Test]` method: skipped for its stated reason, or a fresh instance of
/// its class with the method called on it.
fn run_case(
    unit: &nvs_codegen::Unit,
    ctx: &mut nvs_runtime::Ctx,
    class: &str,
    case: &nvs_types::testing::TestCase,
) -> Outcome {
    if let Some(reason) = skip_reason(case) {
        return Outcome::Skipped(reason);
    }
    let Some(outcome) = unit.call_on_new_instance(ctx, class, &case.method) else {
        return Outcome::Failed(format!(
            "internal error: the compiled unit declares no class `{class}`"
        ));
    };
    match outcome {
        Ok(()) => Outcome::Passed,
        Err(status) if status == nvs_runtime::EXITED => Outcome::Exited(ctx.exit_code()),
        Err(_) => Outcome::Failed(ctx.take_thrown().message().to_owned()),
    }
}

/// The `skip:` option's reason, or `None` for a test that runs.
///
/// § 20 makes the reason the whole point of the option — `#[Test(skip: true)]`
/// does not compile — so a `skip` that folded to anything but a string is one
/// `nvs_types::testing` has already refused, and reading it back as absent
/// here would be a second answer to a settled question.
fn skip_reason(case: &nvs_types::testing::TestCase) -> Option<String> {
    case.options.iter().find_map(|(name, value)| match value {
        ConstArg::Str(reason) if name == "skip" => Some(reason.clone()),
        _ => None,
    })
}

/// § 22's one line per test: the mark, the method's own name, and how long it
/// took.
fn report(method: &str, outcome: &Outcome, elapsed: std::time::Duration) {
    let mark = match outcome {
        Outcome::Passed => "\u{2713}",
        Outcome::Skipped(_) => "-",
        Outcome::Failed(_) | Outcome::Exited(_) => "\u{2717}",
    };
    println!(
        "    {mark} {method:<32} {:.1} ms",
        elapsed.as_secs_f64() * 1000.0
    );
    match outcome {
        Outcome::Passed => {}
        Outcome::Skipped(reason) => println!("      skipped: {reason}"),
        Outcome::Failed(message) => println!("      {message}"),
        Outcome::Exited(code) => println!("      the test called exit({code})"),
    }
}
