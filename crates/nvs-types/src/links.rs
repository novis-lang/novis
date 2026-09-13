//! `rule:routing/link-name-and-params-are-checked`'s link:
//! `Core\Router::url`, `::urlAbsolute` and `::urlSigned` over a **literal**
//! route name, resolved against § 5's finished table while compiling.
//!
//! Two of the four refusals here are that ADR's — an unknown name, and a
//! capture no key supplies. The other two are
//! `rule:routing/the-servers-match-dispatches-nothing`'s,
//! and they are the two halves of one `$params` entry. § 6's is about the
//! **key**, and it exists because that section gives every *other* key a
//! meaning: a key covering no capture becomes the link's query string, so a key
//! covering nothing at all had to stop being one ([`declared`]). § 5's is about
//! the **value**, and it exists because that section narrows a capture to a
//! closed set: a value outside it is a link to a path the router answers `404`
//! to ([`within_set`]).
//!
//! # Why this is two passes and not one
//!
//! The sibling of [`crate::retrieval`] and [`crate::program`] — a `Core` call
//! whose answer is known before the program starts, so the call is replaced
//! with it — and it differs from both in exactly one way, which is the whole
//! shape of this module: **it cannot be answered where it is written.** A
//! retrieval asks about a declaration the attribute table already holds whole,
//! and an enumeration asks about a class graph that is complete before the walk
//! begins. A link asks about the *route table*, which is filled by the same
//! walk: a `url` call in the entry file routinely names a route § 5's scan
//! finds in a file the walk reaches later, so a lookup made at the call site
//! would answer "no such route" for a program that has one.
//!
//! So [`record_site`] only writes down what it saw — the call's span, the
//! folded name, and `$params`' literal keys — and [`resolve`] makes every
//! lookup afterwards, from [`crate::check::check_program`], beside
//! [`crate::routes::check_table`] and for its reason.
//!
//! # What a site records over, and what it leaves alone
//!
//! [`crate::expr::calls`] has already recorded [`ExprInfo::Call`] against the
//! same span. [`resolve`] records [`ExprInfo::RouteLink`] over it, which wins
//! the lookup ([`crate::expr_table::ExprTypeTable::record`]), so the two states
//! a consumer can see are exactly § 4's two: a resolved link, or an ordinary
//! call to a member whose body throws. That is why a **computed** name needs no
//! handling here at all — nothing is recorded, the `Call` stands, and
//! `nvs_stdlib::router`'s own body is § 4's "a computed `$name` throws".
//!
//! # Known gaps
//!
//! 1. **A named argument is not folded.** `Core\Router::url(name: "…")` is
//!    legal and records no site, so it throws at run time as a computed name
//!    would. Reading one needs the slot mapping `check_args_typed` already
//!    built and this pass is not handed.
//!    Decided: Hand these passes the slot mapping check_args_typed already builds — Named arguments are
//!    checked like positional ones, and three passes take a new input.
//!    — owner: unowned-closures
//! 2. **An enum-case capture has no closed set to check against**, so [`within_set`]
//!    passes every value written for one. `crate::routes::closed_set` owns why:
//!    what segment text arrives at a case is `Core\Router::match`'s decision, and
//!    that member lands with the rest of the request-facing half, so the set has
//!    no spelling to compare with yet rather than being one this pass declines to
//!    read.
//!    — owner: M7

use nvs_diagnostics::{Diagnostic, Diagnostics, Span, code};
use nvs_hir::QName;
use nvs_syntax::ast::{CallArgs, Expr, ExprKind};

use crate::Env;
use crate::defaults::ConstArg;
use crate::expr_table::{ExprInfo, ExprTypeTable, UrlPiece};
use crate::routes::RouteTable;

/// The one class this pass answers for.
const OWNER: &str = r"Core\Router";

/// `urlAbsolute`, written once: [`is_link`] admits it and [`resolve`] reads the
/// origin flag back off the same spelling.
const ABSOLUTE: &str = "urlAbsolute";

/// `urlSigned`, on the same terms as [`ABSOLUTE`] — one spelling that both
/// admits the member and says what the fold records for it.
///
/// It asks the table exactly what the other two ask, so every refusal below is
/// its refusal too: an unknown name, a capture no key supplies, a key that
/// covers nothing, a value outside a closed set. What it adds is downstream of
/// the lookup — the resolved name travels to run time, because
/// `rule:core-classes/router-signed-url` signs the route's identity rather than
/// the path it renders to.
const SIGNED: &str = "urlSigned";

