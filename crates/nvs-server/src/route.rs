//! `rule:routing/matched-once-before-the-handler`
//! 's match, taken at the door: the compiled unit's route table against the
//! request that arrived, once, before any application code runs.
//!
//! # Why the door and not the program
//!
//! § 1 names three rules that were already written against a match the server
//! had not got — `rule:security/csrf-is-on-by-default`'s CSRF check, `rule:observability/route-label-is-the-path-pattern`'s `route` label, and
//! § 8's access decision — and each of them needs the answer *before* the
//! handler is called. A program that matched for itself would make two matches
//! of one question and could not have answered the first three at all, so the
//! match happens here and rides on the request:
//! [`nvs_runtime::Inbound::set_route`] is where it is written and
//! `Core\Request::route()` is the one reader.
//!
//! **This module dispatches nothing.** It computes a name and typed parameters
//! and stops, which is § 1's own second rule and `rule:routing/matching-is-not-dispatching`'s refusal list
//! left untouched: nothing here calls the annotated method, and nothing here
//! decides what a return value means.
//!
//! # What the answer is read for
//!
//! Two of § 1's three rules read it here: [`csrf`] is `rule:security/csrf-is-on-by-default`'s
//! refusal, over [`csrf_required`]'s question, and [`label`] is `rule:observability/route-label-is-the-path-pattern`'s `route` label. Neither matches
//! anything — each is a field of the row the door already found — which is what
//! § 1 buys and is why they live beside [`take`] rather than beside the
//! subsystem each belongs to. § 8's access decision has no reader here on
//! purpose: it is the *dispatcher's*, and this crate does not dispatch.
//!
//! # The CSRF refusal, and what arms each half of it
//!
//! [`csrf`] answers a verdict and never a response, which is this module's own
//! refusal list again: the caller turns a [`Csrf::Refused`] into the `403` no
//! application code ever sees. The verdict has two independent grounds, and they
//! arm separately because they cost different things to deploy.
//!
//! **The origin half needs nothing configured.** A checked request whose `Origin`
//! names somewhere other than where it arrived is a cross-site write, and it is
//! refused. That is the forgery the entry is named for, it reads only headers the
//! request brought, and every browser sends the header on an unsafe verb.
//!
//! **The token half is armed by `[http] csrf_key`.** With a key named, a checked
//! request is refused unless it carries a token this deployment issued, bound to
//! *this* request's session — `nvs_runtime::csrf` is the construction, the
//! session is the value of the cookie `[session] cookie` names, read by the
//! reader `Core\Session::start` uses, a request with no session passes no
//! token, and
//! [`TOKEN_HEADER`] says where the door looks: the header, the `_csrf` query
//! parameter, and an urlencoded form's `_csrf` field, which the request's own
//! task reads before the handler ([`reading_the_form`]). A valid token passes whatever the
//! origin says, because it proves the request came from a page this application
//! rendered.
//!
//! **An upgrade is checked on any verb, by its origin alone.** A browser opens
//! a WebSocket with a `GET` that carries the visitor's cookies, so a page on
//! another origin could otherwise hold a socket in the visitor's name. A request
//! with an `Upgrade` header that matched a row is refused when its `Origin` is
//! another origin, whatever the key says, because the handshake has no way to
//! carry a token. The row's `csrf: false` turns this off as it turns off the
//! write check, and a request with no `Origin` passes, as it does there.
//!
//! **What a deployment that names no key gets, stated as the bound:** the origin
//! half and not the token half. A request with no `Origin` header at all — every
//! non-browser client, and so every API caller and every health check — is
//! therefore not refused there. The token half is what covers it, and naming a
//! key is what turns it on; a key the door could not read never reaches here,
//! because `nvs_config::http`'s `validate` refuses that tree at boot rather than
//! leaving a check silently unarmed.
//!
//! # Where it sits among the door's other decisions
//!
//! Later than the rest of them, and necessarily. The peer walk
//! ([`crate::forwarded`]), the ceiling ([`crate::admit`]) and the cross-origin
//! answer ([`crate::cors`]) are all decided from configuration the boot
//! resolved, so [`crate::serve`] takes them on the connection before a mount is
//! even selected. A match needs the *unit*, and which unit answers a request is
//! `rule:http-server/a-request-resolves-in-five-steps`'s five steps — so this runs where the selected program and the
//! arrived request are both in hand, which is the last point before the isolate
//! is built. It is still "before the handler" in § 1's sense: no application
//! code has run.
//!
//! A request whose unit declares no route is matched against nothing and
//! carries no match, which is `rule:routing/table-is-opt-in`'s opt-in rule; so is one whose
//! program failed to compile, because the table is a product of the compile
//! that did not happen. An unsafe verb that carries no match still gets the
//! origin half: a write whose `Origin` names another origin is refused whether
//! or not a row claimed it.
//!
//! **What it spends:** one walk of the table per request, and — for a matched
//! one — what [`nvs_runtime::routes`] accounts for. Nothing is held past the
//! request.

use nvs_runtime::Inbound;
use nvs_runtime::routes::Routes;

/// Matches this request against `routes` once, and records the answer on it.
///
/// The verb and the path are read off the carrier rather than off `hyper`'s
/// request, because the path a route is declared against is the **mount-
/// stripped** one — `rule:http-server/a-request-resolves-in-five-steps` step 2's remainder, which
/// [`crate::mount::Selection::path`] carries and the carrier already holds. A
/// mounted application is written against its own root, so matching the
/// prefixed path would fail every route in a table that is otherwise correct.
///
/// Nothing is written for a request that matched nothing: `None` on the carrier
/// is § 1's `null`, and the program may still serve the request however it
/// likes.
pub fn take(routes: &Routes, inbound: &mut Inbound) {
    // Resolved into a local first, which ends the borrow of the carrier before
    // the answer is written back onto it — and is also the shape that makes the
    // "once" visible: one call, one answer, one write.
    let matched = routes.match_request(inbound.method(), inbound.path());
    if let Some(matched) = matched {
        inbound.set_route(matched);
    }
}

