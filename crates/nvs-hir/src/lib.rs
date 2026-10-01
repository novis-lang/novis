//! HIR for Novis: name resolution over the `nvs-syntax` AST
//! ([`docs/implementation-plan.md`](/docs/implementation-plan.md)'s
//! Architecture diagram — the layer between `nvs-syntax` and `nvs-types`).
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
//!   wrong-kind parent — including a name under `Core` the caller's
//!   [`CoreRoster`] does not list — a circular `extends`/trait-use chain, and
//!   a trait method-name collision with no `insteadof` naming a winner.
//!   [`implementors`] asks that graph the other way — which non-abstract
//!   classes reach one interface — which is `rule:programs/implementing`'s enumeration.
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
//!   diagnoses a missing target or a require cycle. Hands back a [`Loaded`]
//!   per file, entry first, so the phases after this one do not re-parse the
//!   graph to find out what is in it. A program writing
//!   `Core\Program::implementing<T>()` also gets § 3's scan here: every file
//!   [`AutoloadMap::enumerate`] names is loaded, once, whether or not
//!   anything mentions it.
//!
//! `nvs-hir` covers the name-resolution responsibilities the plan's M2
//! paragraph lists for it, and records no gap of its own: what a module
//! still owes is written in that module's own doc, [`requires`] carrying
//! the longest list. What this crate deliberately does not do belongs to
//! somewhere else — a member's visibility and the static type behind a
//! receiver are `nvs-types`' question, which is also [`AliasTable`]'s
//! consumer, and there is no trait-use flattening because
//! `rule:classes/no-traits` leaves the language no `trait` to flatten.

pub mod aliases;
pub mod autoload;
pub mod errors;
pub mod hierarchy;
pub mod imports;
pub mod interfaces;
pub mod members;
pub mod qname;
pub mod requires;
pub mod resolve;
pub mod symbol;

pub use aliases::{AliasResolver, AliasSite, AliasTable};
pub use autoload::{AutoloadMap, Probe};
pub use hierarchy::{
    ClassGraph, ClassLinks, CoreRoster, HierarchyResolver, Undeclared, implementors,
    implements_interface, relative_spelling, resolve_ref, seed_exception_tree, undeclared_name,
    undeclared_name_at,
};
pub use imports::{ImportSite, candidates, import_site};
pub use members::{ClassMembers, MemberResolver, MemberTable, PhpFunctions};
pub use qname::QName;
pub use requires::{Loaded, resolve_program, resolve_program_borrowing, resolve_program_linted};
pub use resolve::{Import, Module, Resolver, resolve_file};
pub use symbol::{Symbol, SymbolKind, SymbolTable};
