//! `rule:attributes/attach-sites-and-forms` and `rule:attributes/payload-is-a-compile-time-constant`'s two rules about a `#[...]` payload: every field value is
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
//! ([`crate::expr::is_assignable`], `rule:types/shape-type`'s width subtyping). So an
//! attribute's name resolves in the ordinary namespace/`use` scope, and an
//! unresolvable one is the ordinary `E0303` rather than a refusal of its own.
//!
//! One consequence of § 2 is checked from here but owned elsewhere: `rule:security/secret-sinks-refuse`
//! 's sink for a `secret` class constant reaching a payload is
//! `crate::expr::quals`', where every other sink already lives.
//! [`check_value`] calls it at each value it reaches, that walk being the one
//! place every payload value passes.

use nvs_diagnostics::{Diagnostic, code};
use nvs_syntax::ast::{
    ArrayItem, Attribute, AttributeGroup, ClassMember, ClassMemberKind, EnumCase, Expr, ExprKind,
    Name, ObjectLiteralField, Param, UnaryOp,
};
use rustc_hash::FxHashSet;

use crate::expr::{check_expr, check_object_literal, is_assignable, report_mismatch};
use crate::locals::LocalScope;
use crate::ty::{Ty, TypeId};
use crate::{Ctx, Env, span_text};

/// Every attribute attached anywhere under one class/interface/enum body:
/// the declaration's own groups, each member's, each method parameter's and
/// each property hook's. One entry point per declaration kind would be a copy
/// of this walk each, so the caller hands over the lists it has.
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
                // `rule:attributes/access-payload`'s repeat rule is a question about the list
                // rather than about one payload, and this is the walk that
                // holds a method's whole list.
                crate::routes::check_one_access(&m.attributes, ctx, env);
                // The markers that mean nothing away from the attribute that
                // reads them — `rule:routing/a-query-parameter-is-declared-like-a-capture`'s `#[Query]`, `rule:attributes/api-adds-and-cannot-contradict`'s
                // `#[Api]`, `rule:attributes/access-is-a-required-sibling`'s `#[Access]` and `rule:tooling/commands-are-compiled`'s
                // `#[Option]`. Asked here because each owning pass walks only
                // the methods its own attribute selects, so a stray marker is
                // invisible to the pass that would refuse it, and this is the
                // walk that visits every method.
                crate::routes::check_stray_query(m, ctx, env);
                crate::routes::check_stray_api(m, ctx, env);
                crate::routes::check_stray_access(m, ctx, env);
                crate::commands::check_stray_options(m, ctx, env);
                check_params(&m.params, ctx, env);
            }
            _ => {}
        }
    }
    for case in cases {
        check_groups(&case.attributes, ctx, env);
    }
}

