//! The request as it arrived.
//!
//! [`Inbound`] is what a host fills in before the request runs and what
//! `Core\Request` reads back — method, path, query, headers, the client
//! address and the [`Scheme`] a trusted proxy asserted
//! (`rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing`).
//!
//! [`InboundSpec`] is the other half: a request as a *description* — the bag
//! `rule:testing/in-process-request`'s `Core\Test::request` takes — and the one
//! place a described request becomes a carrier. A door reads its fields off a
//! socket and fills an [`Inbound`] in; everything else says what it wants and
//! builds one, which is why the four crates that build requests share this
//! type rather than each holding a builder of its own.
//!
//! The body is a [`RequestBody`] rather than bytes, and that is this file's one
//! real decision:
//! `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`'s finite
//! defaults mean a carrier hands the body out a chunk at a time and holds none
//! of it, so what a request has resident is one chunk and never the whole body.
//!
//! One thing here is not a fact the request arrived with, and that is
//! [`UpgradeSlot`]:
//! `rule:concurrency/a-connection-is-a-root-isolate`'s
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
//! **There are two such cells and not one.** `rule:concurrency/two-doors-one-isolate` is the home of why:
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
    ///
    /// **It takes either the carrier or a box of one**, which is not a
    /// convenience: this context boxes it regardless, so a caller already
    /// holding a `Box<Inbound>` — `nvs_host::Isolate`, whose own field is one
    /// so that a `nvs_server::Reply` stays small — hands the allocation on
    /// instead of moving the carrier twice to make a second.
    pub fn set_inbound(&mut self, inbound: impl Into<Box<Inbound>>) {
        let inbound = inbound.into();
        // `rule:observability/an-inbound-traceparent-is-continued`'s trace is the door's decision and rides on the carrier
        // ([`Inbound::set_trace_context`]), so this is where it becomes the
        // context's — the one write [`Self::set_trace_context`] describes, made
        // before the program runs and beside the rest of what a request arrives
        // with. A carrier that carries none leaves the root [`Self::new`] drew
        // standing, which is § 2's "an id exists for every request".
        if let Some(trace) = inbound.trace_context() {
            self.set_trace_context(trace);
        }
        self.inbound = Some(inbound);
    }
    /// The request this context is answering, or `None` where there is none.
    ///
    /// **A borrow rather than a take**, unlike [`Self::take_headers`] and its
    /// two neighbours: those are read once by the finish path and are gone, and
    /// this is read as many times as the program asks. `Core\Request`'s members
    /// are the only readers, and each of them turns `None` into
    /// `rule:security/request-state-throws-in-an-isolate`'s throw.
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
/// (`rule:http-server/two-deployments-and-nothing-a-proxy-owns`
/// ), so `Https` is only ever what a *trusted* proxy asserted through
/// `X-Forwarded-Proto` — § 6's walk is the one thing that decides it, and this
/// carrier holds its answer rather than re-deriving one.
///
/// Not a boolean, because the two values are named in the ADR and a `bool` at a
/// call site would need a comment saying which way round it goes.
///
/// It lives here, one crate below the door, because two readers a crate apart
/// need the same two values and a copy in each is two answers to one question:
/// `nvs_server::secure` conditions `rule:http-server/secure-headers-with-nothing-written`'s HSTS header on it, and
/// [`Inbound::scheme`] is what `Core\Request::scheme()` reads. `nvs_server`
/// re-exports this type rather than declaring its own.
///
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scheme {
    /// A plaintext connection, and what a trusted proxy asserted nothing about.
    Http,
    /// TLS, as a trusted proxy asserted it (`rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing`).
    Https,
}

