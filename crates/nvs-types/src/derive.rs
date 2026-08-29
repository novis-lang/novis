//! [ADR 0071](../../../../docs/adr/0071-derived-codecs.md)'s derive pass:
//! which classes carry `#[Json\Derive]`, which of their properties are fields,
//! and what wire key each field has.
//!
//! # The nominal match, and why it lives here
//!
//! ADR 0071 § 1 makes a **compiler-recognized** attribute the one thing matched
//! by name rather than by shape: the compiler acts on `#[Json\Derive]` only
//! when that `Name` *resolves* — through the ordinary namespace and `use` rules
//! — to one of a closed, `Core`-owned list. Resolution is exactly
//! [`nvs_hir::resolve_ref`], and the active namespace and import set are things
//! only [`crate::check`]'s walk holds, so this pass runs from that walk rather
//! than as a second traversal of its own. [`crate::Ctx`] carries both.
//!
//! [`ATTRIBUTES`] is that closed list. Nothing else is ever matched by name;
//! `Core\Attributes::get<T>`/`::all<T>` retrieval stays structural
//! ([ADR 0046](../../../../docs/adr/0046-attributes-shape-literal-metadata.md)
//! § 4), and a userland `type Derive = {};` resolves to a different `QName` and
//! generates nothing.
//!
//! # What the pass produces
//!
//! One [`DerivedCodec`] per deriving class, recorded into
//! [`crate::ExprTypeTable`] — the table this crate already publishes for
//! `nvs-ir` to read back. It carries the *property* name rather than a slot
//! index, because the slot order is
//! [`crate::layout::ClassLayout`]'s and that table is built after checking; the
//! two are joined in `nvs_ir::lower::lower_file`, which holds both. Each field
//! also carries the declared type a decoder checks against, erased to
//! [`nvs_stdlib::CodecTy`], and the *constructor position* it fills — ADR 0071
//! § 2's "a decode is an ordinary `new`" resolved to an index, so that nothing
//! below this line looks a parameter up by name.
//!
//! # Known gaps
//!
//! 1. **A promoted constructor parameter is not a field**, because
//!    [`crate::layout`] does not give one a slot yet (its own gap 1) and
//!    [`crate::signatures`] does not record it as a property. ADR 0071 § 1's
//!    own example is written with promotion, so this is the first thing to
//!    close — until then a deriving class must declare its properties.
//! 2. **A reachable type whose decoder is not written yet is still
//!    [`CodecTy::Opaque`].** § 2's compile-time refusal is applied — see
//!    [`resolve_field_types`] — but it names only the types that can never
//!    have a wire form. A `decimal`, an `Instant`, an enum, an `array<T>`, an
//!    inline shape and a nested derived class are all *reachable* and all
//!    erase to `Opaque` here, so a `decodeAs<T>` over one still refuses at run
//!    time; the decoders they need are `nvs_stdlib::json`'s own gap, and
//!    keeping the two apart is why this module refuses a type rather than
//!    refusing an `Opaque`.
//! 3. **`#[Db\Derive]`/`#[Db\Field]` resolve to nothing.** `Core\Db` is M8's,
//!    so the two names are deliberately not in [`ATTRIBUTES`] yet: a closed
//!    list that names something with no pass behind it is worse than a short
//!    one.

use nvs_diagnostics::{Diagnostic, Diagnostics, Span, code};
use nvs_hir::QName;
use nvs_syntax::ast::{
    Attribute, AttributeGroup, ClassDecl, ClassMemberKind, ExprKind, Modifier, ObjectLiteralField,
    Param, PropertyMember,
};

use nvs_stdlib::CodecTy;

use crate::ty::{Ty, TypeId};
use crate::{Ctx, Env, span_text, strip_sigil};

