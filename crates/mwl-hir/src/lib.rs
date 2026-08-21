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
//! - [`hierarchy`] — [`HierarchyResolver`]: resolves every class/interface's
//!   `extends`/`implements` and every class/trait's `use Trait, ...;` to real
//!   [`Symbol`]s, into a [`ClassGraph`]; diagnoses an undeclared or
//!   wrong-kind parent, a circular `extends`/trait-use chain, and a trait
//!   method-name collision with no `insteadof` naming a winner.
//! - [`members`] — [`MemberResolver`]: resolves every `Class::member`
//!   reference (a static call, a class constant, an enum case, a static
//!   property) to something actually declared on that class or reached
//!   through [`ClassGraph`], into a [`MemberTable`]; diagnoses an undeclared
//!   class side or an undeclared member.
//! - [`aliases`] — [`AliasResolver`]: substitutes every `type` alias's
//!   expansion — including inside another alias's own expansion — into a
//!   fully-resolved [`AliasTable`]; diagnoses a cycle.
//!
//! # What this slice of M2 covers
//!
//! The plan's M2 paragraph lists five name-resolution responsibilities for
//! `mwl-hir`. This slice covers:
//!
//! 1. Namespace and `use` scoping, the declared symbol table, and the class
//!    hierarchy graph: `extends`/`implements` resolved to real symbols, and
//!    trait-use conflicts resolved by `insteadof` alone. **Known gap:** a
//!    trait pulling in another trait's methods is not flattened recursively
//!    yet — only a trait's own directly-declared methods are checked for a
//!    collision against traits used alongside it. See [`hierarchy`]'s module
//!    docs.
//! 2. Every callable/constant resolves as a class member, with no bare-name
//!    fallback ([ADR 0011](../../../docs/adr/0011-functions-and-constants-are-class-members.md)).
//!    The `Core`-reservation half is enforced by `mwl-syntax`'s parser at the
//!    `namespace` declaration site; the member-resolution half — a
//!    `self`/`static`/`parent`/explicit-class-name `Class::member` reference
//!    checked against the class it names and every ancestor reached through
//!    [`ClassGraph`] — is [`members`]'s job. **Known gaps:** a dynamic class
//!    side, `new`'s target, and member visibility are not checked; see
//!    [`members`]'s module docs.
//! 3. `type` alias declarations are collected into the symbol table, ADR
//!    0015 § 6's bare-class rule is enforced at declaration time
//!    ([`resolve`]), and every alias's expansion — including through another
//!    alias, recursively — is substituted into an [`aliases::AliasTable`],
//!    with a cycle (`type A = B; type B = A;`) diagnosed rather than looped
//!    ([ADR 0015](../../../docs/adr/0015-no-name-aliasing.md) § 5). **Known
//!    gap:** the substituted table is not yet consulted from anywhere else —
//!    there is no property/parameter/return-type walk in `mwl-hir` yet for it
//!    to feed; that arrives with `mwl-types`.
//!
//! Items 4 (`require`'s static resolution) and 5 (the property-access rule)
//! are not started.
//!
//! **Known gap:** [`QName`] compares segments case-sensitively; PHP does not.
//! See its docs.

pub mod aliases;
pub mod hierarchy;
pub mod members;
pub mod qname;
pub mod resolve;
pub mod symbol;

pub use aliases::{AliasResolver, AliasTable};
pub use hierarchy::{ClassGraph, ClassLinks, HierarchyResolver};
pub use members::{ClassMembers, MemberResolver, MemberTable};
pub use qname::QName;
pub use resolve::{Import, Module, Resolver, resolve_file};
pub use symbol::{Symbol, SymbolKind, SymbolTable};