/// The request line a context is answering, as it arrived off the wire —
/// what spec § 15's `Core\Request` reads and the only thing on a context that
/// came from outside the process.
///
/// **It interprets nothing.** The verb is the bytes the peer wrote, the path is
/// what is left of the target after `rule:http-server/a-request-resolves-in-five-steps` step 2 stripped the matched
/// mount's prefix, the query is the raw string after the `?` with no
/// percent-decoding and no bracket convention applied, and a header is one
/// entry per field line in the spelling and the order the peer sent it. Every
/// reading of those — which of `Core\Http\Method`'s cases a verb is, what
/// a query parameter's name means, that a field name matches without regard to
/// case — belongs to `nvs_stdlib::request`, because the rosters and the
/// conventions are that crate's and a second copy of either here would be a
/// second answer. This type is the carrier and nothing else, which is also what
/// lets it exist in a crate that has never heard of HTTP.
///
/// **Everything on it is `tainted`** in the sense
/// `rule:security/tainted-qualifier`
/// means: it is what a client sent. The qualifier itself is a *type*, so it is
/// carried by the registry rows of the members that read this and not by any
/// field here — there is no representation of a qualifier at runtime.
///
/// **What it spends:** a short allocation per field of the request line, plus
/// two per header field line and one growing vector to hold them, one more
/// allocation for a request that arrived with a body, and nothing at all for a
/// process serving none. **Not the body's bytes** — see [`RequestBody`]. The
/// peer costs no allocation at all: an address and a scheme are held inline, as
/// the words the door decided them as.
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
    /// `rule:routing/a-request-reads-its-mount`
    /// 's first half: the prefix `rule:http-server/a-request-resolves-in-five-steps` step 2 took off [`Self::path`]
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
    /// § 7's other half: `rule:http-server/a-mount-table-expands-at-boot`'s glob captures of the row that selected
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
    /// The fields above are read off the request *line* and the head, both of
    /// which are bounded before a mount is even selected. A body is not: ADR
    /// 0105 § 5 lets one be `upload_total` large, which is `"256M"` by default
    /// and `"2G"` at its ceiling, so what is held here decides the resident cost
    /// of every in-flight request and, through `rule:http-server/admission-is-arithmetic-not-a-number`'s arithmetic, the
    /// number of requests this process may admit at once.
    body: Option<Box<dyn RequestBody>>,
    /// The address the request came from, as `rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing`'s walk decided it —
    /// the socket's own peer, or what a *trusted* proxy said instead.
    ///
    /// `None` for a peer that has no address at all: a Unix-domain socket that
    /// forwarded nothing. That is a value `Core\Request::clientIp()` has to be
    /// able to answer, and inventing `"0.0.0.0"` for it would be the repair
    /// `rule:errors/ambiguous-input-refused`
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
    /// `rule:http-server/an-upload-is-received-only-through-files` and `rule:http-server/a-part-is-a-file-iff-it-carries-a-filename`
    /// 's parse of that body, once `Core\Request::files()` has named one
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
    /// The body a buffering reader pulled off the wire, verbatim, for a request
    /// that did not declare a multipart one.
    ///
    /// It is here because the body is a stream that can be pulled once, while
    /// `rule:http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it`
    /// lets more than one member read it. Both halves of that need this field:
    /// `post()` answers **one named field per call**, so the second
    /// `post('description')` on a request would otherwise read an exhausted
    /// supplier and answer `null` for a field the peer sent; and a buffering
    /// reader following another would find the same nothing. The first of them
    /// fills this, and every later one answers out of it.
    ///
    /// **A multipart body fills this only where a buffering reader read it
    /// first**, and that first reading decides which hold the request has.
    /// `post()` and `files()` parse the wire as it arrives, buffering
    /// `rule:http-server/a-part-is-a-file-iff-it-carries-a-filename`'s
    /// non-file fields into [`Self::parts`] and streaming past the rest — the
    /// only reading a body `[limits] upload_total` large has, since that rule
    /// charges the fields against `request_body` and the file parts against
    /// no cap this crate could apply here. So the hold either of them leaves
    /// is the decoded fields, and `body()` after one is refused on the
    /// sentence `rule:http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it`
    /// already carries for a walk: that hold has the fields and never the raw
    /// octets. `body()`, `json()` and `jsonAs()` fill *this* instead, under
    /// `request_body` like any other buffering reader, and a `post()`
    /// following one parses these octets rather than the wire they came off,
    /// which is drained by then and would answer a form with no fields in it.
    /// It reads them **in place** — a [`RequestBody`] over this slice, the
    /// shape [`Self::set_buffered_body`] already hands over — and in bounded
    /// pieces rather than one chunk, because `nvs_stdlib::multipart` holds a
    /// whole chunk in its own buffer and would otherwise hold the body twice.
    ///
    /// The bytes rather than the parsed array, so that what every reader
    /// shares is the one thing all of them agree on: a parse is one member's
    /// reading of these octets and not a second copy of them, which is why
    /// `Core\Request::query` re-parses per call over the query string too.
    /// [`Self::decoded`] is the single hold that is a value of the program's,
    /// and it is a memo one member keeps over these bytes rather than
    /// something any other reader answers out of.
    ///
    /// **What it spends:** the body's own bytes, resident until the request
    /// ends, only for a request whose program buffered one — bounded by
    /// `[limits] request_body`, which `rule:http-server/a-part-is-a-file-iff-it-carries-a-filename` makes the cap on form field
    /// text, and O(in-flight).
    held: Option<Box<[u8]>>,
    /// The document `Core\Request::json` decoded out of [`Self::held`], and the
    /// `maxDepth` it decoded at — `None` until that member has answered once.
    ///
    /// It is here because
    /// `rule:http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it`
    /// has that member answer the same document on every call, and decoding the
    /// held octets again per call would spend a whole parse arriving back at a
    /// value already resident. The rule's table is what says only `json()`
    /// keeps one: `Core\Json::decode` builds arrays and scalars, an array is
    /// copy-on-write, so a later caller can be handed a refcount bump safely —
    /// where `jsonAs<T>()` builds objects, two callers must never share one,
    /// and it therefore keeps nothing beyond the octets.
    ///
    /// **The depth is part of the hold, not a fact about the request.** The bag
    /// is the *call's*, so a call naming a different cap is a different
    /// question and is decoded again — which is what keeps a `{maxDepth: 2}`
    /// call throwing over a document a default-depth one already read. Only the
    /// first reading fills this, and a call at another depth answers and leaves
    /// it alone, so a request holds one document however many depths its
    /// program tried.
    ///
    /// This is the one reference this crate owns on a program's behalf, which
    /// is the whole of why [`HeldValue`] exists rather than a bare [`Value`]
    /// here. [`Ctx`]'s own `Drop` is where it is given back, before the
    /// live-list sweep and for the reason that body states.
    ///
    /// **What it spends:** the decoded document, resident until the request
    /// ends, only for a request whose program called `json()` — a bounded
    /// multiple of the octets [`Self::held`] already charges against
    /// `[limits] request_body`, and O(in-flight).
    decoded: Option<HeldValue>,
    /// Which member has read the body, once one has — the name it spells
    /// itself, so a refusal can say what already took it.
    ///
    /// It is the **first** reader's name and stays it, because that reading is
    /// what decided what the later ones can have: whether one of them is
    /// refused is this name read against [`Self::held`] and [`Self::parts`],
    /// which is [`Self::claim_body`]'s matrix and
    /// `rule:http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it`.
    /// **It lives on the carrier rather than on any one member**,
    /// because what is exclusive is the *request*: each of them consumes the
    /// same stream, so a record kept by one of them could not see the others
    /// — and they are two crates apart, `nvs_stdlib::request` owning the
    /// readers and the parts a `files` walk yields being `rule:http-server/an-upload-is-received-only-through-files`'s own machinery.
    ///
    /// A name rather than a `bool` or an enum of members: the refusal is only
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
    /// `rule:routing/matched-once-before-the-handler`
    /// 's match: the row this request selected out of the program's table,
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
    /// declared no route. They all answer alike, which is § 1's `null` —
    /// nothing here dispatches, so there is no further case to tell apart.
    ///
    /// **What it spends:** one pointer per request, plus — only for a matched
    /// one — an `Arc` bump on the row and one `String` per capture.
    /// [`crate::routes`] accounts for the rest.
    route: Option<crate::routes::Match>,
    /// `rule:observability/an-inbound-traceparent-is-continued`'s trace,
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
    /// `rule:concurrency/a-connection-is-a-root-isolate`'s
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
    /// `rule:concurrency/two-doors-one-isolate`'s
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
    /// `rule:concurrency/a-stream-that-outlives-its-request-is-a-connection`'s
    /// *first* spelling: the cell `Core\Response::stream` opens a body into, for
    /// a request a connection is framing a response for.
    ///
    /// The third field here that is not a fact about what arrived, and the one
    /// whose reader is alive at the same time as its writer — a connection takes
    /// the two upgrades above after the request has ended and takes this one
    /// while the request is still running.
    ///
    /// `None` for every carrier no connection built, which is what makes a
    /// stream off a CLI program an ordinary buffered body rather than a refusal
    /// (`nvs_stdlib::response`'s own gap 1).
    ///
    /// **What it spends:** one pointer per request, plus one small allocation
    /// the connection holds the other reference to.
    stream: Option<crate::stream::BodySlot>,
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
            .field("decoded", &self.decoded.is_some())
            .field("upgrade", &self.upgrade.is_some())
            .field("sse", &self.sse.is_some())
            .field("stream", &self.stream.is_some())
            .finish()
    }
}

/// A [`Value`] a request owns one reference to, with the `maxDepth` it was
/// decoded at, released when the request ends.
///
/// The carrier holds bytes and parses and deliberately no values —
/// [`Inbound::held`] says why — so this type is the whole machinery of the one
/// exception: `Core\Request::json`'s document is the program's value sitting on
/// something that is not the program's, and a reference nobody gave back would
/// be a leak per *request* rather than a bug in one program.
///
/// [`Drop`] is what makes forgetting it impossible, and it is deliberately not
/// the whole story. A carrier is a field of the [`Ctx`], and a field is dropped
/// after that type's own `Drop` body has run — which is after
/// `crate::object::sweep`, so a hold left to this `Drop` would be given back to
/// an allocation the sweep had already reached. `Ctx`'s `Drop` therefore takes
/// this hold itself, exactly as it takes `pending` and for the same reason, and
/// what remains here is the backstop for a carrier dropped anywhere else.
#[derive(Debug)]
pub struct HeldValue {
    /// The document, owning exactly one reference — the one [`Self::holding`]
    /// took and [`Drop`] gives back.
    value: Value,
    /// The `maxDepth` it was decoded at, so a call naming another one is
    /// answered by decoding rather than out of here.
    depth: u32,
}

impl HeldValue {
    /// Takes a reference of its own to `value`, to be given back when this is
    /// dropped.
    ///
    /// **A reference of its own rather than the caller's**, so the member that
    /// decoded the document hands nothing over and returns the very value it
    /// holds: one reference for the program and one for the request is what
    /// two independent lifetimes need, and making the hold take its own is what
    /// keeps the arithmetic at the one call site that can see both.
    ///
    /// # Safety
    ///
    /// `value`'s payload must be live, which for a caller holding a reference
    /// of its own it is.
    #[expect(
        unsafe_code,
        reason = "the payload's liveness is the caller's obligation to state"
    )]
    #[must_use]
    pub unsafe fn holding(value: Value, depth: u32) -> Self {
        // SAFETY: the caller guarantees the payload is live, which is all
        // `Value::retain` asks; the reference it adds is this type's, and
        // `Drop` is where it is given back.
        unsafe { value.retain() };
        Self { value, depth }
    }
    /// The `maxDepth` the held document was decoded at.
    #[must_use]
    pub fn depth(&self) -> u32 {
        self.depth
    }
    /// A further reference to the held document, for a caller to hand on and
    /// release itself.
    ///
    /// Safe where [`Self::holding`] is not: this value's payload is live for as
    /// long as the borrow is, which is the invariant that constructor asked its
    /// own caller for and the only one a retain needs.
    #[must_use]
    pub fn retained(&self) -> Value {
        #[expect(
            unsafe_code,
            reason = "this type owns a reference to that payload, so it is live \
                      for as long as the borrow handing out another is"
        )]
        // SAFETY: `Self::holding` took a reference that is still held, so the
        // allocation cannot have been freed while `&self` stands.
        unsafe {
            self.value.retain();
        }
        self.value
    }
}