/// ADR 0071 § 1's closed, `Core`-owned list of compiler-recognized attribute
/// names, each already fully qualified.
///
/// A `Name` written at an attribute site is resolved with
/// [`nvs_hir::resolve_ref`] and compared against these — so `#[Core\Json\Derive]`
/// and a `use Core\Json;`d `#[Json\Derive]` are the same attribute, and no
/// spelling of a userland name is any of them. Extended, never widened, and
/// each entry owes its own argued ADR section.
///
/// A name on this roster names no shape, so [`crate::attributes`]'s ADR 0046
/// § 1 rule does not apply to it and what its payload may hold is the
/// recognizing pass's own question: `#[Json\Derive]`/`#[Json\Field]` are this
/// module's, `#[Test]` is [`crate::testing`]'s, `#[Command]`/`#[Option]` are
/// [`crate::commands`]', `#[Route]` is [`crate::routes`]'.
pub const ATTRIBUTES: &[&str] = [
    DERIVE, FIELD, TEST, FIXTURE, TEST_WITH, COMMAND, OPTION, ROUTE,
]
.as_slice();

/// `#[Json\Derive]` — ADR 0071 § 1's opt-in, on a class.
pub const DERIVE: &str = r"Core\Json\Derive";

/// `#[Json\Field(name?: string, skip?: bool)]` — ADR 0071 § 3's per-field
/// override, on a property.
pub const FIELD: &str = r"Core\Json\Field";

/// `#[Test(skip?: string, …)]` — ADR 0079 § 1's marker, on a method. It is
/// the class the assertions are members of, so the `use Core\Test;` that lets
/// a test body write `Test::assertEquals(…)` is the same one that places the
/// attribute; [`crate::testing`] owns the payload.
pub const TEST: &str = r"Core\Test";

/// `#[Fixture]` — ADR 0079 § 8's marker, on a `static` method whose return
/// type is what a test asks for by declaring a parameter of it. It sits in
/// the `Core\Test` namespace beside [`crate::error_lib`]'s `Core\Test\Failure`
/// rather than being a second segment of the class itself, because it names
/// no member of anything: it is a recognized name and nothing else, so a file
/// that writes it bare places it with `use Core\Test\Fixture;`.
/// [`crate::testing`] owns what it may carry, which is nothing.
pub const FIXTURE: &str = r"Core\Test\Fixture";

/// `#[TestWith(...)]` — ADR 0079 § 9's data row, on a `#[Test]` method. It
/// sits in the `Core\Test` namespace beside [`FIXTURE`] and for that entry's
/// reason exactly, and it keeps the ADR's own spelling rather than the
/// shorter `With` a namespace would allow: `#[TestWith]` is what § 9 writes,
/// and a file that spells it bare places it with `use Core\Test\TestWith;`.
///
/// It is on this roster rather than being an ADR 0046 § 1 shape alias because
/// the shape it is checked against is not written anywhere: it is the
/// *parameter list* of the method it is attached to, which only
/// [`crate::testing::check_class_tests`] holds. That module owns what a row
/// may carry.
pub const TEST_WITH: &str = r"Core\Test\TestWith";

/// `#[Command(name: string, about?: string)]` — ADR 0086 § 6's marker, on a
/// `static` method. It is the same class the entry point `Core\Command::run`
/// is a member of, so the `use Core\Command;` that lets a program spell the
/// attribute bare is the one that reaches the runner too;
/// [`crate::commands`] owns the payload.
pub const COMMAND: &str = r"Core\Command";

/// `#[Option(short?: string, long?: string, about?: string)]` — ADR 0086 § 6's
/// per-parameter marker, and the one sentence that decides what a parameter is:
/// a parameter is a positional argument unless it carries this, with no
/// inference from defaults or types. [`crate::commands`] owns the payload.
pub const OPTION: &str = r"Core\Option";

