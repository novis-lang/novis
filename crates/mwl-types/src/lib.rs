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
//!   literals, variable reads, the binary-operator result-type table
//!   (including ADR 0013's `Comparable` amendment for `< <= > >= <=>`
//!   between two objects), `as`/cast conversions, array literals checked
//!   against a target element type, and (using [`signatures`]) property
//!   access, method calls, static calls/properties, `new` (including
//!   argument checking against a resolved `constructor`), and
//!   `match`/ternary as the union of their branches' types. See [`expr`]'s
//!   own docs for exactly which receiver shapes resolve and which
//!   diagnostics belong to this crate versus `mwl_hir::members`.
//! - [`check`] — [`check::check_program`]: the entry point, walking a
//!   resolved [`mwl_hir::Module`]'s classes and methods the same way
//!   [`mwl_hir::members`] already does, seeding each method body's
//!   [`locals::LocalScope`] from its lowered parameters (`$this` included,
//!   typed as the enclosing class) and checking every `return` against the
//!   lowered return type.
//! - [`ctor_init`] — [`ctor_init::check_class_init`]: ADR 0022 § 2's
//!   definite-property-initialization check, a second flow-analysis pass
//!   over each class's own constructor (or, absent one, over its required
//!   properties' own declarations directly) — extending [`locals`]'s
//!   definite-assignment idea to a second binding kind, as that ADR's own
//!   framing asks for, rather than reusing `locals`'s code directly (the two
//!   passes track different per-path state and don't share a walker).
//!
//! # Known gaps
//!
//! Deliberately out of scope so far, left for a follow-up (see
//! `NEXT_SESSION_PROMPT.md` for the ordering):
//!
//! - ADR 0033 §§ 2-4 (`secret` propagation/laundering/sink refusal — its § 1
//!   grammar landed in M1, but `lower_atom` still maps all four `Secret*`
//!   atoms straight to `mixed`). ADR 0024 (tainted propagation/laundering)
//!   and ADR 0027 (`callable` value-shape checking) are now done — see
//!   [`expr`]'s own module docs for both: concatenation/interpolation poison
//!   their result, a checked `as uint`/`int`/`float`/`bool`/enum-backing-type
//!   conversion launders for free, `bytes`/`string` preserve the qualifier
//!   across either direction (including the identity-shaped `as string`,
//!   which must not silently launder), and `as Core\Html\Markup` accepts only
//!   a literal string token; a bare string or `[$obj, 'method']` array where
//!   `callable` is expected gets a targeted diagnostic, `$obj(...)` is
//!   refused for any resolved-class `$obj`, and first-class callable syntax
//!   (`$obj->method(...)`, `Foo::bar(...)`) now types as `callable` rather
//!   than the referenced method's own return type. **Known gaps within these
//!   two ADRs:** the sink list in ADR 0024 § 4 (`Core\Db`, `Core\Process`,
//!   `Core\Http`, `Core\Fs`) has no code to refuse anything at yet, since none
//!   of those `Core` classes are declared stdlib until M7/M8 — a plain-typed
//!   parameter on a user-declared method already acts as an equivalent sink
//!   today, via the ordinary `tainted string` vs `string` assignability
//!   rule; § 5's auto-escape default and `Markup + Markup` composition wait on
//!   `Core\Html` actually existing. ADR 0014's "a property
//!   access on any receiver other than `$this` is checked" half turned out to
//!   already be done: [`expr::check_property_access`] reports
//!   `E_UNKNOWN_MEMBER` for exactly that shape (see its own module docs) —
//!   `mwl_hir::members`'s and this module's known-gap notes were just stale
//!   about it. ADR 0022 (definite *property*
//!   initialization) is now done for the shapes its own M2 corpus names —
//!   see [`ctor_init`]'s docs for what is deliberately still out of scope
//!   within that ADR specifically. ADR 0013 (`Comparable`) is now done too
//!   — see [`expr`]'s `object_comparison_result` for the one thing it
//!   doesn't check: that a class claiming `implements Comparable` actually
//!   declares a matching `compareTo` at all, since no ADR has asked for
//!   general interface-method-completeness checking yet (no interface's
//!   methods are verified against its implementers today, for any
//!   interface). ADR 0028 (`Stringable`, `unset()` refusal) is done too —
//!   see [`expr::require_stringable`]/[`expr::check_property_access`]. ADR
//!   0036's checker semantics are now done as well: `object` carries real
//!   subtyping (every class or shape type is `<: object`), a shape type
//!   ([`ty::Ty::Shape`]) is checked structurally by width subtyping plus
//!   ordinary field assignability (see [`expr::is_assignable`]'s own docs),
//!   and a property access through a shape-missing field or plain `object`
//!   is silently erased to `mixed` rather than diagnosed — deferred to ADR
//!   0014 § 5's runtime-checked fallback, which is M4 work (no IR/codegen
//!   exists yet to throw from) — see [`expr::check_property_access`]'s own
//!   docs. ADR 0010's enum-vs-class atom distinction beyond "resolves to *a*
//!   symbol" is now done too: `self`/`static`/`$this` inside an enum
//!   ([`expr::class_of_ctx`], [`lower`]'s `resolve_special`) and a case access
//!   ([`expr`]'s `ClassConstAccess` arm) all recover [`ty::Ty::Enum`] rather
//!   than [`ty::Ty::Class`]; an arithmetic or bitwise operator applied
//!   directly to an enum operand and a conversion from one enum type to a
//!   *different* one, even via `as`, are both diagnosed per ADR 0010 § 5 —
//!   see [`expr`]'s `reject_enum_operand`/`reject_enum_to_enum_conversion`.
//!   `==`/`===` between two different enum types is not yet diagnosed — no
//!   general equality-operand-compatibility check exists for *any* type pair
//!   today (not even `int` against `uint`), so singling out enums there
//!   would be inconsistent; that wants its own pass, not a one-off special
//!   case.
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
pub mod ctor_init;
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
