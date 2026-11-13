//! `rule:routing/link-target-and-params-are-checked`'s link:
//! `Core\Router::url`, `::urlAbsolute` and `::urlSigned` over a **written
//! method reference**, `Shop::show`, resolved against the finished route table
//! while compiling. A route is named by the method it is declared on, so the
//! table is reversed by [`crate::routes::Route::handler`].
//!
//! Two of the four refusals here are that rule's — a method with no route, and
//! a capture no key supplies. The other two are the two halves of one
//! `$params` entry. `rule:routing/a-leftover-link-key-is-a-query-string`'s is
//! about the **key**, and it exists because that rule gives every *other* key a
//! meaning: a key covering no capture becomes the link's query string, so a key
//! covering nothing at all had to stop being one ([`declared`]).
//! `rule:routing/a-capture-narrows-to-a-closed-set`'s is about the **value**, and
//! it exists because that rule narrows a capture to a closed set: a value
//! outside it is a link to a path the router answers `404` to ([`within_set`]).
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
//! walk: a `url` call in the entry file routinely names a route the program
//! enumeration's scan (`rule:routing/table-is-opt-in`) finds in a file the walk
//! reaches later, so a lookup made at the call site
//! would answer "no such route" for a program that has one.
//!
//! So [`record_site`] only writes down what it saw — the call's span, the
//! method the reference resolved to, and `$params`' literal keys — and [`resolve`] makes every
//! lookup afterwards, from [`crate::check::check_program`], beside
//! [`crate::routes::check_table`] and for its reason.
//!
//! # What a site records over, and what it leaves alone
//!
//! [`crate::expr::calls`] has already recorded [`ExprInfo::Call`] against the
//! same span. [`resolve`] records [`ExprInfo::RouteLink`] over it, which wins
//! the lookup ([`crate::expr_table::ExprTypeTable::record`]), so the two states
//! a consumer can see are exactly the link rule's two: a resolved link, or an ordinary
//! call to a member whose body answers at run time. That is why a reference
//! **passed as a value** needs no handling here at all — nothing is recorded,
//! the `Call` stands, and `nvs_stdlib::router`'s own body answers it.
//! The link carries a copy of the call it is recorded over.
//!
//! **The written reference is never called, and never lowered.** It is read for
//! the method it names, so an instance method needs no object and a private
//! one may be linked from another class: [`crate::expr::calls`] marks it in
//! [`crate::Env::link_target_args`] and skips the visibility check, and
//! `nvs_ir` replaces the argument with the route's prepared path.
//!
//! **A `route:` argument is read like a positional one**, because which
//! parameter each written argument fills arrives from
//! [`crate::expr::args::check_args_typed`]: `Core\Router::url(route: Shop::show)`
//! is read where it was written and a method with no route named that way is
//! `E0754` before the program runs
//! (`rule:core-api/parameters-are-callable-by-name`). So the two states above
//! stay the only two. A `$params` the call leaves out is `[]`.

use nvs_diagnostics::{Diagnostic, Diagnostics, Span, code};
use nvs_hir::QName;
use nvs_syntax::ast::{Arg, CallArgs, Expr, ExprKind};

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
/// its refusal too: a method with no route, a capture no key supplies, a key
/// that covers nothing, a value outside a closed set. What it adds is
/// downstream of the lookup — the route's declared path pattern travels to run
/// time, because `rule:core-classes/router-signed-url` signs the mount-relative
/// pattern rather than the path it renders to.
const SIGNED: &str = "urlSigned";

/// `Core\Router\Match`, the class whose [`IS`] this pass also folds.
const MATCH_OWNER: &str = r"Core\Router\Match";

/// `Core\Router\Match::is`, which names a route by a method reference as a
/// link does, and asks the table the first of a link's questions: whether the
/// method has a route. It has no `$params`, so the other three are not asked.
const IS: &str = "is";

