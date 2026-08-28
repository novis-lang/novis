//! Property and method access through `$this`, `self`, `parent` and a typed local.
//!
//! Moved out of `nvs_types::check`'s inline `mod tests`; every test keeps its
//! own name and body. See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use nvs_diagnostics::code;

#[test]
fn a_type_alias_is_substituted_into_a_local_declaration() {
    let diags = check_src(
        "<?nvs\ntype Id = uint;\nclass T {\n  function m(): void {\n    Id $x = 1;\n    int $y = $x;\n  }\n}\n",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "`Id` expands to `uint`, so assigning it into a plain `int` should mismatch: {diags:?}"
    );
}

#[test]
fn self_resolves_inside_a_method_body() {
    let diags =
        check_src("<?nvs\nclass T {\n  function m(): void {\n    self $x = new self();\n  }\n}\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_return_type_mismatch_is_diagnosed() {
    let diags = check_src("<?nvs\nclass T {\n  function m(): int {\n    return \"x\";\n  }\n}\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_BAD_RETURN_TYPE))
    );
}

#[test]
fn a_matching_return_type_is_fine() {
    let diags = check_src("<?nvs\nclass T {\n  function m(): int {\n    return 1;\n  }\n}\n");
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_this_property_access_has_its_declared_type() {
    // The property has an inline default, so ADR 0022's own check
    // (`crate::ctor_init`) has nothing to say about a missing
    // constructor here — this fixture is only exercising property-type
    // recovery.
    let diags = check_src(
        "<?nvs\nclass T {\n  public int $count = 0;\n  function m(): void {\n    int $n = $this->count;\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_this_property_type_mismatch_is_diagnosed() {
    let diags = check_src(
        "<?nvs\nclass T {\n  public int $count;\n  function m(): void {\n    string $n = $this->count;\n  }\n}\n",
    );
    assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
}

#[test]
fn a_this_method_call_returns_its_declared_type() {
    let diags = check_src(
        "<?nvs\nclass T {\n  function a(): int { return 1; }\n  function b(): void {\n    int $n = $this->a();\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn an_undeclared_this_method_call_is_diagnosed() {
    let diags =
        check_src("<?nvs\nclass T {\n  function m(): void {\n    $this->missing();\n  }\n}\n");
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_UNKNOWN_MEMBER)),
        "{diags:?}"
    );
}

#[test]
fn a_property_access_on_a_new_expression_resolves() {
    // Inline default again, for the same reason as the fixture above.
    let diags = check_src(
        "<?nvs\nclass Foo {\n  public int $count = 0;\n}\nclass T {\n  function m(): void {\n    int $n = (new Foo())->count;\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn an_undeclared_property_on_a_typed_local_is_diagnosed() {
    let diags = check_src(
        "<?nvs\nclass Foo {}\nclass T {\n  function m(): void {\n    Foo $x = new Foo();\n    $x->missing;\n  }\n}\n",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_UNKNOWN_MEMBER)),
        "{diags:?}"
    );
}

#[test]
fn an_arity_mismatch_on_a_method_call_is_diagnosed() {
    let diags = check_src(
        "<?nvs\nclass T {\n  function a(int $x): void {}\n  function b(): void {\n    $this->a();\n  }\n}\n",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_ARITY_MISMATCH)),
        "{diags:?}"
    );
}

#[test]
fn an_argument_type_mismatch_on_a_method_call_is_diagnosed() {
    let diags = check_src(
        "<?nvs\nclass T {\n  function a(int $x): void {}\n  function b(): void {\n    $this->a(\"s\");\n  }\n}\n",
    );
    assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
}

#[test]
fn a_constructor_argument_is_type_checked() {
    let diags = check_src(
        "<?nvs\nclass Foo {\n  function constructor(int $x) {}\n}\nclass T {\n  function m(): void {\n    new Foo(\"s\");\n  }\n}\n",
    );
    assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
}

#[test]
fn a_static_call_return_type_is_recovered() {
    let diags = check_src(
        "<?nvs\nclass T {\n  static function make(): int { return 1; }\n  function m(): void {\n    int $n = self::make();\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn new_parent_resolves_to_the_parent_class() {
    let diags = check_src(
        "<?nvs\nclass Base {}\nclass Sub extends Base {\n  function m(): void {\n    Base $x = new parent();\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// `parent` as a *type* atom (a parameter here) — distinct from `new
/// parent(...)`, which the previous test already covers — resolves
/// against the same `extends` link.
#[test]
fn a_parent_typed_parameter_resolves_to_the_parent_class() {
    let diags = check_src(
        "<?nvs\nclass Base {}\nclass Sub extends Base {\n  function m(parent $x): void {\n    Base $y = $x;\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// A class with no `extends` has no parent for the `parent` type atom to
/// name — diagnosed rather than silently `mixed`, unlike `new
/// parent(...)`'s deliberate silent fallback for the same shape.
#[test]
fn a_parent_type_atom_with_no_extends_is_diagnosed() {
    let diags = check_src("<?nvs\nclass Base {\n  function m(parent $x): void {\n  }\n}\n");
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_NO_PARENT_CLASS)),
        "{diags:?}"
    );
}

#[test]
fn a_match_expressions_type_is_the_union_of_its_arms() {
    let diags = check_src(
        "<?nvs\nclass T {\n  function m(): void {\n    string $n = match (1) { 1 => 2, default => 3 };\n  }\n}\n",
    );
    assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
}

#[test]
fn a_ternary_expressions_type_is_the_union_of_its_branches() {
    let diags = check_src(
        "<?nvs\nclass T {\n  function m(): void {\n    int|string $n = true ? 1 : \"s\";\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

#[test]
fn a_ternary_expressions_type_mismatch_is_diagnosed() {
    let diags = check_src(
        "<?nvs\nclass T {\n  function m(): void {\n    int $n = true ? 1 : \"s\";\n  }\n}\n",
    );
    assert!(diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)));
}

#[test]
fn a_class_type_still_refuses_the_nullable_conversion() {
    // ADR 0066 § 3's class row, which is **absolute**: `as` converts between
    // the types ADR 0007 § 2 tabulates, and none of them is a class.
    // `instanceof` plus ADR 0007 § 6's narrowing answers class membership,
    // and § 3a's `tryParse` answers a parse.
    let diags = check_src(
        "<?nvs\nclass P {\n  public int $n = 1;\n}\nclass T {\n  function m(object $o): void {\n    var $p = $o as ?P;\n  }\n}\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_CLASS_CONVERSION_TARGET)),
        "{diags:?}"
    );
    // `Core\Uri` and `Core\Uuid` are named here on purpose. § 3 first admitted
    // them as a two-class *parse roster* where `$s as ?T` compiled, and
    // withdrew it; from a `string` operand — the exact shape the roster
    // existed for — they are the same diagnostic as any other class now, and
    // this is what would notice the exception growing back.
    for class in [r"Core\Uri", r"Core\Uuid"] {
        let diags = check_src(&format!(
            "<?nvs\nclass T {{\n  function m(string $s): void {{\n    var $v = $s as ?{class};\n  }}\n}}\n"
        ));
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_CLASS_CONVERSION_TARGET)),
            "{class}: {diags:?}"
        );
    }
}

