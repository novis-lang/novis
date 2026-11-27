//! `rule:testing/a-test-only-declaration-is-test-code`: `#[TestOnly]` makes a method, or every
//! method of a class, interface or enum, test code, and a place where production code would still
//! need it does not compile.

mod common;

use common::{check_src, check_src_table};
use nvs_diagnostics::{Code, Diagnostics, code};

fn count(diags: &Diagnostics, want: Code) -> usize {
    diags.iter().filter(|d| d.code == Some(want)).count()
}

/// A method that carries `#[TestOnly]` is test code, and its neighbours are not.
#[test]
fn a_test_only_method_is_test_code() {
    let (diags, table) = check_src_table(
        "<?nvs\nuse Core\\TestOnly;\n\
         class Cache {\n  \
           public int $size = 0;\n  \
           function add(): void { $this->size += 1; }\n  \
           #[TestOnly]\n  \
           function clearAll(): void { $this->size = 0; }\n\
         }\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
    assert!(table.is_test_code("Cache::clearAll"));
    assert!(!table.is_test_code("Cache::add"));
}

/// On a type, the attribute makes every method test code, the constructor included.
#[test]
fn every_method_of_a_test_only_type_is_test_code() {
    let (diags, table) = check_src_table(
        "<?nvs\nuse Core\\TestOnly;\n\
         #[TestOnly]\n\
         class FakeClock {\n  \
           function constructor(public int $now = 0) {}\n  \
           function advance(int $by): void { $this->now += $by; }\n  \
           #[TestOnly]\n  \
           function reset(): void { $this->now = 0; }\n\
         }\n\
         #[TestOnly]\n\
         interface Seeded { function seed(): int; }\n\
         #[TestOnly]\n\
         enum Sample {\n  \
           One,\n  \
           Two,\n  \
           function label(): string { return 'sample'; }\n\
         }\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
    for label in [
        "FakeClock::constructor",
        "FakeClock::advance",
        "FakeClock::reset",
        "Seeded::seed",
        "Sample::label",
    ] {
        assert!(table.is_test_code(label), "{label} is not test code");
    }
}

/// A property, a constant, an enum case and a parameter are not sites the attribute lists.
#[test]
fn test_only_on_a_property_or_constant_does_not_compile() {
    let diags = check_src(
        "<?nvs\nuse Core\\TestOnly;\n\
         class Cache {\n  \
           #[TestOnly]\n  \
           public int $hits = 0;\n  \
           #[TestOnly]\n  \
           const int LIMIT = 10;\n  \
           function add(#[TestOnly] int $by): void { $this->hits += $by; }\n\
         }\n\
         enum Status { #[TestOnly] Active, Banned }\n",
    );
    assert_eq!(count(&diags, code::E_ATTRIBUTE_WRONG_SITE), 4, "{diags:?}");
}

/// A test-only method of a production class cannot override or implement production code,
/// because production calls that method through the parent type.
#[test]
fn a_test_only_method_cannot_override_production_code() {
    let diags = check_src(
        "<?nvs\nuse Core\\TestOnly;\n\
         interface Clock { function now(): int; }\n\
         class Store { function size(): int { return 1; } }\n\
         class Cache extends Store implements Clock {\n  \
           #[TestOnly]\n  \
           function size(): int { return 2; }\n  \
           #[TestOnly]\n  \
           function now(): int { return 0; }\n\
         }\n",
    );
    assert_eq!(
        count(&diags, code::E_TEST_ONLY_IN_PRODUCTION_HIERARCHY),
        2,
        "{diags:?}"
    );

    // A test-only class may implement a production interface: no object of it exists outside
    // tests.
    let diags = check_src(
        "<?nvs\nuse Core\\TestOnly;\n\
         interface Clock { function now(): int; }\n\
         #[TestOnly]\n\
         class FakeClock implements Clock { function now(): int { return 0; } }\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// A method that overrides a test-only method has to be test-only too.
#[test]
fn an_override_of_a_test_only_method_must_be_test_only() {
    let diags = check_src(
        "<?nvs\nuse Core\\TestOnly;\n\
         class Store {\n  \
           #[TestOnly]\n  \
           function reset(): void {}\n\
         }\n\
         class Cache extends Store {\n  \
           function reset(): void {}\n\
         }\n",
    );
    assert_eq!(
        count(&diags, code::E_TEST_ONLY_IN_PRODUCTION_HIERARCHY),
        1,
        "{diags:?}"
    );

    let diags = check_src(
        "<?nvs\nuse Core\\TestOnly;\n\
         class Store {\n  \
           #[TestOnly]\n  \
           function reset(): void {}\n\
         }\n\
         class Cache extends Store {\n  \
           #[TestOnly]\n  \
           function reset(): void {}\n\
         }\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// A production class or interface cannot extend or implement a test-only type, and a test-only
/// type can.
#[test]
fn production_code_cannot_extend_or_implement_a_test_only_type() {
    let diags = check_src(
        "<?nvs\nuse Core\\TestOnly;\n\
         #[TestOnly]\n\
         class FakeStore {}\n\
         #[TestOnly]\n\
         interface Seeded {}\n\
         class Store extends FakeStore {}\n\
         class Cache implements Seeded {}\n\
         interface Sized extends Seeded {}\n\
         #[TestOnly]\n\
         class FakeCache extends FakeStore implements Seeded {}\n",
    );
    assert_eq!(
        count(&diags, code::E_TEST_ONLY_IN_PRODUCTION_HIERARCHY),
        3,
        "{diags:?}"
    );
}
