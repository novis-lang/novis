//! The request as it arrived.
//!
//! [`Inbound`] is what a host fills in before the request runs and what
//! `Core\Request` reads back — method, path, query, headers, the client
//! address and the [`Scheme`] a trusted proxy asserted
//! ([ADR 0097](/docs/adr/0097-development-server-and-proxied-origin.md) § 6).
//!
//! The body is a [`RequestBody`] rather than bytes, and that is this file's one
//! real decision:
//! [ADR 0074](/docs/adr/0074-http-defaults-safe-and-finite.md)'s finite
//! defaults mean a carrier hands the body out a chunk at a time and holds none
//! of it, so what a request has resident is one chunk and never the whole body.
//!
//! One thing here is not a fact the request arrived with, and that is
//! [`UpgradeSlot`]:
//! [ADR 0083](/docs/adr/0083-persistent-connections-are-isolates.md) § 1's
//! connection isolate is *prepared* inside the request by `Core\Socket::upgrade`
//! and *started* by the connection once that request has ended, so the two need
//! somewhere to meet. It is on the carrier because a connection is the only
//! thing an upgrade can happen to: a request that arrived on one has a slot, and
//! a CLI program, a `spawn script` child and a request no connection offered one
//! for do not — which is the whole of how that member refuses. The home of why
//! it is here rather than on a completion or on the context is
//! `nvs_stdlib::socket`'s module doc, § *Decision: this member spawns nothing,
//! and the connection starts it*.
//!
//! **There are two such cells and not one.** ADR 0083 § 5 is the home of why:
//! a WebSocket upgrade *takes the socket*, so its isolate starts once the
//! request's own future has ended, while an SSE connection takes nothing and
//! writes into the body of an ordinary `200` the connection is still sending.
//! Two hand-overs arriving at two moments, so two types — [`SseSlot`] is the
//! second, and the door offers it to **every** request a server answers rather
//! than to an upgradable one. That is the same fail-closed rule read against a
//! different hand-over: what still has no cell, and so still throws, is
//! everything that is not a served request.

use std::cell::RefCell;
use std::rc::Rc;

use super::*;

impl Ctx {
    /// Gives this context the request it is answering — the inbound half of
    /// the channel [`super::output`]'s declarations are the outbound half of.
    ///
    /// Called once, by whoever accepted the request, before the program runs.
    /// There is no member that clears one: a context answers one request for
    /// its whole life, and the isolate is what is discarded between two.
    pub fn set_inbound(&mut self, inbound: Inbound) {
        // ADR 0076 § 2's trace is the door's decision and rides on the carrier
        // ([`Inbound::set_trace_context`]), so this is where it becomes the
        // context's — the one write [`Self::set_trace_context`] describes, made
        // before the program runs and beside the rest of what a request arrives
        // with. A carrier that carries none leaves the root [`Self::new`] drew
        // standing, which is § 2's "an id exists for every request".
        if let Some(trace) = inbound.trace_context() {
            self.set_trace_context(trace);
        }
        self.inbound = Some(Box::new(inbound));
    }
    /// The request this context is answering, or `None` where there is none.
    ///
    /// **A borrow rather than a take**, unlike [`Self::take_headers`] and its
    /// two neighbours: those are read once by the finish path and are gone, and
    /// this is read as many times as the program asks. `Core\Request`'s members
    /// are the only readers, and each of them turns `None` into
    /// [ADR 0012](/docs/adr/0012-no-superglobals.md) § 7's throw.
    #[must_use]
    pub fn inbound(&self) -> Option<&Inbound> {
        self.inbound.as_deref()
    }
    /// The same request, borrowed so its body can be pulled from.
    ///
    /// Beside [`Self::inbound`] rather than replacing it, because the two
    /// halves of the carrier are read on different terms: the request line and
    /// the head are facts that stay put however many members ask, and
    /// [`Inbound::body`] advances a socket. A member that only wants a header
    /// takes the shared borrow and cannot consume anything by mistake.
    #[must_use]
    pub fn inbound_mut(&mut self) -> Option<&mut Inbound> {
        self.inbound.as_deref_mut()
    }
}

/// The scheme a request effectively arrived over.
///
/// **Effective, not observed**: Novis terminates no TLS
/// ([ADR 0097](/docs/adr/0097-development-server-and-proxied-origin.md)
/// § 1), so `Https` is only ever what a *trusted* proxy asserted through
/// `X-Forwarded-Proto` — § 6's walk is the one thing that decides it, and this
/// carrier holds its answer rather than re-deriving one.
///
/// Not a boolean, because the two values are named in the ADR and a `bool` at a
/// call site would need a comment saying which way round it goes.
///
/// It lives here, one crate below the door, because two readers a crate apart
/// need the same two values and a copy in each is two answers to one question:
/// `nvs_server::secure` conditions [ADR 0074] § 1's HSTS header on it, and
/// [`Inbound::scheme`] is what `Core\Request::scheme()` reads. `nvs_server`
/// re-exports this type rather than declaring its own.
///
/// [ADR 0074]: ../../../docs/adr/0074-http-defaults-safe-and-finite.md
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scheme {
    /// A plaintext connection, and what a trusted proxy asserted nothing about.
    Http,
    /// TLS, as a trusted proxy asserted it (ADR 0097 § 6).
    Https,
}

