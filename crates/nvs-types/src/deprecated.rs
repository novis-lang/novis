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
//!
//! **Where `W1003` is raised.** A body is checked inside a [`window`], which
//! is closed when the body sits in a deprecated declaration or a member of a
//! deprecated class. When the body is done, every entry the expression table
//! recorded for it is mapped to the declarations it names — the template
//! check's own mapping — and each deprecated one warns once per span. A type
//! position warns as it is lowered, and an override as its method is entered.
//! Nothing outside a window warns, so a template, a signature or a later pass
//! that lowers the same annotation again never raises a second warning.
//!
//! Each warning names the template filled in at its use, and carries that
//! text as its fix where [`fix`] can write one.

mod fix;
mod template;

use std::fmt::Write as _;

use nvs_diagnostics::{Diagnostic, Span, code};
use nvs_hir::QName;
use nvs_syntax::SyntaxIndex;
use nvs_syntax::ast::{
    Attribute, AttributeGroup, ClassMember, ClassMemberKind, EnumCase, ExprKind, NamespaceDecl,
    Param, Stmt, StmtKind,
};

use crate::expr_table::{ArgSlot, DeprecationCheck, ExprInfo};
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
/// itself. A property's and a parameter's name carry no `$` sigil.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Member {
    /// The class, interface or enum itself.
    Type,
    /// A method, by name.
    Method(String),
    /// A property, by name.
    Property(String),
    /// A class constant, by name.
    Const(String),
    /// An enum case, by name.
    Case(String),
    /// A method's parameter: the method's name, then the parameter's, with no
    /// `$` sigil.
    Param(String, String),
}

/// Every declaration in the program that carries `#[Core\Deprecated]`, with
/// what its payload wrote.
#[derive(Default)]
pub(crate) struct Deprecations {
    sites: FxHashMap<(QName, Member), Deprecation>,
}

/// The string fields of one `#[Core\Deprecated]`, as their values read, and
/// what its fix reads where the declaration is written. A field that is not
/// written, or not a string, is `None`.
#[derive(Default)]
pub(crate) struct Deprecation {
    since: Option<String>,
    note: Option<String>,
    replace: Option<String>,
    construct: Option<String>,
    /// The parameters the template fills in: the method's, or on a class the
    /// constructor's, which `construct` fills in.
    params: Vec<fix::Param>,
    /// Where the template's names resolve.
    scope: fix::Scope,
}

impl Deprecation {
    fn of(attr: &Attribute, src: &nvs_diagnostics::SourceFile, scope: fix::Scope) -> Self {
        let mut out = Self {
            scope,
            ..Self::default()
        };
        for field in &attr.fields {
            let ExprKind::Str(lit) = field.value.kind else {
                continue;
            };
            let value = Some(nvs_syntax::string_lit::cook_string_literal(src, lit));
            match span_text(src, field.name) {
                "since" => out.since = value,
                "note" => out.note = value,
                REPLACE => out.replace = value,
                CONSTRUCT => out.construct = value,
                _ => {}
            }
        }
        out
    }
}

/// The text one `#[Core\Deprecated]` wrote, for a tool that shows it: an
/// editor's completion item and hover. A field that is not written, or not a
/// string, is `None`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Notice {
    /// The version the declaration was deprecated in.
    pub since: Option<String>,
    /// Why, in the author's words.
    pub note: Option<String>,
    /// The template a use is rewritten to.
    pub replace: Option<String>,
    /// On a class, the template a `new` is rewritten to.
    pub construct: Option<String>,
}

/// What [`notices`] returns: each deprecated declaration, keyed by the class,
/// interface or enum that declares it and the member.
pub type Notices = FxHashMap<(QName, Member), Notice>;

