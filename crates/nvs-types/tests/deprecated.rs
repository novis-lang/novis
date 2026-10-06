//! `rule:attributes/a-deprecation-names-its-replacement-as-code`: what
//! `#[Core\Deprecated]`'s payload takes, and which declarations it may be
//! written above.

mod common;

use common::check_src;
use nvs_diagnostics::{Code, Diagnostics, code};

fn count(diags: &Diagnostics, want: Code) -> usize {
    diags.iter().filter(|d| d.code == Some(want)).count()
}

/// `since`, `note` and `replace` are taken on each of the nine declarations,
/// and `construct` on a class; any other field is refused, once per field.
#[test]
fn a_deprecated_attribute_takes_since_note_and_replace() {
    let diags = check_src(
        "<?nvs\n\
         class Shop {\n  \
           const int LIMIT = 10;\n  \
           public int $total = 0;\n  \
           public int $count = 0;\n  \
           function size(): int { return $this->count; }\n\
         }\n\
         #[Core\\Deprecated(since: '2.0', note: 'Use Shop.', replace: 'Shop', construct: 'new Shop()')]\n\
         class Store {\n  \
           #[Core\\Deprecated(since: '2.0')]\n  \
           const int LIMIT = 10;\n  \
           #[Core\\Deprecated(note: 'Read `count`.')]\n  \
           public int $total = 0;\n  \
           public int $count = 0;\n  \
           #[Core\\Deprecated(since: '2.0', note: 'Pass a limit.')]\n  \
           function constructor(#[Core\\Deprecated(since: '2.0')] int $limit = 0) {}\n  \
           #[Core\\Deprecated(replace: '$this->count')]\n  \
           function size(): int { return $this->total; }\n\
         }\n\
         #[Core\\Deprecated(since: '2.0')]\n\
         interface Sized {}\n\
         #[Core\\Deprecated(note: 'Use Color.')]\n\
         enum Status { #[Core\\Deprecated(since: '2.0')] Active, Banned }\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");

    let diags = check_src(
        "<?nvs\nuse Core\\Deprecated;\nclass Store {\n  \
         #[Deprecated(reason: 'old', replace: 'x', until: '3.0')]\n  \
         function size(): int { return 1; }\n}\n",
    );
    assert_eq!(count(&diags, code::E_UNKNOWN_OPTION), 2, "{diags:?}");
}

/// A property hook and its parameter are not declarations a use names, and
/// `construct` belongs to a class: each is refused where it is written.
#[test]
fn a_deprecated_attribute_on_an_attach_site_it_does_not_take_is_refused() {
    let diags = check_src(
        "<?nvs\nclass Store {\n  \
         public int $n = 0;\n  \
         public int $doubled {\n    \
           #[Core\\Deprecated(since: '2.0')]\n    \
           get => $this->n * 2;\n    \
           set(#[Core\\Deprecated] int $v) { $this->n = $v; }\n  \
         }\n}\n",
    );
    assert_eq!(
        count(&diags, code::E_DEPRECATED_ON_A_PROPERTY_HOOK),
        2,
        "{diags:?}"
    );

    let diags = check_src(
        "<?nvs\n\
         #[Core\\Deprecated(construct: 'new Shop()')]\n\
         interface Sized {}\n\
         #[Core\\Deprecated(construct: 'new Shop()')]\n\
         enum Status { Active }\n\
         class Shop {\n  \
           #[Core\\Deprecated(construct: 'new Shop()')]\n  \
           function size(#[Core\\Deprecated(construct: 'x')] int $n): int { return $n; }\n\
         }\n",
    );
    assert_eq!(count(&diags, code::E_UNKNOWN_OPTION), 4, "{diags:?}");
}