/// The request line a context is answering, as it arrived off the wire —
/// what spec § 15's `Core\Request` reads and the only thing on a context that
/// came from outside the process.
///
/// **It interprets nothing.** The verb is the bytes the peer wrote, the path is
/// what is left of the target after ADR 0097 § 4 step 2 stripped the matched
/// mount's prefix, the query is the raw string after the `?` with no
/// percent-decoding and no bracket convention applied, and a header is one
/// entry per field line in the spelling and the order the peer sent it. Every
/// reading of those — which of `Core\Http\Method`'s eight cases a verb is, what
/// a query parameter's name means, that a field name matches without regard to
/// case — belongs to `nvs_stdlib::request`, because the rosters and the
/// conventions are that crate's and a second copy of either here would be a
/// second answer. This type is the carrier and nothing else, which is also what
/// lets it exist in a crate that has never heard of HTTP.
///
/// **Everything on it is `tainted`** in the sense
/// [ADR 0024](/docs/adr/0024-taint-tracking-for-injection-sinks.md)
/// means: it is what a client sent. The qualifier itself is a *type*, so it is
/// carried by the registry rows of the members that read this and not by any
/// field here — there is no representation of a qualifier at runtime.
///
/// **What it spends:** three short allocations per served request, plus two per
/// header field line and one growing vector to hold them, one more allocation
/// for a request that arrived with a body, and nothing at all for a process
/// serving none. **Not the body's bytes** — see [`RequestBody`]. The peer costs
/// no allocation at all: an address and a scheme are held inline, as the words
/// the door decided them as.
pub struct Inbound {
    /// The method token the peer wrote, verbatim and un-uppercased.
    method: Box<str>,
    /// The request path with the matched mount's prefix removed, still
    /// percent-encoded.
    path: Box<str>,
    /// Everything after the `?`, without it, and `""` where the target carried
    /// no query at all — the two are not distinguished, because a query with no
    /// pairs and no query yield the same empty set of parameters.
    query: Box<str>,
    /// [ADR 0102](/docs/adr/0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md)
    /// § 7's first half: the prefix ADR 0097 § 4 step 2 took off [`Self::path`]
    /// above, which is the one fact about where an application was deployed
    /// that the application itself is allowed to see.
    ///
    /// `""` for a request no mount table selected — a test, a CLI program, a
    /// carrier built by something that never had a table — and equally for a
    /// mount written `prefix = "/"`. Those are deliberately one answer: § 7's
    /// `prefix` is *what was stripped*, and neither of them stripped anything.
    ///
    /// **What it spends:** one short allocation per request, of a prefix the
    /// mount table already holds. It is copied rather than borrowed because the
    /// carrier outlives the handler that read the table
    /// (`nvs_server::mount::carry` owns that direction).
    mount_prefix: Box<str>,
    /// § 7's other half: ADR 0097 § 3's glob captures of the row that selected
    /// this request, in order — `{1}` is the first — and empty for every mount
    /// whose `scan` had no `*` to capture with.
    ///
    /// The two are one fact and are written by one call ([`Self::set_mount`]):
    /// a prefix standing beside somebody else's captures is not a mount.
    ///
    /// **What it spends:** one allocation per capture, of which § 3 admits as
    /// many as the `scan` has `*`s — one, for the tenant-per-directory layout
    /// § 7 exists to serve.
    mount_captures: Box<[Box<str>]>,
    /// One entry per header field line, in arrival order, name first.
    ///
    /// **A list rather than a map, for [`DeclaredHeader`]'s reason read the
    /// other way round.** A request may carry a field name more than once and
    /// the values are not interchangeable — `X-Forwarded-For` and `Via` are
    /// ordered, `Accept` is a set — so a map would answer with the last line
    /// and lose the rest, which is the silent-wrong-answer shape rather than a
    /// storage choice. Nothing here folds two lines together: whether one name
    /// standing twice is read as one combined value or as two is
    /// `nvs_stdlib::request`'s, and it answers both.
    ///
    /// A name is a token and so is text; a **value is bytes**, because RFC 9110
    /// § 5.5 still admits `obs-text` and refusing one is the door's decision to
    /// make rather than this carrier's. A Novis `string` is bytes either way,
    /// so nothing downstream pays for it.
    headers: Vec<(Box<str>, Box<[u8]>)>,
    /// The body still on the wire, as [`RequestBody`] — never its bytes — and
    /// `None` for a request that arrived without one.
    ///
    /// The three fields above are read off the request *line* and the head, both
    /// of which are bounded before a mount is even selected. A body is not: ADR
    /// 0105 § 5 lets one be `upload_total` large, which is `"256M"` by default
    /// and `"2G"` at its ceiling, so what is held here decides the resident cost
    /// of every in-flight request and, through ADR 0106 § 13's arithmetic, the
    /// number of requests this process may admit at once.
    body: Option<Box<dyn RequestBody>>,
    /// The address the request came from, as ADR 0097 § 6's walk decided it —
    /// the socket's own peer, or what a *trusted* proxy said instead.
    ///
    /// `None` for a peer that has no address at all: a Unix-domain socket that
    /// forwarded nothing. That is a value `Core\Request::clientIp()` has to be
    /// able to answer, and inventing `"0.0.0.0"` for it would be the repair
    /// [ADR 0095](/docs/adr/0095-ambiguous-input-is-refused-never-repaired.md)
    /// forbids.
    ///
    /// An [`IpAddr`] and not the text of one, so that a request nobody asks
    /// pays nothing: this is two words held inline, and the string spelling is
    /// built by the member that answers, on the calls that ask for it.
    ///
    /// **It is not "everything on it is `tainted`"'s exception.** A forwarded
    /// address is a header a proxy wrote and is untrusted on the same terms as
    /// the rest; what the walk decides is *who was allowed to say it*, not that
    /// what they said is safe to concatenate.
    client: Option<IpAddr>,
    /// The scheme this request effectively arrived over — the walk's other
    /// answer, and `Http` for every request until a trusted proxy asserts
    /// otherwise. [`Scheme`] owns why it is two named values.
    scheme: Scheme,
    /// [ADR 0105](/docs/adr/0105-an-uploaded-file-is-a-stream-and-there-is-one-way-to-receive-it.md)
    /// §§ 1-2's parse of that body, once `Core\Request::files()` has named one
    /// — **type-erased**, because the parse is `nvs_stdlib::multipart`'s and
    /// this crate is below it.
    ///
    /// It lives here for [`Self::claimed_by`]'s reason: what a parse is *of* is
    /// the request. `files()` builds it, every `advance()` of the walk it
    /// answers reaches it again through a different value, and `post()` will
    /// read the form fields it buffered on the way past — so none of the three
    /// can be its home and the carrier they share is.
    ///
    /// [`Any`](std::any::Any) rather than a trait declared here, on
    /// [`Self::claimed_by`]'s argument again: a trait would be this crate
    /// holding a roster of what a multipart parse may be asked, which is a copy
    /// of `nvs_stdlib`'s surface kept one crate below it and free to drift.
    /// What this carrier knows is that the request has a parse and that it
    /// outlives any one call into `Core` — nothing else, and nothing else is
    /// needed to store it.
    ///
    /// A single `Option` rather than the keyed table [`Ctx::hold_open_file`]
    /// holds: that table exists because a request may have many files open at
    /// once, and a request has exactly one body and therefore one parse of it.
    ///
    /// **What it spends:** one pointer per request, plus — only for a request
    /// that actually walked its parts — what `nvs_stdlib::multipart`'s own doc
    /// accounts for, which is one wire chunk and one delimiter's tail.
    parts: Option<Box<dyn std::any::Any>>,
    /// The body `Core\Request::post()` read, verbatim, for a request that did
    /// not declare a multipart one.
    ///
    /// It is here because `post()` answers **one named field per call** while
    /// the body it reads is a stream that can be pulled once: the second
    /// `post('description')` on a request would otherwise read an exhausted
    /// supplier and answer `null` for a field the peer sent. A multipart body
    /// needs nothing here — [`Self::parts`] already holds ADR 0105 § 2's
    /// buffered fields, which is the same fact stored where that parse put it.
    ///
    /// The bytes rather than the parsed array, so that this crate holds no
    /// value of the program's and nothing here has a reference to release when
    /// the request ends. `Core\Request::query` re-parses per call over the
    /// query string for exactly that reason, and this is the same trade one
    /// field along.
    ///
    /// **What it spends:** the body's own bytes, resident until the request
    /// ends, only for a request whose program called `post()` — bounded by
    /// `[limits] request_body`, which ADR 0105 § 2 makes the cap on form field
    /// text, and O(in-flight).
    form: Option<Box<[u8]>>,
    /// Which member has read the body, once one has — the name it spells
    /// itself, so a refusal can say what already took it.
    ///
    /// `docs/spec/01-core-library.md` § 15 makes `body`, `bodyStream` and
    /// `files` exclusive on one request, and this field is the whole of that
    /// rule. `post` is the fourth member that takes the claim and the only one
    /// that reads the name back to *join* rather than to refuse — ADR 0105
    /// § 2's form fields being what a `files` walk sets aside — which is
    /// `nvs_stdlib::request`'s `claim_form` and nothing this crate decides.
    /// **It lives on the carrier rather than on any of the three**,
    /// because what is exclusive is the *request*: each of them consumes the
    /// same stream, so a record kept by one of them could not see the other two
    /// — and the three are two crates apart, `nvs_stdlib::request` owning the
    /// first two and the parts the third yields being ADR 0105's own machinery.
    ///
    /// A name rather than a `bool` or an enum of three: the refusal is only
    /// worth raising if it says which reading already happened, since the
    /// program's bug is that it wrote two of them and it needs to know which one
    /// to delete. An enum here would be this crate holding a roster of stdlib
    /// members, which is the dependency this field's whole design avoids.
    ///
    /// Set even where no body arrived. What is exclusive is the *reading*, not
    /// the bytes: a request with an empty body still has one program-visible way
    /// of having been read, and letting the second call through on a request
    /// that happened to carry nothing would make the rule depend on what the
    /// peer sent.
    claimed_by: Option<&'static str>,
    /// [ADR 0102](/docs/adr/0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md)
    /// § 1's match: the row this request selected out of the program's table,
    /// and the captures it filled.
    ///
    /// **It is on the carrier because it is a fact about the request**, and
    /// because § 1's whole rule is that the match is taken *once*, before any
    /// application code, and travels from there — a field on the context would
    /// be a place a second match could be written from inside the program.
    /// [`Ctx::routes`] holds the table it was taken against, which is the
    /// program's and not the request's.
    ///
    /// `None` is "nothing matched" and is also every carrier nobody matched
    /// for: a program run off the command line, and a request whose unit
    /// declared no route. All three answer alike, which is § 1's `null` —
    /// nothing here dispatches, so there is no fourth case to tell apart.
    ///
    /// **What it spends:** one pointer per request, plus — only for a matched
    /// one — an `Arc` bump on the row and one `String` per capture.
    /// [`crate::routes`] accounts for the rest.
    route: Option<crate::routes::Match>,
    /// [ADR 0076](/docs/adr/0076-observability-export.md) § 2's trace,
    /// as the door read it off the arrived `traceparent` — the trace continued
    /// when the header was one this understands, and the root it drew instead
    /// when it was not.
    ///
    /// **It is on the carrier for [`Self::route`]'s reason**: which trace a
    /// request belongs to is a fact about the request, decided once before any
    /// application code and travelling from there. [`Ctx::trace_context`] is
    /// where a program reads it and [`Ctx::set_inbound`] is the one place the
    /// two meet.
    ///
    /// `None` is **not** "no trace". [`Ctx::new`] draws a root eagerly, so a
    /// carrier with nothing here leaves that root standing — which is what a
    /// program run off the command line and a test's own carrier both want, and
    /// is why this is an `Option` rather than a [`crate::TraceContext`]. What
    /// the absence records is that no door asked, never that the request has no
    /// id: § 2 has one exist for every request whatever the sampling decision.
    ///
    /// **What it spends:** 26 bytes per request, held no longer than the
    /// carrier.
    trace: Option<crate::trace_context::TraceContext>,
    /// [ADR 0083](/docs/adr/0083-persistent-connections-are-isolates.md) § 1's
    /// upgrade slot, for a request a connection offered one to, and `None` for
    /// every other carrier — [`Self::offer_upgrade`] owns which is which and
    /// [`UpgradeSlot`] owns why it is a shared cell.
    ///
    /// The one field here that is not a fact about what arrived, which the
    /// module doc's last paragraph is the home of.
    ///
    /// **What it spends:** one pointer per request, and — only for a request
    /// running on an upgradable connection — one small allocation the connection
    /// holds the other reference to.
    upgrade: Option<UpgradeSlot>,
    /// [ADR 0083](/docs/adr/0083-persistent-connections-are-isolates.md) § 5's
    /// SSE cell, for a request a server offered one to, and `None` for every
    /// other carrier — [`Self::offer_sse`] owns which is which and [`SseSlot`]
    /// owns why it is a second cell rather than a second use of the first.
    ///
    /// The other field here that is not a fact about what arrived.
    ///
    /// **What it spends:** what the field above spends, on every request the
    /// server runs rather than only on an upgradable one — one pointer, plus
    /// one small allocation the connection holds the other reference to.
    sse: Option<SseSlot>,
}

