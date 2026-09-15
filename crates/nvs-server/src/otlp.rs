//! `rule:observability/the-exporters-are-crates`'s push half: the spans a
//! sampled request derived, encoded as OTLP and posted to the collector
//! `[trace] endpoint` names.
//!
//! # Why the encoder is here
//!
//! The same reading that put the exposition format in [`crate::prometheus`]
//! puts this one here (`docs/decisions/0186.md` § *Investigation*):
//! `opentelemetry-otlp` fails
//! `rule:observability/an-exporter-brings-no-second-scheduler-and-no-second-client`'s
//! predicate — it brings a runtime and a client of its own — and what is left
//! once those are refused is a protobuf encoder of a few dozen bytes per span
//! and one HTTP request. That is this module, written against the OTLP
//! specification's `trace/v1` messages, and it changes nothing above it:
//! [`crate::trace::spans`] derives the graph whether or not anybody ships it.
//!
//! # The queue is the whole of what a request touches
//!
//! A request hands its spans to [`queue`] — through [`crate::trace::record`],
//! which is the one caller and the one place a door and a scheduled run share —
//! and is finished with them: a lock, a push and a stamp, with no socket, no
//! encoder and no collector anywhere on that path. Everything else — encoding, dialling, waiting on an answer —
//! happens on the drain task [`push_queued_on_this_core`] runs, so a collector
//! that is slow or gone is a collector that is slow or gone and never a
//! response that took longer.
//!
//! The queue is bounded ([`QUEUE_CEILING`]) because the alternative is a
//! process whose memory is a function of how far behind its collector is, which
//! is `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect` in the
//! shape a telemetry path takes it. Past the bound the **oldest** waiting trace
//! is dropped and counted: the spans in front of a full queue are the stale
//! ones, and a trace an operator can still act on is worth more than the one
//! before it.
//!
//! # A collector that is not there
//!
//! A batch that cannot be delivered is **dropped and counted**
//! ([`Pending::dropped`]), never retried and never held back. A retry queue is
//! a second unbounded store wearing a bound's name, and a telemetry path that
//! defends itself by growing is the one failure mode a server cannot afford.
//! What an operator gets instead is the count, and one note on each transition
//! between a collector that answers and one that does not — not one per batch,
//! which is a disk filled by a collector being down.
//!
//! # What it spends
//!
//! Process-wide: one queue, at most [`QUEUE_CEILING`] spans, plus one encoded
//! batch and one connection while a push is in flight. Per request: nothing but
//! the [`crate::trace::spans`] the request already held, moved into the queue
//! rather than copied. Nothing here grows with requests served, which is
//! `rule:programs/memory-priority`'s per-process reading of what an exporter
//! costs.
//!
//! # Known gaps
//!
//! 1. **A span has no window.** [`crate::trace::Span`] carries no timestamps
//!    because [`nvs_runtime::TraceEvent`] carries none: when a call entered and
//!    left is the timeline sink's, and the shared epoch that would put a
//!    monotonic reading on the wall clock is M10's. Every span is encoded as an
//!    instant at the moment its request handed the spans over, so a trace lands
//!    in the right place on a collector's clock and shows no duration.
//!    — owner: M10

use std::collections::VecDeque;
use std::fmt;
use std::future::Future;
use std::io;
use std::net::{SocketAddr, ToSocketAddrs};
use std::pin::{Pin, pin};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::task::{Context, Poll};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use hyper::body::{Body, Bytes, Frame, SizeHint};
use hyper::client::conn::http1;
use hyper::header::{self, HeaderValue};
use hyper::{Request, StatusCode};
use nvs_config::server::Waits;
use nvs_host::{NvsConnection, NvsTcp};
use nvs_runtime::host::Woken;

use crate::io::ConnectionIo;
use crate::serve::Draining;
use crate::trace::{Span, SpanKind};

/// The most spans this process holds waiting to be pushed.
///
/// A count rather than a share of anything, for [`crate::trace::SPAN_CEILING`]'s
/// reason: the worst case is arithmetic an operator can do, and it is this many
/// spans and the strings in them. A collector that keeps up leaves the queue at
/// or near empty, so the bound is what a *stall* costs and not what tracing
/// does.
pub const QUEUE_CEILING: usize = 8 * 1024;

/// The most spans one push carries.
///
/// One request body per batch, so this is the encoded buffer's bound as much as
/// the request's. Smaller than [`QUEUE_CEILING`] on purpose: a queue that filled
/// during an outage drains in several pushes that a collector can refuse
/// individually, rather than in one the size of the whole backlog.
pub const BATCH: usize = 512;

/// How long the drain waits before asking an empty queue again.
///
/// The task parks for this rather than being woken by [`queue`], because a wake
/// per sampled request would put the exporter's bookkeeping back on the path
/// this module exists to keep clear of it. What it costs is this core waking
/// four times a second with nothing to do.
const IDLE: Duration = Duration::from_millis(250);

