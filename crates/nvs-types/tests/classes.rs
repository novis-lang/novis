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
    // The property has an inline default, so `rule:classes/definite-property-initialization`'s own check
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

/// The `Core` half of the same rule, and it is a *different* diagnostic on
/// purpose. A `Core` class is not constructed by a `constructor` member at all
/// — `nvs_stdlib::registry::CONSTRUCTORS` is the whole roster of names `new`
/// may be written on, and `Core\Str` is not one — so the answer names the
/// member the class does not have rather than counting arguments against a
/// signature that was never going to exist. The arguments are what a program
/// actually writes when it mistakes a `Core` namespace for a constructible
/// class, so they are what this pins.
#[test]
fn a_core_class_with_no_constructor_refuses_arguments() {
    let diags = check_src(
        "<?nvs\nclass T {\n  function m(): void {\n    mixed $s = new Core\\Str(\"a\", 1);\n  }\n}\n",
    );
    let found = diags
        .iter()
        .find(|d| d.code == Some(code::E_UNKNOWN_MEMBER))
        .unwrap_or_else(|| panic!("{diags:?}"));
    assert!(
        found
            .message
            .contains("`Core\\Str` has no member named `constructor`"),
        "the refusal names the constructor `Core\\Str` does not have: {found:?}"
    );
}

/// The bare form is refused identically, so the test above reads as "this
/// class is not constructible" rather than "these two arguments are too many".
#[test]
fn a_core_class_with_no_constructor_refuses_a_bare_new_too() {
    let diags = check_src(
        "<?nvs\nclass T {\n  function m(): void {\n    mixed $s = new Core\\Str();\n  }\n}\n",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_UNKNOWN_MEMBER)),
        "{diags:?}"
    );
}

/// A `Core` name the registry does not hold at all is the *class* mistake, not
/// the member one. `QName::is_core` is a spelling test, so `check_new_target`
/// used to accept anything under `Core\` and leave `nvs-codegen` to fail with
/// "this unit declares no descriptor for it" — an internal message for an
/// ordinary typo.
#[test]
fn an_unregistered_core_class_is_undeclared_rather_than_a_codegen_failure() {
    let diags = check_src(
        "<?nvs\nclass T {\n  function m(): void {\n    mixed $x = new Core\\Bogus();\n  }\n}\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_UNDEFINED_CLASS)),
        "{diags:?}"
    );
}

