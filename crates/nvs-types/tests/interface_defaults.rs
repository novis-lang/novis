//! `rule:classes/no-traits`'s interface method bodies — a `public` default and a
//! `private` helper, and who can see each.
//!
//! Moved out of `nvs_types::check`'s inline `mod tests`; every test keeps its
//! own name and body. See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use nvs_diagnostics::code;

// ------------------------------------------------------------------
// `rule:classes/interface-default-methods` and
// `rule:classes/interface-private-methods` -- interface default/private methods.
// ------------------------------------------------------------------

/// `rule:classes/interface-default-methods`, worked through: `Person` never
/// declares `greet()` itself, but inherits it from `Greets` exactly like an
/// ordinary inherited method — the same `resolve_method` ancestor walk
/// `extends` already used, now also walking `implements`.
#[test]
fn a_default_interface_method_is_inherited_and_callable() {
    let diags = check_src(
        "<?nvs\n\
         interface Greets {\n\
         \x20 public function name(): string;\n\
         \x20 public function greet(): string { return \"Hello, \" . $this->name() . \"!\"; }\n\
         }\n\
         class Person implements Greets {\n\
         \x20 private string $personName;\n\
         \x20 function constructor(string $personName) { $this->personName = $personName; }\n\
         \x20 public function name(): string { return $this->personName; }\n\
         }\n\
         class T {\n\
         \x20 function m(): void {\n\
         \x20\x20 Person $p = new Person(\"Ada\");\n\
         \x20\x20 string $g = $p->greet();\n\
         \x20 }\n\
         }\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// A class overriding a default method wins over the interface's own
/// body — an ordinary override, resolved because `resolve_method` checks
/// a class's own signature table before ever walking to `implements`.
/// Proven here by an override that widens the parameter to `?string` and
/// narrows the return to `string`: the `null` argument and the `string`
/// variable would both mismatch if the interface's signature were used
/// instead of the override's own.
#[test]
fn a_class_can_override_a_default_interface_method() {
    let diags = check_src(
        "<?nvs\n\
         interface Greets { public function greet(string $x): ?string { return null; } }\n\
         class Person implements Greets {\n\
         \x20 public function greet(?string $x): string { return \"hi\"; }\n\
         }\n\
         class T {\n\
         \x20 function m(): void {\n\
         \x20\x20 Person $p = new Person();\n\
         \x20\x20 string $s = $p->greet(null);\n\
         \x20 }\n\
         }\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// `rule:classes/interface-default-methods`: `$this` inside an interface's own
/// default method body is typed as that interface, not whatever concrete class
/// ends up implementing it — so a member only the *implementing* class declares is not reachable
/// through `$this` from inside the interface's own body, even though the
/// same call would resolve fine from inside the implementing class.
#[test]
fn this_inside_a_default_method_body_does_not_see_the_implementing_class() {
    let diags = check_src(
        "<?nvs\n\
         interface Greets {\n\
         \x20 public function greet(): string { return $this->onlyOnPerson(); }\n\
         }\n\
         class Person implements Greets {\n\
         \x20 public function onlyOnPerson(): string { return \"x\"; }\n\
         }\n",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_UNKNOWN_MEMBER)),
        "{diags:?}"
    );
}

/// `rule:classes/interface-private-methods`: a `private` interface method is
/// callable from another method of the *same* interface (here, a second
/// default method), and the default method that calls it is itself inherited
/// and externally callable — privacy only closes the helper itself, not the
/// contract around it.
#[test]
fn a_private_interface_method_is_visible_from_its_own_interfaces_default_method() {
    let diags = check_src(
        "<?nvs\n\
         interface Csv {\n\
         \x20 private function escapeField(string $field): string { return $field; }\n\
         \x20 public function toCsvRow(): string { return $this->escapeField(\"x\"); }\n\
         }\n\
         class Report implements Csv {}\n\
         class T {\n\
         \x20 function m(): void {\n\
         \x20\x20 Report $r = new Report();\n\
         \x20\x20 string $s = $r->toCsvRow();\n\
         \x20 }\n\
         }\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// `rule:classes/interface-private-methods`'s diagnostic: an implementing class
/// reaching for the private helper directly via `$this->` is
/// `E_INTERFACE_PRIVATE_METHOD_NOT_VISIBLE` — it was never part of the
/// interface's contract.
#[test]
fn a_private_interface_method_is_not_visible_through_this_from_an_implementing_class() {
    let diags = check_src(
        "<?nvs\n\
         interface Csv {\n\
         \x20 private function escapeField(string $field): string { return $field; }\n\
         \x20 public function toCsvRow(): string { return $this->escapeField(\"x\"); }\n\
         }\n\
         class Report implements Csv {\n\
         \x20 public function toCsvRow(): string { return $this->escapeField(\"y\"); }\n\
         }\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_INTERFACE_PRIVATE_METHOD_NOT_VISIBLE)),
        "{diags:?}"
    );
}

/// The same refusal applies through an explicit qualified call
/// (`InterfaceName::method()`, `rule:classes/member-conflict-is-an-error`'s
/// grammar for reaching a specific default explicitly) — a private helper stays
/// invisible outside its own interface regardless of which call form reaches
/// for it.
#[test]
fn a_private_interface_method_is_not_visible_via_a_qualified_call_from_outside() {
    let diags = check_src(
        "<?nvs\n\
         interface Csv {\n\
         \x20 private function escapeField(string $field): string { return $field; }\n\
         }\n\
         class Report implements Csv {\n\
         \x20 public function useIt(): string { return Csv::escapeField(\"z\"); }\n\
         }\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_INTERFACE_PRIVATE_METHOD_NOT_VISIBLE)),
        "{diags:?}"
    );
}

// ------------------------------------------------------------------
// `rule:classes/member-conflict-is-an-error` -- one method from two sources.
// ------------------------------------------------------------------

/// The messages of the diagnostics a program raises under `E0859`.
fn conflicts(diags: &nvs_diagnostics::Diagnostics) -> Vec<String> {
    diags
        .iter()
        .filter(|d| d.code == Some(code::E_INTERFACE_MEMBER_CONFLICT))
        .map(|d| d.message.clone())
        .collect()
}

#[test]
fn two_defaults_for_one_method_are_an_error_naming_both() {
    let diags = check_src(
        "<?nvs\n\
         interface Greets {\n\
         \x20 public function hello(): string { return \"hi\"; }\n\
         }\n\
         interface Welcomes {\n\
         \x20 public function hello(): string { return \"welcome\"; }\n\
         }\n\
         class Host implements Greets, Welcomes {}\n",
    );
    let found = conflicts(&diags);
    assert_eq!(found.len(), 1, "{diags:?}");
    assert!(
        found[0].contains("`Greets`") && found[0].contains("`Welcomes`"),
        "{}",
        found[0]
    );
}

#[test]
fn a_default_and_a_delegate_for_one_method_are_an_error() {
    let diags = check_src(
        "<?nvs\n\
         interface Greets {\n\
         \x20 public function hello(): string { return \"hi\"; }\n\
         }\n\
         interface Speaks {\n\
         \x20 public function hello(): string;\n\
         }\n\
         class Voice implements Speaks {\n\
         \x20 public function hello(): string { return \"voice\"; }\n\
         }\n\
         class Host implements Greets, Speaks by $voice {\n\
         \x20 private Speaks $voice;\n\
         \x20 function constructor(Speaks $voice) { $this->voice = $voice; }\n\
         }\n",
    );
    let found = conflicts(&diags);
    assert_eq!(found.len(), 1, "{diags:?}");
    assert!(
        found[0].contains("`Greets`") && found[0].contains("`$voice`"),
        "{}",
        found[0]
    );
}

/// The override may still reach one default through `InterfaceName::method()`.
#[test]
fn a_class_that_declares_the_method_resolves_the_conflict() {
    let diags = check_src(
        "<?nvs\n\
         interface Greets {\n\
         \x20 public function hello(): string { return \"hi\"; }\n\
         }\n\
         interface Welcomes {\n\
         \x20 public function hello(): string { return \"welcome\"; }\n\
         }\n\
         class Host implements Greets, Welcomes {\n\
         \x20 public function hello(): string { return Greets::hello() . Welcomes::hello(); }\n\
         }\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// A superclass's concrete method answers first, so two defaults behind it are
/// not a conflict.
#[test]
fn a_superclass_method_wins_over_two_defaults() {
    let diags = check_src(
        "<?nvs\n\
         interface Greets {\n\
         \x20 public function hello(): string { return \"hi\"; }\n\
         }\n\
         interface Welcomes {\n\
         \x20 public function hello(): string { return \"welcome\"; }\n\
         }\n\
         class Base {\n\
         \x20 public function hello(): string { return \"base\"; }\n\
         }\n\
         class Host extends Base implements Greets, Welcomes {}\n",
    );
    assert!(conflicts(&diags).is_empty(), "{diags:?}");
}

/// A default two interfaces reach through one shared parent is one source.
#[test]
fn one_default_reached_through_two_interfaces_is_not_a_conflict() {
    let diags = check_src(
        "<?nvs\n\
         interface Greets {\n\
         \x20 public function hello(): string { return \"hi\"; }\n\
         }\n\
         interface Loud extends Greets {}\n\
         interface Quiet extends Greets {}\n\
         class Host implements Loud, Quiet {}\n",
    );
    assert!(conflicts(&diags).is_empty(), "{diags:?}");
}
