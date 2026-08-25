//! HIR for MWL: name resolution over the `mwl-syntax` AST
//! ([`docs/implementation-plan.md`](../../../docs/implementation-plan.md)'s
//! Architecture diagram — the layer between `mwl-syntax` and `mwl-types`).
//!
//! # Layout
//!
//! - [`qname`] — [`QName`], a fully-qualified, backslash-separated name.
//! - [`symbol`] — [`SymbolTable`]: one entry per declared
//!   class/interface/trait/enum/type alias, keyed by its [`QName`].
//! - [`resolve`] — [`Resolver`]/[`resolve_file`]: walks a parsed file's
//!   top-level statements, tracks namespace and `use` scope PHP-style (a
//!   `namespace Name;` statement both re-namespaces and resets the imported-
//!   short-name set for the rest of the sequence it appears in), and
//!   produces a [`Module`].
//! - [`errors`] — the closed exception tree spec § 10 fixes, as data every
//!   later crate seeds itself from.
//! - [`interfaces`] — the closed set of global interfaces the compiler
//!   declares, with each one's type parameters, on the same
//!   seeded-from-data footing as [`errors`].
//! - [`hierarchy`] — [`HierarchyResolver`]: resolves every class/interface's
//!   `extends`/`implements` and every class/trait's `use Trait, ...;` to real
//!   [`Symbol`]s, into a [`ClassGraph`]; diagnoses an undeclared or
//!   wrong-kind parent, a circular `extends`/trait-use chain, and a trait
//!   method-name collision with no `insteadof` naming a winner.
//! - [`members`] — [`MemberResolver`]: resolves every `Class::member`
//!   reference (a static call, a class constant, an enum case, a static
//!   property) and every `$this->name` property access to something
//!   actually declared on that class or reached through [`ClassGraph`], into
//!   a [`MemberTable`]; diagnoses an undeclared class side, an undeclared
//!   member, or an undeclared property.
//! - [`aliases`] — [`AliasResolver`]: substitutes every `type` alias's
//!   expansion — including inside another alias's own expansion — into a
//!   fully-resolved [`AliasTable`]; diagnoses a cycle.
//! - [`requires`] — [`resolve_program`]: walks the `require` graph reachable
//!   from one entry file, merging every statically-resolvable target's
//!   declarations into one [`Module`] via the same multi-file
//!   `collect_*`-then-resolve shape every resolver above already supports;
//!   diagnoses a missing target or a require cycle.
//!
//! # Known gaps
//!
//! `mwl-hir` now covers all five name-resolution responsibilities the plan's
//! M2 paragraph lists for it; each module above documents its own gaps in
//! full, not repeated here. The sharper edges: [`hierarchy`] doesn't flatten
//! a trait pulling in another trait's methods recursively; [`members`]
//! doesn't check a dynamic class side, `new`'s target, member visibility, or
//! a property access on any receiver but `$this` (that needs `mwl-types`'
//! static types); [`aliases`]'s [`AliasTable`] has no consumer yet — nothing
//! in `mwl-hir` walks a property/parameter/return-type position for it to
//! feed, so that arrives with `mwl-types` too; [`requires`] only recognises a
//! plain quoted-string literal path.

pub mod aliases;
pub mod autoload;
pub mod errors;
pub mod hierarchy;
pub mod interfaces;
pub mod members;
pub mod qname;
pub mod requires;
pub mod resolve;
pub mod symbol;

pub use aliases::{AliasResolver, AliasTable};
pub use autoload::{AutoloadMap, Probe};
pub use hierarchy::{
    ClassGraph, ClassLinks, HierarchyResolver, implements_interface, resolve_ref,
    seed_exception_tree,
};
pub use members::{ClassMembers, MemberResolver, MemberTable};
pub use qname::QName;
pub use requires::resolve_program;
pub use resolve::{Import, Module, Resolver, resolve_file};
pub use symbol::{Symbol, SymbolKind, SymbolTable};
