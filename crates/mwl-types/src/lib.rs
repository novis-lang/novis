//! The MWL type checker (ADR 0007) — M2's last open thread. See
//! `docs/implementation-plan.md`'s M2 paragraph and `docs/agent/handoff.md`
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
//!   class/interface/enum's own declared property types and method
//!   signatures, built once ahead of body-checking, plus the ancestor walk
//!   that looks one up through `extends`/`implements`.
//! - [`defaults`] — [`defaults::ConstArg`]: a parameter default, evaluated
//!   once during signature collection so the *call site* that omits the
//!   parameter can emit it as an ordinary literal argument. See that module's
//!   own docs for why the caller does that rather than the callee, and for the
//!   two shapes (`null`, an enum case) it is expected to grow next.
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
//! - [`expr_table`] — [`expr_table::ExprTypeTable`]: the typed-expression
//!   table `mwl-ir` reads a call's/`new`'s resolved target from once it needs
//!   to lower one — see that module's own docs for the full design and why
//!   `mwl-ir` reads this instead of depending on [`signatures`]/[`ClassGraph`]
//!   directly.
//! - [`layout`] — [`layout::build_class_layouts`]: every declared class's
//!   instance-field *slot order* and its flattened supertype set, the second
//!   thing this crate publishes for `mwl-ir` to read back. See that module's
//!   own docs for why the resolution has to happen here rather than there.
//! - [`string_lit`] — [`string_lit::cook_double_quoted_text`]: cooks a
//!   double-quoted string literal's (or an interpolated-heredoc text run's)
//!   escapes into the `string` it denotes, diagnosing the two ways cooking
//!   can fail — an out-of-range `\u{...}` codepoint, or a byte escape
//!   sequence that isn't valid UTF-8. `pub`, and reused directly by
//!   `mwl-ir`'s own lowering rather than duplicated — see that module's own
//!   docs for why this one, unlike `expr`'s `int_literal_digits`, is shared.
//! - [`lateinit`] — [`lateinit::check_class_lateinit_reads`]: ADR 0038's
//!   `lateinit` property modifier. `signatures::build_signatures` validates
//!   where it may appear (§ 1: refusing a scalar/enum type, `?T`, a promoted
//!   parameter, and `readonly`) and excludes it from `ctor_init`'s
//!   constructor-must-assign obligation; this module adds § 3's one
//!   compile-time bonus check, a third flow-analysis pass — sibling to
//!   `ctor_init`'s, but over every method body rather than only the
//!   constructor.
//!
//! # Known gaps
//!
//! Deliberately out of scope so far, left for a follow-up (see
//! `docs/agent/handoff.md` for the ordering):
//!
//! - ADR 0024 (`tainted` propagation/laundering), ADR 0027 (`callable`
//!   value-shape checking) and ADR 0033 §§ 2-4 (`secret`, the same shape on
//!   an independent axis — its § 1 grammar landed in M1) are all now done —
//!   see [`expr`]'s own module docs for the first two, and for how the first
//!   two's machinery is shared with `secret` rather than duplicated:
//!   concatenation/interpolation poison their result on each axis
//!   independently, a checked `as uint`/`int`/`float`/`bool`/enum-backing-type
//!   conversion launders both qualifiers for free (a known, ADR-accepted gap
//!   for `secret` specifically — see ADR 0033 § 2), `bytes`/`string` preserve
//!   both qualifiers across either direction (including the identity-shaped
//!   `as string`, which must not silently launder either one), and `as
//!   Core\Html\Markup` accepts only a literal string token and separately
//!   refuses a `secret` operand with its own diagnostic (escaping doesn't
//!   restore confidentiality); a `secret` value passed as a `Throwable`-
//!   shaped class's constructor message is refused too, resolved through the
//!   exception tree [`error_lib`] seeds — which is what gives `Throwable` and
//!   its subclasses a signature table without a source declaration, the same
//!   way [`core_lib`] does for `Core`. A bare string or `[$obj, 'method']`
//!   array where `callable` is expected gets a targeted diagnostic,
//!   `$obj(...)` is refused for any resolved-class `$obj`, and first-class
//!   callable syntax (`$obj->method(...)`, `Foo::bar(...)`) now types as
//!   `callable` rather than the referenced method's own return type. **Known
//!   gaps within these three ADRs:** the sink list in ADR 0024 § 4
//!   (`Core\Db`, `Core\Process`, `Core\Http`, `Core\Fs`) has no code to
//!   refuse anything at yet, since none of those `Core` classes are declared
//!   stdlib until M7/M8 — a plain-typed parameter on a user-declared method
//!   already acts as an equivalent sink today, via the ordinary `tainted
//!   string` vs `string` assignability rule; § 5's auto-escape default and
//!   `Markup + Markup` composition wait on `Core\Html` actually existing.
//!   ADR 0033's own remaining sinks — `Core\Log`'s call-site inspection (M8)
//!   and `var_dump`/`print_r`'s redaction (M4) — are deferred by that ADR's
//!   own *Verification* section, as is `serialize()`/the `spawn worker`
//!   boundary refusal (M5). An exception class *does* have a declared member
//!   table now — [`error_lib`] seeds spec § 10's four readonly properties and
//!   the one constructor — so `$e->message` is checked like any other
//!   property read.
//!   ADR 0014's "a property
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
//!   declares a matching `compareTo`. [`conformance`] checks every *other*
//!   interface's members against its implementers, and its own docs say why
//!   `Comparable`/`Stringable` are the two it deliberately leaves out. ADR 0028 (`Stringable`, `unset()` refusal) is done too —
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
//! - **ADR 0007 § 6's narrowing is one of its four spellings.** `=== null`/
//!   `!== null` over a plain local narrows, and [`locals`]' own docs own the
//!   rule, what invalidates one and the two places the walk deliberately
//!   refuses to prove anything. `instanceof`, a comparison against a
//!   literal-typed value and `match (true)` do not narrow yet, and the
//!   residue is restricted to a class — the same conservative direction as
//!   the row above: a missing narrowing is a diagnostic, never a wrong
//!   program.
//! - References (`&$x`) needing both sides to declare the same type.
//! - A **user-declared** class constant's type, a promoted
//!   constructor-parameter property, and a named/spread call argument's
//!   positional checking — see [`signatures`]/[`expr`]'s own known-gaps
//!   lists. A `Core` class's constant is not among them: it is stated by
//!   `mwl_stdlib::registry::CoreConst` and resolved by [`expr`]'s
//!   `ClassConstAccess` arm.
//! - A class with no explicit `constructor` is not held to a zero-argument
//!   arity check on `new` — see [`expr`]'s `New` handling.
//! - A `foreach` **key** binding declared at anything but `string` is not
//!   diagnosed here. ADR 0007 § 5 gives an array one stored key type, so
//!   `foreach ($a as int $k => …)` is always wrong; today it type-checks and
//!   then trips `mwl_ir`'s assertion instead of getting a diagnostic.
//! - **A class member's `private`/`protected` modifier is not enforced at
//!   all.** Only ADR 0043 § 3's private *interface* method is
//!   ([`signatures::MethodSig::interface_private`]); a `private` method or
//!   property declared on a class is callable and readable from anywhere,
//!   which is a PHP-observable divergence, not a design choice. It wants one
//!   pass keyed on the accessing class, over both
//!   [`expr::check_property_access`] and method resolution.
//! - **The reserved `Comparable`/`Stringable` interfaces carry no member
//!   signatures**, so a parameter declared at either type has no method to
//!   call — `$s->toString()` on a `Stringable` is `E0405` — and `$x
//!   instanceof Stringable` records no resolved class, which `mwl_ir` then
//!   panics on rather than lowering. [`conformance`]'s own docs say why the
//!   roster leaves them out today; filling it in is what closes both.