/// One [`is_link`] call whose target is a written method reference, waiting
/// for the table to finish.
///
/// Holds strings rather than borrowing the AST because the walk that recorded
/// it has moved on to the next file by the time [`resolve`] runs, and the
/// `Env` it borrowed through does not outlive that file.
#[derive(Clone, Debug)]
pub(crate) struct LinkSite {
    /// The call's own span — the key [`ExprInfo::RouteLink`] is recorded
    /// against, so it lands on the entry [`crate::expr::calls`] already wrote.
    span: Span,
    /// The member as written, which is both the word a diagnostic quotes and
    /// whether an origin is prepended.
    member: String,
    /// The method argument 0 names, as `Class::method` with the class that
    /// declares it — the spelling [`crate::routes::Route::handler`] is written
    /// in. A site exists only because argument 0 was a written reference.
    handler: String,
    /// Where argument 0 was written, which is where a method with no route is
    /// reported: the call's span underlines `$params` too, and the mistake is
    /// in the target.
    target_span: Span,
    /// The written position of the argument that fills `$params`, and `None`
    /// where the call left it out and it is `[]`.
    params_at: Option<usize>,
    /// The written position of the argument that fills `urlSigned`'s signing
    /// settings, and `None` for the two members that sign nothing.
    settings_at: Option<usize>,
    /// The class and method argument 0 names where the reference resolved to
    /// nothing the program declares, as written and resolved against the
    /// file's namespace and imports. [`resolve`] checks it against a class
    /// under an `autoload links` root, and drops it for any other class, which
    /// name resolution has already reported.
    foreign: Option<(QName, String)>,
    /// Argument 1's literal string keys, in written order, or `None` where it
    /// is not an array literal — in which case there is nothing to check
    /// coverage against and `rule:routing/link-target-and-params-are-checked`
    /// asks for nothing.
    args: Option<Vec<LinkArg>>,
}

/// One entry of a link's `$params` whose key folded to a literal.
#[derive(Clone, Debug)]
struct LinkArg {
    /// The key as written, which is what a capture and a `#[Query]` name are
    /// both compared against.
    key: String,
    /// The value as the segment text it would be substituted as, where it
    /// folded to one — a `string` or an `int` literal, which are the two
    /// spellings `rule:routing/a-capture-narrows-to-a-closed-set`'s closed sets are written in. `None` for
    /// everything else, and a `None` is checked against nothing.
    value: Option<String>,
    /// Whether the value is a `float` literal, which [`converts`] refuses for
    /// an `int`, `uint` or `Core\Uuid` parameter without folding it to text.
    float: bool,
    /// The enum case written here, where the value is one — the entry with
    /// **two** spellings rather than one, and so the entry that cannot be
    /// folded to a segment until the parameter it supplies says which
    /// (`rule:routing/an-enum-capture-is-spelled-by-its-backing-value-or-its-case-name`).
    case: Option<CaseArg>,
}

/// An enum case written as a `$params` value: the case, and both texts the rule
/// above could spell it as.
#[derive(Clone, Debug)]
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

/// Whether `owner::member` is one of
/// `rule:routing/link-target-and-params-are-checked`'s link builders — the same nominal
/// test [`crate::retrieval::is_retrieval`] makes, against a resolved [`QName`]
/// rather than against what the call site spelled.
///
/// Public because `nvs-lsp` offers the routed methods at the route argument
/// of exactly these calls, and asks this rather than keeping a second list of
/// them.
#[must_use]
pub fn is_link(owner: &QName, member: &str) -> bool {
    owner.to_string() == OWNER && matches!(member, "url" | ABSOLUTE | SIGNED)
}

/// Whether `owner::member` is `Core\Router::dispatch`, the call that marks the
/// unit as one whose gate answers the server's refusals
/// ([`crate::routes::RouteTable::dispatches`]). The same nominal test as
/// [`is_link`], and recorded beside it, because both ask which `Core\Router`
/// member a resolved call names and nothing about its arguments.
pub(crate) fn is_dispatch(owner: &QName, member: &str) -> bool {
    owner.to_string() == OWNER && member == "dispatch"
}

