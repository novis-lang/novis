//! `rule:attributes/a-deprecation-names-its-replacement-as-code`'s
//! `#[Core\Deprecated]`: the fields its payload takes, and the declarations it
//! may be written above.
//!
//! The payload's roster is asked by [`crate::attributes`]' recognized-name
//! check, as every roster is. What this module adds is the question that check
//! cannot ask, because it sees one attribute and not where it sits: `construct`
//! is a template for a `new`, so it is a field of a class's deprecation and of
//! nothing else, and a property hook is not a declaration a use ever names, so
//! a deprecation there would have no use to warn at.

use nvs_diagnostics::{Diagnostic, code};
use nvs_syntax::ast::{AttributeGroup, ClassMember, ClassMemberKind, EnumCase, Param};

use crate::testing::OptionTy;
use crate::{Ctx, Env, span_text};

/// The payload's fields, in the order the rule writes them. Every one is a
/// string and every one is optional.
pub(crate) const OPTIONS: &[(&str, OptionTy)] = &[
    ("since", OptionTy::Str),
    ("note", OptionTy::Str),
    ("replace", OptionTy::Str),
    (CONSTRUCT, OptionTy::Str),
];

/// The field a class's deprecation alone may write.
const CONSTRUCT: &str = "construct";

/// What a declaration is, as far as `#[Core\Deprecated]` is concerned: a class
/// takes `construct`, and an interface or an enum does not.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Decl {
    Class,
    Interface,
    Enum,
}

impl Decl {
    fn site(self) -> &'static str {
        match self {
            Self::Class => "a class",
            Self::Interface => "an interface",
            Self::Enum => "an enum",
        }
    }
}

/// Every `#[Core\Deprecated]` under one class, interface or enum body, held to
/// the nine declarations it attaches to. Asked from
/// [`crate::attributes::check_declaration`]'s walk, which visits every attach
/// site of one declaration, beside [`crate::paths::check_marker_sites`].
pub(crate) fn check_sites(
    decl: Decl,
    groups: &[AttributeGroup],
    members: &[ClassMember],
    cases: &[EnumCase],
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    if decl != Decl::Class {
        report_construct(groups, decl.site(), ctx, env);
    }
    for member in members {
        match &member.kind {
            ClassMemberKind::Property(p) => {
                report_construct(&p.attributes, "a property", ctx, env);
                for hook in p.hooks.iter().flatten() {
                    report_on_hook(&hook.attributes, ctx, env);
                    if let Some(param) = &hook.param {
                        report_on_hook(&param.attributes, ctx, env);
                    }
                }
            }
            ClassMemberKind::Const(c) => report_construct(&c.attributes, "a constant", ctx, env),
            ClassMemberKind::Method(m) => {
                report_construct(&m.attributes, "a method", ctx, env);
                check_params(&m.params, ctx, env);
            }
            _ => {}
        }
    }
    for case in cases {
        report_construct(&case.attributes, "an enum case", ctx, env);
    }
}

fn check_params(params: &[Param], ctx: &Ctx<'_>, env: &mut Env<'_>) {
    for param in params {
        report_construct(&param.attributes, "a parameter", ctx, env);
    }
}

/// `construct` in each `#[Core\Deprecated]` among `groups`, which sit on
/// `site` — a declaration that is not a class, so no `new` ever names it.
fn report_construct(groups: &[AttributeGroup], site: &str, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    for attr in groups.iter().flat_map(|group| &group.attributes) {
        if !crate::derive::attribute_is(attr, crate::derive::DEPRECATED, ctx, env) {
            continue;
        }
        for field in &attr.fields {
            if span_text(env.src, field.name) != CONSTRUCT {
                continue;
            }
            env.diags.report(
                Diagnostic::error(
                    code::E_UNKNOWN_OPTION,
                    format!("`construct` is not an option of `#[Core\\Deprecated]` on {site}"),
                )
                .with_primary(field.span, "only a class takes this option")
                .with_help(
                    "`construct` is the code that replaces `new` for a deprecated class. \
                     Use `replace` here instead",
                ),
            );
        }
    }
}

/// Each `#[Core\Deprecated]` among `groups`, which sit on a property hook or
/// on its parameter.
fn report_on_hook(groups: &[AttributeGroup], ctx: &Ctx<'_>, env: &mut Env<'_>) {
    for attr in groups.iter().flat_map(|group| &group.attributes) {
        if !crate::derive::attribute_is(attr, crate::derive::DEPRECATED, ctx, env) {
            continue;
        }
        env.diags.report(
            Diagnostic::error(
                code::E_DEPRECATED_ON_A_PROPERTY_HOOK,
                "`#[Core\\Deprecated]` is on a property hook",
            )
            .with_primary(attr.span, "a use names the property, not its hook")
            .with_help("write `#[Core\\Deprecated]` above the property instead"),
        );
    }
}
