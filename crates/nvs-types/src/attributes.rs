//! ADR 0046 §§ 1-2's two rules about a `#[...]` payload: every field value is
//! a compile-time constant, and the named form's `Name` is a shape-typed
//! `type` alias the literal is then checked against.
//!
//! § 2's rule is one sentence and the reason it is *this* sentence rather than
//! a taste is in that section: the whole literal is resolved once, at compile
//! time, into the compiled unit's constant pool — the storage class an enum
//! case's backing integer and a class constant already use — so there is no
//! moment at which a variable could be read, a call made or a `new`
//! constructed. PHP's attributes do have that moment (a real object is built
//! the first time `ReflectionAttribute::newInstance()` is called) and this
//! ADR deliberately does not, so the refusal here is what keeps the two
//! halves of that decision from disagreeing.
//!
//! § 1's rule is what makes the named form sugar rather than a second
//! namespace: `Name` is never a class, never a new kind of attribute and
//! never anything to instantiate — it is a pre-existing `type` alias whose
//! right-hand side is a shape, and the payload is then checked against it by
//! the very rule an ordinary shape-typed binding is checked by
//! ([`crate::expr::is_assignable`], ADR 0036 § 3's width subtyping). So an
//! attribute's name resolves in the ordinary namespace/`use` scope, and an
//! unresolvable one is the ordinary `E0303` rather than a refusal of its own.
//!
//! One consequence of § 2 is checked from here but owned elsewhere: ADR 0033
//! § 4's fifth sink — a `secret` class constant reaching a payload — is
//! `crate::expr::quals`', where every other sink already lives.
//! [`check_value`] calls it at each value it reaches, that walk being the one
//! place every payload value passes.

use nvs_diagnostics::{Diagnostic, code};
use nvs_syntax::ast::{
    ArrayItem, Attribute, AttributeGroup, ClassMember, ClassMemberKind, EnumCase, Expr, ExprKind,
    Name, Param, UnaryOp,
};
use rustc_hash::FxHashSet;

use crate::expr::{check_object_literal, is_assignable, report_mismatch};
use crate::locals::LocalScope;
use crate::ty::{Ty, TypeId};
use crate::{Ctx, Env, span_text};

/// Every attribute attached anywhere under one class/interface/enum body:
/// the declaration's own groups, each member's, each method parameter's and
/// each property hook's. One entry point per declaration kind would be four
/// copies of the same walk, so the caller hands over the two lists it has.
pub(crate) fn check_declaration(
    groups: &[AttributeGroup],
    members: &[ClassMember],
    cases: &[EnumCase],
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    check_groups(groups, ctx, env);
    for member in members {
        match &member.kind {
            ClassMemberKind::Property(p) => {
                check_groups(&p.attributes, ctx, env);
                for hook in p.hooks.iter().flatten() {
                    check_groups(&hook.attributes, ctx, env);
                }
            }
            ClassMemberKind::Const(c) => check_groups(&c.attributes, ctx, env),
            ClassMemberKind::Method(m) => {
                check_groups(&m.attributes, ctx, env);
                check_params(&m.params, ctx, env);
            }
            _ => {}
        }
    }
    for case in cases {
        check_groups(&case.attributes, ctx, env);
    }
}

/// Every attribute on one parameter list — ADR 0046 § 1's fourth attach site.
pub(crate) fn check_params(params: &[Param], ctx: &Ctx<'_>, env: &mut Env<'_>) {
    for param in params {
        check_groups(&param.attributes, ctx, env);
    }
}

fn check_groups(groups: &[AttributeGroup], ctx: &Ctx<'_>, env: &mut Env<'_>) {
    for group in groups {
        for attr in &group.attributes {
            check_attribute(attr, ctx, env);
        }
    }
}

