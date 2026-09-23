//! `rule:http-server/cors-is-closed-until-origins-are-named`'s cross-origin policy: which origins `[http.cors] origins` names, and what a
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
//! The second is a decision, and [`Cors::preflight`] is it: an `OPTIONS` carrying `Origin` and
//! `Access-Control-Request-Method` — `rule:http-server/a-preflight-is-answered-before-any-code-runs`'s two headers — is a browser asking
//! permission, and a server that has named no origin has none to give. **`403` and not `405`**,
//! because the verb is one this server implements and answers elsewhere — what is refused is the
//! *origin*, and saying `405` would tell a browser to stop asking about a method rather than that
//! nobody may cross.
//!
//! **It is refused before the handler**, beside `rule:http-server/the-server-block-is-boot-class`'s valve, so a preflight nobody
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
//! them as one would be `rule:errors/ambiguous-input-refused`'s repair — inventing an equivalence the operator did not
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
//! # What a preflight is answered with
//!
//! [`Cors::preflight`] answers **every** preflight, under an open policy exactly as under a closed
//! one, and one match decides both: the answer is `204` where [`Cors::answer`] found an origin to
//! allow and `403` where it did not. One rule rather than two, because a policy that has named
//! nobody and a policy that has not named *this* peer are the same answer to the same question —
//! and stating it once is what keeps the refusal from being a second matcher that could disagree
//! with the first.
//!
//! The granting answer carries § 2's configured lines beside the allow: `Allow-Methods` from
//! `methods` — `GET, HEAD, POST` with nothing written — `Allow-Headers` from `headers`, and
//! `Max-Age` from `max_age` in the whole seconds that header is counted in. **Each is written
//! verbatim out of the block and never reflected back off the request.** A preflight that echoed
//! `Access-Control-Request-Headers` would allow whatever was asked for, which is a policy the
//! operator did not write and could not read out of their own configuration; § 2 configures what
//! may be allowed, so the request's own asking is the browser's side of the exchange and not the
//! server's. A list that is empty — written so, or the shipped default for the lists that ship
//! empty — is **no header at all** rather than an empty one, because a line naming nothing permits
//! nothing and a blank value asks every browser to agree on what that means.
//!
//! **`204` and not `200`**: a preflight's answer is its headers, there is no body to send, and a
//! status that promises one invites a peer to wait for it.
//!
//! The refusal carries none of them, a refusal having no permission to describe — but it does
//! carry the `Vary` an open list owes every answer it touched, for the cache reason above: a `403`
//! stored under the URL alone is replayed to the origin that would have been granted one.
//!
//! `max_age` is written as a duration and sent as a number, and the parse that crosses between them
//! is [`nvs_config::value`]'s, which is also where a value that is not a duration is refused —
//! at boot, by `nvs_config::http`'s `validate`, so that nothing here has an arm for one.
//!
//! # `credentials` and `expose`
//!
//! The other keys § 2 configures are carried by [`Crossing`], and one rule covers them:
//! **nothing but the `Vary` is written onto an answer with no origin to allow.** A refused origin
//! and a closed policy have no permission to qualify, and `Access-Control-Allow-Credentials` beside
//! no `Access-Control-Allow-Origin` describes a crossing that was not granted.
//!
//! `credentials = true` is sent as `Access-Control-Allow-Credentials: true` and `false` is sent as
//! nothing at all, because the header has one meaningful value: a browser reads `true` and treats
//! every other spelling, including `false`, as the header's absence. Sending `false` would be a
//! line that says what its own absence already says.
//!
//! `expose` is `Access-Control-Expose-Headers` and goes on an ordinary answer only. A preflight's
//! answer is read by the browser rather than by script, so an exposed-header list on it describes a
//! response that does not exist — the same empty-list rule as the preflight's own lines
//! applies, and nothing is sent where nothing is named.
//!
//! # What is elsewhere
//!
//! § 2's other rule is not this module's: `origins = ["*"]` together with `credentials = true`
//! is refused at boot and by `Core\Config::set`, both through `nvs_config::http`, because it is
//! a question about a written configuration rather than about a request.
//!

