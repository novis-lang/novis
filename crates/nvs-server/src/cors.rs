//! [ADR 0074] § 2's cross-origin policy: what a request from another origin is answered with
//! while `[http.cors] origins` names nobody.
//!
//! § 2's shipped default is `origins = []`, and closed has two observable halves: **no CORS
//! header is emitted at all**, and **a preflight is answered `403`**. They are kept here in
//! two different ways, on purpose.
//!
//! The first is kept by there being nothing in this crate that writes such a header. A policy
//! that can emit `Access-Control-Allow-Origin` before it has a list to match an origin against
//! is the failure § 2 exists to prevent, so the emitting half is written when there is
//! something to match and not one release earlier;
//! [`crate::serve`]'s `cors_is_closed_with_nothing_configured` is that half asserted over the
//! wire.
//!
//! The second is a decision, and [`Cors::preflight`] is it: an `OPTIONS` carrying
//! `Access-Control-Request-Method` is a browser asking permission, and a server that has named
//! no origin has none to give. **`403` and not `405`**, because the verb is one this server
//! implements and answers elsewhere — what is refused is the *origin*, and saying `405` would
//! tell a browser to stop asking about a method rather than that nobody may cross.
//!
//! **It is refused before the handler**, beside ADR 0097 § 5's valve, so a preflight nobody
//! configured selects no mount, allocates no isolate and runs no Novis code. That is also what
//! keeps it honest: an application asked to answer an `OPTIONS` would be answering a question
//! the policy above it had already decided, and the two could disagree.
//!
//! # What is not here yet
//!
//! The open half — an `Origin` matched against a named list, `Access-Control-Allow-Origin` and
//! `Vary: Origin` on the answer, and a preflight answered with the methods, headers, `expose`
//! set and `max_age` § 2 configures. Until it lands a tree that *did* name origins has its
//! preflights handled as any other request, which is the direction that leaks nothing.
//!
//! § 2's other rule is not this module's: `origins = ["*"]` together with `credentials = true`
//! is refused at boot and by `Core\Config::set`, both through `nvs_config::http`, because it is
//! a question about a written configuration rather than about a request.
//!
//! [ADR 0074]: ../../../docs/adr/0074-http-defaults-safe-and-finite.md

use hyper::header::ACCESS_CONTROL_REQUEST_METHOD;
use hyper::{HeaderMap, Method, StatusCode};
use nvs_config::tree::Http;

/// [ADR 0074] § 2's policy, resolved from `[http.cors]` at boot.
///
/// Boot-fixed and process-wide for the reason [`crate::secure::Secure`] is: § 2's block is
/// `Runtime`-class in the ADR's own table, and nothing in this milestone re-reads it under a
/// live socket.
///
/// [ADR 0074]: ../../../docs/adr/0074-http-defaults-safe-and-finite.md
#[derive(Clone, Debug)]
pub struct Cors {
    /// The exact origins `[http.cors] origins` named, as written. **Empty is the whole of what
    /// "closed" means**, and it is what a tree that wrote no `[http.cors]` resolves to.
    origins: Vec<String>,
}

/// § 2's shipped policy, which is what a tree writing no `[http.cors]` resolves to.
impl Default for Cors {
    fn default() -> Self {
        Self::of(None)
    }
}

impl Cors {
    /// The policy in force for a tree.
    ///
    /// Takes the whole `[http]` block rather than `[http.cors]`, so that a caller holding a
    /// [`nvs_config::Config`] passes `config.http.as_ref()` and never has to know which of the
    /// five sub-blocks this reads — the same argument as [`crate::secure::Secure::of`]'s.
    #[must_use]
    pub fn of(http: Option<&Http>) -> Self {
        Self {
            origins: http
                .and_then(|http| http.cors.as_ref())
                .and_then(|cors| cors.origins.clone())
                .unwrap_or_default(),
        }
    }

    /// § 2's answer to a preflight nobody configured, or [`None`] where this request is not a
    /// preflight — or where an origin has been named and the module doc's unwritten half owns
    /// the answer.
    ///
    /// `Access-Control-Request-Method` is what makes an `OPTIONS` a preflight rather than a
    /// plain one, and it is the header this reads for that reason: a program may answer an
    /// ordinary `OPTIONS` — that is what `Allow` is for — and refusing every one of them would
    /// take a method away in order to close a door it never went through.
    #[must_use]
    pub fn preflight(&self, method: &Method, headers: &HeaderMap) -> Option<StatusCode> {
        if !self.origins.is_empty() {
            return None;
        }
        (method == Method::OPTIONS && headers.contains_key(ACCESS_CONTROL_REQUEST_METHOD))
            .then_some(StatusCode::FORBIDDEN)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::header::{HeaderValue, ORIGIN};

    /// The headers a browser's preflight carries: the origin it is asking on behalf of, and
    /// the method it wants permission for.
    fn asking() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            ORIGIN,
            HeaderValue::from_static("https://elsewhere.example"),
        );
        headers.insert(
            ACCESS_CONTROL_REQUEST_METHOD,
            HeaderValue::from_static("POST"),
        );
        headers
    }

    /// § 2's second half: with `origins = []` there is no permission to give.
    #[test]
    fn a_preflight_nobody_configured_is_refused() {
        assert_eq!(
            Cors::default().preflight(&Method::OPTIONS, &asking()),
            Some(StatusCode::FORBIDDEN),
            "a preflight was passed on with `[http.cors] origins` naming nobody"
        );
    }

    /// An `OPTIONS` that asks for no method is not a preflight, so the closed policy has
    /// nothing to say about it and the request is the program's to answer.
    #[test]
    fn a_plain_options_is_not_a_preflight() {
        let mut headers = asking();
        headers.remove(ACCESS_CONTROL_REQUEST_METHOD);
        assert_eq!(Cors::default().preflight(&Method::OPTIONS, &headers), None);
    }

    /// The header that decides is only read on `OPTIONS`: a `GET` from another origin is an
    /// ordinary request, answered as one and — § 2's first half — told nothing.
    #[test]
    fn a_cross_origin_get_is_not_a_preflight() {
        assert_eq!(Cors::default().preflight(&Method::GET, &asking()), None);
    }
}