impl Drop for HeldValue {
    /// Gives back the reference [`HeldValue::holding`] took.
    fn drop(&mut self) {
        #[expect(
            unsafe_code,
            reason = "this type owns exactly the one reference it took, and \
                      this is the only place it is given back"
        )]
        // SAFETY: every other reference to that payload was made by
        // `HeldValue::retained` and is owned by whoever it was handed to.
        unsafe {
            self.value.release();
        }
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
            held: None,
            decoded: None,
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
            // And no connection is framing a response for this carrier, which
            // is what leaves a streaming body buffered whole off a CLI program
            // instead of refused — the one of the three cells whose absence is
            // a fallback rather than a throw.
            stream: None,
        }
    }
    /// Records who the request came from, as `rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing`'s walk decided it.
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

    /// Records `rule:routing/a-request-reads-its-mount`'s mount: the prefix [`Self::path`] no longer
    /// carries, and the captures the row selecting this request was expanded
    /// from.
    ///
    /// Called at most once, beside [`Self::set_peer`] and before the program
    /// runs — `nvs_server::mount::carry` is the served caller and the home of
    /// that direction, and [`InboundSpec::build`] is the synthetic one, which
    /// carries whatever mount a test described. Both facts arrive in one call
    /// for [`Self::set_peer`]'s
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

    /// The prefix `rule:http-server/a-request-resolves-in-five-steps` step 2 took off [`Self::path`], and `""` where
    /// nothing did — the field's own doc owns why those are one answer.
    #[must_use]
    pub fn mount_prefix(&self) -> &str {
        &self.mount_prefix
    }

    /// `rule:http-server/a-mount-table-expands-at-boot`'s glob captures of the mount that selected this request, in
    /// order, and empty where it had none.
    #[must_use]
    pub fn mount_captures(&self) -> &[Box<str>] {
        &self.mount_captures
    }

    /// Records `rule:routing/matched-once-before-the-handler`'s match, which whoever accepted the request took
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
    /// the field's own doc owns why those absences are one case.
    #[must_use]
    pub fn route(&self) -> Option<&crate::routes::Match> {
        self.route.as_ref()
    }

    /// Records `rule:observability/an-inbound-traceparent-is-continued`'s trace, as the door decided it from the arrived
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
    /// Gives this carrier a body that is already whole in memory.
    ///
    /// The one supplier that is not a socket: `nvs run --request` reads a
    /// frozen request off disk, so the octets exist before the program does
    /// and there is no stream left to pull them from. Empty octets are a body
    /// that is present and empty, which [`Self::set_body`] describes as the
    /// other state from carrying none.
    ///
    /// **What it spends**, on `rule:programs/memory-priority`'s ledger: the
    /// whole body resident for the request rather than one chunk of it,
    /// bounded by the file it was read out of — an operator's own file and
    /// never a peer's stream, which is why
    /// `rule:http-server/request-body-and-upload-total-are-two-caps`'s caps
    /// have nothing to hold back here.
    pub fn set_buffered_body(&mut self, body: Vec<u8>) {
        self.set_body(Box::new(Buffered { body, at: 0 }));
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
    /// that already read it — decided by what `member` needs of the body and
    /// by what the last reader left behind.
    ///
    /// `rule:http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it`,
    /// enforced. A streaming reader hands the octets over as they arrive and
    /// keeps none, so nothing reads them after it: answering the next reader
    /// empty — which is what an exhausted stream says on its own — would
    /// report "the peer sent nothing" for a request whose bytes the program had
    /// already been handed, and that is the silent-wrong-answer this whole
    /// carrier is written against. A buffering reader leaves what it read in
    /// [`Self::held`] or in [`Self::parts`], and a later reader that hold
    /// satisfies is answered out of it.
    ///
    /// **The parameter is a [`BodyNeed`] rather than the reader's kind**, whose
    /// argument that type carries, and the three needs against the two holds
    /// are the whole of the rule this carrier can state.
    ///
    /// The claim stays the **first** reader's, whoever is let through
    /// afterwards: it is the name a refusal has to carry, and the reading a
    /// program wrote first is the one that decided what the others can have.
    ///
    /// This crate raises nothing and decides nothing about which member needs
    /// what; the needs arrive with the call, the members are `nvs_stdlib`'s and
    /// so is the rule.
    ///
    /// # Errors
    ///
    /// The name of the member that claimed the body first, where one has and
    /// what it left behind is not what `member` needs.
    pub fn claim_body(&mut self, member: &'static str, need: BodyNeed) -> Result<(), &'static str> {
        let answerable = match need {
            BodyNeed::Octets => self.held.is_some(),
            BodyNeed::OctetsOrParse => self.held.is_some() || self.parts.is_some(),
            BodyNeed::Wire => false,
        };
        match self.claimed_by {
            Some(first) if !answerable => Err(first),
            Some(_) => Ok(()),
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
    /// Whether this request's body has been parsed already, which is the
    /// question a member asks before building a parse of its own.
    ///
    /// Separate from [`Self::parts_mut`] because it is asked where no borrow is
    /// wanted and answered for a request that carried no body: "nothing has
    /// parsed this" and "there is nothing here to parse" are different facts,
    /// and the accessor that hands the parse out collapses them into one
    /// `None`.
    #[must_use]
    pub fn has_parts(&self) -> bool {
        self.parts.is_some()
    }
    /// Gives this carrier the parse of its multipart body, for
    /// [`Self::parts_mut`] to hand back on every later call.
    ///
    /// Called at most once per request. `Core\Request::files()` and
    /// `Core\Request::post()` both name a parse, and the second of them finds
    /// this one filled and walks it instead of building another — which is what
    /// makes the fields a walk buffered readable by the form reader that
    /// follows it, and [`Self::claim_body`] is what keeps a third member off
    /// the body entirely.
    pub fn hold_parts(&mut self, parts: Box<dyn std::any::Any>) {
        self.parts = Some(parts);
    }
    /// The parse and the octets it reads, borrowed **together** — `None` unless
    /// this request has both.
    ///
    /// One accessor rather than two, because the two are borrowed at the same
    /// moment and neither outlives the call: `nvs_stdlib::multipart` takes the
    /// body per call rather than holding one — its own module doc says why —
    /// so every step of a parse needs `&mut` on both at once, and two methods
    /// could not hand that out. They are two fields of one struct, so this is a
    /// borrow the compiler can see through where a pair of calls would not be.
    ///
    /// **Which octets is decided here**, in the one place that can see both
    /// halves: [`Self::held`]'s where a buffering reader filled it, and the
    /// wire otherwise. A parse over a request whose body was already pulled
    /// whole has to read what that reader held — the wire is drained by then
    /// and would answer a form with no fields in it — and asking the member to
    /// know that would put
    /// `rule:http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it`
    /// in two places.
    ///
    /// `None` for a request that carried no body at all, which is a walk over
    /// no parts rather than an error: RFC 9110 § 8.6's "there is no body" is
    /// the same fact [`Self::body`] answers `None` for, and a multipart parse
    /// of it would be a parse of nothing.
    pub fn parts_mut(&mut self) -> Option<(&mut (dyn std::any::Any + 'static), BodySource<'_>)> {
        let parse = self.parts.as_deref_mut()?;
        let source = match self.held.as_deref() {
            Some(octets) => BodySource::Hold { octets, at: 0 },
            None => BodySource::Wire(self.body.as_deref_mut()?),
        };
        Some((parse, source))
    }
    /// Gives this carrier the body a buffering reader pulled off the wire, for
    /// that member's later calls and for every later reader to answer out of —
    /// [`Self::held`] owns why it is the bytes and not a parse of them.
    ///
    /// Called at most once per request: a reader pulls only where
    /// [`Self::held_body`] answered `None`, so the call that fills this is the
    /// one that found it empty.
    pub fn hold_body(&mut self, body: Box<[u8]>) {
        self.held = Some(body);
    }
    /// The body [`Self::hold_body`] was given, or `None` where nothing has
    /// buffered one yet.
    #[must_use]
    pub fn held_body(&self) -> Option<&[u8]> {
        self.held.as_deref()
    }
    /// Gives this carrier the document `Core\Request::json` decoded, for that
    /// member's later calls to answer out of — [`Self::decoded`] owns why it is
    /// a value here where every other hold is bytes.
    ///
    /// Called at most once per request: the member fills this only where
    /// [`Self::decoded_body`] answered `None`, so a call at a depth this one
    /// was not decoded at decodes again and hands the answer straight back.
    pub fn hold_decoded_body(&mut self, decoded: HeldValue) {
        self.decoded = Some(decoded);
    }
    /// The document [`Self::hold_decoded_body`] was given, or `None` where
    /// nothing has decoded one yet.
    ///
    /// The depth rides along because the answer is only this one for a call
    /// asking the same question: [`HeldValue::depth`] is what the caller
    /// compares its own `maxDepth` against.
    #[must_use]
    pub fn decoded_body(&self) -> Option<&HeldValue> {
        self.decoded.as_ref()
    }
    /// Takes the hold back out, for the one caller that has to release it
    /// before this carrier's fields go down of their own accord.
    ///
    /// That caller is [`Ctx`]'s `Drop`, and its body is the home of why: a
    /// carrier is a *field* of the context, so leaving this to [`HeldValue`]'s
    /// own `Drop` would give the reference back after the live-list sweep had
    /// already reached what it points at.
    pub fn take_decoded_body(&mut self) -> Option<HeldValue> {
        self.decoded.take()
    }

    /// Offers `rule:concurrency/a-connection-is-a-root-isolate`'s upgrade to this request: the slot
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
    /// Offers `rule:concurrency/two-doors-one-isolate`'s SSE cell to this request: the slot
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
    /// Offers this request a **streaming response body**: the cell
    /// `Core\Response::stream` opens, whose head the connection answers with
    /// while the request is still running.
    ///
    /// Called for every request a connection is framing a response for, like
    /// [`Self::offer_sse`] and unlike [`Self::offer_upgrade`] — a body written
    /// over time needs nothing of the connection but the response the request
    /// already has. The slot carries the connection's send bound, which is the
    /// only place a request-scoped stream reads one.
    pub fn offer_response_stream(&mut self, slot: crate::stream::BodySlot) {
        self.stream = Some(slot);
    }
    /// The cell [`Self::offer_response_stream`] left, and `None` for a carrier
    /// no connection built.
    ///
    /// A shared borrow, for [`Self::upgrade_slot`]'s reason: the cell is what
    /// serialises the two halves, so opening a body through it takes `&self`.
    #[must_use]
    pub fn response_stream_slot(&self) -> Option<&crate::stream::BodySlot> {
        self.stream.as_ref()
    }
}

/// A request as a description rather than as a carrier: everything a test may
/// say about one, in `rule:testing/in-process-request`'s
/// `Core\Test::request` bag exactly, and the one place a described request
/// becomes an [`Inbound`].
///
/// **It lives beside the carrier it builds** because four crates need to build
/// one and none of them is under the other three: `nvs-test` renders a case's
/// sections, `nvs-cli` reads a frozen request off disk, `nvs-stdlib` answers
/// the member a test calls, and `nvs-server` accepts one off a socket. All four
/// already depend on this crate, so a builder written in any of them is a
/// builder the others copy — and two builders are two answers to what a request
/// nobody sent carries, which is exactly the question a test is asking.
///
/// **It interprets nothing, with three named exceptions.** The query is the raw
/// string past the `?`, a header is the field line as it was given, and a
/// cookie pair is written into the one `cookie` field a peer would have sent —
/// none of it checked, because a test's own reason for building a request by
/// hand is often that the request is a bad one, and a builder that repaired a
/// cookie name would hide the case the test exists for
/// (`rule:errors/cookie-name-bytes` is the reader's rule, not this writer's).
/// The exceptions are the three body spellings that encode: [`SpecBody::Form`],
/// [`SpecBody::Json`] and [`SpecBody::Files`] each produce their octets and
/// their own `Content-Type`, which is the whole point of writing one of them
/// rather than [`SpecBody::Raw`].
///
/// **A field the spec derives is never written twice.** `host`, `cookie`,
/// `content-type` and `content-length` are each written only where
/// [`Self::push_header`] did not already carry that name: a spec that spelled
/// one meant it, and a field standing twice is a field that can disagree with
/// itself.
///
/// **What it spends:** the request's own bytes, once, in a process that is
/// building a request — a test runner or `nvs run --request`, never a served
/// request's path. [`Self::build`] holds the body twice for the length of the
/// encode where the spelling is one that encodes, since the parts and the
/// payload they are written into are both resident until the spec is dropped at
/// the end of that call.
#[derive(Debug, Clone)]
pub struct InboundSpec {
    /// The verb, as the two positional arguments of `Core\Test::request` name
    /// it, and un-uppercased for [`Inbound::method`]'s reason.
    method: String,
    /// The path, already without any query string.
    path: String,
    /// Everything past the `?`, undecoded, and `""` for a request carrying
    /// none — the two states [`Inbound::query`] does not distinguish either.
    query: String,
    /// One entry per field line, in the order given, name first and the value
    /// as bytes: [`Inbound::headers`]'s shape, because that is what these
    /// become.
    headers: Vec<(String, Vec<u8>)>,
    /// The cookie pairs, joined into one `cookie` field line at [`Self::build`].
    ///
    /// A bag key of their own rather than a header a test writes, because a
    /// cookie is what a test is describing and the field is only how it
    /// travels — and because a joiner that lives here is one every caller
    /// shares.
    cookies: Vec<(String, String)>,
    /// The body in whichever of its four spellings was written, and `None` for
    /// a request carrying none — which is the other state from an empty one,
    /// as [`Inbound::set_body`] describes.
    ///
    /// One field for the four keys, so that two of them cannot both be held:
    /// [`Self::set_body`] refuses the second naming both, and the refusal is
    /// therefore impossible to reach from anything reading this.
    body: Option<SpecBody>,
    /// What the request's peer resolves to, as
    /// `rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing`'s
    /// walk would have decided it — `None` for a peer with no address at all.
    client: Option<IpAddr>,
    /// The scheme the request effectively arrived over, `Http` until a spec
    /// says otherwise for [`Inbound::new`]'s fail-closed reason.
    scheme: Scheme,
    /// The authority the request names, written as the `host` field line every
    /// HTTP/1.1 request carries it in, and `None` for a spec that named none.
    host: Option<String>,
    /// The prefix a door is to have stripped off [`Self::path`] before the
    /// program saw it, and `""` for a request that reached no mount — the two
    /// states [`Inbound::mount_prefix`] does not distinguish either, which is
    /// why this is a `String` where the peer above it is an [`Option`].
    mount_prefix: String,
    /// The mount's glob captures, in order, and empty for a prefix that has
    /// none. One field beside the one above rather than a pair, for
    /// [`Inbound::set_mount`]'s reason: they are two halves of one row and
    /// [`Self::build`] hands them over together.
    mount_captures: Vec<String>,
}

/// One of the four spellings a spec's body is written in, three of which encode
/// themselves and say what they encoded to.
///
/// They are four keys of one bag and one field of one spec, so the enum is what
/// makes `rule:errors/ambiguous-input-refused`'s refusal
/// structural: a body is written under exactly one of these names, and two
/// names in one bag are refused whole rather than merged.
#[derive(Debug, Clone)]
pub enum SpecBody {
    /// The octets verbatim, framed by nothing and typed by nothing.
    ///
    /// The spelling a test reaches for when the request it describes is a
    /// malformed one — a truncated multipart body, a form with an `=` where no
    /// pair may have one — since nothing here writes a `Content-Type` for it
    /// and nothing checks what it holds.
    Raw(Vec<u8>),
    /// Pairs, encoded as an `application/x-www-form-urlencoded` payload under
    /// that `Content-Type`.
    ///
    /// The escape set is `Core\Uri::encodeFormValue`'s and is written twice
    /// rather than shared, because that member is `nvs_stdlib`'s and this crate
    /// is below it: ASCII alphanumerics and `-_.` stand, a space is `+`, and
    /// every other octet is `%` and two upper-case hex digits.
    Form(Vec<(String, String)>),
    /// A JSON document, already encoded, under `application/json`.
    ///
    /// The octets rather than a [`Value`], because encoding one is
    /// `Core\Json`'s reading of a program's value and lives a crate above this
    /// one. What this spelling buys over [`Self::Raw`] is the header, which is
    /// what a test would otherwise have to remember to write beside every
    /// document it posts.
    Json(Vec<u8>),
    /// Parts, encoded as a `multipart/form-data` body under that `Content-Type`
    /// and the boundary the encode chose.
    ///
    /// **The encoder is this module's and `nvs_stdlib::multipart` stays a
    /// parser** — one direction each, and not twins under
    /// `rule:core-api/shape-rules` R17, since neither is
    /// reachable through the other.
    ///
    /// A part with no file name is a form field rather than a file
    /// (`rule:http-server/a-part-is-a-file-iff-it-carries-a-filename`), which
    /// is how one spelling describes the mixed body a `form` and a `files` key
    /// together would otherwise have to be merged into.
    Files(Vec<SpecPart>),
}

/// One part of a [`SpecBody::Files`] body: what it is called, what it holds,
/// and whether it is a file at all.
#[derive(Debug, Clone)]
pub struct SpecPart {
    /// The form field name, written into the part's `Content-Disposition`.
    pub name: String,
    /// The file name, and `None` for a part that is a form field rather than a
    /// file — which is the whole of the distinction
    /// `rule:http-server/a-part-is-a-file-iff-it-carries-a-filename` draws.
    pub filename: Option<String>,
    /// The part's own `Content-Type`, written only where one is given: a part
    /// that declares none is one the parser reads as `text/plain`, and writing
    /// that here would put a claim in the body the spec was not asked for.
    pub content_type: Option<String>,
    /// The part's octets, verbatim.
    pub bytes: Vec<u8>,
}

impl SpecBody {
    /// The bag key this body was written under, which is what a refusal names.
    #[must_use]
    pub fn spelling(&self) -> &'static str {
        match self {
            Self::Raw(_) => "body",
            Self::Form(_) => "form",
            Self::Json(_) => "json",
            Self::Files(_) => "files",
        }
    }

    /// The octets this body is, and the `Content-Type` the spelling declares —
    /// `None` for the raw one, which declares nothing.
    fn encode(self) -> (Vec<u8>, Option<String>) {
        match self {
            Self::Raw(octets) => (octets, None),
            Self::Form(pairs) => (
                form_encoded(&pairs),
                Some("application/x-www-form-urlencoded".to_owned()),
            ),
            Self::Json(document) => (document, Some("application/json".to_owned())),
            Self::Files(parts) => {
                let boundary = boundary_for(&parts);
                let body = multipart(&parts, &boundary);
                (
                    body,
                    Some(format!("multipart/form-data; boundary={boundary}")),
                )
            }
        }
    }
}

impl InboundSpec {
    /// The two facts `Core\Test::request` takes before its bag: the verb and
    /// the path, with no query, no header and no body yet.
    #[must_use]
    pub fn new(method: &str, path: &str) -> Self {
        Self {
            method: method.to_owned(),
            path: path.to_owned(),
            query: String::new(),
            headers: Vec::new(),
            cookies: Vec::new(),
            body: None,
            // [`Inbound::new`]'s fail-closed pair, said again here for the same
            // reason: a spec that names no peer describes a request from
            // nobody in particular over plaintext, and `Https` is a claim only
            // [`Self::set_peer`] can make.
            client: None,
            scheme: Scheme::Http,
            host: None,
            mount_prefix: String::new(),
            mount_captures: Vec::new(),
        }
    }

    /// The query string, as it would stand past the `?`.
    pub fn set_query(&mut self, query: &str) {
        self.query = query.to_owned();
    }

    /// Adds one header field line, which is written in the order the calls
    /// came, as [`Inbound::push_header`] keeps them.
    pub fn push_header(&mut self, name: &str, value: &[u8]) {
        self.headers.push((name.to_owned(), value.to_vec()));
    }

    /// Adds one cookie pair to the `cookie` field [`Self::build`] joins.
    pub fn push_cookie(&mut self, name: &str, value: &str) {
        self.cookies.push((name.to_owned(), value.to_owned()));
    }

    /// Gives the spec its body, in one of the four spellings.
    ///
    /// # Errors
    ///
    /// A second spelling is refused naming both, never merged with the first
    /// (`rule:errors/ambiguous-input-refused`): `body`,
    /// `form`, `json` and `files` describe one body four ways, so a bag holding
    /// two of them has said two things and a builder picking either would be
    /// answering a question it was not asked.
    ///
    /// A [`SpecBody::Files`] part whose name or file name holds a `"`, a `\r`
    /// or a `\n` is refused for the same reason one step down: those bytes are
    /// what frames a part header, so a part carrying one would describe parts
    /// the spec was never given. A test that means to build a malformed body
    /// writes [`SpecBody::Raw`], which frames nothing and checks nothing.
    pub fn set_body(&mut self, body: SpecBody) -> Result<(), String> {
        if let Some(held) = &self.body {
            return Err(format!(
                "`{}` and `{}` are two spellings of one body",
                held.spelling(),
                body.spelling()
            ));
        }
        if let SpecBody::Files(parts) = &body {
            for part in parts {
                writable("name", &part.name)?;
                if let Some(filename) = &part.filename {
                    writable("file name", filename)?;
                }
            }
        }
        self.body = Some(body);
        Ok(())
    }

    /// Records who the request is to have come from, on
    /// [`Inbound::set_peer`]'s terms: one walk answers both, so a spec states
    /// both or neither.
    pub fn set_peer(&mut self, client: Option<IpAddr>, scheme: Scheme) {
        self.client = client;
        self.scheme = scheme;
    }

    /// Records the authority the request names, which travels as its `host`
    /// field line.
    pub fn set_host(&mut self, host: &str) {
        self.host = Some(host.to_owned());
    }

    /// Records the mount the request is to have come through, on
    /// [`Inbound::set_mount`]'s terms: the prefix a door stripped and the
    /// captures that selected it are two fields of one row, so a spec states
    /// both or neither.
    ///
    /// The path is the mount-stripped one either way — that is what a
    /// `#[Route]` is declared against — so describing a mount adds the prefix
    /// a program reads back through `Core\Request::mount()` and changes
    /// nothing about what the table is matched on.
    pub fn set_mount(&mut self, prefix: &str, captures: &[String]) {
        self.mount_prefix = prefix.to_owned();
        self.mount_captures = captures.to_vec();
    }

    /// Builds the carrier this spec describes.
    ///
    /// The order is the one a peer would have written: the spec's own field
    /// lines first, then the fields it derived — `host`, `cookie`,
    /// `content-type`, `content-length` — each only where no line of that name
    /// was written by hand.
    #[must_use]
    pub fn build(self) -> Inbound {
        let Self {
            method,
            path,
            query,
            headers,
            cookies,
            body,
            client,
            scheme,
            host,
            mount_prefix,
            mount_captures,
        } = self;
        let mut inbound = Inbound::new(&method, &path, &query);
        for (name, value) in &headers {
            inbound.push_header(name, value);
        }
        if let Some(host) = &host
            && !carries(&headers, "host")
        {
            inbound.push_header("host", host.as_bytes());
        }
        if !cookies.is_empty() && !carries(&headers, "cookie") {
            let pairs: Vec<String> = cookies
                .iter()
                .map(|(name, value)| format!("{name}={value}"))
                .collect();
            inbound.push_header("cookie", pairs.join("; ").as_bytes());
        }
        if let Some(body) = body {
            let (octets, content_type) = body.encode();
            if let Some(content_type) = content_type
                && !carries(&headers, "content-type")
            {
                inbound.push_header("content-type", content_type.as_bytes());
            }
            if !carries(&headers, "content-length") {
                inbound.push_header("content-length", octets.len().to_string().as_bytes());
            }
            inbound.set_buffered_body(octets);
        }
        inbound.set_peer(client, scheme);
        inbound.set_mount(&mount_prefix, &mount_captures);
        inbound
    }
}

/// Whether a field of this name is already written, compared the way RFC 9110
/// § 5.1 compares one.
fn carries(headers: &[(String, Vec<u8>)], name: &str) -> bool {
    headers
        .iter()
        .any(|(field, _)| field.eq_ignore_ascii_case(name))
}

/// Refuses a part header's `text` where it holds a byte that frames one.
fn writable(label: &str, text: &str) -> Result<(), String> {
    if text.contains(['"', '\r', '\n']) {
        return Err(format!(
            "a part's {label} is written into a part header, so it carries no quote and no line break: `{text}`"
        ));
    }
    Ok(())
}

/// The upper-case hex digits a percent-escape is written with — RFC 3986
/// § 2.1's recommendation, and the case `Core\Uri::encodeFormValue` writes.
const HEX: [u8; 16] = *b"0123456789ABCDEF";

/// `pairs` as an `application/x-www-form-urlencoded` payload.
fn form_encoded(pairs: &[(String, String)]) -> Vec<u8> {
    let mut out = Vec::new();
    for (name, value) in pairs {
        if !out.is_empty() {
            out.push(b'&');
        }
        percent(&mut out, name.as_bytes());
        out.push(b'=');
        percent(&mut out, value.as_bytes());
    }
    out
}

/// Appends `text` to `out`, percent-encoded as one form value.
fn percent(out: &mut Vec<u8>, text: &[u8]) {
    for &byte in text {
        match byte {
            b' ' => out.push(b'+'),
            _ if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.') => {
                out.push(byte);
            }
            _ => out.extend_from_slice(&[
                b'%',
                HEX[usize::from(byte >> 4)],
                HEX[usize::from(byte & 0x0f)],
            ]),
        }
    }
}

/// The boundary a `multipart/form-data` body of these parts is framed with.
///
/// A fixed word rather than a random one, so that a test asserting on the
/// octets it built asserts on the same octets every run — and lengthened until
/// no part holds it, which is what keeps the framing honest for the one input
/// that could break it. Only the parts' own bytes are searched: a delimiter is
/// a line, and a name that could stand on one is already refused
/// ([`InboundSpec::set_body`]).
fn boundary_for(parts: &[SpecPart]) -> String {
    let mut boundary = String::from("novisFormBoundary");
    while parts
        .iter()
        .any(|part| holds(&part.bytes, boundary.as_bytes()))
    {
        boundary.push('-');
    }
    boundary
}

/// Whether `haystack` holds `needle` anywhere in it.
fn holds(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// `parts` as a `multipart/form-data` body framed with `boundary`, in RFC 7578's
/// shape: a delimiter line per part, that part's headers, a blank line, its
/// octets, and a closing delimiter.
fn multipart(parts: &[SpecPart], boundary: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for part in parts {
        out.extend_from_slice(b"--");
        out.extend_from_slice(boundary.as_bytes());
        out.extend_from_slice(b"\r\nContent-Disposition: form-data; name=\"");
        out.extend_from_slice(part.name.as_bytes());
        out.push(b'"');
        if let Some(filename) = &part.filename {
            out.extend_from_slice(b"; filename=\"");
            out.extend_from_slice(filename.as_bytes());
            out.push(b'"');
        }
        out.extend_from_slice(b"\r\n");
        if let Some(content_type) = &part.content_type {
            out.extend_from_slice(b"Content-Type: ");
            out.extend_from_slice(content_type.as_bytes());
            out.extend_from_slice(b"\r\n");
        }
        out.extend_from_slice(b"\r\n");
        out.extend_from_slice(&part.bytes);
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(b"--");
    out.extend_from_slice(boundary.as_bytes());
    out.extend_from_slice(b"--\r\n");
    out
}

/// `rule:concurrency/a-connection-is-a-root-isolate`'s
/// connection isolate, prepared and not yet started: the program it runs, and
/// the argument that has already crossed to it.
///
/// **It is exactly what `nvs_host::Isolate::new` takes**, and that is the whole
/// of the type — a connection is not a kind of isolate of its own but § 1's root
/// one with the two things every isolate needs decided a task earlier. Why they
/// are decided there is `nvs_stdlib::socket`'s module doc: the capability, the
/// argument and the code are all the *request's* to refuse, and each of them
/// answers `null` or throws on a context that never ran the program.
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
    /// callable for a static method one. Both of `rule:security/isolate-shares-nothing`'s two entry forms
    /// arrive here as one thing, which is what makes them one isolate.
    program: crate::script::Program,
    /// The argument, on this side of the boundary already — the copy the request
    /// made so that `rule:classes/graph-copy`'s refusal could still be a throw the program
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
/// request for would be the ordering `rule:concurrency/a-connection-is-a-root-isolate` forbids, spelled as something
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
/// `rule:concurrency/two-doors-one-isolate`'s SSE
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
/// `rule:http-server/an-upload-is-received-only-through-files`'s
/// stream, at the seam where it crosses into a crate that has never heard of
/// HTTP.
///
/// # A reader rather than the bytes, and § 5 is what decides it
///
/// The alternative was for whoever accepted the request to read the body to its
/// end and hand [`Inbound`] a `Box<[u8]>` beside its strings, which is
/// what PHP does and what every part of this design would then be built on top
/// of. `rule:http-server/request-body-and-upload-total-are-two-caps` rules it out arithmetically rather than as a preference.
/// That section states **two** caps because they measure two different things:
/// `request_body` (`"8M"`, ceiling `"64M"`) bounds *bytes parsed into memory*,
/// and `upload_total` (`"256M"`, ceiling `"2G"`) bounds *the total of a streamed
/// multipart body*. A door that buffered would collapse them into one, and into
/// the larger — the bytes are already resident by the time any member could
/// apply the tighter number, so `request_body` would be a check over something
/// already paid for, which is not a bound. What is left is a per-request
/// resident cost of `upload_total`, multiplied by `max_in_flight` by
/// `rule:http-server/admission-is-arithmetic-not-a-number`
/// 's arithmetic: a 2G ceiling against a memory budget divides the effective
/// admission ceiling to approximately nothing, so the honest configuration and
/// the working one stop being the same file.
///
/// § 6 closes the remaining option before it is asked: there is no temp file, so
/// "neither resident nor streamed" is not a place a body can be put. Buffered
/// or streamed is the whole choice, and only one of them is bounded — which is
/// exactly what `docs/plan/m7.md`'s load-bearing acceptance case asserts, a
/// multipart body far larger than any in-memory bound received in full at a
/// bounded resident **high-water mark**.
///
/// # A chunk is borrowed, not owned
///
/// [`Self::next_chunk`] answers a slice of the supplier's own buffer, valid
/// until the next pull — `rule:http-server/a-part-is-consumed-in-one-of-three-ways`'s "valid only while this part is the
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
    /// (`rule:concurrency/one-future-per-connection`
    /// ) — so a pull parks that isolate and the connection's next poll is
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
    /// peer stopped short of its declared length, or `rule:http-server/request-body-and-upload-total-are-two-caps`'s
    /// `upload_total` was crossed mid-stream by a body that declared no length.
    /// The message is for the member reading it to turn into its own throw —
    /// this seam classifies nothing, for [`Inbound`]'s own reason. **An `Err`
    /// ends the body**: nothing may be pulled after one, and a supplier that
    /// answered one has already given up on the connection.
    fn next_chunk(&mut self) -> Result<Option<&[u8]>, Box<str>>;
}

/// What a reader needs of a request's body, which is what decides whether it
/// may have it — [`Inbound::claim_body`]'s parameter.
///
/// **A need rather than the reader's kind**, because the kind is not fixed per
/// member and so cannot be a table of names: `Core\Request::post()` consumes
/// the wire where it is the first reader and consumes nothing where a hold is
/// already filled, so it is both kinds on different requests. What *is* fixed
/// is what each member has to be given, and the carrier already holds the
/// facts that answer it — the octets it holds, the parse it holds, and the
/// name that claimed. So the parameter is the need and the matrix in
/// [`Inbound::claim_body`] is the whole rule.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BodyNeed {
    /// The octets themselves: `body()`, `json()` and `jsonAs()`. They are there
    /// for the taking while no reader has claimed the body, and afterwards only
    /// where a buffering reader left them held — a streaming reader keeps none,
    /// and a multipart `post()` keeps the fields it decoded rather than the
    /// bytes they came out of.
    Octets,
    /// The octets **or** a parse of them: `post()`, which reads either a form
    /// out of the body it pulls itself or the fields another reader buffered.
    OctetsOrParse,
    /// The wire: `bodyStream()` and `files()`, which pull the octets as they
    /// arrive and keep none of them. Nothing may have read the body before one
    /// of these, and nothing reads it after.
    Wire,
}

/// The octets a parse is driven with: whichever of the request's own two
/// suppliers is the one that still has them.
///
/// [`Inbound::parts_mut`] hands one out, and which variant it is is the
/// carrier's answer rather than a member's choice: a multipart `post()`
/// following `body()` reads the hold that reader filled, while the same call on
/// a request nothing has read yet reads the wire. A member that picked for
/// itself would be the second copy of
/// `rule:http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it`,
/// and two copies of one rule is how they come to disagree.
pub enum BodySource<'a> {
    /// The request's own supplier, pulled a chunk at a time as the peer sends
    /// it — the only source for a body nothing has held.
    Wire(&'a mut (dyn RequestBody + 'static)),
    /// [`Inbound::held`]'s octets, walked in place.
    ///
    /// In [`HOLD_PIECE`] pieces rather than one chunk: a reader holds a whole
    /// chunk in its own buffer — `nvs_stdlib::multipart` does — so handing one
    /// the body entire would hold it twice, which is exactly the second copy
    /// `[limits] request_body` was set to bound the first of.
    Hold {
        /// The held body, borrowed from the carrier for this walk.
        octets: &'a [u8],
        /// How far the walk has read.
        at: usize,
    },
}