use hyper::header::{
    ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS,
    ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_EXPOSE_HEADERS, ACCESS_CONTROL_MAX_AGE,
    ACCESS_CONTROL_REQUEST_METHOD, HeaderName, HeaderValue, ORIGIN, VARY,
};
use hyper::{HeaderMap, Method, StatusCode};
use nvs_config::tree::{Http, Setting};
use nvs_config::{Quantity, Unit};

/// `rule:http-server/cors-is-closed-until-origins-are-named`'s policy, resolved from `[http.cors]`.
///
/// One per published snapshot, on the terms [`crate::secure::Secure`] is: `[http.cors]` reloads,
/// and [`crate::serve::Serving`] derives the policy again for the first request under a new one.
///
#[derive(Clone, Debug)]
pub struct Cors {
    /// The exact origins `[http.cors] origins` named, as written. **Empty is the whole of what
    /// "closed" means**, and it is what a tree that wrote no `[http.cors]` resolves to.
    origins: Vec<String>,
    /// `[http.cors] methods` as the one `Access-Control-Allow-Methods` line it is sent as, or
    /// [`None`] where the list is empty and there is no line to send.
    methods: Option<HeaderValue>,
    /// `[http.cors] headers`, on the same terms, as `Access-Control-Allow-Headers`.
    headers: Option<HeaderValue>,
    /// `[http.cors] max_age` in the whole seconds `Access-Control-Max-Age` counts, § 2's `10m`
    /// with nothing written. Not optional: § 2 ships a value, so every grant states one.
    max_age: HeaderValue,
    /// `[http.cors] expose` as its `Access-Control-Expose-Headers` line, or [`None`] where the
    /// list is empty — which is § 2's shipped value.
    expose: Option<HeaderValue>,
    /// `[http.cors] credentials`; § 2's default is `false`. The wildcard beside `true` is refused
    /// at boot by `nvs_config::http`, so no started server reaches this holding both.
    credentials: bool,
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
    /// `Access-Control-Expose-Headers`, written only onto an answer a script will read and only
    /// where there is an `allow` to qualify.
    expose: Option<HeaderValue>,
    /// Whether `Access-Control-Allow-Credentials: true` goes on, which is `[http.cors]
    /// credentials` — again only beside an `allow`.
    credentials: bool,
}

