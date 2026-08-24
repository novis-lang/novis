//! Seeding the checker's signature table with the exception tree.
//!
//! [`mwl_hir::errors`] is the one home for the tree's shape; this is the one
//! place that turns it into the same
//! [`ClassSignature`](crate::signatures::ClassSignature) a user-declared class
//! produces — exactly the shape [`crate::core_lib`] already gives `Core`, and
//! for the same reason: after this point `resolve_property` finds
//! `$e->message` the way it finds `$animal->legs`, and `resolve_method` finds
//! `Throwable::constructor` the way it finds any inherited one.
//!
//! # Members are properties, not accessors
//!
//! [docs/spec/01-core-library.md](../../../docs/spec/01-core-library.md) § 10
//! is authoritative: `$e->message`, `$e->previous`, `$e->backtrace`,
//! `$e->location`. There is no `getMessage()`/`getTraceAsString()` to seed,
//! and nothing here declares one.
//!
//! # `ParseError` is the one class with state of its own
//!
//! [ADR 0071](../../../docs/adr/0071-derived-codecs.md) § 5's
//! `issues: array<Core\Issue>`, seeded from
//! [`mwl_hir::errors::OWN_PROPERTIES`] rather than named here, so that the
//! slot order and the signature cannot disagree. `Core\Issue` is an ADR 0036
//! **shape**, `{path: string, message: string}` — see [`issue_shape`] — so a
//! class of that name exists nowhere and a decoder builds one with nothing
//! declared.
//!
//! A class that declares properties also declares a constructor to assign
//! them, which is why the root is no longer the only entry with methods; the
//! bodies are `mwl_ir::lower::exception`'s.
//!
//! # Known gaps
//!
//! * **`previous` can never be set.** Its slot exists (the slot *order* is
//!   load-bearing — see [`mwl_hir::errors::PROPERTIES`]), and reading it
//!   yields `null`, but the constructor takes only a message: an optional
//!   parameter has no model in [`crate::signatures::MethodSig`] at all
//!   (`params` plus a `variadic` flag, with the arity check demanding an
//!   exact count), so a second parameter would make every one-argument
//!   `new LogicError("…")` a diagnostic. Widening the signature model is the
//!   prerequisite, not a change here.
//! * **An `issues` entry cannot be *read* yet.** The list is built, counted
//!   and iterated like any array, but `$issue->path` is a property access on
//!   an ADR 0036 § 4 shape receiver, which `mwl-ir` does not lower — its own
//!   gap, and the reason this crate records no `ExprInfo::Property` for one.
//! * **`location` is the throw site, not the construction site.** The
//!   synthesized constructor leaves it empty and `mwl_ir::lower`'s
//!   `write_throw_location` fills it at the `throw` — which is the choice that
//!   agrees with the backtrace beside it, since that holds the frames the
//!   exception unwound *out of*. An exception constructed and never thrown
//!   therefore reads `""`.
//!
//! A user subclass that declares its own constructor and does not chain to
//! `parent::constructor(…)` is already refused, by the same check every other
//! `extends` gets — nothing exception-specific is needed for that.

use mwl_hir::QName;
use mwl_hir::errors::{PROPERTIES, ROOT, TREE};
use rustc_hash::FxHashMap;

use crate::signatures::{MethodSig, SignatureTable};
use crate::ty::{TypeId, TypeInterner};

/// Adds every [`mwl_hir::errors::TREE`] entry to `table`.
///
/// Called once, alongside [`crate::core_lib::seed`], at the head of
/// [`build_signatures`](crate::signatures::build_signatures).
pub(crate) fn seed(table: &mut SignatureTable, interner: &mut TypeInterner) {
    let root = QName::parse(ROOT);
    for (name, _) in TREE {
        let qname = QName::parse(name);
        // A class that declares no state of its own inherits both its
        // properties and its constructor through the graph
        // `mwl_hir::hierarchy` seeded — the same walk a user subclass goes
        // through. The root and `ParseError` are the two that do declare
        // some; `mwl_hir::errors::OWN_PROPERTIES` is that roster's home.
        let properties = if qname == root {
            root_properties(interner)
        } else {
            own_properties(name, interner)
        };
        let methods = if mwl_hir::errors::declares_constructor(name) {
            constructor(interner)
        } else {
            FxHashMap::default()
        };
        table.seed_class(qname, properties, methods);
    }
}

/// `name`'s own properties beyond the root's, typed —
/// [`mwl_hir::errors::OWN_PROPERTIES`]'s non-root rows.
fn own_properties(name: &str, interner: &mut TypeInterner) -> FxHashMap<String, TypeId> {
    mwl_hir::errors::own_properties(name)
        .iter()
        .map(|property| {
            let ty = match *property {
                "issues" => {
                    let issues = issue_shape(interner);
                    interner.array(issues)
                }
                other => panic!("no type seeded for `{name}::{other}`"),
            };
            ((*property).to_owned(), ty)
        })
        .collect()
}

/// `type Core\Issue = {path: string, message: string}` —
/// [ADR 0071](../../../docs/adr/0071-derived-codecs.md) § 5's one shape.
///
/// An ADR 0036 shape rather than a class, which is what that ADR writes and
/// what lets a decoder build one with no declaration anywhere: the value is an
/// anonymous methodless instance, and `mwl_stdlib::issue` is what builds it.
/// [`TypeInterner::shape`] canonicalizes the field order, so the runtime slot
/// order is the *sorted* one — `message`, then `path`.
fn issue_shape(interner: &mut TypeInterner) -> TypeId {
    let string = interner.string();
    interner.shape(vec![
        ("path".to_owned(), string),
        ("message".to_owned(), string),
    ])
}

