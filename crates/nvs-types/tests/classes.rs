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

/// A class that declares no `constructor` still has one, and it takes nothing:
/// `nvs_types::expr::calls`' `reject_arguments_to_implicit_constructor` owns
/// why an argument written there has no home at all — there is no signature to
/// check it against and `nvs-ir` lowers the `new` with `ctor: None`, so it was
/// never even evaluated for its effects.
#[test]
fn an_implicit_constructor_is_held_to_zero_arguments() {
    let diags = check_src(
        "<?nvs\nclass Plain {\n  public int $n = 0;\n}\nclass T {\n  function m(): void {\n    Plain $p = new Plain(1, 2);\n  }\n}\n",
    );
    let found = diags
        .iter()
        .find(|d| d.code == Some(code::E_ARITY_MISMATCH))
        .unwrap_or_else(|| panic!("{diags:?}"));
    assert!(
        found.message.contains("expected 0 argument(s), found 2"),
        "the count is against the zero arguments an implicit constructor takes: {found:?}"
    );
}

/// The same class constructed the one way it accepts.
#[test]
fn an_implicit_constructor_accepts_a_bare_new() {
    let diags = check_src(
        "<?nvs\nclass Plain {\n  public int $n = 0;\n}\nclass T {\n  function m(): void {\n    Plain $p = new Plain();\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// The refusal is about the *resolved* constructor, not about the constructed
/// class declaring one itself: `new Dog(1)` on a `Dog extends Animal` invokes
/// `Animal::constructor` and is checked against that signature.
#[test]
fn an_inherited_constructor_is_not_an_implicit_one() {
    let diags = check_src(
        "<?nvs\nclass Animal {\n  function constructor(int $legs) {}\n}\nclass Dog extends Animal {}\nclass T {\n  function m(): void {\n    Dog $d = new Dog(1);\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
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

/// ADR 0011's class constant on a **user-declared** class: a read of one
/// answers the type its own declaration wrote, and not the value's, and not
/// `mixed`.
///
/// Both sides of every row, because either half alone reads plausibly. A
/// checker that answered `mixed` everywhere would fail the accepting half —
/// `mixed` is not implicitly assignable to a narrower type — and one that
/// answered the *folded value's* type, ignoring the annotation, would pass it
/// and fail the refusing half on the two rows where the two disagree
/// (`uint`, which folds to an `int`, and `secret string`, which folds to a
/// plain one).
#[test]
fn a_user_declared_class_constant_reads_at_its_declared_type() {
    // `(annotation, written value, a type the annotation is not)`. The
    // disagreeing type is never `float` for an integer row: ADR 0007 § 2's one
    // implicit widening would accept it.
    let rows: &[(&str, &str, &str)] = &[
        ("int", "3", "string"),
        ("uint", "5", "int"),
        ("string", "\"n\"", "int"),
        ("bool", "true", "int"),
        ("float", "0.5", "string"),
        ("secret string", "\"k\"", "string"),
    ];
    for (annotation, value, disagreeing) in rows {
        let accepting = format!(
            "<?nvs
class Limits {{
  public const {annotation} K = {value};
}}
             class T {{
  function m(): void {{
    {annotation} $x = Limits::K;
  }}
}}
"
        );
        let diags = check_src(&accepting);
        assert!(
            !diags.has_errors(),
            "`{annotation}` constant read into a `{annotation}` binding: {diags:?}"
        );

        let refusing = format!(
            "<?nvs
class Limits {{
  public const {annotation} K = {value};
}}
             class T {{
  function m(): void {{
    {disagreeing} $x = Limits::K;
  }}
}}
"
        );
        let diags = check_src(&refusing);
        assert!(
            diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
            "a `{annotation}` constant must not read as `{disagreeing}`: {diags:?}"
        );
    }
}

/// The four spellings a constant is reached by, all answering the one declared
/// type: the class's own, an ancestor's through `extends`, an interface's
/// through `implements`, and `self::` from inside the declaring class.
///
/// `crate::signatures::resolve_const` walks both edges for the reason
/// `ConstTable`'s own lookup does, so this asserts the walk rather than the
/// row — a table that recorded only own declarations would still pass the
/// first spelling.
#[test]
fn a_class_constant_resolves_through_extends_implements_and_self() {
    let diags = check_src(
        "<?nvs
interface HasLimit {
  public const int CEILING = 9;
}
         class Limits implements HasLimit {
  public const int MAX = 3;
           function own(): int {
    return self::MAX;
  }
}
         class Tighter extends Limits {}
         class T {
  function m(): void {
    int $own = Limits::MAX;
             int $inherited = Tighter::MAX;
    int $ceiling = Limits::CEILING;
             int $through = Tighter::CEILING;
  }
}
",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// A constant declared with no annotation, which parses (PHP 8.3's own rule),
/// takes the type of the value it folded to — the closest thing to a
/// declaration the source contains, and the rule `signatures::ConstSig` owns.
#[test]
fn an_unannotated_class_constant_reads_at_its_values_type() {
    let diags = check_src(
        "<?nvs
class Limits {
  public const BARE = \"bare\";
}
         class T {
  function m(): void {
    string $x = Limits::BARE;
  }
}
",
    );
    assert!(!diags.has_errors(), "{diags:?}");

    let diags = check_src(
        "<?nvs
class Limits {
  public const BARE = \"bare\";
}
         class T {
  function m(): void {
    int $x = Limits::BARE;
  }
}
",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}
