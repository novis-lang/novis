//! `name: value` and `...$rest` at a call site: which parameter each written
//! argument fills, and every refusal working that out can produce.
//!
//! The rules and their reasons live on `nvs_types::expr::args::map_arguments`;
//! these pin them. The mapping is also what `nvs-ir` reads back as
//! `ResolvedCall::arg_slots`, so a test that a *named* argument's type is
//! checked against the parameter its name reached — not against the one at its
//! own position — is the one that says the mapping happened at all.

mod common;

use common::*;
use nvs_diagnostics::code;

/// `class T` with one method, wrapped so a fixture is one line of interest.
fn with_method(params: &str, body: &str) -> String {
    format!(
        "<?nvs\nclass T {{\n  static function m({params}): void {{}}\n  static function go(): void {{\n    {body}\n  }}\n}}\n"
    )
}

#[test]
fn a_named_argument_fills_the_parameter_its_name_reaches() {
    let diags = with_method("int $count, string $label", "T::m(label: \"x\", count: 1);");
    let diags = check_src(&diags);
    assert!(
        !diags.has_errors(),
        "a named argument is matched by name, so writing them out of order is fine: {diags:?}"
    );
}

#[test]
fn a_named_argument_is_checked_against_its_own_parameter() {
    // Written positionally this list would be well typed. It is the *name*
    // that puts the string at `$count`, so a mismatch here is the mapping
    // itself being asserted.
    let src = with_method("int $count, string $label", "T::m(label: 1, count: \"x\");");
    let diags = check_src(&src);
    assert_eq!(
        diags
            .iter()
            .filter(|d| d.code == Some(code::E_TYPE_MISMATCH))
            .count(),
        2,
        "both arguments are at the wrong parameter, and each is its own mismatch: {diags:?}"
    );
}

#[test]
fn a_named_argument_may_omit_a_defaulted_parameter() {
    let src = with_method("int $count, string $label = \"-\"", "T::m(count: 1);");
    let diags = check_src(&src);
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_required_parameter_no_name_reached_is_an_arity_error() {
    let src = with_method("int $count, string $label", "T::m(count: 1);");
    let diags = check_src(&src);
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_ARITY_MISMATCH)),
        "`$label` is required and nothing filled it: {diags:?}"
    );
}

#[test]
fn a_positional_argument_cannot_follow_a_named_one() {
    let src = with_method("int $count, string $label", "T::m(count: 1, \"x\");");
    let diags = check_src(&src);
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_POSITIONAL_AFTER_NAMED)),
        "{diags:?}"
    );
}

#[test]
fn one_parameter_cannot_be_filled_twice() {
    let src = with_method(
        "int $count, string $label",
        "T::m(1, count: 2, label: \"x\");",
    );
    let diags = check_src(&src);
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_DUPLICATE_ARG)),
        "`$count` is filled positionally and then again by name: {diags:?}"
    );
}

#[test]
fn a_name_no_parameter_carries_is_refused() {
    let src = with_method("int $count", "T::m(counts: 1);");
    let diags = check_src(&src);
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_UNKNOWN_ARG_NAME)),
        "{diags:?}"
    );
}

#[test]
fn a_variadic_tail_cannot_be_filled_by_name() {
    // PHP collects an unmatched name into the variadic as a string key. Novis
    // builds that array out of the arguments written into it, so there is
    // nothing for a name to key.
    let src = with_method("int $count, string ...$rest", "T::m(1, rest: \"x\");");
    let diags = check_src(&src);
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_UNKNOWN_ARG_NAME)),
        "{diags:?}"
    );
}

