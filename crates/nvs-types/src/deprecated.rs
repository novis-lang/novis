//! `rule:attributes/a-deprecation-names-its-replacement-as-code`'s
//! `#[Core\Deprecated]`: the fields its payload takes, the declarations it may
//! be written above, which declarations carry it, and the template its
//! `replace` and `construct` write.
//!
//! The payload's roster is asked by [`crate::attributes`]' recognized-name
//! check, as every roster is. What this module adds is the question that check
//! cannot ask, because it sees one attribute and not where it sits: `construct`
//! is a template for a `new`, so it is a field of a class's deprecation and of
//! nothing else, and a property hook is not a declaration a use ever names, so
//! a deprecation there would have no use to warn at. [`template`] checks the
//! template itself, against the declaration it is written on.
//!
//! [`Deprecations`] is the program's set of deprecated declarations, built
//! before any body is checked: a use may be written above the declaration it
//! names, and a template may name a declaration in a later file.

mod template;

use nvs_diagnostics::{Diagnostic, code};
use nvs_hir::QName;
use nvs_syntax::ast::{
    Attribute, AttributeGroup, ClassMember, ClassMemberKind, EnumCase, NamespaceDecl, Param, Stmt,
    StmtKind,
};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::testing::OptionTy;
use crate::{Ctx, Env, ProgramFile, span_text, strip_sigil};

/// The payload's fields, in the order the rule writes them. Every one is a
/// string and every one is optional.
pub(crate) const OPTIONS: &[(&str, OptionTy)] = &[
    ("since", OptionTy::Str),
    ("note", OptionTy::Str),
    (REPLACE, OptionTy::Str),
    (CONSTRUCT, OptionTy::Str),
];

/// The field every deprecation may write: the code a use is rewritten to.
const REPLACE: &str = "replace";

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

/// One declaration a deprecation can sit on, named under the class,
/// interface or enum that declares it. [`Member::Type`] is that declaration
/// itself.
#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) enum Member {
    Type,
    Method(String),
    Property(String),
    Const(String),
    Case(String),
    /// A method's parameter: the method's name, then the parameter's, with no
    /// `$` sigil.
    Param(String, String),
}

/// Every declaration in the program that carries `#[Core\Deprecated]`.
#[derive(Default)]
pub(crate) struct Deprecations {
    sites: FxHashSet<(QName, Member)>,
}

impl Deprecations {
    /// Whether `member` of `owner` — the declaring class, never a subclass a
    /// use reached it through — is deprecated.
    pub(crate) fn contains(&self, owner: &QName, member: Member) -> bool {
        self.sites.contains(&(owner.clone(), member))
    }
}