/// `rule:security/csrf-is-on-by-default`'s question, asked of the match rather than of the table: is
/// this request CSRF-checked?
///
/// `true` where the request matched a row an unsafe verb declared and whose
/// `#[Access]` did not write § 1a's `csrf: false`. Both halves are read off
/// [`nvs_runtime::routes::Match::route`] — the row [`take`] already found —
/// which is exactly what `rule:routing/matched-once-before-the-handler` removes: a check that matched for itself
/// would make two matches of one question, and § 4 was specified against the
/// answer this one already has.
///
/// **A request that matched nothing is not covered here**, which is
/// `rule:routing/table-is-opt-in`'s opt-in rule: a program with no route table
/// has no row to write `csrf: false` on, so a token required of its every
/// `POST` would refuse every write it serves. [`csrf`] still refuses such a
/// write when its `Origin` names another origin ([`unmatched_write`]).
///
/// The refusal this answer feeds is [`csrf`], which is where § 4's other half —
/// what makes a covered request acceptable — is decided.
#[must_use]
pub fn csrf_required(inbound: &Inbound) -> bool {
    inbound
        .route()
        .is_some_and(|matched| matched.route().csrf())
}

/// The four unsafe verbs as they arrive on the wire. A method token is
/// case-sensitive (RFC 9110 § 9.1), and the door answers `501` for any other.
const UNSAFE_METHODS: [&str; 4] = ["POST", "PUT", "PATCH", "DELETE"];

/// Whether this request is an unsafe verb no row matched: a write to a program
/// with no route table, or to a path its table does not claim.
///
/// Such a write gets the origin half of [`csrf`] and not the token half. The
/// origin half needs no route, because it refuses only a browser that names
/// another origin. The token half would refuse every write a program with no
/// table serves under a key, a webhook included, and that program has no route
/// to write `csrf: false` on.
fn unmatched_write(inbound: &Inbound) -> bool {
    inbound.route().is_none() && UNSAFE_METHODS.contains(&inbound.method())
}

/// Whether this request asks to upgrade the connection over a row that did not
/// write `csrf: false` — the cross-site WebSocket a browser opens with the
/// visitor's cookies, which no verb check covers because the handshake is a
/// `GET`.
///
/// Any `Upgrade` header counts, whatever protocol it names: `hyper` frames an
/// upgrade for every one of them, and reading the value would be a second parser
/// of a question already answered. A request that matched nothing is not
/// covered, for [`csrf_required`]'s reason: the opt-out is written on a route,
/// and a program with no table has nowhere to write it.
fn upgrade_checked(inbound: &Inbound) -> bool {
    header(inbound, UPGRADE_HEADER).is_some()
        && inbound
            .route()
            .is_some_and(|matched| matched.route().upgrade_checked())
}

/// Where the door looks first for a presented token.
///
/// The spelling is the one every browser-side helper already writes, so `fetch`
/// and a rendered form's script need nothing new. A request that sends no header
/// is asked [`TOKEN_FIELD`] in its query, and then in its body where that body is
/// an urlencoded form.
///
/// **A multipart body is never read for one.** It crosses as a stream the
/// isolate pulls (`rule:http-server/request-body-and-upload-total-are-two-caps`),
/// and a token at the end of a large upload could only be found by buffering the
/// upload before deciding whether to run it, on the attacker's terms. A multipart
/// write carries its token here or in the query. An urlencoded form is read,
/// because `[limits] request_body` already bounds it and the handler's own form
/// parse would buffer the same bytes.
pub const TOKEN_HEADER: &str = "x-csrf-token";

/// The form field and the query parameter a token may ride in instead of
/// [`TOKEN_HEADER`]: what a rendered `<form>` posts with no script at all.
pub const TOKEN_FIELD: &str = "_csrf";

/// The media type of the one body the door reads for a token.
const URLENCODED: &str = "application/x-www-form-urlencoded";

/// The header naming where a checked request says it came from.
const ORIGIN_HEADER: &str = "origin";

/// The header a request asks to take the connection over with.
const UPGRADE_HEADER: &str = "upgrade";

/// What the door decided about a request's cross-origin write —
/// `rule:security/csrf-is-on-by-default`'s refusal, as a verdict.
///
/// A verdict and not a response, which is this module's refusal list: the caller
/// turns [`Self::Refused`] into a `403`, and nothing here knows what a status
/// code is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Csrf {
    /// Run it — not a checked request, or one this deployment can tie to itself.
    Passed,
    /// A checked request this deployment cannot tie to itself: refuse it, before
    /// an isolate exists.
    Refused,
    /// A keyed, checked urlencoded write with no token in its header or query:
    /// its body decides. The door runs inside the connection's poll, where no
    /// body can be pulled (`nvs_runtime::RequestBody::next_chunk`), so the
    /// caller wraps the program in [`reading_the_form`], and the request's own
    /// task reads the field before the handler runs.
    InBody,
}