#[test]
fn a_core_member_is_called_by_the_names_the_spec_writes() {
    // `rule:core-api/shape-rules` R2: `Core\Str::repeat(string $s, uint $times)`, written out of
    // order by name. The names are `nvs_stdlib::registry::CoreMethod::names`
    // and what versions them is `docs/spec/01-core-library.md`'s signature
    // column, so this fixture reads as that column does.
    let diags = check_in_method("string $s = Core\\Str::repeat(times: 2, s: \"x\");\necho $s;\n");
    assert!(!diags.has_errors(), "{diags:?}");
    // And the trailing bag by the one name R2 gives every one of them, which
    // is `nvs_stdlib::registry::OPTIONS_NAME` rather than a per-row spelling.
    let diags = check_in_method(
        "array<int> $a = Core\\Arr::range(1, 5, options: {step: 2});\necho Core\\Arr::count($a);\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_misspelled_name_at_a_core_member_is_unknown() {
    // The refusal a `Core` member now takes: `$s` is the spec's name for
    // `repeat`'s subject, so `subject:` reaches no parameter. Before Stage 0b
    // this was the retired E0485 — "no names at all" — which said nothing
    // about which spelling was wrong.
    let diags = check_in_method("echo Core\\Str::repeat(subject: \"x\", times: 2);\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_UNKNOWN_ARG_NAME)),
        "{diags:?}"
    );
}

#[test]
fn a_skipped_defaulted_core_parameter_takes_the_rows_default() {
    // `Core\Str::padStart(string $s, uint $length, string $padding = " ")`.
    // Named, `$padding` is *skipped* rather than trailing-omitted, so what
    // fills it is `core_lib::defaults_of`'s end-aligned entry; the checker's
    // half of that is that the call is not short an argument.
    let diags =
        check_in_method("string $p = Core\\Str::padStart(length: 4, s: \"x\");\necho $p;\n");
    assert!(!diags.has_errors(), "{diags:?}");
    // The same bound from the other side: a *required* parameter no name
    // reached is named, which is what says the default was the reason above.
    let diags = check_in_method("string $p = Core\\Str::padStart(s: \"x\");\necho $p;\n");
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_ARITY_MISMATCH)),
        "{diags:?}"
    );
}

#[test]
fn a_name_at_a_core_variadic_tail_is_unknown() {
    // `Core\Str::format(string $template, mixed ...$arguments)` is the one
    // registry row with a variadic tail. Its entry in `CoreMethod::names`
    // documents the tail; `rule:core-api/shape-rules` R2 says a name never reaches one, and
    // `MethodSig::param_index` is where that holds for a `Core` row exactly as
    // for a user-declared method.
    let diags = check_in_method(
        "string $s = Core\\Str::format(template: \"%s\", arguments: \"x\");\necho $s;\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_UNKNOWN_ARG_NAME)),
        "{diags:?}"
    );
}

#[test]
fn a_throwable_is_constructed_by_message() {
    // The synthesized constructor is spec § 10's `constructor(string $message,
    // {previous?: Throwable|null})`, and R2 reaches a synthesized member too —
    // `crates/nvs-types/src/error_lib.rs` is where those two names are.
    let diags = check_in_method("throw new LogicError(message: \"x\");");
    assert!(!diags.has_errors(), "{diags:?}");
    let diags = check_in_method(
        "LogicError $e = new LogicError(\"first\");\n\
         throw new LogicError(message: \"second\", options: {previous: $e});",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_spread_argument_fills_a_variadic_tail() {
    let src = with_method(
        "int $count, string ...$rest",
        "array<string> $more = [\"a\"];\n    T::m(1, ...$more);",
    );
    let diags = check_src(&src);
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_spread_argument_is_checked_against_the_tail_element_type() {
    let src = with_method(
        "int $count, string ...$rest",
        "array<int> $more = [1];\n    T::m(1, ...$more);",
    );
    let diags = check_src(&src);
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "an `array<int>` does not spread into a `string ...$rest`: {diags:?}"
    );
}

#[test]
fn a_named_argument_through_a_callable_is_refused() {
    // `rule:types/anonymous-function` gives `callable` no parameter list, so there is no
    // parameter for the name to fill — and nothing below has one either: a
    // callable value records its arity and its parameter tags, never their
    // names. `nvs_types::expr::calls::report_args_with_no_parameter_list`
    // owns the rule, for this callee and for the `mixed` receiver whose
    // callee is chosen when the call runs.
    let src = with_method(
        "int $count",
        "callable $f = fn (int $n): int => $n;\n    $f(n: 1);",
    );
    let diags = check_src(&src);
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_NAMED_ARG_THROUGH_CALLABLE)),
        "{diags:?}"
    );
}

