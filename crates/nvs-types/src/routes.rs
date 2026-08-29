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
//! Between the two, and inside [`collect_route`], the path is read twice: once
//! on its own against § 2's grammar ([`parse_path`], which splits it into
//! [`Capture`]s and refuses the four ways it is not a path), and once against
//! the method the attribute is attached to ([`check_captures`], which asks § 3
//! whether each capture names a parameter and whether that parameter's declared
//! type is one a segment converts to). Both live here rather than in the
//! per-attribute walk for [`check_class_routes`]' own reason: a payload with no
//! declaration around it can see neither the parameter list nor the class.
//!
//! The conversion roster is [`crate::commands::converts_from_string`], read and
//! never copied — ADR 0086 § 6 takes § 3's list unchanged, so the two passes ask
//! one question. The single thing this pass adds to it is that a `{name...}`
//! arrives as the one `tainted string` § 3 says it does, so it binds a `string`
//! and nothing else.
//!
//! # Known gaps
//!
//! 1. **Nothing reverses the table yet.** The rows cross into `nvs-ir` on
//!    [`crate::expr_table::ExprTypeTable::routes`] — the channel every other
//!    whole-program fact travels to lowering by — but § 4's `Core\Router::url`
//!    does not read them, so a literal route name still resolves to nothing at
//!    compile time and throws at run time.
//! 2. **Only the first `#[Route]` on a method becomes a row**, because
//!    [`crate::testing::attribute_named`] answers with one attribute. ADR 0046
//!    § 3's repetition — one method serving two verbs — therefore contributes
//!    one row rather than two, and
//!    [ADR 0110](../../../../docs/adr/0110-one-methods-repeated-routes-share-a-name-when-they-share-a-path.md)
//!    § 1's exception, which lets those repetitions share a `name` when they
//!    share a `path`, has nothing yet to except.

use nvs_diagnostics::{Diagnostic, Diagnostics, Span, code};
use nvs_hir::QName;
use nvs_syntax::ast::{Attribute, ClassDecl, ClassMemberKind, ExprKind, MethodMember};
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
///
/// Public because the finished row is what crosses into `nvs-ir` — the same
/// arrangement [`crate::expr_table::Delegation`] has, and for its reason: what
/// rides across is a decision with no resolution left in it, so the consumer
/// holds strings rather than a second copy of this crate's tables.
#[derive(Debug)]
pub struct Route {
    /// The `Core\Http\Method` case by its own name — `Get`, `Post`. Kept as
    /// the case rather than as ADR 0010 § 3's backing integer because every
    /// reader of a row is a diagnostic or a link, and neither has anything to
    /// say about the integer.
    pub verb: String,
    /// `path` exactly as written, captures and all, and admitted by § 2's
    /// grammar — [`parse_path`] is what a reader splits it with rather than a
    /// second reading of the same string.
    pub path: String,
    /// § 1's optional `name`, with the span that wrote it — the span, because
    /// a duplicate is reported at the field rather than at the attribute.
    pub name: Option<(String, Span)>,
    /// `Class::method` the attribute is attached to, rendered as
    /// [`crate::expr_table::ExprTypeTable::method_label`] renders one.
    pub handler: String,
    /// The whole attribute.
    pub span: Span,
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
pub struct RouteTable {
    rows: Vec<Route>,
}

impl RouteTable {
    /// Every row, in load order.
    #[must_use]
    pub fn rows(&self) -> &[Route] {
        &self.rows
    }

