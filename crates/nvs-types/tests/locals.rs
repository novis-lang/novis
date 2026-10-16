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

/// The codes `check` reported for one method `m` whose parameters are
/// `params` and whose body is `body`.
fn codes_in(params: &str, body: &str) -> Vec<nvs_diagnostics::Code> {
    let src = format!("<?nvs\nclass T {{\n  function m({params}): void {{\n{body}  }}\n}}\n");
    check_src(&src).iter().filter_map(|d| d.code).collect()
}

/// `rule:types/var-inference`: a literal whose elements all have the type `T`
/// is an `array<T>`, fixed like a written one — keys take no part, and an
/// element of another type is the ordinary mismatch a written type gets.
#[test]
fn var_infers_an_array_literal_whose_elements_have_one_type() {
    let reads = codes_in(
        "",
        "var $ids = [1, 2, 3];\narray<int> $same = $ids;\n\
         var $byKey = ['a' => 'x', 'b' => 'y'];\narray<string> $names = $byKey;\n",
    );
    assert!(reads.is_empty(), "{reads:?}");
    let narrower = codes_in("", "var $ids = [1, 2];\narray<string> $s = $ids;\n");
    assert_eq!(narrower, [code::E_TYPE_MISMATCH]);
    let written = codes_in("", "var $ids = [1, 2];\n$ids[] = 'three';\n");
    assert!(!written.is_empty(), "{written:?}");
    assert!(
        !written.contains(&code::E_VAR_ARRAY_LITERAL_NEEDS_TYPE),
        "{written:?}"
    );
}

/// A nested literal is typed first, so the outer one sees one `array<int>`
/// per row and is `array<array<int>>`.
#[test]
fn var_infers_a_nested_array_literal_from_the_inside_out() {
    let codes = codes_in(
        "",
        "var $grid = [[1, 2], [3]];\narray<array<int>> $g = $grid;\n",
    );
    assert!(codes.is_empty(), "{codes:?}");
    let narrower = codes_in("", "var $grid = [[1], [2]];\narray<int> $flat = $grid;\n");
    assert_eq!(narrower, [code::E_TYPE_MISMATCH]);
}

/// A spread gives its source's element type, a spread literal included.
#[test]
fn var_infers_through_a_spread_of_the_same_element_type() {
    let codes = codes_in(
        "array<int> $more",
        "var $all = [...$more, ...[4, 5], 6];\narray<int> $a = $all;\n",
    );
    assert!(codes.is_empty(), "{codes:?}");
}

