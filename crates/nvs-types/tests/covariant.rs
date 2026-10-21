//! `rule:classes/a-covariant-type-is-checked-where-a-value-enters`, at the
//! checker: a `covariant` parameter narrower than the topmost declaration
//! compiles, and every other narrowing is a compile error where it is written.
//! `conformance::check_covariant_params` is the check.

mod common;

use common::check_src;
use nvs_diagnostics::{Diagnostics, code};

const ANIMALS: &str = "class Animal {}\nclass Dog extends Animal {}\nclass Puppy extends Dog {}\n";

/// `decls` after the three classes every case narrows between.
fn check(decls: &str) -> Diagnostics {
    check_src(&format!("<?nvs\n{ANIMALS}{decls}\n"))
}

fn codes(diags: &Diagnostics) -> Vec<&'static str> {
    diags
        .iter()
        .filter_map(|d| d.code.map(|c| c.as_str()))
        .collect()
}

fn helps(diags: &Diagnostics) -> String {
    diags
        .iter()
        .flat_map(|d| d.notes.iter().cloned())
        .collect::<Vec<_>>()
        .join("\n")
}

// covers: lang:classes/covariant
#[test]
fn a_covariant_parameter_narrower_than_the_parents_compiles_in_each_form() {
    let diags = check(
        "class Keeper {\n\
         public function feed(Animal $a, inout Animal $b, Animal ...$rest): void {}\n\
         public static function make(?Animal $a = null): void {}\n\
         public function tag(tainted string|int $s): void {}\n\
         }\n\
         class DogKeeper extends Keeper {\n\
         public function feed(covariant Dog $a, inout covariant Dog $b, covariant Dog ...$rest): void {}\n\
         public static function make(covariant ?Dog $a): void {}\n\
         public function tag(covariant tainted string $s): void {}\n\
         }",
    );
    assert!(codes(&diags).is_empty(), "{diags:?}");
}

#[test]
fn a_covariant_parameter_narrows_an_interfaces_method() {
    let diags = check(
        "interface Feeds { public function feed(Animal $a): void; }\n\
         interface FeedsDogs extends Feeds { public function feed(covariant Dog $a): void; }\n\
         class DogKeeper implements FeedsDogs { public function feed(covariant Dog $a): void {} }",
    );
    assert!(codes(&diags).is_empty(), "{diags:?}");
}

#[test]
fn a_grandchild_is_measured_against_the_topmost_declaration() {
    let narrowed = "class Keeper { public function feed(Animal $a): void {} }\n\
                    class DogKeeper extends Keeper { public function feed(covariant Dog $a): void {} }\n";
    let again = check(&format!(
        "{narrowed}class PuppyKeeper extends DogKeeper {{ public function feed(covariant Puppy $a): void {{}} }}"
    ));
    assert!(codes(&again).is_empty(), "{again:?}");

    let dropped = check(&format!(
        "{narrowed}class PuppyKeeper extends DogKeeper {{ public function feed(Dog $a): void {{}} }}"
    ));
    assert_eq!(codes(&dropped), [code::E_INCOMPATIBLE_OVERRIDE.as_str()]);
    assert!(helps(&dropped).contains("`covariant`"), "{dropped:?}");
}

#[test]
fn a_narrower_parameter_without_the_keyword_names_it_in_the_help() {
    let diags = check(
        "class Keeper { public function feed(Animal $a): void {} }\n\
         class DogKeeper extends Keeper { public function feed(Dog $a): void {} }",
    );
    assert_eq!(codes(&diags), [code::E_INCOMPATIBLE_OVERRIDE.as_str()]);
    assert!(helps(&diags).contains("`covariant`"), "{diags:?}");

    // An unrelated type is not narrower, so its help does not name it.
    let unrelated = check(
        "class Keeper { public function feed(Animal $a): void {} }\n\
         class DogKeeper extends Keeper { public function feed(string $a): void {} }",
    );
    assert_eq!(codes(&unrelated), [code::E_INCOMPATIBLE_OVERRIDE.as_str()]);
    assert!(!helps(&unrelated).contains("`covariant`"), "{unrelated:?}");
}

