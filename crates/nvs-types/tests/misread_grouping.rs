//! Operator mixes whose grouping a reader misreads
//! (`rule:expressions/misread-grouping-warns`), and the two errors whose help
//! names the parentheses a misread grouping needs.
//!
//! The parser raises the warning and `E0105`'s help, and the checker `E0706`'s,
//! so each is held here through the whole front end a program goes through:
//! what is warned, which edits each diagnostic carries, which of them a
//! fix-all may apply, and every neighbouring shape that stays silent.

mod common;

use common::{check_src, check_src_allowing_parse_errors};
use nvs_diagnostics::{Diagnostic, Diagnostics, code};

/// Wraps `body` in a function whose parameters the expressions read.
fn check_body(body: &str) -> Diagnostics {
    check_src(&format!(
        "<?nvs\nclass T {{\n  public static function f(?int $n, int $i, bool $ok, string $s, ?string $t): void {{\n{body}\n  }}\n}}\n"
    ))
}

/// Every `W1022` the analysis reported, in source order.
fn warnings(diags: &Diagnostics) -> Vec<&Diagnostic> {
    diags
        .iter()
        .filter(|d| d.code == Some(code::W_MISREAD_GROUPING))
        .collect()
}

/// Each edit `diagnostic` carries, as `(replacement, alternative)`.
fn edits(diagnostic: &Diagnostic) -> Vec<(&str, bool)> {
    diagnostic
        .suggestions
        .iter()
        .map(|s| (s.replacement.as_str(), s.alternative))
        .collect()
}

/// The first shape: a `??` whose default is a bare arithmetic expression. The
/// first edit keeps what the code does and may be applied by a fix-all; the
/// second applies `??` to its nearest operand and is a choice.
#[test]
fn a_coalesce_over_bare_arithmetic_warns_with_both_groupings() {
    let diags = check_body("    echo $n ?? 0 + 10;");
    assert!(!diags.has_errors(), "{diags:?}");
    let warned = warnings(&diags);
    assert_eq!(warned.len(), 1, "{diags:?}");
    assert_eq!(
        warned[0].message,
        "`??` is applied last, so its default is the whole `+` expression"
    );
    assert_eq!(
        edits(warned[0]),
        [("(0 + 10)", false), ("($n ?? 0) + 10", true)]
    );
    assert!(
        warned[0].suggestions[0].safe,
        "the first edit changes nothing"
    );
}

/// The nearest operand is found down every covered operator, so a longer
/// default regroups around the operand written next to the `??`. `.` is
/// covered beside the arithmetic operators.
#[test]
fn the_likely_grouping_takes_the_operand_next_to_the_coalesce() {
    let diags = check_body("    echo $n ?? $i * 2 + 1;\n    echo $t ?? \"a\" . \"b\";");
    assert!(!diags.has_errors(), "{diags:?}");
    let warned = warnings(&diags);
    assert_eq!(warned.len(), 2, "{diags:?}");
    assert_eq!(edits(warned[0])[1], ("($n ?? $i) * 2 + 1", true));
    assert_eq!(edits(warned[1])[1], ("($t ?? \"a\") . \"b\"", true));
}

/// The second shape, in all three conditions the list names: a `.`, an
/// arithmetic operator and a `??`, in the long `?:` form and the short one.
#[test]
fn a_ternary_over_a_bare_condition_warns_with_both_groupings() {
    let diags = check_body(
        "    echo \"n=\" . $ok ? \"x\" : \"y\";\n\
         \x20   echo $n ?? $ok ? \"a\" : \"b\";\n\
         \x20   echo \"n=\" . $s ?: \"none\";\n\
         \x20   echo $i + 1 ? \"p\" : \"q\";",
    );
    assert!(!diags.has_errors(), "{diags:?}");
    let warned = warnings(&diags);
    let found: Vec<(&str, Vec<(&str, bool)>)> = warned
        .iter()
        .map(|d| (d.message.as_str(), edits(d)))
        .collect();
    assert_eq!(
        found,
        [
            (
                "the `?` tests the whole `.` expression",
                vec![
                    ("(\"n=\" . $ok)", false),
                    ("\"n=\" . ($ok ? \"x\" : \"y\")", true)
                ]
            ),
            (
                "the `?` tests the whole `??` expression",
                vec![
                    ("($n ?? $ok)", false),
                    ("$n ?? ($ok ? \"a\" : \"b\")", true)
                ]
            ),
            (
                "the `?:` tests the whole `.` expression",
                vec![
                    ("(\"n=\" . $s)", false),
                    ("\"n=\" . ($s ?: \"none\")", true)
                ]
            ),
            (
                "the `?` tests the whole `+` expression",
                vec![("($i + 1)", false), ("$i + (1 ? \"p\" : \"q\")", true)]
            ),
        ]
    );
}