/// § 2's whole answer to one preflight, decided before any handler and before any mount.
///
/// It carries a [`Crossing`] rather than deciding an origin again: which origins may cross is one
/// question with one answer, and a preflight that matched separately could permit what an ordinary
/// request from the same peer is refused. What this adds is the status and the lines § 2
/// configures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preflight {
    /// `204` where the policy has permission to give this origin, `403` where it has none.
    status: StatusCode,
    /// The allow-origin and `Vary` decision, exactly as an ordinary answer's.
    crossing: Crossing,
    /// `Access-Control-Allow-Methods`, and [`None`] on a refusal or where the list is empty.
    methods: Option<HeaderValue>,
    /// `Access-Control-Allow-Headers`, on the same terms.
    headers: Option<HeaderValue>,
    /// `Access-Control-Max-Age`, [`None`] on a refusal and present on every grant.
    max_age: Option<HeaderValue>,
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
    /// [`nvs_config::Config`] passes `config.http.as_ref()` and never has to know which
    /// sub-block this reads — the same argument as [`crate::secure::Secure::of`]'s.
    #[must_use]
    pub fn of(http: Option<&Http>) -> Self {
        let cors = http.and_then(|http| http.cors.as_ref());
        Self {
            origins: cors
                .and_then(|cors| cors.origins.clone())
                .unwrap_or_default(),
            methods: line(
                cors.and_then(|cors| cors.methods.as_deref()),
                "GET, HEAD, POST",
            ),
            headers: line(cors.and_then(|cors| cors.headers.as_deref()), ""),
            max_age: seconds_of(cors.and_then(|cors| cors.max_age.as_deref())),
            expose: line(cors.and_then(|cors| cors.expose.as_deref()), ""),
            credentials: cors.and_then(|cors| cors.credentials).unwrap_or(false),
        }
    }

    /// § 2's answer to one preflight, or [`None`] where this request is not a preflight at all.
    ///
    /// A preflight is an `OPTIONS` carrying **both** `Origin` and `Access-Control-Request-Method`,
    /// which is `rule:http-server/a-preflight-is-answered-before-any-code-runs`'s own definition and is read here as written. Neither header alone
    /// is one: a program may answer an ordinary `OPTIONS` — that is what `Allow` is for — and
    /// refusing every one of them would take a method away in order to close a door it never went
    /// through, while a request that named no origin is not asking to cross whatever else it sent.
    ///
    /// Every preflight this *is* one gets an answer from here and never from a program, under an
    /// open policy as under a closed one. The module doc's own section owns why the two are one
    /// rule and why the granting half is `204`.
    ///
    #[must_use]
    pub fn preflight(&self, method: &Method, request: &HeaderMap) -> Option<Preflight> {
        if method != Method::OPTIONS
            || !request.contains_key(ORIGIN)
            || !request.contains_key(ACCESS_CONTROL_REQUEST_METHOD)
        {
            return None;
        }
        let crossing = self.answer(request);
        if crossing.allow.is_none() {
            return Some(Preflight {
                status: StatusCode::FORBIDDEN,
                crossing,
                methods: None,
                headers: None,
                max_age: None,
            });
        }
        Some(Preflight {
            status: StatusCode::NO_CONTENT,
            crossing,
            methods: self.methods.clone(),
            headers: self.headers.clone(),
            max_age: Some(self.max_age.clone()),
        })
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
                expose: self.expose.clone(),
                credentials: self.credentials,
            };
        }
        Crossing {
            allow: self.named(request),
            varies: true,
            expose: self.expose.clone(),
            credentials: self.credentials,
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
        self.write(answer, true);
    }

    /// The whole of it, with `exposing` false where the answer is a preflight's — the module doc's
    /// `expose` paragraph owns why that one line is the only difference between the two.
    fn write(&self, answer: &mut HeaderMap, exposing: bool) {
        if self.varies && !varies_on_origin(answer) {
            answer.append(VARY, HeaderValue::from_static("Origin"));
        }
        // The module doc's one rule for the rest: an answer with no origin to allow carries
        // nothing that qualifies one. Returning here rather than guarding each line is what keeps
        // a later qualifier from being added outside it.
        let Some(allow) = self.allow.as_ref() else {
            return;
        };
        answer
            .entry(ACCESS_CONTROL_ALLOW_ORIGIN)
            .or_insert_with(|| allow.clone());
        if self.credentials {
            answer
                .entry(ACCESS_CONTROL_ALLOW_CREDENTIALS)
                .or_insert_with(|| HeaderValue::from_static("true"));
        }
        if let Some(expose) = self.expose.as_ref().filter(|_| exposing) {
            answer
                .entry(ACCESS_CONTROL_EXPOSE_HEADERS)
                .or_insert_with(|| expose.clone());
        }
    }
}

impl Preflight {
    /// The status § 2 answers this preflight with.
    #[must_use]
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Writes this answer's headers onto the response the door is about to send.
    ///
    /// [`Crossing::fill`]'s rule for each of them: a name the response already spelled is left
    /// alone, and `Vary` appends. Nothing here is reached by an application's answer — the door
    /// builds this response itself — but writing it the other way would make that a fact about the
    /// call site rather than about the writer.
    pub fn fill(&self, answer: &mut HeaderMap) {
        self.crossing.write(answer, false);
        for (name, line) in [
            (ACCESS_CONTROL_ALLOW_METHODS, self.methods.as_ref()),
            (ACCESS_CONTROL_ALLOW_HEADERS, self.headers.as_ref()),
            (ACCESS_CONTROL_MAX_AGE, self.max_age.as_ref()),
        ] {
            let Some(line) = line else { continue };
            answer
                .entry::<HeaderName>(name)
                .or_insert_with(|| line.clone());
        }
    }
}

/// One configured list as the single header line § 2 sends it as, or [`None`] where there is
/// nothing to send.
///
/// An absent key is `default`, which is § 2's shipped value for that key — a list for `methods`
/// and empty for the rest. An empty list either way is no header at all, which the module
/// doc's preflight section owns.
///
/// A value the wire cannot carry answers [`None`] as well, and that is not a repair:
/// `nvs_config::http`'s `validate` has already refused such a tree at boot, so this arm is
/// reachable only from a snapshot that never started a server.
fn line(written: Option<&[String]>, default: &str) -> Option<HeaderValue> {
    let joined = match written {
        Some(values) => values.join(", "),
        None => default.to_owned(),
    };
    if joined.is_empty() {
        return None;
    }
    HeaderValue::from_str(&joined).ok()
}

