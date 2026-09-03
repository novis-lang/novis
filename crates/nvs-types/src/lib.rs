//! The Novis type checker (ADR 0007) — M2's last open thread. See
//! `docs/implementation-plan.md`'s M2 paragraph and `docs/agent/handoff.md`
//! for how this crate grew: a full type checker covering every ADR M2
//! assigns to `nvs-types` was too large for one slice, so the first slice
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
//!   [`nvs_syntax::ast::Type`] into a [`ty::TypeId`], including
//!   `self`/`static`, `type`-alias substitution (the first real consumer of
//!   [`nvs_hir::AliasTable`]), and ADR 0007 § 5's depth-32 array-nesting
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
//!   diagnostics belong to this crate versus `nvs_hir::members`.
//! - [`check`] — [`check::check_program`]: the entry point, walking a
//!   resolved [`nvs_hir::Module`]'s classes and methods the same way
//!   [`nvs_hir::members`] already does, seeding each method body's
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
//!   table `nvs-ir` reads a call's/`new`'s resolved target from once it needs
//!   to lower one — see that module's own docs for the full design and why
//!   `nvs-ir` reads this instead of depending on [`signatures`]/[`ClassGraph`]
//!   directly.
//! - [`mod@derive`] — `derive::check_class_derive`: ADR 0071's derive pass —
//!   which classes carry `#[Json\Derive]`, matched *nominally* against a
//!   closed `Core`-owned list, and what the field list and wire keys of each
//!   are. Runs from [`check`]'s walk because that is what holds the namespace
//!   and import set a nominal match needs; its answer is recorded in
//!   [`expr_table`] and joined against [`layout`]'s slot order by `nvs-ir`.
//! - [`consts`] — [`consts::build_const_table`]: every declared class
//!   constant's folded compile-time value, built beside [`enums`] and read
//!   only where ADR 0047 § 2's `Foo::CONST` appears in *type* position. See
//!   that module's own docs for why an ineligible value is recorded rather
//!   than dropped.
//! - [`layout`] — [`layout::build_class_layouts`]: every declared class's
//!   instance-field *slot order* and its flattened supertype set, the second
//!   thing this crate publishes for `nvs-ir` to read back. See that module's
//!   own docs for why the resolution has to happen here rather than there.
//! - [`string_lit`] — [`string_lit::cook_double_quoted_text`]: cooks a
//!   double-quoted string literal's (or an interpolated-heredoc text run's)
//!   escapes into the `string` it denotes, diagnosing the two ways cooking
//!   can fail — an out-of-range `\u{...}` codepoint, or a byte escape
//!   sequence that isn't valid UTF-8. `pub`, and reused directly by
//!   `nvs-ir`'s own lowering rather than duplicated — see that module's own
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
//!   as is § 5's constant-time `==`, whose share of the work is this crate's
//!   alone to do: the qualifier does not survive `nvs_ir::ty::Ty`, so
//!   [`expr::operators`] records
//!   [`expr_table::ExprInfo::SecretEquality`] at a comparison with a `secret`
//!   operand and `nvs-ir` picks the helper from that —
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
//!   already be done: [`expr::members::check_property_access`] reports
//!   `E_UNKNOWN_MEMBER` for exactly that shape (see its own module docs) —
//!   `nvs_hir::members`'s and this module's known-gap notes were just stale
//!   about it. ADR 0022 (definite *property*
//!   initialization) is now done for the shapes its own M2 corpus names —
//!   see [`ctor_init`]'s docs for what is deliberately still out of scope
//!   within that ADR specifically. ADR 0013 (`Comparable`) is now done too
//!   — see [`expr`]'s `object_comparison_result`, and [`conformance`] for
//!   the half it does not do: a class claiming `implements Comparable` owes
//!   a `compareTo` there, like an implementer of any other interface, now
//!   that [`iter_lib`] seeds one. ADR 0028 (`Stringable`, `unset()` refusal) is done too —
//!   see [`expr::require_stringable`]/[`expr::members::check_property_access`]. ADR
//!   0036's checker semantics are now done as well: `object` carries real
//!   subtyping (every class or shape type is `<: object`), a shape type
//!   ([`ty::Ty::Shape`]) is checked structurally by width subtyping plus
//!   ordinary field assignability (see [`expr::is_assignable`]'s own docs),
//!   and a property access through a shape-missing field or plain `object`
//!   is silently erased to `mixed` rather than diagnosed, recording the
//!   written name for ADR 0014 § 5's runtime-checked fallback, which throws
//!   for real now — see [`expr::members::check_property_access`]'s own docs. ADR 0010's enum-vs-class atom distinction beyond "resolves to *a*
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
//!   refuses to prove anything. The residue is unrestricted — dropping
//!   `null` leaves an array, a scalar or a class alike, and
//!   [`expr_table::ExprInfo::NarrowedRead`] is what carries that to `nvs-ir`.
//!   `instanceof`, a comparison against a literal-typed value and
//!   `match (true)` do not narrow yet — the same conservative direction as
//!   the row above: a missing narrowing is a diagnostic, never a wrong
//!   program.
//! - References (`inout $x`) needing both sides to declare the same type.
//! - A promoted constructor-parameter property and a named/spread call
//!   argument's positional checking — see [`signatures`]/[`expr`]'s own
//!   known-gaps lists. A class constant is not among them, in any of its three
//!   positions: a `Core` class's is stated by `nvs_stdlib::registry::CoreConst`,
//!   a user-declared one's declared type and value are
//!   [`signatures::ConstSig`], both resolved by [`expr`]'s `ClassConstAccess`
//!   arm, and a use in *type* position folds to ADR 0047 § 2's literal type
//!   over [`consts`]. What is left is a constant whose value has no constant
//!   form at all (`public const array<int> ROWS = [1, 2];`), which types as `mixed` and panics
//!   `nvs_ir::lower` if a program reads it.

