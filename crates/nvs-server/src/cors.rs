//! [ADR 0074] § 2's cross-origin policy: which origins `[http.cors] origins` names, and what a
//! request from one of them — or from anybody else — is answered with.
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
//! # An origin that *is* named
//!
//! A list with something in it makes the policy open, and then every answer this server writes
//! is one the policy had a say in. [`Cors::answer`] decides what that say is while the request
//! is still in hand — the handler takes the request by value — and [`Crossing`] carries the
//! decision to the end of the response, which is why the door never has to know that `Origin`
//! is the header that matters.
//!
//! **An origin is matched exactly, byte for byte, against what the operator wrote.** No case
//! folding, no default-port equivalence, no trailing slash forgiven: `https://a.example` and
//! `https://a.example:443` denote the same server and are *not* the same origin, and treating
//! them as one would be [ADR 0095]'s repair — inventing an equivalence the operator did not
//! write, in the one place where being generous hands a cross-origin read to somebody who was
//! not named. A browser sends the serialization RFC 6454 § 6.1 specifies, so an exact match is
//! also the one that works.
//!
//! **`Vary: Origin` goes on every answer an open policy touched, including the ones carrying no
//! `Access-Control-Allow-Origin`** — the refused origin's and the request that sent no `Origin`
//! at all. The header is not about this response; it is about the *next* reader of a cache that
//! stored it. An answer cached under the URL alone is served to whoever asks for that URL, so
//! one origin's `Access-Control-Allow-Origin` reaches another origin's browser, or — the
//! commoner and quieter failure — the header-less variant is stored first and the named origin
//! is broken by a cache that is working exactly as it was told to. Varying costs a cache entry
//! per origin and buys the difference between those two outcomes and neither.
//!
//! A wildcard list is the exception, and it is the honest one: `origins = ["*"]` answers `*` to
//! everybody, so nothing about that answer depends on who asked and saying it varies would be
//! false. § 2 permits the wildcard only beside `credentials = false`, which `nvs_config::http`
//! is where that is refused.
//!
//! # What is not here yet
//!
//! The preflight a named origin gets: the methods, headers, `expose` set and `max_age` § 2
//! configures. [`Cors::preflight`] stands aside under an open policy until that lands, so such
//! a preflight is handled as any other request — the direction that leaks nothing.
//!
//! § 2's other rule is not this module's: `origins = ["*"]` together with `credentials = true`
//! is refused at boot and by `Core\Config::set`, both through `nvs_config::http`, because it is
//! a question about a written configuration rather than about a request.
//!
//! [ADR 0074]: ../../../docs/adr/0074-http-defaults-safe-and-finite.md
//! [ADR 0095]: ../../../docs/adr/0095-ambiguous-input-is-refused-never-repaired.md

use hyper::header::{
    ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_REQUEST_METHOD, HeaderValue, ORIGIN, VARY,
};
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

/// What § 2 has to say about one request's answer, decided before the handler ran.
///
/// The door computes this while the request is in hand and writes it once the answer exists,
/// because [`crate::serve`]'s handler takes the request by value. Carrying the decision rather
/// than the request's headers is also what keeps the door from knowing which header the policy
/// reads; the module doc's second section owns why that matters more here than the two words it
/// saves.
///
/// [`Default`] is the closed policy's answer: nothing written, nothing varied.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Crossing {
    /// What `Access-Control-Allow-Origin` says, or [`None`] where this answer carries none —
    /// a closed policy, or an origin the list does not name.
    allow: Option<HeaderValue>,
    /// Whether this answer depends on the request's `Origin`. True for every answer an open
    /// list of named origins produces, `allow` or no `allow`, and false for the wildcard.
    varies: bool,
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

    /// § 2's answer for one request, read off the headers it arrived with.
    ///
    /// A closed policy answers [`Crossing::default`] — nothing at all, which is § 2's first
    /// half and the reason this returns a value rather than writing one: there is a case where
    /// the right thing to write is nothing, and a writer is easy to call unconditionally.
    #[must_use]
    pub fn answer(&self, request: &HeaderMap) -> Crossing {
        if self.origins.is_empty() {
            return Crossing::default();
        }
        // The module doc's exception: an answer of `*` is the same answer for everybody, so it
        // does not vary. Any list containing `*` is that list — a wildcard beside named origins
        // already permits each of them, and reading it as anything narrower would be answering
        // less than what was written.
        if self.origins.iter().any(|named| named == "*") {
            return Crossing {
                allow: Some(HeaderValue::from_static("*")),
                varies: false,
            };
        }
        Crossing {
            allow: self.named(request),
            varies: true,
        }
    }

    /// The request's own `Origin`, where the list names it exactly, as the value to echo back.
    ///
    /// Echoing the request's bytes rather than the configured string is not a choice between
    /// two spellings: an exact match makes them the same bytes, and reading it off the request
    /// keeps the answer to a header the peer sent from ever being assembled here.
    fn named(&self, request: &HeaderMap) -> Option<HeaderValue> {
        let sent = request.get(ORIGIN)?;
        let written = sent.to_str().ok()?;
        self.origins
            .iter()
            .any(|named| named == written)
            .then(|| sent.clone())
    }
}