/// `#[Route(path: string, method: Core\Http\Method, name?: string)]` — ADR 0077
/// § 1's route declaration, on a method, and repeatable (ADR 0046 § 3) so one
/// method serves two verbs. It names no member of anything — the table is read
/// back through the separate `Core\Router` class — so a file that spells it
/// bare places it with `use Core\Route;`, and importing the router instead
/// imports a different name. [`crate::routes`] owns the payload.
pub const ROUTE: &str = r"Core\Route";

/// One derived class's JSON field list, in declaration order — ADR 0071 § 2's
/// "declaration order fixes encode order, so output is byte-deterministic".
#[derive(Clone, Debug, Default)]
pub struct DerivedCodec {
    /// Every field the codec reads and writes, in declaration order. A
    /// `#[Json\Field(skip: true)]` property is absent rather than marked.
    pub fields: Vec<DerivedField>,
    /// How many parameters the class's `constructor` declares — what a
    /// generated decoder has to fill before it can run one. Zero for a class
    /// that declares none, which [`check_constructor_parameter`] has already
    /// reported through [`crate::ctor_init`].
    pub ctor_arity: usize,
}

/// One field of a [`DerivedCodec`], as read off the *declaration*.
///
/// The slot-resolved half is [`nvs_stdlib::CodecField`]; the two are joined in
/// `nvs_ir::lower::lower_file`, which is the one place both this table and
/// [`crate::layout`]'s slot order are in hand.
#[derive(Clone, Debug)]
pub struct DerivedField {
    /// The declaring property's own name, `$`-sigil stripped — the key into
    /// [`crate::layout::ClassLayout::slot_of`].
    pub property: String,
    /// The JSON key this field is written under: the property's own name, or
    /// `#[Json\Field(name: "...")]`'s override.
    pub key: String,
    /// What a decode has to produce for this field — the declared property
    /// type, erased to the closed roster a native decoder branches on.
    pub ty: CodecTy,
    /// Whether the declared type admits `null` (ADR 0071 § 4's second column).
    pub nullable: bool,
    /// This field's position in the constructor's parameter list, or `None`
    /// when the class declares no matching parameter — which
    /// [`check_constructor_parameter`] has already reported.
    pub param: Option<usize>,
}

/// `declared`, erased to what a native decoder branches on.
///
/// ADR 0071 § 2's codec-reachable set is wider than this: an enum, a
/// `decimal`, an `Instant`, an `array<T>`, a nested derived class and an
/// inline shape are all reachable and all land on [`CodecTy::Opaque`] today —
/// `nvs_stdlib::json`'s own gap owns the decoders they still need, and § 2's
/// compile-time refusal of a genuinely unreachable type is this module's
/// gap 3. Nothing here narrows what *encodes*, which walks the value rather
/// than the declared type.
fn codec_ty(declared: TypeId, env: &Env<'_>) -> CodecTy {
    match env.interner.get(declared) {
        Ty::Bool => CodecTy::Bool,
        Ty::Int => CodecTy::Int,
        Ty::Uint => CodecTy::Uint,
        Ty::Float => CodecTy::Float,
        // A `tainted` string is still a string on the wire; ADR 0071 § 6 makes
        // the qualifier a call-site question, not a decoder one.
        Ty::String | Ty::TaintedString => CodecTy::Str,
        Ty::Mixed => CodecTy::Mixed,
        _ => CodecTy::Opaque,
    }
}

/// One field ADR 0071 § 2's codec-reachable test still owes an answer,
/// recorded as the walk reaches it and resolved by [`resolve_field_types`].
///
/// Carries the *null-stripped* declared type, because `?T` is reachable
/// exactly when `T` is, and the property's own span, because § 2's refusal is
/// at the declaration that wrote the field rather than at the `decodeAs<T>`
/// that would later fail on it.
#[derive(Debug)]
pub struct CodecFieldSite {
    /// The deriving class, for the message alone.
    class: String,
    /// The property's name, `$` stripped.
    property: String,
    /// Where that name is written.
    span: Span,
    /// The declared type, with `?`'s `null` arm already removed.
    declared: TypeId,
}