pub(crate) mod attributes;
pub(crate) mod capability;
pub mod check;
// Public for [`routes`]'s reason: ADR 0086 § 6's finished table is read back
// out of [`expr_table::ExprTypeTable::commands`] by whoever runs the program,
// which hands it to `nvs_runtime::commands` for `Core\Command` to answer from.
pub mod commands;
pub(crate) mod conformance;
pub mod consts;
pub mod core_lib;
pub mod ctor_init;
pub mod defaults;
pub mod derive;
pub mod enums;
pub mod error_lib;
pub mod expr;
pub mod expr_table;
pub(crate) mod generics;
pub(crate) mod intrinsics;
pub mod iter_lib;
pub mod lateinit;
pub mod layout;
pub(crate) mod links;
pub mod locals;
pub mod lower;
pub(crate) mod program;
pub(crate) mod retrieval;
pub(crate) mod returns;
pub mod routes;
pub mod signatures;
pub mod string_lit;
pub mod testing;
pub mod ty;

pub use check::{HOOK_VALUE_PARAM, check_program, check_program_granted};
pub use core_lib::symbol_of as core_symbol_of;
pub use defaults::ConstArg;
pub use derive::{DerivedCodec, DerivedField};
pub use enums::{EnumBacking, EnumInfo, EnumTable, EnumValue};
pub use expr_table::{
    Delegation, ExprId, ExprInfo, ExprTypeTable, ForeachDrive, ResolvedCall, UrlPiece,
};
pub use layout::{ClassLayout, ClassLayoutTable, build_class_layouts};
pub use nvs_stdlib::cli::{
    NAME as CORE_CLI_TEXT_CLASS, TEXT_CONCAT_SYMBOL as CORE_CLI_TEXT_CONCAT,
};
pub use nvs_stdlib::html::{
    MARKUP_CONCAT_SYMBOL as CORE_HTML_MARKUP_CONCAT, MARKUP_NAME as CORE_HTML_MARKUP_CLASS,
    MARKUP_SYMBOL as CORE_HTML_MARKUP,
};
pub use nvs_stdlib::registry::constructor_symbol as core_constructor_symbol;
pub use nvs_stdlib::registry::takes_written_class as core_takes_written_class;
pub use nvs_stdlib::router::link::{
    ABSOLUTE_SYMBOL as CORE_ROUTE_LINK_ABSOLUTE, SYMBOL as CORE_ROUTE_LINK,
};
pub use nvs_stdlib::script::{
    AWAIT_SYMBOL as CORE_SCRIPT_AWAIT, SPAWN_SYMBOL as CORE_SCRIPT_SPAWN,
};
pub use nvs_stdlib::time::FROM_NANOS_SYMBOL as CORE_DURATION_FROM_NANOS;
pub use nvs_stdlib::{CodecField, CodecTy, EnumCases, FieldDefault};
pub use routes::{ApiError, ParamIn, Route, RouteParam, RouteTable};
pub use ty::{CoreShape, CoreShapeField, Ty, TypeId, TypeInterner};