/// `rule:security/csrf-is-on-by-default`'s refusal, taken at the door over the
/// match [`take`] already made.
///
/// `key` is the deployment's token key — `[http] csrf_key` decoded, which
/// `nvs_config::http::csrf_key` answers — and `session_cookie` the name the
/// identifier rides under, which is `[session] cookie` or `nvs_stdlib::session`'s
/// default. Both are the caller's to resolve: this crate reads no configuration
/// and holds no roster.
///
/// The two grounds and what arms each are the module doc's own section. In
/// order: a request the table did not claim for a checked verb passes
/// ([`csrf_required`] is the whole of that question), unless it is an unsafe
/// verb no row matched and its `Origin` is another origin; a request carrying a token
/// this key issued for this request's session passes, whatever its origin says;
/// a request whose `Origin` is not where it arrived is refused; and, where a key
/// is named, a request with no such token is refused even same-origin, because a
/// deployment that named a key asked for its unsafe verbs to carry one.
///
/// The session is read the way `Core\Session::start` reads it ([`session`]),
/// and a request with no session has no token that passes ([`verified`]).
///
/// The token is the [`TOKEN_HEADER`] header, else the [`TOKEN_FIELD`] query
/// parameter, and the first one present decides. With neither, an urlencoded
/// write answers [`Csrf::InBody`] and every other one is refused.
///
/// An upgrade is asked [`upgrade_checked`]'s question before any of that, and
/// only the origin half of it: a browser's WebSocket handshake cannot carry a
/// token header, so a key that refused one without a token would refuse every
/// socket. The verb's own check still follows, so an upgrade header on a checked
/// write takes nothing off the token it owes.
#[must_use]
pub fn csrf(inbound: &Inbound, key: Option<&[u8]>, session_cookie: &str) -> Csrf {
    if upgrade_checked(inbound) && cross_origin(inbound) {
        return Csrf::Refused;
    }
    if !csrf_required(inbound) {
        return if unmatched_write(inbound) && cross_origin(inbound) {
            Csrf::Refused
        } else {
            Csrf::Passed
        };
    }
    // Keyed first, so that a valid token is the one answer that does not depend
    // on a header the request chose: a token proves the page it was rendered
    // into is this application's, which is strictly more than an origin says.
    if let Some(key) = key.and_then(nvs_runtime::csrf::Key::new) {
        let presented = match header(inbound, TOKEN_HEADER) {
            Some(value) => Some(std::str::from_utf8(value).unwrap_or("").to_owned()),
            None => field(inbound.query().as_bytes(), TOKEN_FIELD),
        };
        return match presented {
            Some(presented) if verified(&key, &presented, inbound, session_cookie) => Csrf::Passed,
            Some(_) => Csrf::Refused,
            None if urlencoded(inbound) => Csrf::InBody,
            None => Csrf::Refused,
        };
    }
    if cross_origin(inbound) {
        Csrf::Refused
    } else {
        Csrf::Passed
    }
}

/// Whether this request says it was written somewhere other than where it
/// arrived.
///
/// `false` for a request that sent no `Origin`, which is the bound the module doc
/// states: the header is a browser's to send, so its absence is every
/// non-browser client rather than a suspicious browser, and refusing on it would
/// refuse every API caller in the name of a forgery none of them can mount.
///
/// The comparison is byte for byte against the scheme the peer reached this
/// server by and the `Host` it addressed — the same two values every absolute
/// link this server writes is built from — so a port, a subdomain and a scheme
/// downgrade are each a different origin, which is what a browser means by one.
fn cross_origin(inbound: &Inbound) -> bool {
    let Some(origin) = header(inbound, ORIGIN_HEADER) else {
        return false;
    };
    let Some(host) = header(inbound, "host") else {
        // A request with an `Origin` and no `Host` reached no name on this
        // server, so there is nothing for the origin to be the same as.
        return true;
    };
    let scheme = match inbound.scheme() {
        nvs_runtime::Scheme::Https => "https://",
        nvs_runtime::Scheme::Http => "http://",
    };
    let mut own = Vec::with_capacity(scheme.len() + host.len());
    own.extend_from_slice(scheme.as_bytes());
    own.extend_from_slice(host);
    origin != own.as_slice()
}

/// The first value of the header `name`, matched without regard to case.
///
/// A walk of the carrier's lines rather than a map: a request holds few of them,
/// the door asks for at most five, and `nvs_runtime::Inbound` keeps headers in
/// the spelling and the order the peer sent them on purpose.
fn header<'a>(inbound: &'a Inbound, name: &str) -> Option<&'a [u8]> {
    inbound
        .headers()
        .find(|(field, _)| field.eq_ignore_ascii_case(name))
        .map(|(_, value)| value)
}

/// The session a token must be bound to: the value of the cookie `name`, or
/// `None` where the request carries no session.
///
/// [`nvs_runtime::Inbound::cookie`] reads it, which is the reader
/// `Core\Session::start` uses, so the door checks the token against the session
/// the program will open: every `Cookie` line, pair by pair. A value that is
/// not UTF-8 is no identifier this server minted, and an empty one is no session.
fn session<'a>(inbound: &'a Inbound, name: &str) -> Option<&'a str> {
    inbound
        .cookie(name.as_bytes())
        .and_then(|value| std::str::from_utf8(value).ok())
        .filter(|value| !value.is_empty())
}

/// Whether `presented` is a token `key` issued for this request's session.
///
/// **A request with no session passes no token.** A token for no session is
/// what a page shown to a visitor without one carries, so anyone can load it,
/// and accepting it would let another site sign a visitor in to the attacker's
/// account. An application that checks a write from a visitor with no session
/// writes the session first, so the token it renders is bound to an identifier.
fn verified(key: &nvs_runtime::csrf::Key, presented: &str, inbound: &Inbound, name: &str) -> bool {
    session(inbound, name).is_some_and(|session| key.verify(presented, session))
}

/// The token half for a [`Csrf::InBody`] request: reads its urlencoded body,
/// up to `limit` bytes, and checks the [`TOKEN_FIELD`] field in it.
///
/// Called on the request's own task, before the handler, never from the door:
/// a pull parks the task until the connection delivers the bytes. A body past
/// `limit`, one that fails mid-read and one with no field are each
/// [`Csrf::Refused`]. The bytes read go back on the carrier as a body already in
/// memory, so every reader the program calls finds the whole form.
///
/// **What it spends:** up to `limit`, which is `[limits] request_body`, per
/// such request in flight, attributed to that request. A form reader the
/// handler calls holds a second copy of the same bytes.
#[must_use]
pub fn csrf_in_body(
    inbound: &mut Inbound,
    key: Option<&[u8]>,
    session_cookie: &str,
    limit: usize,
) -> Csrf {
    let Some(key) = key.and_then(nvs_runtime::csrf::Key::new) else {
        return Csrf::Refused;
    };
    let Some(body) = inbound.body() else {
        return Csrf::Refused;
    };
    let mut read = Vec::new();
    loop {
        match body.next_chunk() {
            Ok(Some(chunk)) if read.len().saturating_add(chunk.len()) <= limit => {
                read.extend_from_slice(chunk);
            }
            Ok(None) => break,
            Ok(Some(_)) | Err(_) => return Csrf::Refused,
        }
    }
    let passed = field(&read, TOKEN_FIELD)
        .is_some_and(|presented| verified(&key, &presented, inbound, session_cookie));
    inbound.set_buffered_body(read);
    if passed { Csrf::Passed } else { Csrf::Refused }
}