impl Crossing {
    /// Writes this answer onto a response, leaving a name the answer already spelled alone.
    ///
    /// The `or_insert` half is [`crate::secure::Secure::fill`]'s rule and is here for the same
    /// reason — a program that set its own `Access-Control-Allow-Origin` keeps it. `Vary` is
    /// the exception and *appends*, because it is a list and not a value: a response already
    /// varying on `Accept-Encoding` must go on doing so, and replacing the header would tell a
    /// cache the answer is the same for every encoding.
    pub fn fill(&self, answer: &mut HeaderMap) {
        if self.varies && !varies_on_origin(answer) {
            answer.append(VARY, HeaderValue::from_static("Origin"));
        }
        if let Some(allow) = self.allow.as_ref() {
            answer
                .entry(ACCESS_CONTROL_ALLOW_ORIGIN)
                .or_insert_with(|| allow.clone());
        }
    }
}

/// Whether `answer` already says it varies by `Origin`, under any spelling a cache accepts.
///
/// `Vary` may arrive as several header lines and each line as a comma-separated list, and its
/// names are case-insensitive, so all three have to be walked to answer "is it already there".
/// Writing a duplicate would harm nothing a cache does; it is the reader of the response who
/// pays, and the walk is over the handful of names one answer varies on.
fn varies_on_origin(answer: &HeaderMap) -> bool {
    answer.get_all(VARY).iter().any(|line| {
        line.to_str().is_ok_and(|line| {
            line.split(',')
                .any(|name| name.trim().eq_ignore_ascii_case("origin"))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use nvs_config::tree::HttpCors;

    /// A policy over the origins named, as `[http.cors] origins` would have written them.
    fn naming(origins: &[&str]) -> Cors {
        Cors::of(Some(&Http {
            cors: Some(HttpCors {
                origins: Some(origins.iter().map(|origin| (*origin).to_owned()).collect()),
                ..HttpCors::default()
            }),
            ..Http::default()
        }))
    }

    /// A request carrying one `Origin`.
    fn from(origin: &'static str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(ORIGIN, HeaderValue::from_static(origin));
        headers
    }

    /// The answer `policy` writes for a request from `origin`, as a response's headers.
    fn answered(policy: &Cors, request: &HeaderMap) -> HeaderMap {
        let mut answer = HeaderMap::new();
        policy.answer(request).fill(&mut answer);
        answer
    }

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

    /// § 2's first half, over the writer rather than over the wire: a closed policy has no
    /// answer to write, so a response it touched is one it did not touch.
    #[test]
    fn a_closed_policy_writes_nothing_at_all() {
        assert!(
            answered(&Cors::default(), &from("https://a.example")).is_empty(),
            "a closed policy wrote a header"
        );
    }

    /// The open half: a named origin is echoed back, and the answer says it depends on which
    /// origin asked.
    #[test]
    fn a_named_origin_is_allowed_and_the_answer_varies_on_it() {
        let answer = answered(
            &naming(&["https://a.example", "https://b.example"]),
            &from("https://b.example"),
        );
        assert_eq!(
            answer
                .get(ACCESS_CONTROL_ALLOW_ORIGIN)
                .map(HeaderValue::as_bytes),
            Some(&b"https://b.example"[..]),
            "a named origin was not told it may cross"
        );
        assert!(
            varies_on_origin(&answer),
            "the answer did not vary by origin"
        );
    }

    /// The module doc's cache argument, which is the half a test over the allowed origin alone
    /// cannot see: the answers that carry *no* `Access-Control-Allow-Origin` are the ones a
    /// cache would otherwise replay to an origin that should have got one.
    #[test]
    fn an_answer_with_nothing_to_allow_still_varies() {
        let policy = naming(&["https://a.example"]);
        for request in [from("https://elsewhere.example"), HeaderMap::new()] {
            let answer = answered(&policy, &request);
            assert!(
                !answer.contains_key(ACCESS_CONTROL_ALLOW_ORIGIN),
                "an origin the list does not name was told it may cross"
            );
            assert!(
                varies_on_origin(&answer),
                "an answer an open policy could have varied did not say so"
            );
        }
    }

    /// Exactly, byte for byte. Every value here denotes the same server as the named origin
    /// under some equivalence a lenient matcher might apply, and none of them is that origin.
    #[test]
    fn an_origin_is_matched_exactly_and_never_normalised() {
        let policy = naming(&["https://a.example"]);
        for sent in [
            "https://a.example:443",
            "https://A.example",
            "https://a.example/",
            "http://a.example",
            "https://a.example.evil",
            "https://sub.a.example",
        ] {
            assert!(
                !answered(&policy, &from(sent)).contains_key(ACCESS_CONTROL_ALLOW_ORIGIN),
                "`{sent}` was matched against `https://a.example`"
            );
        }
    }

    /// The wildcard's answer is the same for everybody, so it is the one open policy that does
    /// not vary — saying it did would be false, and a cache would pay for the lie per origin.
    #[test]
    fn a_wildcard_answers_everybody_and_does_not_vary() {
        let policy = naming(&["*"]);
        for request in [from("https://a.example"), HeaderMap::new()] {
            let answer = answered(&policy, &request);
            assert_eq!(
                answer
                    .get(ACCESS_CONTROL_ALLOW_ORIGIN)
                    .map(HeaderValue::as_bytes),
                Some(&b"*"[..]),
            );
            assert!(
                !varies_on_origin(&answer),
                "the wildcard said its answer depends on who asked"
            );
        }
    }

    /// `Vary` is a list. A response that already varies on something else goes on varying on
    /// it, and one that already named `Origin` — under any spelling a cache reads — is not
    /// told twice.
    #[test]
    fn vary_keeps_what_the_answer_already_varied_on() {
        let crossing = naming(&["https://a.example"]).answer(&from("https://a.example"));

        let mut answer = HeaderMap::new();
        answer.insert(VARY, HeaderValue::from_static("Accept-Encoding"));
        crossing.fill(&mut answer);
        assert_eq!(
            answer
                .get_all(VARY)
                .iter()
                .map(|line| line.to_str().unwrap())
                .collect::<Vec<_>>(),
            ["Accept-Encoding", "Origin"],
        );

        let mut already = HeaderMap::new();
        already.insert(VARY, HeaderValue::from_static("accept-encoding, origin"));
        crossing.fill(&mut already);
        assert_eq!(
            already.get_all(VARY).iter().count(),
            1,
            "`Vary` was told twice"
        );
    }

    /// A response that spelled its own answer keeps it — [`crate::secure::Secure::fill`]'s
    /// rule, and the one that lets `Core\Response::setHeader` override a policy-owned header.
    #[test]
    fn an_answer_that_wrote_its_own_allow_origin_keeps_it() {
        let mut answer = HeaderMap::new();
        answer.insert(
            ACCESS_CONTROL_ALLOW_ORIGIN,
            HeaderValue::from_static("https://chosen.example"),
        );
        naming(&["https://a.example"])
            .answer(&from("https://a.example"))
            .fill(&mut answer);
        assert_eq!(
            answer
                .get(ACCESS_CONTROL_ALLOW_ORIGIN)
                .map(HeaderValue::as_bytes),
            Some(&b"https://chosen.example"[..]),
        );
    }
}