use nvs_diagnostics::{SourceFile, Span};
use nvs_hir::{AliasTable, ClassGraph, QName, SymbolTable};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::signatures::SignatureTable;

/// One file of the program being checked: its source text and the statements
/// it parsed to.
///
/// Every table this crate builds once per program — [`enums::build_enum_table`],
/// [`consts::build_const_table`], [`signatures::build_signatures`],
/// [`layout::build_class_layouts`] — and [`check::check_program`] itself take
/// a *slice* of these rather than one file, because a `require` graph is one
/// program: a class declared in one file is referenced from another, so the
/// tables have to be complete before any body is checked. `nvs_hir::Loaded`
/// is where the CLI's slice comes from, in entry-first load order.
///
/// **The file's own [`nvs_diagnostics::SourceId`] is `src.id()`**, and this
/// struct deliberately carries no second copy of it. A consumer that has to
/// match a file against something keyed by id — `nvs-ir` calling the script
/// frame a `require` resolved to (`nvs_hir::Loaded::requires`) — asks the
/// source file, which is the one place the id has ever lived; a field here
/// would be a value every one of this struct's two dozen construction sites
/// has to supply and that could be supplied wrongly.
#[derive(Debug, Clone, Copy)]
pub struct ProgramFile<'a> {
    /// The file's source text, for every span this walk resolves, and — as
    /// [`nvs_diagnostics::SourceFile::id`] — its own id.
    pub src: &'a SourceFile,
    /// Its whole parsed body — top-level statements and declarations alike.
    pub stmts: &'a [nvs_syntax::ast::Stmt],
}

/// The namespace/`use`/enclosing-class scope active at whatever point in the
/// AST is currently being lowered or checked — mirrors
/// [`nvs_hir::members`]'s `Ctx`, for the same reason: this changes as the
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
    /// Whether the body being checked is the `constructor`'s own — the one
    /// place ADR 0038 § 1 lets a write to a `readonly` property through
    /// (`crate::expr::assign::check_write_target`).
    ///
    /// A `bool` rather than the method's name because that is the whole of
    /// what any rule asks, and `false` in a closure body written inside the
    /// constructor: a closure is called at a time this checker cannot bound,
    /// so the write it holds is not proven to happen during construction.
    pub in_constructor: bool,
    /// The element type `T` of the `Iterator<T>` the enclosing body is a
    /// generator for (ADR 0053 § 4), or `None` in an ordinary body.
    ///
    /// One field answers both of `yield`'s questions: whether it is legal
    /// here at all, and what its operand has to satisfy. `crate::check`'s
    /// `check_method` is the only place it is ever set, from
    /// `nvs_syntax::ast::is_generator_body` plus the declared return type.
    pub generator_elem: Option<crate::ty::TypeId>,
}

/// [ADR 0031](../../docs/adr/0031-callable-is-the-only-closure-type.md) § 3's
/// optional self-name, resolved: what a bare call written inside the closure's
/// own body has to spell to mean *this* closure, and what such a call answers
/// with.
///
/// Carries the return type rather than a `TypeId` for the closure, because
/// there is none to carry: § 4 gives every closure the one opaque `callable`,
/// so the recursive call's own type can only come from what the literal
/// declared. A literal that declared nothing gets `mixed` here — its body is
/// mid-check, so its inferred type is not a fact yet, and `mixed` is the same
/// answer every other call through a `callable` gives.
pub(crate) struct FnSelf {
    /// The name as written, which a callee spelling must equal exactly:
    /// § 3's name is lexical and is not resolved through the namespace or the
    /// `use` table.
    pub name: String,
    /// The declared return type, or `mixed`.
    pub ret: crate::ty::TypeId,
}

