//! [ADR 0077](../../../../docs/adr/0077-compile-time-routing.md) § 1's
//! `#[Route]`: what one route declaration may carry.
//!
//! # Why this is a recognized name rather than a shape alias
//!
//! § 1 writes the attribute as an ordinary
//! [ADR 0046](../../../../docs/adr/0046-attributes-shape-literal-metadata.md)
//! `type Core\Route = {path: string, method: Core\Http\Method, name?: string};`
//! and then adds the one thing that makes it not one: the compiler acts on the
//! attribute only when its name **resolves** to `Core\Route`, so a userland
//! `type Route = {…};` is not it however it is spelled and a framework carrying
//! its own `Route`-shaped literal does not contribute a route. That is
//! [ADR 0071](../../../../docs/adr/0071-derived-codecs.md) § 1's rule, so the
//! name sits on [`crate::derive::ATTRIBUTES`] and is matched *nominally* after
//! [`nvs_hir::resolve_ref`] — and, being matched nominally, it names no shape,
//! which is why what it may hold is the roster below rather than an alias
//! lookup.
//!
//! # What is checked here, and what the table still owes
//!
//! Two passes, and they are two because they read different things — the same
//! split [`crate::commands`] makes, for the same reason.
//!
//! **One payload at a time**, from [`crate::attributes`]'s per-attribute walk:
//! the field names against [`OPTIONS`], each value's type, and a field given
//! twice — [`crate::attributes::check_roster`], the walk every recognized name
//! with a payload shares. That is one declaration read on its own.
//!
//! **The whole program's routes**, as [`RouteTable`]: [`check_class_routes`]
//! adds one row per `#[Route]` as [`crate::check`]'s per-class walk reaches it,
//! and [`check_table`] then holds the collected rows to the two of § 1-§ 3's
//! four compile errors that are questions about the *enumeration* — a duplicate
//! route and a duplicate `name`. A row needs a `path` and a `method` to exist
//! at all, so this is also where an attribute that named neither is refused.
//!
//! # Known gaps
//!
//! 1. **The table is collected but not yet handed on**, so nothing reverses it:
//!    § 4's `Core\Router::url` still resolves no name, because the rows do not
//!    reach `nvs-ir`. [`crate::expr_table`] is the channel a compile-time fact
//!    already travels to lowering by, and is where this goes rather than a
//!    second return value on [`crate::check::check_program`].
//! 2. **Nothing reads the method the attribute is attached to**, so § 2's path
//!    grammar and the other two of § 3's compile errors are not reported: a
//!    `{param}` with no matching parameter, and a capture whose parameter type
//!    has no conversion from a segment ([`crate::commands::converts_from_string`]
//!    is that roster, and has no second copy here). A malformed `path` is
//!    admitted for the same reason — its grammar is checked by the pass that
//!    binds captures to parameters, because that is the pass that has to split
//!    it anyway.

use nvs_diagnostics::{Diagnostic, Diagnostics, Span, code};
use nvs_hir::QName;
use nvs_syntax::ast::{Attribute, ClassDecl, ClassMemberKind, ExprKind};
use rustc_hash::FxHashMap;

use crate::testing::OptionTy;
use crate::{Ctx, Env, span_text};

/// The enum a `method:` value is a case of, named once: [`OPTIONS`] places the
/// written value at it, and [`verb_of`] reads the case back out of the same
/// name.
const METHOD_ENUM: &str = r"Core\Http\Method";

const PATH: &str = "path";
const METHOD: &str = "method";
const NAME: &str = "name";

/// `#[Route(path: string, method: Core\Http\Method, name?: string)]` — ADR 0077
/// § 1's own spelling, in the order that section writes it.
///
/// `method` is an enum case rather than a string
/// ([ADR 0063](../../../../docs/adr/0063-core-api-conventions.md) R11), and an
/// enum case is one of the three things a payload may contain (ADR 0046 § 2);
/// it is the same `Core\Http\Method` `Core\Request::method` answers with, which
/// is why the row names that enum rather than a spelling of its own. There is
/// no `methods:` row: § 1 serves two verbs by repeating the attribute
/// (ADR 0046 § 3), so a union or an array here would be a second way to write
/// what the existing rule already covers.
pub(crate) const OPTIONS: &[(&str, OptionTy)] = &[
    (PATH, OptionTy::Str),
    (METHOD, OptionTy::Enum(METHOD_ENUM)),
    (NAME, OptionTy::Str),
];