/// Whether `owner::member` is `Core\Router\Match::is`, whose written target
/// [`record_site`] records as it records a link's. Public for [`is_link`]'s
/// reason: `nvs-lsp` offers the routed methods at its route argument too.
#[must_use]
pub fn is_match_is(owner: &QName, member: &str) -> bool {
    owner.to_string() == MATCH_OWNER && member == IS
}

/// Records one link site, or nothing where
/// `rule:routing/link-target-and-params-are-checked` leaves the call to run time.
///
/// Reports nothing: every refusal this pass makes is a question about the table
/// and is made in [`resolve`], and everything decided here — a reference passed
/// as a value, a `$params` that is not a literal — is legal.
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
    // Each argument is found through the parameter it fills, so a `route:` is
    // read where it was written. Nothing fills `$route` in a call too short to
    // have written it — the arity check's refusal, already made — or in one
    // whose `...` has no variadic tail to land in, which is refused where it
    // was mapped.
    let Some(target) = argument_filling(0, list, slots).map(|arg| &arg.value) else {
        return;
    };
    // A reference written here resolved to its method while the arguments
    // were checked. Anything else is a value, which run time resolves.
    if !matches!(
        target.kind,
        ExprKind::StaticCall {
            args: CallArgs::MethodRef,
            ..
        }
    ) {
        return;
    }
    let (handler, foreign) = match env.exprs.lookup(target.span) {
        Some(ExprInfo::CallableRef(resolved)) => {
            (format!("{}::{}", resolved.class, resolved.method), None)
        }
        // `urlSigned` never takes another module's route, which name
        // resolution reports, and `is` compares a route this program matched.
        _ if member == IS || member == SIGNED => return,
        _ => {
            let Some((class, method)) = written_reference(target, ctx, env) else {
                return;
            };
            (format!("{class}::{method}"), Some((class, method)))
        }
    };
    if member == IS {
        env.links.push(LinkSite {
            span: call.span,
            member: member.to_owned(),
            handler,
            target_span: target.span,
            params_at: None,
            settings_at: None,
            foreign: None,
            args: None,
        });
        return;
    }
    let params_at = written_at(1, list, slots);
    let settings_at = match member == SIGNED {
        true => {
            let Some(at) = written_at(2, list, slots) else {
                return;
            };
            Some(at)
        }
        false => None,
    };
    // A `$params` the call left out is `[]`, whose keys are known: none.
    let args = match params_at {
        Some(at) => literal_args(&list[at].value, ctx, env),
        None => Some(Vec::new()),
    };
    env.links.push(LinkSite {
        span: call.span,
        member: member.to_owned(),
        handler,
        target_span: target.span,
        params_at,
        settings_at,
        foreign,
        args,
    });
}

/// The class and method a written `Class::method` reference names, resolved
/// against the file's namespace and imports, or `None` for any other shape.
fn written_reference(target: &Expr, ctx: &Ctx<'_>, env: &Env<'_>) -> Option<(QName, String)> {
    let ExprKind::StaticCall {
        class,
        method: nvs_syntax::ast::MemberName::Ident(method),
        ..
    } = &target.kind
    else {
        return None;
    };
    let ExprKind::ConstFetch(name) = &class.kind else {
        return None;
    };
    let class = nvs_hir::resolve_ref(
        crate::span_text(env.src, name.span),
        ctx.namespace,
        ctx.imports,
    );
    Some((class, crate::span_text(env.src, *method).to_owned()))
}

/// The written position of the argument that fills parameter `param`, found
/// the way [`argument_filling`] finds it.
fn written_at(param: usize, list: &[Arg], slots: &[ArgSlot]) -> Option<usize> {
    let arg = argument_filling(param, list, slots)?;
    list.iter().position(|written| std::ptr::eq(written, arg))
}

