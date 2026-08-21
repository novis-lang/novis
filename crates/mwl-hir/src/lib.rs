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
//!
//! # What this slice of M2 covers
//!
//! The plan's M2 paragraph lists five name-resolution responsibilities for
//! `mwl-hir`. This first slice covers:
//!
//! 1. Namespace and `use` scoping, and the declared symbol table (class
//!    hierarchy — `extends`/`implements`/trait flattening — is not built
//!    yet; only that each class/interface/trait/enum/type-alias name is
//!    declared once, under the right fully-qualified name).
//! 3. `type` alias declarations are collected into the symbol table, and
//!    [ADR 0015](../../../docs/adr/0015-no-name-aliasing.md) § 6's rule
//!    (an alias may not be a single bare class/interface/enum atom) is
//!    enforced. Substituting an alias's expansion into the types that use it
//!    — the rest of § 5 — is not built yet.
//!
//! Item 2 (every callable/constant resolves as a class member, with a
//! diagnostic for a declaration reusing `Core`) has its `Core`-reservation
//! half already enforced by `mwl-syntax`'s parser at the `namespace`
//! declaration site; the member-resolution half needs the class graph above
//! first. Items 4 (`require`'s static resolution) and 5 (the property-access
//! rule) are not started.
//!
//! **Known gap:** [`QName`] compares segments case-sensitively; PHP does not.
//! See its docs.

pub mod qname;
pub mod resolve;
pub mod symbol;

pub use qname::QName;
pub use resolve::{Import, Module, Resolver, resolve_file};
pub use symbol::{Symbol, SymbolKind, SymbolTable};
