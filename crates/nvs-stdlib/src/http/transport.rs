//! The outbound transport: one HTTP/1.1 exchange over `nvs_host`'s parking
//! stream, under one deadline that covers every hop, every attempt and every
//! backoff between them.
//!
//! [`super`] owns the *policy* — which URLs are allowed and which address one
//! was pinned to. What is here is the part that talks: compose a request, write
//! it, read the reply back, and decide whether what came back is an answer or
//! something to try again. The split is deliberate and it is why this module
//! takes an [`IpAddr`] rather than a `Ctx`: nothing here can widen a decision
//! the door already made, and a test can drive a whole exchange against a
//! listener on loopback without a capability snapshot in front of it.
//!
//! # Re-pinning is asked of the caller, not done here
//!
//! [`send`] takes a `repin` closure and calls it for every redirect hop, which
//! is `rule:http-server/redirects-are-off-and-every-hop-is-re-pinned`'s
//! "every hop is re-checked and re-pinned" with the checking left where the
//! checking lives. A retry never calls it: § 4's other half is that every
//! attempt of one call reuses the address the launderer approved, so there is
//! no second resolution for a rebinding attack to answer differently.
//!
//! The closure is handed one thing this module knows and the caller does not:
//! whether the hop steps out of `https` and into plaintext. Both schemes are
//! only in view here, and what to do about it belongs to the caller's grant and
//! the caller's options
//! (`rule:http-server/an-https-redirect-never-becomes-plaintext`).
//!
//! # A connection outlives its call, under a key this module builds
//!
//! [`one`] draws from [`super::pool`] before it opens anything, and the reply's
//! own reader gives the connection back once the body has been framed to its
//! end. [`pool_key`] is what the two agree on, and
//! `rule:http-server/an-outbound-connection-is-pooled-per-core-and-stays-pinned`
//! is why it is built here: the pinned address, the port, the scheme and the
//! server name are the facts the door checked, and reuse under anything less
//! would hand a call a connection its own check would have refused.
//!
//! A connection this core had been holding that fails with the request still
//! going out is replaced once and costs no attempt — a server closing an idle
//! connection is not a failed request — and every later failure is an attempt
//! under the retry rules. That is the one asymmetry between a drawn connection
//! and a fresh one; everything after the first byte of the reply is the same
//! code for both.
//!
//! What a call spends is one buffer holding the whole reply, capped at
//! [`REPLY_CEILING`], plus the request text, both released with the request. A
//! buffered reply that arrived compressed spends a second buffer for the
//! decoded octets, under that same ceiling lowered onto the operator's
//! (`rule:core-classes/decompression-bound`); a streamed one spends its
//! decoder's window and one [`STEP`] instead ([`compress::Decoder`]), which is
//! what bounds a decode that has no total for a ceiling to be over. What
//! outlives it is the connection itself, charged to the core under that
//! module's two caps.
//!
//! # `https` is three lines, because the stream is a plain `Read`/`Write`
//!
//! An `https` URL connects exactly as an `http` one does and then hands the
//! socket to [`nvs_host::tls::NvsTls`], which completes a handshake and gives
//! back a plaintext stream. Everything after that — [`exchange`] — is written
//! once and is generic over what it writes to, so there is no second copy of
//! the framing, the ceiling or the retry rules under TLS. That module owns the
//! trust anchors and why they are compiled in.
//!
//! Two things are decided here rather than there. The certificate is checked
//! against the **host the launderer approved**, never the address it was pinned
//! to: `rule:http-server/redirects-are-off-and-every-hop-is-re-pinned`
//! pins where the bytes go, and pinning is not a claim about who is there. And
//! a handshake failure splits the same way the rest of this module splits — a
//! certificate that does not verify is a statement about the other end that a
//! second attempt will not change, so it leaves as a `Fault` and never sleeps
//! first, while a timeout or a reset mid-handshake is transport weather and is
//! retried. There is no plaintext fallback, which is a priority-1 failure
//! wearing a feature's name. What a call may relax it says in a
//! [`nvs_host::tls::CallPolicy`] on the [`Call`], built where a grant for it
//! was proved and handed to the one door with the identity
//! (`rule:security/tls-trust-is-relaxed-only-under-a-host-grant`); this module
//! chooses none of it and carries it into the pool key.

use std::cell::RefCell;
use std::io::{ErrorKind, Read, Seek, Write};
use std::net::{IpAddr, SocketAddr};
use std::rc::Rc;
use std::time::{Duration, Instant};

use fluent_uri::component::{Authority, Scheme};
use fluent_uri::{Uri, UriRef};
use nvs_host::net::NvsTcp;
use nvs_host::tls::{CallPolicy, NvsTls, Session};
use nvs_runtime::{Fault, ThrownClass};
use rand::RngExt;
use tungstenite::http::{HeaderName, HeaderValue};
use tungstenite::protocol::{WebSocket, WebSocketConfig};

use super::pool;
use super::span::HttpSpan;
use crate::compress;

/// The most reply a single call will hold, headers and body together.
///
/// A cap and not a configuration: a reply is bytes another host chose, so
/// "until memory runs out" is that host deciding this process's footprint. Two
/// megabytes past the point where a caller should be reaching for
/// `Core\Http\Client::stream` instead.
pub(super) const REPLY_CEILING: usize = 8 * 1024 * 1024;

/// The most head a reply may carry before it is refused.
///
/// [`REPLY_CEILING`]'s argument one level down, and the reason there is a
/// second number: the head arrives before anything about the reply is known,
/// so it is the one part of a reply no framing bounds. A streamed call reads
/// only this much before it hands the connection on, so without a cap here an
/// origin that never ends its header section would be an unbounded hold that
/// the body's own two bounds never see.
const HEAD_CEILING: usize = 64 * 1024;

/// The longest line [`super::stream`]'s readers will frame out of a reply.
///
/// Beside [`REPLY_CEILING`] and for its argument: a line is bytes another host
/// chose, and a reader that framed one "until memory runs out" would let that
/// host pick this process's footprint one line at a time. A reply whose text
/// genuinely has no line breaks is read by `chunks()`, which frames nothing.
pub(super) const LINE_CEILING: usize = 64 * 1024;

/// The most `data` one server-sent event will accumulate, over however many
/// `data` lines it is spread across. [`LINE_CEILING`]'s argument, one level up:
/// the per-line cap alone would leave the event unbounded.
pub(super) const EVENT_CEILING: usize = 1024 * 1024;

/// How much of a file body this module holds at once.
///
/// `rule:http-server/an-outbound-request-carries-one-body` is why there is a
/// number here at all: a file part exists so that a body larger than anything
/// this process would hold can still be sent, and a copy of the file in memory
/// on the way to the socket would give that back. One buffer per attempt,
/// released with it, whatever the file's size.
const BODY_CHUNK: usize = 16 * 1024;

/// One call, whole: what to send, where it was pinned to, and every bound it
/// runs under.
///
/// Built by [`super::request`] out of the row's options and the runtime's
/// `[http.client]` defaults, so nothing here reads configuration and every
/// field is already the answer rather than a place to look one up.
pub(crate) struct Call<'a> {
    /// `Core\Http\Client::get` — what a refusal names.
    pub(crate) member: &'a str,
    /// The method, upper-cased, which is the row's own name.
    pub(crate) verb: &'a str,
    /// The URL as approved, and the one a relative `Location` resolves against.
    pub(crate) url: String,
    /// Every address [`super::pin`] approved for [`Call::url`]'s host, in the
    /// resolver's order and at most eight of them
    /// (`rule:http-server/an-outbound-call-tries-every-approved-address`).
    ///
    /// Empty in one case only: a destination this deployment's proxy resolves,
    /// where the door approved the host and no address for it was ever learned
    /// (`rule:http-server/a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise`).
    /// A name that answered nothing is the launderer's refusal and never
    /// reaches this field.
    pub(crate) addresses: Vec<IpAddr>,
    /// The whole call's budget — every attempt, every hop, every backoff.
    pub(crate) deadline: Instant,
    /// The handshake's own bound, which is separate from the total.
    pub(crate) connect_timeout: Duration,
    /// The longest silence a streamed body may hold — read by
    /// [`send_streamed`] and by nothing a buffered call reaches, since
    /// [`Call::deadline`] is what covers one of those whole.
    pub(crate) idle: Duration,
    /// The longest a streamed body may take altogether, counted from the call.
    /// [`Call::idle`]'s other half, and read in the same one place.
    pub(crate) max_duration: Duration,
    /// The caller's own headers, in the order the array wrote them — sent while
    /// the request is still inside the origin [`Call::url`] was approved at, and
    /// held back whole on a hop to any other
    /// (`rule:http-server/a-cross-origin-redirect-drops-credentials`, decided in
    /// [`compose`]).
    pub(crate) headers: Vec<(String, String)>,
    /// How many redirect hops may be followed. Zero is the default.
    pub(crate) redirects: u32,
    /// Attempts in total, counting the first. At least one.
    pub(crate) attempts: u32,
    /// The base delay full jitter is drawn under.
    pub(crate) backoff: Duration,
    /// `Idempotency-Key`, for the one verb that needs one.
    pub(crate) idempotency_key: Option<String>,
    /// What this call sends after its head, or `None` for a verb that carries
    /// nothing — already framed, like every other field here.
    pub(crate) body: Option<Body>,
    /// What this core's connection store is bounded by — `[http.client]
    /// pool_idle` and `pool_idle_timeout`, already resolved like every other
    /// field here.
    ///
    /// A call carries them rather than the pool reading them because that
    /// module holds no configuration and this one holds no `Ctx`: the two
    /// numbers are `System`-class (`rule:config/three-changeability-classes`),
    /// so every call on a core arrives with the same pair and a request cannot
    /// move them.
    pub(crate) pool: pool::Caps,
    /// What a reply that arrived compressed is decoded under — `[limits]
    /// max_decompressed` and `[limits] max_decompression_ratio`, already
    /// resolved.
    ///
    /// Here for [`Call::pool`]'s reason: [`compress::Bound::ceiling`] reads a
    /// `Ctx` and this module has none. A call carries the bound but cannot
    /// widen it — `rule:core-classes/decompression-bound` is one-way, and a
    /// reply is octets another host chose, so there is nothing here to ask for
    /// more with.
    pub(crate) compress: compress::Bound,
    /// Who this end is when a server asks the client for a certificate, or
    /// `None` where the call named no `identity`.
    ///
    /// Built once per call for [`Call::pool`]'s reason and released with it,
    /// which is O(in-flight): every attempt and every hop of one call presents
    /// the same identity, and no two calls share a built one.
    pub(crate) identity: Option<Identity>,
    /// The relaxations this call asked for and a `[capabilities.tls]` grant
    /// already allows for its host, or the default, which asks for nothing.
    ///
    /// Judged and built in `super::policy_of` for [`Call::identity`]'s reason —
    /// the question belongs to the deployment and this module holds no `Ctx` to
    /// put it to. A relaxing one costs the call one `ClientConfig`, built at the
    /// handshake and released with the call; a default one hands the process's
    /// own configuration back untouched
    /// (`rule:security/tls-trust-is-relaxed-only-under-a-host-grant`).
    pub(crate) policy: CallPolicy,
    /// The W3C `traceparent` naming the request this call is made from, or
    /// `None` where `[trace] propagate` is off.
    ///
    /// Already the answer, like every other field here: whether to propagate and
    /// what the id is are both read in [`super::traceparent_of`], off the `Ctx`
    /// this module deliberately cannot reach.
    pub(crate) traceparent: Option<String>,
    /// The `http` trace event this call files, as the routines that each
    /// contribute a fact to it write it —
    /// `rule:observability/trace-events-carry-a-kind`.
    ///
    /// **The one field here that is not already an answer**, and a `RefCell`
    /// rather than a `&mut` threaded through the hop loop, the attempt loop,
    /// the connection, the handshake and the exchange: those are five layers
    /// apart, every one of them is already holding a `&Call`, and a further
    /// parameter on each would be a signature a reader has to follow to find
    /// out where one number came from. [`Piece::File`]'s handle is the same
    /// shape for the same reason — a fact one attempt reaches through a shared
    /// borrow — and every borrow taken here is one statement long.
    ///
    /// It is filled whatever the debug flags say, for [`super::span`]'s stated
    /// reason, and `Core\Http\Client`'s member decides whether to file it. A
    /// call that never reaches an answer drops it.
    pub(crate) span: RefCell<HttpSpan>,
    /// The forward proxy every destination leaves through, or `None` where the
    /// operator wrote no `[http.client.proxy]` block
    /// (`rule:http-server/an-outbound-proxy-is-operator-configured`).
    ///
    /// Read in `super::proxy_of` for [`Call::pool`]'s reason, and `System`-class
    /// besides, so every call on a core carries the same block and nothing a
    /// request writes can move it.
    pub(crate) proxy: Option<Proxy>,
}

/// The forward proxy a call leaves through, as the operator wrote it and as
/// [`tunnel`] dials it.
///
/// The `url` text travels beside the address because it is what a refusal names
/// and what [`pool_key`] files a tunnelled connection under: two proxies are two
/// pools, and a reload that changes the text leaves nothing the old one made
/// drawable.
pub(crate) struct Proxy {
    /// `[http.client.proxy] url`, as written.
    pub(crate) url: String,
    /// The host [`Proxy::url`] names, resolved at each new tunnel and never
    /// asked `rule:security/net-address-policy`'s question: an endpoint in
    /// root-owned configuration is authorized by that writing, and the table
    /// exists to keep a *program-supplied* address away from the local machine.
    pub(crate) host: String,
    /// The port [`Proxy::url`] names, or `nvs_config::http::DEFAULT_PROXY_PORT`.
    pub(crate) port: u16,
    /// Whether `[http.client.proxy] resolve` says the **proxy** resolves the
    /// destination: `CONNECT` then carries the URL's own `host:port`, because
    /// the door approved the host and never learned an address for it, and
    /// `rule:security/net-address-policy`'s table is the proxy's to enforce
    /// (`rule:http-server/a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise`).
    ///
    /// `false` is `resolve = "local"`, where the set this end approved is what
    /// the tunnel is asked for, one `CONNECT` per address.
    pub(crate) by_name: bool,
    /// `[http.client.proxy] bypass`: the hosts reached directly, each entry
    /// exact or a leading `.` for a suffix.
    pub(crate) bypass: Vec<String>,
    /// The whole `Proxy-Authorization` value `[http.client.proxy] username` and
    /// its password make, or `None` where the operator wrote no credential.
    ///
    /// Built in `super::proxy_of` and carried rather than the pair, so that the
    /// one place holding a password in this module is the one line that writes
    /// it to the proxy: it goes on the `CONNECT` request alone — never to the
    /// destination, never on a redirect hop, and never into a trace event, a log
    /// record or an error message, all of which name [`Proxy::url`], which the
    /// boot refuses to let carry a credential.
    pub(crate) authorization: Option<String>,
}

impl Proxy {
    /// Whether `host` is one the operator listed in `bypass`, matched on the
    /// URL's own text before anything is resolved — so a bypassed destination is
    /// dialled directly and under the full address policy, and a list that could
    /// carry a range would be handing back traffic nobody can see.
    fn bypasses(&self, host: &str) -> bool {
        let host = host.as_bytes();
        self.bypass.iter().any(|entry| {
            let entry = entry.as_bytes();
            if entry.first() == Some(&b'.') {
                host.len() > entry.len()
                    && host[host.len() - entry.len()..].eq_ignore_ascii_case(entry)
            } else {
                host.eq_ignore_ascii_case(entry)
            }
        })
    }

    /// Whether this proxy is the one that resolves `host`: the operator wrote
    /// `resolve = "proxy"` and did not list the host in `bypass`.
    ///
    /// The door asks this — `super::pin_unless_the_proxy_resolves` — because it
    /// is what decides whether an address is looked up at all, and the answer
    /// has to be the same one [`one`] reads when it builds the `CONNECT`. A
    /// bypassed host is left out of it deliberately: it is reached directly and
    /// under the full address policy, so it is pinned like any other.
    pub(crate) fn resolves(&self, host: &str) -> bool {
        self.by_name && !self.bypasses(host)
    }
}

/// A client identity, as a call carries it: the session configuration every
/// handshake this call runs uses, and the name a pooled connection is filed
/// under.
///
/// Two fields and not one because they are read in two places that must not be
/// able to disagree — the handshake presents the chain, and [`pool_key`] writes
/// the fingerprint — and because a pool key is a string that ends up in no
/// diagnostic a private key could reach. The fingerprint is
/// `super::fingerprint_of`'s digest of the leaf, which is public.
pub(crate) struct Identity {
    /// What a handshake under this identity is configured with.
    pub(crate) session: nvs_host::tls::NvsIdentity,
    /// The leaf certificate's SHA-256, lower-case hex.
    pub(crate) fingerprint: String,
}

/// `rule:http-server/an-outbound-request-carries-one-body`'s one body, framed:
/// the type it is sent under, the length it promises, and the pieces in order.
///
/// **Framed once per call and written once per attempt**, which is the rule's
/// last sentence as a shape: [`super::body_of`] decides the encoding, the
/// boundary and the segments before the first connection, so a retry repeats
/// bytes rather than repeating the work that produced them.
///
/// `Content-Length` is a field here rather than a sum taken at the socket,
/// because it is written into the head before any piece is read and the two
/// must be the same number even where the file underneath has since changed
/// size.
pub(crate) struct Body {
    /// The `Content-Type` this framing chose, or `None` where the program sent
    /// octets and named no type for them.
    pub(crate) content_type: Option<String>,
    /// `Content-Length` — known before the first byte, which is why this client
    /// has no chunked request body.
    pub(crate) length: u64,
    /// The body in order.
    pub(crate) pieces: Vec<Piece>,
}

impl Body {
    /// Every piece of this body in one buffer, for a caller that is not a
    /// socket.
    ///
    /// `rule:testing/an-outbound-call-is-answered-from-a-table`'s record is the
    /// one such caller: a test asks what its subject *sent*, and the answer has
    /// to be the framed octets rather than the options the program wrote, or a
    /// case asserting on a multipart body would be asserting on this module's
    /// arithmetic rather than on its output. It is the same writer the socket
    /// gets, so what a faked call records and what a real one sends cannot
    /// disagree.
    ///
    /// # Errors
    ///
    /// [`write_body`]'s, for a file piece that cannot be rewound or read.
    pub(crate) fn collected(&self, member: &str) -> Result<Vec<u8>, Fault> {
        let mut out = Vec::new();
        match write_body(self, &mut out, member)? {
            None => Ok(out),
            // A `Vec` has no failure to report, so this arm is a bug in this
            // module rather than anything a program or a network can cause.
            Some(err) => Err(Fault::fatal(format!(
                "{member} could not collect the body it framed — {err}"
            ))),
        }
    }
}

/// One run of a [`Body`]: octets the framing built, or a file it did not read.
pub(crate) enum Piece {
    /// What the framing composed — a JSON document, an encoded form, a
    /// multipart segment, or a raw body the program was already holding.
    Held(Vec<u8>),
    /// A file, written from disk at [`BODY_CHUNK`] a time.
    ///
    /// The descriptor is opened **once**, through the capability door in
    /// [`super`] where a `Ctx` exists, and rewound for every attempt. Opening
    /// it there rather than re-opening it here is what makes the octets sent
    /// the ones the grant was checked against: a path re-resolved per attempt
    /// is a name another process may have moved between them.
    File {
        /// The open handle, rewound before each attempt writes from it.
        handle: RefCell<Box<dyn Rewindable>>,
        /// Its size when the body was framed, which is the length the head has
        /// already promised.
        length: u64,
    },
}

/// What a file piece is read through.
///
/// A handle and not a path, and a trait rather than the filesystem's own type:
/// this module can rewind one and read it, and has no spelling for opening one.
/// That is `rule:security/capability-check-at-the-door` where a transport meets
/// a file — the door is [`super::part_of`]'s, where a `Ctx` exists to ask it,
/// and what arrives here is already the thing the grant was checked against.
pub(crate) trait Rewindable: Read + Seek {}

impl<T: Read + Seek> Rewindable for T {}

/// What came back: the three things `Core\Http\Response` holds.
///
/// The header list is read twice from here — a `Location` and a `Retry-After`
/// are decisions this module makes, so they are parsed once rather than at two
/// call sites, and the whole list becomes the response's header slot, where
/// `header` and `headers` read it back.
#[derive(Debug)]
pub(crate) struct Reply {
    /// The status line's code.
    pub(crate) status: i64,
    /// The body's octets, exactly as they arrived once the transfer coding and
    /// the content coding were both undone — a `Vec<u8>` and not a `String`
    /// because whether they are text is
    /// `Core\Http\Response::text`'s question and not this module's, and a reply
    /// a program only wanted the bytes of must not throw on the way here.
    pub(crate) body: Vec<u8>,
    /// Every header, names lower-cased, values trimmed, in arrival order — a
    /// list rather than a map because a field the origin sent twice is two
    /// lines, and `Core\Http\Response::headers` is the member that says so.
    pub(crate) headers: Vec<(String, String)>,
    /// The TLS session this reply arrived over, or `None` for a plaintext one —
    /// `Core\Http\Response::tls`'s answer, taken at the framing for
    /// [`Session`]'s reason.
    pub(crate) tls: Option<Tls>,
}

/// The TLS session a reply arrived over, as `Core\Http\Response::tls` reports
/// it: what the handshake settled, and whether it checked anything.
///
/// Two fields because neither place can answer for the other. The session is
/// the completed handshake's own snapshot, from the layer that holds the socket;
/// `verified` is the call's policy read back, from the layer that asked the
/// deployment for it. `rule:http-server/a-reply-reports-its-tls-session` exists
/// for the second one — a deployment that relaxed verification at one host has
/// no other way to assert that every other call still verified.
#[derive(Debug)]
pub(crate) struct Tls {
    /// The version, the cipher and the peer's chain.
    pub(crate) session: Session,
    /// Whether the chain and the name were both checked, which is
    /// `CallPolicy::verifies` and not a reading of the five option fields here.
    pub(crate) verified: bool,
}

/// One connected socket, plaintext or inside a TLS session.
///
/// A trait object rather than the type parameter [`exchange`] would otherwise
/// take, because a streamed reply hands the connection **back**: [`one`] is the
/// last place the two spellings are still distinct, and boxing there is what
/// lets the framing, the ceiling and the reader a program walks be written
/// once.
pub(crate) trait Connection: Read + Write {
    /// Bounds every wait on this connection by `at`, or lifts the bound —
    /// `NvsTcp::set_deadline`, which owns what a deadline is.
    fn bound_by(&mut self, at: Option<Instant>);

    /// What the session over this connection negotiated, or `None` where there
    /// is no session.
    ///
    /// Asked here rather than remembered from [`one`]'s scheme, because this is
    /// the one place a reply's connection is still the typed thing it was
    /// opened as: past the box, plaintext and TLS are the same `Read` and
    /// `Write`, and a pooled connection arrives with no scheme beside it at all.
    fn tls(&self) -> Option<Session>;
}

impl Connection for NvsTcp {
    fn bound_by(&mut self, at: Option<Instant>) {
        self.set_deadline(at);
    }

    fn tls(&self) -> Option<Session> {
        None
    }
}

impl Connection for NvsTls<NvsTcp> {
    fn bound_by(&mut self, at: Option<Instant>) {
        self.set_deadline(at);
    }

    fn tls(&self) -> Option<Session> {
        Some(self.session())
    }
}

/// What delimits the body that follows a head.
#[derive(Clone, Copy)]
enum Frame {
    /// `Transfer-Encoding: chunked` — a hexadecimal length before each piece.
    Chunked,
    /// `Content-Length` — this many octets of body still to come.
    Sized(u64),
    /// Neither field: the body is over when the connection is.
    UntilClose,
}

/// What bounds a body once its head has been read.
///
/// The two arms are the two members, and they are an enum rather than a pair of
/// durations so that neither call can be given the other's bounds by arithmetic
/// — a buffered reply under an `idle` would be the wait
/// `rule:http-server/no-spelling-for-an-unbounded-wait` says has no spelling,
/// reached by passing the wrong field.
#[derive(Clone, Copy)]
enum Bounds {
    /// A buffered reply's: the one deadline that covers the whole call
    /// (`rule:http-server/one-deadline-covers-the-whole-call`).
    Whole,
    /// A streamed body's own two, which are the bag keys
    /// `rule:http-server/a-streamed-reply-is-bounded-by-idle-and-a-lifetime`
    /// names.
    Streamed {
        /// The longest gap between two reads that deliver anything.
        idle: Duration,
        /// When the body stops being allowed to say anything more.
        until: Instant,
    },
}

/// How much of a coded body one [`Incoming::pull`] decodes.
///
/// One socket read's worth, which is what [`Framed::pull`] reads into as well.
/// What a streamed decode holds beyond its window is one of these and never
/// whatever the frame would expand to, and that is the whole of why a decode
/// with no output ceiling over it is still bounded.
const STEP: usize = 8192;

/// A reply's body, read off the wire as a program asks for it.
///
/// This is what makes a streamed reply streamed: [`exchange`] stops at the end
/// of the head and hands the connection here, so what follows is framed one
/// read at a time rather than gathered whole first. A **buffered** reply is the
/// same reader drained in one go ([`Incoming::whole`]), which is why there is
/// one framing implementation and one place a bound is judged rather than two
/// of each.
///
/// A reply that arrived under a coding is that same framing with a decoder in
/// front of it ([`Incoming::under`]), so both kinds hand a reader decoded
/// octets and the walks in [`super::stream`] never learn which one they are on.
///
/// **What it spends:** one read buffer, whatever has been framed and no reader
/// has taken yet, — under chunked framing alone — at most one incomplete chunk
/// waiting for the rest of itself, and for a coded reply one decoding window
/// and one [`STEP`].
pub(crate) struct Incoming {
    /// The body as it is read, with a decoder in front of the framing or not.
    body: Coding,
    /// Where the framing leaves a refusal that has to travel out past a
    /// decoder.
    ///
    /// A decoder is a `Read` and a `Read` carries an `io::Error`, which has no
    /// room for the thrown class that makes a `TimeoutError` catchable as one.
    /// So the framing files the real refusal here on its way out and returns a
    /// plain error for the backend to wrap however it likes, and
    /// [`Incoming::pull`] takes it back before it reads what the backend said.
    faulted: Rc<RefCell<Option<Fault>>>,
}

/// Whether a decoder sits between the framing and the reader.
enum Coding {
    /// The reply arrived as it was written, and what the framing holds is what
    /// a reader takes.
    ///
    /// Boxed, and not for its size: a decoder takes its source as a `Box<dyn
    /// Read>`, so the box a framing already sits in is the one the decode ends
    /// up reading through and [`Incoming::under`] allocates nothing.
    As(Box<Framed>),
    /// One coding, undone as the octets arrive.
    Under {
        /// The decode, which has taken ownership of the framing.
        decoder: compress::Decoder,
        /// Decoded octets not yet taken by a reader.
        held: Vec<u8>,
        /// Whether the decoder has said there is no more.
        ended: bool,
    },
}

/// One reply body's framing: octets off the connection, delimited, with no
/// coding undone.
struct Framed {
    /// The connection the rest of the body is still arriving on, or `None` once
    /// the framing has ended it and for a body that was whole to begin with.
    source: Option<Box<dyn Connection>>,
    /// Body octets pulled and framed, and not yet taken by a reader.
    held: Vec<u8>,
    /// Octets pulled and not yet framed, which only chunked framing ever leaves
    /// anything in.
    raw: Vec<u8>,
    /// What delimits this body.
    frame: Frame,
    /// Whether the framing has said there is no more.
    ended: bool,
    /// The longest silence this body may hold.
    idle: Duration,
    /// The instant it stops being allowed to say anything more.
    until: Instant,
    /// What a refusal names, owned because a stream outlives its [`Call`].
    member: String,
    /// Where the connection goes when the framing ends this body, or `None` for
    /// a reply after which it is closed instead.
    reuse: Option<Reuse>,
    /// [`Incoming::faulted`], shared so a refusal survives the trip out through
    /// a decoder.
    faulted: Rc<RefCell<Option<Fault>>>,
}

/// Where a connection goes once its reply is over: the key it is filed under
/// and the caps its core holds it inside.
///
/// Carried by the reader rather than handed back to [`one`], because the
/// instant the body ends is the instant the connection is good for another
/// request, and only the reader is still there when a streamed one does.
struct Reuse {
    /// [`pool_key`]'s key for the call this reply answered.
    key: String,
    /// What this core's store is bounded by.
    caps: pool::Caps,
}