impl std::fmt::Debug for Inbound {
    /// Written out rather than derived, because a [`RequestBody`] is a socket
    /// mid-read and there is nothing to print of it but whether one is here.
    /// Reading it to say more would consume what the program is owed.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Inbound")
            .field("method", &self.method)
            .field("path", &self.path)
            .field("query", &self.query)
            .field("mount_prefix", &self.mount_prefix)
            .field("mount_captures", &self.mount_captures)
            .field("headers", &self.headers)
            .field("client", &self.client)
            .field("scheme", &self.scheme)
            .field("body", &self.body.is_some())
            .field("parts", &self.parts.is_some())
            .field("upgrade", &self.upgrade.is_some())
            .field("sse", &self.sse.is_some())
            .finish()
    }
}

impl Inbound {
    /// The three facts a request arrives with, as whoever accepted it read
    /// them, and no headers yet — [`Self::push_header`] adds those.
    #[must_use]
    pub fn new(method: &str, path: &str, query: &str) -> Self {
        Self {
            method: method.into(),
            path: path.into(),
            query: query.into(),
            // What a carrier nobody mounted says, and the same thing a mount at
            // `/` with no glob says: nothing was stripped and nothing was
            // captured. [`Self::set_mount`] is the only way to anything else.
            mount_prefix: "".into(),
            mount_captures: Box::new([]),
            headers: Vec::new(),
            body: None,
            // The fail-closed pair, and both are what a carrier built by
            // something that never asked the question says: no peer to name,
            // and a plaintext scheme. `Scheme::Https` is a claim, so it is
            // never a default — [`Self::set_peer`] is the only way to it.
            client: None,
            scheme: Scheme::Http,
            parts: None,
            form: None,
            claimed_by: None,
            // Nothing has matched yet, which is what every carrier says until
            // the door that has a table says otherwise.
            route: None,
            // No door has read a `traceparent` for this carrier, which leaves
            // whatever root its context drew standing — the field's own doc
            // owns why that is not "no trace".
            trace: None,
            // Nothing has offered this request an upgrade, which is what every
            // carrier says until a connection that can be upgraded says
            // otherwise — and is why `Core\Socket::upgrade` throws on one.
            upgrade: None,
            // And nothing has offered it § 5's cell either, which a CLI program
            // and a `spawn script` child both say for good — a server answering
            // a request is what offers one, and this is why `Core\Sse::upgrade`
            // refuses off everything else.
            sse: None,
        }
    }
    /// Records who the request came from, as ADR 0097 § 6's walk decided it.
    ///
    /// Called at most once, by whoever accepted the request, beside the
    /// [`Self::push_header`] calls and before the program runs. Both facts
    /// arrive together because one walk answers both: a proxy trusted to state
    /// the client address is the same proxy trusted to state the scheme, and
    /// setting them apart would let a carrier hold half of one answer.
    pub fn set_peer(&mut self, client: Option<IpAddr>, scheme: Scheme) {
        self.client = client;
        self.scheme = scheme;
    }
    /// The address the request came from, and `None` for a peer that has no
    /// address — the field's own doc owns what that is.
    #[must_use]
    pub fn client(&self) -> Option<IpAddr> {
        self.client
    }
    /// The scheme this request effectively arrived over.
    #[must_use]
    pub fn scheme(&self) -> Scheme {
        self.scheme
    }