impl std::fmt::Debug for BodySource<'_> {
    /// Written out rather than derived, for [`Inbound`]'s own reason: a
    /// [`RequestBody`] is a socket mid-read and there is nothing to print of it
    /// but that it is the source. A hold prints how far the walk has come and
    /// how much there is, never the octets, which are a peer's bytes.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Wire(_) => f.write_str("Wire"),
            Self::Hold { octets, at } => f
                .debug_struct("Hold")
                .field("octets", &octets.len())
                .field("at", at)
                .finish(),
        }
    }
}

/// The most of a held body [`BodySource::Hold`] hands over at once.
///
/// Big enough that a walk over a multipart body of any size is a handful of
/// pulls per part rather than one per line, small enough that a reader
/// buffering a whole piece has not re-held the body. Chunk boundaries mean
/// nothing to a reader ([`RequestBody::next_chunk`]), so this number is free to
/// be an implementation's choice.
pub const HOLD_PIECE: usize = 64 * 1024;

impl RequestBody for BodySource<'_> {
    fn next_chunk(&mut self) -> Result<Option<&[u8]>, Box<str>> {
        match self {
            Self::Wire(body) => body.next_chunk(),
            Self::Hold { octets, at } => {
                let end = octets.len().min(at.saturating_add(HOLD_PIECE));
                if *at == end {
                    return Ok(None);
                }
                let piece = &octets[*at..end];
                *at = end;
                Ok(Some(piece))
            }
        }
    }
}

