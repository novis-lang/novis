//! The MWL type checker (ADR 0007) — M2's last open thread. See
//! `docs/implementation-plan.md`'s M2 paragraph and `NEXT_SESSION_PROMPT.md`
//! for how this crate grew: a full type checker covering every ADR M2
//! assigns to `mwl-types` was too large for one slice, so the first slice
//! covered ADR 0007 §§ 1-4 in full (declared-type recording, per-local
//! definite assignment, the interned type grammar, the arithmetic
//! result-type table) plus enough of §§ 5-6 to satisfy the earliest corpus
//! items; this one adds property/method-call/`new`/`match`/ternary
//! expression typing on top, via a new per-class signature table
//! ([`signatures`]).
//!
//! # Layout
//!
//! - [`ty`] — [`ty::Ty`]/[`ty::TypeId`]/[`ty::TypeInterner`]: the interned
//!   type representation everything else in this crate is built on.
//! - [`lower`] — [`lower::lower_type`]: resolves a parsed
//!   [`mwl_syntax::ast::Type`] into a [`ty::TypeId`], including
//!   `self`/`static`, `type`-alias substitution (the first real consumer of
//!   [`mwl_hir::AliasTable`]), and ADR 0007 § 5's depth-32 array-nesting
//!   bound.
//! - [`signatures`] — [`signatures::build_signatures`]/
//!   [`signatures::resolve_property`]/[`signatures::resolve_method`]: every
//!   class/interface/trait/enum's own declared property types and method
//!   signatures, built once ahead of body-checking, plus the ancestor walk
//!   that looks one up through `extends`/`implements`/trait-use.
//! - [`locals`] — [`locals::LocalScope`]/[`locals::check_block`]: per-body
//!   local-variable declare-once checking and flow-sensitive definite
//!   assignment.
//! - [`expr`] — [`expr::check_expr`]: a bidirectional expression checker —
//!   literals, variable reads, the binary-operator result-type table,
//!   `as`/cast conversions, array literals checked against a target element
//!   type, and (using [`signatures`]) property access, method calls, static
//!   calls/properties, `new` (including argument checking against a
//!   resolved `constructor`), and `match`/ternary as the union of their
//!   branches' types. See [`expr`]'s own docs for exactly which receiver
//!   shapes resolve and which diagnostics belong to this crate versus
//!   `mwl_hir::members`.
//! - [`check`] — [`check::check_program`]: the entry point, walking a
//!   resolved [`mwl_hir::Module`]'s classes and methods the same way
//!   [`mwl_hir::members`] already does, seeding each method body's
//!   [`locals::LocalScope`] from its lowered parameters (`$this` included,
//!   typed as the enclosing class) and checking every `return` against the
//!   lowered return type.
//!
//! # Known gaps
//!
//! Deliberately out of scope so far, left for a follow-up (see
//! `NEXT_SESSION_PROMPT.md` for the ordering):
//!
//! - ADR 0010's enum-vs-class atom distinction beyond "resolves to *a*
//!   symbol", ADR 0013 (`Comparable`), ADR 0014's interplay with a typed
//!   receiver, ADR 0022 (definite *property* initialization — this slice's
//!   flow analysis covers only local variables), ADR 0024 (tainted
//!   propagation/laundering), ADR 0027 (`callable` value-shape checking),
//!   ADR 0028 (`Stringable`, `unset()` refusal).
//! - Exhaustive control-flow reachability (e.g. "every path through this
//!   non-void function returns"); `switch` and `try`/`catch` bodies
//!   conservatively contribute nothing to definite-assignment after them —
//!   safe (may reject a few valid programs), never accepts an invalid one.
//! - References (`&$x`) needing both sides to declare the same type.
//! - `parent` as a *type* atom (`parent $x`) is still unresolved — only
//!   `new parent(...)` is, since that's the one this slice's corpus needed;
//!   see [`lower`]'s own docs.
//! - A class constant's type, a promoted constructor-parameter property, and
//!   a named/spread call argument's positional checking — see
//!   [`signatures`]/[`expr`]'s own known-gaps lists.
//! - A class with no explicit `constructor` is not held to a zero-argument
//!   arity check on `new` — see [`expr`]'s `New` handling.

pub mod check;
pub mod expr;
pub mod locals;
pub mod lower;
pub mod signatures;
pub mod ty;

pub use check::check_program;
pub use ty::{Ty, TypeId, TypeInterner};

use mwl_diagnostics::{SourceFile, Span};
use mwl_hir::{AliasTable, ClassGraph, QName, SymbolTable};
use rustc_hash::FxHashMap;

use crate::signatures::SignatureTable;

/// The namespace/`use`/enclosing-class scope active at whatever point in the
/// AST is currently being lowered or checked — mirrors
/// [`mwl_hir::members`]'s `Ctx`, for the same reason: this changes as the
/// walk descends into a new namespace or class body, while [`Env`]'s tables
/// stay fixed for the whole run.
pub(crate) struct Ctx<'a> {
    pub namespace: &'a [String],
    pub imports: &'a FxHashMap<String, QName>,
    pub current_class: Option<&'a QName>,
}

/// The read-only tables, the source text, the type interner and the
/// diagnostics sink every lowering/checking function needs — bundled so a
/// recursive call threads one argument instead of seven, same idiom as
/// [`mwl_hir::members`]'s `Env`.
pub(crate) struct Env<'a> {
    pub symbols: &'a SymbolTable,
    pub aliases: &'a AliasTable,
    /// Every class/interface/trait's resolved `extends`/`implements`/
    /// trait-use links — needed to walk ancestors when resolving `parent` or
    /// looking up an inherited property/method signature.
    pub graph: &'a ClassGraph,
    /// Every class/interface/trait/enum's own declared property types and
    /// method signatures ([`signatures::build_signatures`]). During the
    /// signature-collection pass itself this points at an unrelated, empty
    /// placeholder table — collection never reads it, only writes to its own
    /// separate `&mut SignatureTable` parameter — see
    /// [`signatures::build_signatures`]'s docs for why that's safe.
    pub signatures: &'a SignatureTable,
    pub src: &'a SourceFile,
    pub interner: &'a mut TypeInterner,
    pub diags: &'a mut mwl_diagnostics::Diagnostics,
}

pub(crate) fn span_text(src: &SourceFile, span: Span) -> &str {
    src.span_text(span).unwrap_or_default()
}

/// Strips a variable's leading `$` sigil, if present.
pub(crate) fn strip_sigil(s: &str) -> &str {
    s.strip_prefix('$').unwrap_or(s)
}