#[test]
fn a_covariant_type_that_is_not_narrower_is_misplaced() {
    let same = check(
        "class Keeper { public function feed(Dog $a): void {} }\n\
         class DogKeeper extends Keeper { public function feed(covariant Dog $a): void {} }",
    );
    assert_eq!(codes(&same), [code::E_COVARIANT_MISPLACED.as_str()]);

    let wider = check(
        "class Keeper { public function feed(Dog $a): void {} }\n\
         class DogKeeper extends Keeper { public function feed(covariant Animal $a): void {} }",
    );
    assert_eq!(codes(&wider), [code::E_COVARIANT_MISPLACED.as_str()]);

    let unrelated = check(
        "class Keeper { public function feed(Dog $a): void {} }\n\
         class DogKeeper extends Keeper { public function feed(covariant string $a): void {} }",
    );
    assert_eq!(codes(&unrelated), [code::E_INCOMPATIBLE_OVERRIDE.as_str()]);
}

#[test]
fn covariant_is_misplaced_where_no_wider_value_can_enter() {
    let diags = check(
        "class Stray {\n\
         public function feed(covariant Dog $a): void {}\n\
         public function constructor(covariant Dog $first, public readonly covariant Dog $second) {}\n\
         public static covariant ?Dog $last = null;\n\
         }\n\
         callable $f = fn(covariant Dog $d) => 1;",
    );
    assert_eq!(codes(&diags), [code::E_COVARIANT_MISPLACED.as_str(); 5]);
}

#[test]
fn a_covariant_type_is_one_is_tests_in_constant_time() {
    for ty in ["array<int>", "{y: int}", "callable(int): int"] {
        let diags = check(&format!(
            "class Keeper {{ public function feed(mixed $a): void {{}} }}\n\
             class DogKeeper extends Keeper {{ public function feed(covariant {ty} $a): void {{}} }}"
        ));
        assert_eq!(
            codes(&diags),
            [code::E_COVARIANT_TYPE_NOT_TESTABLE.as_str()],
            "{ty}: {diags:?}"
        );
    }
    let union = check(
        "class Keeper { public function feed(mixed $a): void {} }\n\
         class DogKeeper extends Keeper { public function feed(covariant Dog|int|null $a): void {} }",
    );
    assert!(codes(&union).is_empty(), "{union:?}");
}

#[test]
fn the_topmost_default_must_pass_the_narrower_type() {
    let refused = check(
        "class Keeper { public function feed(?Animal $a = null): void {} }\n\
         class DogKeeper extends Keeper { public function feed(covariant Dog $a): void {} }",
    );
    assert_eq!(
        codes(&refused),
        [code::E_COVARIANT_DEFAULT_REJECTED.as_str()]
    );

    let scalar = check(
        "class Keeper { public function size(int|string $n = 3): void {} }\n\
         class IntKeeper extends Keeper { public function size(covariant int $n): void {} }\n\
         class TextKeeper extends Keeper { public function size(covariant string $n): void {} }",
    );
    assert_eq!(
        codes(&scalar),
        [code::E_COVARIANT_DEFAULT_REJECTED.as_str()]
    );
}

#[test]
fn covariant_keeps_the_parents_qualifier() {
    let diags = check(
        "class Keeper { public function tag(tainted string|int $s): void {} }\n\
         class TextKeeper extends Keeper { public function tag(covariant string $s): void {} }",
    );
    assert_eq!(codes(&diags), [code::E_INCOMPATIBLE_OVERRIDE.as_str()]);
}

#[test]
fn a_subclass_narrows_compare_to_against_the_class_that_first_wrote_it() {
    let price = "class Price implements Comparable {\n\
                 public function compareTo(Price $other): int { return 0; }\n\
                 }\n";
    let sale = check(&format!(
        "{price}class Sale extends Price {{ public function compareTo(covariant Sale $other): int {{ return 0; }} }}"
    ));
    assert!(codes(&sale).is_empty(), "{sale:?}");

    // `Comparable` already gives `Price` its own class, so there is nothing to narrow.
    let direct = check(
        "class Price implements Comparable {\n\
         public function compareTo(covariant Price $other): int { return 0; }\n\
         }",
    );
    assert_eq!(codes(&direct), [code::E_COVARIANT_MISPLACED.as_str()]);
}
