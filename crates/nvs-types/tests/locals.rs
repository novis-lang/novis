//! ADR 0007 § 1's declare-once rule and the definite-assignment analysis over it, including ADR 0037's `var`.
//!
//! Moved out of `nvs_types::check`'s inline `mod tests`; every test keeps its
//! own name and body. See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use nvs_diagnostics::code;

#[test]
fn a_declared_and_assigned_local_reads_fine() {
    let diags = check_in_method("int $n = 1;\n$n = $n + 1;\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn reading_an_undeclared_local_is_diagnosed() {
    let diags = check_in_method("echo $missing;\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_UNDEFINED_VARIABLE))
    );
}

#[test]
fn redeclaring_a_local_is_diagnosed() {
    let diags = check_in_method("int $n = 1;\nint $n = 2;\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_REDECLARED_LOCAL))
    );
}

/// ADR 0037: `var $n = 1;` fixes `$n`'s type to `int`, exactly as if it
/// had been written out — so a later assignment of a different type is
/// the ordinary `E_TYPE_MISMATCH` a typed local would also get.
#[test]
fn var_infers_the_initializers_type_and_fixes_it() {
    let diags = check_in_method("var $n = 1;\n$n = \"x\";\n");
    assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
}

#[test]
fn var_infers_a_class_type_from_new() {
    let diags = check_src(
        "<?nvs\nclass Foo {}\nclass T {\n  function m(): void {\n    var $x = new Foo();\n    $x->missing;\n  }\n}\n",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_UNKNOWN_MEMBER)),
        "$x should be inferred as `Foo`, so `->missing` is unknown: {diags:?}"
    );
}

#[test]
fn redeclaring_a_var_local_is_diagnosed_like_any_other() {
    let diags = check_in_method("var $n = 1;\nvar $n = 2;\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_REDECLARED_LOCAL))
    );
}

/// ADR 0037 § 2: a bare array literal has no target type to synthesize
/// against, so `var` cannot infer one — this is the one initializer
/// shape it refuses rather than silently falling back to `array<mixed>`.
#[test]
fn var_rejects_a_bare_array_literal_initializer() {
    let diags = check_in_method("var $rows = [1, 2, 3];\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_VAR_ARRAY_LITERAL_NEEDS_TYPE)),
        "{diags:?}"
    );
}

#[test]
fn reading_a_variable_assigned_on_only_one_if_branch_is_diagnosed() {
    let diags = check_in_method("bool $flag = true;\nint $n;\nif ($flag) { $n = 1; }\necho $n;\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_UNDEFINED_VARIABLE)),
        "{diags:?}"
    );
}

#[test]
fn reading_a_variable_assigned_on_both_branches_is_fine() {
    let diags = check_in_method(
        "bool $flag = true;\nint $n;\nif ($flag) { $n = 1; } else { $n = 2; }\necho $n;\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_variable_assigned_before_a_loop_reads_fine_after_it() {
    let diags = check_in_method("int $n = 0;\nwhile (false) { $n = 1; }\necho $n;\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

/// `locals::check_stmt`'s `Switch` arm: every case ends in a `break`, and
/// a `default` covers "no case matched" — so `$n` reads fine after it.
#[test]
fn a_switch_with_default_and_a_break_in_every_case_assigns_definitely() {
    let diags = check_in_method(
        "int $n;\nswitch (1) {\ncase 1:\n  $n = 1;\n  break;\ndefault:\n  $n = 2;\n  break;\n}\necho $n;\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// Same shape but with no `default` arm: "no case matched" is a real
/// path that leaves `$n` unassigned, so the read is still diagnosed.
#[test]
fn a_switch_with_no_default_never_assigns_definitely() {
    let diags =
        check_in_method("int $n;\nswitch (1) {\ncase 1:\n  $n = 1;\n  break;\n}\necho $n;\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_UNDEFINED_VARIABLE)),
        "{diags:?}"
    );
}

/// A case that falls through with no `break` contributes nothing on its
/// own, but the case it falls into still counts — so this still assigns
/// definitely on every path (direct jump to either `case`, or fallthrough
/// from `case 1` into `case 2`).
#[test]
fn a_switch_case_falling_through_into_an_assigning_case_still_assigns_definitely() {
    let diags = check_in_method(
        "int $n;\nswitch (1) {\ncase 1:\n  $n = 1;\ncase 2:\n  $n = 2;\n  break;\ndefault:\n  $n = 3;\n  break;\n}\necho $n;\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// `locals::check_stmt`'s `Try` arm: `body` completing and `catch`
/// completing both assign `$n`, so it reads fine afterward.
#[test]
fn a_try_and_its_catch_both_assigning_reads_fine_after() {
    let diags = check_in_method(
        "int $n;\ntry {\n  $n = 1;\n} catch (LogicError $e) {\n  $n = 2;\n}\necho $n;\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// The `catch` doesn't assign `$n`, so it isn't definite on every path —
/// unlike the fully-conservative old behavior, this now depends on
/// what's actually inside `catch`, not just that a `try` was involved.
#[test]
fn a_try_whose_catch_does_not_assign_is_still_diagnosed() {
    let diags =
        check_in_method("int $n;\ntry {\n  $n = 1;\n} catch (LogicError $e) {\n}\necho $n;\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_UNDEFINED_VARIABLE)),
        "{diags:?}"
    );
}

/// The clause's own binding ends with the clause: only the thrown value
/// assigns it, and no path out of the `try` carries one — not even when the
/// clause is the only way the statement finishes normally, which is the shape
/// that used to leak the name past the checker and panic the lowerer.
#[test]
fn a_catch_binding_is_not_live_after_its_clause() {
    let diags = check_in_method(
        "try {\n  throw new LogicError(\"x\");\n} catch (LogicError $e) {\n  echo $e->message;\n}\necho $e->message;\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_UNDEFINED_VARIABLE)),
        "{diags:?}"
    );

    // A name the body already assigned is a reuse of that binding, not a new
    // one, so what was live going into the `try` is still live coming out.
    let diags = check_in_method(
        "LogicError $e = new LogicError(\"x\");\ntry {\n  echo \"ok\";\n} catch (LogicError $e) {\n}\necho $e->message;\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// `finally` always runs, so its assignment carries forward even though
/// neither `body` nor any `catch` touches `$n` at all.
#[test]
fn a_trys_finally_assignment_reads_fine_after_it() {
    let diags =
        check_in_method("int $n;\ntry {\n  echo \"ok\";\n} finally {\n  $n = 1;\n}\necho $n;\n");
    assert!(!diags.has_errors(), "{diags:?}");
}
