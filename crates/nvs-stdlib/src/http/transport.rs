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
//! # One connection per attempt, closed by the reply
//!
//! Every request carries `Connection: close` and the body ends at end of file,
//! so there is no pool, no keep-alive and no second request sharing a socket.
//! That costs a connection setup per attempt and buys the whole framing
//! question: a reply that ends when the socket does needs no agreement about
//! what comes after it. A pool is a later slice and a measurable one — it is
//! `rule:programs/memory-priority` priority 3
//! against priority 4, and nothing in this goal's acceptance is waiting on it.
//!
//! What a call spends is one buffer holding the whole reply, capped at
//! [`REPLY_CEILING`], plus the request text. Per in-flight request and released
//! with it; nothing here is retained across calls.
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
//! retried. There is no plaintext fallback and no spelling for a session that
//! verifies nothing; both are priority-1 failures wearing a feature's name.

use std::cell::RefCell;
use std::io::{ErrorKind, Read, Seek, Write};
use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};

use fluent_uri::component::{Authority, Scheme};
use fluent_uri::{Uri, UriRef};
use nvs_host::net::NvsTcp;
use nvs_host::tls::NvsTls;
use nvs_runtime::{Fault, ThrownClass};
use rand::RngExt;

/// The most reply a single call will hold, headers and body together.
///
/// A cap and not a configuration: a reply is bytes another host chose, so
/// "until memory runs out" is that host deciding this process's footprint. Two
/// megabytes past the point where a caller should be streaming instead, which
/// is the member this class does not have yet.
const REPLY_CEILING: usize = 8 * 1024 * 1024;

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
    /// The address [`super::pin`] approved for [`Call::url`]'s host.
    pub(crate) address: IpAddr,
    /// The whole call's budget — every attempt, every hop, every backoff.
    pub(crate) deadline: Instant,
    /// The handshake's own bound, which is separate from the total.
    pub(crate) connect_timeout: Duration,
    /// The caller's own headers, in the order the array wrote them.
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
    /// The W3C `traceparent` naming the request this call is made from, or
    /// `None` where `[trace] propagate` is off.
    ///
    /// Already the answer, like every other field here: whether to propagate and
    /// what the id is are both read in [`super::traceparent_of`], off the `Ctx`
    /// this module deliberately cannot reach.
    pub(crate) traceparent: Option<String>,
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
    /// The body's octets, exactly as they arrived once any transfer coding was
    /// undone — a `Vec<u8>` and not a `String` because whether they are text is
    /// `Core\Http\Response::text`'s question and not this module's, and a reply
    /// a program only wanted the bytes of must not throw on the way here.
    pub(crate) body: Vec<u8>,
    /// Every header, names lower-cased, values trimmed, in arrival order — a
    /// list rather than a map because a field the origin sent twice is two
    /// lines, and `Core\Http\Response::headers` is the member that says so.
    pub(crate) headers: Vec<(String, String)>,
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
    Answered(Reply),
    /// The socket did not get there, with the sentence a refusal would carry.
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
    /// Whether the scheme was `https`, and so whether a handshake runs before
    /// the request goes out.
    tls: bool,
}

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
pub(crate) fn send(
    call: &Call<'_>,
    repin: &mut dyn FnMut(&str) -> Result<IpAddr, Fault>,
) -> Result<Reply, Fault> {
    let mut url = call.url.clone();
    let mut address = call.address;
    let mut hops = 0_u32;
    loop {
        let reply = attempts(call, &url, address)?;
        let Some(location) = redirect_of(&reply) else {
            return Ok(reply);
        };
        if hops >= call.redirects {
            return Ok(reply);
        }
        hops += 1;
        url = resolved(&url, &location, call.member)?;
        // Before the connection and not after it: § 4 refuses a hop on its
        // *address*, and an address that has not been asked about yet is one
        // the first URL's approval is standing in for.
        address = repin(&url)?;
    }
}

