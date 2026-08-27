//! ADR 0043's interface method bodies — a `public` default and a `private` helper, and who can see each.
//!
//! Moved out of `nvs_types::check`'s inline `mod tests`; every test keeps its
//! own name and body. See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use nvs_diagnostics::code;

// ------------------------------------------------------------------
// ADR 0043 §§ 2-3 -- interface default/private methods (M2 follow-up).
// ------------------------------------------------------------------

/// § 2's own worked example: `Person` never declares `greet()` itself,
/// but inherits it from `Greets` exactly like an ordinary inherited
/// method — the same `resolve_method` ancestor walk `extends` already
/// used, now also walking `implements`.
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
/// Proven here by a signature-shape change (`string` vs `int`) that would
/// mismatch if the stale interface signature were used instead of the
/// override's own.
#[test]
fn a_class_can_override_a_default_interface_method() {
    let diags = check_src(
        "<?nvs\n\
         interface Greets { public function greet(string $x): void {} }\n\
         class Person implements Greets {\n\
         \x20 public function greet(int $x): void {}\n\
         }\n\
         class T {\n\
         \x20 function m(): void {\n\
         \x20\x20 Person $p = new Person();\n\
         \x20\x20 $p->greet(1);\n\
         \x20 }\n\
         }\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// § 2: `$this` inside an interface's own default method body is typed as
/// that interface, not whatever concrete class ends up implementing it —
/// so a member only the *implementing* class declares is not reachable
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

/// § 3: a `private` interface method is callable from another method of
/// the *same* interface (here, a second default method), and the default
/// method that calls it is itself inherited and externally callable —
/// privacy only closes the helper itself, not the contract around it.
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

/// § 3's own diagnostic: an implementing class reaching for the private
/// helper directly via `$this->` is `E_INTERFACE_PRIVATE_METHOD_NOT_VISIBLE`
/// — it was never part of the interface's contract.
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
/// (`InterfaceName::method()`, § 5's grammar for reaching a specific
/// default explicitly) — a private helper stays invisible outside its own
/// interface regardless of which call form reaches for it.
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
