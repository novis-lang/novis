//! `rule:routing/a-route-target-parameter-is-checked-like-a-link`'s two
//! marks: `#[Core\Router\Target]` on a `callable` parameter and
//! `#[Core\Router\Params]` on an `array` parameter of a user method.
//!
//! The sibling of [`crate::paths`], which reads `#[Core\Path]` the same way.
//! [`declared_slots`] reads the marks off a method's parameters into
//! [`RouteSlots`], which [`crate::signatures::MethodSig::route_slots`] carries
//! to every call of the method. [`check_marker_sites`] holds each mark to the
//! parameter it means something on.
//!
//! # Where a mark may be written
//!
//! Both are declared `on parameter` in [`crate::derive::ATTRIBUTE_DOCS`], so
//! [`crate::attributes`] already reports a mark on a method, a class or a
//! property (`E0874`). What is left here is the one parameter's type and the
//! method's whole list:
//!
//! - **`E0899`**: a `Target` on a parameter whose type is not `callable`, a
//!   `callable(...)` type, or either one nullable; a `Params` on one whose type
//!   is not `array` or `array<...>`, nullable or not; either on a variadic
//!   parameter, which many arguments fill.
//! - **`E1000`**: a `Params` in a method with no `Target`, and a second
//!   `Target` or a second `Params` in one method. A `Params` array is checked
//!   against the route its `Target` names, so it needs exactly one.
//!
//! A mark the site check refuses is not counted here, so one mistake is one
//! error.
//!
//! **Cost:** one scan of a method's parameters while compiling, and one
//! `Option<RouteSlots>` per signature. Nothing at run time.

use nvs_diagnostics::{Diagnostic, code};
use nvs_syntax::ast::{
    Attribute, ClassMember, ClassMemberKind, MethodMember, Param, Type, TypeAtom, TypeKind,
};

use crate::{Ctx, Env, span_text, strip_sigil};

/// Which parameters of a method carry the two marks, by index into its
/// parameter list. `params` is `None` for a method whose `Target` stands alone,
/// as a route with no parameters needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RouteSlots {
    /// The `#[Core\Router\Target]` parameter.
    pub target: usize,
    /// The `#[Core\Router\Params]` parameter, if the method has one.
    pub params: Option<usize>,
}

/// The two marks on `params`, or `None` when no parameter carries a
/// `Target` — every signature that marks nothing. A mark
/// [`check_marker_sites`] refuses is never read here: the program does not
/// compile, so what the slots say about it is never used.
pub(crate) fn declared_slots(params: &[Param], ctx: &Ctx<'_>, env: &Env<'_>) -> Option<RouteSlots> {
    let marked = |name: &str| {
        params.iter().position(|param| {
            !param.is_promoted()
                && crate::testing::attribute_named(&param.attributes, name, ctx, env).is_some()
        })
    };
    let target = marked(crate::derive::ROUTE_TARGET)?;
    Some(RouteSlots {
        target,
        params: marked(crate::derive::ROUTE_PARAMS),
    })
}

/// Every method among `members`, each held to the module's two rules. Asked
/// from [`crate::attributes::check_declaration`]'s walk beside
/// [`crate::paths::check_marker_sites`], for its reason.
pub(crate) fn check_marker_sites(members: &[ClassMember], ctx: &Ctx<'_>, env: &mut Env<'_>) {
    for member in members {
        if let ClassMemberKind::Method(m) = &member.kind {
            check_method(m, ctx, env);
        }
    }
}

/// One method: each mark's parameter, then the pairing over the whole list.
fn check_method(m: &MethodMember, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    let mut targets: Vec<&Attribute> = Vec::new();
    let mut arrays: Vec<&Attribute> = Vec::new();
    for param in m.params.iter().filter(|param| !param.is_promoted()) {
        for attr in param.attributes.iter().flat_map(|group| &group.attributes) {
            if crate::derive::attribute_is(attr, crate::derive::ROUTE_TARGET, ctx, env) {
                check_param(param, attr, Mark::Target, env);
                targets.push(attr);
            } else if crate::derive::attribute_is(attr, crate::derive::ROUTE_PARAMS, ctx, env) {
                check_param(param, attr, Mark::Params, env);
                arrays.push(attr);
            }
        }
    }
    for extra in targets.iter().skip(1) {
        report_pairing(
            extra,
            "a method has a second `#[Core\\Router\\Target]` parameter",
            "a method takes one route",
            env,
        );
    }
    for extra in arrays.iter().skip(1) {
        report_pairing(
            extra,
            "a method has a second `#[Core\\Router\\Params]` parameter",
            "a method takes one array of route values",
            env,
        );
    }
    if let (true, Some(first)) = (targets.is_empty(), arrays.first()) {
        report_pairing(
            first,
            "`#[Core\\Router\\Params]` is in a method with no `#[Core\\Router\\Target]` \
             parameter",
            "no route to check these values against",
            env,
        );
    }
}