/// One URL's worth of attempts, under the call's own deadline.
fn attempts(call: &Call<'_>, url: &str, address: IpAddr) -> Result<Reply, Fault> {
    let mut attempt = 0_u32;
    loop {
        if Instant::now() >= call.deadline {
            return Err(expired(call.member));
        }
        let last = attempt + 1 >= call.attempts;
        let wait = match one(call, url, address)? {
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

/// One connection, one request, one reply.
fn one(call: &Call<'_>, url: &str, address: IpAddr) -> Result<Attempt, Fault> {
    let parts = parts(url, call.member)?;
    let request = compose(call, &parts)?;
    let socket = SocketAddr::new(address, parts.port);

    // The handshake's own bound, clamped by what is left of the total: a
    // `connectTimeout` longer than the remaining deadline would be the one
    // spelling § 5 says does not exist, arrived at by arithmetic.
    let budget = call
        .connect_timeout
        .min(call.deadline.saturating_duration_since(Instant::now()));
    let mut stream = match NvsTcp::connect_timeout(socket, budget) {
        Ok(stream) => stream,
        Err(err) => {
            return Ok(Attempt::Failed(format!(
                "connecting to {socket} failed: {err}"
            )));
        }
    };
    // Before the handshake, not after it: the TLS flight waits on this socket
    // and the deadline is what bounds every wait on it.
    stream.set_deadline(Some(call.deadline));

    if !parts.tls {
        return exchange(call, &mut stream, &request, socket);
    }
    match NvsTls::over(stream, &parts.host) {
        Ok(mut tls) => exchange(call, &mut tls, &request, socket),
        // A name or a certificate this build will not accept is settled: the
        // module doc's second paragraph is why only one of these two shapes is
        // handed back for another attempt.
        Err(err) if matches!(err.kind(), ErrorKind::InvalidData | ErrorKind::InvalidInput) => {
            Err(Fault::thrown(format!(
                "{}: the TLS handshake with `{}` was refused and nothing was sent — {err}",
                call.member, parts.host
            )))
        }
        Err(err) => Ok(Attempt::Failed(format!(
            "the TLS handshake with {socket} failed: {err}"
        ))),
    }
}

/// The request out and the reply back, over whatever is already connected.
///
/// Generic over the stream so `http` and `https` share one copy of the framing,
/// the ceiling and the failure split — the plaintext side of a TLS session is
/// the same `Read` and `Write` as a bare socket, which is the whole reason
/// `nvs-host` exposes it that way.
fn exchange(
    call: &Call<'_>,
    stream: &mut (impl Read + Write),
    request: &str,
    socket: SocketAddr,
) -> Result<Attempt, Fault> {
    if let Err(err) = stream.write_all(request.as_bytes()) {
        return Ok(Attempt::Failed(format!(
            "sending to {socket} failed: {err}"
        )));
    }
    if let Some(body) = &call.body
        && let Some(err) = write_body(body, stream, call.member)?
    {
        return Ok(Attempt::Failed(format!(
            "sending to {socket} failed: {err}"
        )));
    }
    if let Err(err) = stream.flush() {
        return Ok(Attempt::Failed(format!(
            "sending to {socket} failed: {err}"
        )));
    }

    let mut raw = Vec::new();
    let mut buffer = [0_u8; 8192];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                raw.extend_from_slice(&buffer[..read]);
                if raw.len() > REPLY_CEILING {
                    return Err(Fault::thrown(format!(
                        "{}: the reply passed {REPLY_CEILING} bytes, which is as much of one \
                         another host is allowed to make this process hold",
                        call.member
                    )));
                }
            }
            Err(err) if err.kind() == ErrorKind::Interrupted => {}
            Err(err) => return Ok(Attempt::Failed(format!("reading {socket} failed: {err}"))),
        }
    }
    parse(&raw, call.member).map(Attempt::Answered)
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
        "\r\nAccept: */*\r\nConnection: close\r\n"
    ));
    if let Some(key) = &call.idempotency_key {
        field(&mut out, "Idempotency-Key", key, call.member)?;
    }
    // `rule:observability/an-outbound-call-propagates-traceparent`. Skipped where the caller wrote its own: two `traceparent`
    // headers are what the W3C format says to treat as no header at all, so
    // sending both would end the trace here rather than continue it.
    if let Some(traceparent) = &call.traceparent
        && !call
            .headers
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
    for (name, value) in &call.headers {
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
    let tls = scheme.eq_ignore_ascii_case("https");
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

/// The `Location` of a reply that is a redirect, or `None` for one that is not.
fn redirect_of(reply: &Reply) -> Option<String> {
    matches!(reply.status, 301 | 302 | 303 | 307 | 308)
        .then(|| header(reply, "location"))
        .flatten()
        .map(str::to_owned)
}

/// § 6's roster, and nothing else: a `400` is an answer and retrying it is a
/// load generator.
fn retryable(status: i64) -> bool {
    matches!(status, 429 | 502 | 503 | 504)
}

/// A `Retry-After` in seconds, which replaces the computed backoff.
///
/// The HTTP-date form is not read: it needs a clock agreement this module does
/// not have, and a header it cannot parse leaves the jittered backoff in place,
/// which is the safe direction rather than the fast one.
fn retry_after(reply: &Reply) -> Option<Duration> {
    header(reply, "retry-after")
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(Duration::from_secs)
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
fn header<'a>(reply: &'a Reply, name: &str) -> Option<&'a str> {
    reply
        .headers
        .iter()
        .find(|(held, _)| held == name)
        .map(|(_, value)| value.as_str())
}