impl nvs_runtime::HeldReader for Incoming {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl std::fmt::Debug for Incoming {
    /// What the request's table of readers prints, which is what it can:
    /// [`Connection`] is a trait object over a socket with no `Debug` of its
    /// own, and the octets are a reply another host wrote and not something to
    /// spill into a line meant for reading a `Ctx`. The member, the coding and
    /// the lengths are the facts a reader of that line is after.
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut line = out.debug_struct("Incoming");
        line.field("member", &self.member());
        match &self.body {
            Coding::As(framed) => line
                .field("coded", &false)
                .field("held", &framed.held.len())
                .field("raw", &framed.raw.len())
                .field("ended", &framed.ended),
            Coding::Under { held, ended, .. } => line
                .field("coded", &true)
                .field("held", &held.len())
                .field("ended", ended),
        };
        line.finish_non_exhaustive()
    }
}

impl Framed {
    /// A body still on `source`, where `raw` is whatever arrived beside the
    /// head.
    ///
    /// # Errors
    ///
    /// [`Framed::deframe`]'s, for a chunk header that is not a length.
    fn over(
        source: Box<dyn Connection>,
        raw: Vec<u8>,
        frame: Frame,
        idle: Duration,
        until: Instant,
        member: &str,
        reuse: Option<Reuse>,
    ) -> Result<Self, Fault> {
        let mut body = Self {
            source: Some(source),
            held: Vec::new(),
            raw,
            frame,
            ended: false,
            idle,
            until,
            member: member.to_owned(),
            reuse,
            faulted: Rc::new(RefCell::new(None)),
        };
        // The octets that came with the head are already here, and a reply
        // short enough to arrive in one read is over before the first `pull` —
        // which is where its connection goes back, since there is nothing left
        // for anyone to wait for.
        if body.deframe()? {
            body.finished();
        }
        Ok(body)
    }

    /// A body that is already here in full, on no connection at all — what an
    /// armed answer table hands a streamed call
    /// (`rule:testing/an-outbound-call-is-answered-from-a-table`).
    ///
    /// The two bounds are never consulted: they bound a wait, and a body with
    /// nothing left to arrive never waits. Filing the table's octets as a
    /// reader rather than as octets is what keeps one reading of a streamed
    /// body — the walks frame off a reader and have no second arm for a reply
    /// a test wrote.
    fn already(held: Vec<u8>, member: &str) -> Self {
        Self {
            source: None,
            held,
            raw: Vec::new(),
            frame: Frame::UntilClose,
            ended: true,
            idle: Duration::ZERO,
            until: Instant::now(),
            member: member.to_owned(),
            reuse: None,
            faulted: Rc::new(RefCell::new(None)),
        }
    }

    /// Ends the body, and gives the connection back to this core where the
    /// reply left it good for another request.
    ///
    /// `reuse` is `None` for every reply [`reusable`] refused, and the
    /// connection is dropped instead — as it is for a body nobody read to the
    /// end, since this runs only where the framing said there is no more.
    fn finished(&mut self) {
        self.ended = true;
        let Some(connection) = self.source.take() else {
            return;
        };
        if let Some(reuse) = self.reuse.take() {
            pool::release(reuse.key, connection, reuse.caps, Instant::now());
        }
    }

    /// The octets framed and not yet taken.
    fn held(&self) -> &[u8] {
        &self.held
    }

    /// Whether the framing has said there is no more: what tells a walk that
    /// the octets it could not frame an element out of are all it will ever
    /// get, rather than a piece of one still on the wire.
    fn ended(&self) -> bool {
        self.ended
    }

    /// Drops the first `octets` of what is framed and untaken — the element a
    /// walk has just handed a program.
    ///
    /// The move is what bounds a walk's memory: what this holds between two
    /// elements is one read's worth plus whatever of the next element has
    /// arrived, rather than every byte of the body the walk has already been
    /// through.
    fn consume(&mut self, octets: usize) {
        self.held.drain(..octets.min(self.held.len()));
    }

    /// Waits for more body, answering whether any arrived — `false` is the end
    /// of the body, and a silence is a throw rather than an end.
    ///
    /// # Errors
    ///
    /// `TimeoutError` for a silence past `idle` or a body past its lifetime,
    /// `IOError` for a connection that failed mid-body, and a `RuntimeError`
    /// for chunked framing the other end never finished.
    fn pull(&mut self) -> Result<bool, Fault> {
        let had = self.held.len();
        let mut buffer = [0_u8; STEP];
        while !self.ended {
            let now = Instant::now();
            if now >= self.until {
                return Err(outlived(&self.member));
            }
            let Some(mut source) = self.source.take() else {
                self.ended = true;
                break;
            };
            // Both bounds on the one wait: whichever is nearer is what the
            // socket parks under, and which of them it was is read back off the
            // clock when it fires.
            source.bound_by(Some(self.until.min(now + self.idle)));
            match source.read(&mut buffer) {
                Ok(0) => {
                    self.ended = true;
                    if matches!(self.frame, Frame::Chunked) {
                        return Err(malformed(
                            &self.member,
                            "a chunk is shorter than its own header said",
                        ));
                    }
                }
                Ok(read) => {
                    self.source = Some(source);
                    self.raw.extend_from_slice(&buffer[..read]);
                    if self.deframe()? {
                        self.finished();
                    }
                }
                Err(err) if err.kind() == ErrorKind::Interrupted => self.source = Some(source),
                Err(err) if err.kind() == ErrorKind::TimedOut => {
                    return Err(if Instant::now() >= self.until {
                        outlived(&self.member)
                    } else {
                        silent(&self.member, self.idle)
                    });
                }
                Err(err) => {
                    return Err(Fault::thrown_as(
                        ThrownClass::Io,
                        format!("{}: reading the reply failed — {err}", self.member),
                    ));
                }
            }
            if self.held.len() > had {
                return Ok(true);
            }
        }
        Ok(self.held.len() > had)
    }

    /// Moves everything [`Framed::raw`] now holds that is body into
    /// [`Framed::held`], answering whether the framing has ended the body.
    ///
    /// # Errors
    ///
    /// A `RuntimeError` for a chunk header that is not a hexadecimal length.
    fn deframe(&mut self) -> Result<bool, Fault> {
        match self.frame {
            Frame::Chunked => self.dechunk(),
            Frame::Sized(left) => {
                let take = usize::try_from(left)
                    .unwrap_or(usize::MAX)
                    .min(self.raw.len());
                self.held.extend(self.raw.drain(..take));
                let left = left - u64::try_from(take).unwrap_or(u64::MAX);
                self.frame = Frame::Sized(left);
                Ok(left == 0)
            }
            Frame::UntilClose => {
                self.held.append(&mut self.raw);
                Ok(false)
            }
        }
    }

    /// As many whole chunks as [`Framed::raw`] holds, moved across, leaving
    /// the part-chunk at the end for the read that completes it.
    ///
    /// # Errors
    ///
    /// A `RuntimeError` for a chunk header that is not text or not a length.
    fn dechunk(&mut self) -> Result<bool, Fault> {
        let mut from = 0_usize;
        let ended = loop {
            let Some(end) = find(&self.raw[from..], b"\r\n") else {
                break false;
            };
            let header = std::str::from_utf8(&self.raw[from..from + end])
                .map_err(|_| malformed(&self.member, "a chunk header is not text"))?;
            let size =
                usize::from_str_radix(header.split(';').next().unwrap_or_default().trim(), 16)
                    .map_err(|_| {
                        malformed(&self.member, "a chunk header is not a hexadecimal length")
                    })?;
            let at = from + end + 2;
            if size == 0 {
                // Whatever trailer follows the last chunk is not body, and the
                // connection is about to be dropped with it.
                from = self.raw.len();
                break true;
            }
            if self.raw.len() < at + size + 2 {
                break false;
            }
            self.held.extend_from_slice(&self.raw[at..at + size]);
            from = at + size + 2;
        };
        self.raw.drain(..from);
        Ok(ended)
    }
}

impl Read for Framed {
    /// The framed octets, as the source a decoder reads from.
    ///
    /// A decoder needs a source that blocks until there is something or the
    /// body is over, which is exactly [`Framed::pull`]: a `0` here means the
    /// framing ended the body and never that the rest is still coming, so a
    /// decoder cannot mistake a socket that has gone quiet for the end of its
    /// frame — the quiet is [`Framed::pull`]'s own refusal instead.
    ///
    /// That refusal is filed in [`Framed::faulted`] and what comes back is a
    /// plain error, because whatever the backend wraps this in on the way out
    /// would otherwise be all that is left of it.
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        while self.held.is_empty() && !self.ended {
            match self.pull() {
                Ok(true) => {}
                Ok(false) => break,
                Err(fault) => {
                    *self.faulted.borrow_mut() = Some(fault);
                    return Err(std::io::Error::other("the body's own refusal, filed"));
                }
            }
        }
        let take = out.len().min(self.held.len());
        out[..take].copy_from_slice(&self.held[..take]);
        self.held.drain(..take);
        Ok(take)
    }
}

impl Incoming {
    /// A body still on `source`, where `raw` is whatever arrived beside the
    /// head.
    ///
    /// # Errors
    ///
    /// [`Framed::deframe`]'s, for a chunk header that is not a length.
    fn over(
        source: Box<dyn Connection>,
        raw: Vec<u8>,
        frame: Frame,
        idle: Duration,
        until: Instant,
        member: &str,
        reuse: Option<Reuse>,
    ) -> Result<Self, Fault> {
        let framed = Framed::over(source, raw, frame, idle, until, member, reuse)?;
        Ok(Self {
            faulted: Rc::clone(&framed.faulted),
            body: Coding::As(Box::new(framed)),
        })
    }

    /// A body that is already here in full, on no connection at all — what an
    /// armed answer table hands a streamed call
    /// (`rule:testing/an-outbound-call-is-answered-from-a-table`).
    pub(crate) fn already(held: Vec<u8>, member: &str) -> Self {
        let framed = Framed::already(held, member);
        Self {
            faulted: Rc::clone(&framed.faulted),
            body: Coding::As(Box::new(framed)),
        }
    }

    /// The same body with `codec` undone as it arrives.
    ///
    /// The decoder takes the framing, so from here on the socket is reached
    /// only through the decode and what a reader asks this for is decoded
    /// octets. It is wrapped at the head and never later: a decoder that
    /// started part way through a frame would be reading from the middle of
    /// one, and there is no reader yet at the head to have taken anything.
    fn under(self, codec: compress::Codec, member: &str) -> Self {
        let Self { body, faulted } = self;
        match body {
            Coding::As(framed) => Self {
                body: Coding::Under {
                    decoder: compress::Decoder::over(codec, framed, member),
                    held: Vec::new(),
                    ended: false,
                },
                faulted,
            },
            under @ Coding::Under { .. } => Self {
                body: under,
                faulted,
            },
        }
    }

    /// What a refusal from this body names.
    fn member(&self) -> &str {
        match &self.body {
            Coding::As(framed) => &framed.member,
            Coding::Under { decoder, .. } => decoder.member(),
        }
    }

    /// The octets a reader may take: framed, and decoded where there is a
    /// coding.
    pub(crate) fn held(&self) -> &[u8] {
        match &self.body {
            Coding::As(framed) => framed.held(),
            Coding::Under { held, .. } => held,
        }
    }

    /// Whether the body has said there is no more: what tells a walk that the
    /// octets it could not frame an element out of are all it will ever get,
    /// rather than a piece of one still on the wire.
    pub(crate) fn ended(&self) -> bool {
        match &self.body {
            Coding::As(framed) => framed.ended(),
            Coding::Under { ended, .. } => *ended,
        }
    }

    /// Drops the first `octets` of what is held and untaken — the element a
    /// walk has just handed a program.
    ///
    /// The move is what bounds a walk's memory: what this holds between two
    /// elements is one step's worth plus whatever of the next element has
    /// arrived, rather than every byte of the body the walk has already been
    /// through.
    pub(crate) fn consume(&mut self, octets: usize) {
        match &mut self.body {
            Coding::As(framed) => framed.consume(octets),
            Coding::Under { held, .. } => {
                held.drain(..octets.min(held.len()));
            }
        }
    }

    /// Waits for more body, answering whether any arrived — `false` is the end
    /// of the body, and a silence is a throw rather than an end.
    ///
    /// A coded body is decoded one [`STEP`] at a time here rather than gathered
    /// and decoded at the end, which is what keeps a compressed stream a
    /// stream: an event under `gzip` reaches the walk that is waiting for it
    /// while the rest of the body is still being written.
    ///
    /// # Errors
    ///
    /// `TimeoutError` for a silence past `idle` or a body past its lifetime,
    /// `IOError` for a connection that failed mid-body, a `RuntimeError` for
    /// chunked framing the other end never finished, and a `ParseError` for a
    /// coded body whose frame is not what its `Content-Encoding` said.
    pub(crate) fn pull(&mut self) -> Result<bool, Fault> {
        let (decoder, held, ended) = match &mut self.body {
            Coding::As(framed) => return framed.pull(),
            Coding::Under {
                decoder,
                held,
                ended,
            } => (decoder, held, ended),
        };
        if *ended {
            return Ok(false);
        }
        let mut step = [0_u8; STEP];
        let read = match decoder.pull_into(&mut step) {
            Ok(read) => read,
            Err(why) => {
                // A refused frame is over. The decoder owns the framing, and
                // reading one it has already refused would ask the socket for
                // octets nothing downstream can do anything with.
                *ended = true;
                // The framing's own refusal if it had one, because it is the
                // one that says `TimeoutError` rather than `ParseError`.
                return Err(self.faulted.borrow_mut().take().unwrap_or(why));
            }
        };
        if read == 0 {
            *ended = true;
            return Ok(false);
        }
        held.extend_from_slice(&step[..read]);
        Ok(true)
    }

    /// The whole body, under `ceiling`.
    ///
    /// # Errors
    ///
    /// [`Incoming::pull`]'s, and a `RuntimeError` for a body past `ceiling`.
    pub(crate) fn whole(&mut self, ceiling: usize) -> Result<Vec<u8>, Fault> {
        while self.pull()? {
            if self.held().len() > ceiling {
                return Err(Fault::thrown(format!(
                    "{}: the reply passed {ceiling} bytes, which is as much of one another host \
                     is allowed to make this process hold",
                    self.member()
                )));
            }
        }
        Ok(self.taken())
    }

    /// Everything held, moved out.
    fn taken(&mut self) -> Vec<u8> {
        match &mut self.body {
            Coding::As(framed) => std::mem::take(&mut framed.held),
            Coding::Under { held, .. } => std::mem::take(held),
        }
    }
}

/// A reply whose head has arrived and whose body has not.
///
/// What every attempt produces, and what `Core\Http\Client::stream` hands a
/// program: a buffered call is this with [`Incoming::whole`] called on it, so
/// the two members read one reply implementation rather than two.
pub(crate) struct Streamed {
    /// The status line's code.
    pub(crate) status: i64,
    /// Every header, as [`Reply::headers`] describes them.
    pub(crate) headers: Vec<(String, String)>,
    /// The rest of the reply, still on the socket.
    pub(crate) body: Incoming,
    /// The session it arrived over, as [`Reply::tls`] describes it.
    pub(crate) tls: Option<Tls>,
}

/// What one attempt produced: an answer, or a failure worth trying again.
///
/// The distinction is `rule:http-server/retry-is-opt-in-jittered-and-closed`
/// 's "what is retried": a connection failure and a timeout are transport
/// weather and come back as [`Attempt::Failed`], while a malformed reply is a
/// statement about the other end that a second identical request will not
/// change, so it leaves as a `Fault` and never sleeps first.
enum Attempt {
    /// The other end answered, whatever it said.
    Answered(Streamed),
    /// The socket did not get there, with the sentence a refusal would carry.
    Failed(String),
}

/// What one connection's exchange produced, before [`one`] has decided what a
/// failure on *that* connection costs.
///
/// [`Attempt`] is the same question one level up, and the two are separate
/// because the answer depends on where the connection came from: a fresh socket
/// that closes before it answers is a statement about the other end, while the
/// same silence on one this core had been holding idle is the far end retiring
/// it. Only [`one`] knows which it was.
enum Sent {
    /// The other end answered, whatever it said.
    Answered(Streamed),
    /// It failed with the request still going out, so nothing complete reached
    /// the other end.
    WhileSending(String),
    /// The connection ended before an octet of the reply arrived.
    Silent(String),
    /// Anything else the socket did, after the request was out.
    Failed(String),
}

/// The URL, split into the four things composing a request needs.
struct Parts {
    /// The host, brackets and all for an IPv6 literal, as `Host:` writes it.
    authority: String,
    /// The bare host, with no port — what a certificate is checked against.
    ///
    /// Separate from `authority` because that one carries the port where the
    /// URL wrote one, and a server name with a port in it names nothing.
    host: String,
    /// Where to connect, defaulted by scheme.
    port: u16,
    /// The request-target: path, and query if there was one.
    target: String,
    /// Whether the scheme runs inside TLS — `https`, and `wss` for the row that
    /// opens a socket — and so whether a handshake runs before anything is sent.
    tls: bool,
}

/// What a redirect hop is re-pinned with: the hop's URL and whether it leaves
/// TLS behind, answered with the set the door approved.
///
/// A name rather than the signature written out three times, which is the same
/// argument the module doc makes about the closure itself — the decision is
/// [`super::repinned`]'s and what crosses this boundary is only its answer.
/// The lifetime is written out because a trait object inside an alias is
/// `'static` by default, where the same type spelled at the parameter takes the
/// reference's — and every closure handed here borrows the `Ctx` it decides on.
type Repin<'a> = dyn FnMut(&str, bool) -> Result<Vec<IpAddr>, Fault> + 'a;

/// Runs `call` to an answer, re-pinning through `repin` at every redirect hop.
///
/// The loop is the ADR's: attempts inside, hops outside, one deadline over both.
/// A hop past [`Call::redirects`] is not an error — the redirect *is* the
/// answer, and a program that asked to follow none gets the `301` back rather
/// than a throw about a reply the origin was entitled to send.
///
/// # Errors
///
/// A `TimeoutError` when the deadline passes, an `IOError` when the last
/// attempt could not reach the address, a `RuntimeError` for a reply that is
/// not HTTP or a body that is not text, and whatever `repin` refuses a hop
/// with.
fn sent(call: &Call<'_>, repin: &mut Repin<'_>, bounds: Bounds) -> Result<Streamed, Fault> {
    let mut url = call.url.clone();
    let mut addresses = call.addresses.clone();
    let mut tls = parts(&url, call.member)?.tls;
    let mut hops = 0_u32;
    loop {
        let reply = attempts(call, &url, &addresses, bounds)?;
        let Some(location) = redirect_of(&reply) else {
            return Ok(reply);
        };
        if hops >= call.redirects {
            return Ok(reply);
        }
        hops += 1;
        call.span.borrow_mut().hopped();
        url = resolved(&url, &location, call.member)?;
        let onward = parts(&url, call.member)?.tls;
        // Before the connection and not after it: § 4 refuses a hop on its
        // *address*, and an address that has not been asked about yet is one
        // the first URL's approval is standing in for. The second argument is
        // the question this module can answer and must not decide — a hop out
        // of TLS into plaintext, which is the module doc's paragraph above.
        addresses = repin(&url, tls && !onward)?;
        tls = onward;
    }
}

/// A buffered reply, whole: [`sent`] under the one deadline, drained.
///
/// # Errors
///
/// [`sent`]'s, a `RuntimeError` for a body past [`REPLY_CEILING`], and
/// [`coding_of`]'s for a reply under a coding [`OFFERED`] does not name.
pub(crate) fn send(call: &Call<'_>, repin: &mut Repin<'_>) -> Result<Reply, Fault> {
    let mut answer = sent(call, repin, Bounds::Whole)?;
    let body = answer.body.whole(REPLY_CEILING)?;
    let Some(codec) = coding_of(&answer.headers, call.member)? else {
        return Ok(Reply {
            status: answer.status,
            body,
            headers: answer.headers,
            tls: answer.tls,
        });
    };
    // A decoded reply is still a reply this call holds whole, so
    // [`REPLY_CEILING`] is this module's own ask against the operator's
    // ceiling, and [`compress::Bound::within`] takes the smaller on each axis.
    // Lowering it here rather than trusting the ratio is what keeps a call's
    // footprint the one number the rest of this module is written on.
    let bound = call.compress.within(compress::Bound {
        bytes: REPLY_CEILING as u64,
        ratio: u64::MAX,
    });
    let body = compress::decompress_within(codec, &body, bound, call.member)?;
    // The two fields described the frame and not the body. What a program
    // reads back is decoded, so a `Content-Encoding` still naming the coding
    // and a `Content-Length` still counting the compressed octets would both be
    // answers to a question nobody can ask any more.
    answer
        .headers
        .retain(|(name, _)| name != "content-encoding" && name != "content-length");
    Ok(Reply {
        status: answer.status,
        body,
        headers: answer.headers,
        tls: answer.tls,
    })
}

/// What every request offers, and so the only codings a reply may arrive
/// under.
///
/// Three and not more: these are the codings this process can undo under
/// `rule:core-classes/decompression-bound`, and an offer is a promise to decode
/// what comes back. `deflate` is deliberately absent — it is two framings on
/// the wire under one name, and an origin picking the other one is a decode
/// this client would get wrong rather than refuse.
const OFFERED: &str = "gzip, br, zstd";

/// The codec a reply's `Content-Encoding` names, or `None` for a body that
/// arrived as it was written.
///
/// One coding or none. A list is refused rather than peeled: [`OFFERED`] offers
/// three single codings and never a stack of them, so a reply under two is one
/// no request here asked for, and peeling would be a decode per layer where the
/// bound is judged per layer.
///
/// # Errors
///
/// A thrown `RuntimeError` naming the coding, for anything else the field
/// holds.
fn coding_of(headers: &[(String, String)], member: &str) -> Result<Option<compress::Codec>, Fault> {
    let Some(named) = header(headers, "content-encoding") else {
        return Ok(None);
    };
    let named = named.trim();
    // `identity` is the field spelled as the absence of one, and an origin is
    // allowed to write it.
    if named.is_empty() || named.eq_ignore_ascii_case("identity") {
        return Ok(None);
    }
    if named.eq_ignore_ascii_case("gzip") {
        return Ok(Some(compress::Codec::Gzip));
    }
    if named.eq_ignore_ascii_case("br") {
        return Ok(Some(compress::Codec::Brotli));
    }
    if named.eq_ignore_ascii_case("zstd") {
        return Ok(Some(compress::Codec::Zstd));
    }
    Err(Fault::thrown(format!(
        "{member}: the reply arrived under `Content-Encoding: {named}`, which this request did \
         not offer — every one of them offers `{OFFERED}` and nothing else. A coding this client \
         cannot undo is refused rather than handed on as the octets it is still in."
    )))
}

/// The refusal for a program that wrote an `Accept-Encoding` of its own.
///
/// Refused rather than dropped or sent beside ours, because either of those is
/// this client deciding what the program meant: two offers let the origin pick,
/// and one of the two would name a coding [`coding_of`] then refuses.
fn own_offer(member: &str, name: &str) -> Fault {
    Fault::thrown(format!(
        "{member}: `{name}` is this client's own header and not a program's — every request \
         offers `{OFFERED}`, which is exactly the set a reply is decoded from under \
         `rule:core-classes/decompression-bound`."
    ))
}

/// The same call, stopped at the head, with the body left on the socket.
///
/// The head is under [`Call::deadline`] exactly as a buffered call's is — a
/// stream's own two bounds start where the head ends, which is why every retry
/// and every hop below is the same code and only what comes after them differs
/// (`rule:http-server/a-streamed-reply-is-bounded-by-idle-and-a-lifetime`).
///
/// A reply under one of the codings [`OFFERED`] is decoded **as it arrives**
/// rather than gathered first: [`Incoming::under`] puts the decoder in front of
/// the framing, and what the walks read is decoded octets one [`STEP`] at a
/// time. What bounds that decode is its window and not a total, because a
/// stream has no total to have one over.
///
/// # Errors
///
/// [`sent`]'s, and [`coding_of`]'s for a reply under a coding [`OFFERED`] does
/// not name. The body's own refusals arrive later, at the reader
/// ([`Incoming::pull`]).
pub(crate) fn send_streamed(call: &Call<'_>, repin: &mut Repin<'_>) -> Result<Streamed, Fault> {
    let bounds = Bounds::Streamed {
        idle: call.idle,
        until: Instant::now() + call.max_duration,
    };
    let mut answer = sent(call, repin, bounds)?;
    let Some(codec) = coding_of(&answer.headers, call.member)? else {
        return Ok(answer);
    };
    answer.body = answer.body.under(codec, call.member);
    // [`send`]'s reason and the same two fields: what a program reads back is
    // decoded, so a `Content-Encoding` still naming the coding and a
    // `Content-Length` still counting the compressed octets are both answers to
    // a question nobody can ask any more.
    answer
        .headers
        .retain(|(name, _)| name != "content-encoding" && name != "content-length");
    Ok(answer)
}

/// One URL's worth of attempts, under the call's own deadline.
fn attempts(
    call: &Call<'_>,
    url: &str,
    addresses: &[IpAddr],
    bounds: Bounds,
) -> Result<Streamed, Fault> {
    let mut attempt = 0_u32;
    loop {
        if Instant::now() >= call.deadline {
            return Err(expired(call.member));
        }
        let last = attempt + 1 >= call.attempts;
        call.span.borrow_mut().attempted();
        let wait = match one(call, url, addresses, bounds)? {
            Attempt::Answered(reply) => {
                if last || !retryable(reply.status) {
                    return Ok(reply);
                }
                retry_after(&reply).unwrap_or_else(|| backoff(call.backoff, attempt))
            }
            Attempt::Failed(why) => {
                // A failure that arrives with the budget already gone is the
                // budget's, not the socket's: the wait this call was allowed
                // is what ended it, and `TimeoutError` is the class § 5 names.
                if Instant::now() >= call.deadline {
                    return Err(expired(call.member));
                }
                if last {
                    return Err(Fault::thrown_as(
                        ThrownClass::Io,
                        format!("{}: {why}", call.member),
                    ));
                }
                backoff(call.backoff, attempt)
            }
        };

        // § 6: the deadline is not extended. A backoff that would end past it
        // throws now rather than sleeping through the budget and then failing.
        if Instant::now() + wait >= call.deadline {
            return Err(expired(call.member));
        }
        nvs_host::timer::sleep(wait);
        attempt += 1;
    }
}

