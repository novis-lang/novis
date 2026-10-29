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

/// A loop that never writes the name keeps the narrowing made before it, in
/// its condition and in its body.
#[test]
fn a_loop_that_does_not_write_the_name_keeps_an_outer_narrowing() {
    let diags = check_with_node(
        "if ($n == null) {\n  return;\n}\nwhile ($n->label() != \"\") {\n  echo $n->label();\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// The case that first showed the gap: a key and a number, both narrowed by
/// one guard clause, read in a `foreach` that writes neither of them.
#[test]
fn a_property_key_and_a_number_stay_narrowed_through_a_foreach() {
    let diags = check_src(
        "<?nvs\nclass Profile {\n  public string $city = \"London\";\n}\n\
         class Probe {\n  public static function run(?property<Profile> $key, ?int $n, \
         array<string> $rows): void {\n    Profile $profile = new Profile();\n    \
         if ($key == null || $n == null) {\n      return;\n    }\n    \
         foreach ($rows as string $row) {\n      $profile->$key = $row;\n      \
         int $m = $n + 1;\n    }\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// A `Node` class, and a class `T` whose method `m` narrows `$n` and `$k`
/// with a guard clause and then runs `body`. `T::reset` writes its `inout`
/// argument. Returns the source with its diagnostics, so a test can find the
/// read it expects refused.
fn check_after_guard(body: &str) -> (String, Diagnostics) {
    let src = format!(
        "<?nvs\nclass Node {{\n  function label(): string {{ return \"n\"; }}\n}}\n\
         class T {{\n  static function reset(inout ?Node $x): void {{\n    $x = null;\n  }}\n\
         function m(?Node $n, ?int $k, array<?Node> $all, Throwable $e): void {{\n\
         if ($n == null || $k == null) {{\n  return;\n}}\n{body}\n  }}\n}}\n"
    );
    let diags = check_src(&src);
    (src, diags)
}

/// Whether a diagnostic with `code` covers the first occurrence of `read`
/// inside the method body.
fn refused_at(src: &str, diags: &Diagnostics, read: &str, code: nvs_diagnostics::Code) -> bool {
    let body = src
        .find("function m(")
        .expect("the method is in the source");
    let at = u32::try_from(body + src[body..].find(read).expect("the read is in the body"))
        .expect("a fixture fits in u32");
    diags.iter().any(|d| {
        d.code == Some(code)
            && d.primary_span()
                .is_some_and(|s| s.start <= at && at < s.end)
    })
}

/// Every way to write `$n` inside a loop, each one a statement. A
/// destructuring leaf is not one: it declares its variable, so `[?Node $n]`
/// over the declared `$n` is `E0406`.
const WRITES_OF_N: &[&str] = &[
    "$n = null;",
    "$n ??= new Node();",
    "T::reset(inout $n);",
    "foreach ($all as ?Node $n) {\n}",
    "try {\n  echo \"x\";\n} catch (Throwable $caught) {\n  $n = null;\n}",
    "while (true) {\n  $n = null;\n}",
    "for (int $i = 0; $i < 1; $i++) {\n  if ($i == 0) {\n    $n = null;\n  }\n}",
    "var $f = fn() => $n;",
];

/// A write anywhere in a loop drops the narrowing for the whole loop, so a
/// read written *before* the write is refused too: the next pass reaches it
/// after the write.
#[test]
fn every_write_in_a_loop_drops_the_narrowing_before_and_after_it() {
    for write in WRITES_OF_N {
        for body in [
            format!("while (true) {{\n  echo $n->label();\n  {write}\n}}\n"),
            format!("while (true) {{\n  {write}\n  echo $n->label();\n}}\n"),
            format!("foreach ($all as ?Node $other) {{\n  echo $n->label();\n  {write}\n}}\n"),
            format!("do {{\n  echo $n->label();\n  {write}\n}} while (true);\n"),
        ] {
            let (src, diags) = check_after_guard(&body);
            assert!(
                refused_at(&src, &diags, "$n->label()", code::E_NULLABLE_RECEIVER),
                "{body}\n{diags:?}"
            );
        }
    }
}

/// Each write alone in a loop compiles. This is also the check that the scan
/// for a loop's writes covers every write the checker makes: a debug build
/// fails inside the checker when a loop in a program with no error drops a
/// narrowing the scan did not find.
#[test]
fn every_write_alone_in_a_loop_compiles() {
    // `$c` is a plain `int`, so these steps compile in a loop. A step on the
    // narrowed `?int $k` does not (see the next test).
    let more = ["$c++;", "--$c;", "$c += 1;"];
    for write in WRITES_OF_N.iter().chain(&more) {
        for body in [
            format!("int $c = 0;\nwhile (true) {{\n  {write}\n}}\n"),
            format!("int $c = 0;\nforeach ($all as ?Node $other) {{\n  {write}\n}}\n"),
            format!("int $c = 0;\ndo {{\n  {write}\n}} while (false);\n"),
            format!("int $c = 0;\nfor (int $j = 0; $j < 1; $j++) {{\n  {write}\n}}\n"),
        ] {
            let (_, diags) = check_after_guard(&body);
            assert!(!diags.has_errors(), "{body}\n{diags:?}");
        }
    }
}

/// `++` and `--` are writes. Inside the loop the operand is the declared
/// `?int` again, so the step itself is refused as well as the read.
#[test]
fn an_increment_in_a_loop_drops_the_narrowing() {
    for write in ["$k++;", "--$k;"] {
        let body = format!("while (true) {{\n  int $m = $k + 1;\n  {write}\n}}\n");
        let (src, diags) = check_after_guard(&body);
        assert!(
            refused_at(&src, &diags, "$k + 1", code::E_ARITHMETIC_HAS_NO_ROW),
            "{body}\n{diags:?}"
        );
    }
}

/// A `for` step and a loop condition run on every pass, so a write in either
/// drops the narrowing in the body.
#[test]
fn a_write_in_a_for_step_or_a_condition_drops_the_narrowing() {
    for body in [
        "for (int $i = 0; $i < 3; $n = null) {\n  echo $n->label();\n}\n",
        "for (int $i = 0; ($n = null) == null; $i++) {\n  echo $n->label();\n}\n",
        "while (($n = null) == null) {\n  echo $n->label();\n}\n",
        "do {\n  echo $n->label();\n} while (($n = null) == null);\n",
    ] {
        let (src, diags) = check_after_guard(body);
        assert!(
            refused_at(&src, &diags, "$n->label()", code::E_NULLABLE_RECEIVER),
            "{body}\n{diags:?}"
        );
    }
}

/// The condition of a `while` runs again after the body. A body that writes
/// the name therefore leaves the condition at the declared type too.
#[test]
fn a_loop_condition_does_not_keep_a_narrowing_its_body_writes() {
    let (src, diags) = check_after_guard("while ($n->label() != \"\") {\n  $n = null;\n}\n");
    assert!(
        refused_at(&src, &diags, "$n->label()", code::E_NULLABLE_RECEIVER),
        "{diags:?}"
    );
}

/// A `catch` clause and an expression `catch` arm bind their variable on
/// every pass, so reusing the narrowed name for one drops the narrowing.
#[test]
fn a_catch_binding_in_a_loop_drops_the_narrowing() {
    let control = "try {\n    echo \"x\";\n  } catch (Throwable $other) {\n    echo \"y\";\n  }";
    for write in [
        control,
        "try {\n    echo \"x\";\n  } catch (Throwable $e) {\n    echo \"y\";\n  }",
        "string $s = T::label() catch (Throwable $e) => \"y\";",
    ] {
        let src = format!(
            "<?nvs\nclass T {{\n  static function label(): string {{\n    return \"n\";\n  }}\n  \
             function m(Throwable $e): void {{\n    if (!($e is LogicError)) {{\n      \
             return;\n    }}\n    while (true) {{\n      LogicError $only = $e;\n      \
             {write}\n    }}\n  }}\n}}\n"
        );
        let diags = check_src(&src);
        if write == control {
            assert!(
                !diags.has_errors(),
                "a catch binding another name\n{diags:?}"
            );
            continue;
        }
        assert!(
            refused_at(&src, &diags, "LogicError $only", code::E_TYPE_MISMATCH)
                || refused_at(&src, &diags, "$e;", code::E_TYPE_MISMATCH),
            "{write}\n{diags:?}"
        );
    }
}

/// The same loops with no write of `$n` keep the narrowing, so each refusal
/// above is the write's doing.
#[test]
fn the_same_loops_without_the_write_keep_the_narrowing() {
    for body in [
        "while (true) {\n  echo $n->label();\n  $k = 1;\n}\n",
        "foreach ($all as ?Node $other) {\n  echo $n->label();\n}\n",
        "do {\n  echo $n->label();\n} while (false);\n",
        "for (int $i = 0; $i < 3; $i++) {\n  echo $n->label();\n  int $m = $i + 1;\n}\n",
        "while (true) {\n  echo $n->label();\n  var $f = fn() => 1;\n}\n",
    ] {
        let (_, diags) = check_after_guard(body);
        assert!(!diags.has_errors(), "{body}\n{diags:?}");
    }
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

/// `rule:expressions/one-equality-operator` left one spelling, and
/// `nvs_types::locals`' `null_test` reads it: `!= null`/`== null` is the
/// narrowing test, where while both operator pairs existed it had to be
/// `!==`/`===` to stay clear of PHP's truthy table.
/// `rule:expressions/disjoint-comparison-refused` is what keeps it a question
/// only a *nullable* binding can be asked — the same test against a
/// non-nullable one no longer compiles at all.
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

/// **Both arms agree** about the edge that proves nothing, which is what
/// `rule:types/narrowing` states and what makes false-edge narrowing a change to every
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

/// Wraps `body` in a method taking two `?Node`s, for the conditions that join
/// two tests.
fn check_with_two_nodes(body: &str) -> Diagnostics {
    check_src(&format!(
        "<?nvs\nclass Node {{\n  function label(): string {{ return \"n\"; }}\n}}\n\
         class T {{\n  function m(?Node $a, ?Node $b): void {{\n{body}\n  }}\n}}\n"
    ))
}

/// The block of `if (A && B)` runs only where both hold, so it sees what each
/// of them proves.
#[test]
fn the_block_of_an_and_condition_is_narrowed_by_both_operands() {
    let diags = check_with_two_nodes(
        "if ($a != null && $b != null) {\n  echo $a->label(), $b->label();\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// A guard clause written with `||` narrows the rest of the block by both
/// negations, and a `!` around the `||` makes its block the narrowed edge.
#[test]
fn the_false_edge_of_an_or_condition_is_narrowed_by_both_operands() {
    let guard = check_with_two_nodes(
        "if ($a == null || $b == null) {\n  return;\n}\necho $a->label(), $b->label();\n",
    );
    assert!(!guard.has_errors(), "{guard:?}");
    let negated = check_with_two_nodes(
        "if (!($a == null || ($b == null))) {\n  echo $a->label(), $b->label();\n}\n",
    );
    assert!(!negated.has_errors(), "{negated:?}");
}

/// The other edge of each proves only that one operand failed, which names
/// neither: the `else` of an `&&` and the block of an `||` narrow nothing.
#[test]
fn the_other_edge_of_and_and_or_narrows_nothing() {
    let and_else =
        check_with_two_nodes("if ($a == null && $b == null) {\n  return;\n}\necho $a->label();\n");
    assert!(refuses_nullable_receiver(&and_else), "{and_else:?}");
    let or_block =
        check_with_two_nodes("if ($a != null || $b != null) {\n  echo $a->label();\n}\n");
    assert!(refuses_nullable_receiver(&or_block), "{or_block:?}");
}

/// What the `&&` proves ends with the block, and a write inside it still
/// widens the binding again.
#[test]
fn an_and_narrowing_ends_with_its_block_and_a_write_widens_it() {
    let after = check_with_two_nodes(
        "if ($a != null && $b != null) {\n  echo \"both\";\n}\necho $a->label();\n",
    );
    assert!(refuses_nullable_receiver(&after), "{after:?}");
    let written = check_with_two_nodes(
        "if ($a != null && $b != null) {\n  $a = null;\n  echo $a->label();\n}\n",
    );
    assert!(refuses_nullable_receiver(&written), "{written:?}");
}

/// Wraps `body` in a method taking a `Holder` `$h` and an `Outer` `$o`, whose
/// `?Node` property is `readonly` when `modifier` says so. `Holder` also has a
/// method whose body is `holder_body`, so a test can read through `$this`.
fn check_with_holder(modifier: &str, holder_body: &str, body: &str) -> Diagnostics {
    check_src(&format!(
        "<?nvs\nclass Node {{\n  function label(): string {{ return \"n\"; }}\n}}\n\
         class Holder {{\n  public {modifier} ?Node $node;\n\
         \x20 function constructor(?Node $n) {{ $this->node = $n; }}\n\
         \x20 function show(): void {{\n{holder_body}\n  }}\n}}\n\
         class Outer {{\n  public readonly Holder $holder;\n\
         \x20 function constructor(Holder $h) {{ $this->holder = $h; }}\n}}\n\
         class T {{\n  function m(Holder $h, Outer $o): void {{\n{body}\n  }}\n}}\n"
    ))
}

/// A `readonly` property is written once, so a null test narrows it through
/// `$this`, through a variable, and along a path of `readonly` links.
#[test]
fn a_readonly_property_narrows_through_this_and_a_variable() {
    let diags = check_with_holder(
        "readonly",
        "    if ($this->node != null) {\n      echo $this->node->label();\n    }",
        "    if ($h->node != null) {\n      echo $h->node->label();\n    }\n\
         \x20   if ($o->holder->node != null) {\n      echo $o->holder->node->label();\n    }\n\
         \x20   if ($h->node is Node) {\n      echo $h->node->label();\n    }",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// A mutable property may change between the test and the read, so the test
/// narrows nothing and the read is still refused.
#[test]
fn a_mutable_property_does_not_narrow() {
    let diags = check_with_holder(
        "",
        "    if ($this->node != null) {\n      echo $this->node->label();\n    }",
        "",
    );
    assert!(refuses_nullable_receiver(&diags), "{diags:?}");
    let through_a_variable = check_with_holder(
        "",
        "",
        "    if ($h->node != null) {\n      echo $h->node->label();\n    }",
    );
    assert!(
        refuses_nullable_receiver(&through_a_variable),
        "{through_a_variable:?}"
    );
}

/// A write to the root variable makes the path read a different object, so
/// it drops the narrowing of every path below it.
#[test]
fn a_write_to_the_root_variable_drops_a_property_narrowing() {
    let diags = check_with_holder(
        "readonly",
        "",
        "    if ($h->node != null) {\n      $h = new Holder(null);\n      echo $h->node->label();\n    }",
    );
    assert!(refuses_nullable_receiver(&diags), "{diags:?}");
    let path = check_with_holder(
        "readonly",
        "",
        "    if ($o->holder->node != null) {\n      $o = new Outer(new Holder(null));\n      \
         echo $o->holder->node->label();\n    }",
    );
    assert!(refuses_nullable_receiver(&path), "{path:?}");
}

/// Declares `Box` with `box_members` and `Circle` with `circle_members`, and
/// checks `body` inside a method taking `$s` of type `receiver`. The fixtures
/// of `rule:types/a-member-every-class-of-a-union-declares` are written over it.
fn check_union(box_members: &str, circle_members: &str, receiver: &str, body: &str) -> Diagnostics {
    check_src(&format!(
        "<?nvs\ninterface Shape {{\n  function label(): string;\n}}\n\
         class Box {{\n{box_members}\n}}\nclass Circle {{\n{circle_members}\n}}\n\
         class T {{\n  function m({receiver} $s): void {{\n{body}\n  }}\n}}\n"
    ))
}

/// The codes `diags` reports, in order, so a fixture can say there is one.
fn codes(diags: &Diagnostics) -> Vec<&'static str> {
    diags
        .iter()
        .filter_map(|d| d.code.map(|c| c.as_str()))
        .collect()
}

const SHARED: &str = "  public string $name = \"x\";\n  function label(): string { return \"x\"; }";

#[test]
fn a_member_every_class_of_a_union_declares_is_read_written_and_called() {
    let body = "    echo $s->name;\n    $s->name = \"y\";\n    echo $s->label();";
    let diags = check_union(SHARED, SHARED, "Box|Circle", body);
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_nullable_union_is_read_with_the_nullsafe_operator() {
    let body = "    echo $s?->name ?? \"none\";\n    echo $s?->label() ?? \"none\";";
    let diags = check_union(SHARED, SHARED, "?Box|Circle", body);
    assert!(!diags.has_errors(), "{diags:?}");
    let arrow = check_union(SHARED, SHARED, "?Box|Circle", "    echo $s->name;");
    assert_eq!(codes(&arrow), [code::E_NULLABLE_RECEIVER.as_str()]);
}

#[test]
fn a_class_that_lacks_the_member_is_one_error() {
    let read = check_union(
        SHARED,
        "  public int $size = 1;",
        "Box|Circle",
        "    echo $s->name;",
    );
    assert_eq!(codes(&read), [code::E_UNION_MEMBER_NOT_SHARED.as_str()]);
    let call = check_union(
        SHARED,
        "  public int $size = 1;",
        "Box|Circle",
        "    echo $s->label();",
    );
    assert_eq!(codes(&call), [code::E_UNION_MEMBER_NOT_SHARED.as_str()]);
}

#[test]
fn two_types_that_differ_are_one_error() {
    let circle = "  public ?string $name = null;\n  function label(): ?string { return null; }";
    let read = check_union(SHARED, circle, "Box|Circle", "    echo $s->name ?? \"\";");
    assert_eq!(codes(&read), [code::E_UNION_MEMBER_NOT_SHARED.as_str()]);
    let call = check_union(
        SHARED,
        circle,
        "Box|Circle",
        "    echo $s->label() ?? \"\";",
    );
    assert_eq!(codes(&call), [code::E_UNION_MEMBER_NOT_SHARED.as_str()]);
}

#[test]
fn a_member_private_in_one_class_is_one_error() {
    let circle =
        "  private string $name = \"x\";\n  private function label(): string { return \"x\"; }";
    let read = check_union(SHARED, circle, "Box|Circle", "    echo $s->name;");
    assert_eq!(codes(&read), [code::E_UNION_MEMBER_NOT_SHARED.as_str()]);
    let call = check_union(SHARED, circle, "Box|Circle", "    echo $s->label();");
    assert_eq!(codes(&call), [code::E_UNION_MEMBER_NOT_SHARED.as_str()]);
}

#[test]
fn a_write_needs_the_property_writable_in_every_class() {
    let circle =
        "  public readonly string $name;\n  function constructor() { $this->name = \"x\"; }";
    let diags = check_union(SHARED, circle, "Box|Circle", "    $s->name = \"y\";");
    assert_eq!(codes(&diags), [code::E_UNION_MEMBER_NOT_SHARED.as_str()]);
    let read = check_union(SHARED, circle, "Box|Circle", "    echo $s->name;");
    assert!(!read.has_errors(), "{read:?}");
}

#[test]
fn an_interface_in_the_union_keeps_its_error() {
    let read = check_union(SHARED, SHARED, "Box|Shape", "    echo $s->name;");
    assert_eq!(codes(&read), [code::E_RECEIVER_HAS_NO_PROPERTIES.as_str()]);
    let call = check_union(SHARED, SHARED, "Box|Shape", "    echo $s->label();");
    assert!(
        !codes(&call).contains(&code::E_UNION_MEMBER_NOT_SHARED.as_str()) && call.has_errors(),
        "{call:?}"
    );
}