    /// The row § 1's `name` names, or `None` where no route claims it — § 4's
    /// `url`/`urlAbsolute` reverse the table by exactly this question.
    ///
    /// The first match, which is the only one that can be reached: two rows
    /// claiming one name is [`code::E_DUPLICATE_ROUTE_NAME`], so a program in
    /// which this could be ambiguous does not compile.
    #[must_use]
    pub fn named(&self, name: &str) -> Option<&Route> {
        self.rows.iter().find(|row| {
            row.name
                .as_ref()
                .is_some_and(|(claimed, _)| claimed == name)
        })
    }
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
        collect_route(&attr, m, class, handler, ctx, env);
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
fn collect_route(
    attr: &Attribute,
    m: &MethodMember,
    class: &QName,
    handler: String,
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    let path = folded_str(attr, PATH, env);
    let verb = verb_of(attr, ctx, env);
    let (Some((path, path_span)), Some(verb)) = (path, verb) else {
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
    let captures = match parse_path(&path) {
        Ok(captures) => captures,
        Err(Refusal { what, help }) => {
            env.diags.report(
                Diagnostic::error(
                    code::E_ROUTE_PATH_GRAMMAR,
                    format!("`{handler}`'s path {what}"),
                )
                .with_primary(path_span, "the router cannot match this path")
                .with_help(help),
            );
            // The row is not pushed. A path with no grammar has no shape
            // either, so keeping it would report the author a duplicate of
            // something as their second problem before they have fixed their
            // first.
            return;
        }
    };
    check_captures(&captures, path_span, m, class, &handler, env);
    let name = folded_str(attr, NAME, env);
    env.routes.rows.push(Route {
        verb,
        path,
        name,
        handler,
        span: attr.span,
    });
}

/// § 2's three capture forms, each holding the name it binds. A segment that is
/// none of them is a literal, compared byte for byte and case-sensitively
/// ([ADR 0062](../../../../docs/adr/0062-case-sensitivity-is-a-compiler-property.md)),
/// and is not held here at all.
#[derive(Clone, Copy, Debug)]
enum Capture<'a> {
    /// `{name}` — one whole segment.
    One(&'a str),
    /// `{name?}` — one whole segment or none.
    Optional(&'a str),
    /// `{name...}` — every remaining segment as one `tainted string`.
    Rest(&'a str),
}

impl<'a> Capture<'a> {
    /// The parameter name this capture binds to, sigil-less, as § 3 compares
    /// it.
    fn name(self) -> &'a str {
        match self {
            Self::One(name) | Self::Optional(name) | Self::Rest(name) => name,
        }
    }

    /// The capture as an author wrote it, for a diagnostic that has to point
    /// at one of three forms and cannot point at a span inside a folded
    /// literal.
    fn written(self) -> String {
        match self {
            Self::One(name) => format!("{{{name}}}"),
            Self::Optional(name) => format!("{{{name}?}}"),
            Self::Rest(name) => format!("{{{name}...}}"),
        }
    }
}

/// A path § 2's grammar does not admit: the clause that completes "this
/// handler's path …", and the help that follows it.
struct Refusal {
    what: String,
    help: &'static str,
}

/// § 2's grammar over one whole path: the captures it declares, in order, or
/// the one way it is not a path.
///
/// Pure, and separate from the diagnostic it feeds, so the grammar reads as the
/// rules § 2 writes rather than through the reporting around them. It stops at
/// the first refusal for the reason [`collect_route`] drops the row: a path is
/// one thing, and a reader fixing its first fault re-reads the rest anyway.
///
/// § 2's "at most once" and "never in the same path as a `{name...}`" are not
/// checked separately, because both fall out of *last position only*: one
/// segment is last, so a second trailing form is already somewhere it is
/// refused.
fn parse_path(path: &str) -> Result<Vec<Capture<'_>>, Refusal> {
    if !path.starts_with('/') {
        return Err(Refusal {
            what: "does not begin with `/`".to_owned(),
            help: "a route is matched against a request's path, which always begins at the \
                   root, so a path that does not start with `/` could match nothing",
        });
    }
    let segments: Vec<&str> = path.split('/').collect();
    let last = segments.len() - 1;
    let mut captures: Vec<Capture<'_>> = Vec::new();
    for (index, &segment) in segments.iter().enumerate() {
        let Some(capture) = capture_of(segment)? else {
            continue;
        };
        if index != last && !matches!(capture, Capture::One(_)) {
            return Err(Refusal {
                what: format!(
                    "writes `{}` somewhere other than the last position",
                    capture.written()
                ),
                help: "a capture that may absorb the end of a path has nothing to follow it, \
                       so `{name?}` and `{name...}` are written last or not at all",
            });
        }
        if captures.iter().any(|prior| prior.name() == capture.name()) {
            return Err(Refusal {
                what: format!("captures `{}` twice", capture.name()),
                help: "a capture arrives as the parameter it is named after, so two of one \
                       name are two values for one parameter",
            });
        }
        captures.push(capture);
    }
    Ok(captures)
}

/// One segment read as § 2's grammar: the capture it declares, `None` for a
/// literal segment, or the clause saying what it is instead.
///
/// A segment carrying a brace anywhere is held to being a capture *whole*.
/// There is no escape and no partial form, which is the half of § 2 that keeps
/// `{` an ordinary byte in a literal segment impossible rather than ambiguous:
/// a path meaning one of two things is refused rather than repaired
/// ([ADR 0095](../../../../docs/adr/0095-ambiguous-input-is-refused-never-repaired.md)).
fn capture_of(segment: &str) -> Result<Option<Capture<'_>>, Refusal> {
    if !segment.contains('{') && !segment.contains('}') {
        return Ok(None);
    }
    let Some(inner) = segment
        .strip_prefix('{')
        .and_then(|rest| rest.strip_suffix('}'))
    else {
        return Err(Refusal {
            what: format!("has a segment `{segment}` that is neither a literal nor a capture"),
            help: "a capture is a whole segment — `/users/{id}`, never `/users/u{id}` or \
                   `/users/{id}.json`",
        });
    };
    let capture = if let Some(name) = inner.strip_suffix("...") {
        Capture::Rest(name)
    } else if let Some(name) = inner.strip_suffix('?') {
        Capture::Optional(name)
    } else {
        Capture::One(inner)
    };
    let name = capture.name();
    if name.is_empty()
        || name.starts_with(|c: char| c.is_ascii_digit())
        || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(Refusal {
            what: format!("writes `{segment}`, which captures no name a parameter could have"),
            help: "a capture names the parameter it binds to, so it holds an identifier and \
                   nothing else: `{id}`, `{page?}` or `{rest...}`",
        });
    }
    Ok(Some(capture))
}

/// § 3's two questions about the method the attribute is attached to — every
/// capture names a parameter of that name, and that parameter's declared type
/// is one a segment converts to — and § 2's one question of the same kind: a
/// `{name?}` is well-typed only where its parameter has a default.
///
/// The primary span of the two type-shaped refusals is the *parameter*, as it
/// is for [`crate::commands::check_convertible`]: the path is written correctly
/// and it is the declaration that cannot answer it. The unbound capture is the
/// other way round, and is reported at the `path:` field, because that is where
/// the name with no counterpart was written.
fn check_captures(
    captures: &[Capture<'_>],
    path_span: Span,
    m: &MethodMember,
    class: &QName,
    handler: &str,
    env: &mut Env<'_>,
) {
    // Copied out of `env` rather than read through it, so both stay readable
    // while a diagnostic is reported into the same `env`.
    let (src, signatures) = (env.src, env.signatures);
    let method = span_text(src, m.name).to_owned();
    let sig = signatures
        .get(class)
        .and_then(|class_sig| class_sig.methods.get(&method));
    for capture in captures {
        let name = capture.name();
        let Some((index, param)) = m
            .params
            .iter()
            .enumerate()
            .find(|(_, param)| crate::strip_sigil(span_text(src, param.name)) == name)
        else {
            env.diags.report(
                Diagnostic::error(
                    code::E_ROUTE_CAPTURE_UNBOUND,
                    format!(
                        "`{handler}` has no parameter `${name}` for `{}` to arrive as",
                        capture.written()
                    ),
                )
                .with_primary(path_span, "this capture names no parameter")
                .with_help(
                    "a capture is the parameter it is named after and the comparison is exact \
                     (ADR 0029) — the reverse is fine, and a parameter the path does not name \
                     is simply not the router's",
                ),
            );
            continue;
        };
        if matches!(capture, Capture::Optional(_)) && param.default.is_none() {
            env.diags.report(
                Diagnostic::error(
                    code::E_OPTIONAL_CAPTURE_NEEDS_DEFAULT,
                    format!(
                        "`${name}` has no default, so `{}` has nothing to be when it is absent",
                        capture.written()
                    ),
                )
                .with_primary(param.span, "this parameter needs a default")
                .with_secondary(path_span, "the capture that may match no segment")
                .with_help(
                    "an optional capture matches one whole segment or none, and the default is \
                     what makes the absent case well-typed rather than nullable by accident",
                ),
            );
        }
        let Some(ty) = sig.and_then(|sig| sig.params.get(index).copied()) else {
            continue;
        };
        let admitted = match capture {
            // The one row the shared roster does not answer for: nothing about
            // a catch-all is checked, so there is no conversion to choose and
            // it arrives as the `tainted string` it was read as.
            Capture::Rest(_) => matches!(
                env.interner.get(ty),
                crate::ty::Ty::String | crate::ty::Ty::TaintedString
            ),
            _ => crate::commands::converts_from_string(ty, env),
        };
        if admitted {
            continue;
        }
        let described = env.interner.describe(ty);
        let written = capture.written();
        env.diags.report(
            Diagnostic::error(
                code::E_ROUTE_CAPTURE_TYPE_HAS_NO_CONVERSION,
                format!("`{described}` is not a type `{written}` can arrive at"),
            )
            .with_primary(param.span, "no conversion from a path segment")
            .with_secondary(path_span, "the capture bound here")
            .with_help(match capture {
                Capture::Rest(_) => {
                    "a `{name...}` is every remaining segment as one value and nothing about it \
                     was checked, so it arrives as a `string` and at no other type"
                }
                _ => {
                    "a segment is converted to the parameter's declared type, so a capture \
                     binds `string`, `int`, `uint`, `decimal`, an enum, a union of literal \
                     types, or `Core\\Uuid` — and never a regex (ADR 0102 § 5)"
                }
            }),
        );
    }
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

/// § 2's path *shape*: the path with every capture's name erased but its
/// *form* kept, so `/users/{id}` and `/users/{userId}` are the one route they
/// match as while `/posts/{page}` and `/posts/{page?}` stay two.
///
/// The form is kept because § 2's precedence is structural — a literal beats a
/// `{name}`, which beats a `{name?}`, which beats a `{name...}` — so the three
/// are three nodes of the trie and a pair of them has an answer that does not
/// depend on declaration order. Erasing the form would report that pair as the
/// duplicate it is not.
///
/// Only a row [`parse_path`] admitted reaches here, so a segment that is not a
/// capture is a literal and there is no third case to leave alone.
fn shape(path: &str) -> String {
    path.split('/')
        .map(|segment| match capture_of(segment) {
            Ok(Some(Capture::One(_))) => "{}",
            Ok(Some(Capture::Optional(_))) => "{?}",
            Ok(Some(Capture::Rest(_))) => "{...}",
            _ => segment,
        })
        .collect::<Vec<_>>()
        .join("/")
}