/// ADR 0071 § 2's "a field's type must be codec-reachable", once every
/// deriving class in the program has recorded its codec.
///
/// Run after the walk, from [`crate::check::check_program`], for
/// [`crate::links::resolve`]'s reason: a field naming another deriving class
/// must not depend on which file declared it first.
///
/// **What it refuses is the unreachable set, not the undecoded one.** § 2
/// lists a `decimal`, an `Instant`, an enum, an inline shape, an `array<T>`
/// and a nested codec-carrying class as reachable; several of them still
/// erase to [`CodecTy::Opaque`] and are refused by `nvs_stdlib::json` when a
/// `decodeAs<T>` runs, which is that crate's missing decoders and not a
/// contract error. Refusing an `Opaque` here would report those as if the
/// program were wrong — so the test is over the declared type, and this
/// module's gap 2 owns the difference.
pub(crate) fn resolve_field_types(
    sites: &[CodecFieldSite],
    signatures: &crate::signatures::SignatureTable,
    interner: &crate::ty::TypeInterner,
    exprs: &crate::expr_table::ExprTypeTable,
    diags: &mut Diagnostics,
) {
    for site in sites {
        if reachable(site.declared, interner, signatures, exprs) {
            continue;
        }
        let spelling = interner.describe(site.declared);
        let class = &site.class;
        let property = &site.property;
        diags.report(
            Diagnostic::error(
                code::E_DERIVE_FIELD_NOT_CODEC_REACHABLE,
                format!(
                    "`{class}::${property}` is declared `{spelling}`, which has no JSON \
                     representation"
                ),
            )
            .with_primary(site.span, "this type cannot be encoded or decoded")
            .with_help(
                "ADR 0071 § 2: a field is a scalar, an enum, an inline shape, an `array<T>` or \
                 `?T` of one of those, or another class that itself carries a codec — write \
                 `#[Json\\Field(skip: true)]` to leave it off the contract",
            ),
        );
    }
}

/// § 2's reachable set, over the interned type rather than over
/// [`CodecTy`]'s erasure — see [`resolve_field_types`] for why the two are
/// not the same question.
fn reachable(
    ty: TypeId,
    interner: &crate::ty::TypeInterner,
    signatures: &crate::signatures::SignatureTable,
    exprs: &crate::expr_table::ExprTypeTable,
) -> bool {
    match interner.get(ty) {
        // The scalars, plus ADR 0047's three singleton refinements of them:
        // each erases to a scalar and is exactly as spellable on the wire.
        Ty::Null
        | Ty::Bool
        | Ty::Int
        | Ty::Uint
        | Ty::Float
        | Ty::Decimal
        | Ty::String
        | Ty::TaintedString
        | Ty::Mixed
        | Ty::True
        | Ty::False
        | Ty::StringLiteral(_)
        | Ty::IntLiteral(_) => true,
        // ADR 0010: an enum travels as its backing value, range-checked on
        // the way back in.
        Ty::Enum(..) | Ty::EnumCase(..) => true,
        // ADR 0036's inline shape, and `array<T>`/`array<string, T>`, both
        // reachable exactly when what they hold is.
        Ty::Shape(fields) => fields
            .iter()
            .all(|(_, held)| reachable(*held, interner, signatures, exprs)),
        Ty::Array(elem) => reachable(*elem, interner, signatures, exprs),
        Ty::Class(class, _) => class_has_codec(class, signatures, exprs),
        // Everything else: `bytes` has no JSON spelling (ADR 0009 makes it a
        // separate type for that reason), `object`, `callable` and `iterable`
        // name no contract, and a union of two non-`null` arms gives a
        // decoder nothing to pick between — § 2 lists none of them.
        _ => false,
    }
}