/// One row of ADR 0077 § 5's table: a `#[Route]` that named both of the fields
/// a row cannot exist without, resolved to the strings the table is keyed by.
#[derive(Debug)]
pub(crate) struct Route {
    /// The `Core\Http\Method` case by its own name — `Get`, `Post`. Kept as
    /// the case rather than as ADR 0010 § 3's backing integer because every
    /// reader of a row is a diagnostic or a link, and neither has anything to
    /// say about the integer.
    verb: String,
    /// `path` exactly as written, captures and all.
    path: String,
    /// § 1's optional `name`, with the span that wrote it — the span, because
    /// a duplicate is reported at the field rather than at the attribute.
    name: Option<(String, Span)>,
    /// `Class::method` the attribute is attached to. For a message today, and
    /// for § 3's check over that method's parameter list once it lands.
    handler: String,
    /// The whole attribute.
    span: Span,
}

/// Every route the program declares, in the order they were walked — file by
/// file in `nvs_hir::resolve_program`'s entry-first load order, and by
/// declaration within a file.
///
/// A `Vec` rather than a map keyed by either of the two things a route is
/// looked up by, because *both* keys have a duplicate error attached to them:
/// a map would drop the row a collision is reported against, and § 2's
/// precedence makes the path key a shape rather than the written text. Order
/// is what makes the pair deterministic — a duplicate is always reported at
/// the row that arrives second, and the load order it arrives in does not
/// depend on filesystem enumeration ([ADR 0061](../../../../docs/adr/0061-compile-time-autoload-and-program-discovery.md)
/// § 3).
#[derive(Debug, Default)]
pub(crate) struct RouteTable {
    rows: Vec<Route>,
}

/// Every `#[Route]` `decl` declares, collected into `env`'s table.
///
/// A no-op — not even a lookup — for a class carrying no `#[Route]`, which is
/// what keeps § 5's "a program with no `#[Route]` anywhere pays nothing at all,
/// including no pass" true of this walk as well as of the scan.
///
/// Run from [`crate::check`]'s per-class walk rather than from
/// [`crate::attributes`]', for [`crate::commands::check_class_commands`]'
/// reason plus one of its own: a row is keyed by the class and method it is
/// attached to, and the per-attribute walk holds a payload with no declaration
/// around it.
pub(crate) fn check_class_routes(
    decl: &ClassDecl,
    class: &QName,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    for member in &decl.members {
        let ClassMemberKind::Method(m) = &member.kind else {
            continue;
        };
        let Some(attr) =
            crate::testing::attribute_named(&m.attributes, crate::derive::ROUTE, ctx, env)
        else {
            continue;
        };
        let attr = attr.clone();
        let handler = format!("{class}::{}", span_text(env.src, m.name));
        collect_route(&attr, handler, ctx, env);
    }
}

/// One `#[Route]` payload as a row, or the refusal that it is not one.
///
/// § 1's shape marks only `name` optional, and this is where that is enforced
/// rather than in [`crate::attributes::check_roster`]: the roster says what a
/// field may hold, and *required* is a fact about the row being built, which is
/// the reading [`crate::commands`]' own gap gives `#[Command]`'s `name` for the
/// same reason. Both missing fields are named in one diagnostic, because an
/// author who wrote neither wrote the empty attribute once.
fn collect_route(attr: &Attribute, handler: String, ctx: &Ctx<'_>, env: &mut Env<'_>) {
    let path = folded_str(attr, PATH, env);
    let verb = verb_of(attr, ctx, env);
    let (Some((path, _)), Some(verb)) = (path, verb) else {
        // A field written at the wrong type has already been reported by the
        // roster walk, and reporting it again as a missing one would name the
        // author's second problem before their first.
        if written(attr, PATH, env).is_some() && written(attr, METHOD, env).is_some() {
            return;
        }
        let missing = match (written(attr, PATH, env), written(attr, METHOD, env)) {
            (None, None) => "a `path` and a `method`",
            (None, _) => "a `path`",
            _ => "a `method`",
        };
        env.diags.report(
            Diagnostic::error(
                code::E_ROUTE_INCOMPLETE,
                format!("this `#[Route]` on `{handler}` gives no {missing}"),
            )
            .with_primary(attr.span, "not enough to build a route")
            .with_help(
                "a route is a path and a verb together, and only `name` is optional — write \
                 `#[Route(path: \"/users\", method: Core\\Http\\Method::Get)]`",
            ),
        );
        return;
    };
    let name = folded_str(attr, NAME, env);
    env.routes.rows.push(Route {
        verb,
        path,
        name,
        handler,
        span: attr.span,
    });
}

