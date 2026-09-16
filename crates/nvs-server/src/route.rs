//! `rule:routing/matched-once-before-the-handler`
//! 's match, taken at the door: the compiled unit's route table against the
//! request that arrived, once, before any application code runs.
//!
//! # Why the door and not the program
//!
//! § 1 names three rules that were already written against a match the server
//! had not got — `rule:security/csrf-is-on-by-default`'s CSRF check, `rule:observability/route-label-is-the-declared-name`'s `route` label, and
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
//! refusal, over [`csrf_required`]'s question, and [`label`] is `rule:observability/route-label-is-the-declared-name`'s `route` label. Neither matches
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
//! session is the value of the cookie `[session] cookie` names, and
//! [`TOKEN_HEADER`] is where the door looks. A valid token passes whatever the
//! origin says, because it proves the request came from a page this application
//! rendered.
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
//! that did not happen.
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
/// **A request that matched nothing is not covered**, and that is `rule:routing/table-is-opt-in`
/// 's opt-in rule rather than a hole in the default: a program with no route
/// table declares no handler for the door to protect, so refusing its every
/// `POST` would refuse every request it serves.
///
/// The refusal this answer feeds is [`csrf`], which is where § 4's other half —
/// what makes a covered request acceptable — is decided.
#[must_use]
pub fn csrf_required(inbound: &Inbound) -> bool {
    inbound
        .route()
        .is_some_and(|matched| matched.route().csrf())
}

/// Where the door looks for a presented token.
///
/// A header and not a form field, because the body is the one part of a request
/// the door deliberately does not read: it crosses as a stream the isolate pulls
/// (`rule:http-server/request-body-and-upload-total-are-two-caps`), and a door
/// that parsed a form to find a token would be buffering every unsafe request
/// before deciding whether to run it — at the front of the server, on the
/// attacker's terms. The spelling is the one every browser-side helper already
/// writes, so `fetch` and a rendered form's script need nothing new.
pub const TOKEN_HEADER: &str = "x-csrf-token";

/// The cookie header, whose value carries the session identifier a token binds.
const COOKIE_HEADER: &str = "cookie";

/// The header naming where a checked request says it came from.
const ORIGIN_HEADER: &str = "origin";

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
/// ([`csrf_required`] is the whole of that question); a request carrying a token
/// this key issued for this request's session passes, whatever its origin says;
/// a request whose `Origin` is not where it arrived is refused; and, where a key
/// is named, a request with no such token is refused even same-origin, because a
/// deployment that named a key asked for its unsafe verbs to carry one.
#[must_use]
pub fn csrf(inbound: &Inbound, key: Option<&[u8]>, session_cookie: &str) -> Csrf {
    if !csrf_required(inbound) {
        return Csrf::Passed;
    }
    // Keyed first, so that a valid token is the one answer that does not depend
    // on a header the request chose: a token proves the page it was rendered
    // into is this application's, which is strictly more than an origin says.
    if let Some(key) = key.and_then(nvs_runtime::csrf::Key::new) {
        let session = cookie(inbound, session_cookie).unwrap_or("");
        let presented = header(inbound, TOKEN_HEADER)
            .and_then(|value| std::str::from_utf8(value).ok())
            .unwrap_or("");
        return if key.verify(presented, session) {
            Csrf::Passed
        } else {
            Csrf::Refused
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
/// the door asks for at most three, and `nvs_runtime::Inbound` keeps headers in
/// the spelling and the order the peer sent them on purpose.
fn header<'a>(inbound: &'a Inbound, name: &str) -> Option<&'a [u8]> {
    inbound
        .headers()
        .find(|(field, _)| field.eq_ignore_ascii_case(name))
        .map(|(_, value)| value)
}

/// The value of the cookie `name`, out of whatever `Cookie` lines the request
/// carried.
///
/// The name is matched exactly, which is what RFC 6265 § 4.1.1 says a cookie name
/// is, and a value that is not UTF-8 answers `None` rather than a lossy string —
/// what it is compared against is a session identifier this server minted, and
/// that is ASCII.
fn cookie<'a>(inbound: &'a Inbound, name: &str) -> Option<&'a str> {
    let line = header(inbound, COOKIE_HEADER)?;
    std::str::from_utf8(line).ok().and_then(|line| {
        line.split(';').find_map(|pair| {
            let (found, value) = pair.split_once('=')?;
            (found.trim() == name).then_some(value)
        })
    })
}

/// `rule:observability/route-label-is-the-declared-name`'s `route` label for this request: the matched route's
/// **declared name**.
///
/// The one label of that table which would otherwise be unbounded, which is why
/// it is read off the match and computed nowhere else — its value comes from
/// the compile-time table, a closed set, and never from the request's own path,
/// whose cardinality the client chooses.
///
/// `None` in all three of the absences, which are one case: nothing matched, no
/// table to match against, and a matched row that declared no `name`. A label
/// is a name the table chose, so a row that chose none has no label — falling
/// back to the path is the cardinality bomb the rule exists to prevent, and it
/// is not offered as an option.
///
/// [`crate::serve`] is the caller, once per request it ran, off the carrier the
/// reply is still holding — `nvs_host::Isolate::answering_request` owns why the
/// door reads it there and not after the isolate has started.
#[must_use]
pub fn label(inbound: &Inbound) -> Option<&str> {
    inbound.route()?.name()
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
                Some("user.show".to_owned()),
                "App\\Users::show",
                Some("Core\\Audience::Public".to_owned()),
                vec![Capture {
                    name: "id".to_owned(),
                    conv: CaptureConv::Uint,
                }],
            ),
            Route::new("Get", "/users/new", None, "App\\Users::new", None, vec![]),
        ])
    }

    /// What a handler is handed: the carrier by value, exactly as
    /// [`crate::serve`]'s does, so that what the assertions read is what an
    /// application would read and not a carrier the door still owns.
    fn handler(inbound: Inbound) -> Option<(String, Vec<(String, Param)>)> {
        let matched = inbound.route()?;
        Some((matched.name()?.to_owned(), matched.params().to_vec()))
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
                "user.show".to_owned(),
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
                Some("orders.create".to_owned()),
                "App\\Orders::create",
                Some("Core\\Audience::Public".to_owned()),
                vec![],
            ),
            Route::new(
                "Post",
                "/hooks/stripe",
                Some("hooks.stripe".to_owned()),
                "App\\Hooks::stripe",
                Some("Core\\Audience::Public".to_owned()),
                vec![],
            )
            .without_csrf(),
            Route::new(
                "Get",
                "/orders",
                Some("orders.index".to_owned()),
                "App\\Orders::index",
                Some("Core\\Audience::Public".to_owned()),
                vec![],
            ),
        ])
    }

    /// `rule:security/csrf-is-on-by-default`'s CSRF check and `rule:observability/route-label-is-the-declared-name`'s `route` label, which are two
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

        assert_eq!(super::label(&checked_post), Some("orders.create"));
        assert_eq!(super::label(&exempt), Some("hooks.stripe"));
        assert_eq!(super::label(&safe), Some("orders.index"));
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

    /// `rule:observability/route-label-is-the-declared-name`'s label carries the route's **declared name** and never the
    /// request's path, so a matched row that declared none has no label — the
    /// case a fallback would silently turn into the cardinality bomb that rule
    /// exists to prevent.
    #[test]
    fn a_matched_route_with_no_declared_name_has_no_label() {
        let routes = table();
        let mut inbound = Inbound::new("GET", "/users/new", "");
        super::take(&routes, &mut inbound);
        drop(routes);

        assert!(inbound.route().is_some());
        assert_eq!(super::label(&inbound), None);
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
