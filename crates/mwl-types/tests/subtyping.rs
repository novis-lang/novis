//! Class-typed positions: what a derived class and a cursor class satisfy.
//!
//! Moved out of `mwl_types::check`'s inline `mod tests`; every test keeps its
//! own name and body. See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use mwl_diagnostics::code;

/// MWL's one nominal subtyping rule: a value of a derived class satisfies
/// a position declared at any class or interface it reaches through
/// `extends`/`implements`. See `crate::expr::class_satisfied`.
#[test]
fn a_derived_class_satisfies_a_position_declared_at_its_base() {
    let diags = check_src(
        "<?mwl\n\
         class Animal {}\n\
         class Dog extends Animal {}\n\
         class T {\n\
         \x20 function take(Animal $a): void {}\n\
         \x20 function m(): void { $this->take(new Dog()); }\n\
         }\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// The other half of the same rule: an unrelated class still fails.
#[test]
fn an_unrelated_class_still_fails_a_class_typed_position() {
    let diags = check_src(
        "<?mwl\n\
         class Animal {}\n\
         class Rock {}\n\
         class T {\n\
         \x20 function take(Animal $a): void {}\n\
         \x20 function m(): void { $this->take(new Rock()); }\n\
         }\n",
    );
    assert!(
        diags.iter().any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{diags:?}"
    );
}

/// ADR 0053 § 2's generic interfaces are checked at their argument, not
/// just at their name -- which is what makes an `iterate()` declared
/// `Iterator<int>` able to return a concrete cursor class.
#[test]
fn a_cursor_class_satisfies_the_interface_it_implements_at_that_argument() {
    let diags = check_src(
        "<?mwl\n\
         class Nums implements Iterator<int> {\n\
         \x20 function advance(): bool { return false; }\n\
         \x20 function current(): int { return 1; }\n\
         }\n\
         class Bag implements Iterable<int> {\n\
         \x20 function iterate(): Iterator<int> { return new Nums(); }\n\
         }\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// And is invariant in it: `Iterator<int>` is not an `Iterator<string>`.
#[test]
fn a_cursor_class_is_refused_at_a_different_type_argument() {
    let diags = check_src(
        "<?mwl\n\
         class Nums implements Iterator<int> {\n\
         \x20 function advance(): bool { return false; }\n\
         \x20 function current(): int { return 1; }\n\
         }\n\
         class Bag implements Iterable<string> {\n\
         \x20 function iterate(): Iterator<string> { return new Nums(); }\n\
         }\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_BAD_RETURN_TYPE)),
        "{diags:?}"
    );
}