fn check_attribute(attr: &Attribute, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    let mut constant = true;
    for field in &attr.fields {
        constant &= check_value(&field.value, ctx, env);
    }
    let Some(name) = &attr.name else {
        // ADR 0046 § 1's bare form names no shape to check against, so the
        // literal is checked as a well-formed literal and nothing more —
        // which the constant walk above has just done.
        return;
    };
    let Some(shape) = resolve_shape_alias(name, ctx, env) else {
        return;
    };
    // A payload with a computed value has already been reported once, and
    // inferring that value would report it a second time — as an undeclared
    // variable, or as a mismatch against the field the shape declares. The
    // author is told about the value they wrote before they are told what it
    // failed to satisfy.
    if !constant {
        return;
    }
    // Deliberately the ordinary shape-typed position's check, run over an
    // empty scope: § 2 has just proved there is no variable in this payload,
    // so no binding can be read and none can be captured.
    let mut live = FxHashSet::default();
    let scope = LocalScope::new();
    let actual = check_object_literal(&attr.fields, &mut live, &scope, ctx, env);
    if !is_assignable(actual, shape, env.interner, env.graph, env.signatures) {
        report_mismatch(attr.payload, shape, actual, env);
    }
}

/// ADR 0046 § 1's named form: `Name` resolves, in the namespace/`use` scope
/// the attribute is written in, to a `type` alias whose expansion is a shape
/// type.
///
/// Three answers, and they are deliberately three. A name nothing declared is
/// the **ordinary** `E0303` any unresolvable name is — an attribute name is
/// not a new namespace, so it does not get a "no such attribute" of its own.
/// A name that resolves to something that is not a shape-typed alias is
/// `E0726`: a class is the spelling this exists to refuse, § 1 giving an
/// attribute no kind to instantiate, and a `type Id = int;` is the same
/// mistake one step along. Only a shape is handed back, and then the payload
/// is checked against it.
fn resolve_shape_alias(name: &Name, ctx: &Ctx<'_>, env: &mut Env<'_>) -> Option<TypeId> {
    let text = span_text(env.src, name.span).to_owned();
    let qname = nvs_hir::resolve_ref(&text, ctx.namespace, ctx.imports);
    // ADR 0071 § 1's compiler-recognized attributes are the one exemption,
    // and it is a closed, `Core`-owned roster rather than an escape hatch:
    // `#[Json\Derive]` names no shape because it is matched *nominally* by
    // the compiler, and what its payload may hold is [`crate::derive`]'s own
    // option check rather than a shape. ADR 0046 § 1's rule is about the
    // userland names, which are the only ones that could ever be aliases.
    if crate::derive::ATTRIBUTES
        .iter()
        .any(|want| qname == nvs_hir::QName::parse(want))
    {
        return None;
    }
    let Some(alias) = env.aliases.get(&qname).cloned() else {
        // The same roster [`crate::lower`]'s own name atom uses for "this
        // name denotes something," so one name is undeclared in one place.
        let declared = env.symbols.get(&qname).is_some()
            || qname.is_core()
            || qname.is_reserved_global_class()
            || qname.is_reserved_global_interface();
        if declared {
            report_not_a_shape(name, &format!("`{qname}` is not a `type` alias"), env);
        } else {
            env.diags.report(
                Diagnostic::error(
                    code::E_UNDEFINED_CLASS,
                    format!("`{qname}` is not declared"),
                )
                .with_primary(name.span, "no matching declaration"),
            );
        }
        return None;
    };
    let id = crate::lower::lower_type(&alias, ctx, env);
    if matches!(env.interner.get(id), Ty::Shape(_)) {
        return Some(id);
    }
    let described = env.interner.describe(id);
    report_not_a_shape(
        name,
        &format!("`{qname}` is a `type` alias for `{described}`, which is not a shape"),
        env,
    );
    None
}

