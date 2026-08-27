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
//! [`mwl_hir::resolve_ref`], and the active namespace and import set are things
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
//! `mwl-ir` to read back. It carries the *property* name rather than a slot
//! index, because the slot order is
//! [`crate::layout::ClassLayout`]'s and that table is built after checking; the
//! two are joined in `mwl_ir::lower::lower_file`, which holds both. Each field
//! also carries the declared type a decoder checks against, erased to
//! [`mwl_stdlib::CodecTy`], and the *constructor position* it fills — ADR 0071
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
//! 2. **§ 2's codec-reachable type test is not applied**, and neither is § 7's
//!    refusal of a class that hand-writes both halves. A field whose declared
//!    type has no decoder is [`CodecTy::Opaque`] and is refused by
//!    `mwl_stdlib::json` when a `decodeAs<T>` actually runs, rather than at
//!    the declaration that wrote it.
//! 3. **`#[Db\Derive]`/`#[Db\Field]` resolve to nothing.** `Core\Db` is M8's,
//!    so the two names are deliberately not in [`ATTRIBUTES`] yet: a closed
//!    list that names something with no pass behind it is worse than a short
//!    one.

use mwl_diagnostics::{Diagnostic, code};
use mwl_hir::QName;
use mwl_syntax::ast::{
    Arg, AttributeGroup, CallArgs, ClassDecl, ClassMemberKind, ExprKind, Modifier, Param,
    PropertyMember,
};

use mwl_stdlib::CodecTy;

use crate::ty::{Ty, TypeId};
use crate::{Ctx, Env, span_text, strip_sigil};

/// ADR 0071 § 1's closed, `Core`-owned list of compiler-recognized attribute
/// names, each already fully qualified.
///
/// A `Name` written at an attribute site is resolved with
/// [`mwl_hir::resolve_ref`] and compared against these — so `#[Core\Json\Derive]`
/// and a `use Core\Json;`d `#[Json\Derive]` are the same attribute, and no
/// spelling of a userland name is any of them. Extended, never widened: ADR
/// 0077's `Core\Route` is the next entry, and each one owes its own argued ADR
/// section.
pub const ATTRIBUTES: &[&str] = [DERIVE, FIELD].as_slice();

/// `#[Json\Derive]` — ADR 0071 § 1's opt-in, on a class.
pub const DERIVE: &str = r"Core\Json\Derive";

/// `#[Json\Field(name?: string, skip?: bool)]` — ADR 0071 § 3's per-field
/// override, on a property.
pub const FIELD: &str = r"Core\Json\Field";

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
/// The slot-resolved half is [`mwl_stdlib::CodecField`]; the two are joined in
/// `mwl_ir::lower::lower_file`, which is the one place both this table and
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
/// `mwl_stdlib::json`'s own gap owns the decoders they still need, and § 2's
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
    if !carries(&decl.attributes, DERIVE, ctx, env) {
        return;
    }
    let params = constructor_params(decl, env);
    let mut codec = DerivedCodec {
        ctor_arity: params.map_or(0, <[Param]>::len),
        ..DerivedCodec::default()
    };
    for member in &decl.members {
        let ClassMemberKind::Property(p) = &member.kind else {
            continue;
        };
        if p.modifiers.contains(&Modifier::Static) {
            continue; // ADR 0008: class storage, not instance storage
        }
        let Some(field) = codec_field(p, params, ctx, env) else {
            continue;
        };
        codec.fields.push(field);
    }
    env.exprs.record_codec(class.to_string(), codec);
}

/// One property's [`DerivedField`], or `None` when it is skipped or refused.
fn codec_field(
    p: &PropertyMember,
    params: Option<&[Param]>,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> Option<DerivedField> {
    let name = strip_sigil(span_text(env.src, p.name)).to_owned();
    let overrides = field_overrides(&p.attributes, ctx, env);
    if overrides.skip {
        return None;
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
        return None;
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
        return None;
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
    Some(DerivedField {
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
    // with, and `mwl_types::ctor_init` has already reported that its properties
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
    for arg in attribute_args(groups, FIELD, ctx, env) {
        let Some(name_span) = arg.name else {
            report_field_arg(arg, "a `#[Json\\Field]` argument must be named", env);
            continue;
        };
        match (span_text(env.src, name_span), &arg.value.kind) {
            ("name", ExprKind::Str(span)) => {
                out.name = Some(crate::string_lit::cook_string_literal(env.src, *span));
            }
            ("skip", ExprKind::Bool(value)) => out.skip = *value,
            ("name" | "skip", _) => report_field_arg(
                arg,
                "`name` takes a `string` literal and `skip` a `bool` literal",
                env,
            ),
            _ => report_field_arg(
                arg,
                "`#[Json\\Field]` has exactly two options, `name` and `skip`",
                env,
            ),
        }
    }
    out
}

/// One `E_DERIVE_FIELD_ATTRIBUTE`, at the offending argument.
fn report_field_arg(arg: &Arg, why: &str, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(code::E_DERIVE_FIELD_ATTRIBUTE, why.to_owned())
            .with_primary(arg.span, "not an option this attribute declares")
            .with_help(
                "ADR 0071 § 3: `#[Json\\Field(name?: string, skip?: bool)]` — there is no \
                 whole-class naming policy and no third option",
            ),
    );
}

/// Whether any of `groups` names the attribute `want`, resolved.
fn carries(groups: &[AttributeGroup], want: &str, ctx: &Ctx<'_>, env: &Env<'_>) -> bool {
    groups
        .iter()
        .flat_map(|group| &group.attributes)
        .any(|attr| resolves_to(span_text(env.src, attr.name.span), want, ctx))
}

/// Every argument written on the *first* `want`-named attribute in `groups`.
///
/// The first: a second `#[Json\Field]` on one property is a mistake this pass
/// does not yet report, and reading both would silently merge two overrides.
fn attribute_args<'a>(
    groups: &'a [AttributeGroup],
    want: &str,
    ctx: &Ctx<'_>,
    env: &Env<'_>,
) -> &'a [Arg] {
    groups
        .iter()
        .flat_map(|group| &group.attributes)
        .find(|attr| resolves_to(span_text(env.src, attr.name.span), want, ctx))
        .and_then(|attr| match &attr.args {
            Some(CallArgs::List(args)) => Some(args.as_slice()),
            _ => None,
        })
        .unwrap_or_default()
}

/// ADR 0071 § 1's nominal match: `text`, resolved against the active namespace
/// and imports, is exactly `want`.
fn resolves_to(text: &str, want: &str, ctx: &Ctx<'_>) -> bool {
    mwl_hir::resolve_ref(text, ctx.namespace, ctx.imports) == QName::parse(want)
}
