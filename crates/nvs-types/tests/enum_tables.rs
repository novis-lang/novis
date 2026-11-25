//! The goal `enum-tables`: an enum is a full key.
//!
//! `rule:expressions/a-match-over-an-enum-covers-every-case`: a `match` over an enum `E` or `?E`
//! with no `default` arm names every case, and `null` for `?E`. A missing case is `E1016` at the
//! `match` keyword, and the message lists every missing case.
//!
//! A class constant folds an enum case, a case `as int`, another class's literal constant and
//! `Foo::class`, alone and inside an array at any depth. A read inlines the folded value, and
//! `E0792` is left for another class's constant that is itself an array or a case.
//!
//! Every `match` fixture declares `enum Priority { Low, Medium, High }` and a method `Labels::label` whose
//! body is the `match` under test.

mod common;

use common::check_src;
use nvs_diagnostics::{Diagnostics, code};
use nvs_types::ConstArg;
use nvs_types::expr_table::ExprInfo;

/// `Priority`, a class `Defaults` with `consts`, and `label($p)` returning `body`.
fn fixture(param: &str, consts: &str, body: &str) -> String {
    format!(
        "<?nvs\nenum Priority {{ Low, Medium, High }}\nclass Defaults {{\n{consts}\n}}\n\
         class Labels {{\n  public static function label({param} $p): string {{\n    return {body};\n  }}\n}}\n"
    )
}

fn misses(diags: &Diagnostics) -> Vec<String> {
    diags
        .iter()
        .filter(|d| d.code == Some(code::E_MATCH_MISSES_A_CASE))
        .map(|d| d.message.clone())
        .collect()
}

#[test]
fn a_match_over_an_enum_that_misses_a_case_does_not_compile() {
    let diags = check_src(&fixture(
        "Priority",
        "",
        "match ($p) { Priority::Low => 'low' }",
    ));
    assert_eq!(
        misses(&diags),
        ["this `match` does not handle `Priority::Medium` and `Priority::High`"],
        "{diags:?}"
    );
    let every = check_src(&fixture(
        "Priority",
        "",
        "match ($p) { Priority::Low => 'low', Priority::Medium, Priority::High => 'high' }",
    ));
    assert!(!every.has_errors(), "{every:?}");
}

