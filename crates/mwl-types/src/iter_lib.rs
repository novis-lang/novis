//! Seeding the checker's signature table with ADR 0053 § 1's two iteration
//! interfaces.
//!
//! [`mwl_hir::interfaces`] is the one home for *which* global interfaces the
//! compiler declares and what type parameters each takes; this is the one
//! place `Iterable<T>` and `Iterator<T>` become member signatures, in exactly
//! the [`ClassSignature`](crate::signatures::ClassSignature) shape
//! [`crate::error_lib`] gives the exception tree and [`crate::core_lib`]
//! gives `Core`. After this point `resolve_method` finds
//! `Iterator::advance` the way it finds `Animal::name`.
//!
//! # Why the members are written here and not in `mwl-hir`
//!
//! `current(): T` needs a [`TypeId`] for `T`, and `iterate(): Iterator<T>`
//! needs one for a generic class applied to it — neither exists outside this
//! crate's interner. The parameter *names* are read back off
//! [`mwl_hir::interfaces::type_params`] rather than spelled again here, so
//! the two halves cannot drift: rename `T` there and every signature below
//! follows.
//!
//! # Every member is bodiless, and that is what makes them dispatch
//!
//! Neither interface declares a default (ADR 0043 § 2), so
//! [`MethodSig::has_body`] is false throughout. That is not bookkeeping: a
//! call resolving to a bodiless declaration has no compiled function to name,
//! so it dispatches on the receiver's runtime class — which is precisely what
//! driving a cursor whose concrete class the loop never knows requires.
//!
//! # Conformance is checked, and this is what asked for it
//!
//! A class claiming `implements Iterator<int>` and forgetting `advance` would
//! end in a dispatch to nothing rather than in a call the author wrote by
//! hand — which is why [`crate::conformance`] exists and why it landed
//! alongside these two. That module owns the rule and its three exemptions;
//! `Comparable`/`Stringable` still require nothing there, for the reason it
//! gives.

use mwl_hir::QName;
use mwl_hir::interfaces::{ITERABLE, ITERATOR};
use rustc_hash::FxHashMap;

use crate::signatures::{MethodSig, SignatureTable};
use crate::ty::{TypeId, TypeInterner};

/// Adds `Iterable<T>` and `Iterator<T>` to `table`.
///
/// Called once, alongside [`crate::core_lib::seed`] and
/// [`crate::error_lib::seed`], at the head of
/// [`build_signatures`](crate::signatures::build_signatures).
pub(crate) fn seed(table: &mut SignatureTable, interner: &mut TypeInterner) {
    let iterator_elem = elem_var(ITERATOR, interner);
    let bool_ty = interner.bool_ty();
    table.seed_class(
        QName::parse(ITERATOR),
        FxHashMap::default(),
        [
            ("advance".to_owned(), bodiless(Vec::new(), bool_ty)),
            ("current".to_owned(), bodiless(Vec::new(), iterator_elem)),
        ]
        .into_iter()
        .collect(),
    );

    let iterable_elem = elem_var(ITERABLE, interner);
    let cursor = interner.generic_class(QName::parse(ITERATOR), vec![iterable_elem]);
    table.seed_class(
        QName::parse(ITERABLE),
        FxHashMap::default(),
        [("iterate".to_owned(), bodiless(Vec::new(), cursor))]
            .into_iter()
            .collect(),
    );
}

/// The interned type variable standing for `interface`'s sole type parameter.
///
/// Panics if the roster gives it anything other than exactly one — the two
/// entries this module seeds are the only ones it is called for, and a
/// silently-wrong element type would be far worse than a build that stops.
fn elem_var(interface: &str, interner: &mut TypeInterner) -> TypeId {
    let params = mwl_hir::interfaces::type_params(interface)
        .unwrap_or_else(|| panic!("`{interface}` is not on the reserved-interface roster"));
    let [name] = params else {
        panic!("`{interface}` should declare exactly one type parameter, got {params:?}")
    };
    interner.type_var(*name)
}

/// One interface method declared without a default — see this module's docs
/// for why every member here is one.
fn bodiless(params: Vec<TypeId>, return_ty: TypeId) -> MethodSig {
    MethodSig {
        by_ref: vec![false; params.len()],
        params,
        variadic: false,
        return_ty,
        is_static: false,
        interface_private: false,
        has_body: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signatures::resolve_method;
    use mwl_hir::ClassGraph;

    fn seeded() -> (SignatureTable, TypeInterner) {
        let mut table = SignatureTable::new();
        let mut interner = TypeInterner::new();
        seed(&mut table, &mut interner);
        (table, interner)
    }

    #[test]
    fn the_cursor_declares_adr_0053_s_two_members() {
        let (table, interner) = seeded();
        let graph = ClassGraph::default();
        let (owner, advance) =
            resolve_method(&QName::parse(ITERATOR), "advance", &table, &graph).expect("advance");
        assert_eq!(owner.to_string(), ITERATOR);
        assert_eq!(interner.describe(advance.return_ty), "bool");
        assert!(advance.params.is_empty());

        let (_, current) =
            resolve_method(&QName::parse(ITERATOR), "current", &table, &graph).expect("current");
        assert!(current.is_generic(&interner));
    }

    #[test]
    fn iterate_returns_a_cursor_over_the_same_element_type() {
        let (table, interner) = seeded();
        let graph = ClassGraph::default();
        let (_, sig) =
            resolve_method(&QName::parse(ITERABLE), "iterate", &table, &graph).expect("iterate");
        assert_eq!(interner.describe(sig.return_ty), "Iterator<T>");
        assert!(sig.is_generic(&interner));
    }

    #[test]
    fn neither_interface_declares_a_default_body() {
        let (table, _) = seeded();
        for (interface, member) in [
            (ITERATOR, "advance"),
            (ITERATOR, "current"),
            (ITERABLE, "iterate"),
        ] {
            let sig = &table.get(&QName::parse(interface)).expect("seeded").methods[member];
            assert!(!sig.has_body, "`{interface}::{member}` should be bodiless");
            assert!(!sig.is_static);
        }
    }
}