/// One [`is_link`] call whose name folded to a literal, waiting for the table
/// to finish.
///
/// Holds strings rather than borrowing the AST because the walk that recorded
/// it has moved on to the next file by the time [`resolve`] runs, and the
/// `Env` it borrowed through does not outlive that file.
pub(crate) struct LinkSite {
    /// The call's own span — the key [`ExprInfo::RouteLink`] is recorded
    /// against, so it lands on the entry [`crate::expr::calls`] already wrote.
    span: Span,
    /// The member as written, which is both the word a diagnostic quotes and
    /// whether an origin is prepended.
    member: String,
    /// Argument 0, folded. A site exists only because it folded.
    name: String,
    /// Where argument 0 was written, which is where an unknown name is
    /// reported: the call's span underlines `$params` too, and the mistake is
    /// in the name.
    name_span: Span,
    /// Argument 1's literal string keys, in written order, or `None` where it
    /// is not an array literal — in which case there is nothing to check
    /// coverage against and § 4 asks for nothing.
    args: Option<Vec<LinkArg>>,
}

/// One entry of a link's `$params` whose key folded to a literal.
struct LinkArg {
    /// The key as written, which is what a capture and a `#[Query]` name are
    /// both compared against.
    key: String,
    /// The value as the segment text it would be substituted as, where it
    /// folded to one — a `string` or an `int` literal, which are the two
    /// spellings `rule:routing/a-capture-narrows-to-a-closed-set`'s closed sets are written in. `None` for
    /// everything else, and a `None` is checked against nothing.
    value: Option<String>,
}

/// Whether `owner::member` is one of § 4's link builders — the same nominal
/// test [`crate::retrieval::is_retrieval`] makes, against a resolved [`QName`]
/// rather than against what the call site spelled.
pub(crate) fn is_link(owner: &QName, member: &str) -> bool {
    owner.to_string() == OWNER && matches!(member, "url" | ABSOLUTE | SIGNED)
}

/// Records one link site, or nothing where § 4 leaves the call to run time.
///
/// Reports nothing: every refusal this pass makes is a question about the table
/// and is made in [`resolve`], and everything decided here — a computed name, a
/// named argument, a `$params` that is not a literal — is legal.
pub(crate) fn record_site(call: &Expr, member: &str, args: &CallArgs, env: &mut Env<'_>) {
    let CallArgs::List(list) = args else {
        return;
    };
    // Gap 1: the positions below are the written ones, so a named or spread
    // argument anywhere is left to run time rather than read out of order.
    if list.iter().any(|arg| arg.name.is_some() || arg.spread) {
        return;
    }
    let Some(name_arg) = list.first().map(|arg| &arg.value) else {
        // A missing argument is the arity check's refusal, already made.
        return;
    };
    let Some(ConstArg::Str(name)) = folded_as(name_arg, crate::ty::Ty::String, env) else {
        return;
    };
    let args = list.get(1).and_then(|arg| literal_args(&arg.value, env));
    env.links.push(LinkSite {
        span: call.span,
        member: member.to_owned(),
        name,
        name_span: name_arg.span,
        args,
    });
}

/// `$params`' entries where it is an array literal every key of which is a
/// string literal, and `None` otherwise.
///
/// A keyless element contributes no key rather than aborting the read: it is an
/// integer key, which covers no capture, so the coverage check below is exactly
/// right to report it as a capture with nothing supplying it.
///
/// A **value** that does not fold is kept rather than dropped, and it is why
/// this reads entries and not keys: the key is still a key § 6 has a question
/// about, and only the § 5 question below is the one a computed value has no
/// answer to.
fn literal_args(expr: &Expr, env: &mut Env<'_>) -> Option<Vec<LinkArg>> {
    let ExprKind::ArrayLiteral(items) = &expr.kind else {
        return None;
    };
    let mut args = Vec::with_capacity(items.len());
    for item in items {
        if item.spread {
            // A spread hides keys, so the array as written no longer says what
            // it covers and nothing can be concluded from it.
            return None;
        }
        let Some(key) = &item.key else {
            continue;
        };
        let Some(ConstArg::Str(text)) = folded_as(key, crate::ty::Ty::String, env) else {
            return None;
        };
        args.push(LinkArg {
            key: text,
            value: segment_text(&item.value, env),
        });
    }
    Some(args)
}

/// The segment text one `$params` value would be substituted as, where it is a
/// literal — `nvs_stdlib::router`'s `segment_text` reaching the same two tags
/// through `value_to_string`, which is why an `int` is its decimal spelling
/// here as it is there.
fn segment_text(expr: &Expr, env: &mut Env<'_>) -> Option<String> {
    match folded_as(expr, crate::ty::Ty::String, env) {
        Some(ConstArg::Str(text)) => Some(text),
        _ => match folded_as(expr, crate::ty::Ty::Int, env) {
            Some(ConstArg::Int(value)) => Some(value.to_string()),
            _ => None,
        },
    }
}

