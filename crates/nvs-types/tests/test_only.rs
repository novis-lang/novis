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

/// The declarations the use tests below share: a production class with one test-only method, a
/// test-only class with a static property, a constant and a static method, and a test-only enum.
const SHARED: &str = "<?nvs\nuse Core\\TestOnly;\n\
     class Cache {\n  \
       public int $size = 0;\n  \
       #[TestOnly]\n  \
       function clearAll(): void { $this->size = 0; }\n\
     }\n\
     #[TestOnly]\n\
     class FakeClock {\n  \
       public static int $count = 0;\n  \
       const int START = 3;\n  \
       function constructor(public int $now = 0) {}\n  \
       static function make(): FakeClock { return new FakeClock(); }\n\
     }\n\
     #[TestOnly]\n\
     enum Sample { One, Two }\n";

/// The `E0889`s in `diags`.
fn uses(diags: &Diagnostics) -> Vec<&nvs_diagnostics::Diagnostic> {
    diags
        .iter()
        .filter(|d| d.code == Some(code::E_CALL_INTO_TEST_CODE))
        .collect()
}

/// A call to a test-only method from a production method or from the script does not compile,
/// and the help names `#[TestOnly]`.
#[test]
fn a_call_to_a_test_only_method_outside_test_code_does_not_compile() {
    let diags = check_src(&format!(
        "{SHARED}\
         class Shop {{\n  \
           function reset(Cache $cache): void {{ $cache->clearAll(); }}\n\
         }}\n\
         (new Cache())->clearAll();\n"
    ));
    let found = uses(&diags);
    assert_eq!(found.len(), 2, "{diags:?}");
    for diag in found {
        assert_eq!(
            diag.message,
            "`Cache::clearAll` is test code, so only test code can use it"
        );
        let text = format!("{diag:?}");
        assert!(text.contains("It is marked `#[TestOnly]`"), "{text}");
    }
}

/// A `new` of a test-only class outside test code does not compile.
#[test]
fn new_of_a_test_only_class_outside_test_code_does_not_compile() {
    let diags = check_src(&format!(
        "{SHARED}\
         class Shop {{\n  \
           function now(): int {{ var $clock = new FakeClock(5); return $clock->now; }}\n\
         }}\n\
         var $clock = new FakeClock();\n"
    ));
    let found = uses(&diags);
    assert_eq!(found.len(), 2, "{diags:?}");
    assert!(
        found
            .iter()
            .all(|d| d.message == "`FakeClock` is test code, so only test code can use it"),
        "{diags:?}"
    );
}

/// A static property, a constant, a static method and an enum case of a test-only type outside
/// test code do not compile.
#[test]
fn a_static_member_of_a_test_only_type_outside_test_code_does_not_compile() {
    let diags = check_src(&format!(
        "{SHARED}\
         class Shop {{\n  \
           function total(): int {{ return FakeClock::$count + FakeClock::START; }}\n\
         }}\n\
         var $clock = FakeClock::make();\n\
         var $sample = Sample::One;\n"
    ));
    assert_eq!(uses(&diags).len(), 4, "{diags:?}");
}

/// A test-only type in a production signature or in a type written in a production body does not
/// compile: a property, a parameter, a return type, a method with no body, `class<T>`, `is` and
/// a type argument.
#[test]
fn a_test_only_type_in_a_production_signature_does_not_compile() {
    let diags = check_src(&format!(
        "{SHARED}\
         class Shop {{\n  \
           public ?FakeClock $clock = null;\n  \
           function set(FakeClock $clock): void {{}}\n  \
           function get(): ?Sample {{ return null; }}\n  \
           function kind(class<FakeClock> $kind): void {{}}\n  \
           function check(mixed $value): bool {{ return $value is FakeClock; }}\n  \
           function all(): array<FakeClock> {{ return []; }}\n\
         }}\n\
         interface Clocks {{ function take(FakeClock $clock): void; }}\n"
    ));
    let found = uses(&diags);
    assert_eq!(found.len(), 7, "{diags:?}");
    assert!(
        found.iter().all(|d| d
            .message
            .ends_with("is test code, so only test code can use it")),
        "{diags:?}"
    );
}

/// A test, a test-only method and every method of a test-only type use test-only code freely,
/// and a test-only type may be named in another test-only type's signatures.
#[test]
fn test_code_and_test_only_code_use_test_only_code() {
    let diags = check_src(&format!(
        "{SHARED}\
         use Core\\Test;\n\
         class ShopTest {{\n  \
           #[Test]\n  \
           function aClockStartsAtThree(): void {{\n    \
             var $clock = new FakeClock(FakeClock::START);\n    \
             FakeClock::$count += 1;\n    \
             (new Cache())->clearAll();\n    \
             var $all = fn(FakeClock $c): bool => $c is FakeClock;\n    \
             Test::assertSame(3, $clock->now);\n  \
           }}\n\
         }}\n\
         class Shop {{\n  \
           #[TestOnly]\n  \
           function fake(): FakeClock {{ return FakeClock::make(); }}\n\
         }}\n\
         #[TestOnly]\n\
         class FakeShop {{\n  \
           public ?FakeClock $clock = null;\n  \
           function sample(): Sample {{ return Sample::Two; }}\n  \
           function take(class<FakeClock> $kind, array<FakeClock> $all): void {{}}\n\
         }}\n\
         #[TestOnly]\n\
         interface Clocks {{ function take(FakeClock $clock): void; }}\n"
    ));
    assert!(uses(&diags).is_empty(), "{diags:?}");
    assert!(!diags.has_errors(), "{diags:?}");
}
