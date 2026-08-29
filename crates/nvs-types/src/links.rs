//! [ADR 0077](../../../docs/adr/0077-compile-time-routing.md) § 4's link half:
//! `Core\Router::url` and `::urlAbsolute` over a **literal** route name,
//! resolved against § 5's finished table while compiling.
//!
//! Two of the three refusals here are that ADR's — an unknown name, and a
//! capture no key supplies. The third is
//! [ADR 0102](../../../docs/adr/0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md)
//! § 6's, and it exists because that section gives every *other* key a meaning:
//! a key covering no capture becomes the link's query string, so a key covering
//! nothing at all had to stop being one ([`declared`]).
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

/// One `Core\Router::url`/`::urlAbsolute` call whose name folded to a literal,
/// waiting for the table to finish.
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
    keys: Option<Vec<String>>,
}

/// Whether `owner::member` is one of § 4's two link builders — the same nominal
/// test [`crate::retrieval::is_retrieval`] makes, against a resolved [`QName`]
/// rather than against what the call site spelled.
pub(crate) fn is_link(owner: &QName, member: &str) -> bool {
    owner.to_string() == OWNER && matches!(member, "url" | ABSOLUTE)
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
    let Some(ConstArg::Str(name)) = folded_str(name_arg, env) else {
        return;
    };
    let keys = list.get(1).and_then(|arg| literal_keys(&arg.value, env));
    env.links.push(LinkSite {
        span: call.span,
        member: member.to_owned(),
        name,
        name_span: name_arg.span,
        keys,
    });
}

/// `$params`' keys where it is an array literal every key of which is a string
/// literal, and `None` otherwise.
///
/// A keyless element contributes no key rather than aborting the read: it is an
/// integer key, which covers no capture, so the coverage check below is exactly
/// right to report it as a capture with nothing supplying it.
fn literal_keys(expr: &Expr, env: &mut Env<'_>) -> Option<Vec<String>> {
    let ExprKind::ArrayLiteral(items) = &expr.kind else {
        return None;
    };
    let mut keys = Vec::with_capacity(items.len());
    for item in items {
        if item.spread {
            // A spread hides keys, so the array as written no longer says what
            // it covers and nothing can be concluded from it.
            return None;
        }
        let Some(key) = &item.key else {
            continue;
        };
        let Some(ConstArg::Str(text)) = folded_str(key, env) else {
            return None;
        };
        keys.push(text);
    }
    Some(keys)
}

/// One expression folded as a `string`, through the one literal decoder — see
/// [`crate::routes::folded_str`], which reads an attribute field the same way.
fn folded_str(expr: &Expr, env: &mut Env<'_>) -> Option<ConstArg> {
    let declared = env.interner.intern(crate::ty::Ty::String);
    crate::defaults::literal_default(expr, declared, env)
}

/// § 4's two compile errors and ADR 0102 § 6's one, and the fold that follows
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
                    "ADR 0077 § 4: a literal name is checked against the compiled route \
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
        // naming the capture is the half that says what to write.
        if let Some(keys) = &site.keys
            && !(covered(&pieces, keys, site, row, diags)
                && declared(&pieces, keys, site, row, diags))
        {
            continue;
        }
        exprs.record(
            site.span,
            ExprInfo::RouteLink {
                pieces,
                absolute: site.member == ABSOLUTE,
            },
        );
    }
}

/// ADR 0102 § 6's compile error, which is [`covered`] read the other way round:
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
/// ([ADR 0062](../../../docs/adr/0062-case-sensitivity-is-a-compiler-property.md)),
/// as the capture-to-parameter one in [`crate::routes`] is.
fn declared(
    pieces: &[UrlPiece],
    keys: &[String],
    site: &LinkSite,
    row: &crate::routes::Route,
    diags: &mut Diagnostics,
) -> bool {
    let unknown: Vec<&str> =
        keys.iter()
            .map(String::as_str)
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
            "ADR 0102 § 6: a key that is not a capture becomes the link's query string, so one \
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
    keys: &[String],
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
        .filter(|name| !keys.iter().any(|key| key == name))
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
                if keys.is_empty() {
                    "no keys at all".to_owned()
                } else {
                    format!("only {}", keys.join(", "))
                }
            ),
        )
        .with_primary(site.span, "this link cannot be built")
        .with_help(
            "ADR 0077 § 4: a `$params` array that does not cover the route's captures is a \
             compile error — only a `{name?}` may be left out, because its whole segment is \
             dropped when it is",
        ),
    );
    false
}
