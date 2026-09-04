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

/// `nvs check <fixture>`'s standard error, with the exit status asserted to be
/// a refusal first.
///
/// `check` rather than `run`: every case in this file is about what the
/// compiler refuses, and a program that does not compile has no run to observe.
fn refusal(fixture: &str) -> String {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "tests", "fixtures", "spawn"]
        .iter()
        .collect::<PathBuf>()
        .join(fixture);
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