/// The `E0726` half of [`resolve_shape_alias`], which owns why.
fn report_not_a_shape(name: &Name, what: &str, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_ATTRIBUTE_NAME_NOT_A_SHAPE,
            format!("an attribute's name must be a shape-typed `type` alias, and {what}"),
        )
        .with_primary(
            name.span,
            "this names no shape to check the payload against",
        )
        .with_help(
            "ADR 0046 § 1: a named attribute instantiates nothing — the name is only ever an \
             existing alias like `type Route = {path: string, method: string};`, and the \
             payload is checked against it. An attribute with no shape to satisfy is written \
             bare, `#[{field: value}]`",
        ),
    );
}

/// One payload value, reported where it is written rather than at the
/// attribute — a payload with two bad fields is two diagnostics, in source
/// order, because each is its own mistake. Answers whether this value (and
/// every value nested inside it) is a constant, which is what tells
/// [`check_attribute`] the literal is worth checking against a shape.
fn check_value(expr: &Expr, ctx: &Ctx<'_>, env: &mut Env<'_>) -> bool {
    if is_constant(expr) {
        // ADR 0033 § 4's fifth sink, asked of every value this walk reaches
        // and not only of a payload's top level: a `secret` constant nested
        // inside an array or an object literal is folded into the same
        // constant pool. It answers for a `Class::CONST` and for nothing
        // else — see [`crate::expr::reject_secret_attribute_constant`] for
        // why one expression kind is the whole of it.
        crate::expr::reject_secret_attribute_constant(expr, ctx, env);
        // A container's own elements are values in their own right, so a
        // constant-shaped container is descended into rather than trusted.
        return match &expr.kind {
            ExprKind::ArrayLiteral(items) => {
                let mut ok = true;
                for ArrayItem { key, value, .. } in items {
                    if let Some(key) = key {
                        ok &= check_value(key, ctx, env);
                    }
                    ok &= check_value(value, ctx, env);
                }
                ok
            }
            ExprKind::ObjectLiteral(fields) => {
                let mut ok = true;
                for field in fields {
                    ok &= check_value(&field.value, ctx, env);
                }
                ok
            }
            ExprKind::Paren(inner) | ExprKind::Unary { expr: inner, .. } => {
                check_value(inner, ctx, env)
            }
            _ => true,
        };
    }
    env.diags.report(
        Diagnostic::error(
            code::E_ATTRIBUTE_VALUE_NOT_CONSTANT,
            "an attribute's field value is not a compile-time constant",
        )
        .with_primary(expr.span, "this is computed when the program runs")
        .with_help(
            "ADR 0046 § 2: an attribute payload is a literal, a class constant or an enum \
             case — it lives in the constant pool, so there is no point at which a variable, \
             a call or a `new` could be evaluated",
        ),
    );
    false
}

/// The closed list of shapes ADR 0046 § 2 admits. Closed on purpose: an
/// expression kind this does not name is refused, so a shape the grammar
/// grows is refused until someone decides it belongs in a constant pool,
/// rather than accepted because nothing said otherwise.
fn is_constant(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Null
        | ExprKind::Bool(_)
        | ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::Duration(_)
        | ExprKind::Str(_)
        // `Class::CONST` — a class constant, and ADR 0010's enum case with it,
        // which is the one spelling § 2 names beside a literal.
        | ExprKind::ClassConstAccess { .. }
        | ExprKind::ArrayLiteral(_)
        | ExprKind::ObjectLiteral(_) => true,
        // An interpolated string reads a variable by definition, whatever it
        // interpolates, so it is not the `Str` row one syntax along.
        ExprKind::Paren(inner) => is_constant(inner),
        // The sign on a numeric literal, and `~` on an integer one: an
        // operator over a constant operand is folded with it.
        ExprKind::Unary { op, expr } => {
            matches!(op, UnaryOp::Neg | UnaryOp::Plus | UnaryOp::BitNot) && is_constant(expr)
        }
        // Error recovery already reported something; a second diagnostic on
        // the same span names one mistake twice.
        ExprKind::Error => true,
        _ => false,
    }
}
