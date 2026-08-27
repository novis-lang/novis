//! The two global interfaces an operator demands: ADR 0013's `Comparable` and ADR 0028's `Stringable`.
//!
//! Moved out of `mwl_types::check`'s inline `mod tests`; every test keeps its
//! own name and body. See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use mwl_diagnostics::code;

// ADR 0013: `<`/`<=`/`>`/`>=`/`<=>` between two objects.

#[test]
fn comparable_objects_of_the_same_class_type_check_as_bool_or_int() {
    let diags = check_src(
        "<?mwl\n\
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
        "<?mwl\nclass Foo {}\nclass T {\n  function m(): void {\n    Foo $a = new Foo();\n    Foo $b = new Foo();\n    $a < $b;\n  }\n}\n",
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
        "<?mwl\nclass A implements Comparable {\n  function compareTo(self $other): int { return 0; }\n}\nclass B implements Comparable {\n  function compareTo(self $other): int { return 0; }\n}\nclass T {\n  function m(): void {\n    A $a = new A();\n    B $b = new B();\n    $a < $b;\n  }\n}\n",
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
        "<?mwl\nclass Money implements Comparable {\n  function compareTo(self $other): int { return 0; }\n}\nclass Cents extends Money {}\nclass T {\n  function m(): void {\n    Cents $a = new Cents();\n    Cents $b = new Cents();\n    $a < $b;\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn concatenating_a_non_stringable_object_is_diagnosed() {
    let diags = check_src(
        "<?mwl\nclass Foo {}\nclass T {\n  function m(): void {\n    Foo $a = new Foo();\n    string $s = \"\" . $a;\n  }\n}\n",
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
        "<?mwl\nclass Foo {}\nclass T {\n  function m(): void {\n    Foo $a = new Foo();\n    string $s = \"value: $a\";\n  }\n}\n",
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
        "<?mwl\nclass Foo {}\nclass T {\n  function m(): void {\n    Foo $a = new Foo();\n    echo $a;\n  }\n}\n",
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
        "<?mwl\nclass Foo {}\nclass T {\n  function m(): void {\n    Foo $a = new Foo();\n    print $a;\n  }\n}\n",
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
        "<?mwl\nclass Foo {}\nclass T {\n  function m(): void {\n    Foo $a = new Foo();\n    $a as string;\n  }\n}\n",
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
        "<?mwl\n\
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
/// accepts exactly the classes `mwl_stdlib::registry::class_renders` accepts
/// and refuses the rest where they are written rather than leaving a panic
/// below.
///
/// Asked as an **agreement** over a table spanning both rows of that check —
/// a class with a `toString` member, a sink carrier that renders through
/// ADR 0088 § 5 with no member at all, and a `Core` class with neither —
/// because a checker that grew a roster of its own still looks right on any
/// one of those lines. `mwl_stdlib::instance`'s
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
        let renders = mwl_stdlib::registry::class_renders(class);
        let diags = check_src(&format!(
            "<?mwl\nclass T {{\n  function m(): void {{\n    echo {built};\n  }}\n}}\n"
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