    /// Records ADR 0102 § 7's mount: the prefix [`Self::path`] no longer
    /// carries, and the captures the row selecting this request was expanded
    /// from.
    ///
    /// Called at most once, beside [`Self::set_peer`] and before the program
    /// runs — `nvs_server::mount::carry` is the one caller and the home of that
    /// direction. Both facts arrive in one call for [`Self::set_peer`]'s
    /// reason: they are two fields of one row, and a carrier holding half of a
    /// mount would answer `Core\Request::mount()` with a prefix and somebody
    /// else's captures.
    ///
    /// Nothing here interprets either one. § 7 states the captures as
    /// `tainted string` and this crate has no qualifier to write them with —
    /// the taint is declared on the member that hands them over, which is
    /// `nvs_stdlib::request`'s registry row.
    pub fn set_mount(&mut self, prefix: &str, captures: &[String]) {
        self.mount_prefix = prefix.into();
        self.mount_captures = captures
            .iter()
            .map(|capture| capture.as_str().into())
            .collect();
    }

    /// The prefix ADR 0097 § 4 step 2 took off [`Self::path`], and `""` where
    /// nothing did — the field's own doc owns why those are one answer.
    #[must_use]
    pub fn mount_prefix(&self) -> &str {
        &self.mount_prefix
    }

    /// ADR 0097 § 3's glob captures of the mount that selected this request, in
    /// order, and empty where it had none.
    #[must_use]
    pub fn mount_captures(&self) -> &[Box<str>] {
        &self.mount_captures
    }

    /// Records ADR 0102 § 1's match, which whoever accepted the request took
    /// against the program's own table.
    ///
    /// Called at most once, beside [`Self::set_peer`] and before the program
    /// runs — `nvs_server::route` is the one caller and the home of that
    /// direction. Nothing during the request can move it: matching again would
    /// be the second match § 1 exists to remove.
    pub fn set_route(&mut self, matched: crate::routes::Match) {
        self.route = Some(matched);
    }

    /// The match this request arrived with, and `None` where nothing matched —
    /// the field's own doc owns why the three absences are one case.
    #[must_use]
    pub fn route(&self) -> Option<&crate::routes::Match> {
        self.route.as_ref()
    }

    /// Records ADR 0076 § 2's trace, as the door decided it from the arrived
    /// `traceparent`.
    ///
    /// Called at most once, beside [`Self::set_peer`] and before the program
    /// runs — `nvs_server::trace` is the one caller and the home of why the
    /// header is read at the door rather than by whichever subsystem asks for
    /// an id first. A second write mid-request would split one request across
    /// two traces, which is the same rule [`Ctx::set_trace_context`] states of
    /// this fact one layer up.
    pub fn set_trace_context(&mut self, trace: crate::trace_context::TraceContext) {
        self.trace = Some(trace);
    }