pub mod check;
pub(crate) mod conformance;
pub mod core_lib;
pub mod ctor_init;
pub mod defaults;
pub mod enums;
pub mod error_lib;
pub mod expr;
pub mod expr_table;
pub(crate) mod generics;
pub mod iter_lib;
pub mod lateinit;
pub mod layout;
pub mod locals;
pub mod lower;
pub mod signatures;
pub mod string_lit;
pub mod ty;

pub use check::{HOOK_VALUE_PARAM, check_program};
pub use core_lib::symbol_of as core_symbol_of;
pub use defaults::ConstArg;
pub use enums::{EnumBacking, EnumInfo, EnumTable, EnumValue};
pub use expr_table::{ExprId, ExprInfo, ExprTypeTable, ForeachDrive, ResolvedCall};
pub use layout::{ClassLayout, ClassLayoutTable, build_class_layouts};
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
    /// The name of the property whose ADR 0014 § 1 hook body is being
    /// checked, if any — `$`-sigil not included.
    ///
    /// Exists for one rule: inside `$p`'s own `get`/`set` hooks,
    /// `$this->p` is the backing slot rather than a re-entrant call to the
    /// hook currently running. That is what makes a hook that transforms a
    /// stored value terminate, and it is the only place the checker needs to
    /// know *which* accessor it is inside.
    pub current_hook: Option<&'a str>,
    /// The element type `T` of the `Iterator<T>` the enclosing body is a
    /// generator for (ADR 0053 § 4), or `None` in an ordinary body.
    ///
    /// One field answers both of `yield`'s questions: whether it is legal
    /// here at all, and what its operand has to satisfy. `crate::check`'s
    /// `check_method` is the only place it is ever set, from
    /// `mwl_syntax::ast::is_generator_body` plus the declared return type.
    pub generator_elem: Option<crate::ty::TypeId>,
}

