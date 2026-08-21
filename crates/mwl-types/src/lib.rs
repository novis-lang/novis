//! The MWL type checker (ADR 0007) — the first sub-step of M2's last open
//! thread. See `docs/implementation-plan.md`'s M2 paragraph and
//! `NEXT_SESSION_PROMPT.md` for why this crate is scoped the way it is: a
//! full type checker covering every ADR M2 assigns to `mwl-types` is too
//! large for one slice, so this first one covers ADR 0007 §§ 1-4 in full
//! (declared-type recording, per-local definite assignment, the interned
//! type grammar, and the arithmetic result-type table) plus enough of §§ 5-6
//! (array-literal-against-target checking, `mixed`'s one-way absorption
//! rule) to satisfy the early corpus items ADR 0007's own *Verify* line
//! names, and a minimal `return`-type check.
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
//! - [`locals`] — [`locals::LocalScope`]/[`locals::check_block`]: per-body
//!   local-variable declare-once checking and flow-sensitive definite
//!   assignment.
//! - [`expr`] — [`expr::check_expr`]: a minimal bidirectional expression
//!   checker (literals, variable reads, the binary-operator result-type
//!   table, `as`/cast conversions, array literals checked against a target
//!   element type).
//! - [`check`] — [`check::check_program`]: the entry point, walking a
//!   resolved [`mwl_hir::Module`]'s classes and methods the same way
//!   [`mwl_hir::members`] already does, seeding each method body's
//!   [`locals::LocalScope`] from its lowered parameters and checking every
//!   `return` against the lowered return type.
//!
//! # Known gaps
//!
//! Deliberately out of scope for this slice, left for a follow-up (see
//! `NEXT_SESSION_PROMPT.md` for the ordering):
//!
//! - Property, method-call, `new`-target-beyond-a-bare-name, `match` and
//!   ternary expression typing — every such form is treated as opaque
//!   (`mixed`) by [`expr::check_expr`] rather than modeled.
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
//! - `parent` as a type atom (needs the class hierarchy graph, not threaded
//!   into this slice's [`Ctx`] since nothing here exercises it as an operand
//!   type yet).

pub mod check;
pub mod expr;
pub mod locals;
pub mod lower;
pub mod ty;

pub use check::check_program;
pub use ty::{Ty, TypeId, TypeInterner};

use mwl_diagnostics::{SourceFile, Span};
use mwl_hir::{AliasTable, QName, SymbolTable};
use rustc_hash::FxHashMap;

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
/// recursive call threads one argument instead of five, same idiom as
/// [`mwl_hir::members`]'s `Env`.
pub(crate) struct Env<'a> {
    pub symbols: &'a SymbolTable,
    pub aliases: &'a AliasTable,
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