/// One request and one reply, over a connection this core was already holding
/// where it had one, and otherwise over the first address of the approved set
/// to come up.
///
/// The set is walked rather than its first address used, under the one
/// `connectTimeout` clamped below — `rule:http-server/an-outbound-call-tries-every-approved-address`,
/// and [`nvs_host::net::NvsTcp::connect_racing`] is the walk itself.
fn one(call: &Call<'_>, url: &str, addresses: &[IpAddr], bounds: Bounds) -> Result<Attempt, Fault> {
    let parts = parts(url, call.member)?;
    // Where this attempt is going, written before anything can fail: the event
    // names the origin that answered and the path it asked for, and
    // [`HttpSpan::at`] is what keeps the query string out of both.
    call.span
        .borrow_mut()
        .at(parts.tls, &parts.host, parts.port, &parts.target);
    let request = compose(call, &parts)?;
    let approved: Vec<SocketAddr> = addresses
        .iter()
        .map(|&address| SocketAddr::new(address, parts.port))
        .collect();
    let identity = call.identity.as_ref();
    let fingerprint = identity.map(|held| held.fingerprint.as_str());
    // Whether this destination leaves through the proxy at all, asked of the
    // URL's own host text before an address is chosen and before the pool is
    // read: a bypassed host is dialled directly, so it is also a key of its own
    // (`rule:http-server/an-outbound-proxy-is-operator-configured`).
    let through = call
        .proxy
        .as_ref()
        .filter(|proxy| !proxy.bypasses(&parts.host));
    // A destination the proxy resolves has no address on this side at all: the
    // door approved its host and stopped there, so there is one connection to
    // look for and it is filed under the name the `CONNECT` below carries.
    let by_name = through.is_some_and(|proxy| proxy.by_name);
    // A connection to *any* approved address serves the call, since each of
    // them was approved and each is filed under the address it actually goes
    // to: one lookup per address rather than one on the set's first
    // (`rule:http-server/an-outbound-connection-is-pooled-per-core-and-stays-pinned`).
    let drawn = if by_name {
        let key = pool_key(&parts, None, fingerprint, &call.policy, through);
        pool::take(&key, Instant::now()).map(|held| (None, key, held))
    } else {
        approved.iter().find_map(|&address| {
            let key = pool_key(&parts, Some(address), fingerprint, &call.policy, through);
            pool::take(&key, Instant::now()).map(|held| (Some(address), key, held))
        })
    };

    if let Some((socket, key, mut held)) = drawn {
        // The drawn connection carries no bound of its own: the call's deadline
        // is what every wait on it is under, exactly as on a fresh one.
        held.bound_by(Some(call.deadline));
        // Nothing opened and nothing negotiated, which is the pair of zeroes the
        // event reports for a call this core was already holding a connection
        // for.
        call.span.borrow_mut().drawn(socket);
        match exchange(
            call,
            held,
            &request,
            &reached_at(&parts, socket),
            bounds,
            &key,
        )? {
            Sent::Answered(reply) => return Ok(Attempt::Answered(reply)),
            // The rule's *replaced once without spending an attempt*: the
            // request did not leave this process whole, so the far end cannot
            // have acted on it, and a connection it had already decided to
            // retire is not a failed request. Falling through opens a fresh
            // one under the same `attempt`.
            Sent::WhileSending(_) => {}
            Sent::Silent(why) | Sent::Failed(why) => return Ok(Attempt::Failed(why)),
        }
    }

    let (connection, socket) = match dialled(call, &parts, &approved, through)? {
        Dialled::Open { connection, at } => (connection, at),
        Dialled::Failed(why) => return Ok(Attempt::Failed(why)),
    };
    let reached = reached_at(&parts, socket);
    let key = pool_key(&parts, socket, fingerprint, &call.policy, through);

    Ok(
        match exchange(call, connection, &request, &reached, bounds, &key)? {
            Sent::Answered(reply) => Attempt::Answered(reply),
            // On a socket opened for this request, nothing arriving at all is a
            // statement about the other end rather than weather: a second identical
            // request gets the same non-reply.
            Sent::Silent(_) => return Err(malformed(call.member, "no header section ended it")),
            Sent::WhileSending(why) | Sent::Failed(why) => Attempt::Failed(why),
        },
    )
}

/// What dialling the destination left behind: a connection to speak over, or a
/// sentence saying why there is none.
enum Dialled {
    /// An open connection, plaintext or inside a TLS session, and the approved
    /// address it goes to — `None` where the destination was asked for by name
    /// because this deployment's proxy resolves it.
    Open {
        /// The connection itself, past the point where plaintext and TLS are
        /// still distinct types.
        connection: Box<dyn Connection>,
        /// Where it goes, as [`pool_key`] and [`reached_at`] want it.
        at: Option<SocketAddr>,
    },
    /// Nothing was reached, with the sentence a refusal would carry. On the
    /// same footing as any other attempt that failed, since nothing was asked
    /// of the destination and nothing can have acted on it.
    Failed(String),
}

/// One connection to `parts`: through the operator's tunnel where one covers
/// this destination, then inside TLS where the scheme asks for it.
///
/// **One implementation for both things this module opens.** A request's
/// attempt and a socket's opening handshake make the same connection under the
/// same pin, the same `connectTimeout` and the same policy, and a second copy
/// of this would be the one that comes to disagree about which address was
/// approved — which is `rule:security/one-tls-client`'s argument at the layer
/// under it. The pool is the caller's: a request files what it drew and what it
/// opened, and a socket consumes its connection and files nothing.
///
/// # Errors
///
/// A `RuntimeError` for an approved set with no address that nothing tunnels,
/// and a thrown `RuntimeError` where the TLS handshake was refused rather than
/// failed — the settled answers, which no second attempt changes. Everything
/// else comes back as [`Dialled::Failed`].
fn dialled(
    call: &Call<'_>,
    parts: &Parts,
    approved: &[SocketAddr],
    through: Option<&Proxy>,
) -> Result<Dialled, Fault> {
    // An approved set with nothing in it says *ask for this one by name*, and
    // there is one network it can be asked in. `[http.client.proxy]` is
    // `Reload`, so a `Core\Http\Target` outlives the word that laundered it: the
    // operator can put this host in `bypass`, write `resolve = "local"` or
    // remove the block, and the call arrives holding an approval of a host with
    // no address beside it. Refusing here is what keeps the emptiness from
    // being read as *every address is fine*
    // (`rule:http-server/a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise`).
    if approved.is_empty() && !through.is_some_and(|proxy| proxy.by_name) {
        return Err(Fault::thrown_as(
            ThrownClass::Runtime,
            format!(
                "{}: `{}` was approved without an address, which only a deployment whose proxy \
                 resolves the destination does, and nothing tunnels this call — there is no \
                 address to dial and no name to ask for",
                call.member, parts.host
            ),
        ));
    }
    // The handshake's own bound, clamped by what is left of the total: a
    // `connectTimeout` longer than the remaining deadline would be the one
    // spelling § 5 says does not exist, arrived at by arithmetic.
    let budget = call
        .connect_timeout
        .min(call.deadline.saturating_duration_since(Instant::now()));
    let opened = Instant::now();
    let (mut stream, socket) = match through {
        // Over the tunnel this end speaks exactly what it would have spoken to
        // that address directly — the same TLS under the same server name, or
        // plain HTTP in origin form — because the address `CONNECT` named is one
        // the door approved and no second resolution happened anywhere in the
        // path
        // (`rule:http-server/a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise`).
        Some(proxy) => match tunnel(call, proxy, parts, approved, budget)? {
            Tunnel::Open { stream, at } => (stream, at),
            Tunnel::Unreachable(why) => return Ok(Dialled::Failed(why)),
        },
        None => {
            // Unreachable from source: the one set with no address in it
            // belongs to a destination the proxy resolves, which the arm above
            // tunnels and the refusal at the head of this function ends where
            // it does not.
            let Some(&first) = approved.first() else {
                return Err(Fault::fatal(format!(
                    "{}: an approved set with no address in it",
                    call.member
                )));
            };
            let stream = match NvsTcp::connect_racing(approved, budget) {
                Ok(stream) => stream,
                // `nvs_host` names every approved address and what it answered,
                // which is the rule's *one `IOError` naming each*; what this end
                // adds is the host the set was approved for, since the addresses
                // alone do not say which call failed.
                Err(err) => {
                    return Ok(Dialled::Failed(format!(
                        "connecting to `{}` failed: {err}",
                        parts.host
                    )));
                }
            };
            // The address the walk stopped at, which is not always the set's
            // first: a connection goes back into the pool under where it
            // actually goes, and a key built before the walk was built before
            // there was an answer to build it from.
            let at = stream.peer_addr().unwrap_or(first);
            (stream, Some(at))
        }
    };
    call.span.borrow_mut().connected(socket, opened.elapsed());
    let reached = reached_at(parts, socket);
    // Before the handshake, not after it: the TLS flight waits on this socket
    // and the deadline is what bounds every wait on it.
    stream.set_deadline(Some(call.deadline));

    let connection: Box<dyn Connection> = if parts.tls {
        // One door for every call, because the two values it takes answer two
        // different questions and a call may write both: the policy is whom this
        // end believes and the identity is what it presents when the server asks
        // for a certificate. A call that wrote neither hands over a default and a
        // `None` and gets the process's own configuration
        // (`rule:security/one-tls-client`), so the refusal split below is the
        // same split for all of them.
        let began = Instant::now();
        let handshake = NvsTls::over_policy(
            stream,
            &parts.host,
            &call.policy,
            call.identity.as_ref().map(|held| &held.session),
        );
        match handshake {
            Ok(tls) => {
                call.span.borrow_mut().handshook(began.elapsed());
                Box::new(tls)
            }
            // A name or a certificate this build will not accept is settled:
            // the module doc's paragraph on `https` is why only one of these
            // two shapes is handed back for another attempt.
            Err(err) if matches!(err.kind(), ErrorKind::InvalidData | ErrorKind::InvalidInput) => {
                return Err(Fault::thrown(format!(
                    "{}: the TLS handshake with `{}` was refused and nothing was sent — {err}",
                    call.member, parts.host
                )));
            }
            Err(err) => {
                return Ok(Dialled::Failed(format!(
                    "the TLS handshake with {reached} failed: {err}"
                )));
            }
        }
    } else {
        Box::new(stream)
    };

    Ok(Dialled::Open {
        connection,
        at: socket,
    })
}

/// What an opening handshake left: the framed conversation, and the name the
/// two ends settled on.
pub(crate) struct Upgraded {
    /// `tungstenite`'s client half, over the connection the `101` arrived on.
    pub(crate) socket: WebSocket<Box<dyn Connection>>,
    /// The subprotocol the peer chose, or `None` where it chose none. A name
    /// that reaches here is one the call offered: [`settled`] is what a
    /// handshake passes through before it is this value.
    pub(crate) protocol: Option<String>,
}

/// An outbound WebSocket's opening handshake — [ADR 0183](/docs/decisions/0183.md) § 4.
///
/// **The `Call` is the value a request carries**, and its exchange half is what
/// an upgrade is: `GET`, no body, one attempt and no hop. That is not a
/// borrowed shape — the pin, the `connectTimeout`, the deadline that ends at
/// the `101`, the TLS policy, the identity and the operator's tunnel are the
/// same fields answering the same questions, and a struct of their own here
/// would be the copy that comes to be missing one.
///
/// **Nothing is pooled.** A connection that has been upgraded can carry no
/// request, so this one is neither drawn from
/// `rule:http-server/an-outbound-connection-is-pooled-per-core-and-stays-pinned`'s
/// store nor returned to it: the socket consumes it and releases it with the
/// task that opened it.
///
/// # Errors
///
/// A `TimeoutError` where the deadline is gone, an `IOError` naming what was
/// not reached — one attempt, so a dial that failed is the answer rather than a
/// retry — a `RuntimeError` for an answer that is not a `101`, naming the
/// `Location` where that answer was a redirect: a socket has no spelling for
/// following one, so a `3xx` is the end of the call
/// (`rule:http-server/redirects-are-off-and-every-hop-is-re-pinned`); and
/// [`settled`]'s refusal of a `101` that chose a name nobody offered.
pub(crate) fn upgrade(
    call: &Call<'_>,
    protocols: &[String],
    config: WebSocketConfig,
) -> Result<Upgraded, Fault> {
    let parts = parts(&call.url, call.member)?;
    call.span
        .borrow_mut()
        .at(parts.tls, &parts.host, parts.port, &parts.target);
    let approved: Vec<SocketAddr> = call
        .addresses
        .iter()
        .map(|&address| SocketAddr::new(address, parts.port))
        .collect();
    let through = call
        .proxy
        .as_ref()
        .filter(|proxy| !proxy.bypasses(&parts.host));
    if Instant::now() >= call.deadline {
        return Err(expired(call.member));
    }

    call.span.borrow_mut().attempted();
    let connection = match dialled(call, &parts, &approved, through)? {
        Dialled::Open { connection, .. } => connection,
        Dialled::Failed(why) => {
            // The same reading [`attempts`] gives a last attempt that failed,
            // and a socket's every attempt is its last: a budget already gone
            // is what ended this rather than the destination.
            if Instant::now() >= call.deadline {
                return Err(expired(call.member));
            }
            return Err(Fault::thrown_as(
                ThrownClass::Io,
                format!("{}: {why}", call.member),
            ));
        }
    };

    let (socket, answered) = match tungstenite::client::client_with_config(
        asked(call, protocols)?,
        connection,
        Some(config),
    ) {
        Ok(upgraded) => upgraded,
        // Unreachable from source: `nvs_host`'s stream parks the core
        // rather than answering `WouldBlock`, which is the whole of why
        // this codec runs over it with no adapter.
        Err(tungstenite::HandshakeError::Interrupted(_)) => {
            return Err(Fault::fatal(format!(
                "{}: the handshake asked to be resumed on a stream that parks",
                call.member
            )));
        }
        Err(tungstenite::HandshakeError::Failure(why)) => return Err(refused(call, why)),
    };

    let chosen = answered
        .headers()
        .get("sec-websocket-protocol")
        .and_then(|name| name.to_str().ok())
        .map(str::to_owned);
    // A refusal here drops the connection `socket` holds, which is the hang-up
    // a peer answering a question nobody asked has earned.
    let protocol = settled(chosen, protocols, call.member)?;
    Ok(Upgraded { socket, protocol })
}

/// The name the two ends settled on, judged against what the call offered.
///
/// A `101` choosing a subprotocol the request never sent is a peer answering a
/// question nobody asked, and a program reading `protocol()` afterwards would
/// act on it. The judgement lives here rather than at the row because both a
/// live handshake and `rule:testing/an-outbound-socket-is-answered-by-a-scripted-peer`'s
/// scripted peer pass through it, so a peer a test wrote cannot say what a real
/// one would have been refused for.
///
/// `tungstenite` asks the same question of a live `101` and gets there first,
/// refusing without naming the name. This is not that check repeated for the
/// sake of it: what it buys is that [`Upgraded::protocol`]'s invariant rests on
/// this module rather than on what a dependency happens to validate, and the
/// scripted arm — which reaches no codec at all — has the same answer to give.
///
/// # Errors
///
/// A `RuntimeError` naming the subprotocol that was never offered.
pub(crate) fn settled(
    chosen: Option<String>,
    protocols: &[String],
    member: &str,
) -> Result<Option<String>, Fault> {
    if let Some(name) = &chosen
        && !protocols.iter().any(|offer| offer == name)
    {
        return Err(Fault::thrown(format!(
            "{member}: the peer chose the subprotocol `{name}`, which this call never offered"
        )));
    }
    Ok(chosen)
}

/// The opening request, as the caller's headers and offers put it.
///
/// The `Host`, the key, the version and the two upgrade fields are
/// `tungstenite`'s: they are the handshake rather than the call, and the key in
/// particular is what the `101`'s accept is checked against, so this end never
/// writes one.
///
/// # Errors
///
/// Whatever [`field`] refuses — a header a request would not carry is not one a
/// handshake carries either — and a `RuntimeError` for a URL no request can be
/// built from, which the roster and [`parts`] have already ruled out.
fn asked(
    call: &Call<'_>,
    protocols: &[String],
) -> Result<tungstenite::handshake::client::Request, Fault> {
    use tungstenite::client::IntoClientRequest;

    let mut request = call.url.as_str().into_client_request().map_err(|why| {
        Fault::thrown(format!(
            "{}: `{}` is not a URL a handshake can be built from — {why}",
            call.member, call.url
        ))
    })?;
    // Every field goes through the judgement an outbound header line already
    // passes, so one refused at a request is refused here in the same words.
    // What it writes is thrown away: the line itself is `http`'s to write.
    let mut judged = String::new();
    for (name, value) in &call.headers {
        field(&mut judged, name, value, call.member)?;
        request.headers_mut().append(
            header_name(name, call.member)?,
            header_value(value, call.member)?,
        );
    }
    if !protocols.is_empty() {
        let offered = protocols.join(", ");
        field(&mut judged, "Sec-WebSocket-Protocol", &offered, call.member)?;
        request.headers_mut().insert(
            HeaderName::from_static("sec-websocket-protocol"),
            header_value(&offered, call.member)?,
        );
    }
    Ok(request)
}

/// A header name as `http` holds one.
///
/// # Errors
///
/// A `RuntimeError` naming the field. Unreachable after [`field`], which
/// refuses everything this does and more.
fn header_name(name: &str, member: &str) -> Result<HeaderName, Fault> {
    HeaderName::from_bytes(name.as_bytes())
        .map_err(|why| Fault::thrown(format!("{member}: `{name}` is not a header name — {why}")))
}

/// A header value as `http` holds one — [`header_name`]'s other half.
///
/// # Errors
///
/// A `RuntimeError` naming the field, for [`header_name`]'s reason.
fn header_value(value: &str, member: &str) -> Result<HeaderValue, Fault> {
    HeaderValue::from_str(value)
        .map_err(|why| Fault::thrown(format!("{member}: `{value}` is not a header value — {why}")))
}

/// What a handshake that did not end in a `101` throws.
///
/// A redirect is named as a redirect and carries its `Location`, because that
/// is the one answer a caller would otherwise read as a network failure: a
/// socket follows no hop, so a `3xx` is the end of the call rather than the
/// middle of one.
fn refused(call: &Call<'_>, why: tungstenite::Error) -> Fault {
    let tungstenite::Error::Http(answered) = why else {
        let class = if matches!(why, tungstenite::Error::Io(_)) {
            ThrownClass::Io
        } else {
            ThrownClass::Runtime
        };
        return Fault::thrown_as(
            class,
            format!("{}: the opening handshake failed — {why}", call.member),
        );
    };
    let status = answered.status();
    if status.is_redirection() {
        let location = answered
            .headers()
            .get("location")
            .and_then(|to| to.to_str().ok())
            .unwrap_or("nowhere it named");
        return Fault::thrown(format!(
            "{}: the peer answered `{status}` pointing at `{location}` — a socket follows no \
             redirect, so this is the answer rather than a hop",
            call.member
        ));
    }
    Fault::thrown(format!(
        "{}: the peer answered `{status}` rather than upgrading the connection",
        call.member
    ))
}

/// How much of a proxy's answer to a `CONNECT` is read before the answer is
/// itself the refusal: a status line and its fields, and nothing the size of a
/// body a tunnel has no room for.
const TUNNEL_HEAD: usize = 8 * 1024;

/// What asking a proxy for a tunnel left behind it.
enum Tunnel {
    /// An open tunnel, and the approved address `CONNECT` named — which is where
    /// the bytes go and what [`pool_key`] files the connection under, since the
    /// socket underneath is the proxy's and says nothing about the destination.
    Open {
        /// The stream to speak the destination's protocol over, plaintext still.
        stream: NvsTcp,
        /// The approved address the tunnel reaches, and `None` where the
        /// `CONNECT` named the destination instead because
        /// [`Proxy::by_name`] says this deployment never resolves one.
        at: Option<SocketAddr>,
    },
    /// The proxy was not reached at all. This attempt's failure, on the same
    /// footing as a destination that never answered and retried under the same
    /// rules — nothing was asked of the destination, so nothing can have acted
    /// on it.
    Unreachable(String),
}

/// A tunnel to the destination, opened through `proxy` inside `budget`.
///
/// Under `resolve = "local"` the address `CONNECT` names is one the door
/// approved and the `Host` beside it names the same, so a proxy reopens no
/// check-then-connect gap: there is no resolution of the destination in this
/// path at all, and what comes back is a stream to the address the pin already
/// answered for. Under `resolve = "proxy"` there is no such address to name —
/// the door approved the host and stopped — so one `CONNECT` carries the URL's
/// own `host:port` and the address question is the proxy's
/// (`rule:http-server/a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise`).
///
/// An approved set is walked, one `CONNECT` per address, because a proxy that
/// will not reach the first address has said nothing about the second. Each gets
/// a connection of its own: a refusal leaves a stream whose state is the proxy's
/// to decide, and reusing one would be reading the next answer off it. A
/// destination asked for by name is one request, there being one name.
///
/// # Errors
///
/// A `RuntimeError` naming proxy authentication for a `407`, and an `IOError`
/// naming each destination and what the proxy answered when every one of them
/// was refused. Neither is an attempt that failed and neither is retried,
/// because `rule:http-server/retry-is-opt-in-jittered-and-closed` retries an
/// answer from the destination and a proxy that refused the tunnel is not one.
fn tunnel(
    call: &Call<'_>,
    proxy: &Proxy,
    parts: &Parts,
    approved: &[SocketAddr],
    budget: Duration,
) -> Result<Tunnel, Fault> {
    let until = Instant::now() + budget;
    // Resolved and not judged: an endpoint an operator wrote into root-owned
    // configuration is authorized by that writing, which is the one exception
    // `rule:security/net-address-policy` states. It is also the one lookup a
    // tunnelled call makes that a direct call does not, and it is paid per
    // connection opened rather than per request.
    let endpoint = match nvs_runtime::capability::resolve_host(&proxy.host, call.member) {
        Ok(address) => SocketAddr::new(address, proxy.port),
        Err(_) => {
            return Ok(Tunnel::Unreachable(format!(
                "the proxy at `{}` resolves to no address",
                proxy.url
            )));
        }
    };
    // What each `CONNECT` asks for: every address the door approved, or the one
    // name it approved instead where this deployment's `resolve` says the proxy
    // is what resolves the destination.
    let asked_for: Vec<Option<SocketAddr>> = if proxy.by_name {
        vec![None]
    } else {
        approved.iter().copied().map(Some).collect()
    };
    let mut refused = Vec::new();
    for at in asked_for {
        let address = reached_at(parts, at);
        let mut stream = match NvsTcp::connect_timeout(
            endpoint,
            until.saturating_duration_since(Instant::now()),
        ) {
            Ok(stream) => stream,
            Err(err) => {
                return Ok(Tunnel::Unreachable(format!(
                    "connecting to the proxy at `{}` failed: {err}",
                    proxy.url
                )));
            }
        };
        // Before the request, for the reason the handshake's own bound is set
        // before it: every wait on this socket is inside what is left of the
        // connect budget, which is itself inside the call's deadline.
        stream.set_deadline(Some(until));
        let asked = match &proxy.authorization {
            Some(credential) => format!(
                "CONNECT {address} HTTP/1.1\r\nHost: {address}\r\n\
                 Proxy-Authorization: {credential}\r\n\r\n"
            ),
            None => format!("CONNECT {address} HTTP/1.1\r\nHost: {address}\r\n\r\n"),
        };
        if let Err(err) = stream
            .write_all(asked.as_bytes())
            .and_then(|()| stream.flush())
        {
            return Ok(Tunnel::Unreachable(format!(
                "asking the proxy at `{}` for a tunnel to `{address}` failed: {err}",
                proxy.url
            )));
        }
        let head = match tunnel_head(&mut stream) {
            Ok(head) => head,
            Err(why) => {
                return Ok(Tunnel::Unreachable(format!(
                    "the proxy at `{}` {why}",
                    proxy.url
                )));
            }
        };
        let end = find(&head, b"\r\n\r\n").unwrap_or(head.len());
        let (status, _) = head_of(&head[..end], call.member)?;
        if (200..300).contains(&status) {
            if head.len() > end + 4 {
                return Err(Fault::thrown_as(
                    ThrownClass::Io,
                    format!(
                        "{}: the proxy at `{}` sent octets after the head of an open tunnel, \
                         which the destination cannot have written",
                        call.member, proxy.url
                    ),
                ));
            }
            return Ok(Tunnel::Open { stream, at });
        }
        // A `407` is what the proxy says about this deployment rather than about
        // this address, so the rest of the set is not walked for a second
        // opinion it cannot give.
        if status == 407 {
            return Err(Fault::thrown(format!(
                "{}: the proxy at `{}` refused the tunnel and asked for proxy authentication \
                 (`407`)",
                call.member, proxy.url
            )));
        }
        refused.push(format!("`{address}` answered `{status}`"));
    }
    Err(Fault::thrown_as(
        ThrownClass::Io,
        format!(
            "{}: the proxy at `{}` refused a tunnel to every destination it was asked for: {}",
            call.member,
            proxy.url,
            refused.join(", ")
        ),
    ))
}

/// The head of a proxy's answer to a `CONNECT`, read off the socket without
/// reaching past it.
///
/// An open tunnel has nothing behind the blank line — a `2xx` carries no body,
/// and the destination has not been spoken to yet — so octets after it are the
/// proxy's own and would otherwise land in the session about to start over the
/// same stream. [`tunnel`] refuses on them rather than dropping them.
///
/// # Errors
///
/// The clause a refusal writes after the proxy's name: a connection closed
/// before the answer, a head longer than [`TUNNEL_HEAD`], or the socket's own
/// failure.
fn tunnel_head(stream: &mut NvsTcp) -> Result<Vec<u8>, String> {
    let mut head = Vec::new();
    let mut chunk = [0_u8; 512];
    while find(&head, b"\r\n\r\n").is_none() {
        if head.len() >= TUNNEL_HEAD {
            return Err("answered with a head longer than a tunnel's answer can be".to_owned());
        }
        match stream.read(&mut chunk) {
            Ok(0) => return Err("closed the connection before answering".to_owned()),
            Ok(read) => head.extend_from_slice(&chunk[..read]),
            Err(err) => return Err(format!("could not be read: {err}")),
        }
    }
    Ok(head)
}

/// `rule:http-server/an-outbound-connection-is-pooled-per-core-and-stays-pinned`
/// keys reuse on **what the door approved**: the address the URL was pinned to
/// and the port the bytes go to, the scheme that decides whether a session
/// wraps them, and the name the certificate had to be for. A key short of any
/// of those is a connection served to a call whose own check would have
/// refused it — a pool keyed on the URL's host, which is what most clients
/// key on, quietly undoes the pin.
///
/// **The client identity is part of it**, which is that same sentence one step
/// further in: a connection that presented a certificate is authenticated as
/// *that* client for as long as it stays open, so handing it to a call under a
/// different identity — or under none — would let one client's credential carry
/// another's request. A call that named no identity writes an empty field
/// rather than leaving the field out, so the two are two keys rather than one
/// key that is a prefix of the other.
///
/// **So is what the call relaxed**, and it is the same sentence a third time: a
/// session that skipped a name or a chain went on believing whatever the peer
/// said for as long as it stays open, so serving it to a call that verifies
/// would relax that call at a host no grant of its own was asked about
/// (`rule:security/tls-trust-is-relaxed-only-under-a-host-grant`). The strict
/// default writes an empty field, so every call that asked for nothing shares
/// one key and each distinct relaxation gets its own.
///
/// **And so is the proxy it was tunnelled through**, as the operator's own text:
/// a tunnelled connection and a direct one to the same address are two
/// connections with two sets of terms, and an operator who changes `url` and
/// reloads has said that nothing the old value made is to carry a request. A
/// call that leaves directly — no block, or a `bypass` entry naming this host —
/// writes the empty field, so the two are two keys rather than one key that is a
/// prefix of the other.
///
/// **The address field is a name where there is no address**: under
/// `resolve = "proxy"` nothing here ever learned one, so what a connection is
/// filed under is the same `host:port` its `CONNECT` asked for
/// ([`reached_at`]). The port is in it because two ports on one host are two
/// destinations, and the proxy's own `url` beside it is what keeps that key
/// away from every direct connection to the same name.
fn pool_key(
    parts: &Parts,
    at: Option<SocketAddr>,
    identity: Option<&str>,
    policy: &CallPolicy,
    through: Option<&Proxy>,
) -> String {
    let scheme = if parts.tls { "https" } else { "http" };
    format!(
        "{scheme}|{host}|{socket}|{identity}|{policy}|{proxy}",
        host = parts.host,
        socket = reached_at(parts, at),
        identity = identity.unwrap_or_default(),
        policy = policy_key(policy),
        proxy = through.map_or("", |proxy| proxy.url.as_str())
    )
}

/// Where an attempt's octets go, in the one spelling a `CONNECT` line, a pool
/// key, a trace's refusal and an `IOError` all use: the address this end
/// approved and connected to, or the destination's own `host:port` where
/// `resolve = "proxy"` means no address for it was ever learned
/// (`rule:http-server/a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise`).
///
/// One function for all four so that the text a proxy is asked for is the text
/// the connection it answers with is filed under. The port is always written,
/// since a request line has no default to fall back on, and an IPv6 literal
/// keeps the brackets [`Parts::host`] took off it — that spelling is an
/// authority, and a server name is not.
fn reached_at(parts: &Parts, at: Option<SocketAddr>) -> String {
    match at {
        Some(socket) => socket.to_string(),
        None if parts.host.contains(':') => format!("[{}]:{}", parts.host, parts.port),
        None => format!("{}:{}", parts.host, parts.port),
    }
}

/// [`pool_key`]'s last field: one line per distinct [`CallPolicy`], and the
/// empty one for the strict default.
///
/// A digest per variable-length field rather than the text itself, for the
/// reason `super::fingerprint_of` gives about a leaf certificate: a key is
/// compared on every draw and held for as long as the connection is, and a
/// whole PEM bundle inside one buys nothing that 64 octets do not. Each pin is
/// written under its own length so that no two lists flatten to one line —
/// the pins are a program's text, and a separator it can also write would be a
/// relaxation shared between two calls that asked for different ones.
fn policy_key(policy: &CallPolicy) -> String {
    if policy == &CallPolicy::default() {
        return String::new();
    }
    let pins: String = policy
        .pins
        .iter()
        .map(|pin| format!("{}:{pin}", pin.len()))
        .collect();
    format!(
        "{anchors}~{pins}~{any_name}{insecure}~{min_version}",
        anchors = super::fingerprint_of(policy.anchors.as_deref().unwrap_or_default().as_bytes()),
        pins = super::fingerprint_of(pins.as_bytes()),
        any_name = u8::from(policy.any_name),
        insecure = u8::from(policy.insecure),
        min_version = policy.min_version.as_deref().unwrap_or_default()
    )
}

