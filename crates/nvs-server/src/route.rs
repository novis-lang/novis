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
//! # What the answer is read for, and what is still missing
//!
//! Two of § 1's three rules read it here: [`csrf_required`] is `rule:security/csrf-is-on-by-default`'s
//! question and [`label`] is `rule:observability/route-label-is-the-declared-name`'s `route` label. Neither matches
//! anything — each is a field of the row the door already found — which is what
//! § 1 buys and is why they live beside [`take`] rather than beside the
//! subsystem each belongs to. § 8's access decision has no reader here on
//! purpose: it is the *dispatcher's*, and this crate does not dispatch.
//!
//! **Known gap 1: the door answers whether a request is CSRF-checked and does
//! not yet refuse one.** § 4's refusal needs the presented token *verified*,
//! and verification is `rule:security/protocol-roster`'s constant-time comparison against a key bound
//! to the session that issued the token — of which this crate has neither half
//! in reach. `Core\Csrf::verify` is `nvs-stdlib`'s, a crate above this one and
//! deliberately not a dependency of it, and no `[http]` directive names a key
//! for the door to verify against. Closing it is therefore a configuration
//! decision and a seam, not an omission here: what this module owns is which
//! requests the check covers, and that is landed.
//! Decided: At the server door, before an isolate runs — Move token verification into a crate nvs-
//! server can reach and add an [http] key naming the key; the fastest and smallest attack surface, at
//! the cost of one config key and a crate move.
//! — owner: unowned-closures
//!
//! **Known gap 2: the label has a consumer and no caller.**
//! [`crate::metrics::Registry::request`] is what `rule:observability/route-label-is-the-declared-name`'s `route` label
//! reaches — the two request series carry it, and that member's doc owns what an
//! unmatched request's label is — but no core owns a registry yet, because § 8's
//! two exporters are not in this crate's graph and nothing would read one. So
//! [`label`] still has no caller, and it is written here for the reason it
//! always was: the *value* is the part that rule interlocks with `rule:routing/routes-are-compiled-not-registered` over —
//! a name out of the compile-time table and never the request's path — and
//! deriving it anywhere else would be the second match § 1 removes.
//! — owner: unowned
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
/// The refusal itself is this module's known gap 1, above.
#[must_use]
pub fn csrf_required(inbound: &Inbound) -> bool {
    inbound
        .route()
        .is_some_and(|matched| matched.route().csrf())
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
#[must_use]
pub fn label(inbound: &Inbound) -> Option<&str> {
    inbound.route()?.name()
}

#[cfg(test)]
mod tests {
    use nvs_runtime::Inbound;
    use nvs_runtime::routes::{Capture, CaptureConv, Param, Route, Routes};

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