/// A [`RequestBody`] whose octets are already held, which is what
/// [`Inbound::set_buffered_body`] hands over.
///
/// One chunk and then the end, because there is no wire to have split them:
/// chunk boundaries mean nothing to a reader, so the whole body is as valid a
/// division as any other and it is the one that copies nothing.
struct Buffered {
    body: Vec<u8>,
    at: usize,
}

impl RequestBody for Buffered {
    fn next_chunk(&mut self) -> Result<Option<&[u8]>, Box<str>> {
        if self.at == self.body.len() {
            return Ok(None);
        }
        self.at = self.body.len();
        Ok(Some(&self.body))
    }
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

    /// `rule:concurrency/a-connection-is-a-root-isolate`'s slot, as the two halves it is: a carrier nobody offered
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

    /// `rule:concurrency/two-doors-one-isolate`'s second cell, and the one thing that separates it from the
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

    /// The built carrier's head as text, which every [`InboundSpec`] assertion
    /// below reads rather than the spec it came from: what a spec is worth is
    /// what the request ends up carrying.
    fn head(inbound: &Inbound) -> Vec<(String, String)> {
        inbound
            .headers()
            .map(|(name, value)| {
                (
                    name.to_owned(),
                    String::from_utf8(value.to_vec()).expect("a built field is text"),
                )
            })
            .collect()
    }

