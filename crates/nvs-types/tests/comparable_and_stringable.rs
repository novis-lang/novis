//! The two global interfaces an operator demands: ADR 0013's `Comparable` and ADR 0028's `Stringable`.
//!
//! Moved out of `nvs_types::check`'s inline `mod tests`; every test keeps its
//! own name and body. See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use nvs_diagnostics::code;

// ADR 0013: `<`/`<=`/`>`/`>=`/`<=>` between two objects.

#[test]
fn comparable_objects_of_the_same_class_type_check_as_bool_or_int() {
    let diags = check_src(
        "<?nvs\n\
         class Money implements Comparable {\n\
         \x20 private int $cents;\n\
         \x20 function constructor(int $cents) { $this->cents = $cents; }\n\
         \x20 function compareTo(self $other): int { return $this->cents <=> $other->cents; }\n\
         }\n\
         class T {\n\
         \x20 function m(): void {\n\
         \x20\x20 Money $a = new Money(1);\n\
         \x20\x20 Money $b = new Money(2);\n\
         \x20\x20 bool $lt = $a < $b;\n\
         \x20\x20 bool $le = $a <= $b;\n\
         \x20\x20 bool $gt = $a > $b;\n\
         \x20\x20 bool $ge = $a >= $b;\n\
         \x20\x20 int $cmp = $a <=> $b;\n\
         \x20 }\n\
         }\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn comparing_two_objects_of_a_non_comparable_class_is_diagnosed() {
    let diags = check_src(
        "<?nvs\nclass Foo {}\nclass T {\n  function m(): void {\n    Foo $a = new Foo();\n    Foo $b = new Foo();\n    $a < $b;\n  }\n}\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_COMPARISON_REQUIRES_COMPARABLE)),
        "{diags:?}"
    );
}

#[test]
fn comparing_two_different_comparable_classes_is_diagnosed() {
    let diags = check_src(
        "<?nvs\nclass A implements Comparable {\n  function compareTo(self $other): int { return 0; }\n}\nclass B implements Comparable {\n  function compareTo(self $other): int { return 0; }\n}\nclass T {\n  function m(): void {\n    A $a = new A();\n    B $b = new B();\n    $a < $b;\n  }\n}\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_COMPARISON_REQUIRES_COMPARABLE)),
        "{diags:?}"
    );
}

#[test]
fn a_subclass_of_a_comparable_class_is_comparable_to_itself() {
    let diags = check_src(
        "<?nvs\nclass Money implements Comparable {\n  function compareTo(self $other): int { return 0; }\n}\nclass Cents extends Money {}\nclass T {\n  function m(): void {\n    Cents $a = new Cents();\n    Cents $b = new Cents();\n    $a < $b;\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// `rule:types/arithmetic`'s ordering row is closed, so the five spellings have to
/// **agree** on every operand it leaves out — asserted by counting the whole
/// sweep rather than by reading one line off it, since an operator that grew
/// its own answer still looks right on its own.
#[test]
fn every_operator_refuses_every_operand_with_no_ordering_row() {
    let operands = [
        (
            "string $a = \"x\"; string $b = \"y\";",
            code::E_ORDERING_HAS_NO_ROW,
        ),
        (
            "bytes $a = \"x\" as bytes; bytes $b = \"y\" as bytes;",
            code::E_ORDERING_HAS_NO_ROW,
        ),
        (
            "array<int> $a = [1]; array<int> $b = [2];",
            code::E_ORDERING_HAS_NO_ROW,
        ),
        (
            "callable $a = fn (): int => 1; callable $b = fn (): int => 2;",
            code::E_ORDERING_HAS_NO_ROW,
        ),
        (
            "?int $a = null; ?int $b = null; null < null;",
            code::E_ORDERING_HAS_NO_ROW,
        ),
        (
            "Rank $a = Rank::Bronze; Rank $b = Rank::Silver;",
            code::E_ORDERING_HAS_NO_ROW,
        ),
        // The object family keeps ADR 0013's own code however the receiver
        // was spelled — an erased `object` names no class to ask about.
        (
            "object $a = new Plain(); object $b = new Plain();",
            code::E_COMPARISON_REQUIRES_COMPARABLE,
        ),
    ];
    let mut refused = 0;
    for (binding, expected) in operands {
        for op in ["<", "<=", ">", ">=", "<=>"] {
            let src = format!(
                "<?nvs\nenum Rank {{ Bronze, Silver }}\nclass Plain {{}}\nclass T {{\n  function m(): void {{\n    {binding}\n    $a {op} $b;\n  }}\n}}\n"
            );
            let diags = check_src(&src);
            if diags.iter().any(|d| d.code == Some(expected)) {
                refused += 1;
            } else {
                panic!("`{op}` over `{binding}` was not refused: {diags:?}");
            }
        }
    }
    assert_eq!(refused, operands.len() * 5);
}

/// The other side of the same bound: the rows that *are* tabulated stay
/// answered, and `mixed` stays a run-time question rather than a refusal.
#[test]
fn the_tabulated_ordering_rows_are_not_refused() {
    for binding in [
        "int $a = 1; int $b = 2;",
        "uint $a = 1; float $b = 2.5;",
        "float $a = 1.5; int $b = 2;",
        "decimal $a = 1.5; decimal $b = 2.5;",
        // `rule:types/arithmetic` tabulates no `bool` row because that table is about
        // the numeric widenings; two `bool`s are the one bit they already
        // are, which orders exactly and is PHP's answer too.
        "bool $a = true; bool $b = false;",
        "mixed $a = 1; mixed $b = 2;",
    ] {
        let src = format!(
            "<?nvs\nclass T {{\n  function m(): void {{\n    {binding}\n    $a < $b;\n    $a <=> $b;\n  }}\n}}\n"
        );
        let diags = check_src(&src);
        assert!(!diags.has_errors(), "{binding}: {diags:?}");
    }
}

#[test]
fn concatenating_a_non_stringable_object_is_diagnosed() {
    let diags = check_src(
        "<?nvs\nclass Foo {}\nclass T {\n  function m(): void {\n    Foo $a = new Foo();\n    string $s = \"\" . $a;\n  }\n}\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_STRINGABLE_REQUIRED)),
        "{diags:?}"
    );
}

#[test]
fn interpolating_a_non_stringable_object_is_diagnosed() {
    let diags = check_src(
        "<?nvs\nclass Foo {}\nclass T {\n  function m(): void {\n    Foo $a = new Foo();\n    string $s = \"value: $a\";\n  }\n}\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_STRINGABLE_REQUIRED)),
        "{diags:?}"
    );
}

#[test]
fn echoing_a_non_stringable_object_is_diagnosed() {
    let diags = check_src(
        "<?nvs\nclass Foo {}\nclass T {\n  function m(): void {\n    Foo $a = new Foo();\n    echo $a;\n  }\n}\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_STRINGABLE_REQUIRED)),
        "{diags:?}"
    );
}

#[test]
fn printing_a_non_stringable_object_is_diagnosed() {
    let diags = check_src(
        "<?nvs\nclass Foo {}\nclass T {\n  function m(): void {\n    Foo $a = new Foo();\n    print $a;\n  }\n}\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_STRINGABLE_REQUIRED)),
        "{diags:?}"
    );
}

#[test]
fn as_string_on_a_non_stringable_object_is_diagnosed() {
    let diags = check_src(
        "<?nvs\nclass Foo {}\nclass T {\n  function m(): void {\n    Foo $a = new Foo();\n    $a as string;\n  }\n}\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_STRINGABLE_REQUIRED)),
        "{diags:?}"
    );
}

#[test]
fn a_class_implementing_stringable_converts_at_every_site_with_no_diagnostic() {
    let diags = check_src(
        "<?nvs\n\
         class Name implements Stringable {\n\
         \x20 function toString(): string { return \"x\"; }\n\
         }\n\
         class T {\n\
         \x20 function m(): void {\n\
         \x20\x20 Name $a = new Name();\n\
         \x20\x20 string $s1 = \"\" . $a;\n\
         \x20\x20 string $s2 = \"value: $a\";\n\
         \x20\x20 string $s3 = $a as string;\n\
         \x20\x20 echo $a;\n\
         \x20\x20 print $a;\n\
         \x20 }\n\
         }\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// ADR 0028 § 1 over the classes `Core` owns: the registry saying a class has
/// a `toString` is the whole of what makes it stringifiable, so the checker
/// accepts exactly the classes `nvs_stdlib::registry::class_renders` accepts
/// and refuses the rest where they are written rather than leaving a panic
/// below.
///
/// Asked as an **agreement** over a table spanning both rows of that check —
/// a class with a `toString` member, a sink carrier that renders through
/// ADR 0088 § 5 with no member at all, and a `Core` class with neither —
/// because a checker that grew a roster of its own still looks right on any
/// one of those lines. `nvs_stdlib::instance`'s
/// `every_rendering_class_carries_a_renderer_or_is_a_carrier` asserts the
/// run-time half of the same agreement, so a class cannot render where it is
/// written and throw where it is not.
#[test]
fn a_core_class_stringifies_exactly_where_the_registry_says_so() {
    // The expression that builds one, and the class it is an instance of.
    const BUILT: &[(&str, &str)] = &[
        (r#"Core\Uri::parse("https://example.com/a")"#, r"Core\Uri"),
        (
            r#"Core\Uuid::parse("0191b0c4-1c2f-7a3d-8f4e-5a6b7c8d9e0f")"#,
            r"Core\Uuid",
        ),
        (r"Core\Time\Duration::seconds(1)", r"Core\Time\Duration"),
        (r"Core\Out::capture(fn (): void => {})", r"Core\Cli\Text"),
        (r#"Core\Regex::compile("a+")"#, r"Core\Regex\Pattern"),
    ];
    for (built, class) in BUILT {
        let renders = nvs_stdlib::registry::class_renders(class);
        let diags = check_src(&format!(
            "<?nvs\nclass T {{\n  function m(): void {{\n    echo {built};\n  }}\n}}\n"
        ));
        let refused = diags
            .iter()
            .any(|d| d.code == Some(code::E_CORE_CLASS_NOT_STRINGABLE));
        assert_eq!(renders, !refused, "{class} renders={renders}: {diags:?}");
        assert_eq!(
            renders,
            !diags.has_errors(),
            "{class} renders={renders}, but the fixture disagrees: {diags:?}"
        );
    }
}