/// The field `option` as written, or `None` where the payload has no such
/// field. Says nothing about its value — [`collect_route`] uses it to tell a
/// field left out from one written at the wrong type.
fn written<'a>(
    attr: &'a Attribute,
    option: &str,
    env: &Env<'_>,
) -> Option<&'a nvs_syntax::ast::ObjectLiteralField> {
    attr.fields
        .iter()
        .find(|field| span_text(env.src, field.name) == option)
}

/// One `string` option's written value, folded — `None` where the field was
/// not written or where its value was not a `string`, the second of which
/// [`crate::attributes::check_roster`] has already reported.
fn folded_str(attr: &Attribute, option: &str, env: &mut Env<'_>) -> Option<(String, Span)> {
    let field = written(attr, option, env)?;
    let value = field.value.clone();
    let span = field.span;
    let declared = env.interner.intern(crate::ty::Ty::String);
    match crate::defaults::literal_default(&value, declared, env) {
        Some(crate::defaults::ConstArg::Str(text)) => Some((text, span)),
        _ => None,
    }
}

/// The `method:` case's own name — `Get` for `Core\Http\Method::Get`.
///
/// Read as a case rather than folded to its integer and mapped back, because
/// the case is what the row holds and the enum table answers both questions at
/// once: a value that is not a case of [`METHOD_ENUM`] is `None` here and has
/// already been reported by the roster walk, which places it at that enum.
fn verb_of(attr: &Attribute, ctx: &Ctx<'_>, env: &mut Env<'_>) -> Option<String> {
    let field = written(attr, METHOD, env)?;
    let ExprKind::ClassConstAccess { class, name } = &field.value.kind else {
        return None;
    };
    let class = class.clone();
    let case = span_text(env.src, *name).to_owned();
    let qname = crate::expr::resolve_class_expr(&class, ctx, env)?;
    if qname != QName::parse(METHOD_ENUM) {
        return None;
    }
    env.enums.case(&qname, &case)?;
    Some(case)
}

/// The two of § 1-§ 3's compile errors that are questions about the whole
/// enumeration: one route declared twice, and one `name` claimed twice.
///
/// Run once, after every file has been walked, because a route in the entry
/// file collides with one in a file § 5's scan found and neither declaration
/// can see the other. Both are reported at the row that arrives *second* in
/// load order, so which of two colliding declarations is named does not depend
/// on the filesystem.
pub(crate) fn check_table(table: &RouteTable, diags: &mut Diagnostics) {
    let mut routes: FxHashMap<(&str, String), &Route> = FxHashMap::default();
    let mut names: FxHashMap<&str, &Route> = FxHashMap::default();
    for row in &table.rows {
        if let Some(prior) = routes.insert((row.verb.as_str(), shape(&row.path)), row) {
            let (verb, path, handler) = (&row.verb, &row.path, &prior.handler);
            diags.report(
                Diagnostic::error(
                    code::E_DUPLICATE_ROUTE,
                    format!("`{verb} {path}` is already served by `{handler}`"),
                )
                .with_primary(row.span, "this route is declared twice")
                .with_help(
                    "§ 2 matches a path by shape, so two captures that differ only in name are \
                     one route — serve the second verb from its own `#[Route]`, or give the two \
                     paths different literal segments",
                ),
            );
            routes.insert((row.verb.as_str(), shape(&row.path)), prior);
        }
        let Some((name, span)) = &row.name else {
            continue;
        };
        if let Some(prior) = names.insert(name.as_str(), row) {
            let handler = &prior.handler;
            diags.report(
                Diagnostic::error(
                    code::E_DUPLICATE_ROUTE_NAME,
                    format!("the route name `{name}` is already `{handler}`'s"),
                )
                .with_primary(*span, "this name is claimed twice")
                .with_help(
                    "a name is what `Core\\Router::url` reverses the table by, so it names one \
                     route or it names nothing",
                ),
            );
            names.insert(name.as_str(), prior);
        }
    }
}

/// § 2's path *shape*: the path with every capture's name erased, so
/// `/users/{id}` and `/users/{userId}` are the one route they match as.
///
/// A capture is a whole segment or it is not a capture — that much of § 2's
/// grammar is decided here rather than deferred, because a duplicate reported
/// over the written text would miss the pair this error exists for. A segment
/// this rule leaves alone is a segment § 2's own grammar check, once it lands,
/// is what refuses.
fn shape(path: &str) -> String {
    path.split('/')
        .map(|segment| {
            if segment.starts_with('{') && segment.ends_with('}') && segment.len() >= 2 {
                "{}"
            } else {
                segment
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}