    /// The trace the door decided for this request, and `None` where no door
    /// asked — the field's own doc owns why that is not "no trace".
    #[must_use]
    pub fn trace_context(&self) -> Option<crate::trace_context::TraceContext> {
        self.trace
    }
    /// The verb, verbatim.
    #[must_use]
    pub fn method(&self) -> &str {
        &self.method
    }
    /// The mount-stripped request path.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }
    /// The raw query string, without the `?`.
    #[must_use]
    pub fn query(&self) -> &str {
        &self.query
    }
    /// Adds one header field line, which joins whatever is already here rather
    /// than replacing a line of the same name — the inbound half of
    /// [`Ctx::append_header`], and never of [`Ctx::declare_header`].
    ///
    /// Called once per field line by whoever accepted the request, in the order
    /// the lines arrived, before the program runs. Order is part of the value:
    /// RFC 9110 § 5.3 makes two lines of one name equivalent to one comma-joined
    /// value *in the order received*, so a writer that sorted them would have
    /// changed what the peer said.
    pub fn push_header(&mut self, name: &str, value: &[u8]) {
        self.headers.push((name.into(), value.into()));
    }
    /// Every header field line, in arrival order, as `(name, value)`.
    ///
    /// One entry per line and not per name: a reader that wants the two folded
    /// together does the folding, and one that wants them apart cannot get them
    /// back from a fold.
    pub fn headers(&self) -> impl ExactSizeIterator<Item = (&str, &[u8])> {
        self.headers.iter().map(|(name, value)| (&**name, &**value))
    }
    /// Gives this carrier the body the peer is still sending, which nothing
    /// has read a byte of yet.
    ///
    /// Called at most once, by whoever accepted the request, beside the
    /// [`Self::push_header`] calls and before the program runs. A request that
    /// arrived with no body — a `GET`, or a `POST` with `Content-Length: 0` —
    /// leaves this unset, and that is what [`Self::body`] answering `None`
    /// means: not "the body is empty" but "there is no body to read", which is
    /// the distinction RFC 9110 § 8.6 draws and the one a `Core\Request` member
    /// reports differently.
    pub fn set_body(&mut self, body: Box<dyn RequestBody>) {
        self.body = Some(body);
    }
    /// The body, to pull chunks from — `None` where the request carried none.
    ///
    /// **A borrow rather than a take, and a mutable one**, which is the whole
    /// difference between this and [`Self::headers`]: pulling a chunk advances
    /// the wire, so a reader that could be handed out twice would be two
    /// programs consuming one stream. `&mut self` on the carrier is the half of
    /// spec § 15's exclusivity — `body`, `bodyStream` and `files` are exclusive
    /// on one request — that no member can talk its way around; the other half
    /// is [`Self::claim_body`], and a member that reads the body owes a call to
    /// it before the first pull.
    ///
    /// This is deliberately *not* the place the claim is made. A `bodyStream`
    /// pulls one chunk per `advance()` and so borrows here once per chunk, where
    /// it claims once for the whole walk — so a claim taken on the borrow would
    /// refuse a program its second chunk.
    pub fn body(&mut self) -> Option<&mut (dyn RequestBody + 'static)> {
        self.body.as_deref_mut()
    }
    /// Records that `member` is reading this request's body, or names the one
    /// that already is.
    ///
    /// Spec § 15's exclusivity, enforced: `body`, `bodyStream` and `files` each
    /// consume the stream the other two would read, so the second of them on one
    /// request is a program bug. Answering it empty — which is what an exhausted
    /// stream says on its own — would report "the peer sent nothing" for a
    /// request whose bytes the program had already been handed, and that is the
    /// silent-wrong-answer this whole carrier is written against.
    ///
    /// The [`Err`] is the claiming member's own name, for the caller to put in
    /// a message — or to read, `post` being the member that answers some of
    /// those names by joining the reading rather than by refusing. This crate
    /// raises nothing and decides nothing about which is which; the members are
    /// `nvs_stdlib`'s and so is the rule.
    ///
    /// # Errors
    ///
    /// The name of the member that claimed the body first, where one has.
    pub fn claim_body(&mut self, member: &'static str) -> Result<(), &'static str> {
        match self.claimed_by {
            Some(first) => Err(first),
            None => {
                self.claimed_by = Some(member);
                Ok(())
            }
        }
    }
    /// Whether a body arrived at all, without reading it or taking a mutable
    /// borrow to ask.
    #[must_use]
    pub fn has_body(&self) -> bool {
        self.body.is_some()
    }
    /// Gives this carrier the parse of its body that `Core\Request::files()`
    /// built, for [`Self::parts_mut`] to hand back on every later call.
    ///
    /// Called at most once per request: `files()` is the only member that names
    /// a parse, and [`Self::claim_body`] is what makes it callable once.
    pub fn hold_parts(&mut self, parts: Box<dyn std::any::Any>) {
        self.parts = Some(parts);
    }
    /// The parse and the body it reads, borrowed **together** — `None` unless
    /// this request has both.
    ///
    /// One accessor rather than two, because the two are borrowed at the same
    /// moment and neither outlives the call: `nvs_stdlib::multipart` takes the
    /// body per call rather than holding one — its own module doc says why —
    /// so every step of a parse needs `&mut` on both at once, and two methods
    /// could not hand that out. They are two fields of one struct, so this is a
    /// borrow the compiler can see through where a pair of calls would not be.
    ///
    /// `None` for a request that carried no body at all, which is a walk over
    /// no parts rather than an error: RFC 9110 § 8.6's "there is no body" is
    /// the same fact [`Self::body`] answers `None` for, and a multipart parse
    /// of it would be a parse of nothing.
    pub fn parts_mut(
        &mut self,
    ) -> Option<(
        &mut (dyn std::any::Any + 'static),
        &mut (dyn RequestBody + 'static),
    )> {
        Some((self.parts.as_deref_mut()?, self.body.as_deref_mut()?))
    }
    /// Gives this carrier the body `Core\Request::post()` read, for every later
    /// call of that member to parse again — [`Self::form`] owns why.
    ///
    /// Called at most once per request: `post()` reads the body behind
    /// [`Self::claim_body`], so the call that fills this is the only one that
    /// finds it empty.
    pub fn hold_form(&mut self, form: Box<[u8]>) {
        self.form = Some(form);
    }
    /// The body [`Self::hold_form`] was given, or `None` where `post()` has not
    /// read one.
    #[must_use]
    pub fn form(&self) -> Option<&[u8]> {
        self.form.as_deref()
    }

    /// Offers ADR 0083 § 1's upgrade to this request: the slot
    /// `Core\Socket::upgrade` writes a prepared connection isolate into, whose
    /// other half is held by whoever is going to start it.
    ///
    /// Called at most once, by whoever accepted the request, beside
    /// [`Self::set_peer`] and before the program runs — and **only for a request
    /// a connection can actually be upgraded out of**, which for the built-in
    /// server is an h1 request `hyper` framed an upgrade for
    /// (`nvs_server::serve_connection`). Everything else is left with no slot on
    /// purpose: that is what makes the member throw off a CLI program, inside a
    /// `spawn script` child and on an ordinary request alike, rather than
    /// preparing an isolate nothing would ever start.
    pub fn offer_upgrade(&mut self, slot: UpgradeSlot) {
        self.upgrade = Some(slot);
    }
    /// The slot [`Self::offer_upgrade`] left, and `None` for a request no
    /// connection offered one for.
    ///
    /// A shared borrow is all a writer needs, because the slot is a cell: what
    /// `Core\Socket::upgrade` does with it is [`UpgradeSlot::fill`], which takes
    /// `&self` for exactly the reason the connection's own half does.
    #[must_use]
    pub fn upgrade_slot(&self) -> Option<&UpgradeSlot> {
        self.upgrade.as_ref()
    }
    /// Offers ADR 0083 § 5's SSE cell to this request: the slot
    /// `Core\Sse::upgrade` writes a prepared connection isolate into, which the
    /// connection starts against the response body it is still sending.
    ///
    /// **Called for every request a server answers**, and not only for one an
    /// upgrade could be framed out of — that is § 5's own sentence and the
    /// whole difference between this door and [`Self::offer_upgrade`]'s, since
    /// an event stream needs nothing of the connection but the response the
    /// request already has.
    pub fn offer_sse(&mut self, slot: SseSlot) {
        self.sse = Some(slot);
    }
    /// The cell [`Self::offer_sse`] left, and `None` for a carrier no server
    /// offered one to.
    ///
    /// A shared borrow, for [`Self::upgrade_slot`]'s reason: the cell is what
    /// serialises the two halves, so writing into it takes `&self` too.
    #[must_use]
    pub fn sse_slot(&self) -> Option<&SseSlot> {
        self.sse.as_ref()
    }
}

/// [ADR 0083](/docs/adr/0083-persistent-connections-are-isolates.md) § 1's
/// connection isolate, prepared and not yet started: the program it runs, and
/// the argument that has already crossed to it.
///
/// **It is exactly what `nvs_host::Isolate::new` takes**, and that is the whole
/// of the type — a connection is not a fourth kind of isolate but § 1's root one
/// with the two things every isolate needs decided a task earlier. Why they are
/// decided there is `nvs_stdlib::socket`'s module doc: the capability, the
/// argument and the code are all the *request's* to refuse, and each of the
/// three answers `null` or throws on a context that never ran the program.
///
/// **Consumes one reference to `args`**, which the connection hands on to the
/// spawn at the far end; a prepared upgrade that is dropped without being taken
/// leaks that reference, the same obligation `nvs_host::Isolate::new` and
/// [`crate::host::Job`] already document for a child that is never run. The
/// connection takes the slot unconditionally after joining the request, so the
/// only path that drops one is a connection that died before it got there — and
/// a connection that takes one and then declines to start it says so with
/// [`Self::discard`] rather than by dropping it.
///
/// **What it spends:** one boxed closure and one 16-byte value per upgrade, plus
/// the argument graph the copy behind it holds — accounted where that copy is
/// made, which is `nvs_stdlib::socket`'s "copied twice per upgrade".
pub struct Upgrade {
    /// What the connection's isolate runs: the resolver's program for a path
    /// entry, or the closure `nvs_stdlib::socket` built over an already-crossed
    /// callable for a static method one. Both of ADR 0006's two entry forms
    /// arrive here as one thing, which is what makes them one isolate.
    program: crate::script::Program,
    /// The argument, on this side of the boundary already — the copy the request
    /// made so that ADR 0023 § 2's refusal could still be a throw the program
    /// catches.
    args: Value,
}

impl std::fmt::Debug for Upgrade {
    /// Written out for [`Inbound`]'s reason: a [`crate::script::Program`] is a
    /// closure and has no `Debug`, and there is nothing to print of it but that
    /// one is here.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Upgrade")
            .field("args", &self.args)
            .finish_non_exhaustive()
    }
}

impl Upgrade {
    /// Prepares one. **Consumes one reference to `args`** — the type's own doc
    /// owns what that obligates.
    #[must_use]
    pub fn new(program: crate::script::Program, args: Value) -> Self {
        Self { program, args }
    }
    /// Hands both halves over, which is the one thing anybody does with one:
    /// they are `nvs_host::Isolate::new`'s first two arguments and are never
    /// read apart.
    #[must_use]
    pub fn into_parts(self) -> (crate::script::Program, Value) {
        (self.program, self.args)
    }
    /// Gives back what a dropped one would leak — the reference to `args` this
    /// type's own doc says it consumed — and drops the program unrun.
    ///
    /// **Here rather than at the connection that decides to discard one**,
    /// because releasing a [`Value`] is `unsafe` and `nvs-server` is
    /// `forbid(unsafe_code)`: a door that had to refuse a prepared isolate
    /// would otherwise have no spelling for it but the leak. The one caller
    /// today is [`SseSlot::fill`]'s "decided where the response is written" — a
    /// request that asked for a socket *and* an event stream has asked for two
    /// responses, and neither isolate is started.
    ///
    /// Refusing where the cells are read rather than where they are filled is
    /// what keeps each cell ignorant of the other; what it costs is that the
    /// argument crossing already happened, and this is where it is paid back.
    pub fn discard(self) {
        #[expect(
            unsafe_code,
            reason = "`Upgrade::new` consumed exactly one reference to `args` \
                      and this is the only place it is given back unrun"
        )]
        // SAFETY: nothing else points at that reference — the connection is the
        // only reader of a cell, it has taken this upgrade out of it, and it is
        // discarding rather than starting it.
        unsafe {
            self.args.release();
        }
    }
}