/// The list is closed. Parentheses settle every covered shape, and the shapes
/// the rule leaves out stay silent: `&&` beside `||`, `-2 ** 2`, a nested
/// ternary, a chain of `??`, a negative default and a plain condition.
#[test]
fn parentheses_and_every_shape_off_the_list_stay_silent() {
    let diags = check_body(
        "    echo $n ?? (0 + 10);\n\
         \x20   echo (\"n=\" . $ok) ? \"x\" : \"y\";\n\
         \x20   echo ($n ?? 1) ? \"a\" : \"b\";\n\
         \x20   echo \"n=\" . ($ok ? \"x\" : \"y\");\n\
         \x20   echo $ok && $ok || $ok ? 1 : 2;\n\
         \x20   echo -2 ** 2;\n\
         \x20   echo $ok ? 1 : ($ok ? 2 : 3);\n\
         \x20   echo $n ?? $n ?? 3;\n\
         \x20   echo $n ?? -1;\n\
         \x20   echo $n == 1 ? \"one\" : \"other\";\n\
         \x20   echo $n ?: 0 + 1;",
    );
    assert!(warnings(&diags).is_empty(), "{diags:?}");
}

/// `E0105` on a binary target whose rightmost operand can be assigned to: the
/// help names the parentheses PHP adds, the fix writes them, and the parse is
/// recovered as that reading, so no second error follows about the type of a
/// target nobody meant.
#[test]
fn an_assignment_inside_a_condition_names_its_parentheses_once() {
    let diags = check_src_allowing_parse_errors(
        "<?nvs\nclass T {\n  public static function f(string $x, ?string $bar): void {\n\
         \x20   ?string $foo = null;\n\
         \x20   if (\"test\" == $x && $foo = $bar ?? null) { echo $foo; }\n  }\n}\n",
    );
    let errors: Vec<&Diagnostic> = diags.iter().filter(|d| d.is_error()).collect();
    assert_eq!(errors.len(), 1, "one error for one mistake: {diags:?}");
    assert_eq!(errors[0].code, Some(code::E_INVALID_ASSIGN_TARGET));
    assert!(
        errors[0]
            .notes
            .iter()
            .any(|n| n.contains("`$a && ($b = 1)`")),
        "{:?}",
        errors[0].notes
    );
    assert_eq!(
        edits(errors[0]),
        [("\"test\" == $x && ($foo = $bar ?? null)", false)]
    );
}

/// A target that is not a binary expression keeps the plain `E0105`.
#[test]
fn an_assignment_to_a_call_keeps_the_plain_error() {
    let diags = check_src_allowing_parse_errors(
        "<?nvs\nclass T {\n  public static function g(): int { return 1; }\n\
         \x20 public static function f(): void { T::g() = 2; }\n}\n",
    );
    let error = diags
        .iter()
        .find(|d| d.code == Some(code::E_INVALID_ASSIGN_TARGET))
        .expect("the call is refused as a target");
    assert!(
        error.notes.is_empty() && error.suggestions.is_empty(),
        "{error:?}"
    );
}

/// `E0706` over a bare comparison: `$flags & 4 == 4` is `$flags & (4 == 4)`,
/// and the help and the fix name the grouping that compiles. The same error
/// over a `bool` that is not a comparison keeps its general help.
#[test]
fn a_bitwise_operator_over_a_bare_comparison_names_its_parentheses() {
    let diags = check_src_allowing_parse_errors(
        "<?nvs\nclass T {\n  public static function f(int $flags, bool $b): void {\n\
         \x20   if ($flags & 4 == 4) { echo 1; }\n\
         \x20   if ($flags == 2 | $flags) { echo 2; }\n\
         \x20   echo $flags & $b;\n  }\n}\n",
    );
    let refused: Vec<&Diagnostic> = diags
        .iter()
        .filter(|d| d.code == Some(code::E_BITWISE_NOT_INTEGER))
        .collect();
    assert_eq!(refused.len(), 3, "{diags:?}");
    assert_eq!(edits(refused[0]), [("($flags & 4) == 4", false)]);
    assert!(refused[0].notes[0].contains("a comparison is applied before"));
    assert_eq!(edits(refused[1]), [("$flags == (2 | $flags)", false)]);
    assert!(refused[2].suggestions.is_empty(), "{:?}", refused[2]);
    assert!(refused[2].notes[0].contains("`$x as int`"));
}