/// How long a collector has to accept the connection.
///
/// Fixed rather than configured, and short: an unreachable collector must not
/// hold the drain task for the platform's own connect timeout, which is minutes
/// and would leave the queue filling behind a socket nobody is going to answer.
const DIAL: Duration = Duration::from_secs(5);

/// The media type OTLP/HTTP's default encoding is sent as.
const PROTOBUF: &str = "application/x-protobuf";

/// The path OTLP/HTTP puts traces on, appended to an endpoint written as a base
/// URL.
const TRACES: &str = "/v1/traces";

/// The port OTLP/HTTP is served on where an endpoint names none.
const OTLP_PORT: u16 = 4318;

/// One sampled request's spans, stamped when the request handed them over.
///
/// The stamp is taken at [`queue`] rather than at the encode, because how long
/// a batch waited for a collector is not a property of the trace: two requests
/// that ran a second apart and shipped in one push are still a second apart.
#[derive(Clone, Debug)]
pub struct Recorded {
    /// The graph [`crate::trace::spans`] derived, root first.
    pub spans: Vec<Span>,
    /// When the request handed them over, in nanoseconds since the Unix epoch.
    pub at: u64,
}

/// What the process is holding for its collector: the queue and what it never
/// delivered.
///
/// A type rather than a pair of statics so that a caller can hold one of its
/// own — which is what the cases below do, and what keeps a test off the
/// process-wide queue [`queue`] hands out.
#[derive(Debug)]
pub struct Pending {
    /// The waiting traces, oldest first, and the span count they add up to.
    waiting: Mutex<Queued>,
    /// Spans this process recorded and never delivered, whatever the reason.
    dropped: AtomicU64,
    /// The bound, in spans.
    ceiling: usize,
}

/// The queue itself, behind one lock: the traces and the count they carry.
///
/// The count is kept beside the deque rather than summed on each push, because
/// the bound is checked on the request's own thread and a walk of the queue
/// there is the one cost this path is written not to have.
#[derive(Debug, Default)]
struct Queued {
    /// Oldest first, which is the end a full queue drops from.
    traces: VecDeque<Recorded>,
    /// The spans in `traces`, added up.
    spans: usize,
}

impl Pending {
    /// An empty queue bounded at `ceiling` spans.
    #[must_use]
    pub const fn new(ceiling: usize) -> Self {
        Self {
            waiting: Mutex::new(Queued {
                traces: VecDeque::new(),
                spans: 0,
            }),
            dropped: AtomicU64::new(0),
            ceiling,
        }
    }

    /// Takes `spans` for pushing, dropping the oldest traces where the bound
    /// needs the room.
    ///
    /// This is the request's half of the module and it does no work a request
    /// can feel: a stamp, a lock and a push, with the drops — where there are
    /// any — being a pop of a deque. An empty `spans` is a request that was not
    /// sampled and is not queued at all.
    pub fn hand_over(&self, spans: Vec<Span>) {
        if spans.is_empty() {
            return;
        }
        let at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| {
                u64::try_from(since.as_nanos()).unwrap_or(u64::MAX)
            });
        let arriving = spans.len();
        let mut waiting = match self.waiting.lock() {
            Ok(waiting) => waiting,
            // A poisoned lock is a panic in a drain that will not run again,
            // and a request is not the place to raise it: the trace is dropped
            // exactly as a full queue's would be.
            Err(_) => {
                self.count_dropped(arriving);
                return;
            }
        };
        // A single trace larger than the whole bound cannot be held, so it is
        // dropped whole rather than emptying the queue to not fit.
        if arriving > self.ceiling {
            self.count_dropped(arriving);
            return;
        }
        while waiting.spans + arriving > self.ceiling {
            let Some(oldest) = waiting.traces.pop_front() else {
                break;
            };
            waiting.spans -= oldest.spans.len();
            self.count_dropped(oldest.spans.len());
        }
        waiting.spans += arriving;
        waiting.traces.push_back(Recorded { spans, at });
    }

    /// The oldest traces adding up to at most `most` spans, taken out of the
    /// queue.
    ///
    /// At least one trace comes back where the queue is not empty, even when it
    /// alone carries more than `most`: a batch that cannot be formed is a queue
    /// that never drains.
    #[must_use]
    pub fn take(&self, most: usize) -> Vec<Recorded> {
        let mut batch = Vec::new();
        let Ok(mut waiting) = self.waiting.lock() else {
            return batch;
        };
        let mut spans = 0;
        while let Some(front) = waiting.traces.front() {
            if !batch.is_empty() && spans + front.spans.len() > most {
                break;
            }
            let Some(taken) = waiting.traces.pop_front() else {
                break;
            };
            spans += taken.spans.len();
            waiting.spans -= taken.spans.len();
            batch.push(taken);
        }
        batch
    }

    /// Counts `batch`'s spans as never delivered — a collector that refused
    /// them, or one that was not there.
    pub fn undelivered(&self, batch: &[Recorded]) {
        self.count_dropped(batch.iter().map(|trace| trace.spans.len()).sum());
    }

    /// Spans this process recorded and never delivered.
    #[must_use]
    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    /// Spans waiting to be pushed.
    #[must_use]
    pub fn waiting(&self) -> usize {
        self.waiting.lock().map_or(0, |waiting| waiting.spans)
    }

    /// Adds to the count, which saturates rather than wrapping: a number that
    /// went round to zero would read as a process that lost nothing.
    fn count_dropped(&self, spans: usize) {
        let spans = u64::try_from(spans).unwrap_or(u64::MAX);
        let mut running = self.dropped.load(Ordering::Relaxed);
        loop {
            let next = running.saturating_add(spans);
            match self.dropped.compare_exchange_weak(
                running,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return,
                Err(current) => running = current,
            }
        }
    }
}

