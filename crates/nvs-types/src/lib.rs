//! The Novis type checker (`rule:types/declaration`). It covers
//! `rule:types/declaration`, `rule:types/conversion`, `rule:types/grammar` and `rule:types/arithmetic` (declared-type recording, per-local
//! definite assignment, the interned type grammar, the arithmetic
//! result-type table), and it types property, method-call, `new`, `match`
//! and ternary expressions through a per-class signature table
//! ([`signatures`]).
//!
//! # Layout
//!
//! - [`ty`] — [`ty::Ty`]/[`ty::TypeId`]/[`ty::TypeInterner`]: the interned
//!   type representation everything else in this crate is built on.
//! - [`lower`] — [`lower::lower_type`]: resolves a parsed
//!   [`nvs_syntax::ast::Type`] into a [`ty::TypeId`], including
//!   `self`/`static`, `type`-alias substitution (the first real consumer of
//!   [`nvs_hir::AliasTable`]), and `rule:types/arrays`'s depth-32 array-nesting
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
//!   (including `rule:classes/comparable`'s `Comparable` amendment for `< <= > >= <=>`
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
//! - [`ctor_init`] — [`ctor_init::check_class_init`]: `rule:classes/definite-property-initialization`'s
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
//! - [`mod@derive`] — `derive::check_class_derive`: `rule:core-classes/derive-attribute`'s derive pass —
//!   which classes carry `#[Json\Derive]`, matched *nominally* against a
//!   closed `Core`-owned list, and what the field list and wire keys of each
//!   are. Runs from [`check`]'s walk because that is what holds the namespace
//!   and import set a nominal match needs; its answer is recorded in
//!   [`expr_table`] and joined against [`layout`]'s slot order by `nvs-ir`.
//! - [`consts`] — [`consts::build_const_table`]: every declared class
//!   constant's folded compile-time value, built beside [`enums`] and read
//!   only where `rule:types/constant-in-type-position`'s `Foo::CONST` appears in *type* position. See
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
//!   sequence that isn't valid UTF-8. A re-export of `nvs_syntax::string_lit`,
//!   which is the one copy of that grammar the front end has — see that
//!   module's own docs for why it sits below this crate rather than in it.
//! - [`lateinit`] — [`lateinit::check_class_lateinit_reads`]: `rule:classes/lateinit`'s
//!   `lateinit` property modifier. `signatures::build_signatures` validates
//!   where it may appear (§ 1: refusing a scalar/enum type, `?T`, a promoted
//!   parameter, and `readonly`) and excludes it from `ctor_init`'s
//!   constructor-must-assign obligation; this module adds § 3's one
//!   compile-time bonus check, a third flow-analysis pass — sibling to
//!   `ctor_init`'s, but over every method body rather than only the
//!   constructor.
//!
//! # The rules that cross several of these modules
//!
//! A rule the layout above already places is that module's. These four are
//! named here because no single entry owns one.
//!
//! - **`rule:security/tainted-qualifier`'s `tainted` and
//!   `rule:security/secret-qualifier`'s `secret`** are two independent bits
//!   over one representation, and [`expr::quals`] owns both: the sinks that
//!   refuse them, the concatenation and interpolation that poison a result on
//!   each axis independently, and the conversions that launder — a checked
//!   `as uint`/`int`/`float`/`bool`/enum-backing-type conversion clears both
//!   (a known, ADR-accepted gap for `secret`, per
//!   `rule:security/secret-propagation`), while `bytes`/`string` keep both
//!   across either direction, the identity-shaped `as string` included.
//!   `rule:security/secret-comparison-is-constant-time` is the one part that
//!   leaves this crate: the qualifier does not survive `nvs_ir::ty::Ty`, so
//!   [`expr::operators`] records
//!   [`expr_table::ExprInfo::SecretEquality`] at the comparison, and `nvs-ir`
//!   picks the helper off that entry's presence alone.
//! - **`rule:types/callable-is-a-closure`'s `callable` is a value shape**, so a
//!   bare string or an `[$obj, 'method']` array where one is expected gets a
//!   targeted diagnostic, `$obj(...)` is refused for any resolved-class
//!   `$obj`, and first-class callable syntax (`$obj->method(...)`,
//!   `Foo::bar(...)`) types as `callable` rather than as the referenced
//!   method's own return type. [`expr`]'s own docs are the detail.
//! - **A class's shape rules are one module each**: [`ctor_init`] for
//!   `rule:classes/definite-property-initialization`,
//!   [`expr::members::check_property_access`] for
//!   `rule:classes/property-observer`'s "checked on any receiver other than
//!   `$this`" half and for the written name
//!   `rule:classes/no-dynamic-properties`'s runtime-checked fallback needs,
//!   [`expr`]'s `object_comparison_result` for `rule:classes/comparable` with
//!   [`conformance`] holding the `compareTo` that a claim of it owes, and
//!   [`expr::require_stringable`] for `rule:classes/no-magic-methods`.
//!   [`error_lib`] gives `Throwable` and its subclasses a signature table
//!   without a source declaration — spec § 10's four readonly properties and
//!   the one constructor, so `$e->message` is checked like any other property
//!   read — the same way [`core_lib`] does for `Core`.
//! - **`object` is a real supertype, and a shape type is structural.** Every
//!   class or shape type is `<: object`, a [`ty::Ty::Shape`] is checked by
//!   width subtyping plus ordinary field assignability (see
//!   [`expr::is_assignable`]'s own docs), and a property access through a
//!   shape-missing field or a plain `object` erases to `mixed` rather than
//!   diagnosing. An enum atom is not a class atom: `self`/`static`/`$this`
//!   inside an enum ([`expr::class_of_ctx`], [`lower`]'s `resolve_special`)
//!   and a case access recover [`ty::Ty::Enum`] rather than
//!   [`ty::Ty::Class`], and `rule:enums/closed-integer-type`'s two refusals —
//!   an arithmetic or bitwise operator over a value that can be an enum case,
//!   and a conversion from one enum type to a different one — are [`expr`]'s
//!   `report_enum_operand`/`reject_enum_to_enum_conversion`.
//! - **Equality-operand compatibility is one pass over one table.**
//!   [`expr::operators`]'s `types_are_disjoint` partitions every modeled type
//!   into `rule:expressions/disjoint-comparison-refused`'s domains and refuses
//!   a comparison landing in two of them (`E0466`) — from `==`/`!=`, and from a
//!   `switch` label and a `match` arm against their subject, which are the same
//!   comparison written without the operator. Each enum is its own domain, so
//!   two different enum types are refused for the reason an enum and the
//!   integer under it are; `int`, `uint`, `float` and `decimal` are one domain,
//!   so no pairing among them ever is. The pass is deliberately one-sided —
//!   `mixed`, an intersection, a type variable and any class this compilation
//!   did not declare all read as *may overlap*, because a missed diagnostic
//!   costs an author nothing and a wrong one costs them a program that used to
//!   build.
//! - **A declared return type is answered for at both of a body's exits.**
//!   [`returns`] walks for a path that reaches the closing brace (`E0739`) and
//!   [`check`]'s `check_body_exits` refuses a written `return;` under anything
//!   but `void` (`E0822`), over every block body a declared type is checked
//!   against: a method's, a `get` hook's and a block-bodied `fn`'s.
//!   `rule:php-migration/a-body-never-falls-off-its-end` owns the pair.
//!   Definite assignment is likewise not conservative around the block
//!   statements: a `switch` and a `try`/`catch` intersect their arms' live sets
//!   the way an `if` does.