/// `program`, preceded by [`csrf_in_body`]'s check of the request it answers.
///
/// A request that passes runs `program`. A refused one runs nothing and
/// answers an empty `403`, or, where `refusal_mode` says the unit calls
/// `Core\Router::dispatch`, runs `program` with the refusal on the carrier and
/// no body, as the door's own refusal does
/// (`rule:routing/a-gate-answers-around-every-handler`).
pub fn reading_the_form(
    program: nvs_runtime::script::Program,
    key: Vec<u8>,
    session_cookie: String,
    refusal_mode: bool,
) -> nvs_runtime::script::Program {
    Box::new(move |ctx: &mut nvs_runtime::Ctx, args| {
        let limit = ctx.request_body_limit();
        let verdict = ctx.inbound_mut().map_or(Csrf::Refused, |inbound| {
            csrf_in_body(inbound, Some(&key), &session_cookie, limit)
        });
        if verdict == Csrf::Refused {
            if !refusal_mode {
                ctx.declare_status(403);
                return nvs_runtime::Value::null();
            }
            if let Some(inbound) = ctx.inbound_mut() {
                inbound.set_refusal(nvs_runtime::routes::Refusal::Csrf);
                inbound.clear_body();
            }
        }
        program(ctx, args)
    })
}

/// Whether this request says its body is an urlencoded form. The media type is
/// compared without regard to case, and a parameter such as `charset` is
/// ignored.
fn urlencoded(inbound: &Inbound) -> bool {
    header(inbound, "content-type")
        .and_then(|value| std::str::from_utf8(value).ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|media| media.trim().eq_ignore_ascii_case(URLENCODED))
}

/// The first value of the field `name` in an urlencoded `encoded`, decoded.
///
/// A query and a form body are the same format. A pair whose name or value
/// does not decode to UTF-8 is passed over.
fn field(encoded: &[u8], name: &str) -> Option<String> {
    encoded.split(|byte| *byte == b'&').find_map(|pair| {
        let (found, value) = match pair.iter().position(|byte| *byte == b'=') {
            Some(at) => (&pair[..at], &pair[at + 1..]),
            None => (pair, &[][..]),
        };
        if decoded(found)? == name {
            decoded(value)
        } else {
            None
        }
    })
}

/// `text` with `+` read as a space and each `%XX` as its octet, or `None` for
/// an escape that is not two hex digits or a result that is not UTF-8.
fn decoded(text: &[u8]) -> Option<String> {
    let mut out = Vec::with_capacity(text.len());
    let mut at = 0;
    while let Some(&byte) = text.get(at) {
        match byte {
            b'+' => out.push(b' '),
            b'%' => {
                let hex = text.get(at + 1..at + 3)?;
                if !hex.iter().all(u8::is_ascii_hexdigit) {
                    return None;
                }
                out.push(u8::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok()?);
                at += 2;
            }
            byte => out.push(byte),
        }
        at += 1;
    }
    String::from_utf8(out).ok()
}

/// `rule:observability/route-label-is-the-path-pattern`'s `route` label for this request: the matched route's
/// **path pattern** as its `#[Route]` wrote it, such as `/users/{id}`.
///
/// The one label of that table which would otherwise be unbounded, which is why
/// it is read off the match and computed nowhere else — its value comes from
/// the compile-time table, a closed set, and never from the request's own path,
/// whose cardinality the client chooses.
///
/// `None` in the two absences, which are one case: nothing matched, and no
/// table to match against. Every row has a pattern, so every matched request
/// is labelled.
///
/// [`crate::serve`] is the caller, once per request it ran, off the carrier the
/// reply is still holding — `nvs_host::Isolate::answering_request` owns why the
/// door reads it there and not after the isolate has started.
#[must_use]
pub fn label(inbound: &Inbound) -> Option<&str> {
    Some(inbound.route()?.pattern())
}

#[cfg(test)]
mod tests {
    use nvs_runtime::Inbound;
    use nvs_runtime::routes::{Capture, CaptureConv, Param, Route, Routes};

    use super::Csrf;

    /// Two rows over one path shape, so that a matcher which answered with the
    /// first row it could fill would be visible.
    fn table() -> Routes {
        Routes::new(vec![
            Route::new(
                "Get",
                "/users/{id}",
                "App\\Users::show",
                Some("Core\\Audience::Public".to_owned()),
                vec![Capture {
                    name: "id".to_owned(),
                    conv: CaptureConv::Uint,
                }],
            ),
            Route::new("Get", "/users/new", "App\\Users::new", None, vec![]),
        ])
    }

    /// What a handler is handed: the carrier by value, exactly as
    /// [`crate::serve`]'s does, so that what the assertions read is what an
    /// application would read and not a carrier the door still owns.
    fn handler(inbound: Inbound) -> Option<(String, Vec<(String, Param)>)> {
        let matched = inbound.route()?;
        Some((
            matched.route().handler().to_owned(),
            matched.params().to_vec(),
        ))
    }

    /// `rule:routing/matched-once-before-the-handler`: the match is taken before the handler, it is taken once,
    /// and `Core\Request::route()`'s carrier answers with *that* match rather
    /// than with a second one.
    ///
    /// The table is **dropped before the handler is called**, which is the half
    /// a shape-only assertion would miss: a match that travelled cannot be
    /// re-derived, so a carrier that still answers correctly with no table left
    /// to ask is one holding the door's own answer.
    #[test]
    fn a_request_is_matched_once_before_the_handler_and_route_returns_that_match() {
        let routes = table();
        let mut inbound = Inbound::new("GET", "/users/42", "");
        super::take(&routes, &mut inbound);
        drop(routes);

        assert_eq!(
            handler(inbound),
            Some((
                "App\\Users::show".to_owned(),
                vec![("id".to_owned(), Param::Uint(42))]
            ))
        );
    }

