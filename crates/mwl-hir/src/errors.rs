//! The closed exception tree the compiler declares for every program.
//!
//! [docs/spec/01-core-library.md](../../../docs/spec/01-core-library.md) § 10
//! is authoritative for the tree's shape and its members; this module is the
//! one place that shape becomes data the rest of the compiler can read.
//! [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md) § 1 points
//! at § 10 rather than restating it, so nothing else here needs to.
//!
//! # Why a table rather than a written declaration
//!
//! `Throwable` cannot be written in MWL: its `backtrace` is grown by the
//! runtime as a throw propagates (`mwl_runtime::throwable`), and a source
//! declaration would need a body that has no legal spelling. Every consumer
//! that needs a class to exist therefore seeds itself from [`TREE`] —
//! `crate::hierarchy` for the `extends` links, `mwl_types::error_lib` for the
//! property/constructor signatures, `mwl_types::layout` for the slot order,
//! and `mwl_ir::lower` for the one synthesized constructor body.
//!
//! # There is no `Exception` and no `Error`
//!
//! Both are gone, and deliberately: § 10 makes `Throwable` the root that user
//! classes extend directly, so a second root-shaped name would be a second
//! way to spell the same thing ([ADR 0063](../../../docs/adr/0063-core-api-conventions.md)
//! R20). A program naming either gets an ordinary undeclared-class
//! diagnostic.

/// Every exception class, root first, as `(name, parent)`.
///
/// Ordered parent-before-child so a consumer building a flattened supertype
/// set can walk it in one pass.
pub const TREE: &[(&str, Option<&str>)] = &[
    ("Throwable", None),
    ("LogicError", Some("Throwable")),
    ("RuntimeError", Some("Throwable")),
    ("IOError", Some("RuntimeError")),
    ("ParseError", Some("RuntimeError")),
    ("TimeoutError", Some("RuntimeError")),
    ("ArithmeticError", Some("Throwable")),
];

/// The root every other entry in [`TREE`] descends from, and the one name a
/// `catch` clause can use to mean "anything at all".
pub const ROOT: &str = "Throwable";

/// `Throwable`'s own instance properties, in slot order.
///
/// Slot order is load-bearing twice over: `mwl_runtime::object` lays a
/// subclass's slots out *after* its parent's, so these four indices are the
/// same for every exception class in existence — which is what lets
/// `mwl_runtime::throwable` reach `backtrace` on a value it knows nothing
/// else about.
pub const PROPERTIES: &[&str] = &["message", "previous", "backtrace", "location"];

/// The slot [`PROPERTIES`] puts `message` in.
pub const MESSAGE_SLOT: usize = 0;

/// The slot [`PROPERTIES`] puts `backtrace` in — the one the runtime appends
/// a frame label to as a throw propagates.
pub const BACKTRACE_SLOT: usize = 2;

/// Every [`TREE`] entry that declares instance properties **of its own**, in
/// slot order, keyed by class name.
///
/// [`PROPERTIES`] is the root's row; the rest of the tree inherits those four
/// and, with one exception, adds nothing. That exception is `ParseError`,
/// which [ADR 0071](../../../docs/adr/0071-derived-codecs.md) § 5 gives an
/// `issues` list so that a decode reports **every** bad field from one throw
/// rather than the first.
///
/// It is declared on `ParseError` rather than on the root deliberately: the
/// root is allocated by every `throw` in every program, and a fifth slot there
/// would cost sixteen bytes plus one empty-array allocation on a path that
/// PHP-shaped code takes for ordinary control flow
/// ([ADR 0002](../../../docs/adr/0002-error-propagation.md)'s measured cost).
/// `Core\Db\DbError` gains the same property when M8 adds it to [`TREE`].
pub const OWN_PROPERTIES: &[(&str, &[&str])] = &[(ROOT, PROPERTIES), ("ParseError", ISSUES)];

/// `ParseError`'s own row of [`OWN_PROPERTIES`].
const ISSUES: &[&str] = &["issues"];