/// `[http.cors] max_age` as the whole seconds `Access-Control-Max-Age` is counted in.
///
/// The parse is [`nvs_config::value`]'s, so `"10m"`, `"600s"` and `"600"` are one value and a
/// suffix it does not know is not one — and an unparseable value falls to § 2's own `10m` here for
/// [`line`]'s reason, boot having refused it. Sub-second is truncated rather than rounded up,
/// which only reaches `0`: a browser may not cache the preflight, which is the honest reading of a
/// duration written shorter than the unit the header can state.
fn seconds_of(written: Option<&str>) -> HeaderValue {
    const SHIPPED: u64 = 600;
    let seconds = written
        .and_then(|written| {
            let value = Setting::Text(written.to_owned());
            match Quantity::parse("http.cors.max_age", Unit::Duration, &value) {
                Ok(Quantity::Nanos(nanos)) => Some(nanos / 1_000_000_000),
                _ => None,
            }
        })
        .unwrap_or(SHIPPED);
    HeaderValue::from(seconds)
}

/// Whether `answer` already says it varies by `Origin`, under any spelling a cache accepts.
///
/// `Vary` may arrive as several header lines and each line as a comma-separated list, and its
/// names are case-insensitive, so each of those has to be walked to answer "is it already there".
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

    /// A policy over one `[http.cors]` block, as an operator would have written it.
    fn configured(cors: HttpCors) -> Cors {
        Cors::of(Some(&Http {
            cors: Some(cors),
            ..Http::default()
        }))
    }

    /// A policy over the origins named, as `[http.cors] origins` would have written them.
    fn naming(origins: &[&str]) -> Cors {
        configured(HttpCors {
            origins: Some(origins.iter().map(|origin| (*origin).to_owned()).collect()),
            ..HttpCors::default()
        })
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
    fn asking_from(origin: &'static str) -> HeaderMap {
        let mut headers = from(origin);
        headers.insert(
            ACCESS_CONTROL_REQUEST_METHOD,
            HeaderValue::from_static("POST"),
        );
        headers
    }

    /// A preflight from an origin no case here names.
    fn asking() -> HeaderMap {
        asking_from("https://elsewhere.example")
    }

    /// The answer `policy` gives one preflight: its status, and the headers it writes.
    fn preflighted(policy: &Cors, request: &HeaderMap) -> (StatusCode, HeaderMap) {
        let answer = policy
            .preflight(&Method::OPTIONS, request)
            .expect("a preflight was not recognised as one");
        let mut headers = HeaderMap::new();
        answer.fill(&mut headers);
        (answer.status(), headers)
    }

    /// § 2's second half: with `origins = []` there is no permission to give, and the refusal
    /// says nothing beyond the status — a closed policy emits no CORS header even on its own
    /// answer.
    #[test]
    fn a_preflight_nobody_configured_is_refused() {
        let (status, answer) = preflighted(&Cors::default(), &asking());
        assert_eq!(
            status,
            StatusCode::FORBIDDEN,
            "a preflight was passed on with `[http.cors] origins` naming nobody"
        );
        assert!(
            answer.is_empty(),
            "§ 2's closed policy wrote a header onto its own refusal"
        );
    }

    /// The open half of the same rule: a named origin's preflight is granted, and what it is
    /// granted is what the block wrote — never what the request asked for.
    #[test]
    fn a_named_origins_preflight_is_granted_with_what_the_block_configures() {
        let policy = configured(HttpCors {
            origins: Some(vec!["https://a.example".to_owned()]),
            methods: Some(vec!["GET".to_owned(), "DELETE".to_owned()]),
            headers: Some(vec!["Authorization".to_owned()]),
            max_age: Some("1h".to_owned()),
            ..HttpCors::default()
        });

        let (status, answer) = preflighted(&policy, &asking_from("https://a.example"));
        assert_eq!(
            status,
            StatusCode::NO_CONTENT,
            "a granted preflight promised a body it has no way to send"
        );
        for (name, expected) in [
            (ACCESS_CONTROL_ALLOW_ORIGIN, "https://a.example"),
            (ACCESS_CONTROL_ALLOW_METHODS, "GET, DELETE"),
            (ACCESS_CONTROL_ALLOW_HEADERS, "Authorization"),
            // `1h` as the seconds the header counts in, which is the only place in this module
            // where a written duration and a sent number are the same value.
            (ACCESS_CONTROL_MAX_AGE, "3600"),
        ] {
            assert_eq!(
                answer.get(&name).map(HeaderValue::as_bytes),
                Some(expected.as_bytes()),
                "`{name}` is not what `[http.cors]` configured"
            );
        }
        assert!(
            varies_on_origin(&answer),
            "a granted preflight did not say it depends on who asked"
        );
    }

    /// The same match decides both halves, so an origin an open list does not name is refused
    /// exactly as a closed policy refuses everybody — and the refusal still varies, which is the
    /// half a cache pays for: a `403` stored under the URL alone is replayed to the origin that
    /// would have been granted a `204`.
    #[test]
    fn a_preflight_from_an_unnamed_origin_is_refused_and_still_varies() {
        let (status, answer) = preflighted(&naming(&["https://a.example"]), &asking());
        assert_eq!(status, StatusCode::FORBIDDEN);
        for name in [
            ACCESS_CONTROL_ALLOW_ORIGIN,
            ACCESS_CONTROL_ALLOW_METHODS,
            ACCESS_CONTROL_ALLOW_HEADERS,
            ACCESS_CONTROL_MAX_AGE,
        ] {
            assert!(
                !answer.contains_key(&name),
                "a refusal described a permission: `{name}`"
            );
        }
        assert!(
            varies_on_origin(&answer),
            "an answer an open policy could have varied did not say so"
        );
    }

    /// § 2's shipped values are what a block that named an origin and nothing else grants — and
    /// an empty list is no header at all, whether the operator wrote it empty or left the key out
    /// for a default that is empty. Both spellings are asserted together because a writer that
    /// sent `Access-Control-Allow-Headers:` with nothing after it passes either alone.
    #[test]
    fn an_unconfigured_grant_is_section_2s_shipped_answer_and_an_empty_list_is_no_header() {
        let (_, shipped) = preflighted(
            &naming(&["https://a.example"]),
            &asking_from("https://a.example"),
        );
        assert_eq!(
            shipped
                .get(ACCESS_CONTROL_ALLOW_METHODS)
                .map(HeaderValue::as_bytes),
            Some(&b"GET, HEAD, POST"[..]),
        );
        assert_eq!(
            shipped
                .get(ACCESS_CONTROL_MAX_AGE)
                .map(HeaderValue::as_bytes),
            Some(&b"600"[..]),
            "`max_age` left out is not § 2's own `10m`"
        );
        assert!(
            !shipped.contains_key(ACCESS_CONTROL_ALLOW_HEADERS),
            "an empty shipped default was sent as an empty header line"
        );

        let written = configured(HttpCors {
            origins: Some(vec!["https://a.example".to_owned()]),
            methods: Some(Vec::new()),
            ..HttpCors::default()
        });
        let (_, answer) = preflighted(&written, &asking_from("https://a.example"));
        assert!(
            !answer.contains_key(ACCESS_CONTROL_ALLOW_METHODS),
            "`methods = []` permitted a method by naming none"
        );
    }

    /// The wildcard is the one open policy whose preflight does not depend on who asked, so it
    /// grants the origin this list never names and says nothing varies.
    #[test]
    fn a_wildcard_grants_a_preflight_and_does_not_vary() {
        let (status, answer) = preflighted(&naming(&["*"]), &asking());
        assert_eq!(status, StatusCode::NO_CONTENT);
        assert_eq!(
            answer
                .get(ACCESS_CONTROL_ALLOW_ORIGIN)
                .map(HeaderValue::as_bytes),
            Some(&b"*"[..]),
        );
        assert!(
            !varies_on_origin(&answer),
            "the wildcard said its preflight depends on who asked"
        );
    }

    /// § 2's `credentials` is the one key qualifying both answers a policy gives, so a named
    /// origin is told it may send them on the ordinary answer and on its preflight alike — and an
    /// origin the list does not name is told neither, there being no crossing to qualify.
    #[test]
    fn credentials_qualify_both_answers_and_only_beside_an_allowed_origin() {
        let policy = configured(HttpCors {
            origins: Some(vec!["https://a.example".to_owned()]),
            credentials: Some(true),
            ..HttpCors::default()
        });

        for granted in [
            answered(&policy, &from("https://a.example")),
            preflighted(&policy, &asking_from("https://a.example")).1,
        ] {
            assert_eq!(
                granted
                    .get(ACCESS_CONTROL_ALLOW_CREDENTIALS)
                    .map(HeaderValue::as_bytes),
                Some(&b"true"[..]),
                "an allowed origin was not told it may send credentials"
            );
        }
        for refused in [
            answered(&policy, &from("https://elsewhere.example")),
            preflighted(&policy, &asking()).1,
        ] {
            assert!(
                !refused.contains_key(ACCESS_CONTROL_ALLOW_CREDENTIALS),
                "an origin that may not cross was told it may cross with credentials"
            );
        }

        // `false` is § 2's shipped value and is sent as nothing at all: the header has one
        // meaningful spelling, and a browser reads every other one as its absence.
        assert!(
            !answered(&naming(&["https://a.example"]), &from("https://a.example"))
                .contains_key(ACCESS_CONTROL_ALLOW_CREDENTIALS),
            "`credentials = false` was stated rather than left unsaid"
        );
    }

    /// § 2's `expose` reaches the answer a script reads and not the preflight standing in front of
    /// it, and the empty list § 2 ships is no header at all.
    #[test]
    fn expose_reaches_a_simple_answer_and_not_the_preflight() {
        let policy = configured(HttpCors {
            origins: Some(vec!["https://a.example".to_owned()]),
            expose: Some(vec!["X-Total".to_owned(), "X-Page".to_owned()]),
            ..HttpCors::default()
        });

        assert_eq!(
            answered(&policy, &from("https://a.example"))
                .get(ACCESS_CONTROL_EXPOSE_HEADERS)
                .map(HeaderValue::as_bytes),
            Some(&b"X-Total, X-Page"[..]),
            "a script was not told which headers it may read"
        );
        assert!(
            !preflighted(&policy, &asking_from("https://a.example"))
                .1
                .contains_key(ACCESS_CONTROL_EXPOSE_HEADERS),
            "a preflight described a response nobody will read it off"
        );
        assert!(
            !answered(&naming(&["https://a.example"]), &from("https://a.example"))
                .contains_key(ACCESS_CONTROL_EXPOSE_HEADERS),
            "§ 2's shipped empty list was sent as a header naming nothing"
        );
    }

    /// `rule:http-server/a-preflight-is-answered-before-any-code-runs` names two headers and both are required, so an `OPTIONS` missing either is the
    /// program's to answer — under an open list as under a closed one, which is the half that would
    /// otherwise take `Allow` away from every application the moment an origin was named.
    ///
    /// The two removals are one case because what is pinned is that the pair is a conjunction: a
    /// predicate reading either header alone passes the half that removed the other.
    #[test]
    fn a_plain_options_is_not_a_preflight() {
        for missing in [ACCESS_CONTROL_REQUEST_METHOD, ORIGIN] {
            let mut headers = asking();
            headers.remove(&missing);
            for policy in [Cors::default(), naming(&["https://elsewhere.example"])] {
                assert_eq!(
                    policy.preflight(&Method::OPTIONS, &headers),
                    None,
                    "an `OPTIONS` carrying no `{missing}` was answered as a preflight"
                );
            }
        }
    }

    /// The header that decides is only read on `OPTIONS`: a `GET` from another origin is an
    /// ordinary request, answered as one and — under a closed policy — told nothing.
    #[test]
    fn a_cross_origin_get_is_not_a_preflight() {
        for policy in [Cors::default(), naming(&["https://elsewhere.example"])] {
            assert_eq!(policy.preflight(&Method::GET, &asking()), None);
        }
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