#[test]
fn a_spread_argument_through_a_callable_is_accepted() {
    // The other half of the same rule: how many arguments a `...` hands over
    // is its own run-time length, which needs no parameter list to mean
    // something — so it is left alone here and lowers
    // (`nvs_ir::Helper::CallClosureArray`). What still applies is the order
    // rule, asserted below.
    let src = with_method(
        "int $count",
        "callable $f = fn (int $n): int => $n;\n    array<int> $more = [1];\n    $f(...$more);",
    );
    let diags = check_src(&src);
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_positional_argument_cannot_follow_a_spread_through_a_callable() {
    let src = with_method(
        "int $count",
        "callable $f = fn (int $n): int => $n;\n    array<int> $more = [1];\n    $f(...$more, 2);",
    );
    let diags = check_src(&src);
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_POSITIONAL_AFTER_NAMED)),
        "which parameter a positional argument fills is its own place in the list, \
         with or without a signature: {diags:?}"
    );
}

#[test]
fn a_spread_argument_needs_a_variadic_parameter() {
    let src = with_method("int $count", "array<int> $more = [1];\n    T::m(...$more);");
    let diags = check_src(&src);
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_SPREAD_ARG_NOT_VARIADIC)),
        "{diags:?}"
    );
}

#[test]
fn a_spread_argument_cannot_fill_a_fixed_parameter() {
    // How many entries `$more` holds is a run-time fact, so letting it supply
    // `$count` would leave the call's arity uncheckable.
    let src = with_method(
        "int $count, string ...$rest",
        "array<string> $more = [\"a\"];\n    T::m(...$more);",
    );
    let diags = check_src(&src);
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_SPREAD_ARG_NOT_VARIADIC)),
        "{diags:?}"
    );
}

#[test]
fn a_spread_subject_that_is_not_an_array_names_both_types() {
    // `E_SPREAD_SUBJECT_NOT_AN_ARRAY` is what a position with *no* `array<T>`
    // expectation is left with, and a call's spread always has one — even into
    // a `mixed ...$rest`, whose expectation is `array<mixed>`. So the refusal
    // here is the ordinary mismatch, which names both types.
    // `nvs_types::expr::args::declared_for` owns why.
    let src = with_method(
        "int $count, mixed ...$rest",
        "string $s = \"ab\";\n    T::m(1, ...$s);",
    );
    let diags = check_src(&src);
    let found = diags
        .iter()
        .find(|d| d.code == Some(code::E_TYPE_MISMATCH))
        .unwrap_or_else(|| panic!("{diags:?}"));
    assert!(
        found
            .message
            .contains("expected `array<mixed>`, found `string`"),
        "{found:?}"
    );
}

#[test]
fn a_spread_argument_binds_a_generic_variadic_tail() {
    // `Core\Arr::append(array<T> $a, T ...$more)`: the spread binds `T`
    // through `array<T>` against the subject's own array type, which is the
    // same rule one written-out element at a time.
    let diags = check_src(
        "<?nvs\nclass T {\n  static function go(): void {\n    array<int> $xs = [1];\n    array<int> $more = [2];\n    array<int> $all = Core\\Arr::append($xs, ...$more);\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_generic_variadic_tail_still_rejects_a_mismatched_spread() {
    let diags = check_src(
        "<?nvs\nclass T {\n  static function go(): void {\n    array<int> $xs = [1];\n    array<string> $more = [\"a\"];\n    array<int> $all = Core\\Arr::append($xs, ...$more);\n  }\n}\n",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "`T` is bound to `int` by the subject, so an `array<string>` tail mismatches: {diags:?}"
    );
}

#[test]
fn an_all_positional_call_still_reports_a_count() {
    // The all-positional path is untouched: its arity message is a count
    // against a count, which is what every existing call site already gets.
    let src = with_method("int $count, string $label", "T::m(1);");
    let diags = check_src(&src);
    let found = diags
        .iter()
        .find(|d| d.code == Some(code::E_ARITY_MISMATCH))
        .unwrap_or_else(|| panic!("{diags:?}"));
    assert!(
        found.message.contains("expected 2 argument(s), found 1"),
        "{found:?}"
    );
}