/// The place a prepared [`Upgrade`] is left, shared between the request that
/// prepares it and the connection that starts it.
///
/// **A cell with two halves rather than a value on the carrier**, because the
/// two sides are a task apart and only one of them is alive at a time: the
/// request writes through its `nvs_runtime::Inbound`, the request ends and its
/// context — the carrier with it — is dropped, and the connection reads through
/// the half it kept. A field the connection had to reach back into a finished
/// request for would be the ordering ADR 0083 § 1 forbids, spelled as something
/// to remember rather than as something the code can express.
///
/// Cloning one is what "offering" it is: both halves name the same cell, and a
/// slot nobody filled costs one allocation on a connection that never upgraded.
#[derive(Clone, Debug, Default)]
pub struct UpgradeSlot(Rc<RefCell<Option<Upgrade>>>);

impl UpgradeSlot {
    /// An empty slot, whose other half is a [`Clone`] of it.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Records the prepared isolate.
    ///
    /// `&self` rather than `&mut self` because both halves are shared handles
    /// and neither is the owner; the cell is what serialises them, and nothing
    /// holds a borrow of it across a call.
    ///
    /// # Errors
    ///
    /// The upgrade handed back, unchanged, where one is already recorded — a
    /// program that called `Core\Socket::upgrade` twice on one request. It is
    /// returned rather than dropped so that the refusal happens where there is
    /// still a context to release the argument with and a `catch` to report it
    /// to; a fill that overwrote would leak the first argument and open a
    /// connection the program did not think it had asked for.
    pub fn fill(&self, upgrade: Upgrade) -> Result<(), Upgrade> {
        let mut slot = self.0.borrow_mut();
        if slot.is_some() {
            return Err(upgrade);
        }
        *slot = Some(upgrade);
        Ok(())
    }
    /// Takes what was recorded, leaving the slot empty — the connection's half,
    /// and `None` for a request that never upgraded.
    #[must_use]
    pub fn take(&self) -> Option<Upgrade> {
        self.0.borrow_mut().take()
    }
    /// Whether an upgrade has been recorded, without taking it.
    #[must_use]
    pub fn is_filled(&self) -> bool {
        self.0.borrow().is_some()
    }
}

/// The place a prepared [`Upgrade`] is left for a connection that is **still
/// sending the response** —
/// [ADR 0083](/docs/adr/0083-persistent-connections-are-isolates.md) § 5's SSE
/// hand-over, and the second of the carrier's two cells.
///
/// **A separate type rather than a second [`UpgradeSlot`]**, because the two
/// hand-overs are different objects arriving at different moments: a WebSocket
/// upgrade takes the socket and starts after the request's future has ended,
/// while an SSE isolate writes into the body of an ordinary `200
/// text/event-stream` the connection has not finished sending. § 5 refuses the
/// one slot that carried both, and this type is the whole of that refusal: a
/// connection reading a shared slot would have to ask it a *kind* before it
/// could use it, which is a tag standing in for a distinction the types
/// already make.
///
/// What the two do share is the payload. An [`Upgrade`] is what either
/// `upgrade` member prepared — a root isolate's program and its argument — and
/// says nothing about which door it came through, which is why there is one of
/// those and two of these.
///
/// Everything else about the cell is [`UpgradeSlot`]'s, unchanged: both halves
/// name it, [`Self::fill`] refuses a second, and a cell nobody filled costs one
/// allocation on a request that asked for no event stream.
#[derive(Clone, Debug, Default)]
pub struct SseSlot(Rc<RefCell<Option<Upgrade>>>);

impl SseSlot {
    /// An empty cell, whose other half is a [`Clone`] of it.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    /// Records the prepared isolate.
    ///
    /// `&self` for [`UpgradeSlot::fill`]'s reason: both halves are shared
    /// handles and the cell is what serialises them.
    ///
    /// # Errors
    ///
    /// The upgrade handed back, unchanged, where one is already recorded — a
    /// program that called `Core\Sse::upgrade` twice on one request. It is
    /// returned rather than dropped so that the argument is released where
    /// there is still a context to release it with.
    ///
    /// **A request that filled both cells is not refused here.** One cell knows
    /// nothing of the other by construction, and asking for a socket and an
    /// event stream at once is a contradiction about the *response* — decided
    /// where the response is written, not in a cell that cannot see it.
    pub fn fill(&self, upgrade: Upgrade) -> Result<(), Upgrade> {
        let mut slot = self.0.borrow_mut();
        if slot.is_some() {
            return Err(upgrade);
        }
        *slot = Some(upgrade);
        Ok(())
    }
    /// Takes what was recorded, leaving the cell empty — the connection's half,
    /// and `None` for a request that asked for no event stream.
    #[must_use]
    pub fn take(&self) -> Option<Upgrade> {
        self.0.borrow_mut().take()
    }
    /// Whether an isolate has been recorded, without taking it.
    #[must_use]
    pub fn is_filled(&self) -> bool {
        self.0.borrow().is_some()
    }
}

