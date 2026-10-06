//! `rule:attributes/a-deprecation-names-its-replacement-as-code`'s template
//! check: a `replace` or `construct` is compiled where it is declared, so a
//! template that cannot become working code at a use is refused once, above
//! the declaration, and never at the use.
//!
//! **Where the template is parsed.** It is code inside a string literal. The
//! literal's text is parsed in [`nvs_diagnostics::SourceFile::code_at`]'s copy of the file,
//! which keeps the file's id and the offsets the text has in it, so every node
//! of the template names the bytes it was written at. Name resolution, which
//! reads a name's text out of `env.src` by its span, then sees the template's
//! own text, and an error inside it points at the place in the literal. A
//! literal whose value is not its written text — one with an escape sequence
//! in it — has no such copy, and is refused as not parsing.
//!
//! **What is checked.** The template is inferred in the declaration's scope,
//! with the method's parameters and, on an instance member, `$this` bound,
//! into a scratch tail of `env.diags` that is cut off afterwards: an error in
//! it becomes one `E0846` at the literal. Every member its check resolved is
//! read back out of the expression table, from the mark taken before it, and
//! each is held to two questions — whether it is deprecated, and whether it is
//! less visible than the declaration the template replaces. A constant has no
//! visibility in its signature, so it is never the less visible one.

use nvs_diagnostics::{Diagnostic, Diagnostics, Span, code};
use nvs_hir::QName;
use nvs_syntax::ast::{
    Attribute, ClassMember, ClassMemberKind, ConstMember, Expr, ExprKind, MethodMember, Modifier,
    Param, PropertyMember, Visibility,
};

use super::{CONSTRUCT, Decl, Member, REPLACE};
use crate::expr::{class_of_ctx, infer, is_assignable};
use crate::expr_table::ExprInfo;
use crate::locals::{Live, LocalScope};
use crate::lower::lower_optional_type;
use crate::signatures::{
    property_visibility, resolve_const_owned, resolve_method, resolve_property_owned,
};
use crate::ty::{Ty, TypeId};
use crate::{Ctx, Env, span_text, strip_sigil};