/// The read-only tables, the source text, the type interner and the
/// diagnostics sink every lowering/checking function needs — bundled so a
/// recursive call threads one argument instead of seven, same idiom as
/// [`mwl_hir::members`]'s `Env`.
pub(crate) struct Env<'a> {
    pub symbols: &'a SymbolTable,
    pub aliases: &'a AliasTable,
    /// Every class/interface's resolved `extends`/`implements` links —
    /// needed to walk ancestors when resolving `parent` or looking up an
    /// inherited property/method signature.
    pub graph: &'a ClassGraph,
    /// Every class/interface/enum's own declared property types and method
    /// signatures ([`signatures::build_signatures`]). During the
    /// signature-collection pass itself this points at an unrelated, empty
    /// placeholder table — collection never reads it, only writes to its own
    /// separate `&mut SignatureTable` parameter — see
    /// [`signatures::build_signatures`]'s docs for why that's safe.
    pub signatures: &'a SignatureTable,
    /// Every declared enum's backing type and case values
    /// ([`enums::build_enum_table`]) — read wherever an enum-typed annotation
    /// is interned (an enum's backing type is part of [`ty::Ty::Enum`]'s
    /// identity) and wherever `EnumName::CaseName` resolves to its constant.
    /// Built before [`signatures::build_signatures`], which already needs it.
    pub enums: &'a EnumTable,
    pub src: &'a SourceFile,
    pub interner: &'a mut TypeInterner,
    /// Where a call's/`new`'s resolved target is persisted for `mwl-ir` to
    /// read back later — see [`crate::expr_table`]'s own module docs.
    pub exprs: &'a mut crate::expr_table::ExprTypeTable,
    pub diags: &'a mut mwl_diagnostics::Diagnostics,
    /// How many ADR 0031 `fn` closure literals this run has checked so far —
    /// the suffix that makes each one's synthesized environment class label
    /// unique. One counter for the whole run rather than one per body,
    /// because a closure nested inside another closure has no enclosing
    /// declaration of its own to be numbered within.
    pub closure_seq: u32,
}

pub(crate) fn span_text(src: &SourceFile, span: Span) -> &str {
    src.span_text(span).unwrap_or_default()
}

/// Strips a variable's leading `$` sigil, if present.
pub(crate) fn strip_sigil(s: &str) -> &str {
    s.strip_prefix('$').unwrap_or(s)
}