/// The body of a served request, as chunks the program pulls one at a time —
/// [ADR 0105](/docs/adr/0105-an-uploaded-file-is-a-stream-and-there-is-one-way-to-receive-it.md)'s
/// stream, at the seam where it crosses into a crate that has never heard of
/// HTTP.
///
/// # A reader rather than the bytes, and § 5 is what decides it
///
/// The alternative was for whoever accepted the request to read the body to its
/// end and hand [`Inbound`] a `Box<[u8]>` beside its three strings, which is
/// what PHP does and what every part of this design would then be built on top
/// of. ADR 0105 § 5 rules it out arithmetically rather than as a preference.
/// That section states **two** caps because they measure two different things:
/// `request_body` (`"8M"`, ceiling `"64M"`) bounds *bytes parsed into memory*,
/// and `upload_total` (`"256M"`, ceiling `"2G"`) bounds *the total of a streamed
/// multipart body*. A door that buffered would collapse them into one, and into
/// the larger — the bytes are already resident by the time any member could
/// apply the tighter number, so `request_body` would be a check over something
/// already paid for, which is not a bound. What is left is a per-request
/// resident cost of `upload_total`, multiplied by `max_in_flight` by
/// [ADR 0106](/docs/adr/0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md)
/// § 13's arithmetic: a 2G ceiling against a memory budget divides the effective
/// admission ceiling to approximately nothing, so the honest configuration and
/// the working one stop being the same file.
///
/// § 6 closes the third option before it is asked: there is no temp file, so
/// "neither resident nor streamed" is not a place a body can be put. Buffered
/// or streamed is the whole choice, and only one of them is bounded — which is
/// exactly what `docs/plan/m7.md`'s load-bearing acceptance case asserts, a
/// multipart body far larger than any in-memory bound received in full at a
/// bounded resident **high-water mark**.
///
/// # A chunk is borrowed, not owned
///
/// [`Self::next_chunk`] answers a slice of the supplier's own buffer, valid
/// until the next pull — ADR 0105 § 3's "valid only while this part is the
/// iterator's current one", one layer down and as a lifetime rather than as
/// advice. A reader that wants to keep bytes copies them into the bound it
/// chose, which is the point at which a cap can be applied to a number that has
/// not been spent yet; a reader that is writing them to a destination
/// (§ 3's `saveTo`) never copies at all. Returning an owned `Vec` would have
/// charged every upload one allocation and one copy per chunk for a byte almost
/// none of them keep.
///
/// **What it spends:** whatever the supplier holds for one chunk, and nothing
/// per request beyond the `Box` on [`Inbound`]. O(in-flight) by construction,
/// because no implementation may accumulate: the contract below says a chunk is
/// invalidated by the next pull precisely so that none has to.
pub trait RequestBody {
    /// The next chunk of the body, `Ok(None)` at its end.
    ///
    /// Blocks the calling *task* — never the thread — for as long as the peer
    /// takes to send it. The supplier is whoever accepted the request and it
    /// owns how that wait is spelled; for the built-in server the puller is the
    /// request's own isolate, and that is a task **beside** the connection
    /// rather than a frame inside its poll — `nvs_server::serve_connection`'s
    /// service answers `Pending` while it runs
    /// ([ADR 0138](/docs/adr/0138-a-connection-future-is-driven-by-the-coroutine-that-owns-it.md)
    /// § 1) — so a pull parks that isolate and the connection's next poll is
    /// what delivers the bytes. Stated as the rule an implementation has to
    /// keep: **nothing may pull one from inside the poll of the very
    /// connection future that would deliver them.**
    ///
    /// Chunk boundaries are the wire's and mean nothing: a reader that needs a
    /// record, a line or a MIME boundary finds it across chunks and never
    /// assumes one arrives whole.
    ///
    /// # Errors
    ///
    /// A body that cannot be completed: the connection failed under it, the
    /// peer stopped short of its declared length, or ADR 0105 § 5's
    /// `upload_total` was crossed mid-stream by a body that declared no length.
    /// The message is for the member reading it to turn into its own throw —
    /// this seam classifies nothing, for [`Inbound`]'s own reason. **An `Err`
    /// ends the body**: nothing may be pulled after one, and a supplier that
    /// answered one has already given up on the connection.
    fn next_chunk(&mut self) -> Result<Option<&[u8]>, Box<str>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A [`RequestBody`] whose chunks are decided in advance — every part of
    /// the contract this crate can hold, in a crate with no socket in it. What
    /// a real supplier adds is only where the bytes come from.
    struct Canned {
        chunks: Vec<Vec<u8>>,
        at: usize,
    }

    impl Canned {
        fn of(chunks: &[&str]) -> Box<dyn RequestBody> {
            Box::new(Self {
                chunks: chunks.iter().map(|c| c.as_bytes().to_vec()).collect(),
                at: 0,
            })
        }
    }

    impl RequestBody for Canned {
        fn next_chunk(&mut self) -> Result<Option<&[u8]>, Box<str>> {
            if self.at >= self.chunks.len() {
                return Ok(None);
            }
            let at = self.at;
            self.at += 1;
            Ok(Some(&self.chunks[at]))
        }
    }

    /// [`RequestBody`]'s reason for existing, asserted as the thing it is:
    /// the carrier hands the body out a chunk at a time and holds none of it,
    /// so what is resident is one chunk and not the body.
    ///
    /// Chunk boundaries are the wire's, which is why the assertion is over the
    /// *joined* bytes and over the count of pulls separately — a supplier that
    /// split the same body differently is not a different body.
    #[test]
    fn a_carrier_hands_out_its_body_one_chunk_at_a_time_and_never_holds_it() {
        let mut inbound = Inbound::new("POST", "/upload", "");
        inbound.set_body(Canned::of(&["one ", "part ", "at a time"]));

        let mut joined = Vec::new();
        let mut pulls = 0;
        let body = inbound.body().expect("a body was set");
        while let Some(chunk) = body.next_chunk().expect("the canned body cannot fail") {
            joined.extend_from_slice(chunk);
            pulls += 1;
        }
        assert_eq!(String::from_utf8(joined).unwrap(), "one part at a time");
        assert_eq!(pulls, 3);

        // Past the end and staying there: a body is consumed once, and the
        // pull after the last one is not the start of a second reading.
        let body = inbound.body().expect("a body was set");
        assert!(body.next_chunk().expect("still not a failure").is_none());
    }

    /// [`Inbound::set_body`]'s distinction, which is RFC 9110 § 8.6's: a `GET`
    /// carries no body and a `POST` may carry an empty one, and a member that
    /// read them as the same thing would answer `""` where the honest answer is
    /// that there was nothing to read.
    #[test]
    fn a_request_with_no_body_is_not_a_request_whose_body_is_empty() {
        let mut none = Inbound::new("GET", "/", "");
        assert!(!none.has_body());
        assert!(none.body().is_none());

        let mut empty = Inbound::new("POST", "/", "");
        empty.set_body(Canned::of(&[]));
        assert!(empty.has_body());
        let body = empty.body().expect("an empty body is still a body");
        assert!(body.next_chunk().expect("no failure").is_none());
    }

