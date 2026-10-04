//! `rule:types/anonymous-function`'s anonymous functions: what a body captures, and what its declared return type has to be.
//!
//! Moved out of `nvs_types::check`'s inline `mod tests`; every test keeps its
//! own name and body. See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use nvs_diagnostics::code;

/// `rule:types/implicit-capture`: "captures exactly the outer variables its body reads."
/// The second local is in scope and never named, so it is not captured —
/// which is the whole difference between this rule and snapshotting the
/// enclosing frame.
#[test]
fn an_anon_fn_captures_exactly_the_outer_names_its_body_reads() {
    let captures =
        captures_of("<?nvs\nint $a = 1;\nint $b = 2;\nvar $f = fn(int $n): int => $n + $a;\n");
    assert_eq!(captures, vec!["a".to_owned()]);
}

/// A parameter shadows an outer local of the same name (`rule:types/declaration`'s
/// declare-once rule is per body), so nothing is captured at all.
#[test]
fn an_anon_fn_parameter_shadows_an_outer_local_rather_than_capturing_it() {
    let captures =
        captures_of("<?nvs\nint $n = 1;\nvar $f = fn(int $n): int => $n + 1;\necho $n;\n");
    assert!(captures.is_empty(), "{captures:?}");
}

/// `rule:statements/an-anonymous-function-captures-this-only-where-it-uses-it`'s "an anonymous function captures `$this` only where the body uses it"
/// falls out of § 2's capture rule with no code of its own: `$this` is an
/// ordinary name in the enclosing scope.
#[test]
fn an_anon_fn_that_names_this_captures_it_like_any_other_binding() {
    let captures = captures_of(
        "<?nvs\nclass T {\n  public int $v = 1;\n  function m(): void {\n    \
         var $f = fn(): int => $this->v;\n  }\n}\n",
    );
    assert_eq!(captures, vec!["this".to_owned()]);
}

/// The body is checked, not skipped — the whole point of the arm: a
/// anonymous function returning the wrong type is a diagnostic like any other.
#[test]
fn an_anon_fn_body_is_checked_against_its_declared_return_type() {
    let diags = check_src("<?nvs\nvar $f = fn(int $n): int => \"nope\";\n");
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

/// A name neither declared inside the anonymous function nor visible outside it is
/// undefined, exactly as in any other body.
#[test]
fn an_undeclared_name_in_an_anon_fn_body_is_diagnosed() {
    let diags = check_src("<?nvs\nvar $f = fn(int $n): int => $n + $nope;\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_UNDEFINED_VARIABLE)),
        "{diags:?}"
    );
}

/// An expression body is its own answer, so it needs no annotation — the
/// half `E0450` deliberately leaves alone.
#[test]
fn an_expression_bodied_anon_fn_needs_no_declared_return_type() {
    let diags = check_src("<?nvs\nvar $f = fn(int $n) => $n + 1;\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

/// A block body would need whole-body return-type inference, which ADR
/// 0007 does not ask the compiler to grow — so it must say what it
/// returns.
#[test]
fn a_block_bodied_anon_fn_without_a_declared_return_type_is_diagnosed() {
    let diags = check_src("<?nvs\nvar $f = fn(int $n) => { return $n + 1; };\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_ANON_FN_RETURN_TYPE_REQUIRED)),
        "{diags:?}"
    );
}

/// `rule:iteration/generators` confines `yield` to the generator's own body — an anonymous
/// function written inside one is not that body.
#[test]
fn a_yield_inside_an_anon_fn_in_a_generator_is_still_refused() {
    let diags = check_src(
        "<?nvs\nclass T {\n  function m(): Iterator<int> {\n    \
         var $f = fn(): int => yield 1;\n    yield 2;\n  }\n}\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_YIELD_OUTSIDE_GENERATOR)),
        "{diags:?}"
    );
}

/// An anonymous function nested in another one captures through it: the outer one
/// has to hold `$a` in order to have it to hand on.
#[test]
fn a_nested_anon_fn_makes_the_enclosing_one_capture_too() {
    let (diags, exprs) =
        check_src_table("<?nvs\nint $a = 1;\nvar $f = fn(): callable => fn(): int => $a;\n");
    assert!(!diags.has_errors(), "{diags:?}");
    let mut sets: Vec<Vec<String>> = exprs
        .anon_fns()
        .map(|(_, captures, _)| captures.iter().map(|(n, _)| n.clone()).collect())
        .collect();
    sets.sort();
    assert_eq!(sets, vec![vec!["a".to_owned()], vec!["a".to_owned()]]);
}