/// Whether `class` participates in the JSON wire format at all — ADR 0071
/// § 2's "another class that itself has a codec — derived or hand-written".
fn class_has_codec(
    class: &QName,
    signatures: &crate::signatures::SignatureTable,
    exprs: &crate::expr_table::ExprTypeTable,
) -> bool {
    let label = class.to_string();
    if exprs.codec(&label).is_some() {
        return true;
    }
    match signatures.get(class) {
        // § 7's hand-written half, either one of them: a class writing only
        // `toJson` gets the decoder generated, so one declared half is a
        // codec as much as two are.
        Some(sig) => {
            sig.methods.contains_key(ENCODE)
                || sig.methods.contains_key(DECODE)
                // A `Core` value type — § 2's `Instant`, `Duration`, `Uuid`
                // and their siblings — declares no member for this to find:
                // which of them has a wire form is `nvs_stdlib::json`'s
                // roster, not this pass's, and gap 2 is where the missing
                // decoders are owed.
                || label.starts_with(r"Core\")
        }
        // A name with no signature is a name that did not resolve, and that
        // has already been reported where it was written; a second
        // diagnostic here would name the same property for the same reason.
        None => true,
    }
}

/// Records `decl`'s [`DerivedCodec`] if it carries `#[Json\Derive]`, reporting
/// every ADR 0071 § 2/§ 3/§ 6 rule it breaks.
///
/// A no-op — not even a walk of the members — for a class with no recognized
/// attribute, which ADR 0071 § 8 requires: "a program with no derive attribute
/// pays nothing at all, including no pass".
pub(crate) fn check_class_derive(
    decl: &ClassDecl,
    class: &QName,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    let Some(attribute) = attribute_span(&decl.attributes, DERIVE, ctx, env) else {
        return;
    };
    // ADR 0071 § 7: the derive generates only what the class does not write
    // itself, so a class writing both halves gets nothing from it. Reported
    // before anything is collected and returning without a codec, because
    // "generates nothing" is the rule rather than a description of the error.
    if declares_method(decl, ENCODE, env) && declares_method(decl, DECODE, env) {
        env.diags.report(
            Diagnostic::error(
                code::E_DERIVE_BOTH_HALVES,
                format!(
                    "`{class}` declares both `{ENCODE}` and `{DECODE}`, so `#[Json\\Derive]` \
                     generates nothing"
                ),
            )
            .with_primary(attribute, "this attribute has no effect")
            .with_help(
                "ADR 0071 § 7: the derive fills in the half a class does not write — keep one \
                 of the two and the attribute generates the other, or delete the attribute",
            ),
        );
        return;
    }
    let params = constructor_params(decl, env);
    let mut codec = DerivedCodec {
        ctor_arity: params.map_or(0, <[Param]>::len),
        ..DerivedCodec::default()
    };
    // Whether any property was *refused* rather than skipped — see the empty
    // contract check below, which a refusal must not also report.
    let mut refused = false;
    for member in &decl.members {
        let ClassMemberKind::Property(p) = &member.kind else {
            continue;
        };
        if p.modifiers.contains(&Modifier::Static) {
            continue; // ADR 0008: class storage, not instance storage
        }
        match codec_field(p, class, params, ctx, env) {
            FieldOutcome::Kept(field) => codec.fields.push(field),
            FieldOutcome::Skipped => {}
            FieldOutcome::Refused => refused = true,
        }
    }
    // § 2's field list is the declared property list, so no property is an
    // empty wire contract — § 7's rule about an attribute with no effect,
    // reaching the other way a derive can generate nothing. The codec is
    // still recorded: the program does not run with an error reported, and
    // recording it keeps every reader of the table off a special case.
    //
    // A class whose properties were all *refused* is excluded: it has already
    // been told what is wrong with each of them, and "the contract came out
    // empty" is the same mistake counted a second time. A class that skipped
    // them all in writing is not excluded — that is a contract it chose.
    if codec.fields.is_empty() && !refused {
        env.diags.report(
            Diagnostic::error(
                code::E_DERIVE_NO_FIELDS,
                format!("`{class}` derives a JSON codec with no fields in it"),
            )
            .with_primary(attribute, "no declared property reaches the wire contract")
            .with_help(
                "ADR 0071 § 2: the field list is the class's own declared instance properties \
                 — declare one, or delete the attribute. A promoted constructor parameter is \
                 not a property yet (this module's gap 1)",
            ),
        );
    }
    env.exprs.record_codec(class.to_string(), codec);
}

/// The two members [`docs/spec/01-core-library.md`] § 6's `Core\Json\Codec`
/// declares, which are also the two halves ADR 0071 § 7 talks about.
const ENCODE: &str = "toJson";
/// The decoding half of [`ENCODE`] — `static fromJson(mixed $value): static`.
const DECODE: &str = "fromJson";

/// Whether `decl` writes a method named `want` **itself**.
///
/// Own members only: § 7 is about a class that hand-writes a half, and an
/// inherited `toJson` is the base class's declaration, whose own derive
/// question was already asked where it was written.
fn declares_method(decl: &ClassDecl, want: &str, env: &Env<'_>) -> bool {
    decl.members.iter().any(|member| match &member.kind {
        ClassMemberKind::Method(m) => span_text(env.src, m.name) == want,
        _ => false,
    })
}

/// The span of the first `want`-named attribute in `groups`, or `None` when
/// none of them is it.
///
/// Both a "does this class carry it" test and the site it matched at, because
/// the two refusals § 7 reaches put their primary span on the attribute
/// rather than on any member — a class with no property has no member to
/// point at, and the attribute is the thing to delete either way.
fn attribute_span(
    groups: &[AttributeGroup],
    want: &str,
    ctx: &Ctx<'_>,
    env: &Env<'_>,
) -> Option<Span> {
    groups
        .iter()
        .flat_map(|group| &group.attributes)
        .find(|attr| attribute_is(attr, want, ctx, env))
        .map(|attr| attr.span)
}

/// What one declared property contributed to the codec.
///
/// Three outcomes rather than an [`Option`] because the caller has to tell a
/// property left off the contract *in writing* from one that was refused: an
/// empty contract is an error, and a class whose every property was already
/// refused must not be told so twice.
enum FieldOutcome {
    /// A field, in declaration order, in the wire contract.
    Kept(DerivedField),
    /// ADR 0071 § 3's `#[Json\Field(skip: true)]`.
    Skipped,
    /// Refused by § 2 or § 6, with the diagnostic already reported.
    Refused,
}

/// One property's [`DerivedField`], or why it is not one.
fn codec_field(
    p: &PropertyMember,
    class: &QName,
    params: Option<&[Param]>,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> FieldOutcome {
    let name = strip_sigil(span_text(env.src, p.name)).to_owned();
    let overrides = field_overrides(&p.attributes, ctx, env);
    if overrides.skip {
        return FieldOutcome::Skipped;
    }
    // ADR 0071 § 2: `lateinit` is by definition not constructor-assigned, so
    // it can never be a field — reported before the parameter check, which
    // would otherwise report the same declaration twice.
    if p.modifiers.contains(&Modifier::Lateinit) {
        env.diags.report(
            Diagnostic::error(
                code::E_DERIVE_LATEINIT_FIELD,
                format!("`${name}` is `lateinit`, so it cannot be a `#[Json\\Derive]` field"),
            )
            .with_primary(p.name, "assigned after the constructor, not by it")
            .with_help(
                "ADR 0071 § 2: a decode is an ordinary `new`, and ADR 0038 makes a `lateinit` \
                 property one the constructor does not assign — write \
                 `#[Json\\Field(skip: true)]` on it",
            ),
        );
        return FieldOutcome::Refused;
    }
    let declared = crate::lower::lower_type(&p.ty, ctx, env);
    // ADR 0071 § 6: ADR 0033's refusal, moved from wherever the value reached
    // the encoder to the declaration that put it on the wire contract.
    if is_secret(declared, env) {
        env.diags.report(
            Diagnostic::error(
                code::E_DERIVE_SECRET_FIELD,
                format!("`${name}` is `secret`, so it cannot be a `#[Json\\Derive]` field"),
            )
            .with_primary(p.name, "a `secret` value has no wire form")
            .with_help(
                "ADR 0071 § 6: encoding was already an ADR 0033 sink — write \
                 `#[Json\\Field(skip: true)]` to leave it off the contract in writing",
            ),
        );
        return FieldOutcome::Refused;
    }
    let param = check_constructor_parameter(p, &name, declared, params, ctx, env);
    let nullable = env.interner.is_nullable(declared);
    // The `null` arm is what nullability *is*, so the decode target is the
    // rest of the union — `?int` decodes an `int` or a JSON null, never a
    // third thing.
    let carried = if nullable {
        env.interner.without_null(declared)
    } else {
        declared
    };
    // § 2's codec-reachable test, asked of the null-stripped type and asked
    // after the walk — `?T` is reachable exactly when `T` is, and "another
    // class that itself has a codec" is a question about the whole program.
    env.codec_sites.push(CodecFieldSite {
        class: class.to_string(),
        property: name.clone(),
        span: p.name,
        declared: carried,
    });
    FieldOutcome::Kept(DerivedField {
        key: overrides.name.unwrap_or_else(|| name.clone()),
        property: name,
        ty: codec_ty(carried, env),
        nullable,
        param,
    })
}

/// ADR 0071 § 2's "every non-skipped field must also be a constructor
/// parameter of the same name and the same type".
///
/// Reports and returns the parameter's *position*, which is what a generated
/// decoder fills; the field is recorded either way, so one bad property does
/// not silently drop the rest of the wire contract.
fn check_constructor_parameter(
    p: &PropertyMember,
    name: &str,
    declared: TypeId,
    params: Option<&[Param]>,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> Option<usize> {
    // A class with no written constructor has no parameter list to disagree
    // with, and `nvs_types::ctor_init` has already reported that its properties
    // are not definitely assigned (ADR 0022) — a second diagnostic here would
    // only bury that one.
    let params = params?;
    let Some((index, param)) = params
        .iter()
        .enumerate()
        .find(|(_, param)| strip_sigil(span_text(env.src, param.name)) == name)
    else {
        env.diags.report(
            Diagnostic::error(
                code::E_DERIVE_FIELD_NOT_A_PARAMETER,
                format!("`${name}` is a `#[Json\\Derive]` field with no constructor parameter"),
            )
            .with_primary(p.name, "nothing decodes into this")
            .with_help(
                "ADR 0071 § 2: a decode is an ordinary `new`, so every field needs a \
                 same-named constructor parameter — add one, or write \
                 `#[Json\\Field(skip: true)]`",
            ),
        );
        return None;
    };
    let param_ty = crate::lower::lower_optional_type(param.ty.as_ref(), ctx, env);
    if param_ty == declared {
        return Some(index);
    }
    let want = env.interner.describe(declared);
    let got = env.interner.describe(param_ty);
    env.diags.report(
        Diagnostic::error(
            code::E_DERIVE_FIELD_TYPE_MISMATCH,
            format!("`${name}` is declared `{want}` but its constructor parameter is `{got}`"),
        )
        .with_primary(param.name, format!("this is `{got}`"))
        .with_secondary(p.name, format!("the property is `{want}`"))
        .with_help(
            "ADR 0071 § 2: the decoder decodes into the property's declared type and passes \
             it to the constructor, so the two have to agree",
        ),
    );
    Some(index)
}

/// Whether `ty` carries ADR 0033's `secret` qualifier.
fn is_secret(ty: TypeId, env: &Env<'_>) -> bool {
    matches!(
        env.interner.get(ty),
        Ty::SecretString | Ty::SecretBytes | Ty::SecretTaintedString | Ty::SecretTaintedBytes
    )
}

/// The written `constructor`'s parameter list, or `None` when the class
/// declares none. Found by name — ADR 0030 fixes the spelling.
fn constructor_params<'a>(decl: &'a ClassDecl, env: &Env<'_>) -> Option<&'a [Param]> {
    decl.members.iter().find_map(|member| match &member.kind {
        ClassMemberKind::Method(m) if span_text(env.src, m.name) == CONSTRUCTOR => {
            Some(m.params.as_slice())
        }
        _ => None,
    })
}

/// ADR 0030's one spelling of a constructor.
const CONSTRUCTOR: &str = "constructor";

/// ADR 0071 § 3's two per-field options, as written on one property.
#[derive(Debug, Default)]
struct Overrides {
    /// `name: "..."`, if written.
    name: Option<String>,
    /// `skip: true`.
    skip: bool,
}

/// Reads `#[Json\Field(...)]` off one property, reporting anything that is not
/// ADR 0071 § 3's two options with a literal of the right type.
fn field_overrides(groups: &[AttributeGroup], ctx: &Ctx<'_>, env: &mut Env<'_>) -> Overrides {
    let mut out = Overrides::default();
    for field in attribute_fields(groups, FIELD, ctx, env) {
        match (span_text(env.src, field.name), &field.value.kind) {
            ("name", ExprKind::Str(span)) => {
                out.name = Some(crate::string_lit::cook_string_literal(env.src, *span));
            }
            ("skip", ExprKind::Bool(value)) => out.skip = *value,
            ("name" | "skip", _) => report_field_arg(
                field,
                "`name` takes a `string` literal and `skip` a `bool` literal",
                env,
            ),
            _ => report_field_arg(
                field,
                "`#[Json\\Field]` has exactly two options, `name` and `skip`",
                env,
            ),
        }
    }
    out
}

/// One `E_DERIVE_FIELD_ATTRIBUTE`, at the offending field.
fn report_field_arg(field: &ObjectLiteralField, why: &str, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(code::E_DERIVE_FIELD_ATTRIBUTE, why.to_owned())
            .with_primary(field.span, "not an option this attribute declares")
            .with_help(
                "ADR 0071 § 3: `#[Json\\Field(name?: string, skip?: bool)]` — there is no \
                 whole-class naming policy and no third option",
            ),
    );
}

/// Whether one attribute is the named form spelling `want`. A bare
/// `#[{...}]` names nothing at all (ADR 0046 § 1), so it is never one of
/// ADR 0071's two nominal attributes.
pub(crate) fn attribute_is(attr: &Attribute, want: &str, ctx: &Ctx<'_>, env: &Env<'_>) -> bool {
    attr.name
        .as_ref()
        .is_some_and(|name| resolves_to(span_text(env.src, name.span), want, ctx))
}

/// Every field written on the *first* `want`-named attribute in `groups`.
///
/// The first: a second `#[Json\Field]` on one property is a mistake this pass
/// does not yet report, and reading both would silently merge two overrides.
fn attribute_fields<'a>(
    groups: &'a [AttributeGroup],
    want: &str,
    ctx: &Ctx<'_>,
    env: &Env<'_>,
) -> &'a [ObjectLiteralField] {
    groups
        .iter()
        .flat_map(|group| &group.attributes)
        .find(|attr| attribute_is(attr, want, ctx, env))
        .map(|attr| attr.fields.as_slice())
        .unwrap_or_default()
}

/// ADR 0071 § 1's nominal match: `text`, resolved against the active namespace
/// and imports, is exactly `want`.
fn resolves_to(text: &str, want: &str, ctx: &Ctx<'_>) -> bool {
    nvs_hir::resolve_ref(text, ctx.namespace, ctx.imports) == QName::parse(want)
}