/// The read-only tables, the source text, the type interner and the
/// diagnostics sink every lowering/checking function needs — bundled so a
/// recursive call threads one argument instead of seven, same idiom as
/// [`nvs_hir::members`]'s `Env`.
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
    /// Every declared class constant's folded compile-time value
    /// ([`consts::build_const_table`]) — read only where ADR 0047 § 2's
    /// `Foo::CONST` appears in *type* position and has to fold to its own
    /// literal type. Built beside [`Self::enums`], and before
    /// [`signatures::build_signatures`], for the same reason: an annotation
    /// interned during signature collection may be one of these.
    pub consts: &'a crate::consts::ConstTable,
    /// Every `#[...]` attach site in the program, indexed by the declaration
    /// it is attached to — ADR 0046 § 4's retrieval reads it, and nothing else
    /// does. Built whole before any body is checked, because a retrieval may
    /// be written above the declaration it asks about; see
    /// [`crate::retrieval::AttributeTable`].
    pub attributes: &'a crate::retrieval::AttributeTable<'a>,
    /// The `[capabilities]` block of the configuration the *compiling* machine
    /// read, or `None` where nothing read one — [ADR 0067](../../docs/adr/0067-core-db.md)
    /// § 10's "`nvs.toml` is read at boot on the machine that compiles",
    /// which is the only thing in front of this pass that is not the program.
    ///
    /// **`None` says nothing rather than denying**, and
    /// [`crate::check::check_program_granted`] owns why. The rule for anything
    /// added here: a grant may only ever move a refusal
    /// `nvs_runtime::capability::require` would also have made *earlier*,
    /// never make one it would not — so a check reads
    /// [`nvs_config::capability::Capabilities`] through the same list walk the
    /// door does, and a scope this pass cannot resolve statically is left to
    /// the door.
    pub grants: Option<&'a nvs_config::tree::Capabilities>,
    pub src: &'a SourceFile,
    pub interner: &'a mut TypeInterner,
    /// Where a call's/`new`'s resolved target is persisted for `nvs-ir` to
    /// read back later — see [`crate::expr_table`]'s own module docs.
    pub exprs: &'a mut crate::expr_table::ExprTypeTable,
    /// ADR 0077 § 5's route table as it is collected — one row per `#[Route]`
    /// the per-class walk reaches, across every file.
    ///
    /// Here rather than a local of [`crate::check::check_program`]'s loop
    /// because both of the errors [`crate::routes::check_table`] reports are
    /// collisions *between* declarations, and § 5's scan is what puts the two
    /// colliding files in the same program in the first place. The pass that
    /// only collects signatures hands it a scratch table, exactly as it does
    /// [`Self::exprs`].
    pub routes: &'a mut crate::routes::RouteTable,
    /// ADR 0086 § 6's command table as it is collected — one row per
    /// `#[Command]` the per-class walk reaches, across every file.
    ///
    /// Beside [`Self::routes`] and threaded exactly as it is, for its reason:
    /// the error [`crate::commands::check_table`] reports is a collision
    /// *between* declarations, and ADR 0061 § 3's scan is what puts the two
    /// colliding files in the same program.
    pub commands: &'a mut crate::commands::CommandTable,
    /// ADR 0077 § 4's `Core\Router::url`/`urlAbsolute` sites, as the walk
    /// reaches them and before any of them has been looked up.
    ///
    /// Beside [`Self::routes`] and threaded the same way for a stronger form of
    /// the same reason: a link asks the finished table a question, and the walk
    /// filling that table is this one. See [`crate::links`] for why the lookup
    /// cannot be made where the call is written.
    pub links: &'a mut Vec<crate::links::LinkSite>,
    /// ADR 0071 § 2's codec-reachable question, one entry per field the derive
    /// pass keeps, asked after the walk rather than where the property is
    /// declared.
    ///
    /// [`Self::links`]'s reason exactly: "another class that itself has a
    /// codec" is a question about the whole program, and a class whose field
    /// names a deriving class declared in a later file must not answer
    /// differently from one that names an earlier one. See
    /// [`crate::derive::resolve_field_types`].
    pub codec_sites: &'a mut Vec<crate::derive::CodecFieldSite>,
    pub diags: &'a mut nvs_diagnostics::Diagnostics,
    /// How many ADR 0031 `fn` closure literals this run has checked so far —
    /// the suffix that makes each one's synthesized environment class label
    /// unique. One counter for the whole run rather than one per body,
    /// because a closure nested inside another closure has no enclosing
    /// declaration of its own to be numbered within.
    pub closure_seq: u32,
    /// [ADR 0031](../../docs/adr/0031-callable-is-the-only-closure-type.md)
    /// § 3's self-name, for the `fn` literal whose body is being checked —
    /// `None` outside one, and `None` again inside a nested literal that
    /// declares no name of its own.
    ///
    /// Saved and restored across a closure body exactly as [`Self::exit_targets`]
    /// is, and for the same reason: § 3's name reaches one body and no other,
    /// which is the reach `nvs_ir::lower::closure`'s `FN_SELF` receiver has.
    /// [`crate::expr::calls::check_fn_literal`] owns what it resolves to.
    pub fn_self: Option<FnSelf>,
    /// One entry per enclosing `break` target the statement being checked
    /// sits inside, outermost first: `true` for a loop, `false` for a
    /// `switch`. PHP's `break N`/`continue N` count these frames, a `switch`
    /// included, so a level is checked against the length — and `continue`
    /// additionally needs a loop at or outside the frame it lands on, which
    /// is why this is a stack of kinds and not a pair of counters.
    ///
    /// Maintained by [`crate::locals`] as it walks a body, and saved/emptied/
    /// restored across an ADR 0031 closure literal's body, which no enclosing
    /// loop reaches into: a `break` written in one has nothing outside the
    /// closure to leave.
    pub exit_targets: Vec<bool>,
    /// Every subscript level of an assignment *target* this run has seen, by
    /// span, mapped to whether the assignment was a plain `=`.
    ///
    /// `crate::expr::assign::mark_write_target_levels` fills it by walking
    /// the target chain **before** the target is checked, because the two
    /// things that read it are inside that check —
    /// [`crate::expr::check_expr`]'s `ExprKind::Index` arm reports `E0481`
    /// for an `index: None` that is not a plain `=`'s level (append syntax
    /// is a destination and nothing else), and suppresses `E0482` for a
    /// level whose chain root `crate::expr::assign`'s `check_write_target`
    /// is about to refuse more precisely.
    ///
    /// A span rather than a parameter threaded through `check_expr` because
    /// a target's own subscript chain is the only thing either rule turns
    /// on; nothing ever removes an entry, so this is O(element writes
    /// written in the file).
    pub write_target_levels: FxHashMap<Span, bool>,
    /// Every subscript read this run has seen under a `??`, by span.
    ///
    /// Filled by [`crate::expr::check_expr`]'s `ExprKind::Binary` arm
    /// **before** it checks the left operand, because the one thing that reads
    /// it is inside that check: the `ExprKind::Index` arm answers `?elem_ty`
    /// for a guarded read and records the fact on its
    /// [`crate::expr_table::ExprInfo::Index`] entry, which is what makes an
    /// absent key `null` rather than a throw down in `nvs-ir`.
    ///
    /// **Every level of the chain is marked, not just the outermost one.**
    /// PHP reads `$a["k"]["j"] ?? "d"` as "`"d"` unless every level is there",
    /// so marking only the `??`'s immediate operand left the inner read
    /// throwing where PHP yields the default. What that costs is one extra
    /// shape the `ExprKind::Index` arm has to accept: a guarded read answers
    /// `?elem_ty`, so a guarded level's base may be a `?array<T>`, and the arm
    /// drops the `null` before reading the element type off it — under a `??`
    /// only, because there a `null` base has an answer rather than being the
    /// untested nullable `E0482` refuses.
    ///
    /// A span rather than a parameter threaded through `check_expr` for
    /// [`Self::write_target_levels`]'s reason.
    pub coalesce_guarded: FxHashSet<Span>,
}

pub(crate) fn span_text(src: &SourceFile, span: Span) -> &str {
    src.span_text(span).unwrap_or_default()
}

/// Strips a variable's leading `$` sigil, if present.
pub(crate) fn strip_sigil(s: &str) -> &str {
    s.strip_prefix('$').unwrap_or(s)
}
