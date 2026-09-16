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
//! **A `name:` argument is folded like a positional one**, because which
//! parameter each written argument fills arrives from
//! [`crate::expr::args::check_args_typed`]: `Core\Router::url(name: "…")` is
//! read where it was written and an unknown route named that way is `E0754`
//! before the program runs (`rule:core-api/parameters-are-callable-by-name`).
//! So the two states above stay the only two, and a link that throws at run
//! time is a computed name and nothing else.

use nvs_diagnostics::{Diagnostic, Diagnostics, Span, code};
use nvs_hir::QName;
use nvs_syntax::ast::{CallArgs, Expr, ExprKind};

use crate::defaults::ConstArg;
use crate::expr::args::argument_filling;
use crate::expr_table::{ArgSlot, EnumSpelling, ExprInfo, ExprTypeTable, UrlPiece};
use crate::routes::RouteTable;
use crate::{Ctx, Env};

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
    /// The enum case written here, where the value is one — the entry with
    /// **two** spellings rather than one, and so the entry that cannot be
    /// folded to a segment until the parameter it supplies says which
    /// (`rule:routing/an-enum-capture-is-spelled-by-its-backing-value-or-its-case-name`).
    case: Option<CaseArg>,
}

/// An enum case written as a `$params` value: the case, and both texts the rule
/// above could spell it as.
struct CaseArg {
    /// The enum's declared name, fully qualified. Quoted in the refusal, so it
    /// names the case the call wrote rather than the integer it folds to.
    class: String,
    /// The case's own name, which is the segment a **name**-spelled subset
    /// admits.
    name: String,
    /// The case's backing value in decimal, which is the segment a
    /// **value**-spelled subset admits — and the text the value arrives as at
    /// run time under either, since a case is indistinguishable from its
    /// integer by then (`rule:enums/no-class-machinery`).
    value: String,
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
/// `$params` that is not a literal — is legal.
pub(crate) fn record_site(
    call: &Expr,
    member: &str,
    args: &CallArgs,
    slots: &[ArgSlot],
    ctx: &Ctx<'_>,
    env: &mut Env<'_>,
) {
    let CallArgs::List(list) = args else {
        return;
    };
    // Each argument is found through the parameter it fills, so a `name:` is
    // read where it was written. Nothing fills `$name` in a call too short to
    // have written it — the arity check's refusal, already made — or in one
    // whose `...` has no variadic tail to land in, which is refused where it
    // was mapped.
    let Some(name_arg) = argument_filling(0, list, slots).map(|arg| &arg.value) else {
        return;
    };
    let Some(ConstArg::Str(name)) = folded_as(name_arg, crate::ty::Ty::String, env) else {
        return;
    };
    let args = argument_filling(1, list, slots).and_then(|arg| literal_args(&arg.value, ctx, env));
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
fn literal_args(expr: &Expr, ctx: &Ctx<'_>, env: &mut Env<'_>) -> Option<Vec<LinkArg>> {
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
            case: case_arg(&item.value, ctx, env),
        });
    }
    Some(args)
}

/// The segment text one `$params` value would be substituted as, where it is a
/// literal — `nvs_stdlib::router`'s `segment_text` reaching the same two tags
/// through `value_to_string`, which is why an `int` is its decimal spelling
/// here as it is there.
///
/// An enum case is [`case_arg`]'s and not this decoder's. It folds at neither
/// of the two types asked here, and it could not be read at one anyway: the
/// segment it builds is its backing value under one spelling and its case name
/// under the other, and nothing at the call site says which.
fn segment_text(expr: &Expr, env: &mut Env<'_>) -> Option<String> {
    match folded_as(expr, crate::ty::Ty::String, env) {
        Some(ConstArg::Str(text)) => Some(text),
        _ => match folded_as(expr, crate::ty::Ty::Int, env) {
            Some(ConstArg::Int(value)) => Some(value.to_string()),
            _ => None,
        },
    }
}

/// The enum case one `$params` value names, where it names one.
///
/// A **written** case and nothing else: a variable holding one is a computed
/// value, which this pass reads as little as it reads a computed string. The
/// fold is [`crate::defaults::const_reference_default`]'s enum arm with no
/// declared type to place against, because the position gives it none —
/// `$params` is `array<string, mixed>`, so what decides the spelling is the
/// parameter the key supplies and never the slot the case was written in.
fn case_arg(expr: &Expr, ctx: &Ctx<'_>, env: &Env<'_>) -> Option<CaseArg> {
    let ExprKind::ClassConstAccess { class, name } = &expr.kind else {
        return None;
    };
    let qname = crate::expr::resolve_class_expr(class, ctx, env)?;
    let case = crate::span_text(env.src, *name).to_owned();
    let value = env.enums.case(&qname, &case)?;
    Some(CaseArg {
        class: qname.to_string(),
        name: case,
        value: decimal(value),
    })
}