    /// § 4's two answers over one verb, and the verb that was never covered:
    /// an unsafe route that is checked, the same verb carrying § 1a's opt-out —
    /// the webhook § 4 names as the legitimate case — and a safe one.
    fn checked() -> Routes {
        Routes::new(vec![
            Route::new(
                "Post",
                "/orders",
                "App\\Orders::create",
                Some("Core\\Audience::Public".to_owned()),
                vec![],
            ),
            Route::new(
                "Post",
                "/hooks/stripe",
                "App\\Hooks::stripe",
                Some("Core\\Audience::Public".to_owned()),
                vec![],
            )
            .without_csrf(),
            Route::new(
                "Get",
                "/orders",
                "App\\Orders::index",
                Some("Core\\Audience::Public".to_owned()),
                vec![],
            ),
        ])
    }

    /// `rule:security/csrf-is-on-by-default`'s CSRF check and `rule:observability/route-label-is-the-path-pattern`'s `route` label, which are two
    /// of the three rules `rule:routing/matched-once-before-the-handler` was written for: each reads the match the
    /// door already made rather than making a second one.
    ///
    /// **The table is dropped before either is asked**, which is the half a
    /// shape-only assertion would miss — an answer that survives with nothing
    /// left to match against cannot have been re-derived. Both of § 4's answers
    /// are asserted over *one* verb, so a check that had read the request's
    /// method and stopped there passes neither: the opt-out row and the checked
    /// row are both `POST`.
    #[test]
    fn the_csrf_check_and_the_route_label_read_the_match_rather_than_matching_again() {
        let routes = checked();
        let matched = |verb: &str, path: &str| {
            let mut inbound = Inbound::new(verb, path, "");
            super::take(&routes, &mut inbound);
            inbound
        };
        let checked_post = matched("POST", "/orders");
        let exempt = matched("POST", "/hooks/stripe");
        let safe = matched("GET", "/orders");
        let unmatched = matched("POST", "/orders/9");
        drop(routes);

        assert!(super::csrf_required(&checked_post));
        // § 1a's opt-out, on the same verb as the row above it.
        assert!(!super::csrf_required(&exempt));
        assert!(!super::csrf_required(&safe));
        // `rule:routing/table-is-opt-in`: nothing matched declares no handler to protect.
        assert!(!super::csrf_required(&unmatched));

        assert_eq!(super::label(&checked_post), Some("/orders"));
        assert_eq!(super::label(&exempt), Some("/hooks/stripe"));
        assert_eq!(super::label(&safe), Some("/orders"));
        assert_eq!(super::label(&unmatched), None);
    }

    /// `rule:security/csrf-is-on-by-default`'s refusal with **nothing
    /// configured**: a checked write from another origin is refused, and the
    /// same write from this one is not.
    ///
    /// Asserted over one verb and one path again, so that a check which had read
    /// the method and stopped passes none of it. The opt-out row is in it
    /// because § 4's escape hatch is per route and has to survive the origin
    /// half as well as the token half — a webhook arrives cross-origin by
    /// definition.
    #[test]
    fn with_no_key_a_cross_origin_write_is_refused_and_a_same_origin_one_is_not() {
        let routes = checked();
        let arrived = |verb: &str, path: &str, headers: &[(&str, &str)]| {
            let mut inbound = Inbound::new(verb, path, "");
            for (name, value) in headers {
                inbound.push_header(name, value.as_bytes());
            }
            super::take(&routes, &mut inbound);
            inbound
        };
        let door = |inbound: &Inbound| super::csrf(inbound, None, "nvsid");

        let host = ("host", "app.example.com");
        let elsewhere = ("origin", "http://evil.example.com");
        let own = ("origin", "http://app.example.com");

        assert_eq!(
            door(&arrived("POST", "/orders", &[host, own])),
            Csrf::Passed
        );
        assert_eq!(
            door(&arrived("POST", "/orders", &[host, elsewhere])),
            Csrf::Refused
        );
        // A port and a scheme are each part of an origin, exactly as a browser
        // reads one.
        assert_eq!(
            door(&arrived(
                "POST",
                "/orders",
                &[host, ("origin", "https://app.example.com")]
            )),
            Csrf::Refused
        );
        assert_eq!(
            door(&arrived(
                "POST",
                "/orders",
                &[host, ("origin", "http://app.example.com:8080")]
            )),
            Csrf::Refused
        );
        // The bound the module doc states: no `Origin` is every non-browser
        // client, and the token half is what covers it.
        assert_eq!(door(&arrived("POST", "/orders", &[host])), Csrf::Passed);
        // § 1a's opt-out, and a safe verb, neither of which the door covers.
        assert_eq!(
            door(&arrived("POST", "/hooks/stripe", &[host, elsewhere])),
            Csrf::Passed
        );
        assert_eq!(
            door(&arrived("GET", "/orders", &[host, elsewhere])),
            Csrf::Passed
        );
    }

