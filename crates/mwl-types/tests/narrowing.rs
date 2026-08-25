//! Narrowing a nullable local through `!== null` — `mwl_types::locals`'
//! narrowing section owns the rule, this is what holds it.
//!
//! The refusals here are as load-bearing as the acceptances: `mwl-ir` turns a
//! narrowed receiver into an *unchecked* untag, so every one of these is a
//! case where the walk must **not** claim to have proved something.

mod common;

use common::check_src;
use mwl_diagnostics::{Diagnostics, code};

/// Wraps `body` in a method taking a `?Node`, the one shape narrowing
/// applies to today (a `null`-and-one-class union).
fn check_with_node(body: &str) -> Diagnostics {
    check_src(&format!(
        "<?mwl\nclass Node {{\n  function label(): string {{ return \"n\"; }}\n}}\n\
         class T {{\n  function m(?Node $n): void {{\n{body}\n  }}\n}}\n"
    ))
}

fn refuses_nullable_receiver(diags: &Diagnostics) -> bool {
    diags
        .iter()
        .any(|d| d.code == Some(code::E_NULLABLE_RECEIVER))
}

#[test]
fn a_not_null_test_narrows_the_receiver_inside_the_block() {
    let diags = check_with_node("if ($n !=null) {\n  echo $n->label();\n}\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

/// The same body with no test at all is still ADR 0066's refusal — the
/// narrowing is what removes it, not a general relaxation of `->`.
#[test]
fn an_untested_nullable_receiver_is_still_refused() {
    let diags = check_with_node("echo $n->label();\n");
    assert!(refuses_nullable_receiver(&diags), "{diags:?}");
}

#[test]
fn an_is_null_test_narrows_its_else_branch() {
    let diags =
        check_with_node("if ($n == null) {\n  echo \"none\";\n} else {\n  echo $n->label();\n}\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

/// PHP's guard clause: the `if` never falls through, so everything after it
/// is on the branch where the condition was false.
#[test]
fn a_guard_clause_narrows_the_rest_of_the_block() {
    let diags = check_with_node("if ($n == null) {\n  return;\n}\necho $n->label();\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

/// A `while` condition is re-tested before every entry, so what it proves
/// holds for the whole body.
#[test]
fn a_while_condition_narrows_its_own_body() {
    let diags = check_with_node("while ($n !=null) {\n  echo $n->label();\n  $n = null;\n}\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

/// A write is checked against the *declared* type, not the narrowed one —
/// otherwise `$n = null;` inside the block would be a mismatch.
#[test]
fn assigning_null_inside_a_narrowed_block_is_still_legal() {
    let diags = check_with_node("if ($n !=null) {\n  $n = null;\n}\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

/// ...and that same write takes the narrowing back off for everything after
/// it.
#[test]
fn a_write_inside_a_narrowed_block_widens_it_again() {
    let diags = check_with_node("if ($n !=null) {\n  $n = null;\n  echo $n->label();\n}\n");
    assert!(refuses_nullable_receiver(&diags), "{diags:?}");
}

/// A loop's back edge carries a write at the end of the body to a read at
/// its start, and this walk has no back edge to discover that on — so a
/// narrowing established *outside* a loop does not reach inside it.
#[test]
fn a_loop_body_does_not_inherit_an_outer_narrowing() {
    let diags = check_with_node(
        "if ($n == null) {\n  return;\n}\nwhile (true) {\n  echo $n->label();\n}\n",
    );
    assert!(refuses_nullable_receiver(&diags), "{diags:?}");
}

/// The suspended narrowing is still reachable by a *write*, so one inside the
/// body is what the code after the loop sees.
#[test]
fn a_write_inside_a_loop_body_widens_the_local_after_it() {
    let diags = check_with_node(
        "if ($n == null) {\n  return;\n}\nwhile (true) {\n  $n = null;\n}\necho $n->label();\n",
    );
    assert!(refuses_nullable_receiver(&diags), "{diags:?}");
}

/// ADR 0090 § 1 left one spelling, and `mwl_types::locals`' `null_test` reads
/// it: `!= null`/`== null` is the narrowing test, where while both operator
/// pairs existed it had to be `!==`/`===` to stay clear of PHP's truthy table.
/// § 2 is what keeps it a question only a *nullable* binding can be asked —
/// the same test against a non-nullable one no longer compiles at all.
#[test]
fn an_equality_null_test_narrows() {
    let diags =
        check_with_node("if ($n != null) {\n  echo $n->label();\n} else {\n  echo \"none\";\n}\n");
    assert!(!diags.has_errors(), "{diags:?}");
    let refused = check_src(
        "<?mwl\nclass Node {\n  function label(): string { return \"n\"; }\n}\n\
         class T {\n  function m(Node $n): void {\n    if ($n != null) {\n      \
         echo $n->label();\n    }\n  }\n}\n",
    );
    assert!(
        refused
            .iter()
            .any(|d| d.code == Some(code::E_DISJOINT_EQUALITY)),
        "{refused:?}"
    );
}

/// Only a `null`-and-one-class union narrows — see [`mwl_types::locals`]'s
/// `narrow` for why a `?int` narrowed here would trade a clean diagnostic for
/// an `mwl-ir` panic.
#[test]
fn a_nullable_scalar_is_deliberately_not_narrowed() {
    let diags = check_src(
        "<?mwl\nclass T {\n  function m(?int $v): void {\n    \
         if ($v !=null) {\n      int $w = $v;\n      echo $w;\n    }\n  }\n}\n",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

/// A `foreach` binding is a write like any other, so reusing the narrowed
/// name as one drops what the test proved.
#[test]
fn a_foreach_binding_over_the_narrowed_name_widens_it() {
    let diags = check_src(
        "<?mwl\nclass Node {\n  function label(): string { return \"n\"; }\n}\n\
         class T {\n  function m(?Node $n, array<?Node> $all): void {\n    \
         if ($n == null) {\n      return;\n    }\n    \
         foreach ($all as ?Node $n) {\n      echo \"x\";\n    }\n    \
         echo $n->label();\n  }\n}\n",
    );
    assert!(refuses_nullable_receiver(&diags), "{diags:?}");
}