    /// Every octet of a built carrier's body, joined, and `None` where the
    /// request carries none.
    fn body_of(inbound: &mut Inbound) -> Option<String> {
        let body = inbound.body()?;
        let mut octets = Vec::new();
        while let Some(chunk) = body.next_chunk().expect("a held body cannot fail") {
            octets.extend_from_slice(chunk);
        }
        Some(String::from_utf8(octets).expect("these bodies are text"))
    }

    /// `rule:errors/ambiguous-input-refused` at the
    /// door of the builder: `body`, `form`, `json` and `files` are four
    /// spellings of one body, so a bag holding two has said two things and the
    /// refusal names both rather than picking one.
    ///
    /// The second half is what makes the refusal a refusal rather than a
    /// report: the spec is unchanged, so the request that gets built is the
    /// one the first spelling described and never a merge of the two.
    #[test]
    fn two_body_spellings_in_one_spec_are_refused_naming_both() {
        let mut spec = InboundSpec::new("POST", "/users");
        spec.set_body(SpecBody::Form(vec![("name".to_owned(), "ada".to_owned())]))
            .expect("the first spelling is the body");

        let refused = spec
            .set_body(SpecBody::Json(b"{\"name\":\"ada\"}".to_vec()))
            .expect_err("a second spelling is refused");
        assert_eq!(refused, "`form` and `json` are two spellings of one body");

        let mut inbound = spec.build();
        assert_eq!(body_of(&mut inbound).as_deref(), Some("name=ada"));
    }