    /// Attack: another site posts to a program with no route table, or to a path
    /// a routed program's table does not claim. Each write that names another
    /// origin is refused, key or no key. A write with no `Origin`, a safe verb
    /// and an opted-out row still pass.
    #[test]
    fn a_cross_origin_write_nothing_matched_is_refused() {
        let routes = checked();
        let host = ("host", "app.example.com");
        let elsewhere = ("origin", "http://evil.example.com");
        let own = ("origin", "http://app.example.com");
        let arrived = |verb: &str, path: &str, routed: bool, headers: &[(&str, &str)]| {
            let mut inbound = Inbound::new(verb, path, "");
            for (name, value) in headers {
                inbound.push_header(name, value.as_bytes());
            }
            if routed {
                super::take(&routes, &mut inbound);
            }
            inbound
        };
        let key = [4_u8; nvs_runtime::csrf::KEY_LEN];
        for key in [None, Some(key.as_slice())] {
            let door = |inbound: &Inbound| super::csrf(inbound, key, "nvsid");
            for verb in ["POST", "PUT", "PATCH", "DELETE"] {
                // 1. A program with no table, and a path the table does not claim.
                assert_eq!(
                    door(&arrived(verb, "/p", false, &[host, elsewhere])),
                    Csrf::Refused,
                    "{verb} to a program with no table"
                );
                assert_eq!(
                    door(&arrived(verb, "/orders/9", true, &[host, elsewhere])),
                    Csrf::Refused,
                    "{verb} to a path the table does not claim"
                );
                // 2. The same write from this origin, or with no `Origin`, passes.
                assert_eq!(
                    door(&arrived(verb, "/p", false, &[host, own])),
                    Csrf::Passed
                );
                assert_eq!(door(&arrived(verb, "/p", false, &[host])), Csrf::Passed);
            }
            // 3. A safe verb, and the row that opted out, are not checked.
            assert_eq!(
                door(&arrived("GET", "/p", false, &[host, elsewhere])),
                Csrf::Passed
            );
            assert_eq!(
                door(&arrived("POST", "/hooks/stripe", true, &[host, elsewhere])),
                Csrf::Passed
            );
        }
    }

    /// The token half, armed by `[http] csrf_key`: a checked write carries a
    /// token this deployment issued for the session it rides under, or it is
    /// refused — from its own origin as much as from anywhere else.
    ///
    /// The token is built through `nvs_runtime::csrf`, which is the construction
    /// `Core\Csrf::issue` writes: what this pins is that the door reads the
    /// application's token and not a spelling of its own.
    #[test]
    fn with_a_key_a_write_carries_a_token_bound_to_its_session_or_is_refused() {
        let key = [4_u8; nvs_runtime::csrf::KEY_LEN];
        let keyed = nvs_runtime::csrf::Key::new(&key).expect("a 32-octet key keys");
        let token = keyed
            .issue(&[6_u8; nvs_runtime::csrf::NONCE_LEN], "sid-ada", "test")
            .expect("a short identifier seals");
        let elsewhere = keyed
            .issue(&[7_u8; nvs_runtime::csrf::NONCE_LEN], "sid-grace", "test")
            .expect("a short identifier seals");

        let routes = checked();
        let arrived = |headers: &[(&str, &str)]| {
            let mut inbound = Inbound::new("POST", "/orders", "");
            for (name, value) in headers {
                inbound.push_header(name, value.as_bytes());
            }
            super::take(&routes, &mut inbound);
            inbound
        };
        let door = |inbound: &Inbound| super::csrf(inbound, Some(key.as_slice()), "nvsid");

        let host = ("host", "app.example.com");
        let own = ("origin", "http://app.example.com");
        let riding = ("cookie", "theme=dark; nvsid=sid-ada");

        assert_eq!(
            door(&arrived(&[host, own, riding, ("x-csrf-token", &token)])),
            Csrf::Passed
        );
        // A token proves the page was this application's, so it passes from
        // anywhere — which is what a rendered form posting to its own API does.
        assert_eq!(
            door(&arrived(&[
                host,
                ("origin", "http://cdn.example.com"),
                riding,
                ("x-csrf-token", &token),
            ])),
            Csrf::Passed
        );
        // Bound to the session it rides under, and the header is read without
        // regard to case because a peer chooses the spelling.
        assert_eq!(
            door(&arrived(&[host, own, riding, ("X-CSRF-Token", &token)])),
            Csrf::Passed
        );
        assert_eq!(
            door(&arrived(&[host, own, riding, ("x-csrf-token", &elsewhere)])),
            Csrf::Refused
        );
        assert_eq!(
            door(&arrived(&[
                host,
                own,
                ("cookie", "nvsid=sid-grace"),
                ("x-csrf-token", &token),
            ])),
            Csrf::Refused
        );
        // A key was named, so a same-origin write with no token is refused too:
        // that is the difference between the two halves being armed.
        assert_eq!(door(&arrived(&[host, own, riding])), Csrf::Refused);
        assert_eq!(
            door(&arrived(&[host, own, riding, ("x-csrf-token", "sid-ada")])),
            Csrf::Refused
        );
    }

    /// Attack: a page shown to a visitor with no session issues a token for no
    /// session, and anyone can load that page. A keyed door refuses that token,
    /// from this origin and from any other, with or without a cookie.
    #[test]
    fn a_token_issued_for_no_session_is_refused_from_any_origin() {
        let key = [4_u8; nvs_runtime::csrf::KEY_LEN];
        let keyed = nvs_runtime::csrf::Key::new(&key).expect("a 32-octet key keys");
        let nobody = keyed
            .issue(&[8_u8; nvs_runtime::csrf::NONCE_LEN], "", "test")
            .expect("an empty identifier seals");
        let routes = checked();
        let arrived = |headers: &[(&str, &str)]| {
            let mut inbound = Inbound::new("POST", "/orders", "");
            for (name, value) in headers {
                inbound.push_header(name, value.as_bytes());
            }
            super::take(&routes, &mut inbound);
            inbound
        };
        let door = |inbound: &Inbound| super::csrf(inbound, Some(key.as_slice()), "nvsid");
        let host = ("host", "app.example.com");
        let token = ("x-csrf-token", nobody.as_str());

        // 1. No cookie, from this origin, from another one, and with no `Origin`.
        assert_eq!(
            door(&arrived(&[
                host,
                ("origin", "http://app.example.com"),
                token
            ])),
            Csrf::Refused
        );
        assert_eq!(
            door(&arrived(&[
                host,
                ("origin", "http://evil.example.com"),
                token
            ])),
            Csrf::Refused
        );
        assert_eq!(door(&arrived(&[host, token])), Csrf::Refused);
        // 2. A session cookie with an empty value is no session either.
        assert_eq!(
            door(&arrived(&[host, ("cookie", "nvsid="), token])),
            Csrf::Refused
        );
    }

