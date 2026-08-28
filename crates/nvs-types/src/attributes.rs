//! ADR 0046 § 2's rule about what may sit inside a `#[...]` payload: every
//! field value is a compile-time constant, and nothing else.
//!
//! The rule is one sentence and the reason it is *this* sentence rather than
//! a taste is in that section: the whole literal is resolved once, at compile
//! time, into the compiled unit's constant pool — the storage class an enum
//! case's backing integer and a class constant already use — so there is no
//! moment at which a variable could be read, a call made or a `new`
//! constructed. PHP's attributes do have that moment (a real object is built
//! the first time `ReflectionAttribute::newInstance()` is called) and this
//! ADR deliberately does not, so the refusal here is what keeps the two
//! halves of that decision from disagreeing.
//!
//! Two consequences of the same rule are *not* checked here and each has its
//! own home. Whether the named form's `Name` resolves to a shape-typed `type`
//! alias, and whether the literal satisfies it, is ADR 0046 § 1's check
//! against a declared shape — the ordinary shape-typed position's check, not
//! a second one. And ADR 0033's fifth sink — a `secret` class constant
//! reaching a payload — is [`crate::expr::quals`]', where every other sink
//! already lives.

use nvs_diagnostics::{Diagnostic, code};
use nvs_syntax::ast::{
    ArrayItem, Attribute, AttributeGroup, ClassMember, ClassMemberKind, EnumCase, Expr, ExprKind,
    Param, UnaryOp,
};

use crate::Env;

/// Every attribute attached anywhere under one class/interface/enum body:
/// the declaration's own groups, each member's, each method parameter's and
/// each property hook's. One entry point per declaration kind would be four
/// copies of the same walk, so the caller hands over the two lists it has.
pub(crate) fn check_declaration(
    groups: &[AttributeGroup],
    members: &[ClassMember],
    cases: &[EnumCase],
    env: &mut Env<'_>,
) {
    check_groups(groups, env);
    for member in members {
        match &member.kind {
            ClassMemberKind::Property(p) => {
                check_groups(&p.attributes, env);
                for hook in p.hooks.iter().flatten() {
                    check_groups(&hook.attributes, env);
                }
            }
            ClassMemberKind::Const(c) => check_groups(&c.attributes, env),
            ClassMemberKind::Method(m) => {
                check_groups(&m.attributes, env);
                check_params(&m.params, env);
            }
            _ => {}
        }
    }
    for case in cases {
        check_groups(&case.attributes, env);
    }
}

/// Every attribute on one parameter list — ADR 0046 § 1's fourth attach site.
pub(crate) fn check_params(params: &[Param], env: &mut Env<'_>) {
    for param in params {
        check_groups(&param.attributes, env);
    }
}

fn check_groups(groups: &[AttributeGroup], env: &mut Env<'_>) {
    for group in groups {
        for attr in &group.attributes {
            check_attribute(attr, env);
        }
    }
}

fn check_attribute(attr: &Attribute, env: &mut Env<'_>) {
    for field in &attr.fields {
        check_value(&field.value, env);
    }
}

/// One payload value, reported where it is written rather than at the
/// attribute — a payload with two bad fields is two diagnostics, in source
/// order, because each is its own mistake.
fn check_value(expr: &Expr, env: &mut Env<'_>) {
    if is_constant(expr) {
        // A container's own elements are values in their own right, so a
        // constant-shaped container is descended into rather than trusted.
        match &expr.kind {
            ExprKind::ArrayLiteral(items) => {
                for ArrayItem { key, value, .. } in items {
                    if let Some(key) = key {
                        check_value(key, env);
                    }
                    check_value(value, env);
                }
            }
            ExprKind::ObjectLiteral(fields) => {
                for field in fields {
                    check_value(&field.value, env);
                }
            }
            ExprKind::Paren(inner) | ExprKind::Unary { expr: inner, .. } => {
                check_value(inner, env);
            }
            _ => {}
        }
        return;
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