    /// A form body encodes its own pairs and declares its own type, which is
    /// the whole of what the `form` key buys over `body`.
    ///
    /// The escape set is `Core\Uri::encodeFormValue`'s: a space is `+` and the
    /// `&` that would otherwise open a pair of its own is `%26`, so a value
    /// carrying the byte that structures the payload cannot restructure it.
    #[test]
    fn a_form_field_encodes_urlencoded_and_sets_its_content_type() {
        let mut spec = InboundSpec::new("POST", "/users");
        spec.set_body(SpecBody::Form(vec![
            ("name".to_owned(), "ada lovelace".to_owned()),
            ("q".to_owned(), "a&b".to_owned()),
        ]))
        .expect("one spelling");

        let mut inbound = spec.build();
        assert_eq!(
            body_of(&mut inbound).as_deref(),
            Some("name=ada+lovelace&q=a%26b")
        );
        assert_eq!(
            head(&inbound),
            vec![
                (
                    "content-type".to_owned(),
                    "application/x-www-form-urlencoded".to_owned()
                ),
                ("content-length".to_owned(), "25".to_owned()),
            ]
        );
    }

    /// A `json` body is carried verbatim under `application/json`, which is the
    /// whole of what the `json` spelling buys over `body`. Nothing here reads
    /// the document, so a case that means to post a malformed one gets exactly
    /// the octets it wrote, under the type a client would have declared.
    #[test]
    fn a_json_field_encodes_the_value_and_sets_its_content_type() {
        let mut spec = InboundSpec::new("POST", "/users");
        spec.set_body(SpecBody::Json(br#"{"name":"ada"}"#.to_vec()))
            .expect("one spelling");

        let mut inbound = spec.build();
        assert_eq!(body_of(&mut inbound).as_deref(), Some(r#"{"name":"ada"}"#));
        assert_eq!(
            head(&inbound),
            vec![
                ("content-type".to_owned(), "application/json".to_owned()),
                ("content-length".to_owned(), "14".to_owned()),
            ]
        );
    }

    /// A `files` body is framed part by part, a part with no file name is a
    /// form field rather than a file
    /// (`rule:http-server/a-part-is-a-file-iff-it-carries-a-filename`), and the
    /// boundary is lengthened until no part holds it.
    ///
    /// The last of those is the one worth a test: a fixed boundary is what
    /// makes a built body assertable at all, and a part that happened to carry
    /// it would otherwise frame parts nobody wrote.
    #[test]
    fn a_files_field_builds_a_multipart_body_with_a_boundary() {
        let mut spec = InboundSpec::new("POST", "/avatars");
        spec.set_body(SpecBody::Files(vec![
            SpecPart {
                name: "note".to_owned(),
                filename: None,
                content_type: None,
                bytes: b"hi".to_vec(),
            },
            SpecPart {
                name: "avatar".to_owned(),
                filename: Some("a.png".to_owned()),
                content_type: Some("image/png".to_owned()),
                bytes: b"novisFormBoundary".to_vec(),
            },
        ]))
        .expect("one spelling");

        let mut inbound = spec.build();
        assert_eq!(
            body_of(&mut inbound).as_deref(),
            Some(concat!(
                "--novisFormBoundary-\r\n",
                "Content-Disposition: form-data; name=\"note\"\r\n",
                "\r\n",
                "hi\r\n",
                "--novisFormBoundary-\r\n",
                "Content-Disposition: form-data; name=\"avatar\"; filename=\"a.png\"\r\n",
                "Content-Type: image/png\r\n",
                "\r\n",
                "novisFormBoundary\r\n",
                "--novisFormBoundary---\r\n",
            ))
        );
        assert_eq!(
            head(&inbound)[0],
            (
                "content-type".to_owned(),
                "multipart/form-data; boundary=novisFormBoundary-".to_owned()
            )
        );
    }

    /// A part name that could frame a part header is refused, and the spec
    /// keeps no body — the one place the builder checks what it was given,
    /// because these are bytes it writes itself rather than bytes it carries.
    #[test]
    fn a_part_name_that_could_frame_a_header_is_refused() {
        let mut spec = InboundSpec::new("POST", "/avatars");
        let refused = spec
            .set_body(SpecBody::Files(vec![SpecPart {
                name: "a\"; filename=\"passwd".to_owned(),
                filename: None,
                content_type: None,
                bytes: b"x".to_vec(),
            }]))
            .expect_err("a name carrying a quote is refused");
        assert!(refused.starts_with("a part's name is written into a part header"));

        let mut inbound = spec.build();
        assert!(body_of(&mut inbound).is_none());
    }

    /// The four fields a spec derives — `host`, `cookie`, `content-type` and
    /// `content-length` — are each written only where the spec did not write
    /// that name itself, so a request never carries a field that disagrees
    /// with itself.
    #[test]
    fn a_field_the_spec_wrote_itself_is_not_written_twice() {
        let mut spec = InboundSpec::new("POST", "/users");
        spec.push_header("host", b"written.example");
        spec.push_header("cookie", b"session=written");
        spec.push_header("content-type", b"text/plain");
        spec.set_host("bag.example");
        spec.push_cookie("session", "bag");
        spec.set_body(SpecBody::Form(vec![("name".to_owned(), "ada".to_owned())]))
            .expect("one spelling");

        let inbound = spec.build();
        assert_eq!(
            head(&inbound),
            vec![
                ("host".to_owned(), "written.example".to_owned()),
                ("cookie".to_owned(), "session=written".to_owned()),
                ("content-type".to_owned(), "text/plain".to_owned()),
                ("content-length".to_owned(), "8".to_owned()),
            ]
        );
    }

    /// Every cookie a spec names travels as the one `cookie` field line a peer
    /// would have sent — the pairs joined with `"; "` in the order they were
    /// pushed, which is the spelling RFC 6265 § 5.4 gives a client, and never a
    /// field line each.
    ///
    /// A spec that wrote a `cookie` line itself carries that one and no second:
    /// the pairs are dropped rather than merged into it, since a merge would
    /// send a request neither the line nor the pairs describe.
    #[test]
    fn a_cookies_field_becomes_one_cookie_header() {
        let mut spec = InboundSpec::new("GET", "/users");
        spec.push_cookie("session", "abc123");
        spec.push_cookie("theme", "dark");

        assert_eq!(
            head(&spec.build()),
            vec![("cookie".to_owned(), "session=abc123; theme=dark".to_owned())]
        );

        let mut written = InboundSpec::new("GET", "/users");
        written.push_header("cookie", b"session=written");
        written.push_cookie("session", "bag");

        assert_eq!(
            head(&written.build()),
            vec![("cookie".to_owned(), "session=written".to_owned())]
        );
    }

    /// Every field a spec names reaches the carrier: the verb and the path it
    /// was opened with, the query verbatim, the field line it wrote itself, the
    /// host and the cookies as the fields a peer would have sent, the body with
    /// the two fields its spelling declares, and the peer as
    /// `rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing`'s
    /// walk would have decided it.
    ///
    /// One spec naming all of them, because what this asks is that
    /// [`InboundSpec::build`] writes every field — what a given spelling
    /// encodes is that spelling's own test.
    #[test]
    fn a_spec_becomes_an_inbound_with_every_field_it_named() {
        let mut spec = InboundSpec::new("POST", "/users");
        spec.set_query("page=2&q=novis");
        spec.push_header("accept", b"application/json");
        spec.push_cookie("session", "abc123");
        spec.push_cookie("theme", "dark");
        spec.set_host("app.example");
        spec.set_peer(Some(IpAddr::from([203, 0, 113, 9])), Scheme::Https);
        spec.set_body(SpecBody::Form(vec![("name".to_owned(), "ada".to_owned())]))
            .expect("one spelling");

        let mut inbound = spec.build();
        assert_eq!(inbound.method(), "POST");
        assert_eq!(inbound.path(), "/users");
        assert_eq!(inbound.query(), "page=2&q=novis");
        assert_eq!(
            head(&inbound),
            vec![
                ("accept".to_owned(), "application/json".to_owned()),
                ("host".to_owned(), "app.example".to_owned()),
                ("cookie".to_owned(), "session=abc123; theme=dark".to_owned()),
                (
                    "content-type".to_owned(),
                    "application/x-www-form-urlencoded".to_owned()
                ),
                ("content-length".to_owned(), "8".to_owned()),
            ]
        );
        assert_eq!(inbound.client(), Some(IpAddr::from([203, 0, 113, 9])));
        assert_eq!(inbound.scheme(), Scheme::Https);
        assert_eq!(body_of(&mut inbound).as_deref(), Some("name=ada"));
    }
}