pub(crate) mod attributes;
pub(crate) mod callables;
pub(crate) mod capability;
pub mod check;
// Public for [`routes`]'s reason: `rule:tooling/commands-are-compiled`'s finished table is read back
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
pub mod paths;
pub(crate) mod program;
pub(crate) mod reasons;
pub(crate) mod response;
pub(crate) mod retrieval;
pub(crate) mod returns;
pub mod routes;
pub mod signatures;
/// Re-exported, not owned: the escape grammar lives in `nvs-syntax`, the one
/// crate `nvs-hir`'s `require` resolution, this checker and `nvs-ir`'s lowering
/// all depend on. The path stays `nvs_types::string_lit` because that is where
/// a reader of a checker diagnostic about a `\u{...}` codepoint looks first.
pub use nvs_syntax::string_lit;
pub mod testing;
pub mod ty;

pub use check::{HOOK_VALUE_PARAM, check_program, check_program_granted};
pub use core_lib::symbol_of as core_symbol_of;
pub use defaults::ConstArg;
pub use derive::{DerivedCodec, DerivedField};
pub use enums::{EnumBacking, EnumInfo, EnumTable, EnumValue};
pub use expr_table::{
    Delegation, EnumSpelling, ExprId, ExprInfo, ExprTypeTable, ForeachDrive, LocalBinding,
    ResolvedCall, UrlPiece,
};
pub use layout::{
    ClassAttribute, ClassConstant, ClassLayout, ClassLayoutTable, MethodEntry, build_class_layouts,
};
/// The class `Core\Script::finish()` raises, for `nvs-ir` to build an instance
/// of — that crate reaches the compiler's exception tree through this crate,
/// exactly as it reaches every `Core` symbol below.
pub use nvs_hir::errors::FINISH_MARKER as CORE_SCRIPT_FINISH_CLASS;
/// The word `nvs_stdlib::registry::PREPARED_MEMBERS`' argument 0 carries for a
/// literal CLDR date pattern, reached from `nvs-ir` through this crate exactly
/// as every other `Core` symbol below is.
pub use nvs_stdlib::cldr::PREPARED_PATTERN as CORE_CLDR_PREPARED_PATTERN;
pub use nvs_stdlib::cli::{
    NAME as CORE_CLI_TEXT_CLASS, TEXT_CONCAT_SYMBOL as CORE_CLI_TEXT_CONCAT,
};
pub use nvs_stdlib::html::{
    ESCAPE_TEXT_SYMBOL as CORE_HTML_ESCAPE_TEXT, MARKUP_CONCAT_SYMBOL as CORE_HTML_MARKUP_CONCAT,
    MARKUP_NAME as CORE_HTML_MARKUP_CLASS, MARKUP_SYMBOL as CORE_HTML_MARKUP,
    MARKUP_TEXT_SYMBOL as CORE_HTML_MARKUP_TEXT,
};
/// `Core\Regex`'s two engines as `nvs_stdlib::regex::PREPARED_NONE` documents
/// them, and the zero word beside them — the encoding
/// `nvs_stdlib::registry::PREPARED_MEMBERS`' argument 0 carries, reached from
/// `nvs-ir` through this crate exactly as every other `Core` symbol below is.
pub use nvs_stdlib::regex::{
    PREPARED_BACKTRACKING as CORE_REGEX_PREPARED_BACKTRACKING,
    PREPARED_LINEAR as CORE_REGEX_PREPARED_LINEAR, PREPARED_NONE as CORE_REGEX_PREPARED_NONE,
    Tier as RegexTier,
};
pub use nvs_stdlib::registry::constructor_symbol as core_constructor_symbol;
pub use nvs_stdlib::registry::takes_call_site as core_takes_call_site;
pub use nvs_stdlib::registry::takes_prepared as core_takes_prepared;
pub use nvs_stdlib::registry::takes_source as core_takes_source;
pub use nvs_stdlib::registry::takes_written_class as core_takes_written_class;
pub use nvs_stdlib::router::link::{
    ABSOLUTE_SYMBOL as CORE_ROUTE_LINK_ABSOLUTE, SIGNED_SYMBOL as CORE_ROUTE_LINK_SIGNED,
    SYMBOL as CORE_ROUTE_LINK,
};
pub use nvs_stdlib::script::{
    AWAIT_SYMBOL as CORE_SCRIPT_AWAIT, FINISH_SYMBOL as CORE_SCRIPT_FINISH,
    SPAWN_METHOD_SYMBOL as CORE_SCRIPT_SPAWN_METHOD, SPAWN_SYMBOL as CORE_SCRIPT_SPAWN,
};
pub use nvs_stdlib::time::FROM_NANOS_SYMBOL as CORE_DURATION_FROM_NANOS;
pub use nvs_stdlib::{CodecElement, CodecField, CodecTy, EnumCases, FieldDefault};
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
    /// The name of the property whose `rule:classes/property-hooks` hook body is being
    /// checked, if any — `$`-sigil not included.
    ///
    /// Exists for one rule: inside `$p`'s own `get`/`set` hooks,
    /// `$this->p` is the backing slot rather than a re-entrant call to the
    /// hook currently running. That is what makes a hook that transforms a
    /// stored value terminate, and it is the only place the checker needs to
    /// know *which* accessor it is inside.
    pub current_hook: Option<&'a str>,
    /// Whether the body being checked is the `constructor`'s own — the one
    /// place `rule:classes/lateinit-restrictions` lets a write to a `readonly` property through
    /// (`crate::expr::assign::check_write_target`).
    ///
    /// A `bool` rather than the method's name because that is the whole of
    /// what any rule asks, and `false` in a closure body written inside the
    /// constructor: a closure is called at a time this checker cannot bound,
    /// so the write it holds is not proven to happen during construction.
    pub in_constructor: bool,
    /// The element type `T` of the `Iterator<T>` the enclosing body is a
    /// generator for (`rule:iteration/generators`), or `None` in an ordinary body.
    ///
    /// One field answers both of `yield`'s questions: whether it is legal
    /// here at all, and what its operand has to satisfy. `crate::check`'s
    /// `check_method` is the only place it is ever set, from
    /// `nvs_syntax::ast::is_generator_body` plus the declared return type.
    pub generator_elem: Option<crate::ty::TypeId>,
    /// Whether the body being checked is a closure's. A closure is lifted to
    /// a frame of its own that carries neither a receiver nor a called class,
    /// so `static::` has nothing to bind to inside one and is refused there
    /// (`E0834`, [`crate::expr::report_class_keyword_outside_class`]). Set
    /// only where a closure literal's body is entered.
    pub in_closure: bool,
}