/// A template that is not one expression, one type name or one named argument
/// is `E0845`; one that parses and names something its scope does not have is
/// `E0846`. Nothing else is reported for either.
#[test]
fn a_deprecation_template_that_does_not_compile_is_refused_at_the_declaration() {
    let diags = check_src(
        "<?nvs\n\
         class Api {\n  \
           #[Core\\Deprecated(replace: '$this->find(')]\n  \
           function a(): int { return 1; }\n  \
           #[Core\\Deprecated(replace: '$this->find(1) 2')]\n  \
           function b(): int { return 1; }\n  \
           #[Core\\Deprecated(replace: 'find(\\'x\\')')]\n  \
           function c(): int { return 1; }\n  \
           function d(#[Core\\Deprecated(replace: '$n')] int $n, int $m = 0): int { return $n; }\n  \
           function find(int $id): int { return $id; }\n\
         }\n\
         #[Core\\Deprecated(replace: 'new Api()')]\n\
         class Blog {}\n",
    );
    assert_eq!(
        count(&diags, code::E_DEPRECATION_TEMPLATE_DOES_NOT_PARSE),
        5,
        "{diags:?}"
    );
    assert_eq!(diags.error_count(), 5, "{diags:?}");

    let diags = check_src(
        "<?nvs\n\
         class Api {\n  \
           #[Core\\Deprecated(replace: '$this->missing()')]\n  \
           function a(): int { return 1; }\n  \
           #[Core\\Deprecated(replace: '$this->find($nope)')]\n  \
           function b(int $id): int { return 1; }\n  \
           #[Core\\Deprecated(replace: '$this->find(1)')]\n  \
           static function c(): int { return 1; }\n  \
           function d(#[Core\\Deprecated(replace: 'count: $n')] int $n): int { return $n; }\n  \
           function find(int $id): int { return $id; }\n\
         }\n\
         #[Core\\Deprecated(replace: 'Nowhere')]\n\
         class Blog {}\n",
    );
    assert_eq!(
        count(&diags, code::E_DEPRECATION_TEMPLATE_DOES_NOT_COMPILE),
        5,
        "{diags:?}"
    );
    assert_eq!(diags.error_count(), 5, "{diags:?}");

    let diags = check_src(
        "<?nvs\n\
         class Api {\n  \
           #[Core\\Deprecated(replace: '$this->find($id)')]\n  \
           function findById(int $id): int { return $id; }\n  \
           #[Core\\Deprecated(replace: 'Api::make()')]\n  \
           static function create(): Api { return new Api(); }\n  \
           function size(#[Core\\Deprecated(replace: 'limit: $count * 2')] int $count = 0, int $limit = 0): int { return $limit; }\n  \
           function find(int $id): int { return $id; }\n  \
           static function make(): Api { return new Api(); }\n\
         }\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
}

/// A method's template fits its return type, a property's its type, a
/// constant's its type and a parameter's the parameter it names.
#[test]
fn a_deprecation_template_whose_type_does_not_fit_is_refused() {
    let diags = check_src(
        "<?nvs\n\
         class Api {\n  \
           public string $label = '';\n  \
           #[Core\\Deprecated(replace: '$this->label')]\n  \
           public int $total = 0;\n  \
           #[Core\\Deprecated(replace: '$this->label')]\n  \
           function size(): int { return 1; }\n  \
           #[Core\\Deprecated(replace: '\"ten\"')]\n  \
           const int LIMIT = 10;\n  \
           function find(#[Core\\Deprecated(replace: 'limit: \"ten\"')] int $n = 0, int $limit = 0): int { return $limit; }\n  \
           #[Core\\Deprecated(replace: '$this->find()')]\n  \
           function clear(): void {}\n\
         }\n",
    );
    assert_eq!(
        count(&diags, code::E_DEPRECATION_TEMPLATE_DOES_NOT_FIT),
        4,
        "{diags:?}"
    );
    assert_eq!(diags.error_count(), 4, "{diags:?}");
}

/// A template replaces code written outside the class, so it uses nothing
/// less visible than the member it replaces.
#[test]
fn a_deprecation_template_naming_a_less_visible_member_is_refused() {
    let diags = check_src(
        "<?nvs\n\
         class Api {\n  \
           private int $secret = 0;\n  \
           #[Core\\Deprecated(replace: '$this->helper()')]\n  \
           public function a(): int { return 1; }\n  \
           #[Core\\Deprecated(replace: '$this->secret')]\n  \
           public int $total = 0;\n  \
           #[Core\\Deprecated(replace: '$this->helper()')]\n  \
           protected function b(): int { return 1; }\n  \
           #[Core\\Deprecated(replace: '$this->shared()')]\n  \
           public function c(): int { return 1; }\n  \
           #[Core\\Deprecated(replace: '$this->helper()')]\n  \
           private function d(): int { return 1; }\n  \
           #[Core\\Deprecated(replace: '$this->shared()')]\n  \
           protected function e(): int { return 1; }\n  \
           private function helper(): int { return 2; }\n  \
           protected function shared(): int { return 2; }\n\
         }\n",
    );
    assert_eq!(
        count(&diags, code::E_DEPRECATION_TEMPLATE_LESS_VISIBLE),
        4,
        "{diags:?}"
    );
    assert_eq!(diags.error_count(), 4, "{diags:?}");
}

/// A template that uses a deprecated method, property, constant, enum case or
/// class is refused, so one fix never leaves a second warning.
#[test]
fn a_deprecation_template_naming_a_deprecated_member_is_refused() {
    let diags = check_src(
        "<?nvs\n\
         enum Status {\n  \
           #[Core\\Deprecated(replace: 'Status::Banned')]\n  \
           Active,\n  \
           #[Core\\Deprecated(replace: 'Status::Active')]\n  \
           Blocked,\n  \
           Banned\n\
         }\n\
         #[Core\\Deprecated]\n\
         class Legacy { static function make(): int { return 1; } }\n\
         class Api {\n  \
           #[Core\\Deprecated]\n  \
           public int $old = 0;\n  \
           #[Core\\Deprecated]\n  \
           const int OLD = 1;\n  \
           #[Core\\Deprecated(replace: '$this->b()')]\n  \
           function a(): int { return 1; }\n  \
           #[Core\\Deprecated(replace: '$this->old')]\n  \
           function b(): int { return 1; }\n  \
           #[Core\\Deprecated(replace: 'self::OLD')]\n  \
           function c(): int { return 1; }\n  \
           #[Core\\Deprecated(replace: 'Legacy::make()')]\n  \
           function d(): int { return 1; }\n\
         }\n\
         #[Core\\Deprecated(replace: 'Legacy')]\n\
         class Blog {}\n",
    );
    assert_eq!(
        count(&diags, code::E_DEPRECATION_TEMPLATE_NAMES_A_DEPRECATED),
        6,
        "{diags:?}"
    );
    assert_eq!(diags.error_count(), 6, "{diags:?}");
}

/// The class a deprecated class's `replace` names declares every public
/// method, property and constant the deprecated class declares, public and
/// with a type that fits.
#[test]
fn a_class_replacement_missing_a_public_member_is_refused() {
    let diags = check_src(
        "<?nvs\n\
         class Fresh {\n  \
           public int $total = 0;\n  \
           function size(): string { return ''; }\n\
         }\n\
         class Full {\n  \
           const int LIMIT = 1;\n  \
           public int $total = 0;\n  \
           private int $hidden = 0;\n  \
           function size(): int { return 1; }\n  \
           function count(): int { return 1; }\n\
         }\n\
         #[Core\\Deprecated(replace: 'Fresh')]\n\
         class Old {\n  \
           const int LIMIT = 1;\n  \
           public int $total = 0;\n  \
           private int $hidden = 0;\n  \
           function size(): int { return 1; }\n  \
           function count(): int { return 1; }\n  \
           private function helper(): int { return 1; }\n\
         }\n\
         #[Core\\Deprecated(replace: 'Full')]\n\
         class Older {\n  \
           const int LIMIT = 1;\n  \
           public int $total = 0;\n  \
           function size(): int { return 1; }\n  \
           private function helper(): int { return 1; }\n\
         }\n",
    );
    assert_eq!(
        count(&diags, code::E_DEPRECATION_REPLACEMENT_MISSES_A_MEMBER),
        1,
        "{diags:?}"
    );
    assert_eq!(diags.error_count(), 1, "{diags:?}");
    let message = &diags
        .iter()
        .find(|d| d.code == Some(code::E_DEPRECATION_REPLACEMENT_MISSES_A_MEMBER))
        .expect("one refusal")
        .message;
    for name in ["`LIMIT`", "`size()`", "`count()`"] {
        assert!(message.contains(name), "{message}");
    }
    assert!(
        !message.contains("helper") && !message.contains("total"),
        "{message}"
    );
}

/// The message of every `W1003` among `diags`, in the order they were raised.
fn warnings(diags: &Diagnostics) -> Vec<&str> {
    diags
        .iter()
        .filter(|d| d.code == Some(code::W_DEPRECATED))
        .map(|d| d.message.as_str())
        .collect()
}

/// A call, a static call, a `new`, a property read and write, a class
/// constant and an enum case each warn once, and a use of a member that is
/// not deprecated does not.
#[test]
fn every_use_of_a_deprecated_member_is_w1003() {
    let diags = check_src(
        "<?nvs\n\
         class Store {\n  \
           #[Core\\Deprecated(since: '2.0')]\n  \
           const int LIMIT = 10;\n  \
           const int MAX = 20;\n  \
           #[Core\\Deprecated]\n  \
           public int $total = 0;\n  \
           #[Core\\Deprecated]\n  \
           function constructor() {}\n  \
           #[Core\\Deprecated]\n  \
           function size(): int { return 1; }\n  \
           function count(): int { return 1; }\n  \
           #[Core\\Deprecated]\n  \
           static function make(): int { return 1; }\n\
         }\n\
         enum Status { #[Core\\Deprecated] Active, Banned }\n\
         Store $s = new Store();\n\
         echo $s->size(), $s->count();\n\
         echo $s->total;\n\
         $s->total = 5;\n\
         echo Store::LIMIT, Store::MAX;\n\
         echo Store::make();\n\
         Status $a = Status::Active;\n\
         Status $b = Status::Banned;\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
    let found = warnings(&diags);
    assert_eq!(found.len(), 7, "{found:#?}");
    for name in [
        "`Store::constructor()`",
        "`Store::size()`",
        "`Store::$total`",
        "`Store::LIMIT`",
        "`Store::make()`",
        "`Status::Active`",
    ] {
        assert!(found.iter().any(|m| m.contains(name)), "{name}: {found:#?}");
    }
}

/// A deprecated parameter warns where an argument is passed to it, by
/// position or by name, and not where the call leaves it out.
#[test]
fn a_passed_deprecated_parameter_is_w1003_and_an_omitted_one_is_not() {
    let diags = check_src(
        "<?nvs\n\
         class Shop {\n  \
           function find(int $id, #[Core\\Deprecated(since: '2.0')] int $limit = 10): int {\n    \
             return $id;\n  \
           }\n\
         }\n\
         Shop $s = new Shop();\n\
         echo $s->find(1, 5);\n\
         echo $s->find(1, limit: 5);\n\
         echo $s->find(1);\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
    let found = warnings(&diags);
    assert_eq!(found.len(), 2, "{found:#?}");
    assert!(found.iter().all(|m| m.contains("`$limit`")), "{found:#?}");
}

/// A parameter's type and an `implements` clause that name a deprecated
/// class or interface each warn.
#[test]
fn a_type_position_naming_a_deprecated_class_is_w1003() {
    let diags = check_src(
        "<?nvs\n\
         #[Core\\Deprecated(since: '2.0')]\n\
         class Store {}\n\
         #[Core\\Deprecated]\n\
         interface Sized {}\n\
         class Shop implements Sized {\n  \
           function take(Store $s): int { return 1; }\n\
         }\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
    let found = warnings(&diags);
    assert_eq!(found.len(), 2, "{found:#?}");
    assert!(found.iter().any(|m| m.contains("`Store`")), "{found:#?}");
    assert!(found.iter().any(|m| m.contains("`Sized`")), "{found:#?}");
}

/// An implementation of a deprecated method warns at the implementation,
/// unless the class that implements it is deprecated too.
#[test]
fn an_override_of_a_deprecated_method_is_w1003() {
    let diags = check_src(
        "<?nvs\n\
         interface Sized {\n  \
           #[Core\\Deprecated(since: '2.0')]\n  \
           function size(): int;\n\
         }\n\
         class Box implements Sized {\n  \
           function size(): int { return 1; }\n\
         }\n\
         #[Core\\Deprecated]\n\
         class Crate implements Sized {\n  \
           function size(): int { return 2; }\n\
         }\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
    let found = warnings(&diags);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert!(
        found[0].contains("`Box::size()`") && found[0].contains("`Sized::size()`"),
        "{found:#?}"
    );
}

/// A deprecated method's body and a deprecated class's members use
/// deprecated code without a warning. The same use anywhere else warns.
#[test]
fn a_use_inside_a_deprecated_declaration_does_not_warn() {
    let diags = check_src(
        "<?nvs\n\
         class Shop {\n  \
           #[Core\\Deprecated]\n  \
           function old(): int { return 1; }\n  \
           #[Core\\Deprecated]\n  \
           function older(): int { return $this->old(); }\n  \
           function fresh(): int { return $this->old(); }\n\
         }\n\
         #[Core\\Deprecated]\n\
         class Store {\n  \
           function size(Shop $s): int { return $s->old(); }\n\
         }\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
    assert_eq!(warnings(&diags).len(), 1, "{diags:?}");
}

/// The message names the member, then `since`, then `note`, then the
/// replacement, in that order.
#[test]
fn the_w1003_message_names_since_note_and_the_replacement() {
    let diags = check_src(
        "<?nvs\n\
         class Shop {\n  \
           #[Core\\Deprecated(since: '2.0', note: 'Sizes are counts now.', replace: '$this->count()')]\n  \
           function size(): int { return 1; }\n  \
           function count(): int { return 1; }\n\
         }\n\
         Shop $s = new Shop();\n\
         echo $s->size();\n",
    );
    assert!(!diags.has_errors(), "{diags:?}");
    let found = warnings(&diags);
    assert_eq!(found.len(), 1, "{found:#?}");
    let message = found[0];
    let at = |part: &str| {
        message
            .find(part)
            .unwrap_or_else(|| panic!("{part}: {message}"))
    };
    assert!(at("`Shop::size()`") < at("since 2.0"), "{message}");
    assert!(at("since 2.0") < at("Sizes are counts now."), "{message}");
    assert!(at("Sizes are counts now.") < at("->count()`"), "{message}");
}