/// One expression folded at `ty`, through the one literal decoder — see
/// [`crate::routes::folded_str`], which reads an attribute field the same way.
/// The type drives the decoding rather than the literal's own shape, which is
/// [`crate::defaults::literal_default`]'s rule and the reason a value is asked
/// twice above rather than once.
fn folded_as(expr: &Expr, ty: crate::ty::Ty, env: &mut Env<'_>) -> Option<ConstArg> {
    let declared = env.interner.intern(ty);
    crate::defaults::literal_default(expr, declared, env)
}

/// § 4's two compile errors and `rule:routing/a-leftover-link-key-is-a-query-string`'s one, and the fold that follows
/// when none of them applies.
///
/// Run once, after every file has been walked, so `routes` is the whole
/// program's table and a name declared in any file resolves from any other.
pub(crate) fn resolve(
    routes: &RouteTable,
    sites: &[LinkSite],
    exprs: &mut ExprTypeTable,
    diags: &mut Diagnostics,
) {
    for site in sites {
        let Some(row) = routes.named(&site.name) else {
            diags.report(
                Diagnostic::error(
                    code::E_UNKNOWN_ROUTE_NAME,
                    format!(
                        "`Core\\Router::{}` names the route `{}`, and no `#[Route]` claims it",
                        site.member, site.name
                    ),
                )
                .with_primary(site.name_span, "no route is named this")
                .with_help(
                    "`rule:routing/link-name-and-params-are-checked`: a literal name is checked against the compiled route \
                     table — give the route a `name:` field of exactly this spelling, or \
                     correct the name written here",
                ),
            );
            continue;
        };
        let Some(pieces) = crate::routes::link_pieces(&row.path) else {
            // Unreachable: a row exists only for a path § 2's grammar admitted.
            continue;
        };
        // Short-circuiting on purpose: a misspelled key is usually both a
        // capture with nothing supplying it and a key with nothing to be, and
        // naming the capture is the half that says what to write. § 5's
        // question about a *value* comes last for the same reason — it is only
        // a question once the key it is written under names something.
        if let Some(args) = &site.args
            && !(covered(&pieces, args, site, row, diags)
                && declared(&pieces, args, site, row, diags)
                && within_set(args, site, row, diags))
        {
            continue;
        }
        exprs.record(
            site.span,
            ExprInfo::RouteLink {
                pieces,
                absolute: site.member == ABSOLUTE,
                // The name the lookup succeeded on, not the one the call
                // spelled: they are the same string, and taking it from the
                // site is what makes "the signature is over what resolved"
                // true by construction rather than by a second comparison.
                signed: (site.member == SIGNED).then(|| site.name.clone()),
            },
        );
    }
}

/// `rule:routing/a-leftover-link-key-is-a-query-string`'s compile error, which is [`covered`] read the other way round:
/// every key supplies *something*.
///
/// A key names one of the path's captures — any of the three forms, including
/// the `{name?}` [`covered`] does not require — or one of the route's declared
/// `#[Query]` parameters ([`crate::routes::Route::query`]). A key that is
/// neither is not inert: § 6 turns every non-capture key into the link's query
/// string, so a typo ships as `?typo=…` rather than being dropped, and the
/// rule is what makes that turn safe to make at all.
///
/// The comparison is exact and case-sensitive
/// (`rule:classes/names-resolve-case-sensitively`),
/// as the capture-to-parameter one in [`crate::routes`] is.
fn declared(
    pieces: &[UrlPiece],
    args: &[LinkArg],
    site: &LinkSite,
    row: &crate::routes::Route,
    diags: &mut Diagnostics,
) -> bool {
    let unknown: Vec<&str> =
        args.iter()
            .map(|arg| arg.key.as_str())
            .filter(|key| {
                !pieces.iter().any(|piece| match piece {
                    UrlPiece::Required(name) | UrlPiece::Optional(name) | UrlPiece::Rest(name) => {
                        name == key
                    }
                    UrlPiece::Literal(_) => false,
                }) && !row.params.iter().any(|param| {
                    param.source == crate::routes::ParamIn::Query && param.name == *key
                })
            })
            .collect();
    if unknown.is_empty() {
        return true;
    }
    let named = unknown
        .iter()
        .map(|key| format!("`{key}`"))
        .collect::<Vec<_>>()
        .join(", ");
    diags.report(
        Diagnostic::error(
            code::E_ROUTE_LINK_UNKNOWN_PARAM,
            format!(
                "`{}`'s path `{}` has no {named}, and `{}` declares no `#[Query]` parameter of \
                 that name",
                row.handler, row.path, row.handler
            ),
        )
        .with_primary(site.span, "this key would become a query string")
        .with_help(
            "`rule:routing/a-leftover-link-key-is-a-query-string`: a key that is not a capture becomes the link's query string, so one \
             that names nothing ships as a query parameter nobody reads — correct the spelling, \
             or declare the parameter with `#[Query]` on the handler",
        ),
    );
    false
}