/// The slot `ParseError::$issues` occupies.
///
/// `ParseError` descends from the root through `RuntimeError`, and neither
/// declares anything of its own, so its first own slot sits immediately after
/// [`PROPERTIES`] — `parse_error_s_own_slots_start_after_the_root_s` is what
/// holds that rather than a comment.
pub const ISSUES_SLOT: usize = PROPERTIES.len();

/// `name`'s own instance properties, in slot order — empty for a class that
/// declares none, and for a name that is not in [`TREE`] at all.
///
/// The one reader that matters is `mwl_types::layout`, which appends these
/// after every ancestor's; `mwl_types::error_lib` seeds the same list as
/// signatures.
#[must_use]
pub fn own_properties(name: &str) -> &'static [&'static str] {
    OWN_PROPERTIES
        .iter()
        .find(|(entry, _)| *entry == name)
        .map_or(&[], |(_, properties)| *properties)
}

/// Whether `name` declares a synthesized constructor of its own.
///
/// Exactly the classes with own properties: a constructor exists to assign
/// them ([ADR 0022](../../../docs/adr/0022-definite-property-initialization.md)),
/// so a class that adds none inherits its parent's and needs no second one.
/// `mwl_ir::lower::exception` is what actually builds each body.
#[must_use]
pub fn declares_constructor(name: &str) -> bool {
    !own_properties(name).is_empty()
}

/// Whether `name` is one of [`TREE`]'s entries, spelled as a single global
/// segment.
#[must_use]
pub fn is_exception_class(name: &str) -> bool {
    TREE.iter().any(|(entry, _)| *entry == name)
}

/// Every class `name` also *is*, transitively, excluding itself — `None` if
/// `name` is not in [`TREE`] at all.
#[must_use]
pub fn conforms_to(name: &str) -> Option<Vec<&'static str>> {
    let mut current = TREE.iter().find(|(entry, _)| *entry == name)?.1;
    let mut out = Vec::new();
    while let Some(parent) = current {
        out.push(parent);
        current = TREE
            .iter()
            .find(|(entry, _)| *entry == parent)
            .and_then(|(_, up)| *up);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_entry_but_the_root_has_a_parent_that_is_itself_an_entry() {
        for (name, parent) in TREE {
            match parent {
                None => assert_eq!(*name, ROOT),
                Some(parent) => assert!(is_exception_class(parent), "{parent} is not in the tree"),
            }
        }
    }

    #[test]
    fn a_leaf_conforms_to_every_class_above_it() {
        assert_eq!(
            conforms_to("TimeoutError"),
            Some(vec!["RuntimeError", "Throwable"])
        );
        assert_eq!(conforms_to("LogicError"), Some(vec!["Throwable"]));
        assert_eq!(conforms_to("Throwable"), Some(Vec::new()));
        assert_eq!(conforms_to("Animal"), None);
    }

    #[test]
    fn parse_error_s_own_slots_start_after_the_root_s() {
        // `ISSUES_SLOT` is a constant three crates restate, so what it depends
        // on is checked rather than remembered: nothing between `ParseError`
        // and the root contributes a slot.
        let above = conforms_to("ParseError").expect("ParseError is in the tree");
        let inherited: usize = above.iter().map(|name| own_properties(name).len()).sum();
        assert_eq!(inherited, ISSUES_SLOT);
        assert_eq!(own_properties("ParseError"), &["issues"]);
    }

    #[test]
    fn only_the_root_and_parse_error_declare_anything_of_their_own() {
        for (name, _) in TREE {
            let expected = *name == ROOT || *name == "ParseError";
            assert_eq!(declares_constructor(name), expected, "{name}");
        }
        for (name, _) in OWN_PROPERTIES {
            assert!(is_exception_class(name), "{name} is not in the tree");
        }
    }

    #[test]
    fn php_s_exception_and_error_are_not_in_the_tree() {
        assert!(!is_exception_class("Exception"));
        assert!(!is_exception_class("Error"));
    }
}
