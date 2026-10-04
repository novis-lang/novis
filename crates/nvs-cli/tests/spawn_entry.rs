//! [ADR 0006](/docs/decisions/0006.md) § *Decision*'s
//! operand rule: an isolate's entry is a path or a static method, decided
//! syntactically at the spawn site, and the spellings that are neither are
//! refused where they are written.
//!
//! Through the built binary rather than against `nvs_types` directly, for the
//! reason [`bundle`](bundle) already writes down — `nvs-cli` has no library
//! target — and because the rule this file is about is one the *whole* pipeline
//! owes: the refusals here and the method entry's own run are two halves of one
//! check, and a session that split them across crates would be asserting the
//! diagnostic in the crate that emits it and the behaviour somewhere else.
//! `nvs_types::expr::isolate`'s `check_entry` is the rule's home; what is
//! asserted here is what a person running `nvs` sees.

use std::path::PathBuf;
use std::process::Command;

/// The directory every fixture in this file lives in.
fn fixtures() -> PathBuf {
    [env!("CARGO_MANIFEST_DIR"), "tests", "fixtures", "spawn"]
        .iter()
        .collect()
}

/// `nvs check <fixture>`'s standard error, with the exit status asserted to be
/// a refusal first.
///
/// `check` rather than `run`: every case in this file is about what the
/// compiler refuses, and a program that does not compile has no run to observe.
fn refusal(fixture: &str) -> String {
    let path = fixtures().join(fixture);
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("check")
        .arg(&path)
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(
        !out.status.success(),
        "`{fixture}` is refused, so `nvs check` fails: {stderr}"
    );
    stderr
}

/// The `fn` literal half of the rule, and specifically the *help* — the ADR
/// refuses this spelling because `fn() => …` one keyword away in `spawn worker`
/// captures, so a diagnostic that only said "no" would leave a reader with a
/// working alternative one keyword away and no reason to prefer either.
#[test]
fn an_fn_literal_is_refused_as_a_spawn_target_naming_the_method_form() {
    let stderr = refusal("fn-literal-entry.nvs");
    assert!(
        stderr.contains("E0802"),
        "the entry-form refusal, not a type mismatch: {stderr}"
    );
    assert!(
        stderr.contains("an anonymous function"),
        "the label names what was written: {stderr}"
    );
    assert!(
        stderr.contains("Class::method(...)"),
        "`rule:security/isolate-shares-nothing` requires the diagnostic to name the method form: {stderr}"
    );
}

/// The other half, which is the one a type-directed rule could not tell from
/// the accepted method reference: both are `callable`, and only the written
/// shape says whether the compiler can see the function.
#[test]
fn a_callable_typed_variable_is_refused_as_a_spawn_target() {
    let stderr = refusal("callable-variable-entry.nvs");
    assert!(
        stderr.contains("E0802"),
        "the entry-form refusal, not a type mismatch: {stderr}"
    );
    assert!(
        stderr.contains("this is a `callable`"),
        "the label names the type the operand actually has: {stderr}"
    );
    assert!(
        !stderr.contains("E0401"),
        "the operand position accepts two shapes, so it never reports one \
         expected type: {stderr}"
    );
}

/// `nvs run <fixture>`'s standard output, with the exit status asserted to be a
/// success first.
///
/// `--config` names the grant beside the fixtures rather than letting step 2
/// find whatever `nvs.toml` the test process happens to be standing in
/// (`rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`): `script.spawn` is deny-by-default, so a run without it
/// asserts the denial instead of what the case is about.
fn run(fixture: &str) -> String {
    let dir = fixtures();
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--config")
        .arg(dir.join("spawning.toml"))
        .arg("run")
        .arg(dir.join(fixture))
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    assert!(
        out.status.success(),
        "`{fixture}` runs: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// The entry form the ADR specifies, end to end — one unit, a `static` method
/// of it called in a child, and the child's statics its own.
///
/// The three lines are three claims, and the last two are what make the run an
/// *isolate* rather than a call: the child re-materializes `$runs` from the
/// declared default and answers 1 where the parent had set 40, and the parent
/// still reads 40 afterwards. `Ctx::method_isolate` is what arms the child from
/// the parent's own recipes, there being no second unit to install.
#[test]
fn spawn_script_runs_a_static_method_reference_in_a_fresh_isolate() {
    assert_eq!(run("method-entry.nvs"), "ok\n1\n40\n");
}

/// The other half of the entry form: `args:`'s entries reach the entry's
/// parameters **by name**.
///
/// The fixture's map is written in the opposite order to the signature, which
/// is what makes this a test of the binding rather than of a call: read
/// positionally it would put the `int` in the `string` parameter, and the
/// boundary check `nvs_runtime::call_static_bound` runs would refuse it instead
/// of answering `3+may`.
#[test]
fn spawn_script_binds_args_to_the_entrys_parameters_by_name() {
    assert_eq!(run("method-entry-with-args.nvs"), "ok\nmay+3\n");
}

/// `nvs run <fixture>`'s standard error, with the exit status asserted to be a
/// failure first — the run half of [`refusal`], for a rule that is enforced
/// where the spawn happens rather than where it is compiled.
fn failing_run(fixture: &str) -> String {
    let dir = fixtures();
    let out = Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--config")
        .arg(dir.join("spawning.toml"))
        .arg("run")
        .arg(dir.join(fixture))
        .output()
        .expect("the `nvs` binary this test was built beside runs");
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(
        !out.status.success(),
        "`{fixture}` fails at the spawn: {stderr}"
    );
    stderr
}

/// A map that does not name the entry's parameters is `rule:security/isolate-shares-nothing`'s ordinary
/// named-argument error, and it is raised **at the spawn** — the parent's own
/// frame, where the ADR says a non-literal map's mismatch is reported, so
/// nothing is started and no `ScriptResult` carries it.
///
/// `nvs_stdlib::script`'s `entry_names_agree` is the rule's home; the compile-
/// time half for a literal map is the known gap `nvs_types::expr::isolate`
/// records.
#[test]
fn a_method_entrys_args_map_must_name_its_parameters() {
    let stderr = failing_run("method-entry-args-mismatch.nvs");
    assert!(
        stderr.contains("no entry for parameter(s) `month`"),
        "the message names the parameter nothing bound: {stderr}"
    );
    assert!(
        !stderr.contains("unreachable"),
        "the throw is at the spawn, so the line after it never runs: {stderr}"
    );
}