/// Collects [`Deprecations`] over every file. The namespace and `use` walk is
/// `crate::retrieval`'s, made for the same reason: what an attribute's name
/// means is a property of the text it was written in.
pub(crate) fn build_table(files: &[ProgramFile<'_>]) -> Deprecations {
    let mut table = Deprecations::default();
    for file in files {
        collect(file.stmts, file.src, &[], &FxHashMap::default(), &mut table);
    }
    table
}

fn collect(
    stmts: &[Stmt],
    src: &nvs_diagnostics::SourceFile,
    namespace: &[String],
    imports: &FxHashMap<String, QName>,
    table: &mut Deprecations,
) {
    let mut current_ns = namespace.to_vec();
    let mut current_imports = imports.clone();
    for stmt in stmts {
        let (name, attributes, members, cases) = match &stmt.kind {
            StmtKind::NamespaceDecl(NamespaceDecl { name, body, .. }) => {
                let new_ns = name.as_ref().map_or_else(Vec::new, |n| {
                    QName::parse(span_text(src, n.span)).segments().to_vec()
                });
                match body {
                    Some(block) => {
                        collect(&block.stmts, src, &new_ns, &FxHashMap::default(), table);
                    }
                    None => {
                        current_ns = new_ns;
                        current_imports.clear();
                    }
                }
                continue;
            }
            StmtKind::UseDecl(use_decl) => {
                let target = QName::parse(span_text(src, use_decl.path.span));
                current_imports.insert(target.short_name().to_owned(), target);
                continue;
            }
            StmtKind::ClassDecl(d) => (&d.name, &d.attributes, &d.members, &[][..]),
            StmtKind::InterfaceDecl(d) => (&d.name, &d.attributes, &d.members, &[][..]),
            StmtKind::EnumDecl(d) => (&d.name, &d.attributes, &d.members, &d.cases[..]),
            _ => continue,
        };
        let owner = QName::join(&current_ns, span_text(src, name.span));
        let deprecated = |groups: &[AttributeGroup]| {
            groups.iter().flat_map(|g| &g.attributes).any(|attr| {
                attr.member.is_none()
                    && attr.name.as_ref().is_some_and(|n| {
                        nvs_hir::resolve_ref(span_text(src, n.span), &current_ns, &current_imports)
                            == QName::parse(crate::derive::DEPRECATED)
                    })
            })
        };
        let mut found = Vec::new();
        if deprecated(attributes) {
            found.push(Member::Type);
        }
        for member in members {
            match &member.kind {
                ClassMemberKind::Property(p) if deprecated(&p.attributes) => {
                    found.push(Member::Property(
                        strip_sigil(span_text(src, p.name)).to_owned(),
                    ));
                }
                ClassMemberKind::Const(c) if deprecated(&c.attributes) => {
                    found.push(Member::Const(span_text(src, c.name).to_owned()));
                }
                ClassMemberKind::Method(m) => {
                    let method = span_text(src, m.name).to_owned();
                    for param in &m.params {
                        if deprecated(&param.attributes) {
                            let name = strip_sigil(span_text(src, param.name)).to_owned();
                            found.push(Member::Param(method.clone(), name));
                        }
                    }
                    if deprecated(&m.attributes) {
                        found.push(Member::Method(method));
                    }
                }
                _ => {}
            }
        }
        for case in cases {
            if deprecated(&case.attributes) {
                found.push(Member::Case(span_text(src, case.name.span).to_owned()));
            }
        }
        table
            .sites
            .extend(found.into_iter().map(|member| (owner.clone(), member)));
    }
}

/// Every `#[Core\Deprecated]` under one class, interface or enum body, held to
/// the nine declarations it attaches to, and each template it writes held to
/// the declaration under it. Asked from
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
    for attr in deprecations(groups, ctx, env) {
        template::check(template::Target::Type(decl, members), attr, ctx, env);
    }
    for member in members {
        match &member.kind {
            ClassMemberKind::Property(p) => {
                report_construct(&p.attributes, "a property", ctx, env);
                for attr in deprecations(&p.attributes, ctx, env) {
                    template::check(template::Target::Property(p), attr, ctx, env);
                }
                for hook in p.hooks.iter().flatten() {
                    report_on_hook(&hook.attributes, ctx, env);
                    if let Some(param) = &hook.param {
                        report_on_hook(&param.attributes, ctx, env);
                    }
                }
            }
            ClassMemberKind::Const(c) => {
                report_construct(&c.attributes, "a constant", ctx, env);
                for attr in deprecations(&c.attributes, ctx, env) {
                    template::check(template::Target::Const(c), attr, ctx, env);
                }
            }
            ClassMemberKind::Method(m) => {
                report_construct(&m.attributes, "a method", ctx, env);
                for attr in deprecations(&m.attributes, ctx, env) {
                    template::check(template::Target::Method(m), attr, ctx, env);
                }
                check_params(&m.params, ctx, env);
                for param in &m.params {
                    for attr in deprecations(&param.attributes, ctx, env) {
                        template::check(template::Target::Param(m, param), attr, ctx, env);
                    }
                }
            }
            _ => {}
        }
    }
    for case in cases {
        report_construct(&case.attributes, "an enum case", ctx, env);
        for attr in deprecations(&case.attributes, ctx, env) {
            template::check(template::Target::Case, attr, ctx, env);
        }
    }
}

/// Each `#[Core\Deprecated]` among `groups`.
fn deprecations<'g>(
    groups: &'g [AttributeGroup],
    ctx: &Ctx<'_>,
    env: &Env<'_>,
) -> Vec<&'g Attribute> {
    groups
        .iter()
        .flat_map(|group| &group.attributes)
        .filter(|attr| crate::derive::attribute_is(attr, crate::derive::DEPRECATED, ctx, env))
        .collect()
}

fn check_params(params: &[Param], ctx: &Ctx<'_>, env: &mut Env<'_>) {
    for param in params {
        report_construct(&param.attributes, "a parameter", ctx, env);
    }
}

/// `construct` in each `#[Core\Deprecated]` among `groups`, which sit on
/// `site` — a declaration that is not a class, so no `new` ever names it.
fn report_construct(groups: &[AttributeGroup], site: &str, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    for attr in deprecations(groups, ctx, env) {
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
    for attr in deprecations(groups, ctx, env) {
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