/// An element's type is what `var $e = <element>;` would give: a literal and
/// an enum case widen to their base, so two different strings or two cases of
/// one enum are one type, and a call gives its declared return type.
#[test]
fn var_gives_each_element_the_type_a_var_local_would() {
    let diags = check_src(
        "<?nvs\nenum Mode { Read, Write }\nclass T {\n  \
         static function one(): int { return 1; }\n  function m(): void {\n    \
         var $names = ['a', 'b'];\n    $names[] = 'c';\n    \
         var $modes = [Mode::Read, Mode::Write];\n    array<Mode> $all = $modes;\n    \
         var $counts = [T::one(), 2];\n    array<int> $ints = $counts;\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// The help lines `var`'s refusal carries, for the diagnostics `check` reported
/// over `var $x = {literal};`.
fn refusal_help(literal: &str) -> Vec<String> {
    let diags = check_in_method(&format!("var $x = {literal};\n"));
    let codes: Vec<_> = diags.iter().filter_map(|d| d.code).collect();
    assert_eq!(codes, [code::E_VAR_ARRAY_LITERAL_NEEDS_TYPE], "{literal}");
    diags.iter().flat_map(|d| d.notes.clone()).collect()
}

/// Two types are refused once, whatever they are, and nothing is widened to a
/// common type: `int` beside `float` is two types too. The help names the
/// union to write.
#[test]
fn var_refuses_an_array_literal_whose_elements_differ_and_names_the_union() {
    for (literal, union) in [
        ("[1, 'a']", "array<int|string> $x = [1, 'a'];"),
        ("[1, 2.5]", "array<int|float> $x = [1, 2.5];"),
        ("[[1], ['a']]", "array<array<int>|array<string>> $x"),
        ("[...[1], 'a']", "array<int|string> $x"),
    ] {
        let help = refusal_help(literal);
        assert!(
            help.iter().any(|h| h.contains(union)),
            "{literal}: {help:?}"
        );
    }
}

/// `null` beside one other type is written as the nullable type.
#[test]
fn var_refuses_an_array_literal_with_null_and_names_the_nullable_type() {
    let help = refusal_help("[1, null]");
    assert!(
        help.iter()
            .any(|h| h.contains("`array<?int> $x = [1, null];`")),
        "{help:?}"
    );
}

/// A qualifier is part of the type, so `var` can neither drop nor invent one.
#[test]
fn var_refuses_elements_that_differ_only_in_a_qualifier() {
    let mixed = codes_in("tainted string $t", "var $x = [$t, 'plain'];\n");
    assert_eq!(mixed, [code::E_VAR_ARRAY_LITERAL_NEEDS_TYPE]);
    let kept = codes_in(
        "tainted string $t, tainted string $u",
        "var $x = [$t, $u];\narray<string> $plain = $x;\n",
    );
    assert_eq!(kept, [code::E_TYPE_MISMATCH]);
}

#[test]
fn var_refuses_an_empty_array_literal_at_any_depth() {
    for literal in ["[]", "[[1], []]", "[...[]]"] {
        let codes = codes_in("", &format!("var $x = {literal};\n"));
        assert_eq!(codes, [code::E_VAR_ARRAY_LITERAL_NEEDS_TYPE], "{literal}");
    }
}

#[test]
fn var_refuses_an_array_literal_of_two_classes_even_when_one_extends_the_other() {
    let diags = check_src(
        "<?nvs\nclass User {}\nclass Admin extends User {}\nclass T {\n  \
         function m(): void {\n    var $people = [new User(), new Admin()];\n  }\n}\n",
    );
    let codes: Vec<_> = diags.iter().filter_map(|d| d.code).collect();
    assert_eq!(codes, [code::E_VAR_ARRAY_LITERAL_NEEDS_TYPE]);
}

/// An element that reported its own error is that one diagnostic, never a
/// second about the literal.
#[test]
fn var_over_a_literal_with_a_broken_element_reports_the_element_alone() {
    let codes = codes_in("", "var $x = [1, $missing];\n");
    assert_eq!(codes, [code::E_UNDEFINED_VARIABLE]);
}

#[test]
fn foreach_var_over_a_one_type_array_literal_subject_infers() {
    let reads = codes_in("", "foreach ([1, 2] as var $n) { int $m = $n; }\n");
    assert!(reads.is_empty(), "{reads:?}");
    let retyped = codes_in("", "foreach ([1, 2] as var $n) { string $s = $n; }\n");
    assert_eq!(retyped, [code::E_TYPE_MISMATCH]);
}

/// The one diagnostic `body` reports, which must be a mismatch, as its label
/// messages and its notes.
fn the_mismatch(body: &str) -> (Vec<String>, Vec<String>) {
    let diags = check_in_method(body);
    let all: Vec<_> = diags.iter().collect();
    let [d] = all.as_slice() else {
        panic!("one diagnostic, not {diags:?}");
    };
    assert_eq!(d.code, Some(code::E_TYPE_MISMATCH), "{d:?}");
    let labels = d.labels.iter().map(|l| l.message.clone()).collect();
    (labels, d.notes.clone())
}

/// A write that does not fit the type `var` took is the mismatch a written type
/// gets, with a label on the `var` line and the declaration that would accept
/// the value — through a subscript at the depth written, and for a scalar too.
#[test]
fn a_write_that_misses_a_var_inferred_element_type_names_the_var_line() {
    let (labels, notes) = the_mismatch("var $prices = [10, 20];\n$prices[] = 12.5;\n");
    assert!(
        labels.contains(&"`var` gave `$prices` the type `array<int>` here".to_owned()),
        "{labels:?}"
    );
    assert!(
        notes
            .iter()
            .any(|n| n.contains("`array<int|float> $prices = [10, 20];`")),
        "{notes:?}"
    );
    let (_, nested) = the_mismatch("var $grid = [[1], [2]];\n$grid[0][] = 1.5;\n");
    assert!(
        nested
            .iter()
            .any(|n| n.contains("`array<array<int|float>> $grid = [[1], [2]];`")),
        "{nested:?}"
    );
    let (labels, scalar) = the_mismatch("var $n = 1;\n$n = 2.5;\n");
    assert!(
        labels.contains(&"`var` gave `$n` the type `int` here".to_owned()),
        "{labels:?}"
    );
    assert!(
        scalar.iter().any(|n| n.contains("`int|float $n = 1;`")),
        "{scalar:?}"
    );
    let (labels, written) = the_mismatch("array<int> $w = [1];\n$w[] = 1.5;\n");
    assert_eq!(labels.len(), 1, "{labels:?}");
    assert!(written.is_empty(), "{written:?}");
}

/// `rule:ide/no-compile-path-calls-the-synthesis`: the synthesis has one
/// caller, `var_array_literal`, and that has two — `var`'s arm and the
/// `foreach` subject. A third caller anywhere in the crate fails here.
#[test]
fn only_var_and_the_foreach_subject_call_the_synthesis() {
    let src = nvs_repo::path("crates/nvs-types/src");
    let mut synthesis = Vec::new();
    let mut wrapper = Vec::new();
    let mut dirs = vec![src];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                dirs.push(path);
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            for _ in text.matches("synthesize_array_literal(") {
                synthesis.push(name.clone());
            }
            for _ in text.matches("var_array_literal(") {
                wrapper.push(name.clone());
            }
        }
    }
    synthesis.sort();
    // The definition, and the one call inside `var_array_literal`.
    assert_eq!(synthesis, ["literals.rs", "locals.rs"]);
    // The definition, and the `var` arm and the `foreach` subject.
    assert_eq!(wrapper, ["locals.rs", "locals.rs", "locals.rs"]);
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

/// The local's refusal, at the subject, for a literal of two types. A call
/// around the literal gives it a target, as it does for `var $x = f([1, 2]);`.
#[test]
fn foreach_var_over_a_bare_array_literal_is_refused() {
    let bare = codes_in("", "foreach ([1, 'two'] as var $x) { }\n");
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