/// The second roster, and the one row of it the registry does not hold:
/// `nvs_hir::errors::TREE`'s `Core\Test\Failure` is the only namespaced entry,
/// and `QName::is_reserved_global_class`'s own docs say it is trusted through
/// `is_core` rather than through that predicate. Holding a `Core` target to
/// the registry alone withdrew exactly that trust and broke every `throw new
/// Core\Test\Failure(...)` in the corpus.
#[test]
fn the_namespaced_exception_tree_row_is_still_a_new_target() {
    let diags = check_src(
        "<?nvs\nclass T {\n  function m(): void {\n    throw new Core\\Test\\Failure(\"no\");\n  }\n}\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// The registered, constructible end of the same lookup: a name in
/// `CONSTRUCTORS` still passes, arguments and all.
#[test]
fn a_registered_core_constructor_still_accepts_its_own_new() {
    let diags = check_src(
        "<?nvs\nclass T {\n  function m(): void {\n    Core\\ObjectSet<int> $s = new Core\\ObjectSet<int>();\n  }\n}\n",
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
    // `rule:expressions/nullable-conversion-availability`'s class row, which is **absolute**: `as` converts between
    // the types `rule:types/conversion` tabulates, and none of them is a class.
    // `instanceof` plus `rule:types/unions-and-mixed`'s narrowing answers class membership,
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
    // `rule:expressions/try-parse`: the member that replaced the withdrawn roster. It reads
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

/// `rule:attributes/payload-is-a-compile-time-constant`'s compile-time constant set — a literal, another class's
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
        // `rule:enums/no-class-machinery`: the case *is* its backing integer, so the slot an
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

    // And `rule:security/secret-qualifier`'s qualifier travels with the value: a folded constant
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

/// `rule:classes/no-free-functions-or-constants`'s class constant on a **user-declared** class: a read of one
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
    // disagreeing type is never `float` for an integer row: `rule:types/conversion`'s one
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

/// A constant reads at the type its declaration writes — the rule
/// `signatures::ConstSig` owns. PHP 8.3's unannotated spelling, which used to
/// read at the value it folded to instead, is now refused in the parser
/// (`E0246`, docs/adr/README.md § *Decisions taken at project start*).
#[test]
fn a_class_constant_reads_at_its_declared_type() {
    let diags = check_src(
        "<?nvs
class Limits {
  public const string BARE = \"bare\";
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
  public const string BARE = \"bare\";
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

/// `rule:classes/constructor-compatibility`: a `new` over a `class<T>` checks its arguments against `T`'s
/// constructor, and the value may hold any implementor of `T` -- so one whose
/// constructor is incompatible makes that check a promise the program cannot
/// keep. Refused at the `new`, naming the subclass, and never at the subclass's
/// own declaration.
#[test]
fn a_dynamic_new_is_refused_naming_the_subclass_whose_constructor_differs() {
    let divergent = check_src(
        "<?nvs\n\
         abstract class Animal {\n\
         \x20 public function constructor(string $name) {}\n\
         }\n\
         class Dog extends Animal {\n\
         \x20 public function constructor(string $name, int $age) {\n\
         \x20   parent::constructor($name);\n\
         \x20 }\n\
         }\n\
         class T {\n\
         \x20 function m(class<Animal> $c): void { new $c(\"Rex\"); }\n\
         }\n",
    );
    assert!(
        divergent
            .iter()
            .any(|d| d.code == Some(code::E_DYNAMIC_NEW_DIVERGENT_CONSTRUCTOR)),
        "{divergent:?}"
    );
    // The subclass is named, because its file is the one the author edits.
    assert!(
        divergent
            .iter()
            .any(|d| d.notes.iter().any(|n| n.contains("`Dog`"))),
        "{divergent:?}"
    );

    // The very same declarations, instantiated by name: § 5 refuses the `new`
    // over the class reference and leaves `Dog` alone everywhere else.
    let written_out = check_src(
        "<?nvs\n\
         abstract class Animal {\n\
         \x20 public function constructor(string $name) {}\n\
         }\n\
         class Dog extends Animal {\n\
         \x20 public function constructor(string $name, int $age) {\n\
         \x20   parent::constructor($name);\n\
         \x20 }\n\
         }\n\
         class T {\n\
         \x20 function m(): void { Animal $a = new Dog(\"Rex\", 3); }\n\
         }\n",
    );
    assert!(!written_out.has_errors(), "{written_out:?}");

    // An inherited constructor is the common case and a widened one is still
    // substitutable: neither is a divergence.
    let compatible = check_src(
        "<?nvs\n\
         abstract class Animal {\n\
         \x20 public function constructor(string $name) {}\n\
         }\n\
         class Dog extends Animal {}\n\
         class Cat extends Animal {\n\
         \x20 public function constructor(string $name, int $age = 1) {\n\
         \x20   parent::constructor($name);\n\
         \x20 }\n\
         }\n\
         class T {\n\
         \x20 function m(class<Animal> $c): void { new $c(\"Rex\"); }\n\
         }\n",
    );
    assert!(!compatible.has_errors(), "{compatible:?}");
}

/// `rule:types/class-reference-sites`: the three spellings that reach a class through a *value* take
/// a `class<T>` and nothing else. `new $cls()` types its arguments against
/// `T`'s constructor and yields a `T` -- including where `T` is `abstract`,
/// which is the case the feature exists for -- `$cls::f()` resolves the member
/// on `T`'s roster, and `$x is $cls` asks the descriptor. Everything
/// else keeps `E0496` at all three, with a help that names the conversion.
#[test]
fn a_class_reference_carries_the_three_dynamic_sites() {
    let accepted = check_src(
        "<?nvs\n\
         abstract class Animal {\n\
         \x20 public function constructor(string $name) {}\n\
         \x20 public static function kind(): string { return \"animal\"; }\n\
         }\n\
         class Dog extends Animal {}\n\
         class T {\n\
         \x20 function m(class<Animal> $c, mixed $v): void {\n\
         \x20   Animal $a = new $c(\"Rex\");\n\
         \x20   string $k = $c::kind();\n\
         \x20   bool $b = $v is $c;\n\
         \x20 }\n\
         }\n",
    );
    assert!(!accepted.has_errors(), "{accepted:?}");

    // The constructor really is the one consulted: with no signature resolved
    // the extra argument would go unreported, as it did before § 4.
    let arity = check_src(
        "<?nvs\n\
         abstract class Animal {\n\
         \x20 public function constructor(string $name) {}\n\
         }\n\
         class T {\n\
         \x20 function m(class<Animal> $c): void { new $c(\"Rex\", \"extra\"); }\n\
         }\n",
    );
    assert!(arity.has_errors(), "{arity:?}");

    // And the site is typed as `T`, not as `mixed`.
    let typed = check_src(
        "<?nvs\n\
         class Animal {\n\
         \x20 public function constructor(string $name) {}\n\
         }\n\
         class Rock {}\n\
         class T {\n\
         \x20 function m(class<Animal> $c): void { Rock $r = new $c(\"Rex\"); }\n\
         }\n",
    );
    assert!(
        typed.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{typed:?}"
    );

    let refused = check_src(
        "<?nvs\n\
         class Animal {}\n\
         class T {\n\
         \x20 function m(string $s, mixed $v): void {\n\
         \x20   new $s();\n\
         \x20   $s::kind();\n\
         \x20   $v is $s;\n\
         \x20 }\n\
         }\n",
    );
    let dynamic: Vec<_> = refused
        .iter()
        .filter(|d| d.code == Some(code::E_DYNAMIC_CLASS_NAME))
        .collect();
    assert_eq!(dynamic.len(), 3, "{refused:?}");
    assert!(
        dynamic
            .iter()
            .all(|d| d.notes.iter().any(|n| n.contains("as class<Base>"))),
        "{refused:?}"
    );
}

/// `rule:types/class-reference`: `as` is a class reference's only source. A bare `string` does
/// not reach a `class<T>` position, the conversion does, and a written-out
/// `Foo::class` operand is decided where it stands rather than at run time --
/// both ways, so that the "decided" half is not satisfied by accepting
/// everything.
#[test]
fn a_string_becomes_a_class_reference_only_through_as() {
    let bare = check_src(
        "<?nvs\n\
         class Animal {}\n\
         class T {\n\
         \x20 function take(class<Animal> $c): void {}\n\
         \x20 function m(string $s): void { $this->take($s); }\n\
         }\n",
    );
    assert!(
        bare.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{bare:?}"
    );

    let converted = check_src(
        "<?nvs\n\
         class Animal {}\n\
         class Dog extends Animal {}\n\
         class T {\n\
         \x20 function take(class<Animal> $c): void {}\n\
         \x20 function m(string $s): void { $this->take($s as class<Animal>); }\n\
         \x20 function n(): void { $this->take(Dog::class as class<Animal>); }\n\
         \x20 function o(class<Dog> $d): void { $this->take($d as class<Animal>); }\n\
         }\n",
    );
    assert!(!converted.has_errors(), "{converted:?}");

    // The qualifier strips, as every checked conversion strips one: the
    // conversion's whole output range is the classes this program declares to
    // be `Animal`s, which a tainted string cannot widen (`rule:types/class-reference`).
    let laundered = check_src(
        "<?nvs\n\
         class Animal {}\n\
         class T {\n\
         \x20 function take(class<Animal> $c): void {}\n\
         \x20 function m(tainted string $s): void { $this->take($s as class<Animal>); }\n\
         }\n",
    );
    assert!(!laundered.has_errors(), "{laundered:?}");

    let outside = check_src(
        "<?nvs\n\
         class Animal {}\n\
         class Rock {}\n\
         class T {\n\
         \x20 function m(): void { Rock::class as class<Animal>; }\n\
         }\n",
    );
    assert!(
        outside
            .iter()
            .any(|d| d.code == Some(code::E_NO_CONVERSION)),
        "{outside:?}"
    );
}

/// `rule:types/property-key`: a property key's values are the public properties of the
/// class it names -- its own and its ancestors' -- and nothing else, with `as`
/// the only door into one. The written-out operand is decided where it stands,
/// both ways, so that "decided" is not satisfied by accepting everything, and
/// the private name is refused with the misspelled one because visibility is
/// decided at the conversion.
#[test]
fn a_property_key_ranges_over_the_public_properties_and_nothing_else() {
    const CLASSES: &str = "<?nvs\n\
         class Base { public int $id = 0; }\n\
         class User extends Base {\n\
         \x20 public string $email = \"\";\n\
         \x20 private string $token = \"\";\n\
         }\n";

    // Every row of § 2 at once: the `string` door written out (own property and
    // inherited one), the same door computed, the qualifier stripping as every
    // checked conversion strips one, the key-to-key narrowing, and the total
    // way back out.
    let rows = check_src(&format!(
        "{CLASSES}class T {{\n\
         \x20 function take(property<User> $k): void {{}}\n\
         \x20 function m(): void {{ $this->take(\"email\" as property<User>); }}\n\
         \x20 function n(): void {{ $this->take(\"id\" as property<User>); }}\n\
         \x20 function o(string $s): void {{ $this->take($s as property<User>); }}\n\
         \x20 function p(tainted string $s): void {{ $this->take($s as property<User>); }}\n\
         \x20 function q(property<User> $k): property<Base> {{ return $k as property<Base>; }}\n\
         \x20 function r(property<User> $k): string {{ return $k as string; }}\n\
         }}\n"
    ));
    assert!(!rows.has_errors(), "{rows:?}");

    // `as` is the only source: a `string` does not reach the position on its
    // own, and no row produces a key from anything but the two § 2 names.
    let bare = check_src(&format!(
        "{CLASSES}class T {{\n\
         \x20 function take(property<User> $k): void {{}}\n\
         \x20 function m(string $s): void {{ $this->take($s); }}\n\
         }}\n"
    ));
    assert!(
        bare.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{bare:?}"
    );

    let unconvertible = check_src(&format!(
        "{CLASSES}class T {{\n\
         \x20 function m(): void {{ 1 as property<User>; }}\n\
         }}\n"
    ));
    assert!(
        unconvertible
            .iter()
            .any(|d| d.code == Some(code::E_NO_CONVERSION)),
        "{unconvertible:?}"
    );

    // The set is exactly the public roster: a `private` property and a name
    // that names nothing are one failure, reported where the operand is
    // written, and the help lists what the set actually holds.
    for name in ["token", "emial"] {
        let refused = check_src(&format!(
            "{CLASSES}class T {{\n\
             \x20 function m(): void {{ \"{name}\" as property<User>; }}\n\
             }}\n"
        ));
        let unknown: Vec<_> = refused
            .iter()
            .filter(|d| d.code == Some(code::E_UNKNOWN_MEMBER))
            .collect();
        assert_eq!(unknown.len(), 1, "`{name}`: {refused:?}");
        assert!(
            unknown[0].notes.iter().any(|note| {
                note.contains("`$email`") && note.contains("`$id`") && !note.contains("$token")
            }),
            "`{name}`: {refused:?}"
        );
    }
}

/// `rule:types/property-key-access`: `$obj->$key` reads as the union of the set the key ranges
/// over -- both sides of that bound, since a read typed too widely and one
/// typed too narrowly both look right against one half of it. The `private`
/// property contributes a type no public one has, so its absence from the
/// union is asserted rather than assumed.
#[test]
fn a_read_through_a_property_key_types_as_the_union_of_the_set() {
    const CLASSES: &str = "<?nvs\n\
         class Base { public int $id = 0; }\n\
         class User extends Base {\n\
         \x20 public string $email = \"\";\n\
         \x20 private float $rate = 0.0;\n\
         }\n";

    // The union itself: every member of the set is in the read's type, and a
    // return position naming exactly that union takes it.
    let exact = check_src(&format!(
        "{CLASSES}class T {{\n\
         \x20 function m(User $u, property<User> $k): string|int {{ return $u->$k; }}\n\
         }}\n"
    ));
    assert!(!exact.has_errors(), "{exact:?}");

    // The narrow side: one member of the set is not the whole of it, so a
    // position naming only `string` refuses the `int` the same key may name.
    let narrow = check_src(&format!(
        "{CLASSES}class T {{\n\
         \x20 function m(User $u, property<User> $k): string {{ return $u->$k; }}\n\
         }}\n"
    ));
    assert!(
        narrow
            .iter()
            .any(|d| d.code == Some(code::E_BAD_RETURN_TYPE)),
        "{narrow:?}"
    );

    // The wide side: the union is the *public* roster's types and stops there,
    // so the `private` property's `float` is not one of the answers.
    let wide = check_src(&format!(
        "{CLASSES}class T {{\n\
         \x20 function m(User $u, property<User> $k): float {{ return $u->$k; }}\n\
         }}\n"
    ));
    assert!(
        wide.iter().any(|d| d.code == Some(code::E_BAD_RETURN_TYPE)),
        "{wide:?}"
    );

    // A subclass receiver satisfies the key's own class, and the union is
    // still the key's roster -- `Admin`'s own public property is not a name
    // this key can hold, so it is not in the read's type either.
    let subclass = check_src(&format!(
        "{CLASSES}class Admin extends User {{ public bool $super = false; }}\n\
         class T {{\n\
         \x20 function m(Admin $a, property<User> $k): string|int {{ return $a->$k; }}\n\
         }}\n"
    ));
    assert!(!subclass.has_errors(), "{subclass:?}");
}

/// `rule:types/property-key`'s second refusal: `property<T>` over a `T` with no public
/// property is `E0799`, since no value of it could exist -- asked of both
/// spellings the `as` admits, because the written-out operand and the computed
/// one take different paths to the same roster and only one of them was ever
/// diagnosed.
#[test]
fn a_property_key_over_a_class_with_no_public_property_is_refused() {
    const CLASSES: &str = "<?nvs\n\
         class Opaque { private int $n = 0; }\n\
         class User { public string $email = \"\"; }\n";

    // A written-out operand: the fault is the type, so `E0799` is the whole
    // report -- the name it was handed is not a second mistake.
    let written = check_src(&format!(
        "{CLASSES}class T {{\n\
         \x20 function m(): void {{ \"n\" as property<Opaque>; }}\n\
         }}\n"
    ));
    assert!(
        written
            .iter()
            .any(|d| d.code == Some(code::E_PROPERTY_KEY_ARGUMENT_NOT_A_CLASS)),
        "{written:?}"
    );
    assert!(
        !written
            .iter()
            .any(|d| d.code == Some(code::E_UNKNOWN_MEMBER)),
        "{written:?}"
    );

    // A computed operand reaches the same refusal, which is the half that used
    // to lower a run-time test no name could pass.
    let computed = check_src(&format!(
        "{CLASSES}class T {{\n\
         \x20 function m(string $s): void {{ $s as property<Opaque>; }}\n\
         }}\n"
    ));
    assert!(
        computed
            .iter()
            .any(|d| d.code == Some(code::E_PROPERTY_KEY_ARGUMENT_NOT_A_CLASS)),
        "{computed:?}"
    );

    // The other side of the bound: one public property is a set, and neither
    // spelling is refused over it.
    let populated = check_src(&format!(
        "{CLASSES}class T {{\n\
         \x20 function m(string $s): void {{ \"email\" as property<User>; $s as property<User>; }}\n\
         }}\n"
    ));
    assert!(!populated.has_errors(), "{populated:?}");
}

/// `rule:types/property-key-access`'s last paragraph: a write through a key is refused where the
/// key's public set holds a `readonly` property, naming it -- and the bound is
/// asserted on both sides, since a refusal written over the *receiver* rather
/// than over the set would look identical on the failing half alone. The read
/// is asserted green in the same class, because § 5 refuses the write and not
/// the key.
#[test]
fn a_write_through_a_key_whose_set_holds_a_readonly_property_is_refused() {
    const CLASSES: &str = "<?nvs\n\
         class User {\n\
         \x20 public string $email = \"\";\n\
         \x20 function constructor(public readonly int $id) {}\n\
         }\n\
         class Post {\n\
         \x20 public string $title = \"\";\n\
         \x20 public int $views = 0;\n\
         }\n";

    // The refusal itself, naming the `readonly` member the write might have
    // meant rather than the one it was written with -- there is none.
    let refused = check_src(&format!(
        "{CLASSES}class T {{\n\
         \x20 function m(User $u, property<User> $k): void {{ $u->$k = 1; }}\n\
         }}\n"
    ));
    let readonly: Vec<_> = refused
        .iter()
        .filter(|d| d.code == Some(code::E_READONLY_WRITE_AFTER_CONSTRUCTION))
        .collect();
    assert_eq!(readonly.len(), 1, "{refused:?}");
    assert!(readonly[0].message.contains("`User::$id`"), "{refused:?}");

    // A read through the same key is untouched: § 5 refuses the write, and a
    // key over a set holding a `readonly` property is still a key.
    let read = check_src(&format!(
        "{CLASSES}class T {{\n\
         \x20 function m(User $u, property<User> $k): string|int {{ return $u->$k; }}\n\
         }}\n"
    ));
    assert!(!read.has_errors(), "{read:?}");

    // The other side of the bound: every member of this set is assignable, so
    // the same write through a `property<Post>` is not refused at all.
    let allowed = check_src(&format!(
        "{CLASSES}class T {{\n\
         \x20 function m(Post $p, property<Post> $k): void {{ $p->$k = 1; }}\n\
         }}\n"
    ));
    assert!(!allowed.has_errors(), "{allowed:?}");
}

/// `rule:types/property-key-access`: the operand's *type* is what admits `$obj->$key`, so every
/// other operand keeps `E0235` -- now reported by this crate, since a parser
/// sees no types. § 4's own table of neighbours is the body: a call, a
/// receiver with no `T` to check against, and a receiver that is not one.
#[test]
fn a_computed_member_name_without_a_property_key_is_still_e0235() {
    const CLASSES: &str = "<?nvs\n\
         class User { public string $email = \"\"; }\n\
         class Other { public string $email = \"\"; }\n";

    let has_e0235 = |src: &str| {
        check_src(src)
            .iter()
            .any(|d| d.code == Some(code::E_DYNAMIC_MEMBER_NAME))
    };

    // The control: with a key of the receiver's own class, nothing is
    // reported at all -- which is what makes the four refusals below about the
    // operand rather than about the spelling.
    let admitted = check_src(&format!(
        "{CLASSES}class T {{\n\
         \x20 function m(User $u, property<User> $k): void {{ echo $u->$k; }}\n\
         \x20 function n(User $u, property<User> $k): void {{ echo $u->{{$k}}; }}\n\
         }}\n"
    ));
    assert!(!admitted.has_errors(), "{admitted:?}");

    // A plain `string` is the operand the rule exists to refuse: it names a
    // member only when the statement runs.
    assert!(
        has_e0235(&format!(
            "{CLASSES}class T {{\n\
             \x20 function m(User $u, string $s): void {{ echo $u->$s; }}\n\
             }}\n"
        )),
        "a `string` operand"
    );

    // § 4 row 2, both halves: a receiver with no `T` cannot check the one
    // thing the key promises, whether it is erased or merely another class.
    assert!(
        has_e0235(&format!(
            "{CLASSES}class T {{\n\
             \x20 function m(mixed $m, property<User> $k): void {{ echo $m->$k; }}\n\
             }}\n"
        )),
        "a `mixed` receiver"
    );
    assert!(
        has_e0235(&format!(
            "{CLASSES}class T {{\n\
             \x20 function m(Other $o, property<User> $k): void {{ echo $o->$k; }}\n\
             }}\n"
        )),
        "a receiver that does not satisfy the key's class"
    );

    // § 4 row 1: a key is not a method name, so computed dispatch is refused
    // with the same code and for `rule:classes/no-call-magic`'s older reason.
    assert!(
        has_e0235(&format!(
            "{CLASSES}class T {{\n\
             \x20 function m(User $u, property<User> $k): void {{ $u->$k(); }}\n\
             }}\n"
        )),
        "a computed method name"
    );

    // § 4 row 3: `unset` through a key is refused where `unset` of any
    // declared property is, and is one error rather than two.
    let unset = check_src(&format!(
        "{CLASSES}class T {{\n\
         \x20 function m(User $u, property<User> $k): void {{ unset($u->$k); }}\n\
         }}\n"
    ));
    assert!(
        unset
            .iter()
            .any(|d| d.code == Some(code::E_UNSET_ON_PROPERTY))
            && !unset
                .iter()
                .any(|d| d.code == Some(code::E_DYNAMIC_MEMBER_NAME)),
        "{unset:?}"
    );
}

/// `rule:types/type-test`: a `Core` name on the right of `is` is a written type
/// like any other, and the type resolver is the whole of what decides it — a
/// registered class with instances, a namespaced `nvs_hir::errors::TREE` entry,
/// which is an exception class under `Core\` by spelling alone, and a `Core`
/// **namespace** class all resolve as types and answer `bool`. No roster of
/// testable names sits between the two, which is what the one type test bought:
/// the question is what a value can hold, and every name a type may be written
/// with can be asked.
#[test]
fn a_core_name_on_the_right_of_is_is_a_written_type_like_any_other() {
    let accepted = check_src(
        "<?nvs\n\
         class T {\n\
         \x20 function m(mixed $v): void {\n\
         \x20   bool $a = $v is Core\\Time\\Date;\n\
         \x20   bool $b = $v is Core\\Uri;\n\
         \x20   bool $c = $v is Core\\Db\\DbError;\n\
         \x20   bool $d = $v is Core\\Json;\n\
         \x20 }\n\
         }\n",
    );
    assert!(!accepted.has_errors(), "{accepted:?}");
}

/// The one right-hand side `rule:types/class-reference-sites` refuses does not
/// widen with the `Core` roster: a computed name is refused because Novis has
/// no dynamic class names at all. An enum is **not** refused here — declared or
/// `Core`-owned, it is a row of `rule:types/type-test`'s table, and asking
/// `$v is Rank` answers what asking every case in turn answers.
#[test]
fn a_value_naming_no_class_is_refused_and_an_enum_is_a_row_of_the_table() {
    let refused = check_src(
        "<?nvs\n\
         enum Rank: int { Low = 1, High = 2 }\n\
         class T {\n\
         \x20 function m(mixed $v, string $s): void {\n\
         \x20   bool $a = $v is $s;\n\
         \x20   bool $b = $v is Rank;\n\
         \x20   bool $c = $v is Core\\Http\\Method;\n\
         \x20 }\n\
         }\n",
    );
    let refusals: Vec<_> = refused
        .iter()
        .filter(|d| d.code == Some(code::E_DYNAMIC_CLASS_NAME))
        .collect();
    assert_eq!(refusals.len(), 1, "{refused:?}");
    assert!(
        refusals
            .iter()
            .any(|d| d.message.contains("is a type or a class reference")),
        "{refused:?}"
    );
    assert_eq!(refused.iter().count(), 1, "{refused:?}");
}

/// `rule:types/conversion`'s `mixed`-to-class row reaches a `Core` class with instances on a
/// declared class's terms: the descriptor the process publishes is a class to
/// test the value against, so the downcast is checked rather than asserted. A
/// `Core` namespace class names no descriptor and keeps `E0711`, and an operand
/// sharing no value with the target is the ordinary disjointness refusal.
#[test]
fn a_core_class_is_a_checked_conversion_target() {
    let accepted = check_src(
        "<?nvs\n\
         class T {\n\
         \x20 function m(mixed $v): void {\n\
         \x20   Core\\Time\\Date $d = $v as Core\\Time\\Date;\n\
         \x20   string $s = $d->format(\"yyyy\");\n\
         \x20 }\n\
         }\n",
    );
    assert!(!accepted.has_errors(), "{accepted:?}");

    let namespace = check_src(
        "<?nvs\n\
         class T {\n\
         \x20 function m(mixed $v): void { $v as Core\\Json; }\n\
         }\n",
    );
    assert!(
        namespace
            .iter()
            .any(|d| d.code == Some(code::E_UNTESTABLE_CONVERSION_TARGET)),
        "{namespace:?}"
    );

    // `rule:core-classes/html-auto-escape` keeps `as Core\Html\Markup` a lift, so it is decided
    // by `crate::expr::quals` rather than by this table and a literal is
    // accepted where a computed operand is not.
    let lift = check_src(
        "<?nvs\n\
         class T {\n\
         \x20 function m(): void { Core\\Html\\Markup $m = \"<b>b</b>\" as Core\\Html\\Markup; }\n\
         }\n",
    );
    assert!(!lift.has_errors(), "{lift:?}");
}

/// The other three `ConvKind::Object` targets name no class at all, so there is
/// nothing to test a value against and the operand has to be an object already:
/// `$plain as object` is the free widening row, and everything wider than an
/// object keeps `E0711`.
#[test]
fn an_object_a_shape_and_a_callable_are_still_untestable_targets() {
    let refused = check_src(
        "<?nvs\n\
         class T {\n\
         \x20 function m(mixed $v): void {\n\
         \x20   $v as object;\n\
         \x20   $v as callable;\n\
         \x20   $v as {x: int};\n\
         \x20 }\n\
         }\n",
    );
    assert_eq!(
        refused
            .iter()
            .filter(|d| d.code == Some(code::E_UNTESTABLE_CONVERSION_TARGET))
            .count(),
        3,
        "{refused:?}"
    );

    let widened = check_src(
        "<?nvs\n\
         class Foo {}\n\
         class T {\n\
         \x20 function m(Foo $f): void { object $o = $f as object; }\n\
         }\n",
    );
    assert!(!widened.has_errors(), "{widened:?}");
}