/// The request out and the reply's **head** back, over whatever is already
/// connected.
///
/// It stops at the blank line and hands the connection to [`Incoming`] with
/// whatever octets came after it in the same reads. A buffered call drains that
/// reader immediately and a streamed one walks it, so `http` and `https` share
/// one copy of the framing, the ceiling and the failure split — the plaintext
/// side of a TLS session is the same `Read` and `Write` as a bare socket, which
/// is the whole reason `nvs-host` exposes it that way.
fn exchange(
    call: &Call<'_>,
    mut stream: Box<dyn Connection>,
    request: &str,
    reached: &str,
    bounds: Bounds,
    key: &str,
) -> Result<Sent, Fault> {
    if let Err(err) = stream.write_all(request.as_bytes()) {
        return Ok(Sent::WhileSending(format!(
            "sending to {reached} failed: {err}"
        )));
    }
    if let Some(body) = &call.body
        && let Some(err) = write_body(body, &mut stream, call.member)?
    {
        return Ok(Sent::WhileSending(format!(
            "sending to {reached} failed: {err}"
        )));
    }
    if let Err(err) = stream.flush() {
        return Ok(Sent::WhileSending(format!(
            "sending to {reached} failed: {err}"
        )));
    }

    let mut raw = Vec::new();
    let mut buffer = [0_u8; 8192];
    let end = loop {
        if let Some(end) = find(&raw, b"\r\n\r\n") {
            break end;
        }
        if raw.len() > HEAD_CEILING {
            return Err(Fault::thrown(format!(
                "{}: the reply's header section passed {HEAD_CEILING} bytes without ending, \
                 which is more head than an origin has to send before this call will hold one",
                call.member
            )));
        }
        match stream.read(&mut buffer) {
            // Nothing at all, which [`one`] reads against where the connection
            // came from. A head that started and then stopped is the other
            // case: the other end did answer, and what it sent is not a reply,
            // so a second identical request gets the same non-reply.
            Ok(0) if raw.is_empty() => {
                return Ok(Sent::Silent(format!(
                    "{reached} closed the connection before answering"
                )));
            }
            Ok(0) => return Err(malformed(call.member, "no header section ended it")),
            Ok(read) => {
                call.span.borrow_mut().first_octet();
                raw.extend_from_slice(&buffer[..read]);
            }
            Err(err) if err.kind() == ErrorKind::Interrupted => {}
            Err(err) => return Ok(Sent::Failed(format!("reading {reached} failed: {err}"))),
        }
    };

    let (status, headers) = head_of(&raw[..end], call.member)?;
    call.span.borrow_mut().answered(status);
    let rest = raw.split_off(end + 4);
    let frame = frame_of(&headers);
    let (idle, until) = match bounds {
        Bounds::Whole => (
            call.deadline.saturating_duration_since(Instant::now()),
            call.deadline,
        ),
        Bounds::Streamed { idle, until } => (idle, until),
    };
    let reuse = reusable(&raw, &headers, frame).then(|| Reuse {
        key: key.to_owned(),
        caps: call.pool,
    });
    // Before the connection is handed to the reader, because that is the last
    // line at which there is still a connection here to ask: from here it is
    // [`Incoming`]'s, and on a streamed reply it goes back to the pool while the
    // response a program holds is still being read.
    let tls = stream.tls().map(|session| Tls {
        session,
        verified: call.policy.verifies(),
    });
    let body = Incoming::over(stream, rest, frame, idle, until, call.member, reuse)?;
    Ok(Sent::Answered(Streamed {
        status,
        headers,
        body,
        tls,
    }))
}

/// Whether the connection this reply arrived on may carry another request.
///
/// The rule's *only after its reply was read to the end under known framing*,
/// asked before a byte of the body has been read, because all three of its
/// answers are in the head. `head` is the status line and the fields, still as
/// they arrived.
///
/// Every arm is a way for the two ends to disagree about where this reply stops
/// and the next one starts. An HTTP/1.0 reply keeps a connection only where it
/// says so and this client never asks, so one is finished with its reply; a body
/// the close itself delimits has no end short of that close; and `close` from
/// the origin is the origin saying this is its last. On any of them the
/// connection is dropped, because one whose remaining octets are unknown is one
/// that would hand the next request someone else's body.
fn reusable(head: &[u8], headers: &[(String, String)], frame: Frame) -> bool {
    if !head.starts_with(b"HTTP/1.1 ") {
        return false;
    }
    if matches!(frame, Frame::UntilClose) {
        return false;
    }
    !header(headers, "connection").is_some_and(|value| {
        value
            .split(',')
            .any(|token| token.trim().eq_ignore_ascii_case("close"))
    })
}

/// The request text — the line, the headers this module always sends, and the
/// caller's own.
///
/// # Errors
///
/// A thrown `RuntimeError` for a header name or value carrying a control byte.
/// That is header injection and it is a priority-1 refusal: a value holding
/// `\r\n` is a second request the caller did not write, and there is no
/// escaping that makes one safe, only a rejection that makes it visible.
fn compose(call: &Call<'_>, parts: &Parts) -> Result<String, Fault> {
    let mut out = format!(
        "{verb} {target} HTTP/1.1\r\nHost: {authority}\r\n",
        verb = call.verb,
        target = parts.target,
        authority = parts.authority,
    );
    out.push_str(concat!(
        "User-Agent: novis/",
        env!("CARGO_PKG_VERSION"),
        "\r\nAccept: */*\r\n"
    ));
    if let Some((name, _)) = call
        .headers
        .iter()
        .find(|(held, _)| held.eq_ignore_ascii_case("accept-encoding"))
    {
        return Err(own_offer(call.member, name));
    }
    field(&mut out, "Accept-Encoding", OFFERED, call.member)?;
    if let Some(key) = &call.idempotency_key {
        field(&mut out, "Idempotency-Key", key, call.member)?;
    }
    // `rule:http-server/a-cross-origin-redirect-drops-credentials`: a hop to
    // another origin carries none of the caller's headers, and a hop inside one
    // carries all of them. The rule names `Authorization`, `Cookie` and
    // `Proxy-Authorization` and then every header whose value was `secret`, and
    // that last clause is why the answer here is all of them: `secret` is
    // checked and erased before codegen (`rule:security/secret-qualifier`), so
    // which value carried one is not a question this process can ask at run
    // time. Holding a program's `Accept` back on a hop it asked to follow costs
    // it a negotiation; forwarding its API key to whatever host a `Location`
    // names is the exfiltration primitive the rule exists to close, and the
    // priority ordering is what decides between those two.
    let carried: &[(String, String)] = if same_origin(call, parts) {
        &call.headers
    } else {
        &[]
    };
    // `rule:observability/an-outbound-call-propagates-traceparent`. Skipped where the caller wrote its own: two `traceparent`
    // headers are what the W3C format says to treat as no header at all, so
    // sending both would end the trace here rather than continue it. Asked of
    // what this hop carries and not of what the call holds, so a hop that left
    // the caller's own behind sends ours and the trace crosses it.
    if let Some(traceparent) = &call.traceparent
        && !carried
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case("traceparent"))
    {
        field(&mut out, "traceparent", traceparent, call.member)?;
    }
    // Before the caller's own headers, so a program that wrote its own
    // `Content-Type` beside a `json` body still sends one line rather than two
    // — the last one written is the one an origin reads, and the program's is
    // the one it meant.
    if let Some(body) = &call.body {
        if let Some(content_type) = &body.content_type {
            field(&mut out, "Content-Type", content_type, call.member)?;
        }
        field(
            &mut out,
            "Content-Length",
            &body.length.to_string(),
            call.member,
        )?;
    }
    for (name, value) in carried {
        field(&mut out, name, value, call.member)?;
    }
    out.push_str("\r\n");
    Ok(out)
}

/// The body out, piece by piece, after the head.
///
/// `Ok(None)` is a body that went out whole; `Ok(Some(..))` is the **sink**
/// failing, which [`exchange`] names with its socket and hands on as transport
/// weather; an `Err` is the **file** failing, which no second attempt would
/// change. Which of the two halves failed is what decides whether this call is
/// retried at all, so they are two shapes here rather than one message.
///
/// # Errors
///
/// A thrown `RuntimeError` where a file piece cannot be rewound or read, or
/// where it holds fewer bytes than the `Content-Length` already sent — a head
/// promising more than the body delivers leaves the other end waiting, so it is
/// this end's failure and it is named here.
fn write_body(
    body: &Body,
    stream: &mut impl Write,
    member: &str,
) -> Result<Option<std::io::Error>, Fault> {
    for piece in &body.pieces {
        match piece {
            Piece::Held(octets) => {
                if let Err(err) = stream.write_all(octets) {
                    return Ok(Some(err));
                }
            }
            Piece::File { handle, length } => {
                let mut file = handle.borrow_mut();
                file.rewind().map_err(|err| {
                    Fault::thrown(format!(
                        "{member}: the file this request sends could not be rewound for this \
                         attempt — {err}"
                    ))
                })?;
                let mut left = *length;
                let mut buffer = [0_u8; BODY_CHUNK];
                while left > 0 {
                    let want = usize::try_from(left).unwrap_or(BODY_CHUNK).min(BODY_CHUNK);
                    let read = match file.read(&mut buffer[..want]) {
                        Ok(0) => {
                            return Err(Fault::thrown(format!(
                                "{member}: the file this request sends is {left} bytes shorter \
                                 than the `Content-Length` already sent, and the other end is \
                                 waiting for the rest"
                            )));
                        }
                        Ok(read) => read,
                        Err(err) if err.kind() == ErrorKind::Interrupted => continue,
                        Err(err) => {
                            return Err(Fault::thrown(format!(
                                "{member}: reading the file this request sends failed partway \
                                 through — {err}"
                            )));
                        }
                    };
                    if let Err(err) = stream.write_all(&buffer[..read]) {
                        return Ok(Some(err));
                    }
                    left -= read as u64;
                }
            }
        }
    }
    Ok(None)
}

/// One header line, refused if either half could end it early.
fn field(out: &mut String, name: &str, value: &str, member: &str) -> Result<(), Fault> {
    let unsafe_byte = |text: &str| text.bytes().any(|byte| byte < 0x20 || byte == 0x7f);
    if name.is_empty() || unsafe_byte(name) || name.contains(':') || unsafe_byte(value) {
        return Err(Fault::thrown(format!(
            "{member}: `{name}` is not a header this request can carry — a name or value holding \
             a control byte would end the line early, which is a second request the caller did \
             not write"
        )));
    }
    out.push_str(name);
    out.push_str(": ");
    out.push_str(value);
    out.push_str("\r\n");
    Ok(())
}

/// The URL, split for [`compose`] and for the connection.
///
/// # Errors
///
/// A thrown `RuntimeError` for text that is not a URL or names no host. Both
/// are unreachable for the first hop — [`super::pin`] asked the same two
/// questions before this module was reached — and both are live for a
/// `Location` an origin sent.
fn parts(url: &str, member: &str) -> Result<Parts, Fault> {
    let reference =
        UriRef::parse(url).map_err(|_| Fault::thrown(format!("{member}: `{url}` is not a URL")))?;
    let scheme = reference.scheme().map(Scheme::as_str).unwrap_or_default();
    // `wss` is `https`'s answer to this question and gets it for the same
    // reason: the roster refusal at the row is what keeps a scheme off the path
    // that is not that row's, so what reaches here is already one of the two
    // this end speaks (`rule:http-server/allow-url-pins-the-address`).
    let tls = scheme.eq_ignore_ascii_case("https") || scheme.eq_ignore_ascii_case("wss");
    let authority = reference
        .authority()
        .ok_or_else(|| Fault::thrown(format!("{member}: `{url}` names no host to connect to")))?;

    let port = authority
        .port_to_u16()
        .map_err(|_| Fault::thrown(format!("{member}: `{url}` names no TCP port number")))?
        .unwrap_or(if tls { 443 } else { 80 });

    // `Host:` carries what the URL wrote, port and all where there was one, and
    // never the userinfo — an origin routes on the name it was asked for.
    let host = Authority::host(&authority);
    // Without the brackets: `Host:` writes an IPv6 literal inside them and a
    // server name never does, so the two spellings part company here.
    let name = host
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_owned();
    let authority = match authority.port().map(|port| port.as_str()) {
        Some(written) if !written.is_empty() => format!("{host}:{written}"),
        _ => host.to_owned(),
    };

    let path = reference.path().as_str();
    let mut target = if path.is_empty() { "/" } else { path }.to_owned();
    if let Some(query) = reference.query() {
        target.push('?');
        target.push_str(query.as_str());
    }

    Ok(Parts {
        authority,
        host: name,
        port,
        target,
        tls,
    })
}

/// A `Location` resolved against the URL that sent it, since § 4's re-check is
/// of an *address* and a relative reference has none of its own.
fn resolved(base: &str, location: &str, member: &str) -> Result<String, Fault> {
    let malformed = || {
        Fault::thrown(format!(
            "{member}: the redirect to `{location}` is not a URL this request can follow"
        ))
    };
    let base = Uri::parse(base).map_err(|_| malformed())?;
    let hop = UriRef::parse(location).map_err(|_| malformed())?;
    hop.resolve_against(&base)
        .map(|absolute| absolute.to_string())
        .map_err(|_| malformed())
}

/// Whether `hop` is the origin [`Call::url`] was approved at — the scheme, the
/// host and the port all three equal.
///
/// A [`Call::url`] that will not parse answers `false`, which is the safe
/// direction here and unreachable besides: [`super::pin`] parsed it before this
/// module was reached, so that arm exists to hold an answer rather than to be
/// taken.
fn same_origin(call: &Call<'_>, hop: &Parts) -> bool {
    parts(&call.url, call.member).is_ok_and(|origin| {
        origin.tls == hop.tls
            && origin.port == hop.port
            && origin.host.eq_ignore_ascii_case(&hop.host)
    })
}

/// The `Location` of a reply that is a redirect, or `None` for one that is not.
fn redirect_of(reply: &Streamed) -> Option<String> {
    matches!(reply.status, 301 | 302 | 303 | 307 | 308)
        .then(|| header(&reply.headers, "location"))
        .flatten()
        .map(str::to_owned)
}

/// § 6's roster, and nothing else: a `400` is an answer and retrying it is a
/// load generator.
fn retryable(status: i64) -> bool {
    matches!(status, 429 | 502 | 503 | 504)
}

/// A `Retry-After` in either of RFC 9110 § 10.2.3's forms, which replaces the
/// computed backoff — `rule:http-server/retry-is-opt-in-jittered-and-closed`.
///
/// **Read on a `429` and a `503` and on nothing else**, which is narrower than
/// [`retryable`]: those two are the statuses whose meaning *is* "come back
/// later", so the field on them is the origin saying when. A `502` or a `504`
/// is a gateway reporting what happened behind it, and a `Retry-After` there
/// describes the gateway's own state rather than the request's — taking it
/// would let one failing hop hold every client for as long as it liked.
///
/// A **date already past** answers `None` rather than a zero wait, so the call
/// keeps its jittered backoff: one stale date handed to a thousand clients is
/// the synchronised burst the jitter exists to prevent, and the clock the two
/// hosts disagree by is exactly what makes such a date arrive.
///
/// A value neither form spells also leaves the backoff in place, which is the
/// safe direction rather than the fast one. The wait this returns is not
/// clamped here: [`attempts`] throws where any wait would end past the
/// deadline, so a `Retry-After` longer than the call's remaining budget ends it
/// now instead of sleeping through it
/// (`rule:http-server/one-deadline-covers-the-whole-call`).
fn retry_after(reply: &Streamed) -> Option<Duration> {
    if !matches!(reply.status, 429 | 503) {
        return None;
    }
    let value = header(&reply.headers, "retry-after")?.trim();
    if let Ok(seconds) = value.parse::<u64>() {
        return Some(Duration::from_secs(seconds));
    }
    http_date(value)?
        .duration_since(std::time::SystemTime::now())
        .ok()
        .filter(|wait| !wait.is_zero())
}

/// The instant an IMF-fixdate names — `Sun, 06 Nov 1994 08:49:37 GMT`, RFC 9110
/// § 5.6.7's preferred form and the one every origin writing a date sends.
///
/// The two obsolete forms that section still lists are not read. Each is a
/// two-digit year a reader has to guess a century for, and the guess is the
/// whole of what they add here: a header this cannot parse leaves the jittered
/// backoff in place, so the cost of not reading them is one retry at the base
/// delay rather than a wrong answer.
///
/// `None` for anything that is not that shape, including a date before the
/// epoch — a `Retry-After` in 1969 is not a wait, whatever else it is.
fn http_date(value: &str) -> Option<std::time::SystemTime> {
    /// The month names, in the order that gives each its number.
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];

    // The day name is checked for being there and not for agreeing with the
    // date: a sender that names the wrong weekday still named a day, and
    // refusing over it would spend a real wait on a field nothing reads.
    let (_day_name, rest) = value.split_once(", ")?;
    let mut fields = rest.split(' ');
    let day: i64 = fields.next()?.parse().ok()?;
    let name = fields.next()?;
    let month = i64::try_from(MONTHS.iter().position(|held| *held == name)?).ok()? + 1;
    let year: i64 = fields.next()?.parse().ok()?;
    let clock = fields.next()?;
    if fields.next()? != "GMT" || fields.next().is_some() {
        return None;
    }
    let mut parts = clock.split(':');
    let hour: u64 = parts.next()?.parse().ok()?;
    let minute: u64 = parts.next()?.parse().ok()?;
    let second: u64 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=31).contains(&day) || hour > 23 || minute > 59 || second > 60
    {
        return None;
    }
    let days = days_from_civil(year, month, day);
    let seconds = days
        .checked_mul(86_400)?
        .checked_add(i64::try_from(hour * 3_600 + minute * 60 + second).ok()?)?;
    std::time::UNIX_EPOCH.checked_add(Duration::from_secs(u64::try_from(seconds).ok()?))
}

/// Days from `1970-01-01` to a proleptic Gregorian `y-m-d`, negative before it.
///
/// Hinnant's civil-from-days inverse, which is the shape this takes because it
/// is branch-free over the leap rules rather than a table of month lengths with
/// a February correction beside it. March is treated as the year's first month,
/// so the leap day falls at the end and needs no special case at all.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let shifted = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * shifted + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// § 6's full jitter: uniform in `[0, base × 2^attempt]`.
///
/// Not optional and not configurable, for the reason the ADR gives — unjittered
/// retries from many hosts synchronise into a burst against a service that is
/// already failing, which is the failure retrying was supposed to relieve.
fn backoff(base: Duration, attempt: u32) -> Duration {
    let ceiling = base.saturating_mul(1_u32 << attempt.min(16));
    let nanos = u64::try_from(ceiling.as_nanos()).unwrap_or(u64::MAX);
    Duration::from_nanos(rand::rng().random_range(0..=nanos))
}

/// § 5's expiry, as the class that section names.
fn expired(member: &str) -> Fault {
    Fault::thrown_as(
        ThrownClass::Timeout,
        format!(
            "{member}: the deadline covering this call passed before it had an answer. It covers \
             the connection, every redirect hop, every retry attempt and every backoff between \
             them"
        ),
    )
}

/// The first value under `name`, which is already lower-cased in [`Reply`].
fn header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(held, _)| held == name)
        .map(|(_, value)| value.as_str())
}

/// A reply that is not HTTP, as every reading of one is refused.
fn malformed(member: &str, why: &str) -> Fault {
    Fault::thrown(format!(
        "{member}: the other end answered with something that is not an HTTP reply — {why}"
    ))
}

/// A streamed body that said nothing for longer than its `idle`.
fn silent(member: &str, idle: Duration) -> Fault {
    Fault::thrown_as(
        ThrownClass::Timeout,
        format!(
            "{member}: the reply sent nothing for {idle:?}, which is as long a silence as this \
             stream's `idle` allows"
        ),
    )
}

/// A streamed body still arriving past its `maxDuration`.
fn outlived(member: &str) -> Fault {
    Fault::thrown_as(
        ThrownClass::Timeout,
        format!(
            "{member}: the reply was still arriving when this stream's `maxDuration` ran out, \
             which bounds the whole body however much of it is left"
        ),
    )
}

/// A reply's header section, as a status and a header list.
fn head_of(head: &[u8], member: &str) -> Result<(i64, Vec<(String, String)>), Fault> {
    let head = std::str::from_utf8(head)
        .map_err(|_| malformed(member, "its header section is not text"))?;

    let mut lines = head.split("\r\n");
    let status_line = lines.next().unwrap_or_default();
    let mut fields = status_line.splitn(3, ' ');
    if !fields.next().unwrap_or_default().starts_with("HTTP/") {
        return Err(malformed(member, "its first line is not a status line"));
    }
    let status: i64 = fields
        .next()
        .and_then(|code| code.parse().ok())
        .ok_or_else(|| malformed(member, "its status line carries no status code"))?;

    let headers: Vec<(String, String)> = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_owned()))
        .collect();
    Ok((status, headers))
}

/// What the head says delimits the body under it.
///
/// The order is RFC 9112 § 6.3's: a `Transfer-Encoding` settles the framing and
/// a `Content-Length` beside it says nothing, which is the reading that makes
/// request smuggling a parse disagreement rather than a choice.
fn frame_of(headers: &[(String, String)]) -> Frame {
    if header(headers, "transfer-encoding")
        .is_some_and(|value| value.to_ascii_lowercase().contains("chunked"))
    {
        return Frame::Chunked;
    }
    match header(headers, "content-length").and_then(|value| value.trim().parse::<u64>().ok()) {
        Some(length) => Frame::Sized(length),
        None => Frame::UntilClose,
    }
}