/// One case's backing value as the decimal a `$params` entry carries it as —
/// [`crate::enums::EnumValue`]'s two arms, which print the same way and are
/// kept apart only by what they can hold.
fn decimal(value: crate::enums::EnumValue) -> String {
    match value {
        crate::enums::EnumValue::Int(number) => number.to_string(),
        crate::enums::EnumValue::Uint(number) => number.to_string(),
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
                spellings: spellings(&pieces, row),
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

/// Every capture of `pieces` whose parameter narrows to a **name**-spelled enum
/// subset, as the table `nvs_stdlib::router`'s `substitute` converts the
/// arriving value with.
///
/// A value-spelled subset contributes nothing, and that is why this is a table
/// and not a flag: a case reaches run time as its backing integer, whose
/// decimal is already the segment such a subset admits, so only the name-spelled
/// half has a conversion left to make
/// (`rule:routing/an-enum-capture-is-spelled-by-its-backing-value-or-its-case-name`).
/// A `#[Query]` parameter contributes nothing either — its value is not a piece,
/// and a key the path does not consume becomes
/// `rule:routing/a-leftover-link-key-is-a-query-string`'s query string.
fn spellings(pieces: &[UrlPiece], row: &crate::routes::Route) -> Vec<EnumSpelling> {
    pieces
        .iter()
        .filter_map(|piece| match piece {
            UrlPiece::Required(name) | UrlPiece::Optional(name) | UrlPiece::Rest(name) => {
                Some(name)
            }
            UrlPiece::Literal(_) => None,
        })
        .filter_map(|name| {
            let param = row.params.iter().find(|param| {
                param.source == crate::routes::ParamIn::Path && param.name == *name
            })?;
            let narrowed = param.cases.as_ref()?;
            (!narrowed.by_value).then(|| EnumSpelling {
                capture: name.clone(),
                cases: narrowed
                    .cases
                    .iter()
                    .map(|(spelling, value)| (decimal(*value), spelling.clone()))
                    .collect(),
            })
        })
        .collect()
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
/// ([`LinkArg::value`], [`LinkArg::case`]) and a parameter whose type names no
/// set at all ([`crate::routes::RouteParam::admits`]) — in both cases there is
/// nothing to compare, exactly as [`declared`] reads only literal keys.
///
/// **An enum subset is one of the sets**, and the entry against it is read
/// under the subset's own spelling: a case written for a value-spelled subset
/// is its backing value and one written for a name-spelled subset is its name,
/// which is [`supplied`]'s whole job. So a link into either half is checked
/// against exactly the segments the match would claim, and the one written
/// `Lang::Fr` for a subset admitting neither spelling of it is a link to a
/// `404`.
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
        let Some(param) = row.params.iter().find(|param| param.name == arg.key) else {
            continue;
        };
        let (Some(set), Some((segment, written))) = (param.admits(), supplied(arg, param)) else {
            continue;
        };
        if set.contains(&segment) {
            continue;
        }
        let named = set
            .iter()
            .map(|spelling| format!("`{spelling}`"))
            .collect::<Vec<_>>()
            .join(", ");
        diags.report(
            Diagnostic::error(
                code::E_ROUTE_LINK_VALUE_NOT_IN_SET,
                format!(
                    "`{}` admits {named} for `{}`, and this link supplies `{written}`",
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

/// The segment one entry would build, and the text a refusal quotes it as.
///
/// Two answers rather than one because an enum case is written one way and
/// substituted another: the comparison is against the segment, and the message
/// names `Lang::Fr` rather than the integer nobody wrote. The parameter decides
/// which segment a case is — its written backing value under a value-spelled
/// subset, its name under a name-spelled one, and, for a parameter that narrows
/// to no enum at all, the integer it folds to, which is what both `substitute`
/// and the match would make of it.
fn supplied<'a>(arg: &'a LinkArg, param: &crate::routes::RouteParam) -> Option<(&'a str, String)> {
    if let Some(case) = &arg.case {
        let segment = match &param.cases {
            Some(narrowed) if !narrowed.by_value => case.name.as_str(),
            _ => case.value.as_str(),
        };
        return Some((segment, format!("{}::{}", case.class, case.name)));
    }
    let value = arg.value.as_deref()?;
    Some((value, value.to_owned()))
}