/// The one queue a request hands its spans to.
///
/// Process-wide and not per core, which is the opposite of
/// `rule:observability/a-registry-is-per-core-and-nothing-reads-it`'s registry
/// and for the reason that rule gives: a registry is contended per *metric
/// write*, which is per statement, while this is contended once per sampled
/// request and read by one drain task. One queue is what makes that drain one
/// task and one connection rather than one per core.
#[must_use]
pub fn queue() -> &'static Pending {
    static QUEUE: OnceLock<Pending> = OnceLock::new();
    QUEUE.get_or_init(|| Pending::new(QUEUE_CEILING))
}

/// Where `[trace] endpoint` points, resolved.
///
/// The address is resolved once, by the boot that read the key, because
/// `to_socket_addrs` blocks and the drain runs on a core that is also serving
/// requests. `[trace]` is `System`-class
/// (`rule:observability/metrics-and-trace-blocks-are-system`), so the value in
/// force is the one the boot read either way.
#[derive(Clone, Debug)]
pub struct Endpoint {
    /// What the drain dials.
    address: SocketAddr,
    /// The authority as written, which is what goes in `Host` — a collector
    /// behind a name-based proxy is routed by that and not by the address it
    /// resolved to.
    authority: String,
    /// The request target, `/v1/traces` where the endpoint named no path.
    target: String,
}

impl Endpoint {
    /// What `written` names.
    ///
    /// An endpoint written as a base URL gets OTLP/HTTP's own `/v1/traces`
    /// appended, which is the specification's rule for a URL that is not
    /// signal-specific, and one that already names a path is dialled exactly as
    /// written.
    ///
    /// # Errors
    ///
    /// One line naming what is wrong with it: a scheme this build cannot speak,
    /// no host at all, a port that is not a number, or a name that does not
    /// resolve.
    pub fn of(written: &str) -> Result<Self, String> {
        let rest = match written.split_once("://") {
            Some(("http", rest)) => rest,
            // No TLS listener and no outbound TLS in this crate
            // (`rule:http-server/two-deployments-and-nothing-a-proxy-owns`):
            // what reaches a collector over TLS is a proxy on this host, which
            // is the same shape a production deployment already has in front of
            // it.
            Some(("https", _)) => {
                return Err(format!(
                    "`[trace] endpoint` names `{written}` and this server speaks no outbound TLS: \
                     push to a collector on this host, or to a local proxy that holds the \
                     certificate"
                ));
            }
            Some((scheme, _)) => {
                return Err(format!(
                    "`[trace] endpoint` names the scheme `{scheme}`, and OTLP over HTTP is the one \
                     this build pushes with: write an `http://` URL"
                ));
            }
            None => {
                return Err(format!(
                    "`[trace] endpoint` names `{written}`, which is not a URL: write one, as \
                     `http://127.0.0.1:{OTLP_PORT}`"
                ));
            }
        };
        let (authority, path) = match rest.find('/') {
            Some(at) => (&rest[..at], &rest[at..]),
            None => (rest, ""),
        };
        if authority.is_empty() {
            return Err(format!(
                "`[trace] endpoint` names `{written}`, which has no host in it"
            ));
        }
        let dialled = if authority.contains(':') {
            authority.to_owned()
        } else {
            format!("{authority}:{OTLP_PORT}")
        };
        let address = dialled
            .to_socket_addrs()
            .map_err(|error| {
                format!("`[trace] endpoint` names `{written}`, which does not resolve: {error}")
            })?
            .next()
            .ok_or_else(|| {
                format!("`[trace] endpoint` names `{written}`, which resolves to no address at all")
            })?;
        Ok(Self {
            address,
            authority: authority.to_owned(),
            target: if path.is_empty() || path == "/" {
                TRACES.to_owned()
            } else {
                path.to_owned()
            },
        })
    }

    /// The address a push dials.
    #[must_use]
    pub fn address(&self) -> SocketAddr {
        self.address
    }

