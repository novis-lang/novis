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
//! [ADR 0007](/docs/adr/0007-explicit-type-system.md) § 1 parks
//! user-declared type parameters and
//! [ADR 0053](/docs/adr/0053-iteration-and-generators.md) § 2 opens
//! exactly one door in that wall: a user class may implement a
//! *compiler-owned* generic interface at a concrete type. This table is that
//! door's full extent — a type-argument list on any name not in it is
//! refused (`E_TYPE_ARGS_NOT_GENERIC`).

/// Every compiler-declared global interface, as `(name, type-parameter
/// names)`.
///
/// `Comparable` ([ADR 0013](/docs/adr/0013-comparable-interface.md))
/// and `Stringable`
/// ([ADR 0028](/docs/adr/0028-closing-the-remaining-magic-methods.md)
/// § 1) take none; `Iterable` and `Iterator`
/// ([ADR 0053](/docs/adr/0053-iteration-and-generators.md) § 1) each
/// take one. `PropertyObserver`
/// ([ADR 0014](/docs/adr/0014-property-observer.md) § 2) takes none
/// either: it is a contract an ordinary class implements, not a `Core` domain
/// class, so it belongs on this roster beside `Comparable` rather than under a
/// namespace.
pub const RESERVED: &[(&str, &[&str])] = &[
    ("Comparable", &[]),
    ("Stringable", &[]),
    ("PropertyObserver", &[]),
    ("Iterable", &["T"]),
    ("Iterator", &["T"]),
];

/// ADR 0013's `Comparable` — the one interface `<`, `>`, `<=`, `>=` and `<=>`
/// accept over two objects, via its sole member `compareTo(self $other): int`.
pub const COMPARABLE: &str = "Comparable";

/// ADR 0028 § 1's `Stringable` — what an object owes to be converted to
/// `string`, via its sole member `toString(): string`.
pub const STRINGABLE: &str = "Stringable";

/// ADR 0014 § 2's `PropertyObserver` — what a class implements to be told of
/// every read and write of every property it declares, through
/// `onPropertyGet(string, mixed): void` and its `onPropertySet` twin.
pub const PROPERTY_OBSERVER: &str = "PropertyObserver";

/// ADR 0053 § 1's `Iterable<T>` — a thing that can produce a fresh cursor.
pub const ITERABLE: &str = "Iterable";

/// ADR 0053 § 1's `Iterator<T>` — the single-pass cursor itself.
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
    fn the_three_non_generic_interfaces_take_none() {
        assert_eq!(type_params("Comparable"), Some(&[][..]));
        assert_eq!(type_params("Stringable"), Some(&[][..]));
        assert_eq!(type_params(PROPERTY_OBSERVER), Some(&[][..]));
    }

    #[test]
    fn a_name_that_is_not_reserved_has_no_type_parameters_at_all() {
        assert_eq!(type_params("Animal"), None);
        assert!(!is_reserved_interface("Countable"));
    }
}