/// Every declaration in `files` that carries `#[Core\Deprecated]`, keyed by the
/// class, interface or enum that declares it and the member, with the text its
/// payload wrote.
///
/// The same walk the checker makes before it checks a body, so an attribute
/// name means here what it means to `W1003`. It reads attributes and nothing
/// else, and the map is O(deprecated declarations in the program).
#[must_use]
pub fn notices(files: &[ProgramFile<'_>]) -> Notices {
    build_table(files)
        .sites
        .into_iter()
        .map(|(key, deprecation)| {
            let notice = Notice {
                since: deprecation.since,
                note: deprecation.note,
                replace: deprecation.replace,
                construct: deprecation.construct,
            };
            (key, notice)
        })
        .collect()
}

impl Deprecations {
    /// Whether `member` of `owner` — the declaring class, never a subclass a
    /// use reached it through — is deprecated.
    pub(crate) fn contains(&self, owner: &QName, member: Member) -> bool {
        self.sites.contains_key(&(owner.clone(), member))
    }

    fn get(&self, owner: &QName, member: &Member) -> Option<&Deprecation> {
        self.sites.get(&(owner.clone(), member.clone()))
    }
}

/// The uses one body has already warned about, so a span that two table
/// entries describe warns once.
#[derive(Default)]
pub(crate) struct Uses(FxHashSet<(Span, QName, Member)>);

/// Checks one body with `W1003` switched on, unless `owner` is deprecated or
/// `member` of it is: then nothing inside it warns. The table entries the body
/// recorded are warned about when it is done.
pub(crate) fn window<'e, R>(
    owner: Option<&QName>,
    member: Option<Member>,
    env: &mut Env<'e>,
    check: impl FnOnce(&mut Env<'e>) -> R,
) -> R {
    let quiet = owner.is_some_and(|owner| {
        env.deprecations.contains(owner, Member::Type)
            || member.is_some_and(|member| env.deprecations.contains(owner, member))
    });
    let outer = std::mem::replace(&mut env.deprecated_uses, (!quiet).then(Uses::default));
    let mark = env.exprs.len();
    let out = check(env);
    if env.deprecated_uses.is_some() {
        warn_uses(mark, env);
    }
    env.deprecated_uses = outer;
    out
}

/// `W1003` at `span`, when `member` of `owner` is deprecated and a window is
/// open.
pub(crate) fn warn(span: Span, owner: &QName, member: Member, env: &mut Env<'_>) {
    report(span, owner, member, false, &mut None, env);
}

/// `W1003` at an override of a deprecated method: `name` is declared by
/// `class`, and one of its ancestors deprecates it. There is no replacement to
/// offer, because the override is a declaration and not a use.
pub(crate) fn warn_override(class: &QName, name: &str, span: Span, env: &mut Env<'_>) {
    if env.deprecated_uses.is_none() || name == "constructor" {
        return;
    }
    let table = env.deprecations;
    let member = Member::Method(name.to_owned());
    let mut seen = FxHashSet::default();
    let mut stack = vec![class.clone()];
    while let Some(next) = stack.pop() {
        if !seen.insert(next.clone()) {
            continue;
        }
        if &next != class
            && let Some(deprecation) = table.get(&next, &member)
        {
            let message = format!(
                "`{class}::{name}()` overrides {}",
                message(&template::describe(&next, &member), deprecation, None)
            );
            env.diags.report(
                Diagnostic::warning(code::W_DEPRECATED, message)
                    .with_primary(span, "this overrides a deprecated method"),
            );
            return;
        }
        if let Some(links) = env.graph.get(&next) {
            stack.extend(links.extends.iter().cloned());
            stack.extend(links.implements.iter().cloned());
        }
    }
}

/// `W1003` at `span`, naming the template filled in at this use and carrying
/// it as the fix where [`fix::fill`] writes one. `index` is the file's syntax
/// index, built the first time a fix needs it.
fn report(
    span: Span,
    owner: &QName,
    member: Member,
    at_new: bool,
    index: &mut Option<SyntaxIndex>,
    env: &mut Env<'_>,
) {
    let table = env.deprecations;
    let Some(deprecation) = table.get(owner, &member) else {
        return;
    };
    let Some(uses) = &mut env.deprecated_uses else {
        return;
    };
    if !uses.0.insert((span, owner.clone(), member.clone())) {
        return;
    }
    let template = match at_new && deprecation.construct.is_some() {
        true => deprecation.construct.as_deref(),
        false => deprecation.replace.as_deref(),
    };
    let site = nvs_hir::import_site(env.stmts, env.src, span.start);
    let fill = template
        .and_then(|_| fix::fill(span, owner, &member, at_new, deprecation, index, env))
        .filter(|fill| fill.imports.is_empty() || site.is_some());
    let what = template::describe(owner, &member);
    let shown = fill
        .as_ref()
        .map_or(template, |fill| Some(fill.shown.as_str()));
    let mut diagnostic =
        Diagnostic::warning(code::W_DEPRECATED, message(&what, deprecation, shown))
            .with_primary(span, "this is deprecated");
    match (&fill, template) {
        (Some(fill), _) => {
            diagnostic = diagnostic.with_fix(
                fill.span,
                fill.text.clone(),
                format!("replace with `{}`", fill.shown),
            );
            if let Some(site) = site.filter(|_| !fill.imports.is_empty()) {
                diagnostic = diagnostic.with_fix(
                    site.span(span.file),
                    site.use_lines(&fill.imports),
                    format!("import `{}`", fill.imports[0]),
                );
            }
        }
        (None, Some(template)) if member != Member::Type => {
            diagnostic = diagnostic.with_help(format!(
                "write `{template}` here, with the values this use passes in place of its \
                 parameters"
            ));
        }
        (None, _) => {}
    }
    if at_new || !matches!(member, Member::Type | Member::Method(_)) {
        let message = message(&what, deprecation, shown);
        let (line, _) = env.src.line_col(span.start);
        let at = format!("{}:{}", env.src.name(), line + 1);
        env.exprs
            .record_deprecation_check(span, DeprecationCheck { message, at });
    }
    env.diags.report(diagnostic);
}

/// Records the entry check of method `name` of `class`, declared at `span`,
/// when the method is deprecated. Its message names the template as it is
/// written, because no one use fills it in.
pub(crate) fn record_entry(class: &QName, name: &str, span: Span, env: &mut Env<'_>) {
    let member = Member::Method(name.to_owned());
    let Some(deprecation) = env.deprecations.get(class, &member) else {
        return;
    };
    let message = message(
        &template::describe(class, &member),
        deprecation,
        deprecation.replace.as_deref(),
    );
    env.exprs.record_deprecated_entry(
        span,
        DeprecationCheck {
            message,
            at: String::new(),
        },
    );
}

/// The member, then `since`, then `note`, then the replacement.
fn message(what: &str, deprecation: &Deprecation, replacement: Option<&str>) -> String {
    let mut out = format!("{what} is deprecated");
    if let Some(since) = &deprecation.since {
        let _ = write!(out, " since {since}");
    }
    out.push('.');
    if let Some(note) = &deprecation.note {
        let _ = write!(out, " {note}");
    }
    if let Some(replacement) = replacement {
        let _ = write!(out, " Use `{replacement}` instead.");
    }
    out
}

/// Warns about every deprecated declaration the entries recorded after `mark`
/// name, and every deprecated parameter a call among them passed.
fn warn_uses(mark: usize, env: &mut Env<'_>) {
    let mut found = Vec::new();
    for (span, info) in env.exprs.since_at(mark) {
        let at_new = matches!(info, ExprInfo::New { .. } | ExprInfo::NewDynamic { .. });
        for (owner, member, _) in template::resolved(info, env) {
            let at_new = at_new && member == Member::Type;
            found.push((span, owner, member, at_new));
        }
        let call = match info {
            ExprInfo::Call(call) | ExprInfo::ClassRefCall(call) => Some(call),
            ExprInfo::New { ctor, .. } | ExprInfo::NewDynamic { ctor, .. } => ctor.as_ref(),
            _ => None,
        };
        for slot in call.iter().flat_map(|call| &call.arg_slots) {
            let (ArgSlot::Param(index) | ArgSlot::Spread(index)) = *slot else {
                continue;
            };
            let call = call.expect("a slot comes from a call");
            if let Some(name) = call.param_names.get(index) {
                let member = Member::Param(call.method.clone(), name.clone());
                found.push((span, call.class.clone(), member, false));
            }
        }
    }
    let mut index = None;
    for (span, owner, member, at_new) in found {
        report(span, &owner, member, at_new, &mut index, env);
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
            groups
                .iter()
                .flat_map(|g| &g.attributes)
                .find(|attr| {
                    attr.member.is_none()
                        && attr.name.as_ref().is_some_and(|n| {
                            nvs_hir::resolve_ref(
                                span_text(src, n.span),
                                &current_ns,
                                &current_imports,
                            ) == QName::parse(crate::derive::DEPRECATED)
                        })
                })
                .map(|attr| {
                    Deprecation::of(attr, src, fix::Scope::new(&current_ns, &current_imports))
                })
        };
        let params = |params: &[Param]| params.iter().map(|p| fix::Param::of(p, src)).collect();
        let mut found = Vec::new();
        if let Some(mut d) = deprecated(attributes) {
            let constructor = members.iter().find_map(|member| match &member.kind {
                ClassMemberKind::Method(m) if span_text(src, m.name) == "constructor" => Some(m),
                _ => None,
            });
            if let Some(constructor) = constructor {
                d.params = params(&constructor.params);
            }
            found.push((Member::Type, d));
        }
        for member in members {
            match &member.kind {
                ClassMemberKind::Property(p) => {
                    if let Some(d) = deprecated(&p.attributes) {
                        let name = strip_sigil(span_text(src, p.name)).to_owned();
                        found.push((Member::Property(name), d));
                    }
                }
                ClassMemberKind::Const(c) => {
                    if let Some(d) = deprecated(&c.attributes) {
                        found.push((Member::Const(span_text(src, c.name).to_owned()), d));
                    }
                }
                ClassMemberKind::Method(m) => {
                    let method = span_text(src, m.name).to_owned();
                    for param in &m.params {
                        if let Some(d) = deprecated(&param.attributes) {
                            let name = strip_sigil(span_text(src, param.name)).to_owned();
                            found.push((Member::Param(method.clone(), name), d));
                        }
                    }
                    if let Some(mut d) = deprecated(&m.attributes) {
                        d.params = params(&m.params);
                        found.push((Member::Method(method), d));
                    }
                }
                _ => {}
            }
        }
        for case in cases {
            if let Some(d) = deprecated(&case.attributes) {
                found.push((Member::Case(span_text(src, case.name.span).to_owned()), d));
            }
        }
        table.sites.extend(
            found
                .into_iter()
                .map(|(member, d)| ((owner.clone(), member), d)),
        );
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