    /// Attack: a signed-in visitor's session cookie arrives on a second `Cookie`
    /// line, or beside a cookie with a byte that is not UTF-8. The door must find
    /// the session `Core\Session::start` finds, so a token for no session is
    /// refused and the visitor's own token passes.
    #[test]
    fn the_door_finds_the_session_wherever_the_session_reader_finds_it() {
        let key = [4_u8; nvs_runtime::csrf::KEY_LEN];
        let keyed = nvs_runtime::csrf::Key::new(&key).expect("a 32-octet key keys");
        let nobody = keyed
            .issue(&[8_u8; nvs_runtime::csrf::NONCE_LEN], "", "test")
            .expect("an empty identifier seals");
        let own = keyed
            .issue(&[9_u8; nvs_runtime::csrf::NONCE_LEN], "sid-ada", "test")
            .expect("a short identifier seals");
        let routes = checked();
        let arrived = |headers: &[(&str, &[u8])]| {
            let mut inbound = Inbound::new("POST", "/orders", "");
            for (name, value) in headers {
                inbound.push_header(name, value);
            }
            super::take(&routes, &mut inbound);
            inbound
        };
        let door = |inbound: &Inbound| super::csrf(inbound, Some(key.as_slice()), "nvsid");
        let host: (&str, &[u8]) = ("host", b"app.example.com");
        let second_line: [(&str, &[u8]); 2] =
            [("cookie", b"theme=dark"), ("cookie", b"nvsid=sid-ada")];
        let planted: (&str, &[u8]) = ("cookie", b"junk=\xff\xfe; nvsid=sid-ada");

        for cookies in [&second_line[..], &[planted][..]] {
            let with = |token: &str| {
                let mut headers = vec![host, ("x-csrf-token", token.as_bytes())];
                headers.extend_from_slice(cookies);
                door(&arrived(&headers))
            };
            assert_eq!(with(&nobody), Csrf::Refused);
            assert_eq!(with(&own), Csrf::Passed);
        }
    }

    /// `token` as a form or a query writes it: `+`, `/` and `=` escaped.
    fn escaped(token: &str) -> String {
        token
            .replace('+', "%2B")
            .replace('/', "%2F")
            .replace('=', "%3D")
    }

    /// The form field and the query parameter: an urlencoded write with no
    /// header is left to its body, a multipart one is never read and passes
    /// with the header or the `_csrf` query parameter, and the body's field
    /// passes, a wrong one is refused and a body past the limit is refused.
    #[test]
    fn a_form_field_or_a_query_parameter_carries_the_token_and_a_multipart_body_is_never_read() {
        let key = [4_u8; nvs_runtime::csrf::KEY_LEN];
        let keyed = nvs_runtime::csrf::Key::new(&key).expect("a 32-octet key keys");
        let token = keyed
            .issue(&[6_u8; nvs_runtime::csrf::NONCE_LEN], "sid-ada", "test")
            .expect("a short identifier seals");
        let elsewhere = keyed
            .issue(&[7_u8; nvs_runtime::csrf::NONCE_LEN], "sid-grace", "test")
            .expect("a short identifier seals");

        let routes = checked();
        let arrived = |query: &str, headers: &[(&str, &str)]| {
            let mut inbound = Inbound::new("POST", "/orders", query);
            for (name, value) in headers {
                inbound.push_header(name, value.as_bytes());
            }
            super::take(&routes, &mut inbound);
            inbound
        };
        let door = |inbound: &Inbound| super::csrf(inbound, Some(key.as_slice()), "nvsid");

        let host = ("host", "app.example.com");
        let riding = ("cookie", "nvsid=sid-ada");
        let form = (
            "content-type",
            "Application/X-WWW-Form-Urlencoded; charset=UTF-8",
        );
        let upload = ("content-type", "multipart/form-data; boundary=x");
        let query = format!("page=2&_csrf={}", escaped(&token));

        assert_eq!(door(&arrived("", &[host, riding, form])), Csrf::InBody);
        assert_eq!(door(&arrived("", &[host, riding, upload])), Csrf::Refused);
        assert_eq!(
            door(&arrived(
                "",
                &[host, riding, upload, ("x-csrf-token", &token)]
            )),
            Csrf::Passed
        );
        assert_eq!(
            door(&arrived(&query, &[host, riding, upload])),
            Csrf::Passed
        );
        assert_eq!(
            door(&arrived(
                &format!("_csrf={}", escaped(&elsewhere)),
                &[host, riding, form]
            )),
            Csrf::Refused
        );

        let body = |content: String, limit: usize| {
            let mut inbound = arrived("", &[host, riding, form]);
            inbound.set_buffered_body(content.into_bytes());
            let verdict = super::csrf_in_body(&mut inbound, Some(key.as_slice()), "nvsid", limit);
            (verdict, inbound)
        };
        let content = format!("name=Ada+Lovelace&_csrf={}", escaped(&token));
        let (verdict, mut inbound) = body(content.clone(), 4096);
        assert_eq!(verdict, Csrf::Passed);
        // The form is still whole for the handler that reads it next.
        let left = inbound
            .body()
            .expect("the body is back on the carrier")
            .next_chunk()
            .expect("an in-memory body reads")
            .map(<[u8]>::to_vec);
        assert_eq!(left, Some(content.clone().into_bytes()));
        assert_eq!(
            body(format!("_csrf={}", escaped(&elsewhere)), 4096).0,
            Csrf::Refused
        );
        assert_eq!(body("name=Ada".to_owned(), 4096).0, Csrf::Refused);
        assert_eq!(body(content, 16).0, Csrf::Refused);
    }

