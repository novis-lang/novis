//! Narrowing a local through `!= null` or `is` — `nvs_types::locals`'
//! narrowing section owns the rule, this is what holds it.
//!
//! The refusals here are as load-bearing as the acceptances: `nvs-ir` turns a
//! narrowed receiver into an *unchecked* untag, so every one of these is a
//! case where the walk must **not** claim to have proved something.

mod common;

use common::check_src;
use nvs_diagnostics::{Diagnostics, code};

/// Wraps `body` in a method taking a `?Node` — the shape most of these cases
/// are written over, not the only one that narrows (see
/// [`a_nullable_scalar_and_a_nullable_array_both_narrow`]).
fn check_with_node(body: &str) -> Diagnostics {
    check_src(&format!(
        "<?nvs\nclass Node {{\n  function label(): string {{ return \"n\"; }}\n}}\n\
         class T {{\n  function m(?Node $n): void {{\n{body}\n  }}\n}}\n"
    ))
}

/// Wraps `body` in a method taking an erased `object`, so that *failing* to
/// narrow is observable: a method call on a plain `object` names no member and
/// is refused where it is written, while the same call on a narrowed receiver
/// resolves. A `mixed` subject would defer both to run time (`rule:types/erased-member-access`) and
/// assert nothing either way.
fn check_with_erased_object(decls: &str, body: &str) -> Diagnostics {
    check_src(&format!(
        "<?nvs\n{decls}\nclass T {{\n  function m(object $v): void {{\n{body}\n  }}\n}}\n"
    ))
}