#[test]
fn a_match_with_a_default_arm_needs_no_other_case() {
    let diags = check_src(&fixture(
        "Priority",
        "",
        "match ($p) { Priority::Low => 'low', default => 'other' }",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_match_over_a_nullable_enum_needs_a_null_arm() {
    let all = "Priority::Low => 'low', Priority::Medium => 'medium', Priority::High => 'high'";
    let diags = check_src(&fixture(
        "?Priority",
        "",
        &format!("match ($p) {{ {all} }}"),
    ));
    assert_eq!(
        misses(&diags),
        ["this `match` does not handle `null`"],
        "{diags:?}"
    );
    let with_null = check_src(&fixture(
        "?Priority",
        "",
        &format!("match ($p) {{ {all}, null => 'none' }}"),
    ));
    assert!(!with_null.has_errors(), "{with_null:?}");
}

#[test]
fn an_arm_that_is_not_a_case_covers_nothing() {
    // The last arm calls a method that returns a case. A call is not a case written in the arm,
    // so the checker does not count it.
    let diags = check_src(&fixture(
        "Priority",
        "  public static function high(): Priority {\n    return Priority::High;\n  }",
        "match ($p) { Priority::Low => 'low', Priority::Medium => 'medium', Defaults::high() => 'high' }",
    ));
    assert_eq!(
        misses(&diags),
        ["this `match` does not handle `Priority::High`"],
        "{diags:?}"
    );
}

#[test]
fn a_class_constant_that_folds_to_a_case_covers_it() {
    let diags = check_src(&fixture(
        "Priority",
        "  const Priority URGENT = Priority::High;",
        "match ($p) { Priority::Low => 'low', Priority::Medium => 'medium', Defaults::URGENT => 'urgent' }",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_match_over_mixed_or_a_union_is_unchanged() {
    let mixed = check_src(&fixture(
        "mixed",
        "",
        "match ($p) { Priority::Low => 'low' }",
    ));
    assert!(misses(&mixed).is_empty(), "{mixed:?}");
    let union = check_src(&fixture(
        "Priority|string",
        "",
        "match ($p) { Priority::Low => 'low' }",
    ));
    assert!(misses(&union).is_empty(), "{union:?}");
    let condition = check_src(&fixture(
        "Priority",
        "",
        "match (true) { $p == Priority::Low => 'low' }",
    ));
    assert!(misses(&condition).is_empty(), "{condition:?}");
}

/// The value a read of `read` inlines, from the `ExprInfo::ClassConst` the checker records at it.
fn folded(src: &str, read: &str) -> (Diagnostics, Option<ConstArg>) {
    let (diags, declared) = common::check_src_declared(src);
    let value = match declared.folded_at(read) {
        Some(ExprInfo::ClassConst { value, .. }) => Some(value.clone()),
        _ => None,
    };
    (diags, value)
}

/// `const` lines in a class `Defaults`, and a class `Reader` whose method body is `body`.
fn tables(consts: &str, body: &str) -> String {
    format!(
        "<?nvs\nenum Priority {{ Low, Medium, High }}\nenum Size: uint {{ Small = 1, Large = 2 }}\n\
         class Limits {{ const int MAX = 10; }}\n\
         class Other {{ const Priority FIRST = Priority::Low; }}\nclass Defaults {{\n{consts}\n}}\n\
         class Reader {{\n  public static function read(): void {{\n{body}\n  }}\n}}\n"
    )
}

fn entries(value: Option<ConstArg>) -> Vec<(String, ConstArg)> {
    match value {
        Some(ConstArg::Array(entries)) => entries,
        other => panic!("not a folded array: {other:?}"),
    }
}

fn key(name: &str, value: ConstArg) -> (String, ConstArg) {
    (name.to_owned(), value)
}

#[test]
fn a_constant_array_holds_enum_cases() {
    let src = tables(
        "  const array<Priority> ORDER = [Priority::High, Priority::Low];\n\
         \x20 const array<array<Priority>> GROUPS = ['now' => [Priority::High]];\n\
         \x20 const array<Size> SIZES = [Size::Large];",
        "    var $o = Defaults::ORDER;\n    var $g = Defaults::GROUPS;\n    var $s = Defaults::SIZES;",
    );
    let (diags, order) = folded(&src, "Defaults::ORDER");
    assert!(!diags.has_errors(), "{diags:?}");
    assert_eq!(
        entries(order),
        [
            key("0", ConstArg::EnumInt(2)),
            key("1", ConstArg::EnumInt(0))
        ]
    );
    let (_, groups) = folded(&src, "Defaults::GROUPS");
    assert_eq!(
        entries(groups),
        [key(
            "now",
            ConstArg::Array(vec![key("0", ConstArg::EnumInt(2))])
        )]
    );
    let (_, sizes) = folded(&src, "Defaults::SIZES");
    assert_eq!(entries(sizes), [key("0", ConstArg::EnumUint(2))]);
}

#[test]
fn a_constant_array_is_keyed_by_a_case_as_int() {
    let src = tables(
        "  const array<string> LABELS = [Priority::Low as int => 'low', Priority::High as int => 'high'];\n\
         \x20 const array<uint> SIZES = [Size::Large as uint];\n\
         \x20 const int TOP = Priority::High as int;",
        "    var $l = Defaults::LABELS;\n    var $s = Defaults::SIZES;\n    var $t = Defaults::TOP;",
    );
    let (diags, labels) = folded(&src, "Defaults::LABELS");
    assert!(!diags.has_errors(), "{diags:?}");
    assert_eq!(
        entries(labels),
        [
            key("0", ConstArg::Str("low".into())),
            key("2", ConstArg::Str("high".into()))
        ]
    );
    let (_, sizes) = folded(&src, "Defaults::SIZES");
    assert_eq!(entries(sizes), [key("0", ConstArg::Uint(2))]);
    let (_, top) = folded(&src, "Defaults::TOP");
    assert_eq!(top, Some(ConstArg::Int(2)));
}

#[test]
fn a_constant_array_holds_another_class_constant_and_a_class_name() {
    let src = tables(
        "  const array<int> LIMITS = [Limits::MAX, 3];\n\
         \x20 const array<string> CLASSES = [Limits::class, Defaults::class];\n\
         \x20 const string SELF_NAME = Defaults::class;",
        "    var $l = Defaults::LIMITS;\n    var $c = Defaults::CLASSES;\n    var $n = Defaults::SELF_NAME;",
    );
    let (diags, limits) = folded(&src, "Defaults::LIMITS");
    assert!(!diags.has_errors(), "{diags:?}");
    assert_eq!(
        entries(limits),
        [key("0", ConstArg::Int(10)), key("1", ConstArg::Int(3))]
    );
    let (_, classes) = folded(&src, "Defaults::CLASSES");
    assert_eq!(
        entries(classes),
        [
            key("0", ConstArg::Str("Limits".into())),
            key("1", ConstArg::Str("Defaults".into()))
        ]
    );
    let (_, name) = folded(&src, "Defaults::SELF_NAME");
    assert_eq!(name, Some(ConstArg::Str("Defaults".into())));
}

#[test]
fn a_property_default_array_holds_enum_cases() {
    let src = tables(
        "  public array<Priority> $order = [Priority::High, Priority::Low];\n\
         \x20 public static array<string> $labels = [Priority::Low as int => 'low'];\n\
         \x20 public array<array<Size>> $sizes = ['big' => [Size::Large]];",
        "    var $d = new Defaults();",
    );
    let diags = check_src(&src);
    assert!(!diags.has_errors(), "{diags:?}");
    // A string inside a table of cases is not a case, so the whole default is refused.
    let wrong = check_src(&tables(
        "  public array<Priority> $order = [Priority::High, 'low'];",
        "    var $d = new Defaults();",
    ));
    let codes: Vec<_> = wrong.iter().filter_map(|d| d.code).collect();
    assert_eq!(codes, [code::E_PROPERTY_DEFAULT_NOT_LITERAL], "{wrong:?}");
}

#[test]
fn a_constant_that_still_cannot_fold_says_what_to_write() {
    // Another class's constant folds only when its value is a number or a string.
    let src = tables(
        "  const Priority START = Other::FIRST;",
        "    var $s = Defaults::START;",
    );
    let diags = check_src(&src);
    let found: Vec<_> = diags
        .iter()
        .filter(|d| d.code == Some(code::E_CLASS_CONST_NO_CONSTANT_FORM))
        .collect();
    assert_eq!(found.len(), 1, "{diags:?}");
    let help = format!("{:?}", found[0]);
    assert!(
        help.contains("an enum case, `Foo::class` or an array of these")
            && help.contains(
                "Another class's constant works only when its value is a number or a string"
            )
            && !help.contains("cannot yet"),
        "{help}"
    );
}

/// `Priority`, `Size`, a class `Defaults` with `consts`, and a function whose body is `body`.
fn keyed(consts: &str, body: &str) -> String {
    format!(
        "<?nvs\nenum Priority {{ Low, Medium, High }}\nenum Size: uint {{ Small = 1, Large = 2 }}\n\
         class Defaults {{\n{consts}\n}}\n\
         class Run {{\n  public static function plain(array<string> $a): int {{ return 0; }}\n\
         \x20 public static function run(mixed $m, int $i, array<string> $strings): void {{\n{body}\n  }}\n}}\n"
    )
}

/// The messages of every diagnostic with `code`.
fn coded(diags: &Diagnostics, code: nvs_diagnostics::Code) -> Vec<String> {
    diags
        .iter()
        .filter(|d| d.code == Some(code))
        .map(|d| format!("{d:?}"))
        .collect()
}

const LABELS: &str =
    "  array<Priority, string> $labels = [Priority::Low => 'low', Priority::High => 'high'];";

#[test]
fn an_array_keyed_by_an_enum_takes_only_its_cases() {
    let diags = check_src(&keyed(
        "",
        &format!(
            "{LABELS}\n  $labels[Priority::Medium] = 'medium';\n  string $low = $labels[Priority::Low];\n\
             \x20 Priority $p = Priority::High;\n  $labels[$p] = 'h';\n  string $high = $labels[$p];"
        ),
    ));
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn an_int_a_string_or_another_enum_key_is_refused() {
    for key in ["0", "$i", "'Low'", "Size::Small", "$m"] {
        let diags = check_src(&keyed(
            "",
            &format!("{LABELS}\n  string $x = $labels[{key}];"),
        ));
        let found = coded(&diags, code::E_ARRAY_KEY_INVALID_TYPE);
        assert_eq!(found.len(), 1, "{key}: {diags:?}");
        assert!(
            found[0].contains("cases of `Priority`") && found[0].contains("as Priority"),
            "{key}: {}",
            found[0]
        );
    }
    let literal = check_src(&keyed(
        "",
        "  array<Priority, string> $l = [Priority::Low => 'a', 1 => 'b'];",
    ));
    assert_eq!(
        coded(&literal, code::E_ARRAY_KEY_INVALID_TYPE).len(),
        1,
        "{literal:?}"
    );
}

#[test]
fn an_append_to_an_enum_keyed_array_is_refused() {
    let append = check_src(&keyed("", &format!("{LABELS}\n  $labels[] = 'x';")));
    assert_eq!(
        coded(&append, code::E_ENUM_KEYED_ARRAY_APPEND).len(),
        1,
        "{append:?}"
    );
    let positional = check_src(&keyed(
        "",
        "  array<Priority, string> $l = [Priority::Low => 'a', 'b'];",
    ));
    assert_eq!(
        coded(&positional, code::E_ENUM_KEYED_ARRAY_APPEND).len(),
        1,
        "{positional:?}"
    );
    let spread = check_src(&keyed(
        "",
        &format!("{LABELS}\n  array<Priority, string> $copy = [...$labels];"),
    ));
    assert_eq!(
        coded(&spread, code::E_ENUM_KEYED_ARRAY_APPEND).len(),
        1,
        "{spread:?}"
    );
}

#[test]
fn a_foreach_key_over_an_enum_keyed_array_is_the_enum() {
    let diags = check_src(&keyed(
        "",
        &format!(
            "{LABELS}\n  foreach ($labels as Priority $k => string $v) {{ Priority $q = $k; }}\n\
             \x20 foreach ($labels as var $key => var $value) {{ Priority $r = $key; }}"
        ),
    ));
    assert!(!diags.has_errors(), "{diags:?}");
    let as_string = check_src(&keyed(
        "",
        &format!("{LABELS}\n  foreach ($labels as string $k => string $v) {{ }}"),
    ));
    assert!(as_string.has_errors(), "{as_string:?}");
}

#[test]
fn an_enum_keyed_array_is_read_as_a_string_keyed_one() {
    let diags = check_src(&keyed(
        "",
        &format!("{LABELS}\n  int $n = Run::plain($labels);\n  array<string> $copy = $labels;"),
    ));
    assert!(!diags.has_errors(), "{diags:?}");
    for from in ["$strings", "$sized"] {
        let diags = check_src(&keyed(
            "",
            &format!(
                "  array<Size, string> $sized = [Size::Small => 's'];\n\
                 \x20 array<Priority, string> $back = {from};"
            ),
        ));
        assert!(diags.has_errors(), "{from}: {diags:?}");
    }
}

#[test]
fn the_key_type_of_an_array_must_be_an_enum() {
    for key in ["string", "int", "Defaults", "?Priority"] {
        let diags = check_src(&keyed("", &format!("  array<{key}, int> $a = [];")));
        let found = coded(&diags, code::E_ARRAY_KEY_TYPE_NOT_ENUM);
        assert_eq!(found.len(), 1, "{key}: {diags:?}");
        assert!(found[0].contains("`array<int>`"), "{key}: {}", found[0]);
    }
    let diags = check_src(&keyed("", "  array<Priority, int> $a = [];"));
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_literal_with_case_keys_and_no_target_names_the_keyed_type() {
    for body in [
        "  var $l = [Priority::Low => 'a'];",
        "  array<string> $l = [Priority::Low => 'a'];",
    ] {
        let diags = check_src(&keyed("", body));
        let found = coded(&diags, code::E_ARRAY_KEY_INVALID_TYPE);
        assert_eq!(found.len(), 1, "{body}: {diags:?}");
        assert!(
            found[0].contains("array<Priority, T>"),
            "{body}: {}",
            found[0]
        );
    }
}

#[test]
fn a_constant_of_an_enum_keyed_array_folds() {
    let src = keyed(
        "  const array<Priority, string> LABELS = [Priority::Low => 'low', Priority::High => 'high'];",
        "  var $l = Defaults::LABELS;\n  string $low = Defaults::LABELS[Priority::Low];",
    );
    let (diags, labels) = folded(&src, "Defaults::LABELS");
    assert!(!diags.has_errors(), "{diags:?}");
    assert_eq!(
        entries(labels),
        [
            key("0", ConstArg::Str("low".to_owned())),
            key("2", ConstArg::Str("high".to_owned())),
        ]
    );
}

#[test]
fn a_string_keyed_array_converts_with_as_and_checks_each_key() {
    let diags = check_src(&keyed(
        "",
        "  array<Priority, string> $a = $strings as array<Priority, string>;\n\
         \x20 ?array<Priority, string> $b = $strings as ?array<Priority, string>;\n\
         \x20 var $c = $m as array<Priority, string>;\n\
         \x20 foreach ($c as Priority $k => var $v) { }",
    ));
    assert!(!diags.has_errors(), "{diags:?}");
    // Without the `as`, a string-keyed array is not an enum-keyed one.
    let bare = check_src(&keyed("", "  array<Priority, string> $a = $strings;"));
    assert!(bare.has_errors(), "{bare:?}");
}

#[test]
fn a_member_that_keeps_keys_keeps_the_enum_key() {
    let diags = check_src(&keyed(
        "",
        &format!(
            "{LABELS}\n\
             \x20 array<Priority, string> $kept = Core\\Arr::filter($labels, fn($v) => $v != '');\n\
             \x20 array<Priority, int> $lengths = Core\\Arr::map($labels, fn($v) => 1);\n\
             \x20 array<Priority, string> $once = Core\\Arr::unique($labels);\n\
             \x20 array<string> $plain = Core\\Arr::filter($strings, fn($v) => true);\n\
             \x20 array<string> $read = Core\\Arr::filter($labels, fn($v) => true);"
        ),
    ));
    assert!(!diags.has_errors(), "{diags:?}");
    // A string-keyed subject gives string keys, and a member that renumbers
    // keeps no key at all.
    for from in [
        "Core\\Arr::filter($strings, fn($v) => true)",
        "Core\\Arr::values($labels)",
    ] {
        let diags = check_src(&keyed(
            "",
            &format!("{LABELS}\n  array<Priority, string> $back = {from};"),
        ));
        assert!(diags.has_errors(), "{from}: {diags:?}");
    }
}