    /// A refused form never reaches the handler: the request answers `403`
    /// with nothing run, or, for a unit that dispatches, runs with the
    /// refusal on the carrier and no body.
    #[test]
    fn a_refused_form_is_answered_before_the_handler_runs() {
        let key = [4_u8; nvs_runtime::csrf::KEY_LEN];
        let run = |refusal_mode: bool| {
            let mut inbound = Inbound::new("POST", "/orders", "");
            inbound.push_header("content-type", b"application/x-www-form-urlencoded");
            inbound.set_buffered_body(b"_csrf=forged".to_vec());
            let mut ctx = nvs_runtime::Ctx::new(nvs_runtime::OutputSink::Sink);
            ctx.set_inbound(inbound);
            let ran = std::rc::Rc::new(std::cell::Cell::new(None));
            let seen = std::rc::Rc::clone(&ran);
            let program = super::reading_the_form(
                Box::new(move |ctx: &mut nvs_runtime::Ctx, _args| {
                    let inbound = ctx.inbound().expect("the request is on the context");
                    seen.set(Some((inbound.refusal(), inbound.has_body())));
                    nvs_runtime::Value::null()
                }),
                key.to_vec(),
                "nvsid".to_owned(),
                refusal_mode,
            );
            program(&mut ctx, nvs_runtime::Value::null());
            (ran.get(), ctx.take_status())
        };
        assert_eq!(run(false), (None, Some(403)));
        assert_eq!(
            run(true),
            (
                Some((Some(nvs_runtime::routes::Refusal::Csrf), false)),
                None
            )
        );
    }

    /// The upgrade half: a `GET` that asks to upgrade is refused from another
    /// origin and passes from its own or with no `Origin`, a row that wrote
    /// `csrf: false` is not checked, and a key does not make a token owed —
    /// while an upgrade header on a checked write still owes its token.
    #[test]
    fn a_cross_origin_upgrade_is_refused_unless_its_route_opted_out() {
        let routes = Routes::new(vec![
            Route::new("Get", "/chat", "App\\Chat::open", None, vec![]),
            Route::new("Get", "/feed", "App\\Feed::open", None, vec![]).without_csrf(),
            Route::new("Post", "/orders", "App\\Orders::create", None, vec![]),
        ]);
        let arrived = |verb: &str, path: &str, headers: &[(&str, &str)]| {
            let mut inbound = Inbound::new(verb, path, "");
            for (name, value) in headers {
                inbound.push_header(name, value.as_bytes());
            }
            super::take(&routes, &mut inbound);
            inbound
        };
        let key = [4_u8; nvs_runtime::csrf::KEY_LEN];
        let unkeyed = |inbound: &Inbound| super::csrf(inbound, None, "nvsid");
        let keyed = |inbound: &Inbound| super::csrf(inbound, Some(key.as_slice()), "nvsid");

        let host = ("host", "app.example.com");
        let upgrade = ("Upgrade", "websocket");
        let elsewhere = ("origin", "http://evil.example.com");
        let own = ("origin", "http://app.example.com");

        for door in [&unkeyed as &dyn Fn(&Inbound) -> Csrf, &keyed] {
            assert_eq!(
                door(&arrived("GET", "/chat", &[host, upgrade, elsewhere])),
                Csrf::Refused
            );
            assert_eq!(
                door(&arrived("GET", "/chat", &[host, upgrade, own])),
                Csrf::Passed
            );
            assert_eq!(
                door(&arrived("GET", "/chat", &[host, upgrade])),
                Csrf::Passed
            );
            // The opt-out, and the same request with no upgrade header, which
            // is an ordinary `GET` the door does not cover.
            assert_eq!(
                door(&arrived("GET", "/feed", &[host, upgrade, elsewhere])),
                Csrf::Passed
            );
            assert_eq!(
                door(&arrived("GET", "/chat", &[host, elsewhere])),
                Csrf::Passed
            );
            // Nothing matched, so there is no route to have opted out.
            assert_eq!(
                door(&arrived("GET", "/elsewhere", &[host, upgrade, elsewhere])),
                Csrf::Passed
            );
        }
        // An upgrade header on a checked write takes nothing off its token.
        assert_eq!(
            keyed(&arrived("POST", "/orders", &[host, upgrade, own])),
            Csrf::Refused
        );
    }

    /// `rule:observability/route-label-is-the-path-pattern`'s label carries the route's **path pattern** and never
    /// the request's path, so two requests to one route share one series.
    #[test]
    fn a_matched_route_is_labelled_with_its_pattern_and_not_the_requests_path() {
        let routes = table();
        let label = |path: &str| {
            let mut inbound = Inbound::new("GET", path, "");
            super::take(&routes, &mut inbound);
            super::label(&inbound).map(str::to_owned)
        };

        assert_eq!(label("/users/42").as_deref(), Some("/users/{id}"));
        assert_eq!(label("/users/7").as_deref(), Some("/users/{id}"));
        assert_eq!(label("/users/new").as_deref(), Some("/users/new"));
    }

    /// § 1's `null`: nothing matched is a served request like any other, and
    /// the door writes nothing onto it.
    #[test]
    fn a_request_nothing_claims_carries_no_match() {
        let routes = table();
        let mut inbound = Inbound::new("DELETE", "/users/42", "");
        super::take(&routes, &mut inbound);
        assert!(inbound.route().is_none());

        // And a path outside the table, under a verb the table does declare.
        let mut inbound = Inbound::new("GET", "/orders/42", "");
        super::take(&routes, &mut inbound);
        assert!(inbound.route().is_none());
    }

    /// The path matched is `rule:http-server/a-request-resolves-in-five-steps` step 2's remainder — the one the
    /// carrier holds — so a mounted application's routes match under any
    /// prefix, and the prefixed path matches nothing.
    #[test]
    fn the_match_is_taken_against_the_mount_stripped_path() {
        let routes = table();
        let mut stripped = Inbound::new("GET", "/users/new", "");
        super::take(&routes, &mut stripped);
        assert_eq!(
            stripped.route().map(|matched| matched.route().handler()),
            Some("App\\Users::new")
        );

        let mut prefixed = Inbound::new("GET", "/shop/users/new", "");
        super::take(&routes, &mut prefixed);
        assert!(prefixed.route().is_none());
    }
}