    /// The request target a push posts to.
    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }
}

impl fmt::Display for Endpoint {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(out, "http://{}{}", self.authority, self.target)
    }
}

/// Pushes what [`queue`] holds to `endpoint` until the drain begins.
///
/// A task beside the accept loops and not a thread of its own
/// (`rule:concurrency/one-scheduler`), on the one core the boot armed it on: a
/// second pusher would be a second connection sending the same process's spans
/// in an order neither could state. It runs no Novis code and reads nothing but
/// the queue.
///
/// **A drain that has begun still pushes what is waiting.** The spans in the
/// queue at that moment are the last requests this process answered, and losing
/// exactly the traces from a restart is losing them where they are most often
/// wanted.
///
/// `report` is told once when the collector stops answering and once when it
/// starts again, and never per batch: a collector that is down for an hour
/// writes two lines rather than fourteen thousand.
pub fn push_queued_on_this_core(
    endpoint: &Endpoint,
    waits: Waits,
    draining: &Draining,
    mut report: impl FnMut(&str),
) {
    let pending = queue();
    let mut answering = true;
    loop {
        let stopping = draining.is_draining();
        let batch = pending.take(BATCH);
        if batch.is_empty() {
            if stopping {
                break;
            }
            if matches!(nvs_host::sleep(IDLE), Woken::Cancelled) {
                break;
            }
            continue;
        }
        match delivered(endpoint, waits, &batch) {
            Ok(()) => {
                if !answering {
                    report(&format!("the collector at {endpoint} is answering again"));
                    answering = true;
                }
            }
            Err(failed) => {
                pending.undelivered(&batch);
                if answering {
                    report(&format!(
                        "the collector at {endpoint} took no spans: {failed}. Spans are dropped \
                         and counted while it is away, not held"
                    ));
                    answering = false;
                }
            }
        }
        if stopping {
            break;
        }
    }
}

/// One batch, encoded and posted, and what the collector said about it.
///
/// # Errors
///
/// A collector that could not be reached, an exchange that failed, or a status
/// that is not a success — all of which mean the same thing to the caller, which
/// is that these spans are gone.
fn delivered(endpoint: &Endpoint, waits: Waits, batch: &[Recorded]) -> Result<(), String> {
    let stream = NvsTcp::connect_timeout(endpoint.address, DIAL)
        .map_err(|failed| format!("could not connect: {failed}"))?;
    let io = ConnectionIo::new(NvsConnection::Tcp(stream), waits);
    let posting = post(io, &endpoint.authority, &endpoint.target, encode(batch));
    let answered = nvs_host::block_on(posting)
        .ok_or_else(|| "the push task was cancelled mid-exchange".to_owned())?
        .map_err(|failed| format!("the exchange failed: {failed}"))?;
    if answered.is_success() {
        Ok(())
    } else {
        Err(format!("it answered {}", answered.as_u16()))
    }
}

/// One `POST` of `body` over `io`, and the status it was answered with.
///
/// Generic over the stream so that the exchange a core makes over a parking
/// socket and the one a case makes over a blocking one are the same code: what
/// differs is who drives the future, which is [`nvs_host::block_on()`] on a core.
///
/// The connection and the request are two futures that only make progress
/// beside each other — the dispatcher moves the bytes, the request is what waits
/// on the answer — so they are polled in one place rather than spawned, exactly
/// as `nvs ctl` does it on the other side of this crate. The body of the answer
/// is not read: OTLP's is a status message, and the one thing this drain does
/// with a rejection is count the spans it lost.
///
/// # Errors
///
/// Whatever `hyper` ended the exchange on — a collector that went away
/// mid-answer, or bytes that are not HTTP — and the connection's own failures.
pub async fn post<Io>(
    io: Io,
    authority: &str,
    target: &str,
    body: Vec<u8>,
) -> io::Result<StatusCode>
where
    Io: hyper::rt::Read + hyper::rt::Write + Unpin + 'static,
{
    let (mut sender, connection) = http1::handshake::<_, Payload>(io)
        .await
        .map_err(io::Error::other)?;
    let request = Request::builder()
        .method("POST")
        .uri(target)
        .header(
            header::HOST,
            HeaderValue::from_str(authority).map_err(io::Error::other)?,
        )
        .header(header::CONTENT_TYPE, HeaderValue::from_static(PROTOBUF))
        // One connection per batch: the pushes are seconds apart, a collector
        // holding an idle connection per origin is a cost it did not ask for,
        // and a connection kept across a park is one more thing to have gone
        // away without saying so.
        .header(header::CONNECTION, HeaderValue::from_static("close"))
        .body(Payload::of(body))
        .map_err(io::Error::other)?;

    let mut connection = pin!(connection);
    let mut asking = pin!(sender.send_request(request));
    // The connection ends of its own accord once the answer is on the wire, and
    // it ends *before* the answer has been taken out of it — so a finished
    // connection is not an error here, and polling it again after it finished
    // would panic. It is an error only where the request is still waiting.
    let mut ended = false;
    let answered = std::future::poll_fn(|cx: &mut Context<'_>| {
        if let Poll::Ready(answered) = asking.as_mut().poll(cx) {
            return Poll::Ready(answered.map_err(io::Error::other));
        }
        if ended {
            return Poll::Ready(Err(cut_short()));
        }
        match drove(connection.as_mut(), cx) {
            Ok(finished) => {
                ended = finished;
                Poll::Pending
            }
            Err(failed) => Poll::Ready(Err(failed)),
        }
    })
    .await?;
    Ok(answered.status())
}