/// Which of the two marks a parameter carries.
#[derive(Clone, Copy)]
enum Mark {
    Target,
    Params,
}

/// `E0899` when `param`, marked with `attr`, is variadic or has a type the
/// mark cannot be read from.
fn check_param(param: &Param, attr: &Attribute, mark: Mark, env: &mut Env<'_>) {
    let (written, wanted, fits): (&str, &str, fn(&TypeAtom) -> bool) = match mark {
        Mark::Target => (
            "#[Core\\Router\\Target]",
            "a route is a `callable`",
            |atom| matches!(atom, TypeAtom::Callable | TypeAtom::CallableSig { .. }),
        ),
        Mark::Params => (
            "#[Core\\Router\\Params]",
            "route values are an `array`",
            |atom| matches!(atom, TypeAtom::Array(..)),
        ),
    };
    let name = strip_sigil(span_text(env.src, param.name)).to_owned();
    if param.variadic {
        env.diags.report(
            Diagnostic::error(
                code::E_ROUTE_ATTRIBUTE_ON_A_WRONG_PARAMETER,
                format!("`{written}` is on the variadic parameter `...${name}`"),
            )
            .with_primary(attr.span, "many arguments fill this parameter")
            .with_help(format!(
                "`{written}` goes on a parameter that takes one value. Remove the `...`, or \
                 delete the attribute"
            )),
        );
        return;
    }
    if param.ty.as_ref().is_some_and(|ty| is_nullable(ty, fits)) {
        return;
    }
    let ty = param
        .ty
        .as_ref()
        .map_or("no type", |ty| span_text(env.src, ty.span))
        .to_owned();
    let (kind, detailed) = match mark {
        Mark::Target => ("`callable` or `?callable`", "A `callable(...)`"),
        Mark::Params => ("`array` or `?array`", "An `array<...>`"),
    };
    env.diags.report(
        Diagnostic::error(
            code::E_ROUTE_ATTRIBUTE_ON_A_WRONG_PARAMETER,
            format!("`{written} ${name}` is on a parameter of type `{ty}`"),
        )
        .with_primary(attr.span, wanted)
        .with_help(format!(
            "Declare the parameter {kind}, or delete the attribute. {detailed} type is also \
             allowed"
        )),
    );
}

/// `E1000` at `attr`.
fn report_pairing(attr: &Attribute, message: &str, label: &str, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(code::E_ROUTE_ATTRIBUTES_DO_NOT_PAIR, message.to_owned())
            .with_primary(attr.span, label.to_owned())
            .with_help(
                "A method can have one `#[Core\\Router\\Target]` parameter and one \
                 `#[Core\\Router\\Params]` parameter. Delete the extra attribute, or add the \
                 missing `#[Core\\Router\\Target]`",
            ),
    );
}

/// Whether `ty` is an atom `fits` accepts, alone or with `null`: `callable`,
/// `?callable`, `callable|null`, `(callable)`.
fn is_nullable(ty: &Type, fits: fn(&TypeAtom) -> bool) -> bool {
    match &ty.kind {
        TypeKind::Atom(atom) => fits(atom),
        TypeKind::Nullable(inner) | TypeKind::Paren(inner) => is_nullable(inner, fits),
        TypeKind::Union(members) => {
            members.iter().any(|member| is_nullable(member, fits))
                && members.iter().all(|member| {
                    is_nullable(member, fits)
                        || matches!(member.kind, TypeKind::Atom(TypeAtom::Null))
                })
        }
        _ => false,
    }
}