/// `Throwable`'s four readonly properties, spec § 10's list.
fn root_properties(interner: &mut TypeInterner) -> FxHashMap<String, TypeId> {
    let string = interner.string();
    let throwable = interner.class(QName::parse(ROOT));
    let null = interner.null();
    let previous = interner.make_union([throwable, null]);
    let backtrace = interner.array(string);
    let by_name: FxHashMap<&str, TypeId> = [
        ("message", string),
        ("previous", previous),
        ("backtrace", backtrace),
        ("location", string),
    ]
    .into_iter()
    .collect();
    // Driven off `PROPERTIES` rather than the literal above, so a name added
    // there without a type here fails loudly instead of silently vanishing
    // from the checker while still occupying a slot.
    PROPERTIES
        .iter()
        .map(|name| {
            let ty = by_name
                .get(name)
                .copied()
                .unwrap_or_else(|| panic!("no type seeded for `Throwable::{name}`"));
            ((*name).to_owned(), ty)
        })
        .collect()
}

/// `constructor(string $message)` — see this module's known gaps for why
/// `previous` is not a second parameter.
///
/// The same signature for every class that declares one: a subclass's
/// synthesized body differs only in which slots it fills, never in what a
/// `new` writes at the call site.
fn constructor(interner: &mut TypeInterner) -> FxHashMap<String, MethodSig> {
    let string = interner.string();
    let void = interner.void();
    [(
        "constructor".to_owned(),
        MethodSig {
            params: vec![string],
            by_ref: vec![false],
            variadic: false,
            defaults: vec![None],
            type_params: Vec::new(),
            return_ty: void,
            is_static: false,
            interface_private: false,
            // Synthesized by `mwl_ir::lower`, which is still a body.
            has_body: true,
        },
    )]
    .into_iter()
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signatures::{resolve_method, resolve_property};
    use mwl_hir::ClassGraph;

    /// The very graph `mwl_hir::hierarchy` seeds into every real compilation,
    /// so these tests exercise the shipped links rather than a copy of them.
    fn tree_graph() -> ClassGraph {
        let mut graph = ClassGraph::default();
        mwl_hir::seed_exception_tree(&mut graph);
        graph
    }

    #[test]
    fn the_root_declares_every_property_the_spec_lists() {
        let mut interner = TypeInterner::new();
        let mut table = SignatureTable::new();
        seed(&mut table, &mut interner);
        let root = table.get(&QName::parse(ROOT)).expect("Throwable is seeded");
        for name in PROPERTIES {
            assert!(
                root.properties.contains_key(*name),
                "Throwable should declare `{name}`"
            );
        }
        assert_eq!(root.properties.len(), PROPERTIES.len());
    }

    #[test]
    fn a_leaf_inherits_the_root_s_properties_and_constructor() {
        let mut interner = TypeInterner::new();
        let mut table = SignatureTable::new();
        seed(&mut table, &mut interner);
        let graph = tree_graph();

        let ty = resolve_property(&QName::parse("TimeoutError"), "message", &table, &graph)
            .expect("`message` resolves through the seeded links");
        assert_eq!(interner.describe(ty), "string");
        let backtrace = resolve_property(&QName::parse("IOError"), "backtrace", &table, &graph)
            .expect("`backtrace` resolves through the seeded links");
        assert_eq!(interner.describe(backtrace), "array<string>");

        let (owner, sig) = resolve_method(&QName::parse("IOError"), "constructor", &table, &graph)
            .expect("the constructor resolves through the seeded links");
        assert_eq!(owner.to_string(), ROOT);
        assert!(!sig.is_static);
        assert_eq!(sig.params.len(), 1);
    }

    #[test]
    fn parse_error_declares_the_issue_list_and_a_constructor_of_its_own() {
        let mut interner = TypeInterner::new();
        let mut table = SignatureTable::new();
        seed(&mut table, &mut interner);
        let graph = tree_graph();

        let ty = resolve_property(&QName::parse("ParseError"), "issues", &table, &graph)
            .expect("`issues` is ParseError's own property");
        let issues = issue_shape(&mut interner);
        assert_eq!(ty, interner.array(issues));

        let (owner, _) = resolve_method(&QName::parse("ParseError"), "constructor", &table, &graph)
            .expect("ParseError declares its own constructor");
        assert_eq!(owner.to_string(), "ParseError");
        // Every sibling still reaches the root's.
        let (owner, _) = resolve_method(&QName::parse("IOError"), "constructor", &table, &graph)
            .expect("IOError inherits the root's");
        assert_eq!(owner.to_string(), ROOT);
        assert!(resolve_property(&QName::parse("IOError"), "issues", &table, &graph).is_none());
    }

    #[test]
    fn the_root_owes_its_constructor_no_definite_initialization() {
        // The constructor is synthesized by `mwl_ir::lower`, not written in
        // MWL, so ADR 0022's obligation has nothing to check it against.
        let mut interner = TypeInterner::new();
        let mut table = SignatureTable::new();
        seed(&mut table, &mut interner);
        let root = table.get(&QName::parse(ROOT)).expect("Throwable is seeded");
        assert!(root.required_properties.is_empty());
    }
}