#[test]
fn a_try_parse_answers_the_nullable_of_its_class() {
    // ADR 0066 § 3a: the member that replaced the withdrawn roster. It reads
    // one `string` and answers `?T`, so a `?Core\Uri` binding accepts it...
    let diags = check_src(
        "<?nvs\nclass T {\n  function m(string $s): void {\n    ?Core\\Uri $v = Core\\Uri::tryParse($s);\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
    // ...and a non-nullable one does not. The `null` is in the *type*, not
    // only in what happens at run time, which is the half a `try…` name alone
    // could not have promised.
    let diags = check_src(
        "<?nvs\nclass T {\n  function m(string $s): void {\n    Core\\Uri $v = Core\\Uri::tryParse($s);\n  }\n}\n",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

/// ADR 0046 § 2's compile-time constant set — a literal, another class's
/// `const`, an enum case — at a *property* default.
///
/// Written as an **agreement** rather than a row per form: every named
/// constant is asserted to fold to the identical `ConstArg` the literal
/// spelling of the same value folds to, so a form that resolved to a
/// plausible-but-different value (an enum case taken as its ordinal, a `float`
/// property armed from the `int` digits) fails here while still compiling
/// clean. `nvs_types::defaults` owns which forms these are, and both ends of
/// the set are named: the two refusals below are what the widening must *not*
/// have opened.
#[test]
fn a_property_default_accepts_every_compile_time_constant() {
    let (diags, exprs) = check_src_table(
        r#"<?nvs
enum Mode: int { Fast = 3, Slow = 7 }
class Limits {
  public const int COUNT = 12;
  public const int LOW = -4;
  public const string NAME = "novis";
  public const float RATE = 1.5;
  public const bool DEBUG = true;
}
class Config {
  public int $count = Limits::COUNT;
  public int $countLiteral = 12;
  public int $low = Limits::LOW;
  public int $lowLiteral = -4;
  public string $name = Limits::NAME;
  public string $nameLiteral = "novis";
  public float $rate = Limits::RATE;
  public float $rateLiteral = 1.5;
  public bool $debug = Limits::DEBUG;
  public bool $debugLiteral = true;
  public uint $span = Limits::COUNT;
  public uint $spanLiteral = 12;
  public float $widened = Limits::COUNT;
  public float $widenedLiteral = 12;
  public Mode $mode = Mode::Fast;
  public int $modeBacking = 3;
}
"#,
    );
    assert!(!diags.has_errors(), "{diags:?}");
    let defaults = exprs.property_defaults("Config");
    let value = |name: &str| {
        defaults
            .iter()
            .find(|(property, _)| property == name)
            .map(|(_, value)| value.clone())
            .unwrap_or_else(|| panic!("`{name}` armed no default at all: {defaults:?}"))
    };
    for (named, literal) in [
        ("count", "countLiteral"),
        ("low", "lowLiteral"),
        ("name", "nameLiteral"),
        ("rate", "rateLiteral"),
        ("debug", "debugLiteral"),
        ("span", "spanLiteral"),
        ("widened", "widenedLiteral"),
        // ADR 0010 § 3: the case *is* its backing integer, so the slot an
        // enum-typed property arms holds exactly what the integer literal
        // arms — there is no second representation for it to drift into.
        ("mode", "modeBacking"),
    ] {
        assert_eq!(
            value(named),
            value(literal),
            "`${named}` and `${literal}` are the same constant written twice"
        );
    }

    // The other end: a case of a *different* enum is not a constant of this
    // property's declared type, exactly as a `string` literal is not.
    let diags = check_src(
        "<?nvs\nenum Mode { Fast, Slow }\nenum Color { Red, Blue }\nclass T {\n  public Mode $m = Color::Red;\n}\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_PROPERTY_DEFAULT_NOT_LITERAL)),
        "{diags:?}"
    );

    // And ADR 0033 § 1's qualifier travels with the value: a folded constant
    // has lost it, so the declared type is the only thing left to check it
    // against, and a plain `string` slot is not it.
    let diags = check_src(
        "<?nvs\nclass Secrets {\n  public const secret string TOKEN = \"k\";\n}\nclass T {\n  public string $t = Secrets::TOKEN;\n}\n",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "a `secret` constant must not launder into a plain `string` slot: {diags:?}"
    );
}