/// Every attribute on one parameter list — `rule:attributes/attach-sites-and-forms`'s parameter attach site.
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
        // `rule:attributes/attach-sites-and-forms`'s bare form names no shape to check against, so the
        // literal is checked as a well-formed literal and nothing more —
        // which the constant walk above has just done.
        return;
    };
    // `rule:core-classes/derive-attribute`'s compiler-recognized attributes are the one exemption to
    // § 1's "the name is a shape-typed alias", and it is a closed,
    // `Core`-owned roster rather than an escape hatch: such a name is matched
    // *nominally*, so what its payload may hold is the recognizing pass's own
    // option check rather than a shape. § 1's rule is about the userland
    // names, which are the only ones that could ever be aliases. The name is
    // resolved once, here, so that the roster test and the alias lookup below
    // cannot place one `Name` two different ways.
    let text = span_text(env.src, name.span).to_owned();
    let qname = nvs_hir::resolve_ref(&text, ctx.namespace, ctx.imports);
    if crate::derive::ATTRIBUTES
        .iter()
        .any(|want| qname == nvs_hir::QName::parse(want))
    {
        if constant {
            // Only the rosters that have a payload to check appear here; a
            // recognized name whose payload is empty by construction
            // (`#[Fixture]`) has nothing to say and says nothing.
            let recognized = |want: &str| qname == nvs_hir::QName::parse(want);
            if recognized(crate::derive::TEST) {
                crate::testing::check_payload(&attr.fields, ctx, env);
            } else if recognized(crate::derive::COMMAND) {
                check_roster(
                    "Command",
                    crate::commands::COMMAND_OPTIONS,
                    &attr.fields,
                    ctx,
                    env,
                );
            } else if recognized(crate::derive::OPTION) {
                check_roster(
                    "Option",
                    crate::commands::OPTION_OPTIONS,
                    &attr.fields,
                    ctx,
                    env,
                );
            } else if recognized(crate::derive::ROUTE) {
                check_roster("Route", crate::routes::OPTIONS, &attr.fields, ctx, env);
            } else if recognized(crate::derive::ACCESS) {
                // Two calls, because `rule:attributes/access-payload` states two kinds of rule:
                // the roster answers what is asked of every payload, and the
                // pass that owns the attribute answers what is about
                // `#[Access]` alone.
                check_roster(
                    "Access",
                    crate::routes::ACCESS_OPTIONS,
                    &attr.fields,
                    ctx,
                    env,
                );
                crate::routes::check_access(attr, env);
            } else if recognized(crate::derive::API) {
                // Only the roster here. `rule:attributes/api-adds-and-cannot-contradict`'s contradictions are
                // every one of them a comparison against the *declaration* —
                // its `#[Route]`, its return type, the classes the program
                // declares — so they are asked by the per-class walk that
                // holds those, exactly as `#[Route]`'s own path checks are.
                check_roster("Api", crate::routes::API_OPTIONS, &attr.fields, ctx, env);
            }
        }
        return;
    }
    let Some(shape) = resolve_shape_alias(&qname, name, ctx, env) else {
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
    let actual = check_object_literal(&attr.fields, None, &mut live, &scope, ctx, env);
    if !is_assignable(actual, shape, env.interner, env.graph, env.signatures) {
        report_mismatch(attr.payload, shape, actual, env);
    }
}

/// One recognized attribute's payload, checked against its roster of options.
///
/// The walk every `rule:core-classes/derive-attribute` name that carries a payload shares, because a
/// recognized name is matched *nominally* and so has no shape to be checked
/// against: what it may hold is a roster its own module declares — `rule:testing/test-attribute`'s is [`crate::testing`]'s, `rule:tooling/commands-are-compiled`'s are [`crate::commands`]',
/// `rule:routing/route-attribute`'s is [`crate::routes`]'. The answers per field are deliberately
/// these: a name no row declares, a value at the wrong type, and a name given
/// twice.
///
/// Called only for a payload [`check_attribute`] has already proved constant,
/// for this module's own reason: the author is told about a value they wrote
/// before they are told what it failed to satisfy. A rule *between* two
/// options — `rule:testing/runner-is-strict`'s `retries` requiring `because` — is one this walk
/// cannot state, and stays with the roster that declares it.
///
/// `attribute` is the name as a diagnostic writes it, without its `#[]`.
pub(crate) fn check_roster(
    attribute: &str,
    options: &[(&str, crate::testing::OptionTy)],
    fields: &[ObjectLiteralField],
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    // The scope is empty and stays empty: § 2 has just proved this payload
    // reads no variable, so there is no binding to mark live and none to
    // capture.
    let mut live = FxHashSet::default();
    let scope = LocalScope::new();
    let mut seen: Vec<String> = Vec::with_capacity(fields.len());
    for field in fields {
        let name = span_text(env.src, field.name).to_owned();
        let declared = options
            .iter()
            .find(|(option, _)| *option == name)
            .map(|(_, ty)| *ty);
        let expected = declared.and_then(|ty| ty.intern(env));
        check_expr(&field.value, expected, &mut live, &scope, ctx, env);
        if declared.is_none() {
            env.diags.report(
                Diagnostic::error(
                    code::E_UNKNOWN_OPTION,
                    format!("`{name}` is not an option of `#[{attribute}]`"),
                )
                .with_primary(field.span, "no such option")
                .with_help(format!("the options are: {}", roster_names(options))),
            );
        } else if seen.iter().any(|already| already == &name) {
            env.diags.report(
                Diagnostic::error(
                    code::E_DUPLICATE_DECLARATION,
                    format!("the option `{name}` is given twice"),
                )
                .with_primary(field.span, "already set above"),
            );
        }
        seen.push(name);
    }
}