/// The declaration a `#[Core\Deprecated]` is written on.
#[derive(Clone, Copy)]
pub(super) enum Target<'a> {
    /// A class, an interface or an enum, with the members its body declares.
    Type(Decl, &'a [ClassMember]),
    Method(&'a MethodMember),
    Property(&'a PropertyMember),
    Const(&'a ConstMember),
    Case,
    /// A parameter, and the method that declares it.
    Param(&'a MethodMember, &'a Param),
}

/// Checks the `replace` and `construct` templates `attr` writes over `target`.
pub(super) fn check(target: Target<'_>, attr: &Attribute, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    for field in &attr.fields {
        let which = span_text(env.src, field.name);
        if which != REPLACE && which != CONSTRUCT {
            continue;
        }
        // Any other value is the payload check's to refuse.
        let ExprKind::Str(lit) = field.value.kind else {
            continue;
        };
        let Some(text) = written_text(lit, env) else {
            report_unparsed(
                lit,
                "this text has an escape sequence",
                Some("write the template without `\\` escapes"),
                env,
            );
            continue;
        };
        let at = lit.start + 1;
        match (which, target) {
            (REPLACE, Target::Type(decl, members)) => {
                check_type_name(decl, members, &text, lit, ctx, env);
            }
            // `construct` anywhere else is `super::report_construct`'s.
            (_, Target::Type(Decl::Class, members)) => {
                check_construct(members, attr, &text, at, lit, ctx, env);
            }
            (REPLACE, Target::Param(method, param)) => {
                check_param(method, param, &text, at, lit, ctx, env);
            }
            (REPLACE, _) => check_member(target, &text, at, lit, ctx, env),
            _ => {}
        }
    }
}

/// The literal's value, when it is exactly the text written between its
/// quotes — the one case [`nvs_diagnostics::SourceFile::code_at`] lines a template up with.
fn written_text(lit: Span, env: &Env<'_>) -> Option<String> {
    let value = nvs_syntax::string_lit::cook_string_literal(env.src, lit);
    let inner = Span::new(lit.file, lit.start + 1, lit.end.saturating_sub(1));
    (span_text(env.src, inner) == value).then_some(value)
}

/// What a template is compiled with: whether `$this` is bound, and the
/// parameters in scope.
struct Bindings<'a> {
    this: bool,
    params: &'a [Param],
}

/// A method's, a property's, a constant's or an enum case's `replace`: one
/// expression whose type fits the member.
fn check_member(
    target: Target<'_>,
    text: &str,
    at: u32,
    lit: Span,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    let owner = ctx.current_class;
    let (bindings, want, visibility) = match target {
        Target::Method(m) => {
            let is_ctor = span_text(env.src, m.name) == "constructor";
            let this = !is_ctor && !m.modifiers.contains(&Modifier::Static);
            let want = if is_ctor {
                Some(class_of_ctx(ctx, env))
            } else {
                let ret = lower_optional_type(m.return_type.as_ref(), ctx, env);
                (!matches!(env.interner.get(ret), Ty::Void)).then_some(ret)
            };
            let bindings = Bindings {
                this,
                params: &m.params,
            };
            (bindings, want, visibility_of(&m.modifiers))
        }
        Target::Property(p) => {
            let name = strip_sigil(span_text(env.src, p.name));
            let want = owner
                .and_then(|o| env.signatures.get(o))
                .and_then(|sig| sig.properties.get(name).copied());
            let this = !p.modifiers.contains(&Modifier::Static);
            (
                Bindings { this, params: &[] },
                want,
                visibility_of(&p.modifiers),
            )
        }
        Target::Const(c) => {
            let name = span_text(env.src, c.name);
            let want = owner
                .and_then(|o| env.signatures.get(o))
                .and_then(|sig| sig.constants.get(name).map(|k| k.ty));
            let bindings = Bindings {
                this: false,
                params: &[],
            };
            (bindings, want, visibility_of(&c.modifiers))
        }
        Target::Case => {
            let want = Some(class_of_ctx(ctx, env));
            let bindings = Bindings {
                this: false,
                params: &[],
            };
            (bindings, want, Visibility::Public)
        }
        Target::Type(..) | Target::Param(..) => return,
    };
    let Some(expr) = parse(text, at, lit, env) else {
        return;
    };
    compile(&expr, &bindings, want, visibility, lit, ctx, env);
}

/// A parameter's `replace`: one named argument, `name: value`, naming another
/// parameter of the same method, whose value fits that parameter.
fn check_param(
    method: &MethodMember,
    param: &Param,
    text: &str,
    at: u32,
    lit: Span,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    let Some((name, value_at)) = argument_name(text) else {
        report_unparsed(
            lit,
            "this is not one named argument",
            Some("write the parameter that replaces this one, then its value: `limit: $count`"),
            env,
        );
        return;
    };
    let own = strip_sigil(span_text(env.src, param.name));
    let other = method
        .params
        .iter()
        .find(|p| strip_sigil(span_text(env.src, p.name)) == name && name != own);
    let Some(other) = other else {
        let method_name = span_text(env.src, method.name).to_owned();
        env.diags.report(
            Diagnostic::error(
                code::E_DEPRECATION_TEMPLATE_DOES_NOT_COMPILE,
                format!("`{method_name}` has no other parameter named `{name}`"),
            )
            .with_primary(lit, "this template names a parameter that does not exist"),
        );
        return;
    };
    let want = lower_optional_type(other.ty.as_ref(), ctx, env);
    let value = &text[value_at..];
    let Some(expr) = parse(value, at + u32::try_from(value_at).unwrap_or(0), lit, env) else {
        return;
    };
    let bindings = Bindings {
        this: !method.modifiers.contains(&Modifier::Static),
        params: &method.params,
    };
    compile(
        &expr,
        &bindings,
        Some(want),
        visibility_of(&method.modifiers),
        lit,
        ctx,
        env,
    );
}

/// `name: value`'s name, and the offset in `text` its value starts at.
fn argument_name(text: &str) -> Option<(&str, usize)> {
    let colon = text.find(':')?;
    if text[colon + 1..].starts_with(':') {
        return None;
    }
    let name = text[..colon].trim();
    let mut chars = name.chars();
    let first = chars.next()?;
    let is_ident =
        (first.is_alphabetic() || first == '_') && chars.all(|c| c.is_alphanumeric() || c == '_');
    is_ident.then_some((name, colon + 1))
}

/// A class's `construct`: a template over the constructor's parameters, for
/// the `new` it replaces, with the type of the class `replace` names or of
/// this class.
fn check_construct(
    members: &[ClassMember],
    attr: &Attribute,
    text: &str,
    at: u32,
    lit: Span,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    let ctor = members.iter().find_map(|member| match &member.kind {
        ClassMemberKind::Method(m) if span_text(env.src, m.name) == "constructor" => Some(m),
        _ => None,
    });
    let replacement = attr.fields.iter().find_map(|field| {
        let ExprKind::Str(lit) = field.value.kind else {
            return None;
        };
        (span_text(env.src, field.name) == REPLACE)
            .then(|| written_text(lit, env))
            .flatten()
            .map(|text| nvs_hir::resolve_ref(text.trim(), ctx.namespace, ctx.imports))
            .filter(|qname| env.symbols.get(qname).is_some())
    });
    let want = match replacement {
        Some(qname) => env.interner.class(qname),
        None => class_of_ctx(ctx, env),
    };
    let Some(expr) = parse(text, at, lit, env) else {
        return;
    };
    let bindings = Bindings {
        this: false,
        params: ctor.map_or(&[][..], |m| &m.params),
    };
    let visibility = ctor.map_or(Visibility::Public, |m| visibility_of(&m.modifiers));
    compile(&expr, &bindings, Some(want), visibility, lit, ctx, env);
}

/// A class's, an interface's or an enum's `replace`: one type name, declared,
/// not deprecated, and — on a class — declaring every public member the class
/// declares.
fn check_type_name(
    decl: Decl,
    members: &[ClassMember],
    text: &str,
    lit: Span,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    let name = text.trim();
    let is_name = name.split('\\').enumerate().all(|(i, part)| {
        let mut chars = part.chars();
        (i == 0 && part.is_empty())
            || chars.next().is_some_and(|c| c.is_alphabetic() || c == '_')
                && chars.all(|c| c.is_alphanumeric() || c == '_')
    }) && !name.is_empty();
    if !is_name {
        report_unparsed(
            lit,
            "this is not one type name",
            Some("on a class, an interface or an enum, `replace` names the type that replaces it"),
            env,
        );
        return;
    }
    let qname = nvs_hir::resolve_ref(name, ctx.namespace, ctx.imports);
    if env.symbols.get(&qname).is_none() {
        env.diags.report(
            Diagnostic::error(
                code::E_DEPRECATION_TEMPLATE_DOES_NOT_COMPILE,
                format!("`{name}` is not declared"),
            )
            .with_primary(lit, "this template names a type that does not exist"),
        );
        return;
    }
    if env.deprecations.contains(&qname, Member::Type) {
        report_deprecated(lit, &describe(&qname, &Member::Type), env);
        return;
    }
    if decl == Decl::Class
        && let Some(own) = ctx.current_class
    {
        check_replacement_members(own, &qname, members, lit, env);
    }
}

/// `rule:attributes/a-deprecation-names-its-replacement-as-code`'s class
/// replacement: every public method, property and constant `own` declares is
/// declared on `replacement` too, public, and with a type that fits.
fn check_replacement_members(
    own: &QName,
    replacement: &QName,
    members: &[ClassMember],
    lit: Span,
    env: &mut Env<'_>,
) {
    let Some(sig) = env.signatures.get(own) else {
        return;
    };
    let mut missing = Vec::new();
    for member in members {
        match &member.kind {
            ClassMemberKind::Method(m) => {
                let name = span_text(env.src, m.name);
                let Some(want) = sig.methods.get(name) else {
                    continue;
                };
                if name == "constructor" || want.visibility != Visibility::Public {
                    continue;
                }
                let fits = resolve_method(replacement, name, env.signatures, env.graph)
                    .is_some_and(|(_, got)| {
                        got.visibility == Visibility::Public
                            && got.is_static == want.is_static
                            && got.required() <= want.params.len()
                            && is_assignable(
                                got.return_ty,
                                want.return_ty,
                                env.interner,
                                env.graph,
                                env.signatures,
                            )
                    });
                if !fits {
                    missing.push(format!("`{name}()`"));
                }
            }
            ClassMemberKind::Property(p) => {
                let name = strip_sigil(span_text(env.src, p.name));
                let Some(&want) = sig.properties.get(name) else {
                    continue;
                };
                if property_visibility(own, name, env.signatures) != Visibility::Public {
                    continue;
                }
                let fits = resolve_property_owned(replacement, name, env.signatures, env.graph)
                    .is_some_and(|(owner, got)| {
                        property_visibility(&owner, name, env.signatures) == Visibility::Public
                            && is_assignable(got, want, env.interner, env.graph, env.signatures)
                    });
                if !fits {
                    missing.push(format!("`${name}`"));
                }
            }
            ClassMemberKind::Const(c) => {
                let name = span_text(env.src, c.name);
                if visibility_of(&c.modifiers) != Visibility::Public {
                    continue;
                }
                if resolve_const_owned(replacement, name, env.signatures, env.graph).is_none() {
                    missing.push(format!("`{name}`"));
                }
            }
            _ => {}
        }
    }
    if missing.is_empty() {
        return;
    }
    env.diags.report(
        Diagnostic::error(
            code::E_DEPRECATION_REPLACEMENT_MISSES_A_MEMBER,
            format!(
                "`{replacement}` does not declare {} with a type that fits",
                missing.join(", ")
            ),
        )
        .with_primary(
            lit,
            "code that uses the deprecated class needs these members",
        )
        .with_help(format!(
            "declare each of them as `public` on `{replacement}`, or name another class"
        )),
    );
}

/// Parses `text`, written at `at` inside `lit`, as one expression, or reports
/// why it is not one.
fn parse(text: &str, at: u32, lit: Span, env: &mut Env<'_>) -> Option<Expr> {
    let Some(copy) = env.src.code_at(at, text) else {
        report_unparsed(lit, "this is not one expression", None, env);
        return None;
    };
    let mut diags = Diagnostics::new();
    let expr = nvs_syntax::parse_expression(&copy, &mut diags);
    let end = at + u32::try_from(text.trim_end().len()).unwrap_or(0);
    if diags.has_errors() || expr.span.end != end {
        let first = diags
            .iter()
            .find(|d| d.is_error())
            .map(|d| d.message.clone());
        let label = first.unwrap_or_else(|| "this is more than one expression".to_owned());
        report_unparsed(lit, &label, None, env);
        return None;
    }
    Some(expr)
}

/// Infers `expr` in the declaration's scope, then holds it to the member's
/// type, and every member it names to the deprecation set and to
/// `visibility`.
fn compile(
    expr: &Expr,
    bindings: &Bindings<'_>,
    want: Option<TypeId>,
    visibility: Visibility,
    lit: Span,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    let inner = Ctx {
        namespace: ctx.namespace,
        imports: ctx.imports,
        current_class: ctx.current_class,
        current_hook: None,
        in_constructor: false,
        generator_elem: None,
        in_anon_fn: false,
    };
    let mut scope = LocalScope::new();
    let mut live = Live::default();
    if bindings.this && inner.current_class.is_some() {
        let this_ty = class_of_ctx(&inner, env);
        scope.declare_param("this".to_owned(), this_ty, lit);
        live.insert("this".to_owned());
    }
    for param in bindings.params {
        let ty = lower_optional_type(param.ty.as_ref(), &inner, env);
        let ty = if param.variadic {
            env.interner.array(ty)
        } else {
            ty
        };
        let name = strip_sigil(span_text(env.src, param.name)).to_owned();
        scope.declare_param(name.clone(), ty, param.name);
        live.insert(name);
    }

    let before = env.diags.len();
    let mark = env.exprs.len();
    let got = infer(expr, want, &mut live, &scope, &inner, env);
    let first_error = env
        .diags
        .iter()
        .skip(before)
        .find(|d| d.is_error())
        .cloned();
    env.diags.truncate(before);
    if let Some(error) = first_error {
        let mut diag = Diagnostic::error(
            code::E_DEPRECATION_TEMPLATE_DOES_NOT_COMPILE,
            "the replacement does not compile",
        )
        .with_primary(lit, "a use of the deprecated code is rewritten to this");
        diag = match error.primary_span() {
            Some(span) if span != lit => diag.with_secondary(span, error.message),
            _ => diag.with_note(error.message),
        };
        env.diags.report(diag);
        return;
    }
    if let Some(want) = want
        && !is_assignable(got, want, env.interner, env.graph, env.signatures)
    {
        let (got, want) = (env.interner.describe(got), env.interner.describe(want));
        env.diags.report(
            Diagnostic::error(
                code::E_DEPRECATION_TEMPLATE_DOES_NOT_FIT,
                format!("the replacement is `{got}`, and the deprecated code is `{want}`"),
            )
            .with_primary(lit, format!("this is `{got}`"))
            .with_help(format!("write a replacement whose type fits `{want}`")),
        );
        return;
    }
    for (owner, member, level) in named(mark, env) {
        if env.deprecations.contains(&owner, member.clone()) {
            report_deprecated(lit, &describe(&owner, &member), env);
            return;
        }
        if rank(level) < rank(visibility) {
            let what = describe(&owner, &member);
            env.diags.report(
                Diagnostic::error(
                    code::E_DEPRECATION_TEMPLATE_LESS_VISIBLE,
                    format!("the replacement uses {what}, which is {}", word(level)),
                )
                .with_primary(lit, format!("code that cannot see {what} uses this"))
                .with_help(format!(
                    "the deprecated code is {}, so its replacement can only use members that \
                     are at least {}",
                    word(visibility),
                    word(visibility)
                )),
            );
            return;
        }
    }
}

/// Every declaration the expression table resolved after `mark`, with the
/// class that declares it and its visibility.
fn named(mark: usize, env: &Env<'_>) -> Vec<(QName, Member, Visibility)> {
    let mut out = Vec::new();
    let method = |class: &QName, name: &str, out: &mut Vec<_>| {
        if let Some((owner, sig)) = resolve_method(class, name, env.signatures, env.graph) {
            out.push((owner, Member::Method(name.to_owned()), sig.visibility));
        }
    };
    for info in env.exprs.since(mark) {
        match info {
            ExprInfo::Call(call) | ExprInfo::CallableRef(call) | ExprInfo::ClassRefCall(call) => {
                if call.is_static {
                    out.push((call.class.clone(), Member::Type, Visibility::Public));
                }
                method(&call.class, &call.method, &mut out);
            }
            ExprInfo::New { class, ctor, .. }
            | ExprInfo::NewDynamic {
                bound: class, ctor, ..
            } => {
                out.push((class.clone(), Member::Type, Visibility::Public));
                if let Some(ctor) = ctor {
                    method(&ctor.class, "constructor", &mut out);
                }
            }
            ExprInfo::Property { class, name, .. }
            | ExprInfo::StaticProperty { class, name, .. }
            | ExprInfo::HookedProperty { class, name, .. } => {
                let owner = resolve_property_owned(class, name, env.signatures, env.graph)
                    .map_or_else(|| class.clone(), |(owner, _)| owner);
                let level = property_visibility(&owner, name, env.signatures);
                out.push((owner, Member::Property(name.clone()), level));
            }
            ExprInfo::ClassConst { class, name, .. } => {
                out.push((class.clone(), Member::Type, Visibility::Public));
                let owner = resolve_const_owned(class, name, env.signatures, env.graph)
                    .map_or_else(|| class.clone(), |(owner, _)| owner);
                out.push((owner, Member::Const(name.clone()), Visibility::Public));
            }
            ExprInfo::EnumCase { enum_, case, .. } => {
                out.push((enum_.clone(), Member::Type, Visibility::Public));
                out.push((
                    enum_.clone(),
                    Member::Case(case.clone()),
                    Visibility::Public,
                ));
            }
            _ => {}
        }
    }
    out
}

fn visibility_of(modifiers: &[Modifier]) -> Visibility {
    if modifiers.contains(&Modifier::Private) {
        Visibility::Private
    } else if modifiers.contains(&Modifier::Protected) {
        Visibility::Protected
    } else {
        Visibility::Public
    }
}

const fn rank(level: Visibility) -> u8 {
    match level {
        Visibility::Private => 0,
        Visibility::Protected => 1,
        Visibility::Public => 2,
    }
}

const fn word(level: Visibility) -> &'static str {
    match level {
        Visibility::Private => "private",
        Visibility::Protected => "protected",
        Visibility::Public => "public",
    }
}

