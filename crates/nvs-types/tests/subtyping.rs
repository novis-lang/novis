//! Class-typed positions: what a derived class and a cursor class satisfy.
//!
//! Moved out of `nvs_types::check`'s inline `mod tests`; every test keeps its
//! own name and body. See `tests/common/mod.rs` for the shared fixtures.

mod common;

use common::*;
use nvs_diagnostics::code;

/// Novis's one nominal subtyping rule: a value of a derived class satisfies
/// a position declared at any class or interface it reaches through
/// `extends`/`implements`. See `crate::expr::assign::class_satisfied`.
#[test]
fn a_derived_class_satisfies_a_position_declared_at_its_base() {
    let diags = check_src(
        "<?nvs\n\
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
        "<?nvs\n\
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
        "<?nvs\n\
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
        "<?nvs\n\
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

/// ADR 0125 § 3: `class<T>` is covariant in its argument and only upward, so
/// `class<Dog>` reaches a `class<Animal>` position and `class<Animal>` does not
/// reach a `class<Dog>` one. Both halves are asserted together, because a rule
/// that only accepts is satisfied by making the type `mixed`.
#[test]
fn a_class_reference_widens_to_its_supertype_and_not_back() {
    let widening = check_src(
        "<?nvs\n\
         class Animal {}\n\
         class Dog extends Animal {}\n\
         class T {\n\
         \x20 function take(class<Animal> $c): void {}\n\
         \x20 function m(class<Dog> $d): void { $this->take($d); }\n\
         }\n",
    );
    assert!(!widening.has_errors(), "{widening:?}");

    let narrowing = check_src(
        "<?nvs\n\
         class Animal {}\n\
         class Dog extends Animal {}\n\
         class T {\n\
         \x20 function take(class<Dog> $c): void {}\n\
         \x20 function m(class<Animal> $a): void { $this->take($a); }\n\
         }\n",
    );
    assert!(
        narrowing
            .iter()
            .any(|d| d.code == Some(code::E_TYPE_MISMATCH)),
        "{narrowing:?}"
    );
}

/// The argument names a class or an interface, and anything else is refused
/// where it is written -- ADR 0125 § 1, and the half the parser deliberately
/// left to the checker so the refusal can say what the name resolved *to*.
#[test]
fn a_class_reference_over_a_non_class_argument_is_refused() {
    let diags = check_src(
        "<?nvs\n\
         class T {\n\
         \x20 function m(class<int> $c): void {}\n\
         }\n",
    );
    assert!(
        diags
            .iter()
            .any(|d| d.code == Some(code::E_CLASS_REF_ARGUMENT_NOT_A_CLASS)),
        "{diags:?}"
    );
}

/// ADR 0126 § 1's argument rule, which is the sibling above's one row
/// narrower: a key's values are the names an implementor *declares*, so an
/// interface is refused with the scalars while a class reference admits one.
#[test]
fn a_property_key_over_something_other_than_a_class_is_refused() {
    for argument in ["int", "Named"] {
        let diags = check_src(&format!(
            "<?nvs\n\
             interface Named {{}}\n\
             class T {{\n\
             \x20 function m(property<{argument}> $k): void {{}}\n\
             }}\n"
        ));
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(code::E_PROPERTY_KEY_ARGUMENT_NOT_A_CLASS)),
            "`property<{argument}>`: {diags:?}"
        );
    }

    let over_a_class = check_src(
        "<?nvs\n\
         class User { public string $email = \"\"; }\n\
         class T {\n\
         \x20 function m(property<User> $k): void {}\n\
         }\n",
    );
    assert!(
        !over_a_class
            .iter()
            .any(|d| d.code == Some(code::E_PROPERTY_KEY_ARGUMENT_NOT_A_CLASS)),
        "{over_a_class:?}"
    );
}