/// A roster rendered for a help text — `skip: string, at: string, …` — so a
/// typo is answered with the options themselves rather than with a type
/// spelling nobody wrote.
fn roster_names(options: &[(&str, crate::testing::OptionTy)]) -> String {
    options
        .iter()
        .map(|(option, ty)| format!("{option}: {}", ty.describe()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// `rule:attributes/attach-sites-and-forms`'s named form: `Name` resolves, in the namespace/`use` scope
/// the attribute is written in, to a `type` alias whose expansion is a shape
/// type.
///
/// The answers are deliberately these. A name nothing declared is
/// the **ordinary** `E0303` any unresolvable name is — an attribute name is
/// not a new namespace, so it does not get a "no such attribute" of its own.
/// A name that resolves to something that is not a shape-typed alias is
/// `E0726`: a class is the spelling this exists to refuse, § 1 giving an
/// attribute no kind to instantiate, and a `type Id = int;` is the same
/// mistake one step along. Only a shape is handed back, and then the payload
/// is checked against it.
///
/// `qname` is [`check_attribute`]'s own resolution of `name`, which has
/// already taken `rule:core-classes/derive-attribute`'s recognized roster out of this function's way.
fn resolve_shape_alias(
    qname: &nvs_hir::QName,
    name: &Name,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) -> Option<TypeId> {
    let Some(alias) = env.aliases.get(qname).cloned() else {
        // The same roster [`crate::lower`]'s own name atom uses for "this
        // name denotes something," so one name is undeclared in one place.
        let declared = env.symbols.get(qname).is_some()
            || qname.is_core()
            || qname.is_reserved_global_class()
            || qname.is_reserved_global_interface();
        if declared {
            report_not_a_shape(name, &format!("`{qname}` is not a `type` alias"), env);
        } else {
            env.diags.report(nvs_hir::undeclared_name(
                nvs_hir::Undeclared {
                    qname,
                    text: span_text(env.src, name.span),
                    span: name.span,
                    namespace: ctx.namespace,
                    stmts: env.stmts,
                    src: env.src,
                },
                env.symbols,
                &nvs_stdlib::registry::type_names(),
            ));
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
            "`rule:attributes/attach-sites-and-forms`: a named attribute instantiates nothing — the name is only ever an \
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
        // `rule:security/secret-sinks-refuse`'s payload sink, asked of every value this walk reaches
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
            "`rule:attributes/payload-is-a-compile-time-constant`: an attribute payload is a literal, a class constant or an enum \
             case — it lives in the constant pool, so there is no point at which a variable, \
             a call or a `new` could be evaluated",
        ),
    );
    false
}

/// The closed list of shapes `rule:attributes/payload-is-a-compile-time-constant` admits. Closed on purpose: an
/// expression kind this does not name is refused, so a shape the grammar
/// grows is refused until someone decides it belongs in a constant pool,
/// rather than accepted because nothing said otherwise.
pub(crate) fn is_constant(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Null
        | ExprKind::Bool(_)
        | ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::Duration(_)
        | ExprKind::Str(_)
        // `Class::CONST` — a class constant, and `rule:enums/closed-integer-type`'s enum case with it,
        // which is the one spelling § 2 names beside a literal.
        | ExprKind::ClassConstAccess { .. }
        // `Class::class`, which § 2's list already covers: it *is* a class
        // constant, and the only one whose value the compiler resolves rather
        // than reads — `crate::expr::members::check_class_name_const` folds it
        // to the fully qualified name with no runtime step at all. Admitted
        // because `rule:attributes/api-adds-and-cannot-contradict` writes an `errors` entry's `type` as a name,
        // and a name is either this spelling or a magic string; `rule:programs/no-runtime-autoload`'s
        // whole premise is that a name resolves, so the string form would be
        // the one thing § 2 exists to refuse.
        | ExprKind::ClassNameConst { .. }
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
        ExprKind::Error(_) => true,
        _ => false,
    }
}
