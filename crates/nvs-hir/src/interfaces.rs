//! The closed set of global interfaces the compiler declares for every
//! program, and how many type parameters each takes.
//!
//! Same shape, and same reason, as [`crate::errors`]' exception tree: none of
//! these has a source declaration anywhere, so every consumer that needs one
//! to exist seeds itself from [`RESERVED`] rather than from a `.nvs` file.
//! [`QName::is_reserved_global_interface`](crate::QName::is_reserved_global_interface)
//! is the reachability question; `nvs_types::iter_lib` turns every entry here
//! into the member signatures the checker resolves against.
//!
//! # Why the type parameters live here and the members do not
//!
//! A type-parameter list is a property of the *name* — it decides whether
//! `implements Iterable<int>` is well-formed at all, which
//! `nvs_types::lower` must answer while it is still building the signature
//! table and cannot consult it. A member set is a property of the
//! *declaration*, and needs `TypeId`s this crate has no interner for. So the
//! name and its parameters are one fact with one home, here; the members are
//! a second fact with a second home, in `nvs_types::iter_lib`, which reads
//! its parameters back off this table rather than restating them.
//!
//! # These are the only generics user code can name
//!
//! `rule:types/declaration` parks
//! user-declared type parameters and
//! `rule:iteration/concrete-generic-implements` opens
//! exactly one door in that wall: a user class may implement a
//! *compiler-owned* generic interface at a concrete type. This table is that
//! door's full extent — a type-argument list on any name not in it is
//! refused (`E_TYPE_ARGS_NOT_GENERIC`).

/// Every compiler-declared global interface, as `(name, type-parameter
/// names)`.
///
/// `Comparable` (`rule:classes/comparable`),
/// `Stringable`
/// (`rule:classes/stringable`) and `Parses` take none; `Iterable` and
/// `Iterator` (`rule:iteration/two-interfaces`) each
/// take one. `PropertyObserver`
/// (`rule:classes/property-observer`) takes none
/// either: it is a contract an ordinary class implements, not a `Core` domain
/// class, so it belongs on this roster beside `Comparable` rather than under a
/// namespace.
pub const RESERVED: &[(&str, &[&str])] = &[
    ("Comparable", &[]),
    ("Stringable", &[]),
    ("Parses", &[]),
    ("PropertyObserver", &[]),
    ("Iterable", &["T"]),
    ("Iterator", &["T"]),
];

/// `rule:classes/comparable`'s `Comparable` — the one interface `<`, `>`, `<=`, `>=` and `<=>`
/// accept over two objects, via its sole member `compareTo(self $other): int`.
pub const COMPARABLE: &str = "Comparable";

/// `rule:classes/stringable`'s `Stringable` — what an object owes to be converted to
/// `string`, via its sole member `toString(): string`.
pub const STRINGABLE: &str = "Stringable";

/// `Parses` — what a class owes to be built from a piece of text, via its one
/// required member `parse(tainted string $s): static`. Its `tryParse` twin is
/// a default body on the interface rather than a second required member, so
/// `rule:expressions/try-parse`'s single implementation of "is this text a
/// `T`" stays single.
///
/// That default is not reachable *through* an implementor: `Slug::tryParse($s)`
/// is `E0309`, because a reserved interface is declared here rather than in
/// source and its default body has no compiled function to dispatch to. A class
/// implementing `Parses` therefore declares `parse` alone and satisfies the
/// interface, and reaches the non-throwing question by declaring its own
/// `tryParse` or by catching the throw.
///
/// This is what a binding site asks a class for instead of naming one: a route
/// capture, a `#[Query]`, a command argument and a command option all build a
/// value out of text that arrived from outside the process, and all of them
/// are laundered by the conversion rather than by the class
/// (`rule:security/route-capture-is-laundered-by-its-type`).
pub const PARSES: &str = "Parses";

/// `rule:classes/property-observer`'s `PropertyObserver` — what a class implements to be told of
/// every read and write of every property it declares, through
/// `onPropertyGet(string, mixed): void` and its `onPropertySet` twin.
pub const PROPERTY_OBSERVER: &str = "PropertyObserver";

/// `rule:iteration/two-interfaces`'s `Iterable<T>` — a thing that can produce a fresh cursor.
pub const ITERABLE: &str = "Iterable";

/// `rule:iteration/two-interfaces`'s `Iterator<T>` — the single-pass cursor itself.
pub const ITERATOR: &str = "Iterator";

/// Whether `name` is one of [`RESERVED`]'s entries.
#[must_use]
pub fn is_reserved_interface(name: &str) -> bool {
    RESERVED.iter().any(|(entry, _)| *entry == name)
}

/// The type parameters `name` declares, in order — `None` when it is not a
/// reserved interface at all. A non-generic entry answers `Some(&[])`, which
/// is what makes "wrote `Comparable<int>`" and "wrote `Foo<int>`" two
/// different diagnostics.
///
/// The names are load-bearing, not decoration: `nvs_types::iter_lib` writes
/// each member's signature against them, and a call on a receiver typed
/// `Iterator<int>` binds them positionally against the receiver's own
/// arguments to substitute that signature concrete.
#[must_use]
pub fn type_params(name: &str) -> Option<&'static [&'static str]> {
    RESERVED
        .iter()
        .find(|(entry, _)| *entry == name)
        .map(|(_, params)| *params)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_iteration_interfaces_take_one_type_parameter_each() {
        assert_eq!(type_params(ITERABLE), Some(&["T"][..]));
        assert_eq!(type_params(ITERATOR), Some(&["T"][..]));
    }

    #[test]
    fn the_non_generic_interfaces_take_none() {
        assert_eq!(type_params("Comparable"), Some(&[][..]));
        assert_eq!(type_params("Stringable"), Some(&[][..]));
        assert_eq!(type_params(PROPERTY_OBSERVER), Some(&[][..]));
    }

    /// `Parses` is asked of a class by name at a binding site and never
    /// applied to a type argument: the text an implementor is built from is
    /// `string` at every one of those sites, so there is nothing for a
    /// parameter to stand for.
    #[test]
    fn parses_is_a_reserved_interface_taking_no_type_arguments() {
        assert!(is_reserved_interface(PARSES));
        assert_eq!(type_params(PARSES), Some(&[][..]));
    }

    #[test]
    fn a_name_that_is_not_reserved_has_no_type_parameters_at_all() {
        assert_eq!(type_params("Animal"), None);
        assert!(!is_reserved_interface("Countable"));
    }
}
