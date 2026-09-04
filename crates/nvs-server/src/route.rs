//! [ADR 0102](../../../docs/adr/0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md)
//! § 1's match, taken at the door: the compiled unit's route table against the
//! request that arrived, once, before any application code runs.
//!
//! # Why the door and not the program
//!
//! § 1 names three rules that were already written against a match the server
//! had not got — ADR 0096 § 4's CSRF check, ADR 0076 § 1's `route` label, and
//! § 8's access decision — and each of them needs the answer *before* the
//! handler is called. A program that matched for itself would make two matches
//! of one question and could not have answered the first three at all, so the
//! match happens here and rides on the request:
//! [`nvs_runtime::Inbound::set_route`] is where it is written and
//! `Core\Request::route()` is the one reader.
//!
//! **This module dispatches nothing.** It computes a name and typed parameters
//! and stops, which is § 1's own second rule and ADR 0077 § 4's refusal list
//! left untouched: nothing here calls the annotated method, and nothing here
//! decides what a return value means.
//!
//! # Where it sits among the door's other decisions
//!
//! Later than the rest of them, and necessarily. The peer walk
//! ([`crate::forwarded`]), the ceiling ([`crate::admit`]) and the cross-origin
//! answer ([`crate::cors`]) are all decided from configuration the boot
//! resolved, so [`crate::serve`] takes them on the connection before a mount is
//! even selected. A match needs the *unit*, and which unit answers a request is
//! ADR 0097 § 4's five steps — so this runs where the selected program and the
//! arrived request are both in hand, which is the last point before the isolate
//! is built. It is still "before the handler" in § 1's sense: no application
//! code has run.
//!
//! A request whose unit declares no route is matched against nothing and
//! carries no match, which is ADR 0077 § 5's opt-in rule; so is one whose
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
/// stripped** one — ADR 0097 § 4 step 2's remainder, which
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

    /// ADR 0102 § 1: the match is taken before the handler, it is taken once,
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

    /// The path matched is ADR 0097 § 4 step 2's remainder — the one the
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
