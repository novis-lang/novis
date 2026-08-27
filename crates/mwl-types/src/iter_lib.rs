//! Seeding the checker's signature table with the members of every
//! compiler-declared global interface.
//!
//! [`mwl_hir::interfaces`] is the one home for *which* global interfaces the
//! compiler declares and what type parameters each takes; this is the one
//! place all four — `Comparable`, `Stringable`, `Iterable<T>` and
//! `Iterator<T>` — become member signatures, in exactly the
//! [`ClassSignature`](crate::signatures::ClassSignature) shape
//! [`crate::error_lib`] gives the exception tree and [`crate::core_lib`]
//! gives `Core`. After this point `resolve_method` finds
//! `Iterator::advance` the way it finds `Animal::name`.
//!
//! The module keeps the name it had when ADR 0053 § 1's two iteration
//! interfaces were the only ones seeded here; the two older ones joined them
//! rather than getting a second seeding path, because one roster with two
//! places to look is how the halves drift.
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
//! # `compareTo`'s parameter is `Comparable`, and that is `self` here
//!
//! [ADR 0013](../../../docs/adr/0013-comparable-interface.md) § 1 writes the
//! member as `compareTo(self $other): int`, and `self` in the declaration
//! this seeds *is* `Comparable` — an implementation narrows it to its own
//! class, exactly as the ADR's variance paragraph says. Nothing is lost by
//! the wider seeded spelling: the rule that two operands of an ordering
//! operator must be the *same* class is enforced at the operator instead
//! ([`crate::expr::operators`]), where both operands' static types are in
//! hand, and never by this parameter.
//!
//! # Every member is bodiless, and that is what makes them dispatch
//!
//! No interface on the roster declares a default (ADR 0043 § 2), so
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
//! alongside the iteration pair. That module owns the rule and its
//! exemptions, and it reads this table: seeding `compareTo` and `toString`
//! here is what makes `class Money implements Comparable {}` owe a body,
//! which it always should have.

use mwl_hir::QName;
use mwl_hir::interfaces::{COMPARABLE, ITERABLE, ITERATOR, STRINGABLE};
use rustc_hash::FxHashMap;

use crate::signatures::{MethodSig, SignatureTable};
use crate::ty::{TypeId, TypeInterner};

/// Adds `Comparable`, `Stringable`, `Iterable<T>` and `Iterator<T>` to
/// `table`.
///
/// Called once, alongside [`crate::core_lib::seed`] and
/// [`crate::error_lib::seed`], at the head of
/// [`build_signatures`](crate::signatures::build_signatures).
pub(crate) fn seed(table: &mut SignatureTable, interner: &mut TypeInterner) {
    let comparable = interner.class(QName::parse(COMPARABLE));
    let int_ty = interner.int();
    table.seed_class(
        QName::parse(COMPARABLE),
        FxHashMap::default(),
        [("compareTo".to_owned(), bodiless(vec![comparable], int_ty))]
            .into_iter()
            .collect(),
    );

    let string_ty = interner.string();
    table.seed_class(
        QName::parse(STRINGABLE),
        FxHashMap::default(),
        [("toString".to_owned(), bodiless(Vec::new(), string_ty))]
            .into_iter()
            .collect(),
    );

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
/// *generic* entries are the only ones it is called for, and a
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
        defaults: vec![None; params.len()],
        // Installed from this crate's roster rather than parsed, so no
        // slot has a source name — see `MethodSig::param_names`.
        param_names: None,
        params,
        variadic: false,
        type_params: Vec::new(),
        return_ty,
        is_static: false,
        interface_private: false,
        visibility: mwl_syntax::ast::Visibility::Public,
        has_body: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signatures::resolve_method;
    use mwl_diagnostics::{Diagnostics, SourceMap};
    use mwl_hir::{ClassGraph, resolve_file};
    use mwl_syntax::parse_file;

    fn seeded() -> (SignatureTable, TypeInterner) {
        let mut table = SignatureTable::new();
        let mut interner = TypeInterner::new();
        seed(&mut table, &mut interner);
        (table, interner)
    }

    fn check_src(src: &str) -> Diagnostics {
        let mut map = SourceMap::new();
        let file = map.add("t.mwl", src);
        let mut diags = Diagnostics::new();
        let stmts = parse_file(map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to parse: {diags:?}");
        let module = resolve_file(&stmts, map.file(file), &mut diags);
        assert!(!diags.has_errors(), "fixture failed to resolve: {diags:?}");
        let mut interner = TypeInterner::new();
        let mut exprs = crate::expr_table::ExprTypeTable::new();
        crate::check::check_program(
            &[crate::ProgramFile {
                src: map.file(file),
                stmts: &stmts,
            }],
            &module,
            &mut interner,
            &mut exprs,
            &mut diags,
        );
        diags
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

    /// ADR 0013 § 1's `compareTo(self $other): int`, with `self` seeded as
    /// `Comparable` — see this module's docs for why the wider spelling costs
    /// nothing.
    #[test]
    fn comparable_declares_adr_0013_s_ordering_member() {
        let (table, interner) = seeded();
        let graph = ClassGraph::default();
        let (owner, sig) = resolve_method(&QName::parse(COMPARABLE), "compareTo", &table, &graph)
            .expect("compareTo");
        assert_eq!(owner.to_string(), COMPARABLE);
        assert_eq!(interner.describe(sig.return_ty), "int");
        let [param] = sig.params[..] else {
            panic!(
                "`compareTo` takes exactly one parameter, got {:?}",
                sig.params
            )
        };
        assert_eq!(interner.describe(param), COMPARABLE);
    }

    /// ADR 0028 § 1's `toString(): string`.
    #[test]
    fn stringable_declares_adr_0028_s_conversion_member() {
        let (table, interner) = seeded();
        let graph = ClassGraph::default();
        let (owner, sig) = resolve_method(&QName::parse(STRINGABLE), "toString", &table, &graph)
            .expect("toString");
        assert_eq!(owner.to_string(), STRINGABLE);
        assert_eq!(interner.describe(sig.return_ty), "string");
        assert!(sig.params.is_empty());
    }

    /// The gap this closes: a parameter declared at the interface type had no
    /// member to resolve, so the call was `E0405` on a member the interface
    /// plainly declares.
    #[test]
    fn a_stringable_parameter_can_call_to_string() {
        let diags = check_src(
            "<?mwl\n\
             class Render {\n\
             public function label(Stringable $s): string { return $s->toString(); }\n\
             }\n",
        );
        assert!(!diags.has_errors(), "{diags:?}");
    }

    #[test]
    fn no_reserved_interface_declares_a_default_body() {
        let (table, _) = seeded();
        for (interface, member) in [
            (COMPARABLE, "compareTo"),
            (STRINGABLE, "toString"),
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