/// The reply bytes, as a status, a header list and a decoded body.
fn parse(raw: &[u8], member: &str) -> Result<Reply, Fault> {
    let malformed = |why: &str| {
        Fault::thrown(format!(
            "{member}: the other end answered with something that is not an HTTP reply — {why}"
        ))
    };
    let end = find(raw, b"\r\n\r\n").ok_or_else(|| malformed("no header section ended it"))?;
    let head = std::str::from_utf8(&raw[..end])
        .map_err(|_| malformed("its header section is not text"))?;

    let mut lines = head.split("\r\n");
    let status_line = lines.next().unwrap_or_default();
    let mut fields = status_line.splitn(3, ' ');
    if !fields.next().unwrap_or_default().starts_with("HTTP/") {
        return Err(malformed("its first line is not a status line"));
    }
    let status: i64 = fields
        .next()
        .and_then(|code| code.parse().ok())
        .ok_or_else(|| malformed("its status line carries no status code"))?;

    let headers: Vec<(String, String)> = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_owned()))
        .collect();

    let reply = Reply {
        status,
        body: Vec::new(),
        headers,
    };
    let rest = &raw[end + 4..];
    let body = if header(&reply, "transfer-encoding")
        .is_some_and(|value| value.to_ascii_lowercase().contains("chunked"))
    {
        dechunk(rest, &malformed)?
    } else if let Some(length) =
        header(&reply, "content-length").and_then(|value| value.trim().parse::<usize>().ok())
    {
        rest.get(..length.min(rest.len()))
            .unwrap_or_default()
            .to_vec()
    } else {
        rest.to_vec()
    };

    Ok(Reply { body, ..reply })
}

/// A chunked body, joined.
fn dechunk(mut rest: &[u8], malformed: &dyn Fn(&str) -> Fault) -> Result<Vec<u8>, Fault> {
    let mut out = Vec::new();
    loop {
        let end = find(rest, b"\r\n").ok_or_else(|| malformed("a chunk header never ended"))?;
        let header = std::str::from_utf8(&rest[..end])
            .map_err(|_| malformed("a chunk header is not text"))?;
        let size = usize::from_str_radix(header.split(';').next().unwrap_or_default().trim(), 16)
            .map_err(|_| malformed("a chunk header is not a hexadecimal length"))?;
        rest = &rest[end + 2..];
        if size == 0 {
            return Ok(out);
        }
        if rest.len() < size + 2 {
            return Err(malformed("a chunk is shorter than its own header said"));
        }
        out.extend_from_slice(&rest[..size]);
        rest = &rest[size + 2..];
    }
}