/// One poll of the connection: whether it has finished, or the failure it
/// finished on.
fn drove<F>(connection: Pin<&mut F>, cx: &mut Context<'_>) -> io::Result<bool>
where
    F: Future<Output = hyper::Result<()>>,
{
    match connection.poll(cx) {
        Poll::Pending => Ok(false),
        Poll::Ready(Ok(())) => Ok(true),
        Poll::Ready(Err(failed)) => Err(io::Error::other(failed)),
    }
}

/// A connection that ended with the push still waiting on it.
fn cut_short() -> io::Error {
    io::Error::new(
        io::ErrorKind::UnexpectedEof,
        "the collector closed the connection with the push unanswered",
    )
}

/// The encoded batch, sent whole.
///
/// `hyper`'s client is generic over what it sends and this one has the bytes in
/// hand before the request exists, so the body is one frame and the exact size
/// is known — which is what puts a `Content-Length` on the wire rather than a
/// chunked encoding a collector would have to reassemble.
#[derive(Debug)]
pub struct Payload(Option<Bytes>);

impl Payload {
    /// The body `bytes` will be sent as.
    #[must_use]
    pub fn of(bytes: Vec<u8>) -> Self {
        Self(Some(Bytes::from(bytes)))
    }
}

impl Body for Payload {
    type Data = Bytes;
    type Error = io::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, io::Error>>> {
        Poll::Ready(self.0.take().map(|bytes| Ok(Frame::data(bytes))))
    }

    fn is_end_stream(&self) -> bool {
        self.0.is_none()
    }

    fn size_hint(&self) -> SizeHint {
        match &self.0 {
            Some(bytes) => SizeHint::with_exact(u64::try_from(bytes.len()).unwrap_or(u64::MAX)),
            None => SizeHint::with_exact(0),
        }
    }
}

/// `batch` as one OTLP `TracesData` message.
///
/// One `ResourceSpans` and one `ScopeSpans` however many requests are in the
/// batch: the resource is this process and the scope is this runtime, and both
/// are the same for every span a Novis server produces. A span carries its own
/// trace id, which is what keeps one push of several requests several traces on
/// the collector's side.
#[must_use]
pub fn encode(batch: &[Recorded]) -> Vec<u8> {
    let mut out = Vec::new();
    // `TracesData.resource_spans`.
    nested(&mut out, 1, |resource_spans| {
        // `ResourceSpans.resource`.
        nested(resource_spans, 1, resource);
        // `ResourceSpans.scope_spans`.
        nested(resource_spans, 2, |scope_spans| {
            // `ScopeSpans.scope`.
            nested(scope_spans, 1, scope);
            for trace in batch {
                for span in &trace.spans {
                    // `ScopeSpans.spans`.
                    nested(scope_spans, 2, |out| encoded(out, span, trace.at));
                }
            }
        });
    });
    out
}

/// `Resource`: what produced these spans, which is this process.
fn resource(out: &mut Vec<u8>) {
    attribute(out, 1, "service.name", "novis");
    attribute(out, 1, "telemetry.sdk.name", "novis");
    attribute(out, 1, "telemetry.sdk.language", "novis");
    attribute(out, 1, "telemetry.sdk.version", env!("CARGO_PKG_VERSION"));
}

/// `InstrumentationScope`: what instrumented them, which is this crate.
fn scope(out: &mut Vec<u8>) {
    text(out, 1, "nvs-server");
    text(out, 2, env!("CARGO_PKG_VERSION"));
}

/// One `KeyValue` attribute under `field`.
fn attribute(out: &mut Vec<u8>, field: u32, key: &str, value: &str) {
    nested(out, field, |pair| {
        // `KeyValue.key`, then `KeyValue.value`.
        text(pair, 1, key);
        // `AnyValue.string_value`.
        nested(pair, 2, |any| text(any, 1, value));
    });
}

/// One `Span`, as the OTLP message.
///
/// `at` is both ends of it, which is known gap 2 in the module doc rather than
/// a reading of the specification: what a span's window would be measured
/// against is not derived yet.
fn encoded(out: &mut Vec<u8>, span: &Span, at: u64) {
    raw(out, 1, &span.trace_id);
    raw(out, 2, &span.span_id);
    if let Some(parent) = span.parent {
        raw(out, 4, &parent);
    }
    text(out, 5, &span.detail);
    number(out, 6, kind(span.kind));
    fixed64(out, 7, at);
    fixed64(out, 8, at);
}

/// Which `SpanKind` of the specification's five one of
/// `rule:observability/four-kinds-become-a-span`'s four is.
///
/// The root is `SERVER` because the request arrived from a peer; a `query` and
/// an outbound call are `CLIENT` because this process is the one asking; a
/// `spawn` is `INTERNAL` because both ends of it are here.
fn kind(of: SpanKind) -> u64 {
    match of {
        SpanKind::Spawn => 1,
        SpanKind::Root => 2,
        SpanKind::Query | SpanKind::Http => 3,
    }
}

/// A length-delimited submessage under `field`, written by `write`.
///
/// The submessage is built in a buffer of its own because protobuf puts the
/// length in front of the bytes and there is no way to know it before writing
/// them. What that spends is one allocation per message on a task no request is
/// waiting on.
fn nested(out: &mut Vec<u8>, field: u32, write: impl FnOnce(&mut Vec<u8>)) {
    let mut inner = Vec::new();
    write(&mut inner);
    raw(out, field, &inner);
}

/// A length-delimited `bytes` field.
fn raw(out: &mut Vec<u8>, field: u32, value: &[u8]) {
    tag(out, field, 2);
    varint(out, u64::try_from(value.len()).unwrap_or(u64::MAX));
    out.extend_from_slice(value);
}

/// A length-delimited `string` field, which is the same thing carrying UTF-8.
fn text(out: &mut Vec<u8>, field: u32, value: &str) {
    raw(out, field, value.as_bytes());
}

/// A varint field — an enum or a count.
fn number(out: &mut Vec<u8>, field: u32, value: u64) {
    tag(out, field, 0);
    varint(out, value);
}

/// A `fixed64` field, which is how OTLP carries a nanosecond timestamp.
fn fixed64(out: &mut Vec<u8>, field: u32, value: u64) {
    tag(out, field, 1);
    out.extend_from_slice(&value.to_le_bytes());
}

/// The field number and wire type that introduce a field.
fn tag(out: &mut Vec<u8>, field: u32, wire: u8) {
    varint(out, (u64::from(field) << 3) | u64::from(wire));
}

/// A base-128 varint, least significant group first.
fn varint(out: &mut Vec<u8>, mut value: u64) {
    loop {
        let byte = (value & 0x7f).to_le_bytes()[0];
        value >>= 7;
        if value == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::mpsc;
    use std::task::{Context, Poll, Waker};
    use std::time::{Duration, Instant};

    use nvs_runtime::{TraceContext, TraceEvent, TraceKind};

    use super::{BATCH, Endpoint, Pending, encode, post};
    use crate::io::Nonblocking;
    use crate::trace::{Span, spans};

    /// A sampled inbound `traceparent`, so the derivation below produces a
    /// graph at all — `[trace] sample` is `0.0` in the fixture and an arrived
    /// sampled trace is continued regardless
    /// (`rule:observability/sampling-is-head-based`).
    const INBOUND: &str = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";

    /// The spans one sampled request contributes, as the door would derive
    /// them: the root and one span per kind that becomes one.
    fn derived() -> Vec<Span> {
        let trace = TraceContext::continuing(Some(INBOUND), 0.0);
        let events = [
            TraceEvent {
                kind: TraceKind::Call,
                callee: "Orders::total".to_owned(),
                status: None,
            },
            TraceEvent {
                kind: TraceKind::Query,
                callee: "select 1 driver=sqlite rows=1".to_owned(),
                status: None,
            },
            TraceEvent {
                kind: TraceKind::Http,
                callee: "GET https://api.example.test/v1/pay status=200 attempt=3".to_owned(),
                status: None,
            },
        ];
        spans("GET /orders/17", &trace, &events)
    }

    /// The payloads of every length-delimited `field` at the top level of
    /// `message`, with every other field skipped by its wire type.
    ///
    /// A reader rather than a decoder: what these cases assert is the shape the
    /// encoder claims to write — one resource, one scope, one span per derived
    /// span — and a full protobuf decoder would be asserting a crate's reading
    /// of it instead of ours.
    fn fields(message: &[u8], field: u32) -> Vec<&[u8]> {
        let mut found = Vec::new();
        let mut at = 0;
        while at < message.len() {
            let (tag, read) = varint(message, at);
            at = read;
            let (number, wire) = (u32::try_from(tag >> 3).unwrap_or(0), tag & 7);
            match wire {
                0 => at = varint(message, at).1,
                1 => at += 8,
                2 => {
                    let (len, read) = varint(message, at);
                    let len = usize::try_from(len).unwrap_or(0);
                    at = read;
                    if number == field {
                        found.push(&message[at..at + len]);
                    }
                    at += len;
                }
                5 => at += 4,
                _ => break,
            }
        }
        found
    }

    /// One varint out of `message` at `at`: its value, and where the next field
    /// starts.
    fn varint(message: &[u8], mut at: usize) -> (u64, usize) {
        let mut value = 0u64;
        let mut shift = 0;
        while at < message.len() {
            let byte = message[at];
            at += 1;
            value |= u64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                break;
            }
            shift += 7;
        }
        (value, at)
    }

    /// A collector that takes one push: what it was sent, and a `200` back.
    ///
    /// The whole request head and body come back over the channel, because what
    /// the cases are pinning is what went on the wire — the target, the media
    /// type and the message — and not what this crate thinks it sent.
    fn collector() -> (String, mpsc::Receiver<(String, Vec<u8>)>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let address = listener
            .local_addr()
            .expect("the bound address")
            .to_string();
        let (sent, received) = mpsc::channel();
        std::thread::spawn(move || {
            let (stream, _) = listener.accept().expect("the push");
            let mut reading = BufReader::new(stream);
            let mut head = String::new();
            let mut length = 0;
            loop {
                let mut line = String::new();
                if reading.read_line(&mut line).unwrap_or(0) == 0 {
                    break;
                }
                if let Some(written) = line
                    .to_ascii_lowercase()
                    .strip_prefix("content-length:")
                    .map(str::trim)
                    .and_then(|written| written.parse::<usize>().ok())
                {
                    length = written;
                }
                head.push_str(&line);
                if line == "\r\n" {
                    break;
                }
            }
            let mut body = vec![0; length];
            reading.read_exact(&mut body).expect("the whole body");
            reading
                .get_mut()
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\nconnection: close\r\n\r\n")
                .expect("the answer");
            drop(sent.send((head, body)));
        });
        (address, received)
    }

    /// Drives `post` over a blocking socket on this thread, the way `nvs ctl`
    /// drives its own exchange: there is no reactor here and nothing to wake
    /// this loop but itself, so a poll that moved nothing sleeps a millisecond
    /// and asks again.
    fn pushed(stream: TcpStream, authority: &str, target: &str, body: Vec<u8>) -> u16 {
        stream.set_nonblocking(true).expect("a non-blocking socket");
        let driven = Nonblocking::new(stream);
        let moved = driven.moved();
        let mut exchange = Box::pin(post(driven, authority, target, body));
        let mut context = Context::from_waker(Waker::noop());
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Poll::Ready(answered) = exchange.as_mut().poll(&mut context) {
                return answered.expect("the collector answered").as_u16();
            }
            assert!(Instant::now() < deadline, "the push never finished");
            if !moved.replace(false) {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }

    /// `rule:observability/the-exporters-are-crates`'s push half, end to end:
    /// what a sampled request derived arrives at the endpoint as one OTLP
    /// message, at the target and media type the specification names, carrying
    /// one span per derived span and one trace id across all of them.
    ///
    /// The shape is what this asserts rather than the bytes: one `ResourceSpans`
    /// and one `ScopeSpans` however many requests a batch holds, because a
    /// collector reads the trace out of the spans and not out of the nesting.
    #[test]
    fn a_sampled_request_is_pushed_to_the_otlp_endpoint_as_one_trace() {
        let derived = derived();
        assert_eq!(derived.len(), 3, "the fixture's own derivation moved");
        let pending = Pending::new(64);
        pending.hand_over(derived.clone());
        let batch = pending.take(BATCH);
        assert_eq!(batch.len(), 1, "one request is one trace in the batch");
        assert_eq!(pending.waiting(), 0, "the batch left the queue");

        let (address, received) = collector();
        let endpoint = Endpoint::of(&format!("http://{address}")).expect("the endpoint");
        assert_eq!(
            endpoint.target(),
            "/v1/traces",
            "a base URL takes OTLP/HTTP's own path"
        );
        let stream = TcpStream::connect(endpoint.address()).expect("the collector");
        let status = pushed(stream, &address, endpoint.target(), encode(&batch));
        assert_eq!(status, 200);

        let (head, body) = received
            .recv_timeout(Duration::from_secs(10))
            .expect("the push arrived");
        let first = head.lines().next().unwrap_or_default();
        assert_eq!(first, "POST /v1/traces HTTP/1.1", "head: {head}");
        assert!(
            head.to_ascii_lowercase()
                .contains("content-type: application/x-protobuf"),
            "head: {head}"
        );

        let resources = fields(&body, 1);
        assert_eq!(resources.len(), 1, "one resource, which is this process");
        let scopes = fields(resources[0], 2);
        assert_eq!(scopes.len(), 1, "one scope, which is this runtime");
        let encoded = fields(scopes[0], 2);
        assert_eq!(
            encoded.len(),
            derived.len(),
            "a derived span did not reach the wire"
        );
        for (span, sent) in derived.iter().zip(&encoded) {
            assert_eq!(fields(sent, 1), [&span.trace_id[..]], "one trace, not two");
            assert_eq!(fields(sent, 2), [&span.span_id[..]]);
            assert_eq!(
                fields(sent, 5),
                [span.detail.as_bytes()],
                "the span is not of what it was derived from"
            );
        }
    }

    /// `rule:programs/memory-priority`'s bound on the queue and the module
    /// doc's § *A collector that is not there*, together: what cannot be
    /// delivered is counted and dropped, and neither the drop nor the collector
    /// being gone is on the path a request takes.
    ///
    /// The timing half is generous on purpose. What it rules out is a hand-over
    /// that dials, encodes or waits on anything — every one of which costs
    /// whole seconds against a collector that is not answering — and not the
    /// scheduling of a loaded machine.
    #[test]
    fn an_unreachable_collector_drops_and_counts_spans_and_never_delays_a_response() {
        let derived = derived();
        let spans = derived.len();

        // A queue that cannot hold three traces of this size drops the oldest
        // to make room for the newest, and counts what it dropped.
        let pending = Pending::new(spans * 2);
        for _ in 0..3 {
            pending.hand_over(derived.clone());
        }
        assert_eq!(pending.waiting(), spans * 2, "the bound did not hold");
        assert_eq!(
            pending.dropped(),
            u64::try_from(spans).expect("a span count"),
            "the oldest trace was dropped and not counted"
        );

        // An address nothing is listening on, which is what a collector that
        // has gone away looks like from here.
        let closed = {
            let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
            listener.local_addr().expect("the bound address")
        };
        let endpoint = Endpoint::of(&format!("http://{closed}")).expect("the endpoint");
        assert!(
            TcpStream::connect(endpoint.address()).is_err(),
            "something answered on a port that was just closed"
        );
        let batch = pending.take(BATCH);
        let held = pending.dropped();
        pending.undelivered(&batch);
        assert_eq!(
            pending.dropped() - held,
            u64::try_from(spans * 2).expect("a span count"),
            "an undelivered batch's spans were not counted"
        );

        // And the request's own half: handing spans over touches the queue and
        // nothing else, so it costs the same whether or not anything is
        // answering at the endpoint.
        let started = Instant::now();
        for _ in 0..1000 {
            pending.hand_over(derived.clone());
        }
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "handing spans over waited on something: {:?}",
            started.elapsed()
        );
    }

    /// The endpoint grammar: a base URL takes OTLP/HTTP's own path, one that
    /// names a path keeps it, a missing port is the protocol's own, and the two
    /// schemes this build cannot push over are refused where they are written
    /// rather than at the first batch.
    #[test]
    fn an_endpoint_is_a_base_url_and_a_scheme_this_build_speaks() {
        let base = Endpoint::of("http://127.0.0.1:4318").expect("a base URL");
        assert_eq!(base.target(), "/v1/traces");
        assert_eq!(base.address().port(), 4318);

        let named = Endpoint::of("http://127.0.0.1:4318/otlp/v1/traces").expect("a written path");
        assert_eq!(named.target(), "/otlp/v1/traces");

        let bare = Endpoint::of("http://127.0.0.1").expect("no port");
        assert_eq!(bare.address().port(), 4318, "OTLP/HTTP's own port");

        for refused in [
            "https://collector.example.test",
            "grpc://127.0.0.1:4317",
            "127.0.0.1:4318",
        ] {
            let why = Endpoint::of(refused).expect_err(refused);
            assert!(why.contains("`[trace] endpoint`"), "{why}");
        }
    }

    /// A trace larger than the whole queue is dropped whole rather than
    /// emptying it to not fit, which is the one case the pop-the-oldest loop
    /// could otherwise spin on.
    #[test]
    fn a_trace_larger_than_the_queue_is_dropped_without_emptying_it() {
        let derived = derived();
        let pending = Pending::new(derived.len());
        pending.hand_over(derived.clone());
        let mut big = derived.clone();
        big.extend(derived.clone());
        pending.hand_over(big);
        assert_eq!(pending.waiting(), derived.len(), "the queue was emptied");
        assert_eq!(
            pending.dropped(),
            u64::try_from(derived.len() * 2).expect("a span count")
        );
        assert_eq!(
            pending.take(BATCH).len(),
            1,
            "the first trace is still here"
        );
    }

    /// An empty derivation is an unsampled request, and it is not a queue entry:
    /// the queue holds traces somebody is recording.
    #[test]
    fn an_unsampled_request_queues_nothing() {
        let pending = Pending::new(16);
        pending.hand_over(Vec::new());
        assert_eq!(pending.waiting(), 0);
        assert_eq!(pending.dropped(), 0);
        assert!(pending.take(BATCH).is_empty());
    }
}