/// Where `needle` starts in `haystack`.
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::{
        Call, HttpSpan, Incoming, Reply, Streamed, WebSocket, WebSocketConfig, backoff, send,
        send_streamed, upgrade,
    };
    use crate::compress::{Bound, Codec, compress_to};
    use nvs_host::reactor::{Reactor, install, run_until_idle, with_current};
    use nvs_host::scheduler::Scheduler;
    use nvs_host::tls::CallPolicy;
    use nvs_runtime::{Ctx, Fault, OutputSink, TaskRoot, Value};
    use rustls::pki_types::CertificateDer;
    use std::cell::RefCell;
    use std::io::{ErrorKind, Read, Write};
    use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
    use std::rc::Rc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex, mpsc};
    use std::time::{Duration, Instant};

    /// What an origin thread reports once it has served every reply it was
    /// given: the requests in arrival order, and how many connections carried
    /// them.
    struct Served {
        /// Each request as it arrived, whichever connection it came in on.
        asked: Vec<String>,
        /// How many connections were accepted to carry them, which is the
        /// assertion every pooling case is written on.
        connections: usize,
    }

    /// A listener on loopback that answers `replies` in order, keeping each
    /// connection open for as many requests as the client sends on it.
    ///
    /// Loopback and a `std` listener on purpose: what is under test is the
    /// transport, and the address it is handed has already been through the
    /// door. Nothing here needs a `Ctx`.
    ///
    /// It stops as soon as the last reply is out, so the count a case asserts
    /// on is the whole exchange. **A connection gets a thread of its own**,
    /// because a client that pools holds one open with nothing on it while it
    /// talks on another, and a single-threaded origin would be blocked reading
    /// the idle one — a hang, where the failure a case is written to catch is a
    /// count.
    fn origin(replies: Vec<&'static str>) -> (SocketAddr, std::thread::JoinHandle<Served>) {
        origin_raw(replies.into_iter().map(|r| r.as_bytes().to_vec()).collect())
    }

    /// [`origin`] for a reply no `&str` holds: a compressed body is octets a
    /// codec chose, and writing the cases against bytes is what keeps the one
    /// listener answering both kinds.
    fn origin_raw(replies: Vec<Vec<u8>>) -> (SocketAddr, std::thread::JoinHandle<Served>) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a loopback port");
        let at = listener.local_addr().expect("its own address");
        listener
            .set_nonblocking(true)
            .expect("an accept that does not outlive the count");
        let state = Arc::new(Mutex::new(Served {
            asked: Vec::new(),
            connections: 0,
        }));
        let mine = Arc::clone(&state);
        let served = std::thread::spawn(move || {
            // A case that never sends what it said it would ends here rather
            // than holding the run: the assertion it fails is its own.
            let ends = Instant::now() + Duration::from_secs(20);
            while asked_so_far(&state) < replies.len() && Instant::now() < ends {
                let Ok((stream, _)) = listener.accept() else {
                    std::thread::sleep(Duration::from_millis(1));
                    continue;
                };
                let answering = Arc::clone(&state);
                answering.lock().expect("the origin's count").connections += 1;
                let replies = replies.clone();
                std::thread::spawn(move || answer(stream, replies, answering));
            }
            let held = mine.lock().expect("the origin's count");
            Served {
                asked: held.asked.clone(),
                connections: held.connections,
            }
        });
        (at, served)
    }

    /// How many requests this origin has answered.
    fn asked_so_far(state: &Mutex<Served>) -> usize {
        state.lock().expect("the origin's count").asked.len()
    }

    /// One connection's worth of requests, answered from the shared queue until
    /// the client closes it or stops asking.
    ///
    /// The reply goes out under the same lock that records the request, so a
    /// case joining the moment the last one is counted has the octets that
    /// answer it already on the wire.
    fn answer(mut stream: std::net::TcpStream, replies: Vec<Vec<u8>>, state: Arc<Mutex<Served>>) {
        // Said rather than assumed: a connection accepted from a non-blocking
        // listener inherits that mode on Windows and does not on Linux, and a
        // handler that read `WouldBlock` as the end of a connection would close
        // it in the client's face.
        stream
            .set_nonblocking(false)
            .expect("a connection that waits for its request");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("a bound on this origin's own wait");
        loop {
            let mut request = [0_u8; 4096];
            let Ok(read) = stream.read(&mut request) else {
                return;
            };
            if read == 0 {
                return;
            }
            let mut held = state.lock().expect("the origin's count");
            let Some(reply) = replies.get(held.asked.len()) else {
                return;
            };
            stream.write_all(reply).expect("a reply");
            stream.flush().expect("a flushed reply");
            held.asked
                .push(String::from_utf8_lossy(&request[..read]).into_owned());
        }
    }

    /// A loopback origin that answers one opening handshake with a `101`, then
    /// holds the connection until the client lets it go.
    ///
    /// The exchange is [`handshook`]'s, which is where the reply has to be
    /// derived from the request rather than written out. `chosen` is the
    /// subprotocol the reply names, and `None` is a reply that names none.
    ///
    /// The handle carries the opening request as it arrived, which is where a
    /// case reads what the caller's headers and offers became on the wire.
    fn socket_origin(
        chosen: Option<&'static str>,
    ) -> (SocketAddr, std::thread::JoinHandle<String>) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a loopback port");
        let at = listener.local_addr().expect("its own address");
        listener
            .set_nonblocking(true)
            .expect("an accept that does not outlive the case");
        let served = std::thread::spawn(move || {
            // A case that never opens what it said it would ends here rather
            // than holding the run, for [`origin_raw`]'s reason.
            let ends = Instant::now() + Duration::from_secs(20);
            while Instant::now() < ends {
                let Ok((stream, _)) = listener.accept() else {
                    std::thread::sleep(Duration::from_millis(1));
                    continue;
                };
                return handshake(stream, chosen);
            }
            String::new()
        });
        (at, served)
    }

    /// One handshake: the request as it arrived, and the `101` that answers it.
    ///
    /// The connection is held past the reply until the client closes it. A case
    /// drops the socket it opened and then joins, so an origin that hung up on
    /// its own would race the very exchange the `101` was the start of.
    fn handshake(mut stream: std::net::TcpStream, chosen: Option<&'static str>) -> String {
        // Said rather than assumed, for [`answer`]'s reason: an accepted
        // connection inherits the listener's non-blocking mode on Windows.
        stream
            .set_nonblocking(false)
            .expect("a connection that waits for its handshake");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("a bound on this origin's own wait");
        handshook(&mut stream, chosen)
    }

    /// [`handshake`] over whatever carries the octets, which is what lets the
    /// `wss` origin answer the same exchange from inside a TLS session.
    ///
    /// What follows the `101` is read and dropped: this origin has nothing to
    /// say, and reading is what keeps this end from closing in the client's
    /// face. A peer with something to say is [`talks`].
    fn handshook(stream: &mut (impl Read + Write), chosen: Option<&'static str>) -> String {
        let asked = answered(stream, chosen);
        let mut held = [0_u8; 4096];
        while matches!(stream.read(&mut held), Ok(read) if read > 0) {}
        asked
    }

    /// The opening request as it arrived, and the `101` that answers it.
    ///
    /// The reply is derived rather than written out: `tungstenite` hashes the
    /// `Sec-WebSocket-Key` it sent and refuses an accept that is anything else,
    /// so an origin whose reply is a constant opens no socket at all. That is
    /// the whole reason this half is generic — one derivation, plaintext and
    /// TLS alike, rather than a second one behind the session.
    fn answered(stream: &mut (impl Read + Write), chosen: Option<&'static str>) -> String {
        let mut request = [0_u8; 4096];
        let Ok(read) = stream.read(&mut request) else {
            return String::new();
        };
        let asked = String::from_utf8_lossy(&request[..read]).into_owned();
        let Some(key) = header_of(&asked, "sec-websocket-key") else {
            return asked;
        };
        let accept = tungstenite::handshake::derive_accept_key(key.as_bytes());
        let names = chosen.map_or_else(String::new, |name| {
            format!("Sec-WebSocket-Protocol: {name}\r\n")
        });
        let reply = format!(
            "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
             Sec-WebSocket-Accept: {accept}\r\n{names}\r\n"
        );
        stream.write_all(reply.as_bytes()).expect("a reply");
        stream.flush().expect("a flushed reply");
        asked
    }

    /// What a talking peer does next, in the order a case wrote it.
    ///
    /// [`socket_origin`] answers a handshake and then says nothing, which is
    /// every case about the opening. A case about the conversation needs the
    /// other half: a peer that sends, one that goes quiet without going away,
    /// and one that sends more than the other end agreed to hold.
    pub(crate) enum Say {
        /// One text message.
        Text(&'static str),
        /// One binary message of this many octets — how a case writes a message
        /// past a cap without carrying one that size in its own source.
        Bytes(usize),
        /// Nothing at all for this long, answering the other end's pings while
        /// it holds. A peer with nothing to say has not gone away, and the pong
        /// is the only thing that tells the two apart.
        Quiet(Duration),
    }

    /// What a talking peer heard before the conversation ended.
    pub(crate) struct Heard {
        /// The close frame the client sent, as its code and its reason, or
        /// `None` where the connection ended without one.
        pub(crate) closed: Option<(u16, String)>,
        /// Every text and binary message the client sent, oldest first. The
        /// peer reads only while it holds a silence or waits for the close, so
        /// a case that asserts this sends before the script ends.
        pub(crate) said: Vec<tungstenite::Message>,
    }

    /// A loopback origin that answers one opening handshake and then holds up
    /// `script`'s end of the conversation.
    ///
    /// The reply is [`answered`]'s, as [`socket_origin`]'s is. What this adds is
    /// the frames after it, written by `tungstenite`'s server half so that a
    /// case says what the peer *said* rather than which octets carry it, and a
    /// handle that carries how the client ended things.
    pub(crate) fn talking_origin(
        chosen: Option<&'static str>,
        script: Vec<Say>,
    ) -> (SocketAddr, std::thread::JoinHandle<Heard>) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a loopback port");
        let at = listener.local_addr().expect("its own address");
        listener
            .set_nonblocking(true)
            .expect("an accept that does not outlive the case");
        let served = std::thread::spawn(move || {
            // A case that never opens what it said it would ends here rather
            // than holding the run, for [`origin_raw`]'s reason.
            let ends = Instant::now() + Duration::from_secs(20);
            while Instant::now() < ends {
                let Ok((stream, _)) = listener.accept() else {
                    std::thread::sleep(Duration::from_millis(1));
                    continue;
                };
                return talks(stream, chosen, script);
            }
            Heard {
                closed: None,
                said: Vec::new(),
            }
        });
        (at, served)
    }

    /// One conversation: the handshake, then the script, then whatever it takes
    /// to hear the client out.
    ///
    /// **Every frame is allowed to fail rather than asserted.** A case that
    /// bounds a conversation ends it while the peer still has script left, and a
    /// peer that insisted would fail the case from a thread where the assertion
    /// that explains it cannot run.
    fn talks(
        mut stream: std::net::TcpStream,
        chosen: Option<&'static str>,
        script: Vec<Say>,
    ) -> Heard {
        // Said rather than assumed, for [`answer`]'s reason: an accepted
        // connection inherits the listener's non-blocking mode on Windows.
        stream
            .set_nonblocking(false)
            .expect("a connection that waits for its handshake");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("a bound on this origin's own wait");
        answered(&mut stream, chosen);
        let mut peer =
            WebSocket::from_raw_socket(stream, tungstenite::protocol::Role::Server, None);
        let mut heard = Heard {
            closed: None,
            said: Vec::new(),
        };
        for say in script {
            let carried = match say {
                Say::Text(text) => peer.send(tungstenite::Message::Text(text.into())).is_ok(),
                Say::Bytes(octets) => peer
                    .send(tungstenite::Message::Binary(vec![b'.'; octets].into()))
                    .is_ok(),
                Say::Quiet(how_long) => listens(&mut peer, Instant::now() + how_long, &mut heard),
            };
            if !carried {
                break;
            }
        }
        // The script is over and the case's assertion is about how the client
        // ends things, so this end waits for that rather than hanging up first.
        listens(
            &mut peer,
            Instant::now() + Duration::from_secs(5),
            &mut heard,
        );
        heard
    }

    /// Says nothing until `ends`, answering the client's pings meanwhile, and
    /// reports whether the conversation survived that long.
    ///
    /// **Reading is what makes this a live peer.** `tungstenite` answers a ping
    /// with a pong as it reads, so a silence held by sleeping would be a peer
    /// that had stopped answering the protocol rather than one with nothing to
    /// say — and those are the two cases a bound on silence has to tell apart. A
    /// read that times out is the silence doing its job; anything else is the
    /// conversation ending, and a close is recorded where the case reads it, as
    /// is every text and binary message read on the way.
    fn listens(
        peer: &mut WebSocket<std::net::TcpStream>,
        ends: Instant,
        heard: &mut Heard,
    ) -> bool {
        let held = loop {
            let now = Instant::now();
            if now >= ends {
                break true;
            }
            peer.get_mut()
                // Never zero: a `TcpStream` reads a zero timeout as no bound at
                // all, which is the one wait this origin must not take.
                .set_read_timeout(Some((ends - now).max(Duration::from_millis(1))))
                .expect("a bound on this origin's own wait");
            match peer.read() {
                Ok(tungstenite::Message::Close(frame)) => {
                    heard.closed =
                        frame.map(|end| (u16::from(end.code), end.reason.as_str().to_owned()));
                    break false;
                }
                Ok(message @ (tungstenite::Message::Text(_) | tungstenite::Message::Binary(_))) => {
                    heard.said.push(message);
                }
                Ok(_) => {}
                Err(tungstenite::Error::Io(why))
                    if matches!(
                        why.kind(),
                        ErrorKind::TimedOut | ErrorKind::WouldBlock | ErrorKind::Interrupted
                    ) => {}
                Err(_) => break false,
            }
        };
        peer.get_mut()
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("a bound on this origin's own wait");
        held
    }

    /// One header's value out of a request as it arrived, found however the
    /// sender cased the name — which is the only way to ask, since a name
    /// `http` held is lower case by the time it reaches the wire.
    fn header_of(request: &str, name: &str) -> Option<String> {
        request.lines().find_map(|line| {
            let (field, value) = line.split_once(':')?;
            field
                .trim()
                .eq_ignore_ascii_case(name)
                .then(|| value.trim().to_owned())
        })
    }

    /// A call to `at` with one attempt, no redirects and a generous deadline.
    pub(crate) fn call<'a>(at: SocketAddr, member: &'a str) -> Call<'a> {
        Call {
            member,
            verb: "GET",
            url: format!("http://{at}/ok"),
            addresses: vec![at.ip()],
            deadline: Instant::now() + Duration::from_secs(10),
            connect_timeout: Duration::from_secs(5),
            idle: Duration::from_secs(30),
            max_duration: Duration::from_secs(300),
            headers: Vec::new(),
            redirects: 0,
            attempts: 1,
            backoff: Duration::from_millis(1),
            idempotency_key: None,
            body: None,
            pool: super::pool::Caps {
                idle: 16,
                timeout: Duration::from_secs(30),
            },
            // `[limits]`'s shipped ceiling, which only the bomb below lowers.
            compress: Bound {
                bytes: 64 << 20,
                ratio: 1000,
            },
            identity: None,
            policy: CallPolicy::default(),
            traceparent: None,
            span: RefCell::new(HttpSpan::opened("GET", Duration::ZERO)),
            proxy: None,
        }
    }

    /// The `[http.client.proxy]` block a case carries: the loopback proxy at
    /// `at`, with nothing bypassed and this end resolving the destination.
    fn through(at: SocketAddr) -> super::Proxy {
        super::Proxy {
            url: format!("http://{at}"),
            host: at.ip().to_string(),
            port: at.port(),
            by_name: false,
            bypass: Vec::new(),
            authorization: None,
        }
    }

    /// A loopback proxy that answers a `CONNECT` by opening the connection it
    /// was asked for and pumping octets both ways, so what reaches the origin is
    /// exactly what the client wrote over the tunnel — a TLS flight included,
    /// which is what makes the handshake in these cases the client's own rather
    /// than one a test stood in for.
    ///
    /// It answers `refusal`'s status line instead, where a case gives one, and
    /// opens nothing. The counter is live and the handle carries every `CONNECT`
    /// head in arrival order: a case asserting that nothing was retried reads
    /// the counter without joining, for [`tls_origin`]'s reason.
    fn proxy(
        expects: usize,
        refusal: Option<&'static str>,
    ) -> (
        SocketAddr,
        Arc<AtomicUsize>,
        std::thread::JoinHandle<Vec<String>>,
    ) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a loopback port");
        let at = listener.local_addr().expect("its own address");
        listener
            .set_nonblocking(true)
            .expect("an accept that does not outlive the count");
        let asked = Arc::new(Mutex::new(Vec::new()));
        let opened = Arc::new(AtomicUsize::new(0));
        let (mine, counting) = (Arc::clone(&asked), Arc::clone(&opened));
        let thread = std::thread::spawn(move || {
            let ends = Instant::now() + Duration::from_secs(20);
            while asked.lock().expect("the proxy's record").len() < expects && Instant::now() < ends
            {
                let Ok((stream, _)) = listener.accept() else {
                    std::thread::sleep(Duration::from_millis(1));
                    continue;
                };
                counting.fetch_add(1, Ordering::Relaxed);
                let recording = Arc::clone(&asked);
                std::thread::spawn(move || tunnelled(stream, refusal, &recording));
            }
            let held = mine.lock().expect("the proxy's record");
            held.clone()
        });
        (at, opened, thread)
    }

    /// One connection's `CONNECT` and whatever the tunnel behind it carries.
    ///
    /// A destination it cannot reach itself is answered `502`, which is what a
    /// proxy does and what makes an approved set's walk visible from this side:
    /// a status is what sends the client on to the next address, where a closed
    /// connection would end the attempt.
    ///
    /// Every failure ends this thread quietly rather than panicking it, for
    /// [`answer_tls`]'s reason: a client that walks away from a refused tunnel
    /// is what two of these cases are written to see, and each claim they make
    /// is made on the client's side.
    fn tunnelled(
        mut stream: std::net::TcpStream,
        refusal: Option<&'static str>,
        state: &Mutex<Vec<String>>,
    ) {
        drop(stream.set_nonblocking(false));
        drop(stream.set_read_timeout(Some(Duration::from_secs(5))));
        let mut head = Vec::new();
        let mut chunk = [0_u8; 512];
        while super::find(&head, b"\r\n\r\n").is_none() {
            let Ok(read) = stream.read(&mut chunk) else {
                return;
            };
            if read == 0 {
                return;
            }
            head.extend_from_slice(&chunk[..read]);
        }
        let asked = String::from_utf8_lossy(&head).into_owned();
        let target = asked.split(' ').nth(1).unwrap_or_default().to_owned();
        state.lock().expect("the proxy's record").push(asked);
        if let Some(refusal) = refusal {
            drop(stream.write_all(refusal.as_bytes()));
            drop(stream.flush());
            return;
        }
        let Ok(origin) = std::net::TcpStream::connect(target.as_str()) else {
            drop(stream.write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\n\r\n"));
            drop(stream.flush());
            return;
        };
        if stream
            .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
            .is_err()
        {
            return;
        }
        drop(stream.flush());
        let mut from_client = stream.try_clone().expect("the client's half");
        let mut to_origin = origin.try_clone().expect("the origin's half");
        std::thread::spawn(move || drop(std::io::copy(&mut from_client, &mut to_origin)));
        let (mut from_origin, mut to_client) = (origin, stream);
        drop(std::io::copy(&mut from_origin, &mut to_client));
    }

    /// The keystone: a proxied `https` call is tunnelled to **the address the
    /// door approved** and speaks TLS over it under the name the URL was
    /// approved for, so the pin survives the proxy
    /// (`rule:http-server/a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise`).
    ///
    /// Both halves are asserted on the far side of the seam rather than on this
    /// one: the `CONNECT` line is the proxy's own record of what it was asked
    /// for, and the handshake completing at all is what says the client verified
    /// a certificate issued for `localhost` — a client that had sent the pinned
    /// address as its server name would have been refused by its own anchors.
    #[test]
    fn connect_tunnel_is_opened_to_the_pinned_address_and_tls_names_the_host() {
        let (cert, key) = trusted();
        let (port, accepted) = tls_origin(
            cert.clone(),
            key.clone(),
            "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok",
        );
        let (proxying, _, asked) = proxy(1, None);

        let mut asking = call(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port),
            "test",
        );
        asking.url = format!("https://localhost:{port}/ok");
        asking.proxy = Some(through(proxying));
        let reply = send(&asking, &mut never).expect("the answer from behind the tunnel");

        assert_eq!(reply.status, 200);
        assert_eq!(reply.body, b"ok");
        let asked = asked.join().expect("the proxy thread");
        assert_eq!(
            asked.len(),
            1,
            "one call through a proxy is one `CONNECT`: {asked:?}"
        );
        assert!(
            asked[0].starts_with(&format!("CONNECT 127.0.0.1:{port} HTTP/1.1\r\n")),
            "the tunnel was asked for the pinned address and not for the name: {asked:?}"
        );
        assert!(
            asked[0].contains(&format!("Host: 127.0.0.1:{port}\r\n")),
            "`Host` names the same address the request line does: {asked:?}"
        );
        assert_eq!(
            accepted.load(Ordering::Relaxed),
            1,
            "the origin was reached once, through the tunnel"
        );
    }

    /// Every destination is tunnelled, an `http` one included, so there is one
    /// mechanism and no path on which the proxy is handed a whole request in
    /// absolute form (`rule:http-server/an-outbound-proxy-is-operator-configured`).
    ///
    /// The origin's own copy of the request is what the second half is asserted
    /// against: absolute form would be a request line this client never writes,
    /// and the only way to say so is to read what actually arrived.
    #[test]
    fn a_plain_http_url_is_tunnelled_and_never_sent_in_absolute_form() {
        let (at, served) = origin(vec!["HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok"]);
        let (proxying, _, asked) = proxy(1, None);

        let mut asking = call(at, "test");
        asking.proxy = Some(through(proxying));
        let reply = send(&asking, &mut never).expect("the answer from behind the tunnel");

        assert_eq!(reply.status, 200);
        let asked = asked.join().expect("the proxy thread");
        assert!(
            asked[0].starts_with(&format!("CONNECT {at} HTTP/1.1\r\n")),
            "a plain `http` destination is tunnelled like any other: {asked:?}"
        );
        let behind = served.join().expect("the origin thread");
        assert!(
            behind.asked[0].starts_with("GET /ok HTTP/1.1\r\n"),
            "the request over the tunnel is in origin form: {:?}",
            behind.asked
        );
        assert!(
            !behind.asked[0].contains("http://"),
            "nothing on this path writes an absolute-form request line: {:?}",
            behind.asked
        );
    }

    /// Under `resolve = "proxy"` the `CONNECT` carries the destination's own
    /// name: the door approved the host and never learned an address for it, so
    /// [`Call::addresses`] arrives empty and there is nothing else to ask for
    /// (`rule:http-server/a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise`).
    ///
    /// The proxy's own record of the request line is what says so, and the
    /// answer coming back is what says the tunnel it opened was the right one.
    #[test]
    fn a_destination_the_proxy_resolves_is_asked_for_by_name() {
        let (at, served) = origin(vec!["HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok"]);
        let (proxying, _, asked) = proxy(1, None);

        let mut asking = call(at, "test");
        asking.url = format!("http://localhost:{}/ok", at.port());
        asking.addresses = Vec::new();
        let mut through = through(proxying);
        through.by_name = true;
        asking.proxy = Some(through);
        let reply = send(&asking, &mut never).expect("the answer from behind the tunnel");

        assert_eq!(reply.status, 200);
        let asked = asked.join().expect("the proxy thread");
        assert!(
            asked[0].starts_with(&format!("CONNECT localhost:{} HTTP/1.1\r\n", at.port())),
            "the tunnel is asked for the name, there being no address to ask for: {asked:?}"
        );
        assert!(
            asked[0].contains(&format!("Host: localhost:{}\r\n", at.port())),
            "`Host` names the same destination the request line does: {asked:?}"
        );
        let behind = served.join().expect("the origin thread");
        assert!(
            behind.asked[0].starts_with("GET /ok HTTP/1.1\r\n"),
            "and what arrives over the tunnel is the request itself: {:?}",
            behind.asked
        );
    }

    /// A host the operator listed in `bypass` never reaches the proxy, and the
    /// address it is dialled at is one the door pinned — the bypass is what
    /// puts a destination back under the whole of
    /// `rule:security/net-address-policy` in a deployment that otherwise hands
    /// that question to its proxy.
    ///
    /// Both halves are read where each is decided: [`Proxy::resolves`] is the
    /// question the door asks before it looks a name up, and the proxy's
    /// connection counter is what says the octets went straight to the origin.
    #[test]
    fn a_bypassed_host_connects_directly_under_the_full_policy() {
        let (at, served) = origin(vec!["HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok"]);
        let (proxying, opened, _) = proxy(1, None);

        let mut through = through(proxying);
        through.by_name = true;
        through.bypass = vec!["localhost".to_owned()];
        assert!(
            !through.resolves("localhost"),
            "a bypassed host is this end's to resolve, so it is pinned like any other"
        );
        assert!(
            through.resolves("api.example.invalid"),
            "and every other destination under the same block is the proxy's"
        );

        let mut asking = call(at, "test");
        asking.url = format!("http://localhost:{}/ok", at.port());
        asking.proxy = Some(through);
        let reply = send(&asking, &mut never).expect("the answer from the origin itself");

        assert_eq!(reply.status, 200);
        assert_eq!(
            opened.load(Ordering::Relaxed),
            0,
            "a bypassed destination is dialled directly and the proxy is never asked"
        );
        let behind = served.join().expect("the origin thread");
        assert_eq!(
            behind.connections, 1,
            "the origin answered the call itself: {:?}",
            behind.asked
        );
    }

    /// The credential goes to the proxy and stops there: it is on the `CONNECT`
    /// request and on nothing behind it, which is the whole of what
    /// `rule:http-server/an-outbound-proxy-is-operator-configured` says about
    /// where a `Proxy-Authorization` may appear.
    ///
    /// Both halves are read off the wire — the proxy's own copy of the request
    /// it was asked, and the origin's copy of what arrived over the tunnel —
    /// because the failure this is written to catch is a header carried further
    /// than it was meant to go, which only the far ends can see.
    #[test]
    fn proxy_authorization_is_sent_on_connect_and_never_to_the_destination() {
        use base64::Engine as _;

        let (at, served) = origin(vec!["HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok"]);
        let (proxying, _, asked) = proxy(1, None);

        let mut asking = call(at, "test");
        let mut through = through(proxying);
        // What `super::super::proxy_of` builds out of `username` and the
        // materialized password, spelled here so the case is about where the
        // value goes rather than about how it is encoded.
        through.authorization = Some(format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode("operator:must-not-be-forwarded")
        ));
        asking.proxy = Some(through);
        let reply = send(&asking, &mut never).expect("the answer from behind the tunnel");

        assert_eq!(reply.status, 200);
        let asked = asked.join().expect("the proxy thread");
        assert!(
            asked[0]
                .contains("Proxy-Authorization: Basic b3BlcmF0b3I6bXVzdC1ub3QtYmUtZm9yd2FyZGVk"),
            "the credential is on the `CONNECT` request: {asked:?}"
        );
        let behind = served.join().expect("the origin thread");
        assert!(
            !behind.asked[0]
                .to_ascii_lowercase()
                .contains("proxy-authorization"),
            "the destination is never sent the proxy's credential: {:?}",
            behind.asked
        );
        assert!(
            !behind.asked[0].contains("b3BlcmF0b3I6bXVzdC1ub3QtYmUtZm9yd2FyZGVk"),
            "nor the value under another field name: {:?}",
            behind.asked
        );
    }

    /// A destination approved without an address is reachable through a proxy
    /// that resolves it and nowhere else, and `[http.client.proxy]` is `Reload`:
    /// the operator can bypass the host, write `resolve = "local"` or drop the
    /// block while a `Core\Http\Target` laundered under the old one is still in
    /// a program's hand
    /// (`rule:http-server/a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise`).
    ///
    /// Both ways in, because they reach the refusal from opposite sides: with no
    /// proxy at all there is nothing to tunnel through, and with one that
    /// bypasses this host the call was sent back under the full address policy —
    /// which is the policy this set was never checked against.
    #[test]
    fn a_destination_with_no_address_is_refused_where_nothing_tunnels_it() {
        let nowhere: SocketAddr = "192.0.2.1:443".parse().expect("a literal address");
        let mut asking = call(nowhere, "test");
        asking.url = "https://origin.example/ok".to_owned();
        asking.addresses = Vec::new();

        let refused = send(&asking, &mut never).expect_err("nothing here can reach that host");
        assert!(
            matches!(refused, Fault::Thrown(nvs_runtime::ThrownClass::Runtime, _)),
            "a deployment that no longer proxies this host refuses the call: {refused:?}"
        );
        assert!(
            format!("{refused:?}").contains("origin.example"),
            "the refusal names the destination it has nothing to dial for: {refused:?}"
        );

        let mut bypassing = through(nowhere);
        bypassing.by_name = true;
        bypassing.bypass = vec!["origin.example".to_owned()];
        asking.proxy = Some(bypassing);
        let directly = send(&asking, &mut never)
            .expect_err("a bypassed host is dialled directly, and this one has no address");
        assert!(
            matches!(
                directly,
                Fault::Thrown(nvs_runtime::ThrownClass::Runtime, _)
            ),
            "a bypassed destination with no address is the same refusal: {directly:?}"
        );
    }

    /// A proxy that will not open the tunnel is two failures and they are two
    /// classes: a `407` is a `RuntimeError` naming proxy authentication, because
    /// what is missing is a credential the operator writes, and every other
    /// refusal is an `IOError` naming the address and what came back.
    #[test]
    fn a_proxy_refusing_connect_is_an_io_error_and_407_names_authentication() {
        let nowhere: SocketAddr = "192.0.2.1:443".parse().expect("a literal address");
        let (forbidding, _, _) = proxy(
            1,
            Some("HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n"),
        );
        let (challenging, _, _) = proxy(
            1,
            Some("HTTP/1.1 407 Proxy Authentication Required\r\nContent-Length: 0\r\n\r\n"),
        );

        let mut asking = call(nowhere, "test");
        asking.url = "https://origin.example/ok".to_owned();
        asking.addresses = vec![nowhere.ip()];
        asking.proxy = Some(through(forbidding));
        let refused = send(&asking, &mut never).expect_err("a refused tunnel is not an answer");
        assert!(
            matches!(refused, Fault::Thrown(nvs_runtime::ThrownClass::Io, _)),
            "a refused `CONNECT` is an `IOError`: {refused:?}"
        );
        let said = format!("{refused:?}");
        assert!(
            said.contains("192.0.2.1:443") && said.contains("403"),
            "the refusal names the address it was asked for and what came back: {said}"
        );

        asking.proxy = Some(through(challenging));
        let challenged =
            send(&asking, &mut never).expect_err("a challenged tunnel is not an answer");
        assert!(
            matches!(
                challenged,
                Fault::Thrown(nvs_runtime::ThrownClass::Runtime, _)
            ),
            "a `407` is a `RuntimeError`: {challenged:?}"
        );
        let said = format!("{challenged:?}");
        assert!(
            said.contains("proxy authentication") && said.contains("407"),
            "the refusal says what is missing: {said}"
        );
    }

    /// A proxy that will not reach the first address the door approved has said
    /// nothing about the second, so an approved set is walked one `CONNECT` at a
    /// time and the call is answered over the tunnel the second one opened
    /// (`rule:http-server/an-outbound-call-tries-every-approved-address`).
    ///
    /// Each address is asked for over a connection of its own, which is what the
    /// proxy's count says: a refusal leaves a stream whose state is the proxy's
    /// to decide, and asking again over that one would be reading the next
    /// answer off it.
    #[test]
    fn a_connect_refused_for_one_address_is_asked_again_for_the_next() {
        let (at, served) = origin(vec!["HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok"]);
        let (proxying, opened, asked) = proxy(2, None);

        let mut asking = call(at, "test");
        // Loopback rather than a `TEST-NET` literal, because the proxy is what
        // dials this one: nothing listens on it, so what comes back is the
        // refusal above and not a connect the case waits out.
        asking.addresses = vec!["127.0.0.2".parse().expect("a literal address"), at.ip()];
        asking.proxy = Some(through(proxying));

        let reply = send(&asking, &mut never).expect("the answer over the second tunnel");
        assert_eq!(reply.status, 200);
        assert_eq!(reply.body, b"ok");

        let asked = asked.join().expect("the proxy thread");
        assert_eq!(
            asked.len(),
            2,
            "the approved set is walked one `CONNECT` at a time: {asked:?}"
        );
        assert!(
            asked[0].starts_with(&format!("CONNECT 127.0.0.2:{} HTTP/1.1\r\n", at.port())),
            "the first `CONNECT` asks for the first approved address: {asked:?}"
        );
        assert!(
            asked[1].starts_with(&format!("CONNECT {at} HTTP/1.1\r\n")),
            "the one after a refusal asks for the address after it: {asked:?}"
        );
        assert_eq!(
            opened.load(Ordering::Relaxed),
            2,
            "each address is asked for over a connection of its own"
        );
        let behind = served.join().expect("the origin thread");
        assert_eq!(
            behind.connections, 1,
            "the origin was reached once, over the tunnel that opened"
        );
    }

    /// `retryAttempts` retries an answer from the destination, and a proxy that
    /// refused the tunnel is not one — so a `503` from the proxy, which is a
    /// status this client would retry if the origin had sent it, is spent once
    /// (`rule:http-server/retry-is-opt-in-jittered-and-closed`).
    ///
    /// The pause is what makes the count final: a retry would be out on the wire
    /// inside this call's millisecond of backoff.
    #[test]
    fn a_refused_connect_is_not_retried_as_an_answer() {
        let nowhere: SocketAddr = "192.0.2.1:443".parse().expect("a literal address");
        let (refusing, opened, _) = proxy(
            4,
            Some("HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\n\r\n"),
        );

        let mut asking = call(nowhere, "test");
        asking.url = "https://origin.example/ok".to_owned();
        asking.addresses = vec![nowhere.ip()];
        asking.attempts = 3;
        asking.proxy = Some(through(refusing));
        send(&asking, &mut never).expect_err("a refused tunnel is not an answer");

        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(
            opened.load(Ordering::Relaxed),
            1,
            "a refusal of the tunnel is settled, so the three attempts were not spent on it"
        );
    }

    /// A tunnelled connection and a direct one to the same approved address are
    /// two keys, so neither is ever served to a call the other's terms do not
    /// cover — the pool's half of
    /// `rule:http-server/an-outbound-proxy-is-operator-configured`.
    ///
    /// The origin's count is what says it: a key short of the proxy would have
    /// handed the second call the connection the first left, and the proxy would
    /// have been reached no times at all.
    #[test]
    fn a_tunnelled_connection_is_pooled_apart_from_a_direct_one() {
        let ok = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok";
        let (at, served) = origin(vec![ok, ok]);
        let (proxying, opened, _) = proxy(1, None);

        send(&call(at, "test"), &mut never).expect("the direct answer");
        let mut tunnelled = call(at, "test");
        tunnelled.proxy = Some(through(proxying));
        send(&tunnelled, &mut never).expect("the tunnelled answer");

        assert_eq!(
            opened.load(Ordering::Relaxed),
            1,
            "the tunnelled call opened its own tunnel rather than drawing the direct connection"
        );
        assert_eq!(
            served.join().expect("the origin thread").connections,
            2,
            "the origin was reached twice, once directly and once through the tunnel"
        );
    }

    /// A reply carrying `frame` under `coding`, framed by length the way an
    /// origin that compressed its body writes one.
    fn encoded(coding: &str, frame: &[u8]) -> Vec<u8> {
        let mut reply = format!(
            "HTTP/1.1 200 OK\r\nContent-Encoding: {coding}\r\nContent-Length: {}\r\n\r\n",
            frame.len()
        )
        .into_bytes();
        reply.extend_from_slice(frame);
        reply
    }

    /// What a redirect hop must never be asked for.
    fn never(_url: &str, _downgrade: bool) -> Result<Vec<IpAddr>, Fault> {
        panic!("a call with no redirect hop must not re-pin")
    }

    /// A body a case wrote out in full, so [`Incoming`] can be driven over one
    /// without a socket: the cursor is empty, so the first read is the close
    /// that ends an unframed body.
    impl super::Connection for std::io::Cursor<Vec<u8>> {
        fn bound_by(&mut self, _at: Option<Instant>) {}

        fn tls(&self) -> Option<nvs_host::tls::Session> {
            None
        }
    }

    /// A whole reply read the way a socket delivers one — the head off the
    /// front, the rest through the framing — as the answer and its drained
    /// body.
    fn parsed(raw: &[u8]) -> Result<(Streamed, Vec<u8>), Fault> {
        let end = super::find(raw, b"\r\n\r\n").expect("a head this case wrote");
        let (status, headers) = super::head_of(&raw[..end], "test")?;
        let mut body = Incoming::over(
            Box::new(std::io::Cursor::new(Vec::new())),
            raw[end + 4..].to_vec(),
            super::frame_of(&headers),
            Duration::from_secs(1),
            Instant::now() + Duration::from_secs(10),
            "test",
            None,
        )?;
        let octets = body.whole(super::REPLY_CEILING)?;
        Ok((
            Streamed {
                status,
                headers,
                body,
                tls: None,
            },
            octets,
        ))
    }

    /// The two slots a `Core\Http\Response` holds are what a real exchange
    /// fills: the status line's code, and the body the framing said was there.
    #[test]
    fn a_reply_becomes_the_status_and_the_body() {
        let (at, served) = origin(vec!["HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok"]);
        let reply = send(&call(at, "test"), &mut never).expect("an answer");
        assert_eq!(reply.status, 200);
        assert_eq!(reply.body, b"ok");

        let asked = served.join().expect("the origin thread").asked;
        assert!(asked[0].starts_with("GET /ok HTTP/1.1\r\n"), "{}", asked[0]);
        assert!(
            asked[0].contains(&format!("Host: {at}\r\n")),
            "the authority the URL wrote is what `Host:` carries: {}",
            asked[0]
        );
    }

    /// `rule:observability/an-outbound-call-propagates-traceparent`: an outbound call propagates `traceparent`, which is what
    /// makes a trace cross a service boundary at all.
    ///
    /// Asserted on the head that crossed the socket rather than on
    /// [`compose`]'s return, and by **counting** the lines rather than reading
    /// one off: a second `traceparent` is what the W3C format tells a receiver
    /// to read as no header at all, so a rule that emitted one beside the
    /// caller's own would end the trace here while still looking right on the
    /// line that asserts ours was sent.
    #[test]
    fn an_outbound_request_carries_traceparent() {
        let ours = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
        let theirs = "00-11111111111111111111111111111111-2222222222222222-00";
        let empty = "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n";
        let (at, served) = origin(vec![empty, empty, empty]);

        let mut propagating = call(at, "test");
        propagating.traceparent = Some(ours.to_owned());
        send(&propagating, &mut never).expect("an answer");

        // `[trace] propagate = false` reaches this module as nothing to send.
        let quiet = call(at, "test");
        send(&quiet, &mut never).expect("an answer");

        // A caller that wrote its own keeps it, and gets exactly one.
        let mut written = call(at, "test");
        written.traceparent = Some(ours.to_owned());
        written.headers = vec![("traceparent".to_owned(), theirs.to_owned())];
        send(&written, &mut never).expect("an answer");

        let served = served.join().expect("the origin thread");
        assert_eq!(
            served.connections, 1,
            "three calls to one key are three requests on one connection"
        );
        let asked = served.asked;
        let carried = |head: &String| {
            head.lines()
                .filter(|line| line.to_ascii_lowercase().starts_with("traceparent:"))
                .map(str::to_owned)
                .collect::<Vec<_>>()
        };
        assert_eq!(carried(&asked[0]), [format!("traceparent: {ours}")]);
        assert_eq!(carried(&asked[1]), [] as [String; 0], "{}", asked[1]);
        assert_eq!(carried(&asked[2]), [format!("traceparent: {theirs}")]);
    }

    /// `rule:http-server/redirects-are-off-and-every-hop-is-re-pinned`: every hop is re-checked and re-pinned, and a hop refused
    /// on its address fails the request rather than being dropped from the
    /// chain. Both halves are one assertion because they are one rule.
    #[test]
    fn a_redirect_is_re_checked_against_the_same_policy() {
        let (at, served) = origin(vec![
            "HTTP/1.1 301 Moved\r\nLocation: /elsewhere\r\nContent-Length: 0\r\n\r\n",
        ]);
        let mut followed = call(at, "test");
        followed.redirects = 1;

        let mut hops: Vec<String> = Vec::new();
        let refused = send(&followed, &mut |url, _downgrade| {
            hops.push(url.to_owned());
            Err(Fault::thrown("test refuses this address".to_owned()))
        })
        .expect_err("a hop the policy refused fails the request");

        assert_eq!(hops, vec![format!("http://{at}/elsewhere")]);
        assert!(
            format!("{refused:?}").contains("test refuses this address"),
            "the refusal the policy gave is the one that reaches the caller: {refused:?}"
        );
        served.join().expect("the origin thread");
    }

    /// A redirect nobody asked to follow is an answer, not a failure: the
    /// default is zero hops and the `301` is what the origin said.
    #[test]
    fn a_redirect_is_the_answer_when_no_hop_was_asked_for() {
        let (at, served) = origin(vec![
            "HTTP/1.1 301 Moved\r\nLocation: /elsewhere\r\nContent-Length: 0\r\n\r\n",
        ]);
        let reply = send(&call(at, "test"), &mut never).expect("the redirect itself");
        assert_eq!(reply.status, 301);
        served.join().expect("the origin thread");
    }

    /// `rule:http-server/retry-is-opt-in-jittered-and-closed`: a `503` is retried, the wait is jittered rather than
    /// fixed, and every attempt is inside the one deadline the call started
    /// with — which is what the elapsed time asserts, since a per-attempt
    /// budget would have allowed twice it.
    #[test]
    fn a_retry_is_jittered_and_shares_the_covering_deadline() {
        let (at, served) = origin(vec![
            "HTTP/1.1 503 Busy\r\nContent-Length: 0\r\n\r\n",
            "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok",
        ]);
        let mut retried = call(at, "test");
        retried.attempts = 2;
        retried.deadline = Instant::now() + Duration::from_secs(5);

        let started = Instant::now();
        let reply = send(&retried, &mut never).expect("the second attempt's answer");
        assert_eq!(reply.status, 200);
        assert!(started.elapsed() < Duration::from_secs(5));
        assert_eq!(served.join().expect("the origin thread").asked.len(), 2);

        // Full jitter is a draw in `[0, base × 2^n]` and not the bound itself:
        // over enough draws the same number every time is the failure this
        // asserts against, and the bound holding is the other half.
        let base = Duration::from_millis(100);
        let mut drawn: Vec<Duration> = (0..32).map(|_| backoff(base, 3)).collect();
        assert!(drawn.iter().all(|wait| *wait <= base * 8));
        drawn.dedup();
        assert!(drawn.len() > 1, "an unjittered backoff draws one value");
    }

    /// § 6: the deadline is not extended. A backoff that would end past it
    /// throws immediately rather than sleeping through the budget first.
    #[test]
    fn a_backoff_past_the_deadline_throws_rather_than_sleeping() {
        let (at, served) = origin(vec!["HTTP/1.1 503 Busy\r\nContent-Length: 0\r\n\r\n"]);
        let mut retried = call(at, "test");
        retried.attempts = 3;
        retried.backoff = Duration::from_secs(30);
        retried.deadline = Instant::now() + Duration::from_millis(500);

        let started = Instant::now();
        let expired = send(&retried, &mut never).expect_err("the deadline, not the backoff");
        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(format!("{expired:?}").contains("deadline"), "{expired:?}");
        served.join().expect("the origin thread");
    }

    /// A reply with `status` and the one header a `Retry-After` test needs.
    fn told(status: i64, after: &str) -> Streamed {
        Streamed {
            status,
            headers: vec![("retry-after".to_owned(), after.to_owned())],
            body: empty_body(),
            tls: None,
        }
    }

    /// A body with nothing in it and nothing behind it, for the cases that
    /// assert on a head alone.
    fn empty_body() -> Incoming {
        Incoming::over(
            Box::new(std::io::Cursor::new(Vec::new())),
            Vec::new(),
            super::Frame::UntilClose,
            Duration::from_secs(1),
            Instant::now() + Duration::from_secs(10),
            "test",
            None,
        )
        .expect("nothing to frame")
    }

    /// `at`, as the IMF-fixdate an origin would have written it.
    fn fixdate(at: std::time::SystemTime) -> String {
        let seconds = i64::try_from(
            at.duration_since(std::time::UNIX_EPOCH)
                .expect("a time after the epoch")
                .as_secs(),
        )
        .expect("a year this millennium");
        // The inverse of [`days_from_civil`], written out here rather than
        // shared: a formatter the subject also uses would agree with itself
        // whatever either of them got wrong.
        let days = seconds.div_euclid(86_400);
        let rest = seconds.rem_euclid(86_400);
        let mut year = 1970;
        let mut left = days;
        loop {
            let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
            let length = if leap { 366 } else { 365 };
            if left < length {
                break;
            }
            left -= length;
            year += 1;
        }
        let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
        let lengths = [
            31,
            if leap { 29 } else { 28 },
            31,
            30,
            31,
            30,
            31,
            31,
            30,
            31,
            30,
            31,
        ];
        let names = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ];
        let mut month = 0;
        while left >= lengths[month] {
            left -= lengths[month];
            month += 1;
        }
        format!(
            "Mon, {:02} {} {} {:02}:{:02}:{:02} GMT",
            left + 1,
            names[month],
            year,
            rest / 3_600,
            (rest % 3_600) / 60,
            rest % 60
        )
    }

    /// `rule:http-server/retry-is-opt-in-jittered-and-closed`: the field is read
    /// in both of its forms, and the date one replaces the computed backoff
    /// exactly as the seconds one does.
    ///
    /// Asserted on [`super::retry_after`] rather than on an elapsed wait,
    /// because what separates the two forms is a parse: a test that slept would
    /// pass a date it had read as no date at all, since an unread header and a
    /// short wait produce the same second.
    #[test]
    fn retry_after_as_an_http_date_replaces_the_backoff() {
        let soon = std::time::SystemTime::now() + Duration::from_secs(120);
        let dated =
            super::retry_after(&told(503, &fixdate(soon))).expect("a date in the future is a wait");
        assert!(
            dated > Duration::from_secs(100) && dated <= Duration::from_secs(120),
            "{dated:?}"
        );

        // The other form, on the other status, so neither is carrying the test.
        assert_eq!(
            super::retry_after(&told(429, " 7 ")),
            Some(Duration::from_secs(7))
        );

        // And a value that is neither form leaves the backoff in place.
        assert_eq!(super::retry_after(&told(503, "whenever")), None);
        assert_eq!(
            super::retry_after(&told(503, "Mon, 32 Xxx 2026 00:00:00 GMT")),
            None
        );
    }

    /// `rule:http-server/one-deadline-covers-the-whole-call`: a `Retry-After`
    /// longer than what is left of the deadline ends the call now rather than
    /// sleeping through the budget and then failing.
    ///
    /// The origin asks for a minute and the call has half a second, so the
    /// elapsed time is the whole assertion: a clamp that slept to the deadline
    /// and tried again would take that half second and answer `TimeoutError`
    /// too, which is why the bound asserted is well under it.
    #[test]
    fn retry_after_past_the_deadline_throws_now() {
        let (at, served) = origin(vec![
            "HTTP/1.1 503 Busy\r\nRetry-After: 60\r\nContent-Length: 0\r\n\r\n",
        ]);
        let mut retried = call(at, "test");
        retried.attempts = 3;
        retried.backoff = Duration::from_millis(1);
        retried.deadline = Instant::now() + Duration::from_millis(500);

        let started = Instant::now();
        let expired = send(&retried, &mut never).expect_err("the deadline, not the header");
        assert!(started.elapsed() < Duration::from_millis(400), "it slept");
        assert!(format!("{expired:?}").contains("deadline"), "{expired:?}");
        served.join().expect("the origin thread");
    }

    /// A date already past keeps the jittered backoff rather than becoming a
    /// zero wait — the rule's own sentence, and the one every client handed the
    /// same stale date depends on.
    ///
    /// The failure this forbids is a burst: `Some(ZERO)` here would have every
    /// holder of one date retry in the same instant, against a service that has
    /// just said it is busy.
    #[test]
    fn retry_after_already_past_keeps_the_jittered_backoff() {
        let gone = std::time::SystemTime::now() - Duration::from_secs(600);
        assert_eq!(super::retry_after(&told(503, &fixdate(gone))), None);

        // The boundary on the other side: a date far enough ahead is read, so
        // the `None` above is the pastness and not the parse.
        let ahead = std::time::SystemTime::now() + Duration::from_secs(600);
        assert!(super::retry_after(&told(503, &fixdate(ahead))).is_some());
    }

    /// The field is read on a `429` and a `503` and on nothing else, which is
    /// narrower than [`super::retryable`]'s four.
    ///
    /// Asked of every status the retry set holds, so a member that read the
    /// header off whatever it was given fails here while still looking right on
    /// the two rows it was written for.
    #[test]
    fn retry_after_on_a_502_or_504_is_not_read() {
        let read: Vec<i64> = [429, 502, 503, 504]
            .into_iter()
            .filter(|status| super::retry_after(&told(*status, "30")).is_some())
            .collect();
        assert_eq!(read, vec![429, 503], "a gateway does not set the pace");
    }

    /// A chunked body is joined before it is a `string`: the framing is the
    /// transport's question and no reader asks it a second time.
    #[test]
    fn a_chunked_body_is_joined_before_it_is_a_string() {
        let raw = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n\
                    2\r\nok\r\n3\r\n ay\r\n0\r\n\r\n";
        let (_, body) = parsed(raw).expect("a chunked reply");
        assert_eq!(body, b"ok ay");
    }

    /// A body that is not UTF-8 arrives whole and is not repaired here —
    /// whether it is text is `Core\Http\Response::text`'s question
    /// (`rule:errors/ambiguous-input-refused`), and a reply the program only
    /// wanted the octets of must not fail on the way to `bytes()`.
    #[test]
    fn a_body_that_is_not_text_arrives_whole() {
        let mut raw = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n".to_vec();
        raw.extend_from_slice(&[0xff, 0xfe]);
        let (_, body) = parsed(&raw).expect("bytes that are not text");
        assert_eq!(body, [0xff, 0xfe]);
    }

    /// A header a caller wrote cannot end its own line: a value carrying
    /// `\r\n` would be a second request nobody asked for.
    #[test]
    fn a_header_value_carrying_a_line_ending_is_refused() {
        let (at, served) = origin(vec!["HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n"]);
        let mut injected = call(at, "test");
        injected.headers = vec![("X-Trace".to_owned(), "a\r\nGET /admin HTTP/1.1".to_owned())];
        let refused = send(&injected, &mut never).expect_err("an injected line");
        assert!(
            format!("{refused:?}").contains("control byte"),
            "{refused:?}"
        );

        // Nothing was sent, so the origin is still waiting: connect to it
        // ourselves to let the thread finish.
        let _ = std::net::TcpStream::connect(at);
        drop(served);
    }

    /// A chunk header that is not a length is a statement about the other end,
    /// so it fails rather than being guessed at.
    #[test]
    fn a_chunk_header_that_is_not_a_length_fails() {
        let raw = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\nzz\r\nok\r\n0\r\n\r\n";
        assert!(parsed(raw).is_err());
    }

    /// The header lookup is case-insensitive because HTTP field names are.
    #[test]
    fn a_header_is_found_whatever_case_the_origin_wrote_it_in() {
        let raw = b"HTTP/1.1 301 Moved\r\nLOCATION: /next\r\nContent-Length: 0\r\n\r\n";
        let (reply, _) = parsed(raw).expect("a redirect");
        assert_eq!(super::redirect_of(&reply).as_deref(), Some("/next"));
    }

    /// Every request offers the three codings this client can undo, and an
    /// `Accept-Encoding` a program wrote is refused rather than sent beside
    /// ours.
    ///
    /// The refusal is asserted at [`super::compose`] rather than over a socket
    /// because that is where it happens: a head that never goes out is the
    /// point of it, and a case that sent one would be asserting on whether the
    /// origin was still listening.
    #[test]
    fn accept_encoding_offers_gzip_br_and_zstd() {
        let (at, served) = origin(vec!["HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n"]);
        send(&call(at, "test"), &mut never).expect("an answer");
        let asked = served.join().expect("the origin thread").asked;
        assert!(
            asked[0].contains("Accept-Encoding: gzip, br, zstd\r\n"),
            "{}",
            asked[0]
        );

        let mut theirs = call(at, "test");
        theirs.headers = vec![("Accept-Encoding".to_owned(), "deflate".to_owned())];
        let parts = super::parts(&theirs.url, "test").expect("a URL this case wrote");
        let refused = super::compose(&theirs, &parts).expect_err("a program's own offer");
        assert!(
            format!("{refused:?}").contains("Accept-Encoding"),
            "the refusal names the header: {refused:?}"
        );
    }

    /// A reply under each offered coding is decoded before a program sees it,
    /// and one that expands past the bound is refused rather than truncated
    /// (`rule:core-classes/decompression-bound`).
    ///
    /// The two fields that described the frame are asserted **absent** from
    /// what comes back: a `Content-Length` counting compressed octets beside a
    /// decoded body is a number no reader of it can use.
    #[test]
    fn gzip_brotli_and_zstd_replies_are_decoded_under_the_compress_bound_and_a_bomb_is_refused() {
        let payload: Vec<u8> = (0..4096_u32).map(|n| (n % 251) as u8).collect();
        for (coding, codec) in [
            ("gzip", Codec::Gzip),
            ("br", Codec::Brotli),
            ("zstd", Codec::Zstd),
        ] {
            let frame = compress_to(codec, &payload).expect("a frame this case wrote");
            let (at, served) = origin_raw(vec![encoded(coding, &frame)]);
            let reply = send(&call(at, "test"), &mut never).expect("an answer");
            assert_eq!(reply.body, payload, "a `{coding}` reply arrives decoded");
            assert!(
                !reply
                    .headers
                    .iter()
                    .any(|(name, _)| name == "content-encoding" || name == "content-length"),
                "a decoded body keeps neither field that framed it: {:?}",
                reply.headers
            );
            served.join().expect("the origin thread");
        }

        // A megabyte of one octet under a four-kilobyte ceiling: the ratio is
        // wide open, so what refuses it is the absolute half.
        let bomb = compress_to(Codec::Gzip, &vec![b'A'; 1 << 20]).expect("a frame");
        let (at, served) = origin_raw(vec![encoded("gzip", &bomb)]);
        let mut bounded = call(at, "test");
        bounded.compress = Bound {
            bytes: 4096,
            ratio: 1000,
        };
        let refused = send(&bounded, &mut never).expect_err("a bomb past the bound");
        let why = format!("{refused:?}");
        assert!(
            why.contains("rule:core-classes/decompression-bound"),
            "the refusal names the rule: {why}"
        );
        assert!(
            why.contains("test"),
            "the refusal names the member that made the call: {why}"
        );
        served.join().expect("the origin thread");
    }

    /// A coding the request never offered is refused naming it, rather than
    /// handed on as the octets it is still in.
    ///
    /// `deflate` is the one to ask it with: it is the coding this client
    /// deliberately does not offer, and an origin sending it anyway is exactly
    /// the case a silently-undecoded body would be mistaken for text in.
    #[test]
    fn a_content_encoding_the_request_did_not_offer_is_refused_naming_it() {
        let (at, served) = origin_raw(vec![encoded("deflate", b"never read")]);
        let refused = send(&call(at, "test"), &mut never).expect_err("a coding never offered");
        let why = format!("{refused:?}");
        assert!(
            why.contains("deflate"),
            "the refusal names the coding: {why}"
        );
        assert!(
            why.contains("gzip, br, zstd"),
            "and what was offered instead: {why}"
        );
        served.join().expect("the origin thread");
    }

    /// A streamed reply under a coding is decoded **as it arrives**: a reader
    /// takes decoded octets out of it while the rest of the frame is still
    /// unwritten, which is what keeps a compressed stream a stream.
    ///
    /// The origin withholds the tail until this case says so, so what comes out
    /// of the decoder before that cannot have been decoded from octets the
    /// origin had not sent — a pause between two writes would assert the same
    /// thing on a clock instead, and on a loaded machine it would assert it
    /// wrongly.
    ///
    /// The payload is noise on purpose. A compressible one makes a frame short
    /// enough to arrive in a single piece, and the case would then be asserting
    /// what a buffered decode already does.
    #[test]
    fn a_streamed_reply_is_decoded_as_it_arrives_rather_than_gathered_first() {
        let payload = noise(1 << 20);
        for (coding, codec) in [
            ("gzip", Codec::Gzip),
            ("br", Codec::Brotli),
            ("zstd", Codec::Zstd),
        ] {
            let frame = compress_to(codec, &payload).expect("a frame this case wrote");
            // Three quarters, so that every backend has whole blocks of its own
            // framing to work from before the tail arrives.
            let cut = frame.len() / 4 * 3;
            let (at, tail, served) = withheld(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Encoding: {coding}\r\nContent-Length: {}\r\n\r\n",
                    frame.len()
                ),
                frame[..cut].to_vec(),
                frame[cut..].to_vec(),
            );

            let mut asking = call(at, "test");
            asking.idle = Duration::from_secs(10);
            let mut reply = send_streamed(&asking, &mut never).expect("a head");
            assert!(
                !reply
                    .headers
                    .iter()
                    .any(|(name, _)| name == "content-encoding" || name == "content-length"),
                "a decoded stream keeps neither field that framed it: {:?}",
                reply.headers
            );

            assert!(
                reply
                    .body
                    .pull()
                    .expect("what has already arrived, decoded"),
                "a `{coding}` stream answers its first pull from the octets it has"
            );
            let early = reply.body.held().len();
            assert!(
                early > 0 && early < payload.len(),
                "a `{coding}` stream decodes a piece and not the whole body: {early}"
            );
            assert_eq!(
                reply.body.held(),
                &payload[..early],
                "and what it decoded is the body's own first octets"
            );

            tail.send(()).ok();
            while reply.body.pull().expect("the rest of the body") {}
            assert_eq!(
                reply.body.held(),
                payload,
                "a `{coding}` stream read to its end is the whole body, decoded"
            );
            served.join().expect("the origin thread");
        }
    }

    /// A frame this case can split without it compressing away: a linear
    /// congruential sequence, which is the shortest thing here that no codec
    /// finds a pattern in.
    fn noise(octets: usize) -> Vec<u8> {
        let mut state = 0x2545_F491_4F6C_DD1D_u64;
        (0..octets)
            .map(|_| {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                u8::try_from((state >> 33) & 0xFF).unwrap_or_default()
            })
            .collect()
    }

    /// A listener answering `head` and `first`, and then `rest` once the sender
    /// speaks.
    ///
    /// The withheld tail is what makes a case about streaming deterministic:
    /// what a reader got before the channel was spoken to cannot have come from
    /// octets the origin had not written.
    fn withheld(
        head: String,
        first: Vec<u8>,
        rest: Vec<u8>,
    ) -> (SocketAddr, mpsc::Sender<()>, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a loopback port");
        let at = listener.local_addr().expect("its own address");
        let (tell, told) = mpsc::channel::<()>();
        let served = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("the client's connection");
            let mut request = [0_u8; 4096];
            let _ = stream.read(&mut request);
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(&first);
            let _ = stream.flush();
            // A case that fails before it releases the tail ends here rather
            // than holding the run.
            told.recv_timeout(Duration::from_secs(20)).ok();
            let _ = stream.write_all(&rest);
            let _ = stream.flush();
        });
        (at, tell, served)
    }

    /// `rule:core-api/tier-roster`'s "over the runtime's own reactor rather than a second
    /// event loop", asserted from the core's side rather than the caller's: the
    /// exchange is driven with `Scheduler::run` alone, so a core that came back
    /// is what the assertions describe. A transport that blocked in `read` — or
    /// that waited on a poll of its own — would never have returned from that
    /// call with the reply still half-written.
    ///
    /// The origin writes its reply in two halves and holds the second until
    /// this thread releases it, and the loop below turns until the origin says
    /// it has the request. Both are what make the park a **read**'s: on a
    /// platform whose non-blocking connect is still in flight when
    /// `finish_connecting` first asks, the connect parks too, so a test
    /// asserting the first park it sees would pass with the read never reaching
    /// the reactor at all. Past the origin's signal the task has written
    /// everything it is going to write and the reply cannot be complete, so
    /// parked has only the one meaning left.
    #[test]
    fn a_socket_read_runs_on_the_reactor_and_parks_its_coroutine() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a loopback port");
        let at = listener.local_addr().expect("its own address");
        let (asked, has_request) = mpsc::channel();
        let (release, permitted) = mpsc::channel();
        let served = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("a connection");
            let mut request = [0_u8; 4096];
            let read = stream.read(&mut request).expect("a request");
            // Announced before the half-reply and not after it: a signal that
            // trailed the bytes could be missed by exactly the turn that
            // consumed them, and the loop would then wait on a readiness this
            // thread is holding back.
            asked.send(()).expect("the test thread is waiting");
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nCon")
                .expect("half a head");
            stream.flush().expect("a flushed half");
            permitted.recv().expect("the test releases the rest");
            stream
                .write_all(b"tent-Length: 2\r\n\r\nok")
                .expect("the rest of the reply");
            stream.flush().expect("a flushed reply");
            String::from_utf8_lossy(&request[..read]).into_owned()
        });

        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));

        // The outcome is carried out as text rather than asserted inside the
        // task: a panic at a task root is contained by `run_task` and reported
        // through the scheduler, so an `expect` in there would fail quietly.
        let outcome: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
        let answered = Rc::clone(&outcome);
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Worker, move |_ctx| {
            let answer = match send(&call(at, "test"), &mut never) {
                Ok(reply) => {
                    format!("{} {}", reply.status, String::from_utf8_lossy(&reply.body))
                }
                Err(fault) => format!("{fault:?}"),
            };
            *answered.borrow_mut() = Some(answer);
        });

        let mut report = sched.run();
        let mut turns = 0;
        while has_request.try_recv().is_err() {
            with_current(|reactor| reactor.turn(&mut sched))
                .expect("no reactor is installed on this thread")
                .expect("the poll failed");
            report = sched.run();
            turns += 1;
            assert!(turns < 64, "the request never reached the origin");
        }

        assert_eq!(
            report.finished, 0,
            "the exchange returned with half a reply on the wire"
        );
        assert_eq!(report.parked, 1, "the read did not park its coroutine");
        assert_eq!(
            with_current(|reactor| reactor.registrations()),
            Some(1),
            "the park filed nothing for the reactor to wake it on"
        );
        assert!(
            outcome.borrow().is_none(),
            "the task ran past its read without a whole reply to read"
        );

        // Left running rather than abandoned parked: a test that ends here
        // would assert the park and never that the park is survivable.
        release.send(()).expect("the origin thread");
        run_until_idle(&mut sched).expect("the loop failed");
        assert_eq!(
            outcome.borrow().as_deref(),
            Some("200 ok"),
            "the parked read never came back with the whole reply"
        );
        let request = served.join().expect("the origin thread");
        assert!(request.starts_with("GET /ok HTTP/1.1\r\n"), "{request}");
    }

    /// A sink that keeps what it was given and how it was given it.
    struct Chunks {
        /// Every write's length, in order.
        sizes: Vec<usize>,
        /// Everything written, joined.
        written: Vec<u8>,
    }

    impl Write for Chunks {
        fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
            self.sizes.push(buffer.len());
            self.written.extend_from_slice(buffer);
            Ok(buffer.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// A body that is the file at `path`, whatever is in it now.
    fn file_body(path: &std::path::Path) -> super::Body {
        let handle = std::fs::File::open(path).expect("the file this test wrote");
        let length = handle.metadata().expect("its size").len();
        super::Body {
            content_type: None,
            length,
            pieces: vec![super::Piece::File {
                handle: RefCell::new(Box::new(handle)),
                length,
            }],
        }
    }

    /// A file in this platform's temporary directory holding `content`.
    fn scratch(name: &str, content: &[u8]) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(name);
        std::fs::write(&path, content).expect("a scratch file");
        path
    }

    /// `rule:http-server/an-outbound-request-carries-one-body`: a file part is
    /// what a body larger than this process's own ceiling is written as, so the
    /// buffer between the disk and the socket is one chunk however big the file
    /// is.
    #[test]
    fn a_file_body_is_streamed_at_one_chunk_whatever_its_size() {
        let content = vec![b'x'; super::BODY_CHUNK * 5 + 17];
        let path = scratch("nvs-http-one-chunk.bin", &content);
        let body = file_body(&path);

        let mut sink = Chunks {
            sizes: Vec::new(),
            written: Vec::new(),
        };
        let failed = super::write_body(&body, &mut sink, "Core\\Http\\Client::put")
            .expect("the file was readable");

        assert!(failed.is_none(), "a `Vec` sink cannot fail");
        assert_eq!(sink.written, content, "the whole file did not arrive");
        assert_eq!(
            body.length,
            content.len() as u64,
            "the `Content-Length` promised is not the file's size"
        );
        // The claim is the *ceiling*, not the count: a writer holding the file
        // would show one write of five megabytes here and still arrive with the
        // same octets.
        assert!(
            sink.sizes.iter().all(|size| *size <= super::BODY_CHUNK),
            "a write carried more than one chunk: {:?}",
            sink.sizes
        );
        assert!(
            sink.sizes.len() > 1,
            "a file larger than a chunk went out in one write"
        );
    }

    /// `rule:http-server/an-outbound-request-carries-one-body`: the body is
    /// built once and a file part is read again per attempt, so a retry holds
    /// nothing between the two and sends what is on disk when it sends.
    #[test]
    fn a_file_body_is_re_read_on_every_attempt() {
        let path = scratch("nvs-http-re-read.bin", b"the first attempt");
        let body = file_body(&path);
        let member = "Core\\Http\\Client::put";

        let mut first = Chunks {
            sizes: Vec::new(),
            written: Vec::new(),
        };
        super::write_body(&body, &mut first, member).expect("the file was readable");
        assert_eq!(first.written, b"the first attempt");

        // The same length, so the head this attempt already sent is still the
        // truth, and different octets, so a writer that kept the first read
        // fails here.
        std::fs::write(&path, b"the second effort").expect("the scratch file");
        let mut second = Chunks {
            sizes: Vec::new(),
            written: Vec::new(),
        };
        super::write_body(&body, &mut second, member).expect("the file was readable");
        assert_eq!(
            second.written, b"the second effort",
            "the second attempt sent what the first one read"
        );
    }

    /// A listener that answers with `head` and then holds the connection open
    /// saying nothing, until the channel tells it to let go.
    fn quiet(head: &'static str) -> (SocketAddr, mpsc::Sender<()>, std::thread::JoinHandle<()>) {
        listening(head, false)
    }

    /// The same, dribbling a byte every few milliseconds instead of going
    /// quiet — every read lands inside any `idle` worth naming, which is the
    /// case only a lifetime ends.
    fn dribbling(
        head: &'static str,
    ) -> (SocketAddr, mpsc::Sender<()>, std::thread::JoinHandle<()>) {
        listening(head, true)
    }

    /// The two above: answer one connection with `head`, then either dribble or
    /// say nothing until the sender goes or speaks.
    fn listening(
        head: &'static str,
        dribble: bool,
    ) -> (SocketAddr, mpsc::Sender<()>, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a loopback port");
        let at = listener.local_addr().expect("its own address");
        let (done, told) = mpsc::channel();
        let served = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("a connection");
            let mut request = [0_u8; 4096];
            let _ = stream.read(&mut request);
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.flush();
            while matches!(told.try_recv(), Err(mpsc::TryRecvError::Empty)) {
                if dribble && (stream.write_all(b".").is_err() || stream.flush().is_err()) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        });
        (at, done, served)
    }

    /// `rule:http-server/a-streamed-reply-is-bounded-by-idle-and-a-lifetime`'s
    /// first bound. The head arrives and the body never does, and what ends the
    /// call is the silence — well inside a `deadline` that has not moved, which
    /// is the whole point of the stream carrying two bounds of its own.
    #[test]
    fn stream_idle_ends_a_silent_reply() {
        let (at, done, served) = quiet("HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n");
        let mut asking = call(at, "test");
        asking.idle = Duration::from_millis(50);

        let began = Instant::now();
        let mut reply = send_streamed(&asking, &mut never).expect("a head");
        assert_eq!(reply.status, 200);
        let refused = reply.body.pull().expect_err("a body that never came");
        assert!(
            format!("{refused:?}").contains("sent nothing for"),
            "{refused:?}"
        );
        assert!(
            began.elapsed() < Duration::from_secs(5),
            "the silence ended it and not the ten-second deadline"
        );

        done.send(()).ok();
        served.join().expect("the origin thread");
    }

    /// The same rule's second bound, and the case the first one cannot reach:
    /// an origin sending a byte at a time passes every idle check ever written,
    /// so the walk is ended by the lifetime or by nothing.
    #[test]
    fn stream_max_duration_ends_an_endless_reply() {
        let (at, done, served) = dribbling("HTTP/1.1 200 OK\r\n\r\n");
        let mut asking = call(at, "test");
        asking.idle = Duration::from_secs(30);
        asking.max_duration = Duration::from_millis(300);

        let mut reply = send_streamed(&asking, &mut never).expect("a head");
        let refused = loop {
            match reply.body.pull() {
                Ok(true) => {}
                Ok(false) => panic!("an origin that never stops has no end to reach"),
                Err(err) => break err,
            }
        };
        assert!(
            format!("{refused:?}").contains("`maxDuration`"),
            "{refused:?}"
        );

        done.send(()).ok();
        served.join().expect("the origin thread");
    }

    /// § 6's retry, and where it stops. The head is retried like any other
    /// answer; once it has arrived the body is on one connection, and a body
    /// the origin cut short is not a second request — retrying there would send
    /// the request twice for one walk and splice the second reply's tail onto
    /// the first one's.
    #[test]
    fn stream_is_retried_before_its_head_and_never_after() {
        let (at, served) = origin(vec![
            "HTTP/1.1 503 Busy\r\nContent-Length: 0\r\n\r\n",
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n2\r\nok\r\n",
        ]);
        let mut asking = call(at, "test");
        asking.attempts = 2;

        let mut reply = send_streamed(&asking, &mut never).expect("the second attempt's head");
        assert_eq!(
            reply.status, 200,
            "the `503` was retried, not answered with"
        );
        assert_eq!(
            reply.body.held(),
            b"ok",
            "the chunk that arrived in the same reads as the head is already framed"
        );

        let cut = reply.body.pull().expect_err("a body the origin cut short");
        assert!(
            format!("{cut:?}").contains("shorter than its own header"),
            "{cut:?}"
        );

        let asked = served.join().expect("the origin thread").asked;
        assert_eq!(
            asked.len(),
            2,
            "the head was asked for twice and the body for never"
        );
    }

    /// `rule:http-server/an-outbound-connection-is-pooled-per-core-and-stays-pinned`:
    /// a connection outlives its call, and serves only calls whose own key it
    /// was filed under.
    ///
    /// The three draws are the three ways this can go wrong at once — the
    /// second call must reuse, a second port must not, and a second name over
    /// the **same** address must not either, which is the element a pool keyed
    /// on the URL's host would have got right and the one keyed on the address
    /// alone would not.
    #[test]
    fn pooled_connection_is_reused_for_one_pinned_address_and_never_across_two() {
        let ok = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok";
        let (first, one) = origin(vec![ok, ok, ok]);
        let (second, two) = origin(vec![ok]);

        send(&call(first, "test"), &mut never).expect("an answer");
        send(&call(first, "test"), &mut never).expect("an answer on the connection it left");

        send(&call(second, "test"), &mut never).expect("an answer from the other port");

        let mut named = call(first, "test");
        named.url = format!("http://localhost:{}/ok", first.port());
        send(&named, &mut never).expect("an answer under the other name");

        assert_eq!(
            one.join().expect("the origin thread").connections,
            2,
            "the second call reused the first's connection and the third name did not"
        );
        assert_eq!(
            two.join().expect("the origin thread").connections,
            1,
            "a port of its own is a key of its own"
        );
    }

    /// The fallback the approved set exists for, at the level that can see the
    /// clock: the set's first address answers nothing, and the call is served
    /// by the next one inside the same `connectTimeout` rather than spending
    /// the whole of it on the first.
    #[test]
    fn a_dead_first_address_falls_back_to_the_next_within_connect_timeout() {
        let (at, served) = origin(vec!["HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok"]);
        let mut asking = call(at, "test");
        // Documentation address space: a host with a default route sends the
        // SYN and hears nothing, and one without a route to it is refused at
        // once. Both are the case the walk exists for, and neither may cost the
        // call its budget.
        asking.addresses = vec!["192.0.2.1".parse().expect("a literal address"), at.ip()];
        asking.connect_timeout = Duration::from_secs(5);

        let start = Instant::now();
        let reply = send(&asking, &mut never).expect("the second address served the call");

        assert_eq!(reply.status, 200);
        assert!(
            start.elapsed() < Duration::from_secs(3),
            "the walk waited out the dead address instead of starting the next one"
        );
        assert_eq!(
            served.join().expect("the origin thread").connections,
            1,
            "the address that answered is the one the call was carried on"
        );
    }

    /// An error naming only the last address sends the reader to the wrong
    /// host, so every address the call could not reach is in the one it throws.
    #[test]
    fn every_address_failing_is_one_io_error_naming_each() {
        let nowhere: SocketAddr = "203.0.113.7:80".parse().expect("a literal address");
        let mut asking = call(nowhere, "test");
        asking.addresses = vec![
            "192.0.2.1".parse().expect("a literal address"),
            "192.0.2.2".parse().expect("a literal address"),
        ];
        // Short on purpose: what is under test is the error, and the set is
        // documentation address space, so this is the whole of what it costs.
        asking.connect_timeout = Duration::from_millis(300);

        let refused = send(&asking, &mut never).expect_err("a set with nothing behind it");
        let why = format!("{refused:?}");

        assert!(
            why.contains("192.0.2.1"),
            "the error did not name the first address: {why}"
        );
        assert!(
            why.contains("192.0.2.2"),
            "the error did not name the second address: {why}"
        );
    }

    /// A retried call performs exactly one resolution: the second attempt walks
    /// the set the first one was given, and the only way a second lookup could
    /// reach the wire is the hop loop's `repin`, which this case panics from.
    #[test]
    fn a_retry_reuses_the_approved_set_and_never_re_resolves() {
        let (at, served) = origin(vec![
            "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\n\r\n",
            "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok",
        ]);
        let mut retried = call(at, "test");
        retried.attempts = 2;
        retried.addresses = vec!["192.0.2.1".parse().expect("a literal address"), at.ip()];

        let reply = send(&retried, &mut never).expect("the second attempt's answer");

        assert_eq!(reply.status, 200);
        assert_eq!(
            served.join().expect("the origin thread").asked.len(),
            2,
            "the retry did not reach the set the first attempt was approved for"
        );
    }

    /// Each member of the set was approved, so a connection held to any one of
    /// them serves the call — the lookup is per address, and a set whose first
    /// address is not the held one still finds it.
    #[test]
    fn a_pooled_connection_to_any_approved_address_serves_the_call() {
        let ok = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok";
        let (at, served) = origin(vec![ok, ok]);

        send(&call(at, "test"), &mut never).expect("an answer");

        let mut wider = call(at, "test");
        wider.addresses = vec!["192.0.2.1".parse().expect("a literal address"), at.ip()];
        let reply = send(&wider, &mut never).expect("an answer on the connection it left");

        assert_eq!(reply.status, 200);
        assert_eq!(
            served.join().expect("the origin thread").connections,
            1,
            "a call whose set covers the held address opened a second connection"
        );
    }

    /// The rule's *only after its reply was read to the end under known
    /// framing*: a `Content-Length` reply hands its connection back, and a
    /// `Connection: close` beside one takes it away again.
    #[test]
    fn pooled_connection_goes_back_only_after_a_fully_framed_reply() {
        let framed = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok";
        let closing = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok";
        let (at, served) = origin(vec![framed, closing, framed]);

        send(&call(at, "test"), &mut never).expect("a framed reply");
        send(&call(at, "test"), &mut never).expect("the origin's last reply on that connection");
        send(&call(at, "test"), &mut never).expect("a framed reply on a new one");

        assert_eq!(
            served.join().expect("the origin thread").connections,
            2,
            "the `close` ended the first connection and the third call opened the second"
        );

        // The other two arms, asked of the head directly: a body the close
        // itself delimits has no end short of that close, and an HTTP/1.0 reply
        // keeps a connection only where it says so, which this client never
        // asks for.
        let head = b"HTTP/1.1 200 OK\r\n";
        assert!(super::reusable(head, &[], super::Frame::Sized(2)));
        assert!(super::reusable(head, &[], super::Frame::Chunked));
        assert!(!super::reusable(head, &[], super::Frame::UntilClose));
        assert!(!super::reusable(
            b"HTTP/1.0 200 OK\r\n",
            &[],
            super::Frame::Sized(2)
        ));
    }

    /// A connection whose far end has gone: every write fails, as a socket the
    /// other end retired does once the kernel has seen the reset.
    struct Retired;

    impl Read for Retired {
        fn read(&mut self, _into: &mut [u8]) -> std::io::Result<usize> {
            Ok(0)
        }
    }

    impl Write for Retired {
        fn write(&mut self, _from: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::from(std::io::ErrorKind::BrokenPipe))
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Err(std::io::Error::from(std::io::ErrorKind::BrokenPipe))
        }
    }

    impl super::Connection for Retired {
        fn bound_by(&mut self, _at: Option<Instant>) {}

        fn tls(&self) -> Option<nvs_host::tls::Session> {
            None
        }
    }

    /// The rule's *replaced once without spending an attempt*: a connection the
    /// far end retired while this core held it idle is not a failed request, so
    /// a call with one attempt still gets its answer.
    ///
    /// The dead connection is filed by hand rather than produced by a listener
    /// that closes, because whether a write to a socket the other end has just
    /// closed fails on this write or the next one is the kernel's to decide,
    /// and a case built on that would be asserting on the timing rather than on
    /// the rule.
    #[test]
    fn stale_pooled_connection_is_replaced_once_without_spending_an_attempt() {
        let (at, served) = origin(vec!["HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok"]);
        let asking = call(at, "test");
        assert_eq!(asking.attempts, 1, "the replacement is not an attempt");

        let parts = super::parts(&asking.url, "test").expect("the URL this case wrote");
        super::pool::release(
            super::pool_key(
                &parts,
                Some(SocketAddr::new(at.ip(), parts.port)),
                None,
                &CallPolicy::default(),
                None,
            ),
            Box::new(Retired),
            asking.pool,
            Instant::now(),
        );

        let reply = send(&asking, &mut never).expect("the replacement's answer");
        assert_eq!(reply.status, 200);
        assert_eq!(reply.body, b"ok");
        assert_eq!(
            served.join().expect("the origin thread").connections,
            1,
            "the one attempt bought the one connection that answered"
        );
    }

    /// A client identity issued to `name`, as a call carries one.
    ///
    /// Generated rather than read from a fixture: what these cases are about is
    /// that two identities are two clients, and two chains a fixture holds
    /// would be two more files saying the same thing.
    fn client_identity(name: &str) -> super::Identity {
        let issued = rcgen::generate_simple_self_signed(vec![format!("{name}.client.example")])
            .expect("the client certificate could not be generated");
        let session = nvs_host::tls::NvsIdentity::read(
            issued.cert.pem().as_bytes(),
            &issued.signing_key.serialize_der(),
        )
        .expect("a chain and its own key were refused");
        let fingerprint = crate::http::fingerprint_of(session.leaf());
        super::Identity {
            session,
            fingerprint,
        }
    }

    /// Both halves of what an identity is: the session it builds answers a
    /// server that asks the client for a certificate, and the name a connection
    /// under it is filed by carries its leaf.
    ///
    /// *When asked* is the whole of the presenting rule — a session sends
    /// nothing until a `CertificateRequest` arrives — and a call that named no
    /// identity is configured to answer one with nothing at all, which
    /// `nvs_host::tls`'s own case asserts on the shipped configuration.
    #[test]
    fn client_identity_is_presented_when_asked_and_is_part_of_the_pool_key() {
        let (mine, theirs) = (client_identity("one"), client_identity("two"));
        assert!(
            mine.session.presents(),
            "a call under an identity answers a server that asks for a certificate"
        );
        assert_ne!(
            mine.fingerprint, theirs.fingerprint,
            "two certificates are two identities"
        );

        let parts =
            super::parts("https://api.example/v1", "test").expect("the URL this case wrote");
        let socket = SocketAddr::new(IpAddr::from([203, 0, 113, 7]), parts.port);
        let strict = CallPolicy::default();
        let under_mine =
            super::pool_key(&parts, Some(socket), Some(&mine.fingerprint), &strict, None);

        assert!(
            under_mine.contains(&mine.fingerprint),
            "the key names the identity the connection was opened under: {under_mine}"
        );
        assert_ne!(
            under_mine,
            super::pool_key(
                &parts,
                Some(socket),
                Some(&theirs.fingerprint),
                &strict,
                None
            ),
            "two identities never share a connection"
        );
        assert_ne!(
            under_mine,
            super::pool_key(&parts, Some(socket), None, &strict, None),
            "an identity is not the absence of one"
        );
    }

    /// The anchor every `https` case here runs under: the self-signed
    /// `localhost` certificate `crate::tests::outbound_client` trusts.
    ///
    /// It is built there rather than here because the client is the *process's*
    /// and `cache`'s TLS store reaches the same one — that module doc is the
    /// home of the reasoning.
    fn trusted() -> &'static (CertificateDer<'static>, Vec<u8>) {
        crate::tests::outbound_client()
    }

    /// A loopback origin that terminates TLS under `cert` and answers `reply`
    /// to the first request on every connection it accepts, with the count of
    /// those connections.
    ///
    /// The count is what the refusing case asserts on: a handshake the client
    /// walks away from leaves the origin nothing to report, so what the far end
    /// still holds is how many times it was reached.
    fn tls_origin(
        cert: CertificateDer<'static>,
        key: Vec<u8>,
        reply: &'static str,
    ) -> (u16, Arc<AtomicUsize>) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a loopback port");
        let port = listener.local_addr().expect("its own address").port();
        listener
            .set_nonblocking(true)
            .expect("an accept that does not outlive the count");
        let accepted = Arc::new(AtomicUsize::new(0));
        let counting = Arc::clone(&accepted);
        std::thread::spawn(move || {
            let config = serving(cert, key);
            let ends = Instant::now() + Duration::from_secs(20);
            while Instant::now() < ends {
                let Ok((sock, _)) = listener.accept() else {
                    std::thread::sleep(Duration::from_millis(1));
                    continue;
                };
                counting.fetch_add(1, Ordering::Relaxed);
                let config = Arc::clone(&config);
                std::thread::spawn(move || answer_tls(sock, config, reply));
            }
        });
        (port, accepted)
    }

    /// The server side of [`trusted`]'s certificate: the one `rustls`
    /// configuration an origin thread terminates TLS under, whichever exchange
    /// it goes on to answer.
    fn serving(cert: CertificateDer<'static>, key: Vec<u8>) -> Arc<rustls::ServerConfig> {
        Arc::new(
            rustls::ServerConfig::builder_with_provider(Arc::new(
                rustls::crypto::ring::default_provider(),
            ))
            .with_safe_default_protocol_versions()
            .expect("the provider refused the shipped versions")
            .with_no_client_auth()
            .with_single_cert(
                vec![cert],
                rustls::pki_types::PrivateKeyDer::Pkcs8(
                    rustls::pki_types::PrivatePkcs8KeyDer::from(key),
                ),
            )
            .expect("the certificate and the key did not pair"),
        )
    }

    /// [`socket_origin`] behind TLS: one opening handshake, answered with a
    /// `101` from inside a session under [`trusted`]'s certificate, with the
    /// request as it arrived on the far side of it.
    ///
    /// Both halves are the ones already here — [`serving`]'s certificate and
    /// [`handshook`]'s derived accept — because what a `wss` case is written to
    /// see is the one outbound TLS client carrying the handshake this module
    /// already writes (`rule:security/one-tls-client`), and an origin with its
    /// own idea of either would be asserting against a path nothing runs.
    ///
    /// A client that walks away from the certificate leaves this thread with
    /// nothing to report, which is the empty string: what such a case asserts
    /// on is the refusal on the client's side.
    fn tls_socket_origin(
        cert: CertificateDer<'static>,
        key: Vec<u8>,
        chosen: Option<&'static str>,
    ) -> (u16, std::thread::JoinHandle<String>) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a loopback port");
        let port = listener.local_addr().expect("its own address").port();
        listener
            .set_nonblocking(true)
            .expect("an accept that does not outlive the case");
        let served = std::thread::spawn(move || {
            let config = serving(cert, key);
            // A case that never opens what it said it would ends here rather
            // than holding the run, for [`origin_raw`]'s reason.
            let ends = Instant::now() + Duration::from_secs(20);
            while Instant::now() < ends {
                let Ok((mut sock, _)) = listener.accept() else {
                    std::thread::sleep(Duration::from_millis(1));
                    continue;
                };
                // Said rather than assumed, for [`answer`]'s reason: an
                // accepted connection inherits the listener's non-blocking mode
                // on Windows.
                drop(sock.set_nonblocking(false));
                drop(sock.set_read_timeout(Some(Duration::from_secs(5))));
                let Ok(mut conn) = rustls::ServerConnection::new(Arc::clone(&config)) else {
                    return String::new();
                };
                let mut tls = rustls::Stream::new(&mut conn, &mut sock);
                return handshook(&mut tls, chosen);
            }
            String::new()
        });
        (port, served)
    }

    /// One connection's handshake and the one request behind it.
    ///
    /// Every failure ends this thread quietly rather than panicking it: a
    /// client that refuses the certificate hangs up mid-flight, which is
    /// precisely what one of the two cases is written to see, and every claim
    /// either of them makes is made on the client's side.
    fn answer_tls(mut sock: std::net::TcpStream, config: Arc<rustls::ServerConfig>, reply: &str) {
        // Said rather than assumed, for [`answer`]'s reason: an accepted
        // connection inherits the listener's non-blocking mode on Windows.
        drop(sock.set_nonblocking(false));
        drop(sock.set_read_timeout(Some(Duration::from_secs(5))));
        let Ok(mut conn) = rustls::ServerConnection::new(config) else {
            return;
        };
        let mut tls = rustls::Stream::new(&mut conn, &mut sock);
        let mut request = [0_u8; 4096];
        if tls.read(&mut request).is_err() {
            return;
        }
        drop(tls.write_all(reply.as_bytes()));
        drop(tls.flush());
    }

    /// The seam stage 9 exists for, end to end: an `https` call made the way a
    /// program makes one reaches an origin whose certificate the operator's
    /// `roots` file vouches for, and the reply comes back through the same
    /// framing a plaintext one does.
    ///
    /// Every other case in this module stops at one end of that seam or the
    /// other — `nvs_host::tls` proves which certificates a configuration
    /// accepts, and the cases above prove what this module does with a socket
    /// once it has one. What neither covers is that the two are wired together:
    /// the anchors this transport verifies against are the ones the process was
    /// configured with.
    #[test]
    fn https_call_through_the_client_reaches_a_loopback_origin_under_a_roots_file() {
        let (cert, key) = trusted();
        let (port, accepted) = tls_origin(
            cert.clone(),
            key.clone(),
            "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok",
        );

        let mut asking = call(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port),
            "test",
        );
        asking.url = format!("https://localhost:{port}/ok");
        let reply = send(&asking, &mut never).expect("the loopback origin's answer");

        assert_eq!(reply.status, 200);
        assert_eq!(reply.body, b"ok");
        assert_eq!(
            accepted.load(Ordering::Relaxed),
            1,
            "one call that handshook once is one connection"
        );
    }

    /// `rule:http-server/a-reply-reports-its-tls-session`'s three facts, and the
    /// `null` beside them: an `https` reply carries what the handshake settled,
    /// and a plaintext reply carries nothing at all.
    ///
    /// Both sides, because the report on its own would pass just as well on a
    /// client that filled the slot for every reply, and telling the two apart is
    /// the slot's whole purpose. The chain is asserted against the certificate
    /// this origin was stood up with rather than against a length, so a client
    /// reporting *a* chain rather than this peer's fails here too.
    #[test]
    fn tls_info_reports_version_cipher_and_the_peer_chain() {
        let (cert, key) = trusted();
        let (port, _) = tls_origin(
            cert.clone(),
            key.clone(),
            "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok",
        );

        let mut asking = call(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port),
            "test",
        );
        asking.url = format!("https://localhost:{port}/ok");
        let reply = send(&asking, &mut never).expect("the loopback origin's answer");

        let reported = reply.tls.expect("an https reply reports its session");
        assert_eq!(reported.session.version(), "TLSv1.3");
        assert!(
            reported.session.cipher().starts_with("TLS_"),
            "the suite is reported under its IANA name: {}",
            reported.session.cipher()
        );
        assert_eq!(
            reported.session.chain(),
            [cert.as_ref().to_vec()].as_slice(),
            "the chain is this peer's own certificate, leaf first"
        );
        assert!(
            reported.verified,
            "a call under the configured roots checked the chain and the name"
        );

        let (at, served) = origin(vec!["HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok"]);
        let plain = send(&call(at, "test"), &mut never).expect("a plaintext answer");
        assert!(
            plain.tls.is_none(),
            "a plaintext reply has no session to report"
        );
        served.join().expect("the origin thread");
    }

    /// `verified` is the member the reporting slot exists for: a pinned call and
    /// a `tlsVerify: false` one both answer `false`, while `tlsCa` — which
    /// replaces the trust set and then checks the chain and the name against it —
    /// still answers `true` (ADR 0180 § 13).
    ///
    /// All three against an origin no configured root vouches for, so every
    /// relaxation here is load-bearing: the same call under the default policy is
    /// the refusal the case above asserts. That is what stops this passing on a
    /// client that reported `false` for every session it ever negotiated.
    #[test]
    fn tls_info_is_unverified_after_a_pinned_or_relaxed_call() {
        use base64::Engine as _;
        use rcgen::PublicKeyData as _;
        use sha2::Digest as _;

        // The anchors this process runs under vouch for another certificate,
        // which is what makes the origin below an unvouched-for one.
        trusted();
        let issued = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])
            .expect("the loopback certificate could not be generated");
        let pin = format!(
            "sha256//{}",
            base64::engine::general_purpose::STANDARD.encode(sha2::Sha256::digest(
                issued.signing_key.subject_public_key_info()
            ))
        );
        let (port, _) = tls_origin(
            issued.cert.der().clone(),
            issued.signing_key.serialize_der(),
            "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok",
        );

        for (policy, verified, why) in [
            (
                CallPolicy {
                    pins: vec![pin],
                    ..CallPolicy::default()
                },
                false,
                "a pin accepts a key with no chain built at all",
            ),
            (
                CallPolicy {
                    insecure: true,
                    ..CallPolicy::default()
                },
                false,
                "`tlsVerify: false` checks neither the chain nor the name",
            ),
            (
                CallPolicy {
                    anchors: Some(issued.cert.pem()),
                    ..CallPolicy::default()
                },
                true,
                "`tlsCa` checks both, against the certificates the call handed over",
            ),
        ] {
            let mut asking = call(
                SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port),
                "test",
            );
            asking.url = format!("https://localhost:{port}/ok");
            asking.policy = policy;
            let reply = send(&asking, &mut never).expect("the relaxed call was refused");
            let reported = reply.tls.expect("an https reply reports its session");
            assert_eq!(reported.verified, verified, "{why}");
        }
    }

    /// The same call against an origin holding a certificate no configured
    /// anchor vouches for: it throws, and no second attempt is made.
    ///
    /// Both halves matter. A handshake refused for the peer's certificate is
    /// settled — retrying reaches the same origin with the same chain and the
    /// same answer — so this module hands it back as
    /// `rule:core-api/failure-throws`'s throw rather than as an attempt that
    /// failed, and the connection count is the far end's own record that it was
    /// reached once. The pause is what makes the count final: a retry would be
    /// out on the wire within this call's millisecond of backoff.
    #[test]
    fn https_call_to_an_origin_no_root_vouches_for_throws_and_is_not_retried() {
        // The anchors, installed before anything else: what makes this origin's
        // certificate unvouched-for is that the `roots` file holds another one.
        trusted();
        let unvouched = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])
            .expect("the loopback certificate could not be generated");
        let (port, accepted) = tls_origin(
            unvouched.cert.der().clone(),
            unvouched.signing_key.serialize_der(),
            "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok",
        );

        let mut asking = call(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port),
            "test",
        );
        asking.url = format!("https://localhost:{port}/ok");
        asking.attempts = 3;
        let refused = send(&asking, &mut never).expect_err("an unvouched-for chain was accepted");

        let said = format!("{refused:?}");
        assert!(
            said.contains("the TLS handshake with `localhost` was refused"),
            "the refusal names the host and says nothing was sent: {said}"
        );
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(
            accepted.load(Ordering::Relaxed),
            1,
            "a refused certificate is settled, so the three attempts were not spent on it"
        );
    }

    /// What a deployment writes to let one host be redirected down into
    /// plaintext, and the same deployment without that one key.
    const DOWNGRADE_GRANTED: &str = "[capabilities.net]\nconnect = [\"127.0.0.1\"]\ninternal = \
                                     [\"127.0.0.1\"]\ndowngrade = [\"127.0.0.1\"]\n";
    /// See [`DOWNGRADE_GRANTED`].
    const DOWNGRADE_UNGRANTED: &str =
        "[capabilities.net]\nconnect = [\"127.0.0.1\"]\ninternal = [\"127.0.0.1\"]\n";

    /// One `https` call to an origin whose only answer is a `Location` into
    /// `plain`, run through the very re-pin `super::super::exchanged` hands this
    /// module.
    ///
    /// The closure is the real one and not a stand-in, which is what makes the
    /// three cases below statements about the client rather than about a test's
    /// own copy of the rule: `grant` is what the deployment wrote and `option`
    /// is what the call wrote, so each case moves exactly one of them.
    fn downgrade_hop(plain: SocketAddr, grant: &str, option: bool) -> Result<Reply, Fault> {
        let (cert, key) = trusted();
        let location: &'static str = Box::leak(
            format!(
                "HTTP/1.1 301 Moved\r\nLocation: http://{plain}/next\r\nContent-Length: 0\r\n\r\n"
            )
            .into_boxed_str(),
        );
        let (port, _) = tls_origin(cert.clone(), key.clone(), location);

        let mut followed = call(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port),
            "test",
        );
        followed.url = format!("https://localhost:{port}/ok");
        followed.redirects = 1;

        let mut ctx = Ctx::buffered();
        ctx.set_config(crate::tests::granting(grant));
        let mut args = [Value::null(); super::super::REQUEST_ARITY];
        if option {
            args[super::super::REDIRECT_TO_HTTP] = Value::bool(true);
        }
        send(&followed, &mut |hop, downgrade| {
            super::super::repinned(&mut ctx, &args, hop, downgrade, "test")
        })
    }

    /// `rule:http-server/an-https-redirect-never-becomes-plaintext`'s
    /// deployment half: the call asked for the hop and nobody said this host
    /// could be reached in the clear.
    ///
    /// The refusal has to land before the plaintext socket, which is what the
    /// accept below asserts — a client that checked afterwards would have sent
    /// the request in the clear and then complained about it.
    #[test]
    fn https_to_http_redirect_is_refused_without_the_downgrade_grant() {
        let plain = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a loopback port");
        plain
            .set_nonblocking(true)
            .expect("a listener that answers now rather than waiting");
        let at = plain.local_addr().expect("its own address");

        let refused = downgrade_hop(at, DOWNGRADE_UNGRANTED, true)
            .expect_err("a hop out of TLS to a host no `net.downgrade` names");
        let said = format!("{refused:?}");
        assert!(
            said.contains("net.downgrade"),
            "the refusal names the grant an operator would write: {said}"
        );

        match plain.accept() {
            Err(err) if err.kind() == ErrorKind::WouldBlock => {}
            Ok(_) => panic!("the grant was missing and the request went out in the clear anyway"),
            Err(err) => panic!("the listener failed for a reason that is not the point: {err}"),
        }
    }

    /// The same rule's call half: the deployment named this host, and the call
    /// never said it expected to leave TLS behind. Following a redirect is
    /// already opt-in, and this is the one hop opting in does not include.
    #[test]
    fn https_to_http_redirect_is_refused_under_the_grant_without_the_option() {
        let plain = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a loopback port");
        plain
            .set_nonblocking(true)
            .expect("a listener that answers now rather than waiting");
        let at = plain.local_addr().expect("its own address");

        let refused = downgrade_hop(at, DOWNGRADE_GRANTED, false)
            .expect_err("a grant is not a downgrade the call asked for");
        let said = format!("{refused:?}");
        assert!(
            said.contains(super::super::REDIRECT_TO_HTTP_OPTION) && said.contains("net.downgrade"),
            "the refusal names both halves, since the operator reading it holds one: {said}"
        );

        match plain.accept() {
            Err(err) if err.kind() == ErrorKind::WouldBlock => {}
            Ok(_) => panic!("the call expected no downgrade and the request went out anyway"),
            Err(err) => panic!("the listener failed for a reason that is not the point: {err}"),
        }
    }

    /// Both halves written, and the hop is taken: the body that comes back is
    /// the plaintext origin's, so the two refusals above are about the two keys
    /// rather than about a hop this client cannot follow at all.
    #[test]
    fn https_to_http_redirect_is_followed_under_the_grant_and_the_option() {
        let (at, served) = origin(vec!["HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nplain"]);

        let reply = downgrade_hop(at, DOWNGRADE_GRANTED, true)
            .expect("the deployment named the host and the call expected the hop");
        assert_eq!(reply.status, 200);
        assert_eq!(reply.body, b"plain");

        let served = served.join().expect("the plaintext origin's thread");
        assert_eq!(served.asked.len(), 1, "one hop is one request in the clear");
    }

    /// A call that names no identity never draws a connection that presented
    /// one.
    ///
    /// The one that matters of the two directions: a TLS connection that
    /// presented a certificate stays authenticated as that client for as long
    /// as it is open, so an anonymous call reusing one would send its request
    /// under somebody else's credential. Asserted by the connection still being
    /// in the store afterwards, which says it was never drawn — a fresh one
    /// answering says only that one was opened.
    #[test]
    fn a_call_without_an_identity_never_reuses_a_connection_that_presented_one() {
        let (at, served) = origin(vec!["HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok"]);
        let asking = call(at, "test");
        assert!(
            asking.identity.is_none(),
            "this case's call is the anonymous one"
        );

        let identity = client_identity("pooled");
        let parts = super::parts(&asking.url, "test").expect("the URL this case wrote");
        let theirs = super::pool_key(
            &parts,
            Some(SocketAddr::new(at.ip(), parts.port)),
            Some(&identity.fingerprint),
            &CallPolicy::default(),
            None,
        );
        super::pool::release(
            theirs.clone(),
            Box::new(Retired),
            asking.pool,
            Instant::now(),
        );

        let reply = send(&asking, &mut never).expect("the anonymous call's answer");
        assert_eq!(reply.status, 200);
        assert_eq!(
            served.join().expect("the origin thread").connections,
            1,
            "the anonymous call opened one connection of its own"
        );
        assert!(
            super::pool::take(&theirs, Instant::now()).is_some(),
            "the identity's connection was still in the store, so nothing anonymous drew it"
        );
    }

    /// A call that verifies never draws a connection opened under a policy
    /// that did not.
    ///
    /// `rule:security/tls-trust-is-relaxed-only-under-a-host-grant`'s last
    /// clause about the pool, and the case above's argument with the other
    /// weakening in it: a session that checked neither the chain nor the name
    /// stays that session for as long as it is open, so a strict call served
    /// one would be talking to whoever answered an earlier call — at a host its
    /// own grants were never asked about, because it asked for nothing.
    /// Asserted by the connection still being in the store afterwards, which
    /// says it was never drawn; a fresh one answering says only that one was
    /// opened.
    #[test]
    fn a_relaxed_connection_never_serves_a_strict_call_from_the_pool() {
        let (at, served) = origin(vec!["HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok"]);
        let asking = call(at, "test");
        assert_eq!(
            asking.policy,
            CallPolicy::default(),
            "this case's call is the one that asked for nothing"
        );

        let parts = super::parts(&asking.url, "test").expect("the URL this case wrote");
        let relaxed = super::pool_key(
            &parts,
            Some(SocketAddr::new(at.ip(), parts.port)),
            None,
            &CallPolicy {
                insecure: true,
                ..CallPolicy::default()
            },
            None,
        );
        super::pool::release(
            relaxed.clone(),
            Box::new(Retired),
            asking.pool,
            Instant::now(),
        );

        let reply = send(&asking, &mut never).expect("the strict call's answer");
        assert_eq!(reply.status, 200);
        assert_eq!(
            served.join().expect("the origin thread").connections,
            1,
            "the strict call opened one connection of its own"
        );
        assert!(
            super::pool::take(&relaxed, Instant::now()).is_some(),
            "the relaxed connection was still in the store, so nothing strict drew it"
        );
    }

    /// `rule:http-server/a-cross-origin-redirect-drops-credentials`: a hop to
    /// another origin carries none of the caller's headers, so the three the
    /// rule names go, and with them the one this case wrote a credential into
    /// under a name no list holds.
    ///
    /// All of them and not a list, because `secret` is erased before codegen
    /// (`rule:security/secret-qualifier`): `X-Api-Key` is a plain `String` by
    /// the time this module holds it, exactly as a program's `secret string`
    /// is. This client's own headers are composed for the hop, which is what
    /// the offer asserts — they are not the caller's to leak.
    #[test]
    fn cross_origin_redirect_drops_authorization_cookie_and_every_secret_header() {
        let (elsewhere, answering) = origin(vec!["HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok"]);
        let (at, served) = origin_raw(vec![
            format!(
                "HTTP/1.1 302 Found\r\nLocation: http://{elsewhere}/next\r\nContent-Length: 0\r\n\r\n"
            )
            .into_bytes(),
        ]);

        let mut followed = call(at, "test");
        followed.redirects = 1;
        followed.headers = vec![
            ("Authorization".to_owned(), "Bearer sk-live-42".to_owned()),
            ("Cookie".to_owned(), "session=abcdef".to_owned()),
            (
                "Proxy-Authorization".to_owned(),
                "Basic ZGVtbw==".to_owned(),
            ),
            ("X-Api-Key".to_owned(), "k-93ce".to_owned()),
        ];

        let reply = send(&followed, &mut |_url, _downgrade| Ok(vec![elsewhere.ip()]))
            .expect("the hop's answer");
        assert_eq!(reply.status, 200);

        let hop = &answering.join().expect("the second origin thread").asked[0];
        let lower = hop.to_ascii_lowercase();
        for name in [
            "authorization",
            "cookie",
            "proxy-authorization",
            "x-api-key",
        ] {
            assert!(
                !lower.contains(&format!("\r\n{name}:")),
                "`{name}` crossed to another origin: {hop}"
            );
        }
        for value in [
            "Bearer sk-live-42",
            "session=abcdef",
            "Basic ZGVtbw==",
            "k-93ce",
        ] {
            assert!(!hop.contains(value), "a credential's value crossed: {hop}");
        }
        assert!(
            hop.contains("Accept-Encoding: gzip, br, zstd\r\n"),
            "the hop still carries this client's own headers: {hop}"
        );
        served.join().expect("the first origin thread");
    }

    /// The rule's other half: a hop inside one origin — the scheme, the host
    /// and the port all equal — keeps every header the caller wrote, because
    /// nothing about who is being talked to has changed.
    #[test]
    fn same_origin_redirect_keeps_the_callers_headers() {
        let (at, served) = origin(vec![
            "HTTP/1.1 302 Found\r\nLocation: /next\r\nContent-Length: 0\r\n\r\n",
            "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok",
        ]);
        let mut followed = call(at, "test");
        followed.redirects = 1;
        followed.headers = vec![
            ("Authorization".to_owned(), "Bearer sk-live-42".to_owned()),
            ("X-Api-Key".to_owned(), "k-93ce".to_owned()),
        ];

        let reply = send(&followed, &mut |_url, _downgrade| Ok(vec![at.ip()]))
            .expect("the hop's own answer");
        assert_eq!(reply.status, 200);

        let asked = served.join().expect("the origin thread").asked;
        assert!(
            asked[1].starts_with("GET /next HTTP/1.1\r\n"),
            "the hop is the case's second request: {}",
            asked[1]
        );
        assert!(
            asked[1].contains("Authorization: Bearer sk-live-42\r\n"),
            "{}",
            asked[1]
        );
        assert!(asked[1].contains("X-Api-Key: k-93ce\r\n"), "{}", asked[1]);
    }

    /// The rule's second paragraph: no header value reaches a trace span, a log
    /// record or an error message.
    ///
    /// This module reaches no `Ctx` at all — [`Call::pool`]'s own doc is why —
    /// so a span and a record are not things it can write, and what is left to
    /// assert is every message it produces about a header. Both name the header
    /// and neither carries the value, which is what makes the guard a property
    /// rather than a habit.
    #[test]
    fn no_trace_span_or_error_message_carries_a_header_value() {
        let credential = "sk-live-must-not-appear";
        let nowhere = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 9);
        let parts = {
            let any = call(nowhere, "test");
            super::parts(&any.url, any.member).expect("a URL this case wrote")
        };

        let mut injected = call(nowhere, "Core\\Http\\Client::get");
        injected.headers = vec![(
            "X-Api-Key".to_owned(),
            format!("{credential}\r\nX-Smuggled: 1"),
        )];
        let refused = format!(
            "{:?}",
            super::compose(&injected, &parts).expect_err("a value that would end the line")
        );
        assert!(
            refused.contains("X-Api-Key"),
            "the refusal names the header: {refused}"
        );
        assert!(
            !refused.contains(credential),
            "a header value reached a message: {refused}"
        );

        let mut offered = call(nowhere, "Core\\Http\\Client::get");
        offered.headers = vec![("Accept-Encoding".to_owned(), credential.to_owned())];
        let refused = format!(
            "{:?}",
            super::compose(&offered, &parts).expect_err("a program's own offer")
        );
        assert!(
            refused.contains("Accept-Encoding"),
            "the refusal names the header: {refused}"
        );
        assert!(
            !refused.contains(credential),
            "a header value reached a message: {refused}"
        );
    }

    /// [ADR 0183](/docs/decisions/0183.md) § 4: an outbound socket is opened
    /// like an outbound call, so the handshake is dialled at an address the
    /// door approved and asks for the URL's own path.
    ///
    /// Asserted on the octets the origin read rather than on what [`asked`]
    /// built, because what is under test is that the `Call`'s pin is what was
    /// connected to — a question only the far end can answer
    /// (`rule:http-server/allow-url-pins-the-address`).
    #[test]
    fn socket_handshake_connects_to_the_approved_address() {
        let (at, served) = socket_origin(None);
        let mut opening = call(at, "Core\\Http\\Client::openSocket");
        opening.url = format!("ws://{at}/chat");

        let opened = upgrade(&opening, &[], WebSocketConfig::default())
            .expect("a `101` from the address the door approved");
        assert_eq!(
            opened.protocol, None,
            "a reply naming no subprotocol settles on none"
        );
        // Before the join: the origin ends its connection when this one does.
        drop(opened.socket);

        let asked = served.join().expect("the origin thread");
        assert_eq!(
            asked.lines().next(),
            Some("GET /chat HTTP/1.1"),
            "the handshake asks for the URL's own path: {asked}"
        );
        assert_eq!(
            header_of(&asked, "host"),
            Some(at.to_string()),
            "the authority the URL wrote is what `Host:` carries: {asked}"
        );
    }

    /// [ADR 0183](/docs/decisions/0183.md) § 4: the headers are the call's and
    /// the offered subprotocols are the socket's, and one handshake carries
    /// both.
    ///
    /// The offer is asserted by **counting** the fields as well as reading the
    /// one: RFC 6455 has a peer choose from a single comma-joined list, and an
    /// end that emitted a line per offer would still look right on the line
    /// that asserts the first one was sent. What the peer chose is read back
    /// off [`Upgraded`] rather than off the reply, since judging it against the
    /// offer belongs to the row that holds the offer.
    #[test]
    fn socket_handshake_carries_the_callers_headers_and_offered_protocols() {
        let (at, served) = socket_origin(Some("chat.v2"));
        let mut opening = call(at, "Core\\Http\\Client::openSocket");
        opening.url = format!("ws://{at}/chat");
        opening.headers = vec![("X-Tenant".to_owned(), "acme".to_owned())];
        let offered = vec!["chat.v1".to_owned(), "chat.v2".to_owned()];

        let opened = upgrade(&opening, &offered, WebSocketConfig::default())
            .expect("a `101` naming one of the two offers");
        assert_eq!(
            opened.protocol.as_deref(),
            Some("chat.v2"),
            "the name the peer chose is what the handshake left"
        );
        drop(opened.socket);

        let asked = served.join().expect("the origin thread");
        assert_eq!(
            header_of(&asked, "x-tenant"),
            Some("acme".to_owned()),
            "the caller's own header crosses the wire: {asked}"
        );
        assert_eq!(
            header_of(&asked, "sec-websocket-protocol"),
            Some("chat.v1, chat.v2".to_owned()),
            "both offers are on it, in the order the call gave them: {asked}"
        );
        let fields = asked
            .lines()
            .filter(|line| {
                line.split_once(':').is_some_and(|(field, _)| {
                    field.trim().eq_ignore_ascii_case("sec-websocket-protocol")
                })
            })
            .count();
        assert_eq!(fields, 1, "the offers are one field, not one each: {asked}");
    }

    /// [ADR 0183](/docs/decisions/0183.md) § 4: a `101` naming a subprotocol
    /// the call never offered is refused, and the connection it arrived on is
    /// dropped rather than handed back.
    ///
    /// Both sides of the boundary are asserted together: the same handshake
    /// against a peer that chose the offered name opens, and against one that
    /// chose a name beside it does not. A judgement that never fired at all
    /// would still pass either line on its own.
    ///
    /// The live refusal is `tungstenite`'s, which asks the question inside the
    /// handshake and answers it without naming the name, so what names it is
    /// asserted of [`settled`] directly — the judgement the scripted arm of
    /// `Core\Http\Client::openSocket` reaches, where there is no codec to ask.
    #[test]
    fn a_101_choosing_a_protocol_not_offered_is_refused() {
        let offers = vec!["chat.v1".to_owned()];

        let (at, served) = socket_origin(Some("chat.v1"));
        let mut opening = call(at, "Core\\Http\\Client::openSocket");
        opening.url = format!("ws://{at}/chat");
        let opened = upgrade(&opening, &offers, WebSocketConfig::default())
            .expect("the name this call offered");
        assert_eq!(opened.protocol.as_deref(), Some("chat.v1"));
        drop(opened.socket);
        served.join().expect("the origin thread");

        let (at, served) = socket_origin(Some("chat.v9"));
        let mut asking = call(at, "Core\\Http\\Client::openSocket");
        asking.url = format!("ws://{at}/chat");
        let Err(why) = upgrade(&asking, &offers, WebSocketConfig::default()) else {
            panic!("a name nobody offered opened a socket");
        };
        let refused = format!("{why:?}");
        assert!(
            refused.to_lowercase().contains("subprotocol"),
            "the refusal says what was wrong with the `101`: {refused}"
        );
        served.join().expect("the origin thread");

        let Err(named) = super::settled(Some("chat.v9".to_owned()), &offers, "test") else {
            panic!("a name nobody offered settled");
        };
        let named = format!("{named:?}");
        assert!(
            named.contains("chat.v9"),
            "the judgement names what the peer chose: {named}"
        );
        assert!(
            super::settled(Some("chat.v1".to_owned()), &offers, "test").is_ok(),
            "the offered name is the one that settles"
        );
    }

    /// `rule:http-server/redirects-are-off-and-every-hop-is-re-pinned`, as
    /// [ADR 0183](/docs/decisions/0183.md) § 4 reads it for a socket: there is
    /// no spelling for following a hop here, so a `3xx` answering the
    /// handshake is the end of the call.
    ///
    /// It is named as a redirect and carries its `Location` because that is the
    /// one answer a caller would otherwise read as a network failure — the peer
    /// is reachable and did answer, and nothing but the message says where it
    /// pointed.
    #[test]
    fn a_redirect_answering_a_socket_handshake_is_a_runtime_error_naming_the_location() {
        let elsewhere = "ws://moved.invalid/chat";
        let (at, served) = origin(vec![
            "HTTP/1.1 302 Found\r\nLocation: ws://moved.invalid/chat\r\nContent-Length: 0\r\n\r\n",
        ]);
        let mut opening = call(at, "Core\\Http\\Client::openSocket");
        opening.url = format!("ws://{at}/chat");

        let Err(why) = upgrade(&opening, &[], WebSocketConfig::default()) else {
            panic!("a redirect is the answer rather than a hop");
        };
        let Fault::Thrown(class, said) = &why else {
            panic!("a redirect is something a `catch` can see: {why:?}");
        };
        assert!(
            matches!(class, nvs_runtime::ThrownClass::Runtime),
            "a peer that answered is not an `IOError`: {why:?}"
        );
        assert!(
            said.contains(elsewhere),
            "the refusal names where the peer pointed: {said}"
        );
        assert!(said.contains("302"), "and what it answered with: {said}");

        let asked = served.join().expect("the origin thread").asked;
        assert_eq!(asked.len(), 1, "nothing followed the hop: {asked:?}");
    }

    /// `rule:security/one-tls-client`, as [ADR 0183](/docs/decisions/0183.md)
    /// § 4 reads it for a socket: a `wss` handshake is a `ws` handshake inside
    /// a session the process's one outbound client established, built from the
    /// policy the `Call` carries, and what it leaves is the same [`Upgraded`].
    ///
    /// Both sides of that policy, because the opening half alone would pass
    /// just as well on an end that handed the session no policy at all: the
    /// same handshake, against an origin holding the same certificate, under a
    /// call whose own anchors vouch for a different one, never reaches a `101`.
    /// The refusal is read as the TLS one rather than as any error, since a
    /// socket that failed to connect at all would satisfy a bare `is_err`.
    #[test]
    fn wss_socket_completes_over_the_tls_client_under_the_calls_policy() {
        let (cert, key) = trusted();
        let offered = vec!["chat.v1".to_owned()];

        let (port, served) = tls_socket_origin(cert.clone(), key.clone(), Some("chat.v1"));
        let mut opening = call(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port),
            "Core\\Http\\Client::openSocket",
        );
        opening.url = format!("wss://localhost:{port}/chat");

        let opened = upgrade(&opening, &offered, WebSocketConfig::default())
            .expect("a `101` inside the session the roots file vouches for");
        assert_eq!(
            opened.protocol.as_deref(),
            Some("chat.v1"),
            "a `wss` handshake settles a subprotocol as a `ws` one does"
        );
        // Before the join: the origin ends its connection when this one does.
        drop(opened.socket);

        let asked = served.join().expect("the origin thread");
        assert_eq!(
            asked.lines().next(),
            Some("GET /chat HTTP/1.1"),
            "the handshake crossed the session intact: {asked}"
        );
        assert_eq!(
            header_of(&asked, "host"),
            Some(format!("localhost:{port}")),
            "the authority the URL wrote is what `Host:` carries: {asked}"
        );

        let elsewhere = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])
            .expect("a second loopback certificate could not be generated");
        let (port, served) = tls_socket_origin(cert.clone(), key.clone(), Some("chat.v1"));
        let mut asking = call(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port),
            "Core\\Http\\Client::openSocket",
        );
        asking.url = format!("wss://localhost:{port}/chat");
        asking.policy = CallPolicy {
            anchors: Some(elsewhere.cert.pem()),
            ..CallPolicy::default()
        };

        let Err(why) = upgrade(&asking, &offered, WebSocketConfig::default()) else {
            panic!("a certificate the call's own anchors do not vouch for opened a socket");
        };
        let refused = format!("{why:?}").to_lowercase();
        assert!(
            refused.contains("tls handshake"),
            "the refusal is the session's, not the socket's: {refused}"
        );
        served.join().expect("the origin thread");
    }

    /// `rule:http-server/a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise`,
    /// as [ADR 0183](/docs/decisions/0183.md) § 4 reads it for a socket: the
    /// operator's tunnel is the call's, so a handshake goes through `CONNECT`
    /// to the address the door approved and the `101` comes back over it.
    ///
    /// The bypassed half is asserted beside it, because an end that tunnelled
    /// unconditionally would pass the first half alone and quietly send a host
    /// the operator excluded through the proxy anyway. What says so is the
    /// proxy's own count, read rather than joined, for [`tls_origin`]'s reason:
    /// a tunnel that was never asked for leaves nothing to join on.
    #[test]
    fn a_socket_is_opened_through_the_proxy_tunnel_when_one_is_configured() {
        let offered = vec!["chat.v1".to_owned()];

        let (at, served) = socket_origin(Some("chat.v1"));
        let (proxying, _, asked) = proxy(1, None);
        let mut opening = call(at, "Core\\Http\\Client::openSocket");
        opening.url = format!("ws://{at}/chat");
        opening.proxy = Some(through(proxying));

        let opened = upgrade(&opening, &offered, WebSocketConfig::default())
            .expect("a `101` from behind the tunnel");
        assert_eq!(
            opened.protocol.as_deref(),
            Some("chat.v1"),
            "the conversation the tunnel carries is the one that was opened"
        );
        // Before the join: the origin ends its connection when this one does.
        drop(opened.socket);

        let asked = asked.join().expect("the proxy thread");
        assert!(
            asked[0].starts_with(&format!("CONNECT {at} HTTP/1.1\r\n")),
            "the tunnel is opened to the address the door approved: {asked:?}"
        );
        let behind = served.join().expect("the origin thread");
        assert_eq!(
            behind.lines().next(),
            Some("GET /chat HTTP/1.1"),
            "and the handshake itself is what crossed it: {behind}"
        );
        assert!(
            !behind.contains("CONNECT"),
            "nothing of the tunnel's own exchange reaches the origin: {behind}"
        );

        let (at, served) = socket_origin(Some("chat.v1"));
        let (proxying, reached, _) = proxy(1, None);
        let mut direct = call(at, "Core\\Http\\Client::openSocket");
        direct.url = format!("ws://{at}/chat");
        let mut past = through(proxying);
        past.bypass = vec![at.ip().to_string()];
        direct.proxy = Some(past);

        let opened = upgrade(&direct, &offered, WebSocketConfig::default())
            .expect("a `101` from the origin itself");
        drop(opened.socket);
        served.join().expect("the origin thread");
        assert_eq!(
            reached.load(Ordering::Relaxed),
            0,
            "a bypassed host is dialled directly, tunnel configured or not"
        );
    }
}
