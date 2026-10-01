//! `rule:types/declaration`'s declare-once rule and the definite-assignment analysis over it, including `rule:types/var-inference`'s `var`.
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

/// `rule:types/var-inference`: `var $n = 1;` fixes `$n`'s type to `int`, exactly as if it
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

/// `rule:types/var-inference`: a bare array literal has no target type to synthesize
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

/// The codes `check` reported for one method `m` whose parameters are
/// `params` and whose body is `body`.
fn codes_in(params: &str, body: &str) -> Vec<nvs_diagnostics::Code> {
    let src = format!("<?nvs\nclass T {{\n  function m({params}): void {{\n{body}  }}\n}}\n");
    check_src(&src).iter().filter_map(|d| d.code).collect()
}

/// `rule:types/var-inference`: a `var` value binding takes the subject's
/// element type, and that type is fixed like a written one.
#[test]
fn foreach_var_value_takes_the_element_type_and_fixes_it() {
    let reads = codes_in(
        "array<int> $rows",
        "foreach ($rows as var $v) { int $n = $v; }\n",
    );
    assert!(reads.is_empty(), "{reads:?}");
    let retyped = codes_in(
        "array<int> $rows",
        "foreach ($rows as var $v) { $v = 'x'; }\n",
    );
    assert_eq!(retyped, [code::E_TYPE_MISMATCH]);
}

#[test]
fn foreach_var_key_is_a_string() {
    let reads = codes_in(
        "array<int> $rows",
        "foreach ($rows as var $k => int $v) { string $s = $k; }\n",
    );
    assert!(reads.is_empty(), "{reads:?}");
    let as_int = codes_in(
        "array<int> $rows",
        "foreach ($rows as var $k => int $v) { int $n = $k; }\n",
    );
    assert_eq!(as_int, [code::E_TYPE_MISMATCH]);
}

/// `var` is not a way to drop a qualifier: the element type arrives whole.
#[test]
fn foreach_var_keeps_the_element_types_qualifiers() {
    let codes = codes_in(
        "array<tainted string> $rows",
        "foreach ($rows as var $r) { string $plain = $r; }\n",
    );
    assert_eq!(codes, [code::E_TYPE_MISMATCH]);
}

/// What a `var` local over a `mixed` initializer gets.
#[test]
fn foreach_var_over_a_mixed_subject_is_mixed() {
    let accepts = codes_in("mixed $m", "foreach ($m as var $x) { $x = 'a'; $x = 1; }\n");
    assert!(accepts.is_empty(), "{accepts:?}");
    let narrows = codes_in("mixed $m", "foreach ($m as var $x) { string $s = $x; }\n");
    assert_eq!(narrows, [code::E_TYPE_MISMATCH]);
}

/// The local's refusal, at the subject. A call around the literal gives it a
/// target, as it does for `var $x = f([1, 2]);`.
#[test]
fn foreach_var_over_a_bare_array_literal_is_refused() {
    let bare = codes_in("", "foreach ([1, 2] as var $x) { }\n");
    assert_eq!(bare, [code::E_VAR_ARRAY_LITERAL_NEEDS_TYPE]);
    let called = check_src(
        "<?nvs\nclass T {\n  static function same(array<int> $a): array<int> { return $a; }\n  \
         function m(): void {\n    foreach (T::same([1, 2]) as var $x) { int $n = $x; }\n  }\n}\n",
    );
    assert!(!called.has_errors(), "{called:?}");
}

#[test]
fn foreach_var_key_over_a_cursor_is_still_refused() {
    let codes = codes_in(
        "Iterator<int> $it",
        "foreach ($it as var $k => var $v) { }\n",
    );
    assert_eq!(codes, [code::E_FOREACH_KEY_ON_CURSOR]);
}

/// `inout var` binds at the element type itself, so the exact-type obligation
/// holds, and a write of another type is the ordinary mismatch.
#[test]
fn foreach_inout_var_is_the_element_type_exactly() {
    let writes = codes_in(
        "array<int> $rows",
        "foreach ($rows as inout var $v) { $v = $v * 2; }\n",
    );
    assert!(writes.is_empty(), "{writes:?}");
    let wrong = codes_in(
        "array<int> $rows",
        "foreach ($rows as inout var $v) { $v = 'x'; }\n",
    );
    assert_eq!(wrong, [code::E_TYPE_MISMATCH]);
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