/// `$params`' entries where it is an array literal every key of which is a
/// string literal, and `None` otherwise.
///
/// A keyless element contributes no key rather than aborting the read: it is an
/// integer key, which covers no capture, so the coverage check below is exactly
/// right to report it as a capture with nothing supplying it.
///
/// A **value** that does not fold is kept rather than dropped, and it is why
/// this reads entries and not keys: the key is still a key
/// `rule:routing/a-leftover-link-key-is-a-query-string` has a question about,
/// and only the `rule:routing/a-capture-narrows-to-a-closed-set` question below
/// is the one a computed value has no answer to.
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
            float: is_float_literal(&item.value),
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

/// Whether `expr` is a `float` literal, negated or not.
fn is_float_literal(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Float(_) => true,
        ExprKind::Unary {
            op: nvs_syntax::ast::UnaryOp::Neg,
            expr,
        } => matches!(expr.kind, ExprKind::Float(_)),
        _ => false,
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

/// `rule:routing/link-target-and-params-are-checked`'s compile errors and
/// `rule:routing/a-leftover-link-key-is-a-query-string`'s one, and the fold that
/// follows when none of them applies.
///
/// Run once, after every file has been walked, so `routes` is the whole
/// program's table and a name declared in any file resolves from any other.
pub(crate) fn resolve(
    routes: &RouteTable,
    targets: &nvs_hir::LinkTargets,
    sites: &[LinkSite],
    exprs: &mut ExprTypeTable,
    diags: &mut Diagnostics,
) {
    for site in sites {
        let foreign_row;
        let row = match &site.foreign {
            None => routes.handled_by(&site.handler),
            Some((class, method)) => {
                // A class no links root declares was reported as undeclared,
                // and a method the class does not declare as undefined.
                let Some(target) = targets.get(class) else {
                    continue;
                };
                if !target.methods.iter().any(|declared| declared == method) {
                    continue;
                }
                foreign_row = foreign_route(target, method, &site.handler, site.target_span);
                foreign_row.as_ref()
            }
        };
        let Some(row) = row else {
            report_no_route(site, diags);
            continue;
        };
        let Some(ExprInfo::Call(call)) = exprs.lookup(site.span) else {
            // Unreachable: a site is recorded only for a call that resolved,
            // and that call recorded its `Call` at the same span.
            continue;
        };
        let call = Box::new(call.clone());
        if site.member == IS {
            exprs.record(
                site.span,
                ExprInfo::RouteIs {
                    call,
                    handler: site.handler.clone(),
                },
            );
            continue;
        }
        let Some(pieces) = crate::routes::link_pieces(&row.path) else {
            // Unreachable: a row exists only for a path the path grammar admitted.
            continue;
        };
        // Short-circuiting on purpose: a misspelled key is usually both a
        // capture with nothing supplying it and a key with nothing to be, and
        // naming the capture is the half that says what to write. The closed-set
        // question about a *value* comes last for the same reason — it is only
        // a question once the key it is written under names something.
        if let Some(args) = &site.args
            && !(covered(&pieces, args, site, row, diags)
                && declared(&pieces, args, site, row, diags)
                && within_set(args, site, row, diags)
                && converts(args, site, row, diags))
        {
            continue;
        }
        exprs.record(
            site.span,
            ExprInfo::RouteLink {
                call,
                spellings: spellings(&pieces, row),
                pieces,
                absolute: site.member == ABSOLUTE,
                // The row's declared path pattern, mount-relative: what
                // `signedRoute` derives again from the route the request
                // matched (`rule:core-classes/router-signed-url`).
                signed: (site.member == SIGNED).then(|| row.path.clone()),
                params_at: site.params_at,
                settings_at: site.settings_at,
                handler: site.foreign.as_ref().map(|_| site.handler.clone()),
            },
        );
    }
}

/// A route row for `method` of a class under an `autoload links` root, built
/// from what [`nvs_hir::link_targets`] read of its file, or `None` where the
/// method has no `#[Route]` whose path is a plain string.
///
/// The row carries what [`resolve`]'s four checks read: the path, and each
/// capture and `#[Query]` parameter with its type. A capture whose type is a
/// class or an enum has no type here, so its value is not checked. Nothing
/// else on the row is read for a link, and nothing past this function sees
/// the row.
fn foreign_route(
    target: &nvs_hir::LinkTarget,
    method: &str,
    handler: &str,
    span: Span,
) -> Option<crate::routes::Route> {
    use crate::routes::{ParamIn, Route, RouteParam};
    let route = target
        .routes
        .iter()
        .find(|route| route.method == method && route.path.is_some())?;
    let path = route.path.clone()?;
    let pieces = crate::routes::link_pieces(&path)?;
    let param = |name: &str, source: ParamIn, required: bool| {
        let declared = route.params.iter().find(|param| param.name == name);
        RouteParam {
            name: name.to_owned(),
            source,
            required,
            ty: declared.and_then(|param| param.ty.clone()),
            allowed: None,
            parses: false,
            cases: None,
            default: None,
        }
    };
    let mut params: Vec<RouteParam> = pieces
        .iter()
        .filter_map(|piece| match piece {
            UrlPiece::Required(name) | UrlPiece::Rest(name) => {
                Some(param(name, ParamIn::Path, true))
            }
            UrlPiece::Optional(name) => Some(param(name, ParamIn::Path, false)),
            UrlPiece::Literal(_) => None,
        })
        .collect();
    params.extend(
        route
            .params
            .iter()
            .filter(|declared| declared.query)
            .map(|declared| param(&declared.name, ParamIn::Query, !declared.has_default)),
    );
    Some(Route {
        verb: String::new(),
        path,
        handler: handler.to_owned(),
        static_handler: route.is_static,
        returns_value: false,
        params,
        access: None,
        csrf: true,
        slotted: false,
        summary: None,
        description: None,
        returns: None,
        bodies: Vec::new(),
        tags: Vec::new(),
        security: Vec::new(),
        errors: Vec::new(),
        example: None,
        operation_id: None,
        span,
    })
}

/// `E0754` for a site whose method has no `#[Route]`, worded for the member
/// that wrote it.
fn report_no_route(site: &LinkSite, diags: &mut Diagnostics) {
    let (message, help) = match site.member == IS {
        true => (
            format!(
                "`Core\\Router\\Match::is` compares the route with `{}`, and that method has no \
                 `#[Route]`",
                site.handler
            ),
            "`is` checks whether the request matched a route of the method you pass. Add a \
             `#[Route]` to this method, or pass the method that has the route",
        ),
        false => (
            format!(
                "`Core\\Router::{}` links to `{}`, and that method has no `#[Route]`",
                site.member, site.handler
            ),
            "a link is built from the `#[Route]` of the method you pass. Add a `#[Route]` to \
             this method, or pass the method that has the route",
        ),
    };
    diags.report(
        Diagnostic::error(code::E_LINK_METHOD_HAS_NO_ROUTE, message)
            .with_primary(site.target_span, "this method has no route")
            .with_help(help),
    );
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
/// neither is not inert: `rule:routing/a-leftover-link-key-is-a-query-string`
/// turns every non-capture key into the link's query string, so a typo ships as
/// `?typo=…` rather than being dropped, and the rule is what makes that turn
/// safe to make at all.
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
            "a key that is not a capture goes into the link's query string. No parameter reads \
             this key. Correct the name, or declare the parameter with `#[Query]` on the handler",
        ),
    );
    false
}