/// `rule:types/closure-self-name`'s
/// optional self-name, resolved: what a bare call written inside the closure's
/// own body has to spell to mean *this* closure, and what such a call answers
/// with.
///
/// Carries the signature itself rather than a `TypeId` for the closure,
/// because there is none to carry: § 4 gives every closure the one opaque
/// `callable`, so the recursive call's own types can only come from what the
/// literal declared. A literal that declared no return type gets `mixed` here —
/// its body is mid-check, so its inferred type is not a fact yet, and `mixed`
/// is the same answer every other call through a `callable` gives.
pub(crate) struct FnSelf {
    /// The name as written, which a callee spelling must equal exactly:
    /// § 3's name is lexical and is not resolved through the namespace or the
    /// `use` table.
    pub name: String,
    /// The literal's own parameter types, in written order — what
    /// `rule:types/callable-signature` checks a recursive call's arguments
    /// against, the closure being written *being* the signature. Unlike `ret`
    /// this is never a stand-in: a parameter is annotated or takes its type
    /// from the position the literal is written in
    /// (`rule:types/callable-literal-inference`), so the list is a fact before
    /// the body is checked.
    pub params: Vec<crate::ty::TypeId>,
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
    /// ([`consts::build_const_table`]) — read only where `rule:types/constant-in-type-position`'s
    /// `Foo::CONST` appears in *type* position and has to fold to its own
    /// literal type. Built beside [`Self::enums`], and before
    /// [`signatures::build_signatures`], for the same reason: an annotation
    /// interned during signature collection may be one of these.
    pub consts: &'a crate::consts::ConstTable,
    /// Every `#[...]` attach site in the program, indexed by the declaration
    /// it is attached to — `rule:attributes/structural-retrieval`'s retrieval reads it, and nothing else
    /// does. Built whole before any body is checked, because a retrieval may
    /// be written above the declaration it asks about; see
    /// [`crate::retrieval::AttributeTable`].
    pub attributes: &'a crate::retrieval::AttributeTable<'a>,
    /// The `[capabilities]` block of the configuration the *compiling* machine
    /// read, or `None` where nothing read one — `rule:core-classes/db-literal-query-checking`'s "`nvs.toml` is read at boot on the machine that compiles",
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
    /// The file's top-level statements, which is where the `use` line an
    /// undeclared name's fix inserts would go (`nvs_hir::import_site`) — the
    /// same statements [`ProgramFile::stmts`] hands the passes, kept beside
    /// [`Self::src`] so a diagnostic raised deep in a body can name the site.
    pub stmts: &'a [nvs_syntax::ast::Stmt],
    /// Every file of the program, so a `type` alias's expansion is read in the
    /// file that declares it ([`crate::lower::in_alias_site`]) when the file
    /// being checked is another one.
    pub files: &'a [ProgramFile<'a>],
    pub interner: &'a mut TypeInterner,
    /// Where a call's/`new`'s resolved target is persisted for `nvs-ir` to
    /// read back later — see [`crate::expr_table`]'s own module docs.
    pub exprs: &'a mut crate::expr_table::ExprTypeTable,
    /// `rule:routing/table-is-opt-in`'s route table as it is collected — one row per `#[Route]`
    /// the per-class walk reaches, across every file.
    ///
    /// Here rather than a local of [`crate::check::check_program`]'s loop
    /// because both of the errors [`crate::routes::check_table`] reports are
    /// collisions *between* declarations, and § 5's scan is what puts the two
    /// colliding files in the same program in the first place. The pass that
    /// only collects signatures hands it a scratch table, exactly as it does
    /// [`Self::exprs`].
    pub routes: &'a mut crate::routes::RouteTable,
    /// `rule:tooling/commands-are-compiled`'s command table as it is collected — one row per
    /// `#[Command]` the per-class walk reaches, across every file.
    ///
    /// Beside [`Self::routes`] and threaded exactly as it is, for its reason:
    /// the error [`crate::commands::check_table`] reports is a collision
    /// *between* declarations, and `rule:programs/implementing`'s scan is what puts the two
    /// colliding files in the same program.
    pub commands: &'a mut crate::commands::CommandTable,
    /// `rule:routing/link-name-and-params-are-checked`'s `Core\Router::url`/`urlAbsolute` sites, as the walk
    /// reaches them and before any of them has been looked up.
    ///
    /// Beside [`Self::routes`] and threaded the same way for a stronger form of
    /// the same reason: a link asks the finished table a question, and the walk
    /// filling that table is this one. See [`crate::links`] for why the lookup
    /// cannot be made where the call is written.
    pub links: &'a mut Vec<crate::links::LinkSite>,
    /// `rule:core-classes/derive-field-list`'s codec-reachable question, one entry per field the derive
    /// pass keeps, asked after the walk rather than where the property is
    /// declared.
    ///
    /// [`Self::links`]'s reason exactly: "another class that itself has a
    /// codec" is a question about the whole program, and a class whose field
    /// names a deriving class declared in a later file must not answer
    /// differently from one that names an earlier one. See
    /// [`crate::derive::resolve_field_types`].
    pub codec_sites: &'a mut Vec<crate::derive::CodecFieldSite>,
    /// Every `queryAs<T>` and `streamAs<T>` this run has walked past, for exactly
    /// [`Self::codec_sites`]' reason one layer out: the row class a call names
    /// is routinely declared in a later file than the call. See
    /// [`crate::derive::check_row_sites`].
    pub row_sites: &'a mut Vec<crate::derive::RowSite>,
    /// Every call this run has walked past that builds a written class out of a
    /// **document** — the rest of the same roster, which
    /// [`crate::derive::hydrates_a_row`] partitions. [`Self::row_sites`]' reason
    /// exactly, and one of its own: a document is a tree, so the question is
    /// asked of every deriving class the written one reaches and all of them
    /// have to have recorded their fields first. See
    /// [`crate::derive::check_json_sites`].
    pub json_sites: &'a mut Vec<crate::derive::JsonSite>,
    /// Every `Core\Request::jsonAs<T>` this run has walked past, for
    /// [`Self::row_sites`]' reason and one of its own:
    /// `rule:security/derived-codec-qualifiers` asks its question of the whole
    /// codec — the written class's text fields and those of every deriving
    /// class they name — so it can only be answered once all of them have
    /// recorded their fields. See [`crate::derive::check_decode_sites`].
    pub decode_sites: &'a mut Vec<crate::derive::DecodeSite>,
    pub diags: &'a mut nvs_diagnostics::Diagnostics,
    /// How many `rule:types/closure-literal` `fn` closure literals this run has checked so far —
    /// the suffix that makes each one's synthesized environment class label
    /// unique. One counter for the whole run rather than one per body,
    /// because a closure nested inside another closure has no enclosing
    /// declaration of its own to be numbered within.
    pub closure_seq: u32,
    /// How many `ExprKind::Error` nodes [`crate::expr::infer`] has typed so far
    /// in this file — an expression the parser already refused and reported.
    ///
    /// [`crate::expr::check_expr`] reads it before and after inferring an
    /// expression: a change means the expression contains a refused node, so
    /// its type is a stand-in (`mixed`, or a generic result inferred from
    /// `mixed`) and no mismatch is reported against it. The parser's own error
    /// is the one the reader needs, and the program fails to compile on it
    /// regardless, so nothing reaches a human less checked. An ordinary
    /// `mixed` value never moves the count.
    pub refused_exprs: u32,
    /// `rule:types/closure-self-name`'s self-name, for the `fn` literal whose body is being checked —
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
    /// restored across an `rule:types/closure-literal` closure literal's body, which no enclosing
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
    /// Every read this run has seen under a `??`, an `isset` or an `empty`,
    /// by span — a subscript level and a property level alike, since
    /// `rule:types/shape-type`'s optional field asks the same "may it be
    /// absent" question of a shape that a key asks of an array.
    ///
    /// Filled by [`crate::expr::presence::mark_guarded_places`] **before** the
    /// operand is checked, because the things that read it are inside that
    /// check: the `ExprKind::Index` arm answers `?elem_ty` and records
    /// `guarded` on its [`crate::expr_table::ExprInfo::Index`] entry, and
    /// [`crate::expr::members::check_property_member`] does the same for a
    /// shape's optional field — which is what makes absence `null` rather than
    /// a throw down in `nvs-ir`.
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
    /// `rule:security/response-body-is-one-typed-member`'s sixth row, as the body being checked has answered it so
    /// far: what has already written this response's body.
    ///
    /// Installed and put back per method body by [`crate::check`], exactly as
    /// [`Self::exit_targets`] is across a closure and for the same reason — the
    /// rule is about one body, and a second body's writers are not this one's.
    /// [`crate::response`] owns which bodies it is armed for.
    pub body_writers: crate::response::BodyWriters,
    /// Whether the expression being checked sits inside a call's argument list.
    ///
    /// One rule reads it: `rule:security/secret-qualifier`'s container refusal
    /// (`crate::expr::quals::reject_secret_into_container`), which steps aside
    /// for an argument because `rule:security/secret-sinks-refuse` names three
    /// positions a credential legitimately reaches — a bound database
    /// parameter, a process argv, an outbound request — and all three are
    /// written as an `array<mixed>` argument. What decides an argument is the
    /// call's own rules, which are the sink refusals in that module.
    ///
    /// Set and put back around a whole argument list by
    /// [`crate::expr::args::check_args_typed`], the way [`Self::exit_targets`]
    /// is around a closure body: a container literal nested inside an argument
    /// is still inside that argument, and the flag has to survive the recursion
    /// rather than be re-derived at each level.
    pub in_call_argument: bool,
    /// The span of every argument a registry row's
    /// `nvs_stdlib::registry::CoreTy::MethodRef` parameter gives a meaning to —
    /// `rule:testing/interaction-after-the-fact`'s method reference.
    ///
    /// A set of spans rather than a flag, because unlike
    /// [`Self::in_call_argument`] this is about *one* argument of a call and
    /// not about a region: `Mailer::send` is an undefined constant one
    /// argument over, and has to stay one. [`crate::expr::calls`] marks the
    /// span before the argument list is checked and
    /// [`crate::expr::members::infer_class_const`] reads it at the argument
    /// itself; nothing clears it, since a span identifies its own call site.
    pub method_ref_args: FxHashSet<Span>,
}

pub(crate) fn span_text(src: &SourceFile, span: Span) -> &str {
    src.span_text(span).unwrap_or_default()
}

/// Strips a variable's leading `$` sigil, if present.
pub(crate) fn strip_sigil(s: &str) -> &str {
    s.strip_prefix('$').unwrap_or(s)
}
