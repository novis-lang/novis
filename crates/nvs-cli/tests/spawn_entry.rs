//! [ADR 0006](/docs/adr/0006-isolated-script-execution.md) § *Decision*'s
//! operand rule: an isolate's entry is a path or a static method, decided
//! syntactically at the spawn site, and the two spellings that are neither are
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
        stderr.contains("an `fn` literal"),
        "the label names what was written: {stderr}"
    );
    assert!(
        stderr.contains("Class::method(...)"),
        "ADR 0006 requires the diagnostic to name the method form: {stderr}"
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
/// (ADR 0103 § 1): `script.spawn` is deny-by-default, so a run without it
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

/// The half of the method form still unlowered, refused where it is written.
///
/// This case dies with `E0804`: it exists because the child calls the entry
/// with no arguments, and `nvs_runtime::abi` requires exactly the callee's
/// arity — so accepting it would be a slot nobody filled rather than a wrong
/// answer. The slice that binds `args:` by name deletes the code, the fixture
/// and this test together.
#[test]
fn a_method_entry_declaring_a_parameter_is_refused_until_args_bind() {
    let stderr = refusal("method-entry-with-parameters.nvs");
    assert!(
        stderr.contains("E0804"),
        "the unbound-parameter refusal, not the entry-form one: {stderr}"
    );
    assert!(
        stderr.contains("Core\\Script::args()"),
        "the help names the way through for an entry that needs its map: {stderr}"
    );
}