/// Where `needle` starts in `haystack`.
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::{Call, Reply, backoff, dechunk, parse, send};
    use nvs_host::reactor::{Reactor, install, run_until_idle, with_current};
    use nvs_host::scheduler::Scheduler;
    use nvs_runtime::{Ctx, Fault, OutputSink, TaskRoot};
    use std::cell::RefCell;
    use std::io::{Read, Write};
    use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
    use std::rc::Rc;
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    /// A listener on loopback that answers each connection with the next of
    /// `replies`, verbatim, and hands back what it was asked.
    ///
    /// Loopback and a `std` listener on purpose: what is under test is the
    /// transport, and the address it is handed has already been through the
    /// door. Nothing here needs a `Ctx`.
    fn origin(replies: Vec<&'static str>) -> (SocketAddr, std::thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("a loopback port");
        let at = listener.local_addr().expect("its own address");
        let served = std::thread::spawn(move || {
            let mut asked = Vec::new();
            for reply in replies {
                let (mut stream, _) = listener.accept().expect("a connection");
                let mut request = [0_u8; 4096];
                let read = stream.read(&mut request).expect("a request");
                asked.push(String::from_utf8_lossy(&request[..read]).into_owned());
                stream.write_all(reply.as_bytes()).expect("a reply");
                stream.flush().expect("a flushed reply");
            }
            asked
        });
        (at, served)
    }

    /// A call to `at` with one attempt, no redirects and a generous deadline.
    fn call<'a>(at: SocketAddr, member: &'a str) -> Call<'a> {
        Call {
            member,
            verb: "GET",
            url: format!("http://{at}/ok"),
            address: at.ip(),
            deadline: Instant::now() + Duration::from_secs(10),
            connect_timeout: Duration::from_secs(5),
            headers: Vec::new(),
            redirects: 0,
            attempts: 1,
            backoff: Duration::from_millis(1),
            idempotency_key: None,
            body: None,
            traceparent: None,
        }
    }

    /// What a redirect hop must never be asked for.
    fn never(_url: &str) -> Result<IpAddr, Fault> {
        panic!("a call with no redirect hop must not re-pin")
    }

    /// The two slots a `Core\Http\Response` holds are what a real exchange
    /// fills: the status line's code, and the body the framing said was there.
    #[test]
    fn a_reply_becomes_the_status_and_the_body() {
        let (at, served) = origin(vec!["HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok"]);
        let reply = send(&call(at, "test"), &mut never).expect("an answer");
        assert_eq!(reply.status, 200);
        assert_eq!(reply.body, b"ok");

        let asked = served.join().expect("the origin thread");
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

        let asked = served.join().expect("the origin thread");
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
        let refused = send(&followed, &mut |url| {
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
        assert_eq!(served.join().expect("the origin thread").len(), 2);

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

    /// A chunked body is joined before it is a `string`: the framing is the
    /// transport's question and no reader asks it a second time.
    #[test]
    fn a_chunked_body_is_joined_before_it_is_a_string() {
        let raw = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n\
                    2\r\nok\r\n3\r\n ay\r\n0\r\n\r\n";
        let reply = parse(raw, "test").expect("a chunked reply");
        assert_eq!(reply.body, b"ok ay");
    }

    /// A body that is not UTF-8 arrives whole and is not repaired here —
    /// whether it is text is `Core\Http\Response::text`'s question
    /// (`rule:errors/ambiguous-input-refused`), and a reply the program only
    /// wanted the octets of must not fail on the way to `bytes()`.
    #[test]
    fn a_body_that_is_not_text_arrives_whole() {
        let mut raw = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n".to_vec();
        raw.extend_from_slice(&[0xff, 0xfe]);
        let reply = parse(&raw, "test").expect("bytes that are not text");
        assert_eq!(reply.body, [0xff, 0xfe]);
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
        let malformed = |why: &str| Fault::thrown(format!("test: {why}"));
        assert!(dechunk(b"zz\r\nok\r\n0\r\n\r\n", &malformed).is_err());
    }

    /// The header lookup is case-insensitive because HTTP field names are.
    #[test]
    fn a_header_is_found_whatever_case_the_origin_wrote_it_in() {
        let raw = b"HTTP/1.1 301 Moved\r\nLOCATION: /next\r\nContent-Length: 0\r\n\r\n";
        let reply: Reply = parse(raw, "test").expect("a redirect");
        assert_eq!(super::redirect_of(&reply).as_deref(), Some("/next"));
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
}
