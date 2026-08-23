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
    fn php_s_exception_and_error_are_not_in_the_tree() {
        assert!(!is_exception_class("Exception"));
        assert!(!is_exception_class("Error"));
    }
}