    /// The two halves of the carrier reach a program on different terms —
    /// [`Ctx::inbound`] for what stays put, [`Ctx::inbound_mut`] for what
    /// advances a socket — and a context that answers one answers both.
    #[test]
    fn the_head_is_read_as_often_as_asked_and_the_body_exactly_once() {
        let mut ctx = Ctx::new(OutputSink::Sink);
        let mut inbound = Inbound::new("POST", "/upload", "part=1");
        inbound.push_header("content-type", b"text/plain");
        inbound.set_body(Canned::of(&["body"]));
        ctx.set_inbound(inbound);

        assert_eq!(ctx.inbound().map(Inbound::method), Some("POST"));
        assert_eq!(ctx.inbound().map(Inbound::query), Some("part=1"));
        assert_eq!(ctx.inbound().map(Inbound::query), Some("part=1"));

        let body = ctx
            .inbound_mut()
            .and_then(Inbound::body)
            .expect("the carrier arrived with one");
        assert_eq!(body.next_chunk().expect("no failure"), Some(&b"body"[..]));

        // And the head is still there afterwards: reading the body consumed
        // the body and nothing else.
        assert_eq!(ctx.inbound().map(Inbound::method), Some("POST"));
    }

    /// A [`Upgrade`] whose program records that it ran, which is the only thing
    /// a prepared isolate can be asked to prove on this side of the boundary:
    /// what crossed is a closure, so "the same one came back out" is that
    /// closure running and nothing else.
    fn prepared(marker: &Rc<std::cell::Cell<u8>>, mark: u8) -> Upgrade {
        let marker = Rc::clone(marker);
        Upgrade::new(
            Box::new(move |_ctx: &mut Ctx, _args: Value| {
                marker.set(mark);
                Value::null()
            }),
            Value::null(),
        )
    }

    /// ADR 0083 § 1's slot, as the two halves it is: a carrier nobody offered
    /// one to has none — which is the whole of how `Core\Socket::upgrade`
    /// refuses off a CLI program and inside a `spawn script` child — and a
    /// carrier that has one is writing into the cell the connection kept.
    ///
    /// The program is asserted by *running* it after the take, because the
    /// claim is that the connection ends up holding the thing the request
    /// prepared and not merely a slot that is occupied.
    #[test]
    fn an_offered_upgrade_reaches_the_half_the_connection_kept() {
        let mut nothing = Inbound::new("GET", "/", "");
        assert!(nothing.upgrade_slot().is_none());
        nothing.set_body(Canned::of(&[]));

        // The door's half, and the request's: one cell, two names.
        let connection = UpgradeSlot::new();
        let mut inbound = Inbound::new("GET", "/chat", "");
        inbound.offer_upgrade(connection.clone());

        let marker = Rc::new(std::cell::Cell::new(0));
        let slot = inbound.upgrade_slot().expect("the door offered one");
        assert!(!slot.is_filled());
        assert!(slot.fill(prepared(&marker, 1)).is_ok());

        // The request is over: its carrier is gone and the connection reads
        // what it left, which is § 1's ordering as the code expresses it.
        drop(inbound);
        assert!(connection.is_filled());
        let (program, args) = connection
            .take()
            .expect("the request filled it")
            .into_parts();
        assert_eq!(args.tag(), Some(crate::Tag::Null));
        assert!(!connection.is_filled());

        let mut ctx = Ctx::new(OutputSink::Sink);
        program(&mut ctx, Value::null());
        assert_eq!(marker.get(), 1);
    }

    /// [`UpgradeSlot::fill`]'s refusal, asserted on both sides of it: the second
    /// upgrade comes back to its caller rather than being dropped — there is an
    /// argument reference on it that only the request can release — and the
    /// first one is still what the connection takes.
    #[test]
    fn a_second_upgrade_is_handed_back_and_the_first_one_stands() {
        let slot = UpgradeSlot::new();
        let marker = Rc::new(std::cell::Cell::new(0));

        assert!(slot.fill(prepared(&marker, 1)).is_ok());
        let refused = slot
            .fill(prepared(&marker, 2))
            .expect_err("one request upgrades once");

        let mut ctx = Ctx::new(OutputSink::Sink);
        let (first, _) = slot.take().expect("the first fill stands").into_parts();
        first(&mut ctx, Value::null());
        assert_eq!(marker.get(), 1);

        // And the refused one is intact rather than half-consumed: the caller
        // is what releases its argument, so it has to arrive whole.
        let (second, _) = refused.into_parts();
        second(&mut ctx, Value::null());
        assert_eq!(marker.get(), 2);
    }

    /// ADR 0083 § 5's second cell, and the one thing that separates it from the
    /// first: it reaches a request no upgrade could have been framed out of.
    /// Asserted as the pair the door will write — a plain `GET` carrying an SSE
    /// cell and no upgrade slot — because "offered to every request" is a claim
    /// about the two fields being independent and about nothing else.
    #[test]
    fn the_sse_cell_is_offered_to_a_request_that_could_not_be_upgraded() {
        let mut inbound = Inbound::new("GET", "/events", "");
        assert!(inbound.sse_slot().is_none());

        // The connection's half, and the request's: one cell, two names.
        let connection = SseSlot::new();
        inbound.offer_sse(connection.clone());
        assert!(inbound.upgrade_slot().is_none());

        let marker = Rc::new(std::cell::Cell::new(0));
        let slot = inbound.sse_slot().expect("the door offered one");
        assert!(!slot.is_filled());
        assert!(slot.fill(prepared(&marker, 3)).is_ok());

        // The request is over, and the connection — still sending the response
        // this isolate writes into — reads what it left.
        drop(inbound);
        assert!(connection.is_filled());
        let (program, args) = connection
            .take()
            .expect("the request filled it")
            .into_parts();
        assert_eq!(args.tag(), Some(crate::Tag::Null));
        assert!(!connection.is_filled());

        let mut ctx = Ctx::new(OutputSink::Sink);
        program(&mut ctx, Value::null());
        assert_eq!(marker.get(), 3);
    }

    /// The two cells are independent, which is what makes them two: filling one
    /// leaves the other empty, and each refusal is its own. The single slot § 5
    /// refuses — one place carrying both hand-overs — passes every assertion in
    /// the test above this one and fails this one on its second line.
    #[test]
    fn each_cell_refuses_only_its_own_second_and_leaves_the_other_empty() {
        let socket = UpgradeSlot::new();
        let events = SseSlot::new();
        let marker = Rc::new(std::cell::Cell::new(0));

        assert!(events.fill(prepared(&marker, 1)).is_ok());
        assert!(!socket.is_filled());
        assert!(socket.fill(prepared(&marker, 2)).is_ok());
        assert!(events.is_filled());

        let refused = events
            .fill(prepared(&marker, 3))
            .expect_err("one request opens one event stream");

        let mut ctx = Ctx::new(OutputSink::Sink);
        let (first, _) = events.take().expect("the first fill stands").into_parts();
        first(&mut ctx, Value::null());
        assert_eq!(marker.get(), 1);

        // And the refused one is intact, for [`UpgradeSlot::fill`]'s reason.
        let (third, _) = refused.into_parts();
        third(&mut ctx, Value::null());
        assert_eq!(marker.get(), 3);
    }
}