/// § 4's second compile error: every capture that must be substituted has a key
/// supplying it.
///
/// A `{name?}` is the one form left out — its whole segment is dropped where
/// the key is absent, which is what makes it optional at all — so what is
/// required is exactly [`UrlPiece::Required`] and [`UrlPiece::Rest`].
fn covered(
    pieces: &[UrlPiece],
    args: &[LinkArg],
    site: &LinkSite,
    row: &crate::routes::Route,
    diags: &mut Diagnostics,
) -> bool {
    let missing: Vec<&str> = pieces
        .iter()
        .filter_map(|piece| match piece {
            UrlPiece::Required(name) | UrlPiece::Rest(name) => Some(name.as_str()),
            UrlPiece::Literal(_) | UrlPiece::Optional(_) => None,
        })
        .filter(|name| !args.iter().any(|arg| arg.key == *name))
        .collect();
    if missing.is_empty() {
        return true;
    }
    let named = missing
        .iter()
        .map(|name| format!("`{name}`"))
        .collect::<Vec<_>>()
        .join(", ");
    diags.report(
        Diagnostic::error(
            code::E_ROUTE_LINK_MISSING_PARAM,
            format!(
                "`{}`'s path `{}` captures {named}, and this `$params` supplies {}",
                row.handler,
                row.path,
                if args.is_empty() {
                    "no keys at all".to_owned()
                } else {
                    format!(
                        "only {}",
                        args.iter()
                            .map(|arg| arg.key.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            ),
        )
        .with_primary(site.span, "this link cannot be built")
        .with_help(
            "`rule:routing/link-name-and-params-are-checked`: a `$params` array that does not cover the route's captures is a \
             compile error — only a `{name?}` may be left out, because its whole segment is \
             dropped when it is",
        ),
    );
    false
}

/// `rule:routing/a-capture-narrows-to-a-closed-set`'s compile error: a literal value the parameter it supplies
/// cannot hold.
///
/// § 5 narrows a capture to a closed set with a *type*, and a segment outside
/// that set fails the conversion and falls through to a `404` — so a link built
/// out of one is a link to a route that will not match, and the table can say
/// so before the program runs. That is the same laundering argument § 4 makes
/// about the path: a value is checked where it is written, because the sink it
/// reaches cannot check it.
///
/// **A `#[Query]` parameter's set is checked too**, and by the same walk, since
/// § 3 gives a query value the same type list a capture has: a value outside it
/// is the handler's own `400` rather than a `404`, which is a different answer
/// to the same dead link. What is *not* checked is a value that did not fold
/// ([`LinkArg::value`]) and a parameter whose type names no set at all
/// ([`crate::routes::RouteParam::allowed`]) — in both cases there is nothing to
/// compare, exactly as [`declared`] reads only literal keys.
///
/// The **first** offending value is reported and the walk stops, as the pair
/// above stops: one refusal per call, because a `$params` written against the
/// wrong route is usually wrong in every entry at once and a list of them says
/// nothing the first does not.
fn within_set(
    args: &[LinkArg],
    site: &LinkSite,
    row: &crate::routes::Route,
    diags: &mut Diagnostics,
) -> bool {
    for arg in args {
        let (Some(value), Some(param)) = (
            arg.value.as_deref(),
            row.params.iter().find(|param| param.name == arg.key),
        ) else {
            continue;
        };
        let Some(allowed) = &param.allowed else {
            continue;
        };
        if allowed.iter().any(|admitted| admitted == value) {
            continue;
        }
        let named = allowed
            .iter()
            .map(|admitted| format!("`{admitted}`"))
            .collect::<Vec<_>>()
            .join(", ");
        diags.report(
            Diagnostic::error(
                code::E_ROUTE_LINK_VALUE_NOT_IN_SET,
                format!(
                    "`{}` admits {named} for `{}`, and this link supplies `{value}`",
                    row.handler, arg.key
                ),
            )
            .with_primary(site.span, "no request could carry this value")
            .with_help(
                "`rule:routing/a-capture-narrows-to-a-closed-set`: a capture narrows to a closed set with a type, and a segment \
                 outside it falls through to a `404` rather than reaching the handler — so this \
                 link names a route it would not match. Correct the value, or widen the \
                 parameter's declared type",
            ),
        );
        return false;
    }
    true
}
