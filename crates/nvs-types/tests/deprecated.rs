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
         #[Core\\Deprecated(since: '2.0', note: 'Use Shop.', replace: 'Shop', construct: 'new Shop()')]\n\
         class Store {\n  \
           #[Core\\Deprecated(since: '2.0')]\n  \
           const int LIMIT = 10;\n  \
           #[Core\\Deprecated(note: 'Read `count`.')]\n  \
           public int $total = 0;\n  \
           #[Core\\Deprecated(since: '2.0', note: 'Pass a limit.', replace: '$this->total')]\n  \
           function constructor(#[Core\\Deprecated(since: '2.0')] int $limit = 0) {}\n  \
           #[Core\\Deprecated(replace: '$this->total')]\n  \
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