/// `rule:routing/link-target-and-params-are-checked`'s capture compile error:
/// every capture that must be substituted has a key supplying it.
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
            "`$params` must give a value for every capture of the route. Only a `{name?}` \
             capture may be left out. Its whole segment is then left out of the link",
        ),
    );
    false
}

/// `rule:routing/a-capture-narrows-to-a-closed-set`'s compile error: a literal value the parameter it supplies
/// cannot hold.
///
/// That rule narrows a capture to a closed set with a *type*, and a segment
/// outside that set fails the conversion and falls through to a `404` — so a link
/// built out of one is a link to a route that will not match, and the table can
/// say so before the program runs. That is the same laundering argument
/// `rule:routing/link-target-and-params-are-checked` makes about the path: a
/// value is checked where it is written, because the sink it reaches cannot
/// check it.
///
/// **A `#[Query]` parameter's set is checked too**, and by the same walk, since
/// `rule:routing/a-query-parameter-is-declared-like-a-capture` gives a query
/// value the same type list a capture has: a value outside it
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
                "a request with this value does not match the route, and the server returns \
                 `404`. Correct the value, or change the parameter's type so that it allows \
                 the value",
            ),
        );
        return false;
    }
    true
}

/// `rule:routing/link-target-and-params-are-checked`'s value check: a literal
/// value that the conversion of the parameter it supplies refuses.
///
/// [`within_set`] asks this of a closed set, and this asks it of the types the
/// matcher converts natively: `int`, `uint`, `decimal` and `Core\Uuid`. The
/// segment the value builds is run through the matcher's own conversion
/// ([`nvs_stdlib::router::segment_converts`]), so `"x"`, `-3` and `2.5` for a
/// `uint` are each a link to a route that will not match. A `float` literal is
/// refused for every one of those types but `decimal` without being folded to
/// text: it is not a whole number, and it is not a UUID. A value that did not
/// fold, a `string` parameter and a class built from text are not checked.
///
/// `E0772` is reused: its card already says the value is not one the
/// parameter allows. A `#[Query]` parameter is checked by the same walk, as
/// [`within_set`] checks it.
fn converts(
    args: &[LinkArg],
    site: &LinkSite,
    row: &crate::routes::Route,
    diags: &mut Diagnostics,
) -> bool {
    for arg in args {
        let Some(param) = row.params.iter().find(|param| param.name == arg.key) else {
            continue;
        };
        if param.admits().is_some() {
            continue;
        }
        let Some(ty) = param.ty.as_deref() else {
            continue;
        };
        let refused = match (&arg.value, arg.float) {
            (_, true) => nvs_stdlib::router::native_conv(ty).is_some() && ty != "decimal",
            (Some(segment), false) => {
                nvs_stdlib::router::segment_converts(ty, segment) == Some(false)
            }
            (None, false) => false,
        };
        if !refused {
            continue;
        }
        diags.report(
            Diagnostic::error(
                code::E_ROUTE_LINK_VALUE_NOT_IN_SET,
                format!(
                    "`{}` declares `{}` as `{ty}`, and this link gives it a value that does \
                     not convert to `{ty}`",
                    row.handler, arg.key
                ),
            )
            .with_primary(site.span, "no request could carry this value")
            .with_help(
                "the router converts each part of the path to the type of its parameter. A \
                 value that does not convert leads to a `404` page. Give a value of the \
                 parameter's type",
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

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use nvs_diagnostics::{Code, Diagnostics, SourceMap, code};
    use nvs_hir::{AutoloadMap, CoreRoster};

    use crate::expr_table::{ExprInfo, ExprTypeTable};
    use crate::ty::TypeInterner;

    /// The module a link reaches: one class under a links root, with a routed
    /// `show` that takes a capture and a `#[Query]` parameter, and an unrouted
    /// `plain`. Its bodies are empty, because nothing in it is compiled.
    const TARGET: &str = "<?nvs\n\
        namespace Framework\\Shop\\View;\n\
        \n\
        use Core\\Http\\Method;\n\
        use Core\\Query;\n\
        use Core\\Route;\n\
        \n\
        class Index\n\
        {\n\
            #[Route(path: '/item/{id}', method: Method::Get)]\n\
            public static function show(int $id, #[Query] ?string $q = null): void {}\n\
        \n\
            public static function plain(): void {}\n\
        }\n";

    /// What one fixture's compile left behind.
    struct Checked {
        exprs: ExprTypeTable,
        diags: Diagnostics,
        map: SourceMap,
        autoload: AutoloadMap,
        /// The canonical path of every file the walk compiled.
        loaded: Vec<PathBuf>,
    }

    impl Checked {
        fn codes(&self) -> Vec<Code> {
            self.diags.iter().filter_map(|d| d.code).collect()
        }

        fn has(&self, want: Code) -> bool {
            self.codes().contains(&want)
        }

        /// Every link the check resolved, as its handler.
        fn handlers(&self) -> Vec<Option<String>> {
            self.exprs
                .since(0)
                .iter()
                .filter_map(|info| match info {
                    ExprInfo::RouteLink { handler, .. } => Some(handler.clone()),
                    _ => None,
                })
                .collect()
        }
    }

    /// Whether `path` is the target file, `Shop/src/View/Index.nvs`.
    fn is_target(path: &Path) -> bool {
        path.ends_with(Path::new("View").join("Index.nvs"))
    }

    /// Writes an entry with `body` after its `autoload links` line, and the
    /// target module beside it, then resolves and checks the program from disk.
    fn check(body: &str) -> Checked {
        let dir = nvs_repo::scratch("links-target");
        let view = dir.join("Shop").join("src").join("View");
        std::fs::create_dir_all(&view).expect("create the target's folder");
        std::fs::write(view.join("Index.nvs"), TARGET).expect("write the target");
        let entry = dir.join("main.nvs");
        let src = format!("<?nvs\nautoload links 'Framework\\Shop' from './Shop/src';\n\n{body}");
        std::fs::write(&entry, src).expect("write the entry");

        let mut map = SourceMap::new();
        let mut diags = Diagnostics::new();
        let entry_id = map.load(&entry).expect("load the entry");
        let stmts = nvs_syntax::parse_file(map.file(entry_id), &mut diags);
        assert!(!diags.has_errors(), "the entry failed to parse: {diags:?}");
        let (module, files, autoload) =
            nvs_hir::resolve_program(entry_id, stmts, &mut map, CoreRoster::Trusted, &mut diags);
        let program: Vec<_> = files
            .iter()
            .map(|file| crate::ProgramFile {
                src: map.file(file.id),
                stmts: &file.stmts,
            })
            .collect();
        let mut interner = TypeInterner::new();
        let mut exprs = ExprTypeTable::new();
        crate::check_program(&program, &module, &mut interner, &mut exprs, &mut diags);
        let loaded = files
            .iter()
            .filter_map(|file| map.file(file.id).path().map(Path::to_path_buf))
            .collect();
        Checked {
            exprs,
            diags,
            map,
            autoload,
            loaded,
        }
    }

    /// `rule:routing/link-target-and-params-are-checked`, over a route of a
    /// links root: a good link resolves with the target's handler, and each
    /// of the local link's mistakes is the local link's error.
    #[test]
    fn a_link_to_a_links_root_route_is_checked_like_a_local_one() {
        let good = check(
            "Core\\Router::url(Framework\\Shop\\View\\Index::show, ['id' => 3, 'q' => 'a']);\n",
        );
        assert!(
            !good.diags.has_errors(),
            "a good link failed: {:?}",
            good.diags
        );
        assert_eq!(
            good.handlers(),
            [Some(r"Framework\Shop\View\Index::show".to_owned())]
        );

        let cases = [
            ("['q' => 'a']", "show", code::E_ROUTE_LINK_MISSING_PARAM),
            (
                "['id' => 3, 'nope' => 1]",
                "show",
                code::E_ROUTE_LINK_UNKNOWN_PARAM,
            ),
            ("['id' => 'x']", "show", code::E_ROUTE_LINK_VALUE_NOT_IN_SET),
            ("[]", "plain", code::E_LINK_METHOD_HAS_NO_ROUTE),
            ("[]", "missing", code::E_UNDEFINED_MEMBER),
        ];
        for (params, method, want) in cases {
            let bad = check(&format!(
                "Core\\Router::url(Framework\\Shop\\View\\Index::{method}, {params});\n"
            ));
            assert!(
                bad.has(want),
                "`{method}` with {params} gave {:?}, want {want:?}",
                bad.codes()
            );
        }
    }

    /// A links root is read for one link and never compiled: none of its
    /// routes is in the program's table, and its file is not a compiled file.
    #[test]
    fn a_links_root_route_never_joins_the_table() {
        let checked =
            check("Core\\Router::url(Framework\\Shop\\View\\Index::show, ['id' => 3]);\n");
        assert!(!checked.diags.has_errors(), "{:?}", checked.diags);
        let rows = checked.exprs.routes().rows();
        assert!(
            rows.iter().all(|row| !row.handler.contains("Index")),
            "a links root's route joined the table: {:?}",
            rows.iter().map(|row| &row.handler).collect::<Vec<_>>()
        );
        assert!(
            !checked.loaded.iter().any(|path| is_target(path)),
            "the target file was compiled: {:?}",
            checked.loaded
        );
    }

    /// Every use of a links-root class but a link's route is `E0897`, never
    /// the plain unknown-class `E0303`.
    #[test]
    fn a_links_root_class_used_for_anything_else_does_not_compile() {
        let uses = [
            "new Framework\\Shop\\View\\Index();\n",
            "class Holder\n{\n    public static function take(Framework\\Shop\\View\\Index $page): void {}\n}\n",
        ];
        for body in uses {
            let checked = check(body);
            assert!(
                checked.has(code::E_LINKS_ONLY_CLASS),
                "{body} gave {:?}",
                checked.diags
            );
            assert!(
                !checked.has(code::E_UNDEFINED_CLASS),
                "{body} still gave E0303: {:?}",
                checked.diags
            );
        }
    }

    /// A `use` import may name a links-root class the program links to, so a
    /// link writes its short name. The imported name used for anything else is
    /// still `E0897`.
    #[test]
    fn a_use_import_names_a_links_root_class_for_a_link() {
        let good = check(
            "use Framework\\Shop\\View\\Index;\n\n\
             Core\\Router::url(Index::show, ['id' => 3]);\n",
        );
        assert!(
            !good.diags.has_errors(),
            "an imported link failed: {:?}",
            good.diags
        );
        assert_eq!(
            good.handlers(),
            [Some(r"Framework\Shop\View\Index::show".to_owned())]
        );

        let bad = check(
            "use Framework\\Shop\\View\\Index;\n\n\
             Core\\Router::url(Index::show, ['id' => 3]);\n\
             new Index();\n",
        );
        assert!(
            bad.has(code::E_LINKS_ONLY_CLASS),
            "the imported class was constructed: {:?}",
            bad.diags
        );
    }

    /// `rule:core-classes/router-signed-url`: a program cannot sign a link
    /// that another program checks.
    #[test]
    fn a_signed_link_to_a_links_root_route_does_not_compile() {
        let checked = check(
            "Core\\Router::urlSigned(Framework\\Shop\\View\\Index::show, ['id' => 3], \
             ['keys' => [], 'until' => null]);\n",
        );
        assert!(
            checked.has(code::E_SIGNED_LINK_TO_ANOTHER_MODULE),
            "{:?}",
            checked.codes()
        );
    }

    /// The target file is in the source map and in the probe trace, so a
    /// changed or removed route recompiles the program that links to it.
    #[test]
    fn a_links_target_file_joins_the_revalidation_set() {
        let checked =
            check("Core\\Router::url(Framework\\Shop\\View\\Index::show, ['id' => 3]);\n");
        assert!(
            checked
                .map
                .files()
                .any(|file| file.path().is_some_and(is_target)),
            "the target is not in the source map"
        );
        assert!(
            checked
                .autoload
                .probe_trace()
                .probed()
                .iter()
                .any(|path| is_target(path)),
            "the target is not in the probe trace: {:?}",
            checked.autoload.probe_trace().probed()
        );
    }
}