fn describe(owner: &QName, member: &Member) -> String {
    match member {
        Member::Type => format!("`{owner}`"),
        Member::Method(name) => format!("`{owner}::{name}()`"),
        Member::Property(name) => format!("`{owner}::${name}`"),
        Member::Const(name) | Member::Case(name) => format!("`{owner}::{name}`"),
        Member::Param(method, name) => format!("the parameter `${name}` of `{owner}::{method}()`"),
    }
}

fn report_unparsed(lit: Span, label: &str, help: Option<&str>, env: &mut Env<'_>) {
    let mut diag = Diagnostic::error(
        code::E_DEPRECATION_TEMPLATE_DOES_NOT_PARSE,
        "the replacement is not one expression",
    )
    .with_primary(lit, label);
    if let Some(help) = help {
        diag = diag.with_help(help);
    }
    env.diags.report(diag);
}

fn report_deprecated(lit: Span, what: &str, env: &mut Env<'_>) {
    env.diags.report(
        Diagnostic::error(
            code::E_DEPRECATION_TEMPLATE_NAMES_A_DEPRECATED,
            format!("the replacement uses {what}, which is also deprecated"),
        )
        .with_primary(lit, "applying this fix would leave a second warning")
        .with_help(format!("write what replaces {what} here instead")),
    );
}