/// The direction a program written against an abstraction actually takes: the
/// subject is proved to implement the interface, so its members resolve.
#[test]
fn an_is_test_narrows_its_subject_to_an_interface() {
    let diags = check_with_erased_object(
        "interface Labelled {\n  function label(): string;\n}",
        "    if ($v is Labelled) {\n      echo $v->label();\n    }",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// The same shape with no test at all, so the acceptance above is read as
/// *narrowing* rather than as "an `object` receiver resolves anything".
#[test]
fn the_same_call_without_the_test_is_still_refused() {
    let diags = check_with_erased_object(
        "interface Labelled {\n  function label(): string;\n}",
        "    echo $v->label();",
    );
    assert!(diags.has_errors(), "{diags:?}");
}

/// The one name still left out, and why: `rule:iteration/concrete-generic-implements`'s iteration interfaces
/// are written with a type argument everywhere they are declared, and
/// `is Iterator` supplies none — so narrowing to a bare `Iterator`
/// would name a type no annotation does. `nvs_types::locals`' narrowing
/// section owns the rule.
#[test]
fn an_interface_taking_type_arguments_narrows_nothing() {
    let diags = check_with_erased_object(
        "",
        "    if ($v is Iterator) {\n      echo $v->current() as string;\n    }",
    );
    assert!(diags.has_errors(), "{diags:?}");
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

/// The same body with no test at all is still `rule:expressions/nullable-conversion`'s refusal — the
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

/// `rule:expressions/one-equality-operator` left one spelling, and `nvs_types::locals`' `null_test` reads
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
        "<?nvs\nclass Node {\n  function label(): string { return \"n\"; }\n}\n\
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

/// Every `?T` narrows to whatever dropping `null` leaves — a scalar and an
/// `array<T>` alike, not only the single class this once restricted itself
/// to. [`nvs_types::locals`]'s `narrow` owns what lifted that restriction, and
/// `nvs_types::expr_table::ExprInfo::NarrowedRead` is the half below it.
#[test]
fn a_nullable_scalar_and_a_nullable_array_both_narrow() {
    let diags = check_src(
        "<?nvs\nclass T {\n  function m(?int $v, ?array<string> $rows): void {\n    \
         if ($v !=null) {\n      int $w = $v;\n      echo $w;\n    }\n    \
         if ($rows !=null) {\n      string $first = $rows[\"0\"];\n      echo $first;\n    \
         }\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// `rule:types/unions-and-mixed`'s first narrowing form, over the shape every other case here
/// is written on: the class the test names has no `null` in it, so proving it
/// removes `E_NULLABLE_RECEIVER` exactly as `!= null` does.
#[test]
fn an_is_test_over_a_written_class_narrows_its_subject() {
    let diags = check_with_node("if ($n is Node) {\n  echo $n->label();\n}\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

/// The asymmetry with `null_test`: `$n is Node` being *false* leaves
/// every other thing the declared type can hold, `null` among them, so the
/// `else` branch has proved nothing to install.
#[test]
fn an_is_test_proves_nothing_on_its_false_edge() {
    let diags =
        check_with_node("if ($n is Node) {\n  echo \"yes\";\n} else {\n  echo $n->label();\n}\n");
    assert!(refuses_nullable_receiver(&diags), "{diags:?}");
}

/// A `!` inverts which edge proves the class rather than removing it, which is
/// the guard clause a ported program writes.
#[test]
fn an_is_guard_clause_narrows_the_rest_of_the_block() {
    let diags = check_with_node("if (!($n is Node)) {\n  return;\n}\necho $n->label();\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

/// A write drops a class narrowing exactly as it drops a `!= null`
/// one — [`LocalScope::overwrite`] does not care which test installed it.
#[test]
fn a_write_inside_an_is_narrowed_block_widens_it_again() {
    let diags = check_with_node("if ($n is Node) {\n  $n = null;\n  echo $n->label();\n}\n");
    assert!(refuses_nullable_receiver(&diags), "{diags:?}");
}

/// An interface residue drops `null` like any other, which is the half of
/// item 47 a `?T` subject sees: the test proves an object, and `null` is not
/// one. The residue is `Labelled` rather than `Node` — a nominal type either
/// way, so nothing downstream can tell which of the two installed it.
#[test]
fn an_is_test_against_an_interface_drops_null_too() {
    let diags = check_src(
        "<?nvs\ninterface Labelled {\n  function label(): string;\n}\n\
         class Node implements Labelled {\n  function label(): string { return \"n\"; }\n}\n\
         class T {\n  function m(?Node $n): void {\n    \
         if ($n is Labelled) {\n      echo $n->label();\n    }\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// The value arm proves the reference's base on the true edge
/// (`rule:types/class-reference-sites`), which is the same narrowing a written
/// class name reaches — asserted through a member only `Node` declares, so a
/// test that narrowed to nothing fails to compile rather than passing quietly.
#[test]
fn an_is_test_narrows_its_subject_on_the_true_edge() {
    let diags = check_src(
        "<?nvs\ninterface Labelled {\n  function label(): string;\n}\n\
         class Node implements Labelled {\n  function label(): string { return \"n\"; }\n}\n\
         class T {\n  function m(?Node $n, class<Node> $cls): void {\n    \
         if ($n is $cls) {\n      echo $n->label();\n    }\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// **Both arms agree** about the edge that proves nothing, which is what ADR
/// 0150 § 9 decided and what makes false-edge narrowing a change to every
/// spelling at once rather than to one. Asserted as one case for that reason: a
/// written class name that grew a false edge of its own would still look right
/// beside the value arm.
#[test]
fn an_is_test_does_not_narrow_the_false_edge_in_either_arm() {
    let written =
        check_with_node("if ($n is Node) {\n  echo \"yes\";\n} else {\n  echo $n->label();\n}\n");
    assert!(refuses_nullable_receiver(&written), "{written:?}");
    let reference = check_src(
        "<?nvs\nclass Node {\n  function label(): string { return \"n\"; }\n}\n\
         class T {\n  function m(?Node $n, class<Node> $cls): void {\n    \
         if ($n is $cls) {\n      echo \"yes\";\n    } else {\n      echo $n->label();\n    }\n  \
         }\n}\n",
    );
    assert!(refuses_nullable_receiver(&reference), "{reference:?}");
}

/// `rule:types/narrowing`'s closing rule: the narrowing described the value
/// that was there, not the slot, so a write inside the block widens the binding
/// again — `LocalScope::overwrite` does not care which of the five tests
/// installed one.
#[test]
fn a_write_inside_the_narrowed_block_widens_the_binding_again() {
    let diags = check_with_node("if ($n is Node) {\n  $n = null;\n  echo $n->label();\n}\n");
    assert!(refuses_nullable_receiver(&diags), "{diags:?}");
}

/// Narrowing changes what the checker knows on one path and never what the
/// binding was declared as (`rule:types/narrowing`), which shows on both sides
/// of the block: the write of a `null` inside it is checked against `?Node` and
/// accepted, and the read after it is nullable again.
#[test]
fn narrowing_never_changes_the_declared_type_of_the_binding() {
    let write_inside = check_with_node("if ($n is Node) {\n  $n = null;\n}\n");
    assert!(!write_inside.has_errors(), "{write_inside:?}");
    let read_after =
        check_with_node("if ($n is Node) {\n  echo $n->label();\n}\necho $n->label();\n");
    assert!(refuses_nullable_receiver(&read_after), "{read_after:?}");
}

/// Wraps `body` in a method taking a `"read"|"write"`, the shape `rule:types/single-value-types`'s
/// guard row is about, and declares a `"read"`-typed local it can only be
/// assigned to where the comparison narrowed it.
fn check_with_mode(body: &str) -> Diagnostics {
    check_src(&format!(
        "<?nvs\nclass T {{\n  function m(\"read\"|\"write\" $mode): void {{\n{body}\n  }}\n}}\n"
    ))
}

fn refuses_mismatch(diags: &Diagnostics) -> bool {
    diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH))
}

/// `rule:types/single-value-types`'s guard row, and the `!=` spelling of the same edge.
#[test]
fn a_comparison_against_a_literal_narrows_its_subject() {
    let eq = check_with_mode("if ($mode == \"read\") {\n  \"read\" $only = $mode;\n}\n");
    assert!(!eq.has_errors(), "{eq:?}");
    let ne = check_with_mode("if ($mode != \"read\") {\n  return;\n}\n\"read\" $only = $mode;\n");
    assert!(!ne.has_errors(), "{ne:?}");
}

/// The edge the comparison does *not* prove keeps the declared union — the
/// asymmetry `!= null` does not have, because a literal names one value out of
/// several rather than the one value the other edge is.
#[test]
fn a_comparison_proves_nothing_on_the_edge_it_does_not_hold() {
    let diags = check_with_mode(
        "if ($mode == \"read\") {\n  echo \"r\";\n} else {\n  \"read\" $only = $mode;\n}\n",
    );
    assert!(refuses_mismatch(&diags), "{diags:?}");
}

/// The residue must be a subtype of what the local was declared: this is a
/// guard reaching one member of a union, never a re-declaration.
#[test]
fn a_comparison_the_declared_type_does_not_admit_narrows_nothing() {
    let diags = check_src(
        "<?nvs\nclass T {\n  function m(string $s): void {\n    \
         if ($s == \"read\") {\n      1 $one = $s;\n    }\n  }\n}\n",
    );
    assert!(refuses_mismatch(&diags), "{diags:?}");
}

/// A write drops it, exactly as it drops the other two tests' narrowings.
#[test]
fn a_write_inside_a_literal_narrowed_block_widens_it_again() {
    let diags = check_with_mode(
        "if ($mode == \"read\") {\n  $mode = \"write\";\n  \"read\" $only = $mode;\n}\n",
    );
    assert!(refuses_mismatch(&diags), "{diags:?}");
}

/// A `foreach` binding is a write like any other, so reusing the narrowed
/// name as one drops what the test proved.
#[test]
fn a_foreach_binding_over_the_narrowed_name_widens_it() {
    let diags = check_src(
        "<?nvs\nclass Node {\n  function label(): string { return \"n\"; }\n}\n\
         class T {\n  function m(?Node $n, array<?Node> $all): void {\n    \
         if ($n == null) {\n      return;\n    }\n    \
         foreach ($all as ?Node $n) {\n      echo \"x\";\n    }\n    \
         echo $n->label();\n  }\n}\n",
    );
    assert!(refuses_nullable_receiver(&diags), "{diags:?}");
}
