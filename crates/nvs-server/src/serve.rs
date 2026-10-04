//! The accept loop: one listening socket, one coroutine per connection, and one
//! `hyper` connection future driven on that coroutine's own stack.
//!
//! `rule:concurrency/one-future-per-connection`
//! is the seam and [`crate::io`] is the adapter; this module is what puts a
//! socket on either end of them. Its whole shape is three lines: accept, spawn
//! a child task, and [`nvs_host::block_on()`] the connection on that child.
//! Nothing is queued, nothing is polled centrally, and there is no executor —
//! a connection is a coroutine, and a coroutine is what the scheduler already
//! has.
//!
//! # One connection per coroutine, and the accept loop is a task too
//!
//! [`serve_on_this_core`] runs *as a task*, so every connection it accepts is
//! its child ([`nvs_host::spawn_child`], `rule:concurrency/a-child-belongs-to-the-calling-task`'s "each is a child of the
//! calling task"). That is not a convenience: it is what makes a connection
//! cancellable with the server, and it is the tree
//! `rule:concurrency/nothing-is-still-running-when-a-call-returns`
//! reads when it has to prove nothing is still running. A loop that spawned
//! roots would have to grow its own registry of live connections and its own
//! shutdown, both of which the task tree already is.
//!
//! **What it spends**, per `rule:programs/memory-priority`:
//! one coroutine stack and one `hyper` connection state per connection being
//! served, plus the accepting task's own, plus — while a request is actually
//! running on one of them — that request's isolate, which is one `Ctx` and one
//! pooled task stack under
//! `rule:security/isolate-budget-is-the-trees`
//! 's accounting. O(in-flight) at both levels: nothing is held per connection
//! already closed or per request already answered.
//!
//! # What this loop does not decide, and who does
//!
//! - **No routing inside this loop.** The handler is still the caller's
//!   function; what [`crate::mount`] gives it is
//!   `rule:http-server/a-request-resolves-in-five-steps`
//!   's five steps to answer with, and [`Reply`] is the two things this loop
//!   can do with one. Nothing here reads a path from request bytes — that
//!   module's docs own the one place a remainder meets a filesystem, and § 2's
//!   rule with it.
//! - **No file policy of its own, for a selection or for a program.** A
//!   [`crate::mount::What::Static`] selection is answered in full by
//!   [`crate::statics`] — § 4's `ETag`, `Range` and MIME policy, one policy in
//!   both deployments — and this loop only carries the [`Reply`] back. A
//!   request that *ran* and named a file with `Core\Response::sendFile` reaches
//!   that same policy through `sent`, so the two deployments and the two ways
//!   of choosing a file are one answer. What a *program's* own response may say
//!   is the next paragraph.
//! - **No response policy beyond a status.** A request that ran answers `200`
//!   carrying what it echoed, and one that did not answers `500` carrying
//!   nothing; `answer`'s own docs are the home of that second call.
//!   `rule:errors/renderings`'s rendering of a failure into a development response is the
//!   configuration slice's, because a mode is what decides it and this loop has
//!   not been given one.
//!   `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect` is not
//!   on that list: § 1's header set is filled into every response this loop
//!   writes ([`crate::secure`]) and § 2's closed CORS refuses a preflight above
//!   the handler ([`crate::cors`]), neither of which a mode changes.
//! - **The accept loop backs off.** `rule:http-server/the-server-block-is-boot-class`'s last process-wide bound, as
//!   `rule:http-server/the-accept-loop-backs-off`
//!   states it: descriptor exhaustion is the one `accept` failure the next
//!   iteration recovers from, so it is the one this loop waits out instead of
//!   ending on, and the one it logs once per window instead of once per
//!   attempt. [`AcceptBackoff`] is both halves and nothing else in this file
//!   knows the condition. `max_in_flight` is here too: [`crate::admit`] is the
//!   arithmetic and the counter, and the refusal is taken in the service below
//!   *before* the handler is asked for a [`Reply`], which is the order § 5 makes
//!   the whole point of the valve. The four waits are here too:
//!   [`crate::io::Phase`] is which one is in force, and the connection loop
//!   below is what moves it.
//! - **One core.** The name says so: a listener bound once and handed to several
//!   cores through [`nvs_host::NvsListener::from_std`] is the fan-out, and it is
//!   the same loop on each of them.

use std::cell::{Cell, RefCell};
use std::convert::Infallible;
use std::io;
use std::ops::ControlFlow;
use std::path::Path;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{Arc, Mutex, PoisonError, RwLock};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use hyper::body::{Body, Bytes, Frame, Incoming, SizeHint};
use hyper::header::{self, HeaderName, HeaderValue};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{HeaderMap, Request, Response, StatusCode};
use nvs_config::Waits;
use nvs_host::{
    Completion, Isolate, NvsConnection, NvsListener, Output, Running, Waiting, Wake, block_on,
    spawn_child, suspend_current,
};
use nvs_runtime::host::Woken;
use nvs_runtime::stream;
use nvs_runtime::{Ctx, Drain, OutputSink, TaskRoot};

use crate::ConnectionIo;
use crate::admit::Admission;
use crate::body::Supply;
use crate::cors::Cors;
use crate::forwarded::{Arrival, Origin, Trusted};
use crate::io::Phase;
use crate::mount::OnDisk;
use crate::secure::{Scheme, Secure};
use crate::statics;

/// A response body: one this server already holds in full, or one an isolate
/// writes over time.
///
/// A type of ours rather than `http-body-util`'s `Full` and its streaming
/// siblings, and that is a dependency not taken rather than a wheel reinvented.
/// Both shapes are decided before they reach here: a whole body is the output
/// an isolate produced (`rule:tooling/echo-always-has-a-sink`
/// 's table binds `echo` to the response body), a buffer the runtime hands over
/// whole; a streamed one is [`nvs_runtime::stream::Drain`], whose wake pair and
/// one-chunk-in-flight bound are that module's and not a generic stream's. What
/// an adapter crate would want is a `futures` `Stream` to adapt *from*, which
/// neither arm is.
///
/// The two arms differ on the wire in exactly one place, and it is
/// [`Body::size_hint`]. A whole body reports its exact length, which is what
/// makes `hyper` send a `Content-Length`; a stream reports no exact size, which
/// is what makes `hyper` chunk it.
#[derive(Debug)]
pub enum Answer {
    /// The bytes are already here, and go out as one frame.
    Whole(Option<Bytes>),
    /// The bytes arrive while the response is being sent —
    /// `rule:concurrency/a-stream-that-outlives-its-request-is-a-connection`'s
    /// two spellings, which differ in whose isolate holds the writing half and
    /// not in what this end does with it.
    Streaming {
        /// Where the writing isolate's chunks arrive.
        drain: stream::Drain,
        /// The bounds an event stream is held inside — its keep-alive clock
        /// and its lifetime. `None` for a request-scoped stream, and that is
        /// the line the two spellings differ on here: a streamed response is
        /// bounded by the request writing it
        /// (`rule:http-server/a-requests-blast-radius-is-bounded-at-four-tiers`),
        /// so a program that goes quiet inside one is a program the request's
        /// own ceiling is already answering for. An event stream outlives
        /// every ceiling but the connection's, and quiet is the state it is
        /// designed to spend most of its life in.
        alive: Option<crate::bounds::EventStream>,
        /// The drain period a request-scoped stream ends at, and `None` for an
        /// event stream, whose `alive` ends it under a drain instead.
        cut: Option<DrainCut>,
    },
}

/// The end of a request-scoped stream's drain period
/// (`rule:concurrency/a-drain-closes-a-connection-cleanly`): the connection's
/// own period, read off the same cell the adapter's waits and the service's
/// cut read, so all three end at one instant.
///
/// `due` is the connection loop's `cut_due`, filed on every poll so the loop
/// wakes at the period's end (`crate::bounds::wake_at` owns why the wake is the
/// loop's). It is emptied when the body is dropped, so a stream that ended
/// leaves no deadline behind for the next request.
#[derive(Debug)]
pub struct DrainCut {
    draining: Draining,
    seen: crate::io::DrainSeen,
    period: Duration,
    due: Rc<Cell<Option<Instant>>>,
}

impl DrainCut {
    /// Whether the period has ended. Files its end for the loop's wake when a
    /// drain has begun, and nothing when none has.
    fn reached(&self) -> bool {
        let at = self
            .draining
            .is_draining()
            .then(|| self.seen.period_ends(self.period));
        self.due.set(at);
        at.is_some_and(|at| at <= Instant::now())
    }
}

impl Drop for DrainCut {
    fn drop(&mut self) {
        self.due.set(None);
    }
}

/// The error a request-scoped stream ends in when the drain period ends while
/// it is still being written.
///
/// An error rather than an end, because an end makes `hyper` write the
/// terminating chunk and a client would read a cut body as a whole one. On an
/// error `hyper` aborts the connection with no terminating chunk.
#[derive(Debug)]
pub struct DrainPeriodEnded;

impl std::fmt::Display for DrainPeriodEnded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the drain period ended while the response was being written")
    }
}

impl std::error::Error for DrainPeriodEnded {}

impl Answer {
    /// The body a caller already has the bytes of.
    #[must_use]
    pub fn new(bytes: impl Into<Bytes>) -> Self {
        Self::Whole(Some(bytes.into()))
    }

    /// No body at all — a `204`, a `304`, or the answer to a `HEAD`.
    #[must_use]
    pub fn empty() -> Self {
        Self::Whole(None)
    }

    /// A body an isolate writes over time, and the writing half to hand it.
    ///
    /// The send timeout and the message bound both come from the connection's
    /// own bounds and are read here, which is the only place either is read: a
    /// cell that chose its own would be a second bound beside
    /// `rule:concurrency/connection-bounds-are-finite`'s table, invisible to
    /// the operator reading that one. `message` rather than `frame` because
    /// nothing between a program and this cell is framed — what crosses whole
    /// is the chunk, which on an event stream is one event.
    #[must_use]
    pub fn stream(bounds: &crate::bounds::Connection) -> (stream::Emit, Self) {
        let (emit, drain) = stream::open(bounds.send, bounds.message);
        (
            emit,
            Self::Streaming {
                drain,
                alive: None,
                cut: None,
            },
        )
    }

    /// The same body, held inside the bounds an event stream has of its own: it
    /// writes `nvs_runtime::sse::KEEPALIVE` rather than letting `write_idle`
    /// close the connection under it, and it ends at the connection's lifetime
    /// however busy it was.
    ///
    /// The wait is taken rather than the interval, because
    /// `crate::bounds::heartbeat` is the one place the second is derived from
    /// the first and a caller passing its own interval would be a second bound
    /// beside `rule:concurrency/connection-bounds-are-finite`'s table. Taken
    /// here rather than at [`Self::stream`] because the two halves of a stream
    /// are opened before anything knows which door they are for, and the door
    /// is what decides whether quiet is a fault: [`event_stream`] is the one
    /// caller. A whole body has no silence to answer for and is handed back as
    /// it is.
    ///
    /// `due_at` is the connection loop's half of the clock, and
    /// `crate::bounds::wake_at` owns why the wake is arranged there rather than
    /// from inside this poll.
    #[must_use]
    pub fn as_an_event_stream(
        self,
        bounds: &crate::bounds::Connection,
        draining: &Draining,
        write_idle: Duration,
        due_at: crate::bounds::NextBeat,
    ) -> Self {
        match self {
            Self::Streaming { drain, .. } => Self::Streaming {
                drain,
                alive: Some(crate::bounds::EventStream::opened(
                    bounds, draining, write_idle, due_at,
                )),
                cut: None,
            },
            whole => whole,
        }
    }

    /// The bytes a whole body carries, empty where it carries none and empty
    /// for a stream, whose bytes no caller here holds.
    ///
    /// A whole response body is one buffer this crate already has, so reading it
    /// needs no poll and no `Context`. That is what makes [`crate::statics`]'s
    /// cases assertions about *bytes* — the one range that was asked for, and
    /// the empty body of a `304` — rather than about a status and a header pair
    /// that happen to look right.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        match self {
            Self::Whole(bytes) => bytes.as_deref().unwrap_or_default(),
            Self::Streaming { .. } => &[],
        }
    }
}

impl Body for Answer {
    type Data = Bytes;
    type Error = DrainPeriodEnded;

    /// A whole body's one frame, then the end of the stream — or, for a stream,
    /// whatever chunk the writing isolate has put in the cell, and
    /// [`Poll::Pending`] with this poll's waker left where the writer will fire
    /// it.
    ///
    /// **A stream carrying a heartbeat answers its own silence.** A poll that
    /// would have been `Pending` past the beat's due instant writes
    /// [`nvs_runtime::sse::KEEPALIVE`] instead, which is a comment line every
    /// client discards and the only thing that makes an idle event stream
    /// outlive `write_idle`. Every other answer is a byte that moved, so the
    /// interval starts again from it and a stream under load never writes one.
    ///
    /// **An event stream's first frame is its reconnection hint**, before any
    /// event and before the cell is read at all: it is a `retry:` block with no
    /// `data:` line, so a client takes the wait and dispatches nothing, and the
    /// wait is drawn for this stream alone (`crate::bounds::reconnect_hint`).
    ///
    /// **And an event stream that has met a bound ends here** — its lifetime,
    /// or the drain of a server shutting down — ahead of the cell rather than
    /// after it: a bound that let one more chunk out would be a bound the
    /// busiest stream is never held by, which is the half of
    /// `rule:concurrency/connection-bounds-are-finite` the word *however* is
    /// doing. Under a drain a stream with nothing to write ends on the poll
    /// that finds it idle, and one still writing ends at the drain period's
    /// end (`rule:concurrency/a-drain-closes-a-connection-cleanly`). Ending
    /// the body is the whole close on this door, which has no
    /// close frame and needs none: `hyper` writes the terminating chunk, the
    /// peer reads a stream that finished rather than a connection that was
    /// reset, and the hint above is what paces its way back.
    ///
    /// **A request-scoped stream ends in [`DrainPeriodEnded`] at the drain
    /// period's end**, also ahead of the cell. It has no way to say it was
    /// cut, so the error is what stops `hyper` from writing the terminating
    /// chunk, and the end of `serve_connection` abandons the program still
    /// writing it.
    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match &mut *self {
            Self::Whole(bytes) => Poll::Ready(bytes.take().map(|bytes| Ok(Frame::data(bytes)))),
            Self::Streaming { drain, alive, cut } => {
                if cut.as_ref().is_some_and(DrainCut::reached) {
                    return Poll::Ready(Some(Err(DrainPeriodEnded)));
                }
                if let Some(alive) = alive {
                    if let Some(opening) = alive.opening() {
                        return Poll::Ready(Some(Ok(Frame::data(Bytes::from(opening)))));
                    }
                    if alive.over() {
                        alive.ended();
                        return Poll::Ready(None);
                    }
                }
                match drain.next_chunk(cx.waker()) {
                    stream::Drained::Chunk(chunk) => {
                        if let Some(alive) = alive {
                            alive.moved();
                        }
                        Poll::Ready(Some(Ok(Frame::data(Bytes::from(chunk)))))
                    }
                    stream::Drained::Pending => {
                        if let Some(alive) = alive
                            && alive.drain_begun()
                        {
                            alive.ended();
                            return Poll::Ready(None);
                        }
                        if alive.as_mut().is_some_and(crate::bounds::EventStream::due) {
                            Poll::Ready(Some(Ok(Frame::data(Bytes::from_static(
                                nvs_runtime::sse::KEEPALIVE,
                            )))))
                        } else {
                            Poll::Pending
                        }
                    }
                    stream::Drained::Ended => {
                        if let Some(alive) = alive {
                            alive.ended();
                        }
                        Poll::Ready(None)
                    }
                }
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        match self {
            Self::Whole(bytes) => bytes.is_none(),
            // Whether a stream is over is the cell's answer and it is given by
            // taking from it, so the honest answer before a poll is "no".
            Self::Streaming { .. } => false,
        }
    }

    fn size_hint(&self) -> SizeHint {
        match self {
            Self::Whole(bytes) => {
                let len = bytes.as_ref().map_or(0, Bytes::len);
                // Widening on every target this builds for; the fallible
                // spelling is here because the lint policy has no exception for
                // a cast that happens to be safe.
                SizeHint::with_exact(u64::try_from(len).unwrap_or(u64::MAX))
            }
            // No exact size, which is what leaves `hyper` chunking it.
            Self::Streaming { .. } => SizeHint::default(),
        }
    }
}

/// What a handler answers one request with — `rule:http-server/two-deployments-and-nothing-a-proxy-owns`'s § 4 outcomes, as the
/// two things this loop can do with them.
///
/// A handler used to answer with an [`Isolate`] and nothing else, which was
/// `rule:http-server/a-request-resolves-in-five-steps` with only step 5 in it. Step 1's third arrow is a `404` and
/// step 3 is a file's bytes, and neither is a program: they are responses this
/// server already holds in full, so the type says so rather than a handler
/// inventing an isolate whose only job is to `echo` a status.
///
#[derive(Debug)]
pub enum Reply {
    /// Run this isolate as a child of the connection, and answer with what it
    /// echoed — § 4 steps 4 and 5, and `rule:security/sink-predicate`'s table.
    ///
    /// **The second field is the connection's half of that request's body**, and
    /// it comes back out of the handler because it may not travel with the
    /// isolate: `hyper`'s [`Incoming`] is polled with the *connection's*
    /// context, and the isolate is a different task
    /// ([`crate::body`]'s module docs are the whole argument). `None` where the
    /// request carried no body, which is also what leaves
    /// [`nvs_runtime::Inbound::has_body`] false.
    ///
    Run(Isolate, Option<Supply>),
    /// Answer with this, having run nothing: § 4 step 1's `404`, step 3's static
    /// file, and every refusal a mount table can reach before a program exists.
    Done(Response<Answer>),
}

impl Reply {
    /// A bodiless response carrying `status` — the shape every refusal in this
    /// crate takes until a mode says one may say more
    /// ([`answer`]'s docs own that direction).
    #[must_use]
    pub fn status(status: StatusCode) -> Self {
        let mut response = Response::new(Answer::empty());
        *response.status_mut() = status;
        Self::Done(response)
    }

    /// Run this isolate for a request that carries no body — [`Self::Run`] with
    /// nothing to supply, spelled so that a caller which never reads one does
    /// not have to name the half it has not got.
    #[must_use]
    pub fn run(isolate: Isolate) -> Self {
        Self::Run(isolate, None)
    }

    /// A body whose declared length is already over [`crate::body::UPLOAD_TOTAL`]:
    /// `413`, before a mount is asked for a program.
    ///
    /// `rule:http-server/request-body-and-upload-total-are-two-caps`
    /// puts this refusal in the server rather than in each consumer, and
    /// before dispatch rather than after it, so that the honest oversized client
    /// never reaches application code and nothing has been allocated to tell it
    /// so. A peer that declares no length is bounded on the wire instead, by the
    /// supplier that is the only thing counting bytes.
    #[must_use]
    pub fn too_large() -> Self {
        Self::status(StatusCode::PAYLOAD_TOO_LARGE)
    }

    /// A checked unsafe verb the door could not tie to this deployment: `403`,
    /// before an isolate exists.
    ///
    /// `rule:security/csrf-is-on-by-default` is the decision and
    /// [`crate::route::csrf`] is where it is taken; this is only the status it
    /// comes out as. Bodiless like every other refusal here, and `403` rather
    /// than `400` because the request is well formed and this server is
    /// declining it — a distinction a browser's own reporting reads.
    #[must_use]
    pub fn forbidden() -> Self {
        Self::status(StatusCode::FORBIDDEN)
    }

    /// `rule:http-server/a-request-resolves-in-five-steps` step 1's third arrow — no mount covers the request, so
    /// there is no application to give it to and none to have written this.
    ///
    /// Spelled here rather than at each call site so that a caller does not have
    /// to depend on `hyper` to say the one thing every mount table says.
    #[must_use]
    pub fn not_found() -> Self {
        Self::status(StatusCode::NOT_FOUND)
    }

    /// The verb the peer wrote is outside the eight `Core\Http\Method` names, so
    /// no program is asked to answer it: `501`, before an isolate exists.
    ///
    /// `501` and not `405`, which is the route table's answer to a verb it knows
    /// standing at a path that does not accept it — a distinction RFC 9110 § 15.5
    /// draws and `Core\Router` owns the other half of. A verb this server has no
    /// case for is not implemented *anywhere* on it, and saying so before any
    /// application code runs is what keeps `Core\Request::method()`'s closed
    /// roster total: a member that answers one of eight cases can only do that if
    /// a ninth never reaches it.
    ///
    /// **The roster itself is `nvs_stdlib::request`'s**, which this crate has no
    /// dependency on and gains none for a list of eight words: the caller asks
    /// that crate and answers with this. `nvs-cli`'s handler is the one door
    /// today.
    #[must_use]
    pub fn not_implemented() -> Self {
        Self::status(StatusCode::NOT_IMPLEMENTED)
    }

    /// `rule:http-server/the-server-block-is-boot-class`'s probe, answered by a server that is accepting: `200`
    /// with an empty body, no dependency check and no version.
    ///
    /// Spelled here for [`not_found`](Reply::not_found)'s reason, and because
    /// the whole of the endpoint is its status — a body would be the thing § 5
    /// says it does not report.
    #[must_use]
    pub fn healthy() -> Self {
        Self::status(StatusCode::OK)
    }

    /// The same probe, answered by a server that has stopped accepting: `503`,
    /// still with an empty body.
    ///
    /// Deliberately not [`crate::admit`]'s `503`, which carries `Retry-After`
    /// because the condition it reports clears on its own. This one does not:
    /// the process is on its way out, and a proxy told to retry in a second
    /// would be told to come back to a socket that will not be there.
    #[must_use]
    pub fn draining() -> Self {
        Self::status(StatusCode::SERVICE_UNAVAILABLE)
    }

    /// § 5's probe as this server's own state — `200` while it accepts, `503`
    /// while it drains — which is the one place that choice is made.
    ///
    /// A server answering it from anywhere else would be a second reading of the
    /// same fact, and the fact is what a proxy takes an instance out of rotation
    /// on: the two answers have to change over at the instant the accept loop
    /// stops, which is [`Draining`]'s whole job.
    #[must_use]
    pub fn health(draining: &Draining) -> Self {
        if draining.is_draining() {
            Self::draining()
        } else {
            Self::healthy()
        }
    }
}

/// Whether this server has stopped accepting — `rule:http-server/the-server-block-is-boot-class`'s drain, as the one
/// bit a probe and an application both read.
///
/// **A drain is begun by whatever is stopping, and the accept loop obeys it.**
/// What stops a process is outside this crate — a terminating signal, a service
/// manager, an operator on the control socket — so each of those writes this
/// bit, and every accept loop in the process reads it and stops accepting.
/// [`serve_on_this_core`]'s `keep_serving` seam is the other direction, for a
/// loop that ends on its own terms, and its tail writes the bit as well: a loop
/// that has stopped accepting is draining however it was asked, and a probe
/// answering `200` for a socket that is already closed is exactly the window a
/// proxy uses this endpoint to avoid.
///
/// **The bit is [`nvs_runtime::Drain`] and this is the handle over it.** An
/// application reads the same drain through `Core\Server::isDraining()`, and
/// `nvs-stdlib` does not depend on this crate — so the fact sits in the crate
/// both rest on, and that module's docs own the atomic, its ordering, and why a
/// server that is this process takes the process's bit rather than one per
/// core. What lives here is the rule above: who may write it.
#[derive(Clone, Debug)]
pub struct Draining(Drain);

impl Draining {
    /// The process's drain — the handle a server that *is* this process takes,
    /// and so the one whose answer a proxy takes an instance out of rotation
    /// on.
    #[must_use]
    pub fn process() -> Self {
        Self(Drain::process())
    }

    /// A drain no other server in this process shares.
    ///
    /// For an accept loop whose stopping is not this process stopping — a
    /// second listener a harness ends on its own, and every test that runs this
    /// loop to completion beside others. [`nvs_runtime::drain`] owns why that
    /// is a case rather than an escape hatch.
    #[must_use]
    pub fn detached() -> Self {
        Self(Drain::detached())
    }

    /// Stop accepting: from here the probe answers `503`.
    ///
    /// Idempotent, because a drain that has begun cannot begin again and a
    /// second core reaching this is the same shutdown, not a new one.
    pub fn begin(&self) {
        self.0.begin();
    }

    /// Whether the drain has begun.
    #[must_use]
    pub fn is_draining(&self) -> bool {
        self.0.is_draining()
    }

    /// The bit itself, for a caller that needs to **park** on the drain rather
    /// than read it: [`serve_on_this_core`] registers a wake against it so that
    /// a loop parked in `accept` is told rather than finding out when the next
    /// connection arrives, and `nvs_host::worker::register_this_core` gives the
    /// receptionist a serving core offers itself through the same ending.
    ///
    /// [`nvs_runtime::Drain::wake_at_drain`] is the whole of what this adds over
    /// the handle — beginning a drain and reading one are already here — and
    /// what is behind it is the process's own bit, which
    /// [`nvs_runtime::Drain::process`] hands to anybody who asks for it.
    pub fn bit(&self) -> &Drain {
        &self.0
    }

    /// The drain a connection accepted under the tree `generation` reads: this
    /// server's drain joined with that tree's, so a stop and a reload both
    /// close it the same way.
    fn for_connection(&self, generations: &Generations, generation: u64) -> Self {
        Self(Drain::any_of(&[
            &self.0,
            &generations.drain_for(generation),
        ]))
    }
}

/// The drain of the connections accepted under each published tree.
///
/// `rule:concurrency/a-drain-closes-a-connection-cleanly` gives a reload the
/// ending a stop has, so the connections accepted under the tree a reload
/// replaces are drained, and the accept loop keeps accepting under the new
/// one. One drain is held, the newest tree's. A newer generation begins it and
/// takes its place, whether a reload ([`Generations::retire_before`]) or an
/// accept loop that saw the newer tree first ([`Generations::drain_for`]) asks.
/// So a connection accepted between the publish and the reload's call is
/// still given the new tree's drain.
///
/// Memory: one drain per tree that still has a connection open, freed with the
/// last of them.
#[derive(Debug)]
pub struct Generations(Mutex<(u64, Drain)>);

impl Default for Generations {
    fn default() -> Self {
        Self(Mutex::new((0, Drain::detached())))
    }
}

impl Generations {
    /// The drain of the connections accepted under the tree `generation`. It
    /// has already begun when a newer tree has been published.
    pub fn drain_for(&self, generation: u64) -> Drain {
        let mut held = self.held();
        if generation < held.0 {
            let retired = Drain::detached();
            retired.begin();
            return retired;
        }
        if generation > held.0 {
            held.1.begin();
            *held = (generation, Drain::detached());
        }
        held.1.clone()
    }

    /// Begin the drain of every connection accepted under a tree older than
    /// `generation`, which is the tree a reload has just published.
    pub fn retire_before(&self, generation: u64) {
        drop(self.drain_for(generation));
    }

    /// The held drain, taking a poisoned lock as the pair it holds: nothing
    /// done under it can leave the pair half written.
    fn held(&self) -> std::sync::MutexGuard<'_, (u64, Drain)> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// What every connection this server hands over is served under: `rule:http-server/the-server-block-is-boot-class`'s
/// valve, the published tree, and the header set, cross-origin policy, proxy
/// list, waits and connection bounds that tree configures.
///
/// One argument rather than one per policy because these are the *shared* half
/// of a connection's context — an [`Arc`] each, so every core answers under the
/// one valve and the one configuration. A connection copies its waits and
/// bounds out of the snapshot published when it is accepted, and keeps that
/// copy for its whole life. A reload reaches the next connection, and drains
/// the ones already open ([`Generations`]).
#[derive(Clone, Debug)]
pub struct Serving {
    /// § 5's in-flight ceiling, asked before the handler is.
    admission: Arc<Admission>,
    /// The header set, the cross-origin policy and the proxy list of the last
    /// snapshot a request was answered under, shared by every core
    /// ([`Serving::policy`]).
    policy: Arc<RwLock<Arc<Policy>>>,
    /// The tree this instance is serving — `rule:config/the-config-is-an-immutable-snapshot`'s
    /// published snapshot, which every request reads through a clone taken at
    /// its own start ([`serve_connection`]). It rides here for the reason the
    /// policies above it do: one tree for the whole instance, shared by every
    /// core rather than resolved per connection, and never re-read under a
    /// request that has begun.
    ///
    /// The holder rather than the snapshot, so that a reload published on the
    /// control endpoint reaches the next request rather than the next start
    /// (`rule:config/one-local-control-socket`). A request that has begun is
    /// unaffected either way: it holds the [`Arc`] it took, and the clone here
    /// is taken once at its start.
    current: Arc<nvs_config::Current>,
    /// The drain of the connections accepted under each tree `current` has
    /// published, which a reload begins for the tree it replaced.
    generations: Arc<Generations>,
}

impl Serving {
    /// What a boot resolves, as the one argument every connection is served
    /// under, over a tree nothing publishes to.
    ///
    /// For a server whose configuration cannot move under it — every embedder
    /// and every test that is about something else. A process with a control
    /// endpoint takes [`live`](Self::live) instead, since the holder is the
    /// thing the endpoint publishes into.
    #[must_use]
    pub fn new(
        admission: Arc<Admission>,
        secure: Arc<Secure>,
        trusted: Arc<Trusted>,
        cors: Arc<Cors>,
        snapshot: Arc<nvs_config::Snapshot>,
    ) -> Self {
        Self::live(
            admission,
            secure,
            trusted,
            cors,
            Arc::new(nvs_config::Current::new(snapshot)),
        )
    }

    /// [`new`](Self::new) over a holder somebody else publishes into.
    ///
    /// `secure`, `cors` and `trusted` are the policies for the snapshot
    /// `current` holds now. A snapshot published later gets policies derived
    /// from its own `[http]` block and `[server] trusted_proxies`
    /// ([`Serving::policy`]).
    #[must_use]
    pub fn live(
        admission: Arc<Admission>,
        secure: Arc<Secure>,
        trusted: Arc<Trusted>,
        cors: Arc<Cors>,
        current: Arc<nvs_config::Current>,
    ) -> Self {
        let policy = Policy {
            of: current.load(),
            secure,
            cors,
            trusted,
            waits: None,
            bounds: crate::bounds::Connection::default(),
        };
        Self {
            admission,
            policy: Arc::new(RwLock::new(Arc::new(policy))),
            current,
            generations: Arc::new(Generations::default()),
        }
    }

    /// The drains of the trees this server has accepted under, for whoever
    /// publishes into its holder: that publisher retires the old tree's
    /// connections with [`Generations::retire_before`].
    #[must_use]
    pub fn generations(&self) -> Arc<Generations> {
        Arc::clone(&self.generations)
    }

    /// The header set, the cross-origin policy and the proxy list a request
    /// that took `snapshot` is answered under.
    ///
    /// `[http.headers]`, `[http.cors]` and `[server] trusted_proxies` are
    /// `Reload`-class (`rule:config/reloadability-is-its-own-field`), so a
    /// reload that moves any of them reaches the next request. An entry of the
    /// proxy list that names no network is dropped here without a note: the
    /// boot is where one is reported. Rendering the policies checks and builds
    /// every header value, so it happens once per snapshot and not once per request:
    /// the first request under a new snapshot derives them and stores them for
    /// every core, and each request after it pays a read lock and a pointer
    /// comparison. Two requests under two snapshots at once can derive in
    /// turn, and each is still answered under its own snapshot.
    ///
    /// Memory: the policies of one snapshot, and that snapshot kept alive
    /// until a request takes a newer one.
    ///
    /// # Panics
    ///
    /// If a thread panicked while holding the lock. Nothing done under it can
    /// panic — an `Arc` clone and an `Arc` store.
    fn policy(&self, snapshot: &Arc<nvs_config::Snapshot>) -> Arc<Policy> {
        let held = Arc::clone(
            &self
                .policy
                .read()
                .expect("the policy lock is never poisoned"),
        );
        if Arc::ptr_eq(&held.of, snapshot) {
            return held;
        }
        let http = snapshot.config.http.as_ref();
        let (trusted, _) = Trusted::of(
            snapshot
                .config
                .server
                .as_ref()
                .and_then(|server| server.trusted_proxies.as_deref())
                .unwrap_or_default(),
        );
        // A published tree passed `nvs_config::server::validate`, so neither
        // arm below fails. If one did, the connection keeps the numbers the
        // last snapshot gave it.
        let no_origins = std::collections::BTreeMap::new();
        let waits =
            nvs_config::server::waits_for(&snapshot.config, &no_origins).map_or(held.waits, Some);
        let bounds = nvs_config::server::connection_bounds_for(&snapshot.config, &no_origins)
            .map_or(held.bounds, crate::bounds::Connection::configured);
        let derived = Arc::new(Policy {
            of: Arc::clone(snapshot),
            secure: Arc::new(Secure::of(http)),
            cors: Arc::new(Cors::of(http)),
            trusted: Arc::new(trusted),
            waits,
            bounds,
        });
        *self
            .policy
            .write()
            .expect("the policy lock is never poisoned") = Arc::clone(&derived);
        derived
    }

    /// The same, under the connection bounds a boot resolved, rather than under
    /// the ones this server ships. A snapshot published later brings the
    /// bounds its own `[server.connection]` block resolves to.
    ///
    /// A step after the constructor and not a parameter of it, because that is
    /// what the two callers are: a process that read a `[server.connection]`
    /// block takes this, and every embedder and every test that is about
    /// something else is served under
    /// [`crate::bounds::Connection::default`] — which is
    /// `rule:concurrency/connection-bounds-are-finite`'s point, that the
    /// unconfigured table is already a complete one.
    #[must_use]
    pub fn bounded_by(self, bounds: crate::bounds::Connection) -> Self {
        {
            let mut held = self
                .policy
                .write()
                .expect("the policy lock is never poisoned");
            let booted = &**held;
            *held = Arc::new(Policy {
                of: Arc::clone(&booted.of),
                secure: Arc::clone(&booted.secure),
                cors: Arc::clone(&booted.cors),
                trusted: Arc::clone(&booted.trusted),
                waits: booted.waits,
                bounds,
            });
        }
        self
    }

    /// The waits and the connection bounds a connection accepted now is
    /// served under: those of the snapshot published now, or `booted` and the
    /// bounds [`bounded_by`](Self::bounded_by) set while that is still the
    /// snapshot this was built with.
    fn connection_terms(&self, booted: Waits) -> (Waits, crate::bounds::Connection) {
        let policy = self.policy(&self.current.load());
        (policy.waits.unwrap_or(booted), policy.bounds)
    }
}

/// The policies and the connection terms one published snapshot configures.
#[derive(Debug)]
struct Policy {
    /// The snapshot these were derived from. Only its address is compared.
    of: Arc<nvs_config::Snapshot>,
    /// `rule:http-server/secure-headers-with-nothing-written`'s header set,
    /// filled into every response this loop writes.
    secure: Arc<Secure>,
    /// `rule:http-server/cors-is-closed-until-origins-are-named`'s cross-origin
    /// policy, asked of a preflight before the handler is. Closed is the
    /// default — [`crate::cors`] owns what that means and why the refusal is
    /// taken here rather than in an application.
    cors: Arc<Cors>,
    /// `rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing`'s
    /// `[server] trusted_proxies`, resolved: who may assert a client address
    /// or a scheme. Empty is the default and means no forwarded header is read
    /// at all — [`crate::forwarded`] owns that difference.
    trusted: Arc<Trusted>,
    /// `rule:http-server/the-server-block-is-boot-class`'s waits and
    /// `drain_timeout`, for a connection accepted under this snapshot. `None`
    /// for the snapshot [`Serving`] was built with: a connection accepted under
    /// it keeps the waits its caller handed [`serve_connection`].
    waits: Option<Waits>,
    /// `rule:concurrency/connection-bounds-are-finite`'s table, for a
    /// connection accepted under this snapshot.
    bounds: crate::bounds::Connection,
}

/// The isolate answering one request, held by the future that is waiting for it.
///
/// Two things live here that a bare `Box<dyn Running>` does not say. The first
/// is that the wait is a **poll** and not a park:
/// [`nvs_host::Running::finished`] is the non-parking question the service
/// future asks each time `hyper` polls it, which is what leaves the
/// connection's own task free to go round the dispatcher's loop — and the read
/// side of that loop is where a request's body comes from
/// ([`serve_connection`]'s docs own the argument).
///
/// The second is what a **drop** means. A service future can be dropped with
/// its request still running — `hyper` giving up on a connection whose client
/// went away, or this task being torn down under it — and a drop that simply
/// released the handle would leave an isolate running with nothing left that
/// could prove it finished, which is
/// `rule:concurrency/nothing-is-still-running-when-a-call-returns`
/// gone rather than kept. So the drop **waits for the request to end**, on this
/// connection's own task, and discards its answer:
/// `rule:http-server/a-request-outlives-a-client-that-goes-away`. The task
/// parking here is the request's same-core owner, which keeps everything the
/// connection held for the request — its admission place, the drain's count of
/// it — held until the request ends, with no second owner to hand them to.
///
/// The drop **abandons** instead — cancel, then wait, which
/// [`nvs_host::Running::abandon`] owns with the one case that may not wait —
/// for a method `[limits] cancel_on_disconnect` lists, once `disconnect_grace`
/// has passed for a request that has no `wall_time` at that moment, once the
/// drain period has ended, and when this task is itself cancelled or torn down.
/// A request with a `wall_time` is not cut by the grace, and clearing that
/// `wall_time` later wakes this wait, so the grace bounds it from then on.
struct Peer {
    running: Option<Box<dyn Running>>,
    /// The connection's half of the request's *own* body, pumped by whoever
    /// polls this connection — the wait in the service future, and the drive
    /// loop once a streamed answer's head has gone out, since answering a head
    /// early does not end the request. The drop hangs it up before it waits,
    /// because nothing pumps it after that.
    supply: Option<Supply>,
    /// Whether the request's method is listed in `cancel_on_disconnect`, read
    /// once at its start from the snapshot it runs under.
    cancels: bool,
    /// `[limits] disconnect_grace`, read beside `cancels`: how long the
    /// request runs on after its client left while it has no `wall_time`.
    grace: Duration,
    /// The request's tree, for the `wall_time` it has now and a wake when that
    /// moves — what decides whether the grace bounds the request at all.
    tree: nvs_runtime::SafepointView,
    /// The drain this connection is under, and when it saw it: what ends a
    /// wait for a request whose client is gone at the drain period's end.
    draining: Draining,
    seen: crate::io::DrainSeen,
    period: Duration,
}

impl Peer {
    /// Whether the request has ended, asked without waiting for it.
    fn finished(&self) -> bool {
        self.running
            .as_ref()
            .is_none_or(|running| running.finished())
    }

    /// Ends the request now and waits for it to stop: the drain period's cut.
    fn cancel(mut self) {
        if let Some(running) = self.running.take() {
            running.abandon();
        }
    }

    /// When the drain period ends, once a drain has begun.
    fn cut_at(&self) -> Option<Instant> {
        self.draining
            .is_draining()
            .then(|| self.seen.period_ends(self.period))
    }

    /// Takes the answer, once the request has ended.
    ///
    /// **"Ended" is the request's own frame and not its whole tree.**
    /// `rule:concurrency/after-response-outlives-the-connection`
    /// 's after-response work runs on that tree once the answer here has
    /// been filed, and `nvs_host::isolate` cuts the tree loose from this
    /// connection before it does — so a connection that closes the instant its
    /// response is written cancels none of it.
    ///
    /// `None` only for a `Peer` already collected, which the one caller cannot
    /// reach — worth an answer rather than a panic all the same, since what it
    /// would cost a future caller is one `500` instead of a connection.
    fn collect(&mut self, ctx: &mut Ctx) -> Option<Completion> {
        self.running.take().map(|running| running.join(ctx))
    }
}

impl Drop for Peer {
    fn drop(&mut self) {
        let Some(running) = self.running.take() else {
            return;
        };
        if let Some(supply) = self.supply.take() {
            supply.hang_up();
        }
        if self.cancels {
            running.abandon();
            return;
        }
        let grace_ends = Instant::now() + self.grace;
        // The request's own end wakes this task, as it does the wait in the
        // service future. The other wakes filed here are a drain beginning, the
        // drain's cut once it has, the grace's end, and the request's
        // `wall_time` moving, which decides whether the grace bounds it at all.
        while !running.finished() {
            let now = Instant::now();
            let cut_at = self.cut_at();
            let grace_at = (self.tree.wall_time() == 0).then_some(grace_ends);
            if cut_at.is_some_and(|at| at <= now)
                || grace_at.is_some_and(|at| at <= now)
                || nvs_runtime::Teardown::in_progress()
            {
                break;
            }
            // One timer per task, so the earlier of the two.
            if let Some(at) = cut_at.into_iter().chain(grace_at).min() {
                crate::bounds::wake_at(at);
            }
            // Only before a drain: one that has begun fires a wake at once,
            // and its cut is the timer above.
            let _woken_at_drain = cut_at
                .is_none()
                .then(|| nvs_host::wake_at_drain(self.draining.bit()))
                .flatten();
            if let Some(wake) = nvs_host::current_task()
                .and_then(|me| nvs_host::reactor::with_current(|reactor| reactor.remote_wake(me)))
            {
                self.tree.watch_wall_time(Box::new(move || {
                    let _ = wake.wake();
                }));
            }
            let resumed = suspend_current(Waiting::Parked);
            // Taken back by this task, which settles it without a wake.
            drop(self.tree.unwatch_wall_time());
            if !resumed.suspended() || resumed.cancelled() {
                break;
            }
        }
        // Returns at once for a request that ended: what it answered is
        // dropped, because the client it was for has gone.
        running.abandon();
    }
}

/// A request that answered its head while it was still writing its body: the
/// isolate still running, and the place it holds while it runs.
///
/// Both of these are what an ordinary request keeps inside the service future's
/// own frame. A streamed answer leaves that frame early — that is the whole of
/// what "the head goes out when the member is called" costs — so the two are
/// held on the connection instead, where [`joined_when_ended`] takes them once
/// the body is over, and the end of [`serve_connection`] drops them for a peer
/// that went away first.
struct Streamed<'a> {
    /// The isolate, joined once its body has ended. If this connection ends
    /// first, [`Peer`]'s own drop waits for it or cancels it, and that drop
    /// runs before the place below is given back.
    peer: Peer,
    /// `rule:http-server/a-requests-blast-radius-is-bounded-at-four-tiers`'s
    /// tier C, held for as long as this request is still spending a core. A
    /// place given back when the head went out would be a request running
    /// outside the one ceiling that counts it, and a peer that reads slowly
    /// would be how a server acquires unbounded concurrency.
    _place: crate::admit::InFlight<'a>,
    /// What a sampled request's root span is of, and the trace it belongs to —
    /// read off the carrier before the isolate took it, and held here for the
    /// same reason the isolate is: a streamed answer leaves the service
    /// future's frame early, so what its end owes the exporter travels with it.
    /// `None` for every request nobody is recording.
    recording: Option<(String, nvs_runtime::TraceContext)>,
}

/// Joins a streaming request's isolate once the body it was writing has ended,
/// and leaves it running until then.
///
/// **"The body has ended" is asked as [`nvs_host::Running::finished`]**, and the
/// two are one question rather than two: the writing half lives on the
/// request's own context and the isolate's finish path is where it is dropped
/// (`nvs_runtime::Ctx::take_body_stream`), so a body ends exactly when the
/// isolate writing it does. Asking the drain instead would be asking after a
/// half this connection has already handed to `hyper`.
///
/// Joining any earlier is what this exists to prevent, and it is not a slow
/// path but a deadlock: [`nvs_host::Running::join`] parks until the child has
/// ended, the child cannot end until the chunks it is parked on have been
/// taken, and the task that would take them is the one that just parked.
fn joined_when_ended(writing: &RefCell<Option<Streamed<'_>>>, ctx: &mut Ctx) {
    let ended = writing
        .borrow()
        .as_ref()
        .is_some_and(|streamed| streamed.peer.finished());
    if !ended {
        return;
    }
    let Some(mut streamed) = writing.borrow_mut().take() else {
        return;
    };
    if let Some(mut done) = streamed.peer.collect(ctx) {
        // The spans first, because the discharge below is the end of this
        // completion: a streamed request files its events like any other, and
        // the head having gone out early is no reason for its trace to be the
        // one that is never exported.
        if let Some((request, trace)) = &streamed.recording {
            crate::trace::record(request, trace, &done.trace);
        }
        // Nowhere to move the child's returned value to, and nothing left to
        // say with what it wrote: the answer is already on the wire. [`answer`]
        // makes the same discharge for the same reason, this crate forbidding
        // the `unsafe` that a release takes.
        done.discard_value();
    }
}

/// Drives one accepted connection to completion on the calling coroutine.
///
/// The whole of `rule:concurrency/one-future-per-connection`: one future, on this task's own stack, polled by
/// [`nvs_host::block_on()`]. `hyper` with `http1` and `server` alone spawns
/// nothing, so there is no executor to install and no second scheduler to
/// reconcile with [`nvs_host::Scheduler`].
///
/// `handler` is asked once per request for the [`Reply`] that request is. Where
/// that is a program it is an
/// `rule:security/isolate-shares-nothing` [`Isolate`] —
/// the same type `spawn script` runs, and deliberately **not** a second isolation
/// path, since M7's state-bleed suite is a parameterisation of one mechanism and
/// would prove nothing about two of them. Where it is already a response
/// (`rule:http-server/a-request-resolves-in-five-steps`'s `404`, or a file), nothing is run for it at all.
///
/// **That isolate runs as a peer task, and the service answers `Pending` until
/// it has ended.** The request is started here ([`Isolate::start`]) and
/// collected here, but it is not *run* here: what stands between the two is a
/// future that asks [`nvs_host::Running::finished`] each time `hyper` polls it,
/// so the connection's own task is free to go round its dispatcher's loop while
/// the request is still going. That is the shape the body needs and the reason
/// it is not the simpler one — `hyper`'s h1 dispatcher polls the read side and
/// the service on **one** task, so a request that has to wait for body bytes
/// can only get them if this future can answer `Pending` and be polled again;
/// a service that ran the isolate to completion inside its own poll would be
/// waiting for a read that its own frame is what owes. [`Peer`] is that wait,
/// and it is also `rule:concurrency/nothing-is-still-running-when-a-call-returns`'s cancellation.
///
/// `rule:concurrency/one-future-per-connection` is what makes answering `Pending` cheap rather than an
/// executor: the connection future is driven on this coroutine's own stack, so
/// the park is [`nvs_host::block_on()`]'s and the resume lands back inside the
/// same poll, with the core having served its other connections in between.
/// Nothing re-enters while it is parked, because a suspended task runs nothing
/// at all, which is what gives the borrow below one borrower by construction.
///
/// `ctx` is the connection task's, and that makes it the root of this
/// connection's request tree
/// (`rule:concurrency/a-child-belongs-to-the-calling-task`
/// ): a request's isolate is a child of the connection, so a client that
/// goes away takes its request's tasks with it rather than leaving them
/// behind.
///
/// The request is handed to `handler` with its body unread, and `handler`
/// hands the connection's half of it back in [`Reply::Run`]: an [`Incoming`] is
/// polled with *this* task's context, so it may not travel to the isolate, and
/// [`crate::body`]'s two halves over one cell are what crosses instead. This
/// function pumps that half once per poll of the wait above, which is what
/// makes a pull on the request's stack a read on this one.
///
/// **A body the program never reads is never drained.** Nothing here reads
/// ahead — the supply only polls the [`Incoming`] for a request that has asked
/// — so a request that ignored its body leaves bytes on the wire and `hyper`
/// ends the connection rather than framing a second request on it. That is the
/// fail-closed direction and it costs a keep-alive:
/// `rule:http-server/request-body-and-upload-total-are-two-caps`
/// 's cap bounds what a program *asks* for, and draining what it did not ask
/// for would spend the same bytes with nobody having wanted them.
///
/// **`serving` carries § 5's valve, and it is asked before `handler` is.** A
/// request over the ceiling is answered with [`crate::admit::over_capacity`] and
/// nothing is started for it; [`crate::admit`]'s own docs own the order and why
/// it is the whole of the guarantee.
///
/// **It carries `rule:http-server/secure-headers-with-nothing-written`'s header set too, and this function is the one
/// place that set is applied.** Every response that leaves here goes through
/// [`Secure::fill`] — a program's, a mount table's `404`, a static file's and
/// § 5's `503` alike — and it *fills* rather than overwrites, which is what
/// leaves `Core\Response::setHeader` its override on one response.
/// [`crate::secure`]'s own docs own that direction, and the effective scheme
/// this passes.
///
/// **The waits and the connection bounds are the published snapshot's.**
/// Both are copied once, when this function starts, and the connection keeps
/// them for its whole life, so a reload reaches the next connection. `waits`
/// is what a connection gets while the snapshot published is still the one
/// `serving` was built with, which lets a caller frame a connection under a
/// clock of its own.
///
/// **The waits are the clock.** `rule:http-server/the-server-block-is-boot-class`
/// 's four waits bound this connection from the moment it is accepted, and
/// [`crate::io`]'s § *The clock* is where they are actually enforced; what this
/// function owns is the one phase change no adapter can see, which is that
/// `hyper` framing a head ends the header wait and answering the request starts
/// the write one.
///
/// **An upgradable request is offered
/// `rule:concurrency/a-connection-is-a-root-isolate`'s
/// slot, and only an upgradable one.** `hyper` leaves an `OnUpgrade` on the
/// requests it framed an upgrade for; this function takes it, keeps it, and
/// hands the request's isolate the other half of a
/// [`nvs_runtime::UpgradeSlot`] ([`Isolate::offering_upgrade`]) so that
/// `Core\Socket::upgrade` has somewhere to leave the connection isolate it
/// prepared. `nvs_stdlib::socket`'s module doc is the home of why the member
/// cannot start one itself.
///
/// **Every request is offered § 5's cell, and both cells are read at one
/// point.** An event stream takes nothing of this connection — its response is
/// an ordinary `200 text/event-stream` the request already has — so
/// [`nvs_runtime::SseSlot`] is made for every request rather than for an
/// upgradable one ([`Isolate::offering_sse`]), and what it holds opens the same
/// root isolate § 1's slot does. A request that filled **both** is answered
/// `500` and neither isolate is started: it asked for two responses where this
/// connection has one, and `nvs_runtime::SseSlot::fill` is the home of why that
/// refusal belongs here rather than in a cell that cannot see the other one.
/// The arguments of the two it discards are given back by
/// [`nvs_runtime::Upgrade::discard`], which exists because this crate forbids
/// the `unsafe` that a release takes.
///
/// **This function starts what the slot was filled with, and it starts it after
/// the request has ended.** § 1's ordering is the security property rather than
/// a sequencing detail: the request's isolate is joined first, so its arena and
/// its carrier are already released when the connection's own isolate is built,
/// and the connection cannot reach the request's session, cookies or headers
/// because there is nothing left holding them. It is started from **this
/// function's** context, which makes it the request's sibling under the
/// connection rather than a child of the request tree, and it is joined after
/// `hyper`'s connection future ends — a connection isolate outlives the request
/// that opened it, and a task cancelled with the connection that started it
/// would not.
///
/// **The socket's isolate is answered `101` and started last; the event
/// stream's is started inside the request's own future.** That is one ordering
/// stated twice, because the two hand-overs need different things. A socket
/// isolate's peer does not exist until RFC 6455's handshake is on the wire and
/// `hyper` has stopped framing, so the prepared upgrade waits here and
/// [`crate::socket`] frames the connection `http1::Connection::into_parts`
/// hands back — and the `101` *replaces* whatever the request wrote for itself,
/// this connection having one response left. An event stream takes no socket,
/// so nothing is waiting for it.
///
/// The handshake is read off the request rather than assumed: `hyper` framing
/// an upgrade says the connection can be taken over, and a
/// `Sec-WebSocket-Key` with version 13 says a WebSocket is what asked. A
/// request missing either is offered no slot at all, so it cannot upgrade and
/// is answered as the ordinary request it is.
///
/// § 5's `200 text/event-stream` is written here and never by the program that
/// asked for one: the request's own answer is replaced by [`event_stream`],
/// whose body is the half [`nvs_host::Isolate::over_event_stream`] hands the
/// connection's isolate, exactly as the `101` replaces an upgrading request's.
/// The isolate still runs with `Output::Capture`, what it echoes being no part
/// of that body.
///
/// **Every request is offered a third cell, and that one is answered before its
/// request has ended.** `rule:concurrency/a-stream-that-outlives-its-request-is-a-connection`'s
/// other spelling is a body written over time by the request's *own* isolate,
/// so `nvs_runtime::stream::BodySlot` is filled by `Core\Response::stream`
/// while the service future is still waiting, and the wait answers with the
/// head instead of with a completion. What the peer then reads is written by an
/// isolate this connection has not joined yet: [`joined_when_ended`] is where
/// it is, once the body ends, and [`Streamed`] is what holds it until then.
///
/// **`draining` is carried through rather than read here.** No part of an HTTP
/// request's life asks it — the probe's `503` is [`Reply::health`]'s, and this
/// function is never the one holding a probe — but `rule:concurrency/connection-bounds-are-finite`'s shutdown
/// close is a connection's *own*, taken at its next `receive`, so the handle
/// travels with the socket into [`crate::socket::Framed`]. That module's
/// `receive` is where it is argued why the close is taken there and not from
/// the accept loop.
///
/// # Errors
///
/// `hyper`'s own for this connection: a peer that spoke something other than
/// h1, a socket that failed under it, a request it refused to frame, or a wait
/// that expired under it. A cancelled task is not one of them — the drive
/// answers `None` and this reports `Ok`, because the connection ended for a
/// reason its caller already knows about. **A request's own failure is not one
/// either**: `rule:security/isolate-shares-nothing`'s failure is a value, so it becomes a response instead.
pub fn serve_connection<H>(
    stream: NvsConnection,
    arrival: Arrival,
    ctx: &mut Ctx,
    handler: &H,
    waits: Waits,
    serving: &Serving,
    draining: &Draining,
) -> hyper::Result<()>
where
    H: Fn(Request<Incoming>, Origin) -> Reply,
{
    let ctx = RefCell::new(ctx);
    // Borrowed once, here, rather than captured: the service below hands its
    // captures on to a future that outlives the call that made it, and a
    // shared reference is the one kind of capture an `async move` may take out
    // of an `Fn` closure — it copies rather than moves.
    let ctx = &ctx;
    // `rule:concurrency/a-connection-is-a-root-isolate`'s isolate, between the request future that starts it and the
    // end of this function that joins it. A cell rather than a return value
    // because the two are a `hyper` connection apart: the service below is an
    // `Fn` whose futures outlive the call that made them, and the only thing
    // one of them can hand back is the response.
    //
    // At most one, because [`nvs_runtime::UpgradeSlot::fill`] refuses a second
    // upgrade on one request and a request that upgraded answers nothing more
    // on this connection: `hyper` stops framing at the `101`, so bytes a peer
    // pipelined behind the handshake are frames and travel with the socket
    // (`crate::socket`'s `Prefixed`) rather than being read as a second
    // request.
    let connection_isolate: RefCell<Option<Box<dyn Running>>> = RefCell::new(None);
    let connection_isolate = &connection_isolate;
    // § 1's socket hand-over, waiting for the two things it needs that the
    // request cannot give it: the `101` on the wire, and the descriptor back
    // from `hyper`. A request that filled the slot leaves its prepared isolate
    // here beside the `OnUpgrade` that will yield the socket, and the end of
    // this function is where the two meet.
    //
    // At most one for the same reason the cell above holds at most one, and
    // more strongly: a connection that answered `101` frames no further request
    // on this socket, so there is no second request to fill it.
    let pending_socket: RefCell<Option<nvs_runtime::Upgrade>> = RefCell::new(None);
    let pending_socket = &pending_socket;
    // `rule:concurrency/connection-bounds-are-finite`'s table and the waits
    // for this connection, copied once here from the snapshot published now,
    // rather than read per request or per hand-over: every response written
    // over this socket writes through the same send bound, and the framing at
    // the end of this function is held inside the same numbers. A reload
    // reaches the next connection.
    let (waits, bounds) = serving.connection_terms(waits);
    let send_timeout = bounds.send;
    // A request whose head has gone out and whose body is still being written.
    // At most one, because `hyper`'s h1 dispatcher writes one response at a
    // time: the next request on this connection is framed after the body of
    // this one has ended, which is the same moment [`joined_when_ended`] takes
    // what is here.
    let writing: RefCell<Option<Streamed<'_>>> = RefCell::new(None);
    let writing = &writing;
    // When this connection's event stream, if it opened one, next owes a byte.
    // Empty on every other connection and emptied again when the stream ends,
    // so the loop below arms a wake for an event stream and for nothing else —
    // `crate::bounds::wake_at` is why the arming is the loop's and not the
    // body's.
    let beat_due: crate::bounds::NextBeat = Rc::new(Cell::new(None));
    let beat_due = &beat_due;
    // When the drain period cuts the request this connection is running, if a
    // drain has begun and the request is parked or streaming its answer.
    // Filed by the loop below for the same reason as `beat_due`, and shared
    // because a streamed body ([`DrainCut`]) outlives the service's frame.
    let cut_due: Rc<Cell<Option<Instant>>> = Rc::new(Cell::new(None));
    let cut_due = &cut_due;
    let drain_period = waits.drain;
    let io = ConnectionIo::new(stream, waits).ending_at_drain(draining);
    // Taken before the adapter is handed to `hyper`, because that is the last
    // moment anything on this side can reach it.
    let phase = io.phase();
    let phase = &phase;
    let drain_seen = io.drain_seen();
    let drain_seen = &drain_seen;
    let service = service_fn(move |request: Request<Incoming>| async move {
        // A head that framed is a head that arrived: what this connection is
        // waiting for from here is the body, and then nothing until the answer
        // exists.
        phase.set(Phase::Body);
        // The two facts `rule:observability/default-series`'s request series are
        // counted with that are only readable *here*: the verb, off a request
        // the handler below takes by value, and the instant the door first had
        // it. Taken whether or not this core is metering, because a standard
        // verb's clone copies an enum and `Instant::now` is a counter read —
        // less than the branch that would ask.
        let verb = request.method().clone();
        let began = Instant::now();
        // `rule:observability/route-label-is-the-declared-name`'s label, filled
        // by the one arm below that can have it and empty for every answer the
        // door writes for itself. Owned rather than borrowed because the match
        // it is read off goes down with the request's own context
        // (`nvs_host::Isolate::answering_request`), while the status it is
        // counted beside does not exist until that context has ended.
        let mut labelled: Option<Box<str>> = None;
        // One place, and every answer below reaches it: a request the forwarded
        // walk refused, one the ceiling refused, a preflight § 2 answered, and
        // the one the handler ran. All four are responses this server wrote, so
        // all four are requests a scrape is owed — a `503` that went uncounted
        // would hide exactly the overload the series exists to show.
        let counted = |response: &Response<Answer>, route: Option<&str>| {
            crate::metrics::count_request(
                verb.as_str(),
                response.status().as_u16(),
                route,
                began.elapsed(),
            );
        };
        // `rule:config/the-config-is-an-immutable-snapshot`'s one clone, taken
        // at the request's start and not when this connection was accepted: a
        // connection carries any number of requests, so a tree read once per
        // socket would answer a request under whatever stood when its peer
        // dialled. It is taken before the door writes anything, because the
        // header set and the cross-origin policy every answer below carries are
        // this tree's (`Serving::policy`), and the program the handler runs is
        // configured by the same one.
        let snapshot = serving.current.load();
        let policy = serving.policy(&snapshot);
        // `rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing`, and it is asked here rather than once per connection
        // because what asserts it is a *header*: one connection carries many
        // requests and a proxy writes the line on each. Before the valve below,
        // for the reason the valve is before the handler — this reads borrowed
        // header bytes and allocates nothing, and a `503` that dropped HSTS
        // behind a TLS-terminating proxy would be answering with less policy
        // than the request it refused was owed.
        //
        // `rule:http-server/secure-headers-with-nothing-written`'s effective scheme is `https` here and nowhere else:
        // Novis terminates no TLS (`rule:http-server/two-deployments-and-nothing-a-proxy-owns`), so a trusted proxy's
        // `X-Forwarded-Proto` is the only thing that can assert it.
        let origin = match crate::forwarded::walk(arrival, &policy.trusted, request.headers()) {
            Ok(origin) => origin,
            // § 6's one refusal: the walk stopped on a token that was about to
            // become the client address and is not one.
            Err(crate::forwarded::Unusable) => {
                phase.set(Phase::Write);
                let mut refused = unusable_forward();
                policy.secure.fill(refused.headers_mut(), Scheme::Http);
                counted(&refused, None);
                return Ok::<_, Infallible>(refused);
            }
        };
        // The whole `Origin` goes to the handler below, because the carrier a
        // request reaches its program through is built there — this loop never
        // holds one — and both of the walk's answers belong on it
        // (`nvs_runtime::Inbound::set_peer`). What stays here is the scheme,
        // read again for `rule:http-server/secure-headers-with-nothing-written`'s header set at the end of this closure:
        // an `Origin` is `Copy`, so the two readings are one decision.
        //
        // `origin.ignored_address_header()` is § 6's one `Warn` and still has
        // nowhere to go: it goes wherever this loop's other reports go once it
        // has been given a log, and it changes nothing about what is served.
        let scheme = origin.scheme();
        // `rule:http-server/the-server-block-is-boot-class`, and this line is the *order* rather than the number:
        // the ceiling is asked before the handler is, so a refused request has
        // selected no mount, allocated no isolate, compiled nothing and run no
        // Novis code. A valve that allocated in order to refuse would not
        // protect what it exists to protect. The guard lives to the end of this
        // closure — or, for a response whose body is still being written when
        // that closure ends, to the end of the isolate writing it ([`Streamed`]
        // is what carries it there). That is the whole of what "in flight"
        // means here: a request is in flight while it is still spending a core,
        // however much of its answer is already on the wire.
        let Some(place) = serving.admission.admit() else {
            phase.set(Phase::Write);
            let mut refused = crate::admit::over_capacity();
            policy.secure.fill(refused.headers_mut(), scheme);
            counted(&refused, None);
            return Ok::<_, Infallible>(refused);
        };
        // `rule:http-server/cors-is-closed-until-origins-are-named`, asked here for the reason the valve above it is: a
        // preflight selects no mount, allocates no isolate and runs no Novis
        // code, under an open policy exactly as under a closed one — § 2
        // configures the whole answer, so an application asked to produce it
        // would be answering a question already decided above it. Under the
        // valve rather than over it, because the ceiling is what protects the
        // process and a `503` is the answer a server at capacity owes every
        // request, whatever it was going to ask. [`crate::cors`] owns which
        // status the policy gives and what it grants with it.
        if let Some(preflight) = policy.cors.preflight(request.method(), request.headers()) {
            phase.set(Phase::Write);
            let mut permitted = Response::new(Answer::empty());
            *permitted.status_mut() = preflight.status();
            policy.secure.fill(permitted.headers_mut(), scheme);
            preflight.fill(permitted.headers_mut());
            counted(&permitted, None);
            return Ok::<_, Infallible>(permitted);
        }
        // § 2's other half, decided here because the handler below takes the
        // request by value and the `Origin` it must read is on it. The valve's
        // own `503` carries none of this on purpose: it is the door's answer
        // rather than the policy's, it carries no CORS header under any policy,
        // and saying it varied by an origin nothing looked at would be a claim
        // about an answer the policy never produced. [`crate::cors`] owns the
        // rest, including why a cache is what `Vary` is for.
        let crossing = policy.cors.answer(request.headers());
        // What a request that answers with a **file** will be answered against —
        // taken here for `crossing`'s reason, the handler below owning the
        // request from the next line on, and kept rather than the whole header
        // map because a program declares a file body only after it has run:
        // there is no earlier moment at which this loop can know whether the
        // conditional and the range will be needed. [`crate::statics::asked`]
        // is the one home of which headers those are, so this line cannot come
        // to hold a different subset than the policy reads. A request that
        // named neither leaves an empty map, which allocates nothing.
        let asked = statics::asked(request.headers());
        // `rule:concurrency/a-connection-is-a-root-isolate`'s offer, taken here for `crossing`'s reason and one more:
        // `hyper` leaves an `OnUpgrade` in the extensions of a request it framed
        // an upgrade for and of no other, so this is both the last moment
        // anything on this side can read it and the whole of the question "can
        // this connection be upgraded" — asked of `hyper`'s own answer rather
        // than of a header set re-read here. An ordinary request is offered
        // nothing, which is what `nvs_runtime::Inbound::offer_upgrade` says is
        // the point.
        //
        // The slot's clone rides out to the request below; the accept key stays
        // with this connection, because the `101` it answers is this
        // connection's response and not the request's to write.
        //
        // **The `OnUpgrade` is read and never taken.** `hyper` resolves one only
        // for a caller driving `with_upgrades()`, which wants `I: Send + 'static`
        // — a bound `crate::io::ConnectionIo` cannot meet and should not, since
        // its whole point is that this connection never leaves the core that
        // accepted it. The descriptor comes back through
        // `http1::Connection::into_parts` at the end of this function instead,
        // which is the same object by a route with no `Send` on it. What the
        // extension is still good for is the question, unchanged: `hyper` leaves
        // one on a request it framed an upgrade for and on no other.
        //
        // **`hyper`'s answer is necessary and RFC 6455's opening is the rest of
        // it.** `hyper` frames an upgrade for any `Connection: Upgrade`, so what
        // it says is that this connection *can* be taken over — not that a
        // WebSocket is what asked. A handshake with no `Sec-WebSocket-Key` has
        // nothing to derive an accept from, and one naming a version other than
        // 13 speaks a framing this server does not: neither is offered the slot,
        // so the program answers it as the ordinary request it is and the peer
        // reads a status that is not `101`, which is RFC 6455's own spelling of
        // a refused handshake. No `426` and no negotiation — there is one
        // version, and a door that bargained would be a second handshake to keep
        // correct.
        let offered = request
            .extensions()
            .get::<hyper::upgrade::OnUpgrade>()
            .is_some()
            .then(|| websocket_opening(request.headers()))
            .flatten()
            .map(|accept| (nvs_runtime::UpgradeSlot::new(), accept));
        // `rule:concurrency/two-doors-one-isolate`'s cell, and the line above is the whole of what makes it
        // a second one: it is made for **every** request rather than for a
        // request `hyper` framed an upgrade for, because an event stream takes
        // nothing of this connection but the response the request already has.
        // A request that asks for no stream leaves it empty, which costs the one
        // allocation `nvs_runtime::SseSlot` documents.
        let streaming = nvs_runtime::SseSlot::new();
        // The other stream, and the cell is made on the same terms for a
        // sharper version of the same reason: a body written over time takes
        // nothing of this connection at all, being the response this request
        // already has, written in pieces. It carries the send bound above and
        // the message bound beside it, which are the two numbers of this
        // connection's a streamed body meets — a peer that stops reading, and a
        // chunk larger than the connection may hold — and nothing else in
        // `crate::bounds` is a request's to be held inside.
        let opening = stream::BodySlot::new(send_timeout, bounds.message);
        let mut answered = match handler(request, origin) {
            // Already an answer: a mount table's `404`, or a file this server is
            // sending rather than running. Nothing is started for it, so the
            // isolate accounting below does not apply to it either.
            Reply::Done(response) => response,
            // **This future may not run the request; it may only wait for it.**
            // The reason is `hyper`'s dispatcher rather than anything here: its
            // h1 loop runs `poll_read` and `poll_write` in that order on one
            // task, and this future is what `poll_write` polls. A body chunk
            // therefore only arrives on an iteration of that loop — so a frame
            // that sat here holding the core until the request had finished
            // would be waiting for a read its own frame is what owes, which is
            // not a slow path but a deadlock, reached by the first chunk of the
            // smallest body. Starting the isolate and answering `Pending` until
            // it has ended is what takes the request off this task: the
            // isolate's pull can then park the isolate, `want`'s two-way
            // signalling wakes this connection, its `poll_read` delivers the
            // chunk and wakes the isolate back.
            //
            // The supplier is the other half of that, and it is this future's
            // to drive: `crate::body::Supply` holds the `Incoming` the handler
            // could not send to the isolate, and one `pump` per poll reads a
            // chunk for a request that is waiting for one.
            Reply::Run(isolate, supply) => {
                // The label, off the match `crate::route::take` already wrote
                // onto the carrier — a name out of the compile-time table, which
                // is the whole of why this reads a match rather than a path. The
                // one arm that can have one: a `Reply::Done` is a mount table's
                // `404` or a file, neither of which matched a route, and the
                // three refusals above it answered before a unit was even
                // selected.
                labelled = isolate
                    .answering_request()
                    .and_then(crate::route::label)
                    .map(Box::from);
                // `rule:observability/four-kinds-become-a-span`'s root, taken
                // off the carrier beside the label and at the same last moment,
                // for the same reason: the trace the door decided rides on the
                // request ([`crate::trace::take`]), and `Isolate::start` is
                // about to move that request into a context this loop never
                // holds. The request line is copied here rather than derived
                // from the completion, which carries no request.
                //
                // Asked under the sampled flag, so an unsampled request pays a
                // load and a branch and never an allocation: what `spans` would
                // answer for one is an empty graph, and this is where that is
                // cheapest to know. The path carries no query string, which is
                // `rule:observability/trace-events-carry-a-kind`'s rule about
                // what a trace is a `secret` sink for.
                let recording = isolate.answering_request().and_then(|request| {
                    let trace = request.trace_context()?;
                    trace
                        .sampled()
                        .then(|| (format!("{} {}", request.method(), request.path()), trace))
                });
                // The offer reaches the program through the carrier the handler
                // built, which is why it is made here and not above: this loop
                // never holds an `Inbound`, and `Isolate::offering_upgrade` is
                // the one point at which the slot and that carrier are in the
                // same hand. A reply that answers no request is left alone by
                // it, exactly as `Reply::Done` is by having no arm here at all.
                let isolate = match offered.as_ref() {
                    Some((slot, _accept)) => isolate.offering_upgrade(slot.clone()),
                    None => isolate,
                };
                // § 5's cell, offered unconditionally beside it and in the same
                // hand for the same reason. No `match`, because there is no
                // question to ask: every request the server runs is offered
                // one, and a reply that answers no request is left alone by
                // `Isolate::offering_sse` itself.
                let isolate = isolate.offering_sse(streaming.clone());
                // The third cell, offered with no question asked for the reason
                // the second one is not asked about either: every request the
                // server runs may answer in pieces, and one that does not
                // leaves the cell empty.
                let isolate = isolate.offering_response_stream(opening.clone());
                // This request's own stop word and deadline, before the line
                // below arms anything off them: a connection carries any number
                // of requests and the tree a request is stopped in is that
                // request's, where a word held for the socket would make the
                // request after a stopped one a `FATAL` at its first poll.
                // `Ctx::reroot` owns why the pair is replaced rather than
                // cleared, and why this is ahead of the arming and not behind
                // it.
                ctx.borrow_mut().reroot();
                // The tree the door took at this request's start. It is
                // written to the connection's own context because that context
                // is this request tree's root — `Ctx::isolate` carries the
                // configuration down to the child, while the ceiling a watchdog
                // charges the tree against is read off this one
                // ([`nvs_host::Isolate::watched_by`]). Every ceiling under
                // `[limits]` and every capability an entry asks for is this
                // line: a context nobody configured states no ceiling and
                // grants nothing.
                ctx.borrow_mut().set_config(Arc::clone(&snapshot));
                // A statement of its own, because the borrow a `match`
                // scrutinee takes lives to the end of the whole `match` — and
                // the arm below borrows the same context again to collect.
                let started = isolate.start(&mut ctx.borrow_mut());
                match started {
                    Ok(running) => {
                        // An unreadable list is one the boot refused, so the
                        // `false` here is never reached by a request.
                        let disconnect =
                            nvs_config::app::disconnect_for(&snapshot.config, &snapshot.origins)
                                .ok();
                        let mut peer = Peer {
                            running: Some(running),
                            supply,
                            cancels: disconnect
                                .as_ref()
                                .is_some_and(|disconnect| disconnect.cancels(verb.as_str())),
                            grace: disconnect
                                .map_or(nvs_config::app::DISCONNECT_GRACE, |disconnect| {
                                    disconnect.grace
                                }),
                            tree: ctx.borrow().safepoint_view(),
                            draining: draining.clone(),
                            seen: drain_seen.clone(),
                            period: drain_period,
                        };
                        let mut parked = false;
                        let mut cut = false;
                        // No waker is registered, and that is the seam rather
                        // than an omission: what ends this wait is the
                        // isolate's own end waking the **task** that started it
                        // (`Isolate::start` takes this connection's `Wake`),
                        // and `rule:concurrency/one-future-per-connection`'s loop re-polls whatever the task
                        // was resumed for. A waker stored here would be a
                        // second route to the same resume.
                        let opened = std::future::poll_fn(|cx| {
                            // The one thing this wait does besides ask: a
                            // request parked on a pull has published that it
                            // wants a chunk and woken this task, and `cx` is
                            // the context `hyper`'s `Incoming` has to be
                            // polled with. Ahead of the question, so that a
                            // request whose last act is to read its body is
                            // answered on this poll rather than one later.
                            if let Some(supply) = peer.supply.as_mut() {
                                supply.pump(cx);
                            }
                            // The head, asked **before** the end below and
                            // never after it: a request that opened a stream
                            // and then ended between two polls still answers
                            // with the head it opened, where the other order
                            // would answer it with an empty buffered body and
                            // drop every byte it wrote. This poll's waker goes
                            // into the cell for the case the wait exists for —
                            // a program that opens a stream and then parks,
                            // whose head nothing else would come back to look
                            // for.
                            if let Some(head) = opening.take(cx.waker()) {
                                return Poll::Ready(Some(head));
                            }
                            if peer.finished() {
                                return Poll::Ready(None);
                            }
                            // `rule:concurrency/a-drain-closes-a-connection-cleanly`:
                            // the drain period bounds a request whose program is
                            // still running, and such a request moves no bytes, so
                            // no wait of the adapter's ends it. The cut is taken
                            // here, at the end of the same period the adapter's
                            // waits end at.
                            let cut_at = draining
                                .is_draining()
                                .then(|| drain_seen.period_ends(drain_period));
                            if cut_at.is_some_and(|at| at <= Instant::now()) {
                                cut = true;
                                return Poll::Ready(None);
                            }
                            // The drive files the wake for this instant after
                            // its pass, for `crate::bounds::wake_at`'s reason.
                            cut_due.set(cut_at);
                            parked = true;
                            Poll::Pending
                        })
                        .await;
                        cut_due.set(None);
                        // A request that made this future answer `Pending` left
                        // `hyper`'s read side blocked on the head it speculated
                        // about while the isolate ran — and a blocked read side
                        // is the one case `Conn::maybe_notify` skips its
                        // post-response read in. That read is what ends
                        // `Phase::Write` and starts the keep-alive clock
                        // (`crate::io`'s § *The clock*), so without a second
                        // poll an idle connection would sit under the write
                        // wait instead of § 5's keep-alive one. One wake is the
                        // whole fix: `rule:concurrency/a-waker-is-one-permission-to-poll`'s loop re-polls with the
                        // response written and the dispatcher idle, which is
                        // where `hyper` reads again.
                        if parked {
                            std::future::poll_fn(|cx| {
                                cx.waker().wake_by_ref();
                                Poll::Ready(())
                            })
                            .await;
                        }
                        match opened {
                            // The head goes out with the request still running,
                            // so nothing is joined here: the isolate and the
                            // place it holds move onto the connection, and the
                            // drive loop at the end of this function collects
                            // them once the body they are writing has ended.
                            Some(head) => {
                                let head = streamed(
                                    head,
                                    DrainCut {
                                        draining: draining.clone(),
                                        seen: drain_seen.clone(),
                                        period: drain_period,
                                        due: Rc::clone(cut_due),
                                    },
                                );
                                *writing.borrow_mut() = Some(Streamed {
                                    peer,
                                    _place: place,
                                    recording,
                                });
                                head
                            }
                            // The drain period ended with the program still
                            // running. The request tree is cancelled and this
                            // waits for it to end, and the client is told the
                            // server is going away.
                            None if cut => {
                                peer.cancel();
                                cut_by_the_drain()
                            }
                            // Nothing left to wait for, so this join does not
                            // park.
                            None => {
                                peer.collect(&mut ctx.borrow_mut())
                                    .map_or_else(failed, |done| {
                                        // The events the request filed, on the
                                        // completion because that is the one channel out
                                        // of a finished isolate
                                        // (`nvs_runtime::host::Completion::trace`), and
                                        // derived here because this is where the request
                                        // has ended and its graph is complete.
                                        if let Some((request, trace)) = &recording {
                                            crate::trace::record(request, trace, &done.trace);
                                        }
                                        finished(done, &asked)
                                    })
                            }
                        }
                    }
                    // The *argument* had no meaning on the other side, so no
                    // request was ever started. Everywhere else that is the
                    // parent's to raise; here the parent is a connection with
                    // nobody to raise it in, so it is one more `500`.
                    Err(_refused) => failed(),
                }
            }
        };
        // `rule:concurrency/a-connection-is-a-root-isolate` and `rule:concurrency/two-doors-one-isolate`'s other half, and the line above is what makes it
        // § 1 rather than a resumed request: the request has been joined, so
        // its arena, its carrier and everything the peer authenticated with are
        // released before anything of the connection's exists. Both cells are
        // taken unconditionally — a `Reply::Done` never filled either, and a
        // request that ran and called neither `upgrade` member leaves them
        // empty, which is the same `None` and needs no second question.
        //
        // Started from `ctx`, which is this **connection's** context and not
        // the request's: that is the whole of "a root isolate, not a child of
        // the request tree", spelled as the parent it is given rather than as
        // a rule to remember. `Output::Capture` in both doors, because what a
        // connection *echoes* is no part of a response either way: § 1's bytes
        // are frames on a socket this response is over, and § 5's events reach
        // the wire through the writing half the isolate is handed
        // (`nvs_host::Isolate::over_event_stream`) rather than through a sink,
        // so a stray `echo` between two events cannot land inside the framing a
        // client is parsing.
        //
        // Both cells are read here and § 5's contradiction is decided here,
        // which is what `nvs_runtime::SseSlot::fill` means by "decided where
        // the response is written": a request that filled both asked for a
        // socket *and* an event stream, which is two responses where this
        // connection has one, and neither cell could have seen the other
        // without being the single tagged slot § 5 spends two types to refuse.
        // Neither isolate is started, both prepared upgrades are discarded —
        // `nvs_runtime::Upgrade::discard` is what gives their arguments back,
        // since this crate forbids the `unsafe` a release takes — and the peer
        // is answered the `500` that says the server failed to build what it
        // was asked for. As with the refusals above it, there is no `catch`
        // left to report it to: the request that asked is over.
        //
        // The two are not started in the same place, and that is § 1's own
        // ordering rather than a shape this function chose: a socket isolate's
        // peer does not exist until the `101` is on the wire and `hyper` has
        // given the descriptor back, so it is prepared here and started at the
        // end of this function. An event stream's isolate takes no socket, so
        // there is nothing for it to wait for and it starts here.
        let socket = offered.as_ref().and_then(|(slot, _accept)| slot.take());
        let opened = match (socket, streaming.take()) {
            (Some(socket), Some(stream)) => {
                socket.discard();
                stream.discard();
                answered = failed();
                None
            }
            // § 1's `101`, and it **replaces** whatever the request wrote for
            // itself: this connection has one response left and the peer is
            // owed the handshake rather than a page it cannot read. The request
            // still ended normally — that is what makes the upgrade legal — and
            // its own bytes are discarded here for the reason `answer` discards
            // a failed request's, which is that a half-meant body is worse than
            // none.
            (Some(socket), None) => {
                let (_slot, accept) = offered.expect("a slot that was filled came from an offer");
                answered = switching(&accept);
                *pending_socket.borrow_mut() = Some(socket);
                None
            }
            (None, stream) => stream,
        };
        if let Some(upgrade) = opened {
            let (program, args) = upgrade.into_parts();
            // The body before the isolate that writes it, because the two halves
            // are one call: the writing half goes into the isolate and the
            // reading half is what this connection answers with, so neither can
            // be handed anywhere before both exist.
            let (events, body) = Answer::stream(&bounds);
            match Isolate::new(program, args, Output::Capture)
                .over_event_stream(events)
                .start(&mut ctx.borrow_mut())
            {
                // The head goes out here and the events after it, so this
                // **replaces** whatever the request wrote for itself, for the
                // reason § 1's `101` does: this connection has one response
                // left and the peer is owed the stream it asked for rather than
                // a page it stopped reading for.
                Ok(running) => {
                    answered = event_stream(
                        body,
                        &bounds,
                        draining,
                        waits.write_idle,
                        Rc::clone(beat_due),
                    );
                    *connection_isolate.borrow_mut() = Some(running);
                }
                // The *argument* had no meaning on the other side. Not
                // reachable through either `upgrade` member, whose own copy
                // already accepted this graph once (`nvs_stdlib::socket`'s
                // "copied twice per upgrade"), so there is no program to hand
                // it back to and no `catch` left to report it in — the request
                // that asked for a connection is over. It becomes the
                // connection's `500` for the reason the request's refusal
                // above does: the peer asked for something this server then
                // failed to build, and answering it as though it had succeeded
                // is the one thing that would be a lie.
                Err(_refused) => answered = failed(),
            }
        }
        // The request took as long as it took — a request's own runtime is
        // `rule:http-server/a-requests-blast-radius-is-bounded-at-four-tiers`'s ceiling and not a socket wait — and what remains on this
        // connection is a peer reading what it asked for.
        phase.set(Phase::Write);
        // Last, and once for every path above: `rule:http-server/secure-headers-with-nothing-written`'s set is what this
        // response carries beside whatever wrote it, and filling leaves a name
        // the answer already spelled for itself exactly as it is. § 2's answer
        // is written on the same terms and at the same point, so a response has
        // one place where policy reaches it rather than two.
        policy.secure.fill(answered.headers_mut(), scheme);
        crossing.fill(answered.headers_mut());
        // After the policy and before the answer goes back, so that what is
        // counted is the response this connection actually writes — including
        // one the `101` or an event stream replaced the request's own with. A
        // streamed body's later bytes are not in the duration: what is measured
        // is the request, and a peer that reads slowly is not one.
        counted(&answered, labelled.as_deref());
        Ok(answered)
    });
    // **Driven through a borrow rather than moved in**, which is the whole of
    // what taking the socket back costs: `http1::Connection` is `Unpin` — the
    // service's future is already boxed inside `hyper`'s own dispatcher — so
    // polling it through a `Pin::new` leaves it owned by this frame, and
    // `into_parts` below can then have it. The alternative is
    // `with_upgrades()`, whose `I: Send + 'static` this connection cannot meet
    // and should not: [`crate::io::ConnectionIo`] is an `Rc` and a socket
    // registered on the core that accepted it, and making it `Send` to satisfy
    // a bound would be saying it may move between cores.
    let mut connection = http1::Builder::new().serve_connection(io, service);
    let mut asked_to_close = false;
    let framed = block_on(std::future::poll_fn(|cx| {
        // `rule:concurrency/a-drain-closes-a-connection-cleanly`: a connection
        // idle between requests closes the moment it sees the drain, and only
        // `hyper` knows whether it is idle — the adapter's phase reads the
        // end-of-stream probe `hyper` makes during a streaming body as the
        // keep-alive wait, so the phase alone would cut a stream short. Once
        // the phase says the last answer was written, the question is put to
        // `hyper` once: an idle connection is closed on this poll, and one
        // still writing has keep-alive disabled and closes after its response.
        if !asked_to_close && phase.get() == Phase::KeepAlive && draining.is_draining() {
            asked_to_close = true;
            Pin::new(&mut connection).graceful_shutdown();
        }
        // A request that is still writing its answer may still be reading its
        // own body, and the wait that used to pump it has already returned —
        // so the pump moves here with the request. Ahead of the poll below for
        // the reason it was ahead of the question before: a chunk this task
        // already read reaches the program on this iteration rather than one
        // later. The borrow ends before the poll, which runs the service.
        if let Some(supply) = writing
            .borrow_mut()
            .as_mut()
            .and_then(|streamed| streamed.peer.supply.as_mut())
        {
            supply.pump(cx);
        }
        let polled = Pin::new(&mut connection).poll(cx);
        // The streamed half of the join above, on every poll of this connection
        // rather than at its end: a keep-alive connection frames its next
        // request as soon as this body has ended, so a join deferred to the end
        // of this function would leave one request's isolate uncollected across
        // the whole of the next one's.
        joined_when_ended(writing, &mut ctx.borrow_mut());
        // Last, and after the whole pass rather than inside it: an event stream
        // that is about to park has to be woken in time to write its keep-alive,
        // and a request the drain period cuts in time to be cut.
        // `crate::bounds::wake_at` owns why this is the only place that
        // deadline survives being filed. The cut's entry may replace an earlier
        // socket deadline. That wait is then read at the cut's wake, and it
        // ends no later than the request it belongs to.
        if polled.is_pending()
            && let Some(at) = beat_due.get().into_iter().chain(cut_due.get()).min()
        {
            crate::bounds::wake_at(at);
        }
        polled
    }))
    .unwrap_or(Ok(()));
    // `rule:concurrency/a-connection-is-a-root-isolate`'s hand-over, at the first moment both halves exist: the
    // `101` is on the wire, `hyper` has stopped framing, and the descriptor
    // under it is nobody's until this line takes it. `hyper` ends a connection
    // it answered `101` on without shutting the socket down, which is what
    // makes the same bytes a WebSocket from here.
    //
    // A connection that upgraded nothing never reaches this: `into_parts` is
    // asked for only where a request filled the slot, so the ordinary path
    // still drops the whole connection here and closes it.
    if let Some(upgrade) = pending_socket.borrow_mut().take() {
        let parts = connection.into_parts();
        // What `hyper` read past the request head travels with the stream:
        // `crate::socket`'s docs § *What `hyper` had already read* is the home
        // of why dropping it would lose a frame for a client that did not wait
        // for the handshake.
        // `rule:concurrency/connection-bounds-are-finite`'s bounds, arming here because this is the first moment
        // the descriptor is a connection rather than a request: `crate::bounds`
        // is the home of the numbers, and every wait, frame and message from
        // this line on is inside them.
        let peer = crate::socket::Framed::new(
            parts.io.into_stream(),
            parts.read_buf.into(),
            // § 7's numbers, the same ones every response on this connection
            // was written under: they are read at the top of this function,
            // which is the one place they are read.
            bounds,
            // § 7's third bullet: this server's drain, handed to the object
            // that acts on it. The close a shutdown sends is taken by the
            // connection's own loop, and [`crate::socket`]'s `receive` is
            // where that is argued.
            draining.clone(),
        );
        // § 7's per-process ceiling, and the ordering is the whole of what it
        // buys: a connection the process has no room for is told 1013 and
        // dropped *before* an isolate is allocated for it, which is
        // `crate::admit`'s rule about refusing before allocating applied to the
        // longer-lived thing.
        if peer.admitted() {
            let (program, args) = upgrade.into_parts();
            // The *argument* had no meaning on the other side, which is the one
            // refusal left at this point and the one place it cannot be
            // reported: the request that asked is over, there is no `catch` to
            // reach and a peer that has read a `101` would not understand a
            // status. The socket closes with `peer` instead, which is the only
            // honest answer left.
            if let Ok(running) = Isolate::new(program, args, Output::Capture)
                .over_socket(Box::new(peer))
                .start(&mut ctx.borrow_mut())
            {
                *connection_isolate.borrow_mut() = Some(running);
            }
        } else {
            peer.refuse();
        }
    }
    // The connection isolate outlives every request on this socket, so this is
    // where it is waited for: `hyper` has no more requests to frame, and a
    // connection task that returned here would retire with a live child, which
    // `nvs_host::Scheduler::orphan` cancels. It is the same point for both
    // doors even though they start a slice apart — an upgraded connection is
    // one whose HTTP life has ended and whose isolate now owns the socket, so
    // "after the connection future" is where its whole life is.
    //
    // The completion is dropped: nothing on this side reads a connection's
    // answer, and there is no response left to put one in.
    if let Some(running) = connection_isolate.borrow_mut().take() {
        drop(running.join(&mut ctx.borrow_mut()));
    }
    framed
}

/// The response one finished request is.
///
/// `rule:tooling/echo-always-has-a-sink`
/// 's table in code: inside an HTTP request `echo` writes to the response
/// body, and an isolate's `echo` reaches its own capture buffer, so
/// [`Completion::output`] **is** the body and no call site had to name a
/// format.
///
/// A request that failed answers `500` **with no body at all**, discarding
/// whatever it echoed before it failed. That is a decision rather than an
/// omission: a page rendered half-way is worse than none, and the failure
/// itself reaches a response only where a mode says it may
/// (`rule:errors/renderings`'s HTML rendering of a `Throwable`, in development), which is the
/// configuration slice's. Until there is a mode to ask, the fail-closed answer
/// is the status and nothing else.
/// What a response carries when nothing declared otherwise —
/// `rule:security/response-body-is-one-typed-member`
/// 's last bullet, which is what makes `rule:statements/nvs-is-the-only-open-tag`'s inline-HTML page shape
/// work with no ceremony. The runtime holds the value, so `Core\Test::request`
/// reports the same one.
const ECHOED: &str = nvs_runtime::host::ECHOED_MEDIA_TYPE;

/// What a declaration that is not a header value becomes.
///
/// `rule:http-server/a-request-resolves-in-five-steps`
/// already names this for a static file's unknown extension, and it is the
/// fail-closed answer for the same reason: `nosniff` renders it inert, so a
/// program that declared something a header cannot carry gets a body no
/// browser will execute rather than one it guesses at. Falling back to
/// [`ECHOED`] would be the opposite — the one type this server should never
/// arrive at by accident.
const UNSPELLABLE: &str = "application/octet-stream";

fn answer(mut done: Completion) -> Response<Answer> {
    // There is nowhere to move the child's returned value to — a request
    // answers with what it wrote, not with what it returned — and that field
    // carries one reference this crate would otherwise leak. `discard_value`
    // is the safe discharge, which is the whole reason it exists: this crate
    // forbids `unsafe_code`.
    done.discard_value();
    if !done.ok {
        // Ahead of the status below, and deliberately: a request that failed
        // answers `500` whatever it had declared before it failed, because the
        // declaration was about the answer it did not manage to give.
        return failed();
    }
    // `rule:security/response-body-is-one-typed-member`: the request's own body member said what these bytes are,
    // and an `echo` said nothing, which § 4 reads as HTML.
    let declared = done.content_type.as_deref().unwrap_or(ECHOED);
    let content_type =
        HeaderValue::from_str(declared).unwrap_or(HeaderValue::from_static(UNSPELLABLE));
    let mut response = Response::new(Answer::new(done.output));
    // Spec § 15's status, on the same channel and read the same way: the
    // request said what its answer means, and a request that said nothing
    // means `200`, which is what `Response::new` already built. `from_u16`
    // cannot refuse what `Core\Response::setStatus` admits — it accepts
    // `100..=999` and the member accepts `100..=599` — so the fallback here is
    // the same layer-below arrangement [`UNSPELLABLE`] is, kept rather than
    // reduced to a comment about one.
    if let Some(code) = done.status {
        *response.status_mut() = StatusCode::from_u16(code).unwrap_or(StatusCode::OK);
    }
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, content_type);
    // [`overrides`] owns the ordering and what it costs.
    overrides(&mut response, done.headers);
    response
}

/// The response one finished request is, whichever of the two bodies it has.
///
/// A request answers with the bytes it wrote or with a file it **named**, and
/// the name is both the question and the answer: `Core\Response::sendFile`
/// leaves a path on the completion and writes nothing into
/// [`Completion::output`], so there is one field to ask and no ordering to get
/// right. [`answer`] takes the first body and [`sent`] the second.
///
/// A request that failed goes to [`answer`] whatever it named, which is that
/// function's `500` reached without opening anything: a handler that declared a
/// file and then threw did not get to give the response the file was for, and
/// reading a path for a body nobody will be sent is authority spent on nothing.
fn finished(mut done: Completion, asked: &HeaderMap) -> Response<Answer> {
    match done.file_body.take() {
        Some(file) if done.ok => sent(done, &file, asked, &OnDisk),
        _ => answer(done),
    }
}

/// The response a request that named a file is — [`answer`]'s other half for
/// `rule:security/response-body-is-one-typed-member`'s file row.
///
/// **The file is answered by the static policy and by nothing here**, which is
/// the whole of this function: [`crate::statics::send`] already owns the
/// media-type table, the `ETag`, the conditional and the range
/// ([0186](/docs/decisions/0186.md) § 4), and it is the same policy in both
/// deployments. So a program that sends a file gets what a mount table serving
/// the same file gets, rather than a second answer this loop would have to keep
/// correct — and the bytes are read at the connection, one range's worth,
/// instead of ever having passed through the request.
///
/// **Nothing about the program's authority is re-asked here.** The member
/// resolved the name under the calling namespace's `fs.read` and refused
/// everything it could not send (`nvs_stdlib::response`'s `sendFile`), and this
/// side has no capability table to ask. What the policy does re-ask is whether
/// the file is *still there*: one that vanished between the two answers `404`,
/// which is its own rule rather than a hole in this one.
///
/// A declared status is honoured only over the policy's `200`. A `206`, a `304`
/// and a `416` are answers about **this representation** — the peer asked a
/// question about the bytes and got the reply that question has — while a
/// program's `setStatus` was about the response as a whole, and letting it
/// overwrite a `304` would answer a validated cache with a body it did not ask
/// for. [`overrides`] applies after, unchanged: `Content-Disposition` is how a
/// download name is set on a file this member sent, and spec § 15 gives
/// `setHeader` the last word over a policy-owned header either way.
fn sent(
    mut done: Completion,
    file: &Path,
    asked: &HeaderMap,
    source: &dyn statics::Source,
) -> Response<Answer> {
    // [`answer`]'s first line, for its reason: this crate forbids `unsafe_code`
    // and the returned value carries a reference it would otherwise leak.
    done.discard_value();
    // There is no isolate in a file, so the policy answers with a response and
    // never with something to run. The other arm is [`Reply`]'s second variant
    // rather than a case this can reach.
    let Reply::Done(mut response) = statics::send(file, asked, source) else {
        return failed();
    };
    let declared = done.status.filter(|_| response.status() == StatusCode::OK);
    if let Some(code) = declared {
        *response.status_mut() = StatusCode::from_u16(code).unwrap_or(StatusCode::OK);
    }
    overrides(&mut response, done.headers);
    response
}

/// The head of a response whose body is still being written, and the reading
/// half as that body.
///
/// [`answer`]'s other half, and what is *not* here is the difference: the
/// request has not ended, so there is no completion to read a status, a content
/// type or a header off. What the peer is told is what the request had declared
/// when it opened the stream, which `nvs_runtime::stream::Opened` carries and
/// owns the reasoning for. No fallback to [`ECHOED`] either — a stream declares
/// its own type by construction, the member that opens one takes the type as
/// its argument.
fn streamed(head: stream::Opened, cut: DrainCut) -> Response<Answer> {
    let content_type =
        HeaderValue::from_str(&head.content_type).unwrap_or(HeaderValue::from_static(UNSPELLABLE));
    let mut response = Response::new(Answer::Streaming {
        drain: head.drain,
        alive: None,
        cut: Some(cut),
    });
    if let Some(code) = head.status {
        *response.status_mut() = StatusCode::from_u16(code).unwrap_or(StatusCode::OK);
    }
    response
        .headers_mut()
        .insert(header::CONTENT_TYPE, content_type);
    overrides(&mut response, head.headers);
    response
}

/// Writes what the program declared onto the response this server built.
///
/// Last, and that ordering is the whole of what `setHeader` means: spec § 15
/// gives a program an override of a policy-owned header on one response, so
/// what the program set is written after everything this server wrote for
/// itself. `rule:http-server/policy-headers-are-runtime-class-and-setheader-wins`
/// is why the policy is not narrowing-only, and the reverse order would leave
/// the member with no effect on exactly the headers it exists to change.
///
/// A pair this crate cannot spell is dropped rather than answered with:
/// `Core\Response::setHeader` refuses a name that is not a token and a value
/// outside printable ASCII at the member, so nothing a program can write
/// arrives here — this is [`UNSPELLABLE`]'s arrangement again, kept as a layer
/// below rather than reduced to a comment about one.
///
/// `insert` and `append` are the two halves of the row's own
/// `DeclaredHeader::append`, and this is the layer that would otherwise
/// collapse a repeated name: `insert` replaces every value already under it,
/// which is what an override is and what a second `Set-Cookie` must not meet.
/// Applying the rows in order is safe because `Ctx::declare_header` holds a
/// replacing row ahead of every appending one for its name.
fn overrides(response: &mut Response<Answer>, headers: Vec<nvs_runtime::DeclaredHeader>) {
    for declared in headers {
        let name = HeaderName::try_from(&*declared.name);
        let value = HeaderValue::from_str(&declared.value);
        let (Ok(name), Ok(value)) = (name, value) else {
            continue;
        };
        if declared.append {
            response.headers_mut().append(name, value);
        } else {
            response.headers_mut().insert(name, value);
        }
    }
}

/// `400`, carrying nothing — `rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing`'s one refusal, joining
/// `rule:errors/http-message-defects`'s closed list.
///
/// No body for [`failed`]'s reason and one more of its own: the peer that would
/// read it is a proxy, the operator who needs the detail is reading a log, and
/// naming which hop of an `X-Forwarded-For` chain was unreadable would echo
/// attacker-written text back over the wire.
fn unusable_forward() -> Response<Answer> {
    let mut response = Response::new(Answer::empty());
    *response.status_mut() = StatusCode::BAD_REQUEST;
    response
}

/// RFC 6455's opening handshake, read off the request that framed an upgrade:
/// the `Sec-WebSocket-Accept` this connection would answer with, and `None`
/// where the request was not one.
///
/// **Two headers decide it and no more.** The key is what the accept is derived
/// from, so a handshake without one cannot be answered at all; the version is
/// the framing that follows the `101`, and 13 is the only one RFC 6455 defines.
/// `Connection` and `Upgrade` are deliberately not re-read here — `hyper`
/// framing an upgrade *is* that question, already answered by the code that
/// parses the request line, and a second reading is a second parser to keep in
/// step.
fn websocket_opening(headers: &header::HeaderMap) -> Option<String> {
    let thirteen = headers
        .get("sec-websocket-version")
        .is_some_and(|version| version.as_bytes() == b"13");
    if !thirteen {
        return None;
    }
    headers
        .get("sec-websocket-key")
        .map(|key| crate::socket::accept_key(key.as_bytes()))
}

/// `101`, and the three field lines RFC 6455 answers an opening with.
///
/// It carries no body, and `hyper` treats it as the switch it is: the bytes
/// after this response are frames, and the descriptor is handed back through
/// the `OnUpgrade` this connection kept.
///
/// A malformed accept cannot reach here — [`websocket_opening`] derived it from
/// the peer's own key and base64 is header-safe — so the fallible spelling of
/// building the value is discharged with a `500`, which is the same answer the
/// rest of this door gives a thing it failed to build.
fn switching(accept: &str) -> Response<Answer> {
    let Ok(accept) = HeaderValue::from_str(accept) else {
        return failed();
    };
    let mut response = Response::new(Answer::empty());
    *response.status_mut() = StatusCode::SWITCHING_PROTOCOLS;
    let headers = response.headers_mut();
    headers.insert(header::UPGRADE, HeaderValue::from_static("websocket"));
    headers.insert(header::CONNECTION, HeaderValue::from_static("Upgrade"));
    headers.insert(header::SEC_WEBSOCKET_ACCEPT, accept);
    response
}

/// `rule:concurrency/two-doors-one-isolate`'s `200 text/event-stream`, over the
/// body a connection isolate writes its events into.
///
/// [`switching`] one door over, and the same replacement: a request that opened
/// an event stream is answered with this instead of with what it wrote for
/// itself. What differs is that there is a body — § 1's hand-over takes the
/// socket and this one takes nothing but the response the request already had.
///
/// **The status is `200` because the protocol says so**, which is why nothing
/// here reads what the request declared: `Core\Sse::stream` refuses a program
/// that set one, and door one's program is a different isolate that never saw
/// this response at all. The media type and the field lines beside it are
/// [`nvs_runtime::sse`]'s, read from there by both doors so that one event
/// stream cannot arrive with a head the other would not have sent.
///
/// A name or a value this crate could not spell is skipped rather than
/// answered, which is [`overrides`]' own shape and unreachable here: these are
/// constants, and a constant that could not be a header would fail the case
/// below before it reached a peer.
///
/// **The stream's own bounds are armed here**, which is the first moment there
/// is a response to arm them on, and `crate::bounds::EventStream` is what they
/// are. An event stream that sends nothing still has to move a byte before
/// `nvs_config::server::Waits::write_idle` closes the connection it is being
/// written over, and a stream saying nothing is the ordinary state of one
/// rather than a fault; and a stream that never goes quiet is closed at the
/// connection's lifetime regardless. Which of `crate::bounds::Connection`'s
/// fields an event stream reads, and which one it must not, is that type's.
fn event_stream(
    body: Answer,
    bounds: &crate::bounds::Connection,
    draining: &Draining,
    write_idle: Duration,
    due_at: crate::bounds::NextBeat,
) -> Response<Answer> {
    let mut response = Response::new(body.as_an_event_stream(bounds, draining, write_idle, due_at));
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(nvs_runtime::sse::MEDIA_TYPE),
    );
    for (name, value) in nvs_runtime::sse::DECLARED_HEADERS {
        let (Ok(name), Ok(value)) = (HeaderName::try_from(name), HeaderValue::from_str(value))
        else {
            continue;
        };
        headers.insert(name, value);
    }
    response
}

/// `500`, carrying nothing — `answer`'s docs own why the body is empty.
fn failed() -> Response<Answer> {
    let mut response = Response::new(Answer::empty());
    *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
    response
}

/// `503` with `Connection: close`, for a request the drain period ended while
/// its program was still running
/// (`rule:concurrency/a-drain-closes-a-connection-cleanly`).
///
/// [`Reply::draining`]'s status, and no `Retry-After` for its reason: this
/// process is on its way out. The client gets a status it can retry on
/// another instance, where closing the connection with no answer would look
/// like a network failure.
fn cut_by_the_drain() -> Response<Answer> {
    let mut response = Response::new(Answer::empty());
    *response.status_mut() = StatusCode::SERVICE_UNAVAILABLE;
    response
        .headers_mut()
        .insert(header::CONNECTION, HeaderValue::from_static("close"));
    response
}

/// A listening socket [`serve_on_this_core`] accepts on, and the whole of what
/// differs between the families it may be.
///
/// `rule:http-server/a-unix-socket-listener` gives the server a second
/// transport and says nothing about a connection differs past the accept, so
/// the accept is where the difference is spent: one method, answering the
/// connection as [`NvsConnection`] and who connected as an [`Arrival`]. That
/// second half is why the trait is here rather than in `nvs-host` — deriving
/// an arrival is deciding what the operating system just asserted about trust
/// (`rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing`), and
/// that reading belongs in the crate that applies it.
///
/// Object-safe on purpose: a boot whose `[server] listen` mixes the two
/// families holds one collection of listeners, so `nvs serve` hands each core a
/// `dyn Listening` and the loop above is generic over `?Sized`.
pub trait Listening {
    /// The next connection, or `None` for a park that ended without one —
    /// [`nvs_host::NvsAcceptor::accept_or_woken`], with the arrival derived.
    ///
    /// # Errors
    ///
    /// The socket's own, on that function's terms.
    fn arrived_or_woken(&mut self) -> io::Result<Option<(NvsConnection, Arrival)>>;
}

impl Listening for NvsListener {
    /// The peer is taken from the accept rather than asked of the socket
    /// afterwards: this is the one place where who connected is a fact the
    /// operating system has just stated, and a `peer_addr` later would be
    /// re-deriving it from a descriptor that may already have failed.
    fn arrived_or_woken(&mut self) -> io::Result<Option<(NvsConnection, Arrival)>> {
        Ok(self
            .accept_or_woken()?
            .map(|(stream, peer)| (NvsConnection::Tcp(stream), Arrival::Tcp(peer.ip()))))
    }
}

#[cfg(unix)]
impl Listening for nvs_host::NvsUnixListener {
    /// The peer is not read at all. A Unix-domain socket's address is a path or
    /// nothing, never an address a forwarded header could be checked against,
    /// and `rule:http-server/a-unix-socket-listener` makes the connection
    /// trusted for the filesystem's own reason: what decided who may connect
    /// was the mode on the socket.
    fn arrived_or_woken(&mut self) -> io::Result<Option<(NvsConnection, Arrival)>> {
        Ok(self
            .accept_or_woken()?
            .map(|(stream, _)| (NvsConnection::Unix(stream), Arrival::Unix)))
    }
}

/// Accepts on `listener`, giving every connection its own coroutine, until
/// `draining` begins, `keep_serving` breaks, or the listener itself fails.
///
/// Called from a task on a core: the connections are its children, which is
/// this module's docs § *One connection per coroutine*. `keep_serving` is asked
/// after each connection has been handed over — a server that runs until the
/// process ends answers `ControlFlow::Continue(())` every time, and a test that
/// wants one connection answers `Break`. It is a callback rather than a flag
/// because what stops a server is a decision the caller owns
/// (`rule:config/the-config-is-an-immutable-snapshot`'s
/// control socket is one such caller) and this loop has no business polling for
/// it.
///
/// `admission` is shared with every other core rather than cloned per core,
/// which is `rule:http-server/the-server-block-is-boot-class`'s "counted process-wide": a per-core share would let
/// one hot core refuse while its neighbours idle. Every connection this loop
/// hands over gets a handle on the same count.
///
/// `report` is handed the one line [`AcceptBackoff`] produces per window, for
/// the reason [`crate::admit::Ceiling::clamp_note`] hands its own back as a
/// `String`: this crate is given a socket and not a logger, and the boot that
/// chose where a note goes is the one that owns writing it there.
///
/// `draining` is read at the top of every pass and written by the tail. Read,
/// because what stops a process — a terminating signal, an operator on the
/// control socket — begins the drain from another thread entirely, and a loop
/// parked in `accept` would otherwise find out when the next connection
/// arrived: the wake registered for the loop's whole life is what ends that
/// park, and [`nvs_host::wake_at_drain`] owns its contract. Written, because a
/// loop that stopped for `keep_serving` is draining too, and § 5's probe
/// answers `503` through [`Reply::health`] from that line on — the tail below
/// is that drain. Handed in rather than returned because the handler is built
/// before the loop is, and it is what the probe is answered from.
///
/// `waits` are the boot's, handed to every connection, which trades them for
/// a reloaded snapshot's own ([`serve_connection`]). A connection already
/// accepted keeps the waits it started with: `header_timeout` and
/// `keepalive_timeout` apply before any Novis code exists on it, so moving
/// them under an open socket is a promise two of the four could not keep.
///
/// `serving` is handed to every connection by clone rather than by copy, which
/// is what [`Serving`]'s own docs say it is for: one valve, one header set and
/// one configuration for the whole process, not one of each per core.
///
/// # Errors
///
/// The listener's own, which ends the whole loop — a listening socket that
/// cannot accept is not a condition the next iteration recovers from. The one
/// exception is descriptor exhaustion, which [`AcceptBackoff`] waits out rather
/// than returning: it is a condition of the process and of every other core's
/// listener too, so ending this loop would turn a transient shortage into a
/// server that stays down after it clears (`rule:http-server/the-accept-loop-backs-off`).
/// [`nvs_host::NvsListener::accept`] already retries the failures that belong
/// to one connection rather than to the socket. Also `Other` when this is
/// called off a task, because there is then no parent to put a connection
/// under and serving it on this stack would silently be a one-connection
/// server.
pub fn serve_on_this_core<L, H>(
    listener: &mut L,
    handler: &Rc<H>,
    waits: Waits,
    serving: &Serving,
    draining: &Draining,
    mut report: impl FnMut(&str),
    mut keep_serving: impl FnMut() -> ControlFlow<()>,
) -> io::Result<()>
where
    L: Listening + ?Sized,
    H: Fn(Request<Incoming>, Origin) -> Reply + 'static,
{
    // Taken once, and it is also the check that this is a task at all: a wake
    // exists exactly when `spawn_child` has a parent to hang a child off.
    let Some(parent) = Wake::current().map(Rc::new) else {
        return Err(io::Error::other(
            "the accept loop must run as a task on a core",
        ));
    };
    // `rule:observability/a-registry-is-per-core-and-nothing-reads-it`'s
    // registry, taken here because this is the core that will serve through it
    // and because the tree naming the exporter is on [`Serving`] already.
    // `[metrics]` reloads (`rule:config/reloadability-is-its-own-field`), so each
    // accepted connection compares the published snapshot with the one this core
    // was last metered from, and meters it again where they differ: a new
    // exporter or `max_series` reaches the core without a counter resetting.
    // Nothing is built where no exporter is named, and every count below is then
    // a branch — `crate::metrics::meter_this_core` owns why a registry that
    // already exists is adopted rather than started again.
    let mut metered = serving.current.load();
    crate::metrics::meter_this_core(&metered.config);
    // The drain every connection this core accepts under one tree reads, made
    // again only when an accept finds a newer tree published. So an accept
    // costs no lock and no registration while the tree stays the same.
    let mut accepted_under = (
        metered.generation,
        draining.for_connection(&serving.generations, metered.generation),
    );
    let outstanding = Rc::new(Cell::new(0_usize));
    let mut backoff = AcceptBackoff::default();
    // One registration for the whole loop rather than one per park: this task
    // outlives every connection it accepts, so a wake taken per accept would be
    // a cross-thread handle issued and dropped once per *connection served* on
    // the one path a server spends its life in. It is dropped at the tail, by
    // which point the drain it was waiting for has either fired it or is over.
    let woken_at_drain = nvs_host::wake_at_drain(draining.bit());

    loop {
        // Read here rather than only through `keep_serving` below, because the
        // two answers arrive differently: a caller's seam is a decision this
        // loop asks for after a connection, and a drain is a fact some other
        // thread published while this one was parked in `accept`.
        if draining.is_draining() {
            break;
        }
        let (stream, arrival) = match listener.arrived_or_woken() {
            // The park ended and named nothing — the drain above is what this
            // goes back round to read.
            Ok(None) => continue,
            Ok(Some(accepted)) => {
                backoff.accepted();
                accepted
            }
            Err(err) => {
                // `rule:http-server/the-accept-loop-backs-off`. `after` answering `None` is every other
                // failure, and those still end the loop on the terms above.
                let Some((wait, note)) = backoff.after(&err, Instant::now()) else {
                    return Err(err);
                };
                if let Some(note) = note {
                    report(&note);
                }
                // The core is handed back for the wait, so the connections this
                // loop already spawned keep being served through a shortage
                // that is the process's and not theirs. A cancelled wait is the
                // task being torn down, which is the tail's own answer below.
                if matches!(nvs_host::sleep(wait), Woken::Cancelled) {
                    break;
                }
                continue;
            }
        };
        let published = serving.current.load();
        if published.generation != accepted_under.0 {
            accepted_under = (
                published.generation,
                draining.for_connection(&serving.generations, published.generation),
            );
        }
        if !Arc::ptr_eq(&published, &metered) {
            crate::metrics::meter_this_core(&published.config);
            metered = published;
        }
        let handler = Rc::clone(handler);
        // `Arc`s and not `Rc`s: § 5's valve is counted process-wide and `rule:http-server/secure-headers-with-nothing-written`
        // 's header set is one policy for the whole server, so what a
        // connection clones is shared by every core rather than by every
        // connection on this one.
        let serving = serving.clone();
        // The connection's own drain, which this server's stop and a reload
        // that replaces the tree it was accepted under both begin.
        // `rule:concurrency/connection-bounds-are-finite`'s shutdown close is
        // taken by a connection isolate's own loop, so what the drain needs is
        // a handle on the far side of the hand-over. The health probe and
        // `Core\Server::isDraining()` read the server's drain, never this one.
        let draining_here = accepted_under.1.clone();
        // Counted in *here* rather than inside the body, so that a connection
        // handed over is already outstanding by the time the shutdown below can
        // look; `Served`'s `Drop` is what counts it back out, and it is a drop
        // and not a line at the end of the body because a cancelled coroutine
        // is torn down where it parked and never reaches one.
        outstanding.set(outstanding.get() + 1);
        let served = Served {
            outstanding: Rc::clone(&outstanding),
            parent: Rc::clone(&parent),
        };
        let spawned = spawn_child(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |ctx| {
            let _served = served;
            // This task's context is the root of one connection's request tree,
            // and `OutputSink::Sink` because a connection of itself writes
            // nothing: a request's bytes are captured by its own isolate and
            // come back as data (`rule:tooling/echo-always-has-a-sink`).
            //
            // A connection's own failure is the connection's. There is nobody
            // to report a reset peer to, the socket that would carry the report
            // is the one that failed, and the accept loop above must not stop
            // for it — the log this belongs in is the slice that gives this
            // loop a configuration.
            drop(serve_connection(
                stream,
                arrival,
                ctx,
                handler.as_ref(),
                waits,
                &serving,
                &draining_here,
            ));
        });
        if spawned.is_none() {
            // Unreachable while `Wake::current` answered above, and the guard
            // went down with the closure that was never run, so the tally is
            // already honest.
            return Err(io::Error::other(
                "the accept loop must run as a task on a core",
            ));
        }
        if keep_serving().is_break() {
            break;
        }
    }

    // Deregistered before the tail, since what it was to wake this task for has
    // either happened or is no longer something this loop would act on.
    drop(woken_at_drain);
    // `rule:http-server/the-server-block-is-boot-class`'s drain begins here, and the tail below *is* the drain: this
    // loop has stopped accepting, so from this line the probe answers `503` and
    // a proxy can take the instance out of rotation while the connections
    // already accepted are still being answered. Marked before the park rather
    // than after it, since a drain that announced itself once it was over would
    // report the one state nobody can act on. The child spawned by the last
    // iteration has not run yet — it cannot, until this task parks — so the
    // request on it sees the drain, which is the answer a shutdown wants.
    draining.begin();

    // `rule:concurrency/nothing-is-still-running-when-a-call-returns`, and it is the whole reason this function has a tail: the
    // connections are this task's children, so a loop that simply returned
    // would take every connection still being served down with it. It parks
    // instead, and each connection's guard wakes it on the way out. The park is
    // not a lost wakeup: one core runs one task at a time, so no child can
    // finish between the read of the tally and the suspend below.
    while outstanding.get() > 0 {
        let resumed = suspend_current(Waiting::Parked);
        // Cancelled: the caller is being torn down and the children go with it,
        // which is § 4's own answer and not something to wait out. Not
        // suspended: there is no core to hand back, so there is no turn in
        // which a child could ever run and this would spin.
        if resumed.cancelled() || !resumed.suspended() {
            break;
        }
    }
    Ok(())
}

/// The first wait a descriptor-exhausted `accept` parks for.
///
/// Short enough that a shortage clearing in a millisecond costs a millisecond,
/// which is what keeps this off the price of an ordinary burst.
const FIRST_WAIT: Duration = Duration::from_millis(10);

/// The longest wait, and therefore the bound on how late an accept can be once
/// descriptors are back: one attempt a second is a cost the loop can carry
/// indefinitely, and a shortage that has lasted this long is not one the next
/// attempt is going to clear either.
const LONGEST_WAIT: Duration = Duration::from_secs(1);

/// How often an episode is reported. `rule:http-server/the-accept-loop-backs-off`'s own reason for a window
/// rather than a line per attempt: at [`LONGEST_WAIT`] the log alone would be a
/// line a second for as long as the condition lasts.
const REPORT_WINDOW: Duration = Duration::from_secs(60);

/// The two error numbers § 8 names, because `io::ErrorKind` has no stable
/// variant for either: `EMFILE` and `ENFILE` on Unix, and on Windows
/// `WSAEMFILE` with the file-table exhaustion that reaches an `accept` through
/// the same door.
#[cfg(unix)]
const EXHAUSTED: &[i32] = &[24, 23];
#[cfg(windows)]
const EXHAUSTED: &[i32] = &[10024, 4];
#[cfg(not(any(unix, windows)))]
const EXHAUSTED: &[i32] = &[];

/// Whether `err` is the process running out of descriptors rather than anything
/// about this socket.
fn out_of_descriptors(err: &io::Error) -> bool {
    err.raw_os_error()
        .is_some_and(|code| EXHAUSTED.contains(&code))
}

/// `rule:http-server/the-accept-loop-backs-off`'s bounded backoff, and the once-per-window note that comes with
/// it.
///
/// An `accept` that failed with `EMFILE`/`ENFILE` returns immediately and will
/// fail the same way the instant it is retried, so a loop that retries at once
/// is a core pinned at full utilisation for as long as the shortage lasts and a
/// disk filled at the speed of the loop. The wait below bounds the first, the
/// window bounds the second, and neither costs an accepting listener anything:
/// [`Self::accepted`] puts the whole thing back to its default.
///
/// The wait doubles from [`FIRST_WAIT`] to [`LONGEST_WAIT`] rather than parking
/// for a fixed span, because the two shortages this covers want opposite
/// answers — a burst of connections that clears in a millisecond, and a leak
/// that will still be there in a minute — and doubling is what asks the first
/// question first.
#[derive(Debug, Default)]
pub(crate) struct AcceptBackoff {
    /// What the previous exhausted `accept` parked for, and `None` whenever the
    /// listener is accepting: the state that makes an episode an episode.
    waited: Option<Duration>,
    /// When this episode was last reported. `None` reports at once, which is
    /// what makes the first exhaustion after a recovery a line rather than
    /// silence inside a window opened by an episode already over.
    reported: Option<Instant>,
}

impl AcceptBackoff {
    /// The listener accepted: the episode is over and the next one reports.
    pub(crate) fn accepted(&mut self) {
        *self = Self::default();
    }

    /// How long to park after `err`, and the line to log when this is the first
    /// exhaustion in a window.
    ///
    /// `None` is every failure that is not descriptor exhaustion — which the
    /// caller ends the loop on, unchanged.
    pub(crate) fn after(
        &mut self,
        err: &io::Error,
        now: Instant,
    ) -> Option<(Duration, Option<String>)> {
        if !out_of_descriptors(err) {
            return None;
        }
        let wait = match self.waited {
            None => FIRST_WAIT,
            Some(previous) => (previous * 2).min(LONGEST_WAIT),
        };
        self.waited = Some(wait);
        let due = self
            .reported
            .is_none_or(|at| now.duration_since(at) >= REPORT_WINDOW);
        if !due {
            return Some((wait, None));
        }
        self.reported = Some(now);
        Some((
            wait,
            Some(format!(
                "the accept loop is out of file descriptors and is backing off \
                 up to {LONGEST_WAIT:?} an attempt, reporting once per \
                 {REPORT_WINDOW:?}: {err}"
            )),
        ))
    }
}

/// One connection's place in the accept loop's tally, given back however that
/// connection's task ended.
///
/// A guard rather than a decrement at the end of the body: `rule:concurrency/cancellation-runs-no-user-code`'s
/// cancellation tears a coroutine down where it parked, so the end of the body
/// is exactly the line a cancelled connection never reaches. The wake is here
/// too, because a shutdown that is parked on the tally has to hear about the
/// same event however it happened.
struct Served {
    /// The loop's count of connections handed over and not yet finished.
    outstanding: Rc<Cell<usize>>,
    /// The accepting task, which may be parked on that count reaching zero.
    parent: Rc<Wake>,
}

impl Drop for Served {
    fn drop(&mut self) {
        self.outstanding.set(self.outstanding.get() - 1);
        // Waking a task that is not parked does nothing, which is the ordinary
        // case: the loop is usually still in `accept`.
        self.parent.wake();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::admit::Ceiling;
    use nvs_config::Capacity;
    use nvs_host::{Output, Program};
    use nvs_runtime::Value;
    use std::io::{Read as _, Write as _};
    use std::net::TcpStream;
    use std::time::Duration;

    /// How long a test waits on a core before it gives up: the read deadline
    /// every client here sets, and the bound [`run_the_core`] drives under.
    ///
    /// Comfortably above every wait these tests configure and comfortably below
    /// any default, so a phase machine that never armed the wait under test
    /// fails the assertion instead of hanging the suite: the client's own close
    /// is what lets the accept loop finish and the test report.
    const CLIENT_PATIENCE: Duration = Duration::from_secs(10);

    /// Drives `sched` to the end of its work, the way `nvs serve` drives a
    /// worker rather than the way a one-shot program does.
    ///
    /// [`nvs_host::run_until_idle`] returns as soon as one blocking poll wakes
    /// nothing, and `rule:concurrency/the-reactor-reports-readiness` is why:
    /// the readiness it collected belongs to no parked task, and continuing
    /// would spin on a report nobody owns. Readiness for a task that has
    /// already ended is the ordinary way that happens, and **whether the
    /// platform reports it at all is the platform's**: a closed descriptor
    /// leaves an `epoll` set silently, where a completion already in flight for
    /// one is still delivered. So a single call ends a run that is still
    /// accepting — the accept loop is a parked task — at whichever stale wake
    /// came first, on some targets and not others.
    ///
    /// The loop is `nvs-cli`'s worker loop, which takes a report with anything
    /// still parked as a turn to take rather than an end. The deadline is what
    /// a test adds on top of it, so a core that really is stuck fails here
    /// instead of hanging the suite.
    fn run_the_core(sched: &mut nvs_host::Scheduler) -> nvs_host::RunReport {
        let give_up_at = Instant::now() + CLIENT_PATIENCE;
        let mut total = nvs_host::RunReport::default();
        loop {
            let turn = nvs_host::run_until_idle(sched).expect("the loop failed");
            total.resumes += turn.resumes;
            total.finished += turn.finished;
            total.cancelled += turn.cancelled;
            total.parked = turn.parked;
            if turn.parked == 0 {
                return total;
            }
            assert!(
                Instant::now() < give_up_at,
                "the core never went idle: {} task(s) are still parked, so whatever the run was \
                 waiting on never arrived",
                turn.parked
            );
        }
    }

    /// One finished request, as [`answer`] takes it.
    fn completed(output: &str, content_type: Option<&str>) -> Completion {
        Completion {
            ok: true,
            value: Value::null(),
            output: output.as_bytes().to_vec(),
            content_type: content_type.map(Into::into),
            file_body: None,
            status: None,
            headers: Vec::new(),
            error: None,
            wall: None,
            trace: Vec::new(),
        }
    }

    /// `rule:security/response-body-is-one-typed-member`: the body member a request used is what decides the
    /// `Content-Type`, and an `echo` that used none means HTML.
    ///
    /// This is the only place in the tree where a declaration is observable —
    /// a `.nvst` case runs a program and can assert the body bytes, never the
    /// header — so `Core\Response`'s conformance cases pin the body and this
    /// pins the half they cannot reach.
    #[test]
    fn a_declared_media_type_is_the_responses_content_type() {
        let declared = answer(completed("{}", Some("application/json")));
        assert_eq!(
            declared.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json",
            "a body member's declaration did not reach the response"
        );
        let echoed = answer(completed("<p>hi</p>", None));
        assert_eq!(
            echoed.headers().get(header::CONTENT_TYPE).unwrap(),
            ECHOED,
            "a request that only echoed did not get § 4's HTML default"
        );
    }

    /// The files a case sends, with one `mtime` for all of them — a fake source
    /// rather than a directory, because what these two cases assert is the route
    /// from a completion into [`crate::statics`] and never the disk under it.
    struct Files(&'static [(&'static str, &'static [u8])]);

    /// The pair the file cases send: two extensions, so the media type asserted
    /// below is read off the name rather than off a constant.
    const FILES: Files = Files(&[("page.html", b"<p>hi</p>"), ("notes.txt", b"plain")]);

    impl Files {
        fn at(&self, path: &Path) -> Option<&'static [u8]> {
            self.0
                .iter()
                .find(|(name, _)| Path::new(name) == path)
                .map(|(_, bytes)| *bytes)
        }
    }

    impl statics::Source for Files {
        fn stat(&self, path: &Path) -> Option<statics::Stat> {
            Some(statics::Stat {
                len: u64::try_from(self.at(path)?.len()).ok()?,
                mtime_nanos: 7,
            })
        }

        fn read(&self, path: &Path, at: u64, len: u64) -> Option<Vec<u8>> {
            let bytes = self.at(path)?;
            let at = usize::try_from(at).ok()?;
            let len = usize::try_from(len).ok()?;
            bytes.get(at..at.checked_add(len)?).map(<[u8]>::to_vec)
        }
    }

    /// One finished request that named `file` with `Core\Response::sendFile`,
    /// answered against the request headers `asked` and whatever status the
    /// program declared.
    fn named(file: &str, asked: &[(&str, &str)], status: Option<u16>) -> Response<Answer> {
        let mut headers = HeaderMap::new();
        for (name, value) in asked {
            headers.insert(
                HeaderName::from_bytes(name.as_bytes()).expect("a header name"),
                HeaderValue::from_str(value).expect("a header value"),
            );
        }
        let mut done = completed("", None);
        done.status = status;
        sent(done, Path::new(file), &headers, &FILES)
    }

    /// `rule:security/response-body-is-one-typed-member`'s file row, answered by
    /// the one static policy: a program hands over a **name**, so what the bytes
    /// are called is [`crate::statics`]'s media-type table and not this loop's.
    ///
    /// Two extensions rather than one, and neither is [`ECHOED`]: a path that
    /// declared a constant which happened to match would pass against a single
    /// file and fails against a pair. The `404` and the `500` beside them are
    /// the two ways there are no bytes to send — the file went between the
    /// member's answer and this one, and the handler threw after naming it —
    /// which is what makes the sent half a bound named on both sides.
    #[test]
    fn a_declared_file_body_is_streamed_under_the_static_policys_media_type() {
        let page = named("page.html", &[], None);
        assert_eq!(page.status(), StatusCode::OK);
        assert_eq!(
            page.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/html; charset=utf-8",
            "a declared file body did not take the static policy's media type"
        );
        assert_eq!(
            page.body().bytes().to_vec(),
            b"<p>hi</p>".to_vec(),
            "the file's own bytes are the body"
        );
        let notes = named("notes.txt", &[], None);
        assert_eq!(
            notes.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/plain; charset=utf-8",
            "the media type is read off the name, not declared once for every file"
        );
        assert_eq!(notes.body().bytes().to_vec(), b"plain".to_vec());

        let gone = named("vanished.html", &[], None);
        assert_eq!(
            gone.status(),
            StatusCode::NOT_FOUND,
            "a file that went between the member's answer and this one is the policy's 404"
        );

        let mut threw = completed("", None);
        threw.ok = false;
        threw.file_body = Some(Path::new("page.html").into());
        let refused = finished(threw, &HeaderMap::new());
        assert_eq!(
            refused.status(),
            StatusCode::INTERNAL_SERVER_ERROR,
            "a request that named a file and then failed still answers 500"
        );
        assert!(
            refused.body().bytes().is_empty(),
            "nothing was opened for a response that is not going to be given"
        );
    }

    /// The rest of the policy reaches a program's file too — the conditional and
    /// the range — which is the whole reason `sendFile` hands over a name
    /// instead of bytes.
    ///
    /// The validator is taken from the first answer rather than computed here:
    /// what is asserted is that the two answers are about one representation,
    /// which a case rebuilding the `ETag` itself would assert of its own
    /// arithmetic. The declared status rides along on both halves, because the
    /// direction that matters is the one where it does **not** win: a `201`
    /// overwriting a `304` would answer a validated cache with a body it did not
    /// ask for, while the same `201` over the policy's `200` is spec § 15's
    /// `setStatus` doing exactly what it says.
    #[test]
    fn a_declared_file_body_answers_a_range_and_a_conditional_request() {
        let whole = named("page.html", &[], None);
        let tag = whole
            .headers()
            .get(header::ETAG)
            .expect("the policy states a validator")
            .to_str()
            .expect("a validator is ASCII")
            .to_owned();

        let fresh = named("page.html", &[("if-none-match", &tag)], Some(201));
        assert_eq!(
            fresh.status(),
            StatusCode::NOT_MODIFIED,
            "a program's file body did not answer its own validator"
        );
        assert!(fresh.body().bytes().is_empty(), "a 304 carries no body");

        let part = named("page.html", &[("range", "bytes=3-5")], None);
        assert_eq!(part.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(
            part.headers().get(header::CONTENT_RANGE).unwrap(),
            "bytes 3-5/9",
            "the range is stated against the whole file's length"
        );
        assert_eq!(
            part.body().bytes().to_vec(),
            b"hi<".to_vec(),
            "only the asked-for bytes were read"
        );

        let declared = named("page.html", &[], Some(201));
        assert_eq!(
            declared.status(),
            StatusCode::CREATED,
            "setStatus has no effect on a file body the policy answered 200 for"
        );
    }

    /// A declaration a header cannot carry becomes the inert type rather than
    /// the HTML one — [`UNSPELLABLE`]'s own reasoning, asserted because the
    /// fail-open direction is the one that would look identical in every other
    /// test in this file.
    ///
    /// `Core\Response::bytes` refuses such a string at the member, so nothing
    /// a program can write reaches here today; this is the layer below that,
    /// and it is asserted so that it stays a layer rather than becoming a
    /// comment about one.
    #[test]
    fn a_media_type_no_header_can_carry_is_inert_and_not_html() {
        let smuggled = answer(completed("body", Some("text/html\r\nX-Injected: yes")));
        assert_eq!(
            smuggled.headers().get(header::CONTENT_TYPE).unwrap(),
            UNSPELLABLE,
            "an unspellable declaration did not fail closed"
        );
        assert_eq!(
            smuggled.headers().get("x-injected"),
            None,
            "a header value carried a second header into the response"
        );
    }

    /// Spec § 15's status crosses the way the media type does, and a request
    /// that declared none is `200` — pinned here for [`answer`]'s own reason:
    /// a `.nvst` case can assert a body and never a status line.
    ///
    /// The third assertion is the one with a direction to get wrong. A request
    /// that failed answers `500` whatever it declared, because a declaration
    /// describes the answer the program meant to give and a failed one did not
    /// give it; the opposite would let a handler that threw halfway through
    /// still tell the peer it had succeeded.
    #[test]
    fn a_declared_status_is_the_responses_status_and_a_failure_outranks_it() {
        let mut declared = completed("gone", None);
        declared.status = Some(410);
        assert_eq!(
            answer(declared).status(),
            StatusCode::GONE,
            "a `setStatus` did not reach the response"
        );
        assert_eq!(
            answer(completed("hi", None)).status(),
            StatusCode::OK,
            "a request that declared no status did not answer `200`"
        );
        let mut threw = completed("half a body", None);
        threw.status = Some(201);
        threw.ok = false;
        assert_eq!(
            answer(threw).status(),
            StatusCode::INTERNAL_SERVER_ERROR,
            "a failed request answered with the status it had declared"
        );
    }

    /// Spec § 15's `setHeader` reaches the response, and a request that failed
    /// carries none of what it set — pinned here for the reason the three tests
    /// above are: a `.nvst` case can assert a body and never a header line.
    ///
    /// What is *not* asserted here is the override itself, because there is
    /// nothing to override yet: the one header this server writes for itself is
    /// `Content-Type`, which the member refuses outright since a body member
    /// owns it. The ordering in [`answer`] is what `rule:http-server/secure-headers-with-nothing-written`'s policy set
    /// will be overridden by when it lands, and `insert` rather than `append`
    /// is what makes that a replacement.
    #[test]
    fn a_declared_header_reaches_the_response_and_a_failure_drops_it() {
        let mut declared = completed("{}", Some("application/json"));
        declared.headers = vec![
            nvs_runtime::DeclaredHeader::set("X-Request-Id", "9f2"),
            nvs_runtime::DeclaredHeader::set("Cache-Control", "no-store"),
        ];
        let answered = answer(declared);
        assert_eq!(
            answered.headers().get("x-request-id").unwrap(),
            "9f2",
            "a `setHeader` did not reach the response"
        );
        assert_eq!(
            answered.headers().get(header::CACHE_CONTROL).unwrap(),
            "no-store",
            "the second of two declared headers did not reach the response"
        );
        assert_eq!(
            answered.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/json",
            "a declared header displaced the body member's media type"
        );
        let mut threw = completed("half a body", None);
        threw.headers = vec![nvs_runtime::DeclaredHeader::set("X-Request-Id", "9f2")];
        threw.ok = false;
        assert_eq!(
            answer(threw).headers().get("x-request-id"),
            None,
            "a failed request answered with a header it had declared"
        );
    }

    /// A name declared twice survives this layer, which is the half of the
    /// `Set-Cookie` path that lives here: `insert` and `append` differ *only*
    /// for a repeated name, so nothing above can tell whether this crate kept
    /// the pair, and a dropped cookie reads as a session that lost a value
    /// rather than as a header that went missing.
    ///
    /// The replacing row beside them is the invariant `Ctx::declare_header`
    /// maintains, asserted from the far side: an override still overrides while
    /// two appending rows both survive.
    #[test]
    fn an_appending_row_joins_a_name_the_response_already_carries() {
        let mut declared = completed("ok", Some("text/plain"));
        declared.headers = vec![
            nvs_runtime::DeclaredHeader::add("Set-Cookie", "sid=1; HttpOnly"),
            nvs_runtime::DeclaredHeader::add("Set-Cookie", "theme=dark"),
            nvs_runtime::DeclaredHeader::set("Cache-Control", "no-store"),
        ];
        let answered = answer(declared);
        let cookies: Vec<&str> = answered
            .headers()
            .get_all("set-cookie")
            .iter()
            .map(|value| value.to_str().expect("a value this test wrote is ASCII"))
            .collect();
        assert_eq!(
            cookies,
            vec!["sid=1; HttpOnly", "theme=dark"],
            "two declarations of one name did not both reach the peer, in order"
        );
        assert_eq!(
            answered.headers().get(header::CACHE_CONTROL).unwrap(),
            "no-store",
            "a replacing row beside two appending ones did not reach the response"
        );
    }

    /// M7's header injection suite — `rule:http-server/secure-headers-with-nothing-written`, one row per published
    /// technique rather than one case per spelling.
    ///
    /// **This is the lower of two layers, and the only one a `-p nvs-server`
    /// fixture can reach.** `Core\Response::setHeader` refuses a name that is
    /// not a token and a value a header line cannot carry *at the member*,
    /// which is `nvs-stdlib`'s half and is pinned by
    /// `tests/conformance/core/a-response-set-header-names-both-sides-of-a-header-line.nvst`;
    /// so every row below is a pair no program can write today, and what is
    /// asserted here is what [`answer`] does if one arrives anyway — the
    /// `continue` that drops it rather than repairing it into something
    /// spellable or answering with it.
    ///
    /// **Every row's expected answer is the response the same request would
    /// have got having declared nothing at all**, asserted by counting the head
    /// rather than by searching it for the injected name: a door that rewrote a
    /// `\r\n` to a space, or that kept the name and dropped only the value,
    /// passes a search for `X-Injected` and fails here.
    #[test]
    fn the_header_injection_suite_passes() {
        // Each row is `(the technique, the name, the value)`, and every one of
        // them is trying to reach the peer with the same marker, so a row that
        // got through is legible from its own line.
        let suite = [
            (
                "CRLF and a second header",
                "X-Request-Id",
                "9f2\r\nX-Injected: yes",
            ),
            (
                "a bare LF, which a lenient parser accepts alone",
                "X-Request-Id",
                "9f2\nX-Injected: yes",
            ),
            ("a bare CR", "X-Request-Id", "9f2\rX-Injected: yes"),
            (
                "CRLF twice, ending the head and starting a body",
                "X-Request-Id",
                "9f2\r\n\r\nX-Injected: yes",
            ),
            (
                "a leading CRLF, folding onto the line above",
                "X-Request-Id",
                "\r\n X-Injected: yes",
            ),
            ("a NUL in the value", "X-Request-Id", "9f2\0X-Injected"),
            ("a lone control character", "X-Request-Id", "\u{b}"),
            ("a DEL", "X-Request-Id", "9f2\u{7f}"),
            (
                "a colon in the name",
                "X-Request-Id: 9f2\r\nX-Injected",
                "yes",
            ),
            ("a space in the name", "X Request Id", "9f2"),
            ("CRLF in the name", "X-Request-Id\r\nX-Injected", "yes"),
            ("a NUL in the name", "X-Request\0Id", "9f2"),
            ("a name that is only a fold", "\n X-Injected", "yes"),
            ("an empty name", "", "9f2"),
        ];
        for (technique, name, value) in suite {
            let mut declared = completed("ok", Some("text/plain"));
            declared.headers = vec![nvs_runtime::DeclaredHeader::set(name, value)];
            let answered = answer(declared);
            assert_eq!(
                answered.headers().len(),
                1,
                "{technique}, spelled {name:?}: {value:?}, reached the peer"
            );
            assert_eq!(
                answered.headers().get(header::CONTENT_TYPE).unwrap(),
                "text/plain",
                "{technique} displaced the body member's media type"
            );
        }

        // The one byte the two layers disagree about, named here rather than
        // left to read as an omission from the table: `Core\Response::setHeader`
        // refuses a value outside printable ASCII *at the member*, while a
        // header line may carry obs-text, so the door admits it. That is not a
        // gap — no byte above 127 ends a header line, which is the only thing
        // this layer is defending — and the layer that refuses it is the one
        // whose rule it is.
        let mut obs_text = completed("ok", Some("text/plain"));
        obs_text.headers = vec![nvs_runtime::DeclaredHeader::set(
            "X-Request-Id",
            "9f2\u{e9}",
        )];
        assert_eq!(
            answer(obs_text).headers().len(),
            2,
            "the door drops what a header line cannot carry, which obs-text is not"
        );

        // The suite is only worth its length if a pair the member *would* have
        // admitted still reaches the peer, and if `rule:http-server/secure-headers-with-nothing-written`'s set is intact
        // beside it: a door that dropped every declaration would pass all
        // fourteen rows above and nothing here.
        let mut declared = completed("ok", Some("text/plain"));
        declared.headers = vec![
            nvs_runtime::DeclaredHeader::set("X-Request-Id", "9f2"),
            // The override § 1 exists to allow, beside an attempt to smuggle a
            // second policy header in on the back of one this server writes.
            // `Secure::fill` writes a name the answer does not already carry,
            // so an accepted injection here would be a policy header the
            // program chose rather than one it overrode.
            nvs_runtime::DeclaredHeader::set("Referrer-Policy", "no-referrer"),
            nvs_runtime::DeclaredHeader::set(
                "X-Content-Type-Options",
                "nosniff\r\nX-Frame-Options: ALLOWALL",
            ),
        ];
        let mut answered = answer(declared);
        Secure::default().fill(answered.headers_mut(), Scheme::Http);
        let headers = answered.headers();
        assert_eq!(
            headers.get("x-request-id").unwrap(),
            "9f2",
            "a header the member admits did not survive the suite's own layer"
        );
        assert_eq!(
            headers.get(header::REFERRER_POLICY).unwrap(),
            "no-referrer",
            "§ 1's set overwrote the program's override of one of its members"
        );
        assert_eq!(
            headers.get("x-content-type-options").unwrap(),
            "nosniff",
            "the dropped pair left its name absent, so the policy value is what is on the wire"
        );
        assert_eq!(
            headers.get("x-frame-options"),
            None,
            "an injected pair split the policy header it was declared under"
        );
        assert_eq!(
            headers.get(header::CONTENT_SECURITY_POLICY).unwrap(),
            "frame-ancestors 'none'",
            "one overridden header took the rest of § 1's set with it"
        );
        // The governing rule, over the whole head at once rather than over the
        // name a row happened to choose: nothing a program declared can put a
        // byte on the wire that ends a header line.
        for (name, value) in headers {
            assert!(
                !value
                    .as_bytes()
                    .iter()
                    .any(|&byte| matches!(byte, b'\r' | b'\n' | 0)),
                "a value on the wire carries a byte that ends a header line: {name}"
            );
        }
    }

    /// The isolate the first two tests answer with: a program that echoes the
    /// path back, so that a response asserted below is the answer to the
    /// request that asked for it and not merely a well-formed response.
    ///
    /// A closure is the program a test at this level can build — turning a path
    /// into runnable code is `nvs_runtime::script`'s seam and there is no
    /// compiler in this crate — and it writes through `Ctx::write_output`,
    /// which is the buffer a compiled `echo` reaches under `rule:tooling/echo-always-has-a-sink`'s
    /// table.
    fn echo_the_path() -> Rc<impl Fn(Request<Incoming>, Origin) -> Reply> {
        Rc::new(|request: Request<Incoming>, _origin: Origin| {
            let path = request.uri().path().to_owned();
            let program: Program = Box::new(move |child: &mut Ctx, _args| {
                child
                    .write_output(format!("hello {path}").as_bytes())
                    .expect("a buffer");
                Value::null()
            });
            Reply::run(Isolate::new(program, Value::null(), Output::Capture))
        })
    }

    /// The isolate the carrier case answers with: a handler that reads the
    /// arrived request into `nvs_runtime::Inbound` and a program that reads it
    /// back off its **own** context, which is the whole of the seam
    /// [`Isolate::answering`] opens.
    ///
    /// It says the request line and every `X-Trace` line, in arrival order, so
    /// that the assertion is about what the peer sent rather than about a
    /// response merely being well-formed. The path is the target's own here:
    /// stripping a mount prefix is `crate::mount`'s step 2 and this crate's
    /// tests have no table.
    fn echo_the_carrier() -> Rc<impl Fn(Request<Incoming>, Origin) -> Reply> {
        Rc::new(|request: Request<Incoming>, _origin: Origin| {
            let mut inbound = nvs_runtime::Inbound::new(
                request.method().as_str(),
                request.uri().path(),
                request.uri().query().unwrap_or(""),
            );
            for (name, value) in request.headers() {
                inbound.push_header(name.as_str(), value.as_bytes());
            }
            let program: Program = Box::new(move |child: &mut Ctx, _args| {
                let said = {
                    let inbound = child
                        .inbound()
                        .expect("the isolate ran with no request in front of it");
                    let mut said = format!(
                        "{} {} {}",
                        inbound.method(),
                        inbound.path(),
                        inbound.query()
                    );
                    for (name, value) in inbound.headers() {
                        if name == "x-trace" {
                            said.push(' ');
                            said.push_str(&String::from_utf8_lossy(value));
                        }
                    }
                    said
                };
                child.write_output(said.as_bytes()).expect("a buffer");
                Value::null()
            });
            Reply::run(Isolate::new(program, Value::null(), Output::Capture).answering(inbound))
        })
    }

    /// A request reaches the program answering it, end to end: off the socket,
    /// through the handler, into the isolate's own context, and back out as the
    /// body — spec § 15's request line and headers as `Core\Request` will read
    /// them.
    ///
    /// The two `X-Trace` lines are the load-bearing half. `Ctx::push_header`
    /// keeps one entry per *field line* rather than per name, and a carrier that
    /// folded them into a map would answer `8` alone — which reads as a working
    /// server right up to the request whose `X-Forwarded-For` chain matters.
    #[test]
    fn a_served_request_reaches_its_isolate_as_the_inbound_carrier() {
        let mut listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .write_all(
                    b"GET /greet?who=world HTTP/1.1\r\nHost: localhost\r\nX-Trace: 7\r\n\
                      X-Trace: 8\r\nConnection: close\r\n\r\n",
                )
                .expect("the write failed");
            let mut answer = String::new();
            socket
                .read_to_string(&mut answer)
                .expect("the response could not be read");
            answer
        });

        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
            serve_on_this_core(
                &mut listener,
                &echo_the_carrier(),
                Waits::default(),
                &wide_open(),
                &Draining::detached(),
                |_note| {},
                || ControlFlow::Break(()),
            )
            .expect("the accept loop failed");
        });
        nvs_host::run_until_idle(&mut sched).expect("the loop failed");

        let answer = client.join().expect("the client thread panicked");
        assert!(
            answer.ends_with("GET /greet who=world 7 8"),
            "the request did not reach the program as the peer sent it: {answer}"
        );
    }

    /// The isolate `rule:concurrency/a-connection-is-a-root-isolate`'s offer case answers with: a program that says
    /// whether its **own** carrier has an upgrade slot on it.
    ///
    /// It reads the slot through `Core\Socket::upgrade`'s own route —
    /// `Ctx::inbound()` and nothing else — so what the case sees is what that
    /// member will see, and it never touches the request's headers: whether this
    /// connection can be upgraded is `hyper`'s answer, and a handler re-reading
    /// `Connection: Upgrade` for itself would be asserting a second one.
    fn say_whether_an_upgrade_was_offered() -> Rc<impl Fn(Request<Incoming>, Origin) -> Reply> {
        Rc::new(|request: Request<Incoming>, _origin: Origin| {
            let inbound = nvs_runtime::Inbound::new(
                request.method().as_str(),
                request.uri().path(),
                request.uri().query().unwrap_or(""),
            );
            let path = request.uri().path().to_owned();
            let program: Program = Box::new(move |child: &mut Ctx, _args| {
                let offered = child
                    .inbound()
                    .and_then(nvs_runtime::Inbound::upgrade_slot)
                    .is_some();
                let said = format!("{path} {}", if offered { "offered" } else { "none" });
                child.write_output(said.as_bytes()).expect("a buffer");
                Value::null()
            });
            Reply::run(Isolate::new(program, Value::null(), Output::Capture).answering(inbound))
        })
    }

    /// `rule:concurrency/a-connection-is-a-root-isolate`'s
    /// offer, at the door: a request that opened RFC 6455's handshake reaches
    /// its program with a slot on its carrier, and neither of the two requests
    /// after it on the same connection does.
    ///
    /// **The middle one is the narrow half of the rule.** `hyper` frames an
    /// upgrade for any `Connection: Upgrade`, which answers "can this
    /// connection be taken over" and not "did a WebSocket ask": a handshake
    /// with no `Sec-WebSocket-Key` has no accept to answer with, so the door
    /// offers nothing and the program answers it as the ordinary request it is.
    ///
    /// **All of it on one connection**, because the claim is that the offer is
    /// per *request* and not per connection — the socket really is upgradable
    /// throughout, so a door that decided once at accept time and remembered
    /// would pass a case that asked only the first question. It is also the half
    /// that makes `Core\Socket::upgrade` refuse: a request with no slot is what
    /// that member throws on, and here it is an ordinary `GET` over a connection
    /// that could have carried one.
    #[test]
    fn only_an_upgradable_request_is_offered_a_slot() {
        let mut listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            // RFC 6455's opening handshake, whole.
            socket
                .write_all(upgrade_request("/chat").as_bytes())
                .expect("the write failed");
            let mut seen = String::new();
            read_until(&mut socket, "/chat offered", &mut seen);
            // The same framing with no key, which `hyper` frames an upgrade for
            // just the same: it is a `Connection: Upgrade` and nothing more, so
            // the door has nothing to derive an accept from and offers nothing.
            socket
                .write_all(
                    b"GET /nokey HTTP/1.1\r\nHost: localhost\r\nConnection: Upgrade\r\n\
                      Upgrade: websocket\r\n\r\n",
                )
                .expect("the keyless write failed");
            read_until(&mut socket, "/nokey none", &mut seen);
            socket
                .write_all(b"GET /plain HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .expect("the second write failed");
            socket
                .read_to_string(&mut seen)
                .expect("the second response could not be read");
            end_the_loop(addr);
            seen
        });

        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
            serve_on_this_core(
                &mut listener,
                &say_whether_an_upgrade_was_offered(),
                Waits::default(),
                &wide_open(),
                &Draining::detached(),
                |_note| {},
                until_the_loop_is_ended(),
            )
            .expect("the accept loop failed");
        });
        nvs_host::run_until_idle(&mut sched).expect("the loop failed");

        let seen = client.join().expect("the client thread panicked");
        assert!(
            seen.contains("/chat offered"),
            "an upgradable request reached its program with no slot: {seen}"
        );
        assert!(
            seen.contains("/nokey none"),
            "a request that framed an upgrade but opened no WebSocket was \
             offered a slot: {seen}"
        );
        assert!(
            seen.contains("/plain none"),
            "an ordinary request was offered an upgrade slot: {seen}"
        );
    }

    /// The handler `rule:concurrency/a-connection-is-a-root-isolate`
    /// 's three start cases answer with: a request that fills the slot on its
    /// own carrier exactly as `Core\Socket::upgrade` will — one
    /// [`nvs_runtime::Upgrade`] over a hand-written program — reports itself, and
    /// ends.
    ///
    /// `carried` is the query the request's carrier holds, so a case can put a
    /// *measurable* amount of the request's own state on it. `said` is the one
    /// place both isolates report to, in the order they ran: there is no
    /// response left for the connection's to write into and no socket yet for it
    /// to frame on. An `Rc` reaches both because the connection isolate runs on
    /// this core on a task of the connection's, which is the claim rather than a
    /// convenience.
    fn upgrade_leaving(
        door: Door,
        carried: String,
        said: Rc<RefCell<Vec<String>>>,
        connection: fn(&mut Ctx) -> String,
    ) -> Rc<impl Fn(Request<Incoming>, Origin) -> Reply> {
        Rc::new(move |request: Request<Incoming>, _origin: Origin| {
            let inbound = nvs_runtime::Inbound::new(
                request.method().as_str(),
                request.uri().path(),
                &carried,
            );
            let path = request.uri().path().to_owned();
            let said = Rc::clone(&said);
            let program: Program = Box::new(move |child: &mut Ctx, _args| {
                // What the member will leave in the slot, built here for the
                // same reason it can be: nothing but a program and an argument
                // crosses, so a case needs no compiler to fill the slot the way
                // `Core\Socket::upgrade` fills it.
                let said_over_there = Rc::clone(&said);
                let opened: Program = Box::new(move |conn: &mut Ctx, _args| {
                    let line = connection(conn);
                    said_over_there
                        .borrow_mut()
                        .push(format!("connection {line}"));
                    Value::null()
                });
                let filled = door.fill(child, opened);
                // What the *other* cell says about this same request, reported
                // beside the fill because it is what tells the two doors apart:
                // § 1's slot is there only where `hyper` framed an upgrade, so
                // an event stream that opened without one opened over a
                // connection with no socket behind it — and nothing a `receive`
                // could ever read from.
                let framing = if child
                    .inbound()
                    .and_then(nvs_runtime::Inbound::upgrade_slot)
                    .is_some()
                {
                    FRAMED
                } else {
                    NO_FRAMING
                };
                let mine = if filled {
                    door.answered(&path, framing)
                } else {
                    format!("{path} no cell")
                };
                // Read here, with this request's carrier still alive and its
                // arena at its peak — the number the connection's own reading
                // is compared against — and what this request answered with
                // beside it. That line is reported rather than read off the
                // wire because § 5's door replaces it there: the peer gets the
                // event stream, and the one place a case can still see what the
                // request said for itself is here.
                said.borrow_mut().push(format!(
                    "request {} {mine}",
                    nvs_runtime::budget::live_bytes()
                ));
                child.write_output(mine.as_bytes()).expect("a buffer");
                Value::null()
            });
            Reply::run(Isolate::new(program, Value::null(), Output::Capture).answering(inbound))
        })
    }

    /// Two mebibytes of query on an upgrading request's carrier — far more than
    /// anything either isolate holds for its own reasons, which is what makes
    /// the two readings [`balance`] compares tell one answer from the other.
    const CARRIED: usize = 2 * 1024 * 1024;

    /// What a connection isolate holds that the request's own reading did not:
    /// its context, its output buffer, and for § 1 the 128 KiB input buffer
    /// `tungstenite` allocates per socket ([`crate::socket`]'s docs § *What it
    /// spends*). A quarter of [`CARRIED`], so the allowance cannot hide the
    /// failure the cases using it are there for.
    const CONNECTIONS_OWN: isize = 512 * 1024;

    /// A connection isolate that reports how much of this thread is still live
    /// under it, which is the reading the request's own is compared against.
    fn say_the_balance(_conn: &mut Ctx) -> String {
        nvs_runtime::budget::live_bytes().to_string()
    }

    /// The number one of `said`'s lines opens with — the first word, since the
    /// request reports what it answered with after its reading and the
    /// connection reports nothing else at all.
    fn balance(line: &str, whose: &str) -> isize {
        line.strip_prefix(whose)
            .and_then(|rest| rest.split_whitespace().next())
            .unwrap_or_else(|| panic!("{whose} never reported a balance: {line}"))
            .parse()
            .unwrap_or_else(|_| panic!("{whose} reported no number: {line}"))
    }

    /// RFC 6455's opening handshake, written down once: the two field lines
    /// `hyper` frames an upgrade for, and the two this door reads for itself.
    ///
    /// It carries no `Connection: close`, because an upgradable request may not
    /// also ask for the connection to end; the client closes the socket itself
    /// once it has read the answer, which is what lets the connection future end
    /// and [`serve_connection`] reach the isolate it started.
    fn upgrade_request(path: &str) -> String {
        format!(
            "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: Upgrade\r\n\
             Upgrade: websocket\r\nSec-WebSocket-Key: {KEY}\r\n\
             Sec-WebSocket-Version: 13\r\n\r\n"
        )
    }

    /// RFC 6455 § 1.3's own worked example, key half — so what the cases below
    /// assert is the RFC's arithmetic and not this tree's agreement with itself.
    const KEY: &str = "dGhlIHNhbXBsZSBub25jZQ==";
    /// And the accept half, which is what the peer checks before it believes a
    /// `101`.
    const ACCEPT: &str = "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=";

    /// What a request reports about § 1's slot when `hyper` framed an upgrade
    /// for it.
    const FRAMED: &str = "over a framed upgrade";
    /// And when it framed none, which is the half § 5's case reads back.
    const NO_FRAMING: &str = "with no framed upgrade";

    /// Which of `rule:concurrency/a-connection-is-a-root-isolate`'s
    /// two cells a case's request fills — § 1's socket hand-over or § 5's event
    /// stream — which on this side is the whole of what separates the two
    /// doors.
    ///
    /// It decides three things at once: what the client sends, which cell the
    /// program fills, and the line the case reads back. They are one type
    /// because they are one claim — § 1's slot is offered only where an upgrade
    /// was framed and § 5's cell to every request — and a case that sent one
    /// door's opening and read the other's answer would assert nothing while
    /// still going green.
    #[derive(Clone, Copy)]
    enum Door {
        /// § 1: the socket, taken over RFC 6455's opening handshake.
        Socket,
        /// § 5: the event stream, opened out of an ordinary `GET`.
        Sse,
        /// Both cells at once, which is the contradiction the door refuses: it
        /// takes § 1's opening, because a request that framed no upgrade is
        /// offered only one of the two and could not ask for both.
        Both,
    }

    impl Door {
        /// The opening the client sends.
        fn opening(self, path: &str) -> String {
            match self {
                Self::Socket | Self::Both => upgrade_request(path),
                Self::Sse => format!("GET {path} HTTP/1.1\r\nHost: localhost\r\n\r\n"),
            }
        }

        /// Fills this door's cell on the request's own carrier, exactly as its
        /// `upgrade` member will, and answers whether one was there to fill.
        fn fill(self, child: &Ctx, opened: Program) -> bool {
            let Some(inbound) = child.inbound() else {
                return false;
            };
            match self {
                Self::Socket => inbound.upgrade_slot().is_some_and(|slot| {
                    slot.fill(nvs_runtime::Upgrade::new(opened, Value::null()))
                        .is_ok()
                }),
                Self::Sse => inbound.sse_slot().is_some_and(|cell| {
                    cell.fill(nvs_runtime::Upgrade::new(opened, Value::null()))
                        .is_ok()
                }),
                // The socket half runs nothing and says nothing: the claim is
                // that *neither* isolate is started, so a second program
                // reporting to `said` would only be a second way to read the
                // one line that must not be there.
                Self::Both => {
                    let socket: Program = Box::new(|_conn: &mut Ctx, _args| Value::null());
                    inbound.upgrade_slot().is_some_and(|slot| {
                        slot.fill(nvs_runtime::Upgrade::new(socket, Value::null()))
                            .is_ok()
                    }) && inbound.sse_slot().is_some_and(|cell| {
                        cell.fill(nvs_runtime::Upgrade::new(opened, Value::null()))
                            .is_ok()
                    })
                }
            }
        }

        /// The line the request writes when it filled this door's cell, with
        /// what the *other* cell said about the same request beside it.
        fn answered(self, path: &str, framing: &str) -> String {
            match self {
                Self::Socket => format!("{path} upgraded {framing}"),
                Self::Sse => format!("{path} streaming {framing}"),
                Self::Both => format!("{path} both {framing}"),
            }
        }

        /// What [`upgrade_once`] reads until — for each door, the last bytes
        /// its response puts on the wire, so that a case asserts over the whole
        /// of what its peer was sent rather than over a prefix of it.
        ///
        /// **No door's needle is the line the request wrote for itself**, and
        /// that is one claim rather than three coincidences: a request that
        /// opened either connection is answered with what the *door* writes,
        /// so a case reading its own line back would be reading the one thing
        /// that must not be there.
        fn needle(self) -> String {
            match self {
                // The handshake, because a request that upgraded does **not**
                // send the line it wrote for itself: the `101` is this
                // connection's one remaining response and the program's bytes
                // go nowhere. Reading to the accept key rather than to the
                // status line is what makes the wait cover the whole head.
                Self::Socket => ACCEPT.to_owned(),
                // The terminating chunk of the event stream's body, written
                // when the connection isolate ends and the writing half goes
                // with it — so what a case holds is the head and every event
                // the isolate sent. It cannot match inside the head: that would
                // need a field line that is a single `0`, and a head has none.
                Self::Sse => "\r\n0\r\n\r\n".to_owned(),
                // The refused case reads the *status*, because the answer this
                // request wrote for itself is the one thing it must not get.
                Self::Both => "500 Internal Server Error".to_owned(),
            }
        }
    }

    /// Runs one connection's worth of the accept loop against `handler`, sends
    /// `door`'s opening, reads until its answer and closes — the four cases
    /// below differ only in the door they go through and the program they leave
    /// in its cell. The loop is ended from the client ([`end_the_loop`]) once
    /// the answer is in, because the loop's tail begins the drain and a drain
    /// ends an idle event stream and a parked `receive` at once — which is not
    /// what any of these cases is about.
    fn upgrade_once<H>(
        handler: impl FnOnce() -> Rc<H> + 'static,
        path: &'static str,
        door: Door,
    ) -> String
    where
        H: Fn(Request<Incoming>, Origin) -> Reply + 'static,
    {
        upgrade_once_under(Waits::default(), handler, path, door)
    }

    /// [`upgrade_once`] with the waits named, for the one case whose claim is
    /// about a wait rather than about a door: a case asserting that a stream
    /// outlives `write_idle` has to name a `write_idle` it can outlive inside a
    /// test's running time.
    fn upgrade_once_under<H>(
        waits: Waits,
        handler: impl FnOnce() -> Rc<H> + 'static,
        path: &'static str,
        door: Door,
    ) -> String
    where
        H: Fn(Request<Incoming>, Origin) -> Reply + 'static,
    {
        let mut listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .write_all(door.opening(path).as_bytes())
                .expect("the write failed");
            let mut seen = String::new();
            read_until(&mut socket, &door.needle(), &mut seen);
            end_the_loop(addr);
            seen
        });

        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
            serve_on_this_core(
                &mut listener,
                &handler(),
                waits,
                &wide_open(),
                &Draining::detached(),
                |_note| {},
                until_the_loop_is_ended(),
            )
            .expect("the accept loop failed");
        });
        nvs_host::run_until_idle(&mut sched).expect("the loop failed");
        client.join().expect("the client thread panicked")
    }

    /// `rule:concurrency/a-connection-is-a-root-isolate`'s
    /// two halves at once: an upgrade opens a **root isolate** — its own context
    /// and its own output, none of it this response's — and the request that
    /// opened it ends normally rather than becoming it.
    ///
    /// The connection's own `echo` is what separates the two claims. Bytes it
    /// writes reach its own buffer and never the wire, so an implementation that
    /// resumed the request under another name, or that started the isolate while
    /// the response was still open to it, fails here — where a case that only
    /// counted the isolate's runs would pass either way. The order of `said`'s
    /// two lines is § 1's ordering: the request reports before the connection
    /// exists.
    #[test]
    fn an_upgrade_opens_a_root_isolate_and_the_upgrading_request_ends() {
        fn write_where_nobody_is_reading(conn: &mut Ctx) -> String {
            conn.write_output(b"never on this connection's wire")
                .expect("a buffer");
            "opened".to_owned()
        }

        let said = Rc::new(RefCell::new(Vec::new()));
        let handler_said = Rc::clone(&said);
        let seen = upgrade_once(
            move || {
                upgrade_leaving(
                    Door::Socket,
                    String::new(),
                    handler_said,
                    write_where_nobody_is_reading,
                )
            },
            "/chat",
            Door::Socket,
        );

        assert!(
            seen.contains("101 Switching Protocols") && seen.contains(ACCEPT),
            "the upgrading request was not answered RFC 6455's handshake: {seen}"
        );
        assert!(
            !seen.contains("/chat upgraded"),
            "the upgrading request's own answer was sent as well as the \
             handshake: {seen}"
        );
        assert!(
            !seen.contains("never on this connection's wire"),
            "the connection isolate wrote into the request's response: {seen}"
        );
        let said = said.borrow();
        assert_eq!(
            said.len(),
            2,
            "one of the two isolates did not run: {said:?}"
        );
        assert!(
            said[0].starts_with("request ") && said[1] == "connection opened",
            "the connection isolate did not open after the request ended: {said:?}"
        );
    }

    /// `rule:concurrency/a-connection-is-a-root-isolate`'s
    /// cost half: "the connection is not a suspended request, does not hold the
    /// request's arena". [`serve_connection`]'s doc states the same thing as an
    /// ordering — the request is joined before the connection's isolate is
    /// built — and this is the reading that says the ordering had the effect it
    /// is claimed for.
    ///
    /// **Two readings of one thread's live bytes**, taken by the two isolates
    /// in the order they ran: the request's, with its carrier at its peak, and
    /// the connection's, with the request over. The gap between them has to be
    /// the query the request carried, less what the connection holds for its
    /// own reasons, which is [`CONNECTIONS_OWN`]. A connection that inherited,
    /// or that merely outlived, the request's context reads within kilobytes of
    /// the *first* number rather than [`CARRIED`] below it.
    ///
    /// What the reading does not include is the handler's own copy of the
    /// query, which this fixture holds for the length of the run and the
    /// server never has — the assertion is about the carrier the request was
    /// answered from and nothing else.
    ///
    /// This is the case that `rule:programs/memory-priority`'s "O(in-flight) rather than O(requests
    /// served)" is asserted by on the request path: it failed for as long as
    /// `nvs_host::Scheduler` filed *every* task's context on its `finished`
    /// list, which under a server is every request ever served, and
    /// [`nvs_host::Finished`] is now the home of why only a root files one.
    #[test]
    fn the_upgrading_requests_arena_is_released_while_the_connection_is_open() {
        let said = Rc::new(RefCell::new(Vec::new()));
        let handler_said = Rc::clone(&said);
        let seen = upgrade_once(
            move || {
                upgrade_leaving(
                    Door::Socket,
                    format!("q={}", "x".repeat(CARRIED)),
                    handler_said,
                    say_the_balance,
                )
            },
            "/chat",
            Door::Socket,
        );

        assert!(
            seen.contains(ACCEPT),
            "the upgrading request did not reach its own end: {seen}"
        );
        let said = said.borrow();
        assert_eq!(
            said.len(),
            2,
            "one of the two isolates did not run: {said:?}"
        );
        let peak = balance(&said[0], "request ");
        let open = balance(&said[1], "connection ");
        assert!(
            peak - open >= CARRIED.cast_signed() - CONNECTIONS_OWN,
            "the upgrading request's arena was still held while the connection \
             ran: {peak} bytes live under the request, {open} under the \
             connection, and the query alone is {CARRIED}"
        );
    }

    /// `rule:concurrency/a-connection-is-a-root-isolate`'s
    /// security property: the connection "cannot see the request's session,
    /// cookies or headers unless a value was explicitly passed".
    ///
    /// The request's carrier is the one thing every one of those rides on, so
    /// the assertion is that the connection isolate has none — and it is made
    /// against a query the request itself could read, which is what makes the
    /// answer a boundary rather than an empty carrier. Nothing was passed here,
    /// so nothing is what the connection may have; what an explicitly passed
    /// value looks like is `args`, and the member that copies one is the next
    /// slice.
    #[test]
    fn a_connection_isolate_cannot_read_the_upgrading_requests_state() {
        fn say_what_it_can_see(conn: &mut Ctx) -> String {
            conn.inbound().map_or_else(
                || "no request".to_owned(),
                |inbound| format!("{}?{}", inbound.path(), inbound.query()),
            )
        }

        let said = Rc::new(RefCell::new(Vec::new()));
        let handler_said = Rc::clone(&said);
        let seen = upgrade_once(
            move || {
                upgrade_leaving(
                    Door::Socket,
                    "session=abc123".to_owned(),
                    handler_said,
                    say_what_it_can_see,
                )
            },
            "/private",
            Door::Socket,
        );
        assert!(
            seen.contains(ACCEPT),
            "the request never filled the slot: {seen}"
        );

        let said = said.borrow();
        assert_eq!(
            said[1], "connection no request",
            "the connection isolate reached the upgrading request's carrier: {said:?}"
        );
    }

    /// `rule:concurrency/a-connection-is-a-root-isolate`'s
    /// other half: the socket **arrives**. A connection isolate reads a frame
    /// the peer sent and answers one the peer reads, over the descriptor that
    /// carried the request an instant earlier.
    ///
    /// This is the case the three above could not make. Each of them asserts
    /// something about the isolate the upgrade opened — that it is a root, that
    /// it holds none of the request — and every one of them would still pass
    /// against a door that answered the `101` and then dropped the connection
    /// on the floor. What separates the two is a byte going each way, so that
    /// is what this sends.
    ///
    /// **The client is `tungstenite` in the other role**, deliberately: a
    /// hand-rolled frame would assert this tree's agreement with itself, where
    /// a client codec masking its payload and refusing a masked answer is RFC
    /// 6455 checking both directions. The handshake above it is still written
    /// by hand, because the accept key is what [`websocket_opening`] derives
    /// and a client that computed its own would hide a wrong one.
    ///
    /// The park is the other thing under test and it is not asserted
    /// separately: the server has nothing to read when the isolate first calls
    /// `receive`, so a codec that answered "would block" — or one that blocked
    /// the *core* rather than the task — deadlocks this case rather than
    /// failing it slowly.
    #[test]
    fn a_connection_isolate_reads_and_writes_frames_over_the_upgraded_socket() {
        fn talk_to_the_peer(conn: &mut Ctx) -> String {
            let Some(peer) = conn.peer() else {
                return "no peer".to_owned();
            };
            let heard = match peer.receive() {
                Ok(Some(nvs_runtime::PeerFrame::Text(text))) => text,
                other => return format!("read {other:?}"),
            };
            let sent = peer.send(nvs_runtime::PeerFrame::Text(format!("echo of {heard}")));
            peer.close(nvs_runtime::Closing::Done);
            match sent {
                Ok(()) => format!("heard {heard}"),
                Err(error) => format!("send {error}"),
            }
        }

        let mut listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .write_all(upgrade_request("/chat").as_bytes())
                .expect("the write failed");
            // To the end of the head and no further, so that the codec below
            // starts on the first byte after it: the accept key is not the last
            // field line, `rule:http-server/secure-headers-with-nothing-written`'s set being filled in after it.
            let mut head = String::new();
            read_until(&mut socket, "\r\n\r\n", &mut head);
            let mut peer = tungstenite::protocol::WebSocket::from_raw_socket(
                socket,
                tungstenite::protocol::Role::Client,
                None,
            );
            peer.send(tungstenite::Message::text("a frame from the peer"))
                .expect("the frame could not be sent");
            let answer = peer.read().expect("the connection isolate sent nothing");
            // Only now, because the loop's tail begins the drain and a drain
            // closes a `receive` waiting on its peer at once.
            end_the_loop(addr);
            (head, answer.to_text().unwrap_or("not text").to_owned())
        });

        let said = Rc::new(RefCell::new(Vec::new()));
        let handler_said = Rc::clone(&said);
        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
            let handler =
                upgrade_leaving(Door::Socket, String::new(), handler_said, talk_to_the_peer);
            serve_on_this_core(
                &mut listener,
                &handler,
                Waits::default(),
                &wide_open(),
                &Draining::detached(),
                |_note| {},
                until_the_loop_is_ended(),
            )
            .expect("the accept loop failed");
        });
        nvs_host::run_until_idle(&mut sched).expect("the loop failed");
        let (head, answer) = client.join().expect("the client thread panicked");

        assert!(
            head.contains("101 Switching Protocols") && head.contains(ACCEPT),
            "the peer was not answered RFC 6455's handshake: {head}"
        );
        assert_eq!(
            answer, "echo of a frame from the peer",
            "the connection isolate did not answer the frame it was sent"
        );
        let said = said.borrow();
        assert_eq!(
            said[1], "connection heard a frame from the peer",
            "the socket did not reach the connection isolate: {said:?}"
        );
    }

    /// `rule:concurrency/a-connection-is-a-root-isolate`'s
    /// "closed with a defined code": a connection that ran past one of the
    /// `[limits]` values § 1 gives it its own budget of is **told** why, and
    /// what the peer reads is RFC 6455's 1011 rather than the reset a process
    /// killed for the memory it was holding leaves behind. That contrast is
    /// what "not OOM" names — `nvs_runtime::Closing::Faulted`'s own doc is
    /// where it is argued, and this is it asserted from the client's side.
    ///
    /// **The breach is induced rather than compiled**, and the two halves are
    /// separate on purpose. The close itself is landed and keyed off
    /// `Completion::ok`, so what a fixture owes is a connection isolate that
    /// ends *not ok* for the reason § 1 names: this one sets its own ceiling —
    /// `Ctx::set_memory_limit`, the way an isolate holding no configuration
    /// gets one — allocates past it, and then makes the two calls
    /// `nvs_runtime`'s safepoint poll makes on a memory breach. A hand-written
    /// program has no safepoint between two statements to make them for it, and
    /// `Ctx::memory_breach` is the same arithmetic either way, so what is
    /// pinned here is the teardown rather than the poll.
    ///
    /// The hog is released before the report for the reason the compiled path
    /// gets for free: a fatal takes the arena with it, and a fixture still
    /// holding 32 MiB would breach again inside the teardown's own helper calls
    /// (`nvs_runtime::run_helper`) instead of asserting this close.
    #[test]
    fn a_connection_over_its_budget_is_closed_with_the_defined_code_not_oom() {
        /// The ceiling this connection is held to. Roomy in absolute terms, so
        /// that only the hog below can cross it: everything the isolate
        /// allocates for itself is measured against this same number.
        const CEILING: usize = 4 << 20;
        /// And what it is asked to hold — far enough past the ceiling that no
        /// profile's inlining decides the answer.
        const HOG: usize = 32 << 20;

        fn allocate_past_its_ceiling(conn: &mut Ctx) -> String {
            conn.set_memory_limit(CEILING);
            // Behind a `black_box` because an allocation nothing else reads is
            // one the optimizer is allowed to remove, and the whole of what
            // this program does is hold bytes.
            let hog = std::hint::black_box(vec![0_u8; HOG]);
            let breach = conn.memory_breach();
            drop(hog);
            let Some(nvs_runtime::Fault::Fatal(message)) = breach else {
                return "stayed inside its ceiling".to_owned();
            };
            // `rule:errors/on-limit`'s tier-1 handler is the other line the poll makes,
            // and there is none registered here — a connection that registered
            // one is that section's case rather than this one's.
            conn.set_pending(message);
            "held past its ceiling".to_owned()
        }

        let mut listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .write_all(upgrade_request("/chat").as_bytes())
                .expect("the write failed");
            let mut head = String::new();
            read_until(&mut socket, "\r\n\r\n", &mut head);
            // Nothing is sent from here: the connection isolate never reaches a
            // `receive`, so the only frame this socket will ever carry is the
            // close the server chose to send.
            let mut peer = tungstenite::protocol::WebSocket::from_raw_socket(
                socket,
                tungstenite::protocol::Role::Client,
                None,
            );
            let closed = match peer.read() {
                Ok(tungstenite::Message::Close(Some(frame))) => u16::from(frame.code).to_string(),
                Ok(tungstenite::Message::Close(None)) => "a close carrying no code".to_owned(),
                Ok(other) => format!("a frame rather than a close: {other:?}"),
                Err(error) => format!("no close at all: {error}"),
            };
            (head, closed)
        });

        let said = Rc::new(RefCell::new(Vec::new()));
        let handler_said = Rc::clone(&said);
        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
            let handler = upgrade_leaving(
                Door::Socket,
                String::new(),
                handler_said,
                allocate_past_its_ceiling,
            );
            serve_on_this_core(
                &mut listener,
                &handler,
                Waits::default(),
                &wide_open(),
                &Draining::detached(),
                |_note| {},
                || ControlFlow::Break(()),
            )
            .expect("the accept loop failed");
        });
        nvs_host::run_until_idle(&mut sched).expect("the loop failed");
        let (head, closed) = client.join().expect("the client thread panicked");

        assert!(
            head.contains("101 Switching Protocols"),
            "the peer was not answered RFC 6455's handshake, so nothing here is \
             about a connection: {head}"
        );
        let said = said.borrow();
        assert_eq!(
            said.len(),
            2,
            "the connection isolate did not run, so no budget was spent: {said:?}"
        );
        assert_eq!(
            said[1], "connection held past its ceiling",
            "the fixture did not put the connection over its own ceiling, so \
             the close below would say nothing: {said:?}"
        );
        assert_eq!(
            closed, "1011",
            "a connection over its budget did not end in `rule:concurrency/a-connection-is-a-root-isolate`'s defined \
             code — a reset here is the OOM kill the code exists to say did not \
             happen: {closed}"
        );
    }

    /// The other end of the case above: a connection isolate that fails in a
    /// way `rule:errors/escalation-ladder` has no ladder
    /// for — a panic in the engine itself — costs its own connection and
    /// **nothing else**. `nvs_runtime::run_task` is the boundary that contains
    /// it, and what this case reads back is the state on the far side: the core
    /// survives, and the accept loop reaches its own end.
    ///
    /// **That last line is the claim rather than a formality.**
    /// [`serve_on_this_core`]'s tail parks until every connection it spawned
    /// has counted itself back out, so a loop that *returns* is one whose
    /// panicking connection's task finished as well — the guard in
    /// `nvs_host::isolate` fired, the waiter it woke took a cancelled
    /// completion, and nothing was left parked on a child that would never
    /// answer. A second connection would say the same thing less directly and
    /// cannot be asked for here in any case; the playbook owns why.
    ///
    /// **What the panicking peer reads is a reset, and that is decided rather
    /// than missing.** `nvs_host::isolate`'s close sits after `finish`, so an
    /// unwind leaves through [`Ended`]'s guard without reaching it; the comment
    /// on that branch is the home of why the close may not move into the guard,
    /// and this case is that decision asserted from outside. A reset here is
    /// therefore not the failure `Closing::Faulted` exists to rule out — that
    /// one is a *process* killed under a connection, and this process is still
    /// answering on the next line.
    #[test]
    fn a_connection_whose_isolate_panics_is_contained() {
        fn panic_instead_of_running(_conn: &mut Ctx) -> String {
            panic!("a connection isolate's own bug, which is nobody else's");
        }

        let mut listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .write_all(upgrade_request("/chat").as_bytes())
                .expect("the write failed");
            let mut head = String::new();
            read_until(&mut socket, "\r\n\r\n", &mut head);
            let mut peer = tungstenite::protocol::WebSocket::from_raw_socket(
                socket,
                tungstenite::protocol::Role::Client,
                None,
            );
            let read = match peer.read() {
                Ok(frame) => Some(format!("{frame:?}")),
                Err(_) => None,
            };
            (head, read)
        });

        let said = Rc::new(RefCell::new(Vec::new()));
        let handler_said = Rc::clone(&said);
        let loop_said = Rc::clone(&said);
        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
            let handler = upgrade_leaving(
                Door::Socket,
                String::new(),
                handler_said,
                panic_instead_of_running,
            );
            serve_on_this_core(
                &mut listener,
                &handler,
                Waits::default(),
                &wide_open(),
                &Draining::detached(),
                |_note| {},
                || ControlFlow::Break(()),
            )
            .expect("the accept loop failed");
            // Reported to the same place both isolates report to, and last: the
            // drain above it is what makes this line mean the connection's own
            // task ended rather than only that the accept stopped.
            loop_said
                .borrow_mut()
                .push("the accept loop drained and returned".to_owned());
        });
        nvs_host::run_until_idle(&mut sched).expect("the loop failed");
        let (head, read) = client.join().expect("the client thread panicked");

        assert!(
            head.contains("101 Switching Protocols"),
            "the peer was not answered RFC 6455's handshake, so nothing here is \
             about a connection: {head}"
        );
        assert!(
            read.is_none(),
            "a panicking isolate sent its peer a frame, where the close it \
             would have sent sits after `finish` and the unwind never reaches \
             it: {read:?}"
        );
        let said = said.borrow();
        assert_eq!(
            said.len(),
            2,
            "the request isolate did not run, or the connection isolate \
             returned rather than panicking: {said:?}"
        );
        assert_eq!(
            said[1], "the accept loop drained and returned",
            "the panic was not contained to its own connection — the core \
             never reached the end of the loop that spawned it: {said:?}"
        );
    }

    /// `rule:concurrency/connection-bounds-are-finite`'s
    /// third bullet: a graceful shutdown and a `nvs ctl reload` "close
    /// connections with a defined code after a drain period, so a client's
    /// reconnect logic sees a clean close rather than a reset".
    ///
    /// **One case for both spellings, because there is one mechanism.**
    /// [`Draining::begin`] is the door a stopping process and a reload that
    /// needs one both go through — this loop's tail is the only writer, which
    /// [`Draining`]'s own docs own — and what a connection reads is the bit
    /// behind it. The close is then the connection isolate's *own*, taken at
    /// its next wait rather than reached in from here;
    /// [`crate::socket::Framed`]'s `receive` is where that is argued and why
    /// the two alternatives are refused.
    ///
    /// The isolate is § 3's loop with nothing to say: it waits for a frame that
    /// never comes, which is exactly the connection the bullet is about — one
    /// doing nothing when its server stops still has to be *told*, and it is
    /// told at once. **What makes the case deterministic** is the ordering the
    /// drain already has: `keep_serving` breaks before this connection's child
    /// has run at all, so the bit is set before the isolate's first `receive`,
    /// which reads it before it parks. The case that follows begins the drain
    /// *while* that `receive` is parked.
    ///
    /// The last line into `said` is the other half of the claim. The accept
    /// loop's tail parks until every connection has counted itself out, so a
    /// drain that returned while this connection was still open would either
    /// never reach that line or reach it before the connection's own — and the
    /// case would fail on the order rather than pass on the close.
    #[test]
    fn reload_and_shutdown_close_every_connection_after_the_drain() {
        fn wait_for_a_frame_that_never_comes(conn: &mut Ctx) -> String {
            let Some(peer) = conn.peer() else {
                return "no peer".to_owned();
            };
            let mut heard = 0_usize;
            loop {
                match peer.receive() {
                    Ok(Some(_)) => heard += 1,
                    Ok(None) => return format!("the loop ended after {heard} frames"),
                    Err(error) => return format!("receive {error}"),
                }
            }
        }

        let mut listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .write_all(upgrade_request("/chat").as_bytes())
                .expect("the write failed");
            let mut head = String::new();
            read_until(&mut socket, "\r\n\r\n", &mut head);
            let mut peer = tungstenite::protocol::WebSocket::from_raw_socket(
                socket,
                tungstenite::protocol::Role::Client,
                None,
            );
            // Nothing is sent: this peer is the client that connected and then
            // had nothing to say, which is the one the drain has to reach.
            let closed = match peer.read() {
                Ok(tungstenite::Message::Close(Some(frame))) => {
                    Ok((u16::from(frame.code), frame.reason.to_string()))
                }
                other => Err(format!("{other:?}")),
            };
            (head, closed)
        });

        let said = Rc::new(RefCell::new(Vec::new()));
        let handler_said = Rc::clone(&said);
        let loop_said = Rc::clone(&said);
        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
            let handler = upgrade_leaving(
                Door::Socket,
                String::new(),
                handler_said,
                wait_for_a_frame_that_never_comes,
            );
            serve_on_this_core(
                &mut listener,
                &handler,
                Waits::default(),
                &wide_open(),
                &Draining::detached(),
                |_note| {},
                || ControlFlow::Break(()),
            )
            .expect("the accept loop failed");
            loop_said
                .borrow_mut()
                .push("the accept loop drained and returned".to_owned());
        });
        nvs_host::run_until_idle(&mut sched).expect("the loop failed");
        let (head, closed) = client.join().expect("the client thread panicked");

        assert!(
            head.contains("101 Switching Protocols"),
            "the peer was not answered RFC 6455's handshake, so nothing here is \
             about a connection: {head}"
        );
        assert_eq!(
            closed,
            Ok((1001, nvs_runtime::Closing::ShuttingDown.reason().to_owned())),
            "the drain did not leave the peer RFC 6455's `going away`, which is \
             the clean close § 7 asks for in place of a reset"
        );
        let said = said.borrow();
        assert_eq!(
            said.len(),
            3,
            "the connection isolate did not end its own loop, or the accept \
             loop never came out of its drain: {said:?}"
        );
        assert_eq!(
            said[1], "connection the loop ended after 0 frames",
            "the close reached the peer without `receive` answering § 3's \
             `null`, so the program's loop was not what ended: {said:?}"
        );
        assert_eq!(
            said[2], "the accept loop drained and returned",
            "the drain returned before the connection it was draining: {said:?}"
        );
    }

    /// `rule:concurrency/a-drain-closes-a-connection-cleanly` on a WebSocket
    /// whose program is already parked in `receive` when the drain begins: it
    /// is woken and closes at once, rather than at its idle wait's end five
    /// minutes on. The case above begins the drain before the program's first
    /// `receive`; here the loop is ended from the client ([`end_the_loop`])
    /// only once the program has said it is about to wait, which is the
    /// ordering a real shutdown has.
    ///
    /// **The program says so on the socket itself**, one frame before its
    /// `receive`, and the client ends the loop only after reading it: a `send`
    /// that has returned and a `receive` that parks are one run of the task
    /// with no park between them, so by the time the loop's tail begins the
    /// drain the read is parked and the cut is what ends it. Read against the
    /// clock: the idle bound is the shipped five minutes and the client's
    /// patience is [`CLIENT_PATIENCE`], so a drain that left the program parked
    /// fails by the clock rather than passing slowly.
    #[test]
    fn a_websocket_parked_on_its_peer_is_closed_the_moment_the_drain_begins() {
        fn say_ready_then_wait(conn: &mut Ctx) -> String {
            let Some(peer) = conn.peer() else {
                return "no peer".to_owned();
            };
            if let Err(error) = peer.send(nvs_runtime::PeerFrame::Text("ready".to_owned())) {
                return format!("send {error}");
            }
            match peer.receive() {
                Ok(Some(_)) => "a frame arrived".to_owned(),
                Ok(None) => "the loop ended".to_owned(),
                Err(error) => format!("receive {error}"),
            }
        }

        let mut listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .set_read_timeout(Some(CLIENT_PATIENCE))
                .expect("the socket refused a read timeout");
            socket
                .write_all(upgrade_request("/chat").as_bytes())
                .expect("the write failed");
            let mut head = String::new();
            read_until(&mut socket, "\r\n\r\n", &mut head);
            let mut peer = tungstenite::protocol::WebSocket::from_raw_socket(
                socket,
                tungstenite::protocol::Role::Client,
                None,
            );
            let ready = match peer.read() {
                Ok(tungstenite::Message::Text(text)) => text.to_string(),
                other => format!("{other:?}"),
            };
            end_the_loop(addr);
            let began = Instant::now();
            let closed = match peer.read() {
                Ok(tungstenite::Message::Close(Some(frame))) => {
                    Ok((u16::from(frame.code), frame.reason.to_string()))
                }
                other => Err(format!("{other:?}")),
            };
            (head, ready, closed, began.elapsed())
        });

        let said = Rc::new(RefCell::new(Vec::new()));
        let handler_said = Rc::clone(&said);
        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
            let handler = upgrade_leaving(
                Door::Socket,
                String::new(),
                handler_said,
                say_ready_then_wait,
            );
            serve_on_this_core(
                &mut listener,
                &handler,
                Waits::default(),
                &wide_open(),
                &Draining::detached(),
                |_note| {},
                until_the_loop_is_ended(),
            )
            .expect("the accept loop failed");
        });
        nvs_host::run_until_idle(&mut sched).expect("the loop failed");
        let (head, ready, closed, took) = client.join().expect("the client thread panicked");

        assert!(
            head.contains("101 Switching Protocols"),
            "the peer was not answered RFC 6455's handshake: {head}"
        );
        assert_eq!(
            ready, "ready",
            "the program did not say it was about to wait"
        );
        assert_eq!(
            closed,
            Ok((1001, nvs_runtime::Closing::ShuttingDown.reason().to_owned())),
            "the drain did not leave a parked peer RFC 6455's `going away`"
        );
        assert!(
            took < Duration::from_secs(5),
            "the drain left the program parked on its peer: closed after {took:?}"
        );
        let said = said.borrow();
        assert!(
            said.iter().any(|line| line == "connection the loop ended"),
            "the close reached the peer without `receive` answering § 3's \
             `null`: {said:?}"
        );
    }

    /// `rule:concurrency/a-drain-closes-a-connection-cleanly` on a program
    /// that only sends: a push feed that sends, sleeps and sends again, with
    /// no `receive` for the drain to end. The sleep is ten minutes, so a
    /// drain that only reached the program at the sleep's end fails by the
    /// clock; what is asserted is that the drain wakes the sleep, the sleep
    /// then ends at the drain deadline, and the program's next `send` throws
    /// there and leaves the peer `going away` — with no further update sent,
    /// since the program was asleep for the whole period.
    ///
    /// The loop is ended from the client once the first update is in, so the
    /// program is asleep when the drain begins: a `send` that has returned and
    /// a sleep that parks are one run of the task with no park between them.
    #[test]
    fn a_send_only_websocket_program_is_ended_at_the_periods_end() {
        fn push_until_told(conn: &mut Ctx) -> String {
            let mut sent = 0_usize;
            loop {
                let Some(peer) = conn.peer() else {
                    return "no peer".to_owned();
                };
                if let Err(error) = peer.send(nvs_runtime::PeerFrame::Text(format!("tick {sent}")))
                {
                    return format!("send {sent} failed: {error}");
                }
                sent += 1;
                nvs_runtime::peer::sleep(conn, Duration::from_secs(600));
            }
        }

        let mut listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .set_read_timeout(Some(CLIENT_PATIENCE))
                .expect("the socket refused a read timeout");
            socket
                .write_all(upgrade_request("/feed").as_bytes())
                .expect("the write failed");
            let mut head = String::new();
            read_until(&mut socket, "\r\n\r\n", &mut head);
            let mut peer = tungstenite::protocol::WebSocket::from_raw_socket(
                socket,
                tungstenite::protocol::Role::Client,
                None,
            );
            let first = match peer.read() {
                Ok(tungstenite::Message::Text(text)) => text.to_string(),
                other => format!("{other:?}"),
            };
            end_the_loop(addr);
            let began = Instant::now();
            let mut ticks = 0_usize;
            let closed = loop {
                match peer.read() {
                    Ok(tungstenite::Message::Text(_)) => ticks += 1,
                    Ok(tungstenite::Message::Close(Some(frame))) => {
                        break Ok((u16::from(frame.code), frame.reason.to_string()));
                    }
                    other => break Err(format!("{other:?}")),
                }
            };
            (first, ticks, closed, began.elapsed())
        });

        let said = Rc::new(RefCell::new(Vec::new()));
        let handler_said = Rc::clone(&said);
        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
            let handler =
                upgrade_leaving(Door::Socket, String::new(), handler_said, push_until_told);
            serve_on_this_core(
                &mut listener,
                &handler,
                Waits::default(),
                &wide_open(),
                &Draining::detached(),
                |_note| {},
                until_the_loop_is_ended(),
            )
            .expect("the accept loop failed");
        });
        nvs_host::run_until_idle(&mut sched).expect("the loop failed");
        let (first, ticks, closed, took) = client.join().expect("the client thread panicked");

        assert_eq!(first, "tick 0", "the feed did not send its first update");
        assert_eq!(
            closed,
            Ok((1001, nvs_runtime::Closing::ShuttingDown.reason().to_owned())),
            "a send past the drain deadline did not leave the peer `going away`"
        );
        assert_eq!(
            ticks, 0,
            "a program asleep for the whole period sent another update"
        );
        assert!(
            took < Duration::from_secs(5),
            "the drain waited for a ten-minute sleep: closed after {took:?}"
        );
        let said = said.borrow();
        assert!(
            said.iter()
                .any(|line| line.starts_with("connection send ") && line.contains("shutting down")),
            "the program was not ended by its `send` throwing: {said:?}"
        );
    }

    /// `rule:concurrency/two-doors-one-isolate`:
    /// "the isolate is the same; the door is not". An event stream opens the
    /// same root isolate the three cases above assert of § 1 — its own context,
    /// its own output, none of the request's state — out of a request nothing
    /// framed an upgrade for.
    ///
    /// **The plain `GET` is the case.** § 5's cell is offered to *every*
    /// request the server runs, so the opening carries no `Connection:
    /// Upgrade`, and the request reports § 1's slot absent in the same line it
    /// reports § 5's cell filled. That pair is the whole of "with no receive",
    /// and it is permanent rather than a stage of the work: there is no socket
    /// behind this connection for a peer frame to arrive on, so the isolate has
    /// nothing to wait on and nothing but `send` to do — where an implementation
    /// that
    /// offered the cell only where an upgrade *was* framed would open the same
    /// isolate and still fail here, because no upgrade was.
    ///
    /// Where the isolate's bytes go is deliberately not asserted here: that the
    /// body they arrive as is this response's is
    /// [`what_the_connection_isolate_sends_reaches_the_peer_as_this_responses_body`]'s
    /// claim, and this case is about the isolate the door opens.
    #[test]
    fn sse_is_a_connection_isolate_with_no_receive() {
        fn say_what_it_can_see(conn: &mut Ctx) -> String {
            conn.inbound().map_or_else(
                || "no request".to_owned(),
                |inbound| format!("{}?{}", inbound.path(), inbound.query()),
            )
        }

        let said = Rc::new(RefCell::new(Vec::new()));
        let handler_said = Rc::clone(&said);
        let seen = upgrade_once(
            move || {
                upgrade_leaving(
                    Door::Sse,
                    "session=abc123".to_owned(),
                    handler_said,
                    say_what_it_can_see,
                )
            },
            "/live",
            Door::Sse,
        );
        assert!(
            !seen.contains(&Door::Sse.answered("/live", NO_FRAMING)),
            "the request's own answer was sent in place of the event stream's \
             response: {seen}"
        );

        let said = said.borrow();
        assert_eq!(
            said.len(),
            2,
            "one of the two isolates did not run: {said:?}"
        );
        assert!(
            said[0].ends_with(&Door::Sse.answered("/live", NO_FRAMING)),
            "an ordinary request was offered no event stream cell, or § 1's \
             slot was offered to it as well: {said:?}"
        );
        assert_eq!(
            said[1], "connection no request",
            "the event stream did not open a root isolate after the request \
             ended: {said:?}"
        );
    }

    /// Goal prose stage 4: **an event stream is answered `200
    /// text/event-stream`**, written by this door and not by the program that
    /// asked for one.
    ///
    /// The head is asserted line by line because not one of them is optional:
    /// the status the protocol fixes, the media type without which a client is
    /// not reading events at all, and
    /// [`nvs_runtime::sse::DECLARED_HEADERS`] — one instruction to a cache and
    /// one to a proxy, without which the stream arrives late or not at all.
    /// `hyper` writes a field name lower-cased, so the reading is taken that
    /// way.
    #[test]
    fn an_event_stream_is_answered_two_hundred_with_the_event_stream_media_type() {
        fn say_it_opened(_conn: &mut Ctx) -> String {
            "opened".to_owned()
        }

        let seen = upgrade_once(
            move || {
                upgrade_leaving(
                    Door::Sse,
                    String::new(),
                    Rc::new(RefCell::new(Vec::new())),
                    say_it_opened,
                )
            },
            "/live",
            Door::Sse,
        );
        let head = seen.to_ascii_lowercase();

        assert!(
            seen.starts_with("HTTP/1.1 200 OK"),
            "an event stream was answered something other than the status the \
             protocol fixes for it: {seen}"
        );
        assert!(
            head.contains("content-type: text/event-stream"),
            "the response a client is to read events from did not say they \
             were events: {seen}"
        );
        assert!(
            head.contains("cache-control: no-cache, no-transform")
                && head.contains("x-accel-buffering: no"),
            "the head went out without what makes the stream arrive: {seen}"
        );
    }

    /// A response wait short enough to be a test's running time and long enough
    /// that a scheduling delay is not the reason a case passed.
    const SHORT_WRITE_IDLE: Duration = Duration::from_millis(400);

    /// How long the idle stream below stays open: several beats derived from
    /// [`SHORT_WRITE_IDLE`], so the claim is that the wait is outlived over and
    /// over rather than survived once.
    const STAYS_OPEN: Duration = Duration::from_millis(1_400);

    /// A connection isolate that opens an event stream and sends nothing at
    /// all, which is the ordinary state of one rather than a fault.
    fn say_nothing_for_a_while(_conn: &mut Ctx) -> String {
        let _ = nvs_host::sleep(STAYS_OPEN);
        "idled".to_owned()
    }

    /// `rule:concurrency/connection-bounds-are-finite` on the door that has no
    /// frames: **an event stream that sends nothing outlives the wait that
    /// would otherwise close it**.
    ///
    /// Without the keep-alive this is the case that fails by the peer's
    /// connection being closed mid-body — `write_idle` bounds the response
    /// phase, an event stream is a response being written, and a stream saying
    /// nothing is what SSE is for. The reading is taken twice over: the body
    /// ends with its terminating chunk, which it cannot do on a connection that
    /// was closed under it, and the comment lines
    /// [`nvs_runtime::sse::KEEPALIVE`] are there to say what moved the byte.
    ///
    /// **The count is the claim**, and it is derived rather than picked: enough
    /// beats to cover [`SHORT_WRITE_IDLE`] and one more is a stream that was
    /// still being written past the instant the wait would have closed it.
    /// [`STAYS_OPEN`] buys several times that, so what a loaded machine can
    /// cost this case is beats it does not need.
    #[test]
    fn an_idle_event_stream_outlives_the_write_idle_wait() {
        let seen = upgrade_once_under(
            Waits {
                write_idle: SHORT_WRITE_IDLE,
                ..Waits::default()
            },
            move || {
                upgrade_leaving(
                    Door::Sse,
                    String::new(),
                    Rc::new(RefCell::new(Vec::new())),
                    say_nothing_for_a_while,
                )
            },
            "/live",
            Door::Sse,
        );

        let beat = crate::bounds::heartbeat(SHORT_WRITE_IDLE);
        let past_the_wait = usize::try_from(SHORT_WRITE_IDLE.as_millis() / beat.as_millis())
            .expect("the beats inside one wait are countable")
            + 1;
        let keepalives = seen
            .matches(
                std::str::from_utf8(nvs_runtime::sse::KEEPALIVE).expect("a comment line is text"),
            )
            .count();
        assert!(
            keepalives >= past_the_wait,
            "a stream idle for {STAYS_OPEN:?} wrote {keepalives} keep-alive(s) \
             at {beat:?}, which does not reach past the {SHORT_WRITE_IDLE:?} \
             wait that closes one: {seen:?}"
        );
    }

    /// `rule:concurrency/two-doors-one-isolate`'s hand-over at the end it
    /// exists for: **what the connection isolate sends is this response's
    /// body**.
    ///
    /// Two readings, because [`nvs_host::Isolate::over_event_stream`] is one
    /// hand-over carrying both. The isolate holds a writing half at all, which
    /// is what `Core\Sse->send` reaches for and what puts its bytes on this
    /// wire; and its context is marked as writing an event stream, which is the
    /// only thing `Core\Sse::current` can answer inside a door that was handed
    /// no peer. Neither could have arrived by inheritance — the isolate is a
    /// root with no request in front of it, which the case beside this one is
    /// the reading of.
    #[test]
    fn what_the_connection_isolate_sends_reaches_the_peer_as_this_responses_body() {
        fn send_one_event(conn: &mut Ctx) -> String {
            let marked = conn.has_event_stream();
            let framed = nvs_runtime::sse::Event::carrying(b"through the body")
                .frame()
                .expect("a payload with nothing in it to refuse");
            conn.body_stream()
                .expect("an event stream's isolate was handed no body")
                .send(framed)
                .expect("the first chunk goes into an empty cell");
            format!("sent, marked {marked}")
        }

        let said = Rc::new(RefCell::new(Vec::new()));
        let handler_said = Rc::clone(&said);
        let seen = upgrade_once(
            move || upgrade_leaving(Door::Sse, String::new(), handler_said, send_one_event),
            "/live",
            Door::Sse,
        );

        let body = seen
            .split_once("\r\n\r\n")
            .expect("the response carried no head")
            .1;
        assert!(
            body.contains("data: through the body"),
            "what the connection isolate sent was not the body of the response \
             the request was answered with: {seen}"
        );
        let said = said.borrow();
        assert_eq!(
            said.len(),
            2,
            "one of the two isolates did not run: {said:?}"
        );
        assert_eq!(
            said[1], "connection sent, marked true",
            "the event stream's isolate was handed a body it was not told the \
             meaning of: {said:?}"
        );
    }

    /// [`the_upgrading_requests_arena_is_released_while_the_connection_is_open`]
    /// for § 5's door, where the ordering is tighter and the claim is the same:
    /// `rule:concurrency/a-connection-is-a-root-isolate`'s "the request that
    /// upgraded it ends" holds for an isolate started **inside** the request's
    /// own future, a slice after that request was joined rather than after
    /// `hyper` stopped framing.
    ///
    /// This door is the one where the ordering could plausibly have gone the
    /// other way — an event stream waits for no socket, so nothing but the rule
    /// stops the isolate being started while the request that asked for it is
    /// still live. The two mebibytes on that request's carrier are what says it
    /// was not.
    #[test]
    fn the_upgrading_requests_arena_is_released_before_the_streams_isolate_starts() {
        let said = Rc::new(RefCell::new(Vec::new()));
        let handler_said = Rc::clone(&said);
        let seen = upgrade_once(
            move || {
                upgrade_leaving(
                    Door::Sse,
                    format!("q={}", "x".repeat(CARRIED)),
                    handler_said,
                    say_the_balance,
                )
            },
            "/live",
            Door::Sse,
        );

        assert!(
            seen.starts_with("HTTP/1.1 200 OK"),
            "the request that opened the stream did not reach its own end: {seen}"
        );
        let said = said.borrow();
        assert_eq!(
            said.len(),
            2,
            "one of the two isolates did not run: {said:?}"
        );
        let peak = balance(&said[0], "request ");
        let open = balance(&said[1], "connection ");
        assert!(
            peak - open >= CARRIED.cast_signed() - CONNECTIONS_OWN,
            "the upgrading request's arena was still held while the event \
             stream's isolate ran: {peak} bytes live under the request, {open} \
             under the connection, and the query alone is {CARRIED}"
        );
    }

    /// `rule:concurrency/two-doors-one-isolate`'s
    /// two cells, filled by one request: a program that asked for a socket
    /// *and* an event stream asked for two responses where the connection has
    /// one, and [`serve_connection`] refuses it rather than picking.
    ///
    /// `nvs_runtime::SseSlot::fill` is where that reading is recorded — neither
    /// cell can see the other, so the contradiction is decided where the
    /// response is written — and this is it decided. The assertion is on both
    /// halves of the refusal: the peer gets the `500` and **not** the answer the
    /// request wrote for itself, and neither prepared isolate runs. What the
    /// discarded upgrades' arguments cost is `nvs_runtime::Upgrade::discard`'s,
    /// and the valgrind leg is what reads that.
    #[test]
    fn a_request_that_asks_for_both_hand_overs_is_refused() {
        fn never_reached(_conn: &mut Ctx) -> String {
            "opened".to_owned()
        }

        let said = Rc::new(RefCell::new(Vec::new()));
        let handler_said = Rc::clone(&said);
        let seen = upgrade_once(
            move || upgrade_leaving(Door::Both, String::new(), handler_said, never_reached),
            "/two",
            Door::Both,
        );
        assert!(
            seen.contains("500 Internal Server Error") && !seen.contains("/two both"),
            "a request that filled both cells was answered as though one of \
             them had been honoured: {seen}"
        );

        let said = said.borrow();
        assert_eq!(
            said.len(),
            1,
            "a connection isolate was started for a request that asked for two \
             of them: {said:?}"
        );
    }

    /// The handler `rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing`'s case answers with: the walk's two answers put
    /// on the carrier exactly as `nvs-cli`'s door puts them there, and a program
    /// that says them back.
    ///
    /// It reads nothing off the request itself — not even the headers the walk
    /// decided from — so what the assertion sees is the walk's answer and could
    /// not be a header echoed back under another name.
    fn echo_the_peer() -> Rc<impl Fn(Request<Incoming>, Origin) -> Reply> {
        Rc::new(|_request: Request<Incoming>, origin: Origin| {
            let mut inbound = nvs_runtime::Inbound::new("GET", "/", "");
            inbound.set_peer(origin.client(), origin.scheme());
            let program: Program = Box::new(move |child: &mut Ctx, _args| {
                let said = {
                    let inbound = child
                        .inbound()
                        .expect("the isolate ran with no request in front of it");
                    format!(
                        "{} {:?}",
                        inbound
                            .client()
                            .map_or_else(|| "none".to_owned(), |ip| ip.to_string()),
                        inbound.scheme()
                    )
                };
                child.write_output(said.as_bytes()).expect("a buffer");
                Value::null()
            });
            Reply::run(Isolate::new(program, Value::null(), Output::Capture).answering(inbound))
        })
    }

    /// The request that claims to have been forwarded, which is the only
    /// request either half below sends: what changes between them is who this
    /// server was told to believe.
    fn claiming_to_be_forwarded(addr: std::net::SocketAddr) -> std::thread::JoinHandle<String> {
        std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .write_all(
                    b"GET / HTTP/1.1\r\nHost: localhost\r\nX-Forwarded-For: 203.0.113.9\r\n\
                      X-Forwarded-Proto: https\r\nConnection: close\r\n\r\n",
                )
                .expect("the write failed");
            let mut answer = String::new();
            socket
                .read_to_string(&mut answer)
                .expect("the response could not be read");
            answer
        })
    }

    /// `rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing`'s answer reaches the program, and it is the *walk's* answer:
    /// `nvs_runtime::Inbound` carries the client address and the effective
    /// scheme, which is what `Core\Request::clientIp()` and `::scheme()` read.
    ///
    /// Asserted from both sides of the trust decision with **one request**, so
    /// the case is about who was allowed to speak for the peer rather than
    /// about a header being parsed. With nothing configured the two forwarded
    /// lines are inert and the carrier says the socket's own peer over
    /// plaintext; with the loopback trusted the same lines are the answer. A
    /// carrier that stored what arrived rather than what the walk decided would
    /// print the same string for both halves.
    #[test]
    fn the_walks_answer_reaches_the_program_on_the_carrier() {
        let listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");
        let answer = served_under(
            listener,
            &echo_the_peer(),
            claiming_to_be_forwarded(addr),
            wide_open(),
        );
        assert!(
            answer.ends_with("127.0.0.1 Http"),
            "an untrusted peer speaks only for itself: {answer}"
        );

        let listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");
        let (trusted, rejected) = Trusted::of(&["127.0.0.1/32".to_owned()]);
        assert!(
            rejected.is_empty(),
            "the fixture named no network: {rejected:?}"
        );
        let behind_a_proxy = Serving::new(
            Arc::new(Admission::new(&Ceiling::of(&Capacity {
                configured: 10_000,
                per_request: None,
                budget: None,
            }))),
            Arc::new(Secure::default()),
            Arc::new(trusted),
            Arc::new(Cors::default()),
            Arc::default(),
        );
        let answer = served_under(
            listener,
            &echo_the_peer(),
            claiming_to_be_forwarded(addr),
            behind_a_proxy,
        );
        assert!(
            answer.ends_with("203.0.113.9 Https"),
            "a trusted proxy states both facts, and both reach the carrier: {answer}"
        );
    }

    /// The two field lines the carrier cases below arrive with, so that what
    /// differs between them is the trust directive and never the request.
    fn a_forwarded_request() -> hyper::HeaderMap {
        let mut headers = hyper::HeaderMap::new();
        headers.append(
            HeaderName::from_static("x-forwarded-for"),
            HeaderValue::from_static("203.0.113.9"),
        );
        headers.append(
            HeaderName::from_static("x-forwarded-proto"),
            HeaderValue::from_static("https"),
        );
        headers
    }

    /// What `rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing` decided is what the carrier holds: with the
    /// loopback written into `trusted_proxies`, both facts the proxy stated
    /// reach `nvs_runtime::Inbound`, which is where `Core\Request::clientIp()`
    /// and `::scheme()` read them.
    ///
    /// Asserted at the seam itself — the two lines [`echo_the_peer`] and
    /// `nvs-cli`'s door both write — rather than through a socket, so a carrier
    /// that stored the peer or the connection's own scheme fails here with no
    /// listener, thread or program standing between the walk and the answer.
    #[test]
    fn the_forwarded_walks_answer_is_set_on_the_inbound() {
        let (trusted, rejected) = Trusted::of(&["127.0.0.1/32".to_owned()]);
        assert!(
            rejected.is_empty(),
            "the fixture named no network: {rejected:?}"
        );
        let origin = crate::forwarded::walk(
            Arrival::Tcp("127.0.0.1".parse().expect("a literal address")),
            &trusted,
            &a_forwarded_request(),
        )
        .expect("a chain of one readable address");

        let mut inbound = nvs_runtime::Inbound::new("GET", "/", "");
        inbound.set_peer(origin.client(), origin.scheme());

        assert_eq!(
            inbound.client(),
            Some("203.0.113.9".parse().expect("a literal address")),
            "a trusted proxy states the client address, and the carrier holds it"
        );
        assert_eq!(
            inbound.scheme(),
            Scheme::Https,
            "the same proxy states the scheme, and one walk answered both"
        );
    }

    /// The other side of that decision, in the two spellings that reach it: a
    /// `trusted_proxies` that is empty, and one that names a network this peer
    /// is not in. Both leave the carrier holding the socket's own peer over
    /// plaintext, with the forwarded lines present and unread.
    ///
    /// They are asserted together because the walk answers them on one branch
    /// while the rule keeps them apart — empty means the headers are never
    /// parsed, an untrusted peer means they are ignored silently — so a change
    /// that turned either into a refusal, or that read one of them, has to fail
    /// something. Nothing here is a `400`: the value was never about to be used.
    #[test]
    fn an_untrusted_peers_forwarded_header_does_not_reach_the_carrier() {
        let peer: std::net::IpAddr = "127.0.0.1".parse().expect("a literal address");
        let (elsewhere, rejected) = Trusted::of(&["10.0.0.0/8".to_owned()]);
        assert!(
            rejected.is_empty(),
            "the fixture named no network: {rejected:?}"
        );

        for (directive, trusted) in [
            ("empty", Trusted::none()),
            ("naming a network this peer is not in", elsewhere),
        ] {
            let origin =
                crate::forwarded::walk(Arrival::Tcp(peer), &trusted, &a_forwarded_request())
                    .expect("a chain that is never read cannot refuse");

            let mut inbound = nvs_runtime::Inbound::new("GET", "/", "");
            inbound.set_peer(origin.client(), origin.scheme());

            assert_eq!(
                inbound.client(),
                Some(peer),
                "with trusted_proxies {directive}, the client is the socket peer"
            );
            assert_eq!(
                inbound.scheme(),
                Scheme::Http,
                "with trusted_proxies {directive}, X-Forwarded-Proto states nothing"
            );
        }
    }

    /// The isolate the two body cases answer with: a handler that splits the
    /// arrived body the way `nvs-cli`'s door does, and a program that pulls it
    /// to its end off its own context.
    ///
    /// It says the bytes rather than a length, so that a case asserting them is
    /// asserting order and completeness together — a supplier that dropped a
    /// chunk or answered one twice would still report a plausible count.
    fn echo_the_body() -> Rc<impl Fn(Request<Incoming>, Origin) -> Reply> {
        Rc::new(|request: Request<Incoming>, _origin: Origin| {
            let mut inbound = nvs_runtime::Inbound::new(
                request.method().as_str(),
                request.uri().path(),
                request.uri().query().unwrap_or(""),
            );
            let (head, incoming) = request.into_parts();
            let supply = match crate::body::of(&head.headers, incoming) {
                crate::body::Arrived::Streaming(supply, pull) => {
                    inbound.set_body(pull);
                    Some(supply)
                }
                crate::body::Arrived::Absent | crate::body::Arrived::TooLarge => None,
            };
            let program: Program = Box::new(move |child: &mut Ctx, _args| {
                let mut said = String::new();
                {
                    let inbound = child
                        .inbound_mut()
                        .expect("the isolate ran with no request in front of it");
                    match inbound.body() {
                        None => said.push_str("no body"),
                        Some(body) => {
                            said.push_str("body=");
                            loop {
                                match body.next_chunk() {
                                    Ok(Some(chunk)) => {
                                        said.push_str(&String::from_utf8_lossy(chunk));
                                    }
                                    Ok(None) => break,
                                    Err(message) => {
                                        said = format!("failed: {message}");
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                child.write_output(said.as_bytes()).expect("a buffer");
                Value::null()
            });
            Reply::Run(
                Isolate::new(program, Value::null(), Output::Capture).answering(inbound),
                supply,
            )
        })
    }

    /// A body reaches the program in full, and the pull that read it parked.
    ///
    /// The pause between the two halves is the case rather than realism: the
    /// second half cannot be on the wire when the program asks for it, so the
    /// only way this answers at all is the shape `rule:http-server/request-body-and-upload-total-are-two-caps` and `rule:concurrency/one-future-per-connection`
    /// name together — the isolate parks on its own task, the connection's next
    /// read delivers, and the isolate is woken back. A supplier polled from
    /// inside the connection's own poll would deadlock here instead of
    /// answering late.
    #[test]
    fn a_request_body_crosses_to_the_isolate_in_full_across_a_park() {
        let listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .write_all(
                    b"POST /upload HTTP/1.1\r\nHost: localhost\r\nContent-Length: 8\r\n\
                      Connection: close\r\n\r\nabcd",
                )
                .expect("the write failed");
            std::thread::sleep(Duration::from_millis(50));
            socket.write_all(b"efgh").expect("the second write failed");
            let mut answer = String::new();
            socket
                .read_to_string(&mut answer)
                .expect("the response could not be read");
            answer
        });

        let answer = served_by(listener, &echo_the_body(), client);
        assert!(
            answer.ends_with("body=abcdefgh"),
            "the body did not reach the program in full and in order: {answer}"
        );
    }

    /// The isolate the streaming cases answer with: a program that opens a
    /// response body stream, writes a chunk, waits for the other half of its
    /// **own** request body, and writes the last chunk before it ends.
    ///
    /// **The park in the middle is what makes the ordering a fact rather than a
    /// race.** The second half of the request body is not on the wire until the
    /// client has read the head, so a program that reaches its last chunk at
    /// all was still running when that head went out — asserted with no sleep
    /// and with nothing said about which task happened to run first.
    ///
    /// It opens the cell directly rather than through `Core\Response::stream`
    /// for the reason every program in this module is a Rust closure: this
    /// crate compiles nothing. `nvs_stdlib::response`'s own cases are the
    /// member's half, and the `.nvst` corpus is the language's.
    fn stream_across_a_park() -> Rc<impl Fn(Request<Incoming>, Origin) -> Reply> {
        Rc::new(|request: Request<Incoming>, _origin: Origin| {
            let mut inbound = nvs_runtime::Inbound::new(
                request.method().as_str(),
                request.uri().path(),
                request.uri().query().unwrap_or(""),
            );
            let (head, incoming) = request.into_parts();
            let supply = match crate::body::of(&head.headers, incoming) {
                crate::body::Arrived::Streaming(supply, pull) => {
                    inbound.set_body(pull);
                    Some(supply)
                }
                crate::body::Arrived::Absent | crate::body::Arrived::TooLarge => None,
            };
            let program: Program = Box::new(move |child: &mut Ctx, _args| {
                let cell = child
                    .inbound()
                    .and_then(nvs_runtime::Inbound::response_stream_slot)
                    .cloned()
                    .expect("a served request was offered no response-stream cell");
                let emit = cell
                    .open("text/csv", Some(201), Vec::new())
                    .expect("the first stream on a request opens");
                child.set_body_stream(emit);
                child
                    .body_stream()
                    .expect("the writing half was just set")
                    .send(b"opened;".to_vec())
                    .expect("the first chunk goes into an empty cell");
                {
                    let inbound = child
                        .inbound_mut()
                        .expect("the isolate ran with no request in front of it");
                    if let Some(body) = inbound.body() {
                        while matches!(body.next_chunk(), Ok(Some(_))) {}
                    }
                }
                child
                    .body_stream()
                    .expect("the writing half is still there")
                    .send(b"closed".to_vec())
                    .expect("the last chunk reaches a connection that is still reading");
                Value::null()
            });
            Reply::Run(
                Isolate::new(program, Value::null(), Output::Capture).answering(inbound),
                supply,
            )
        })
    }

    /// The client half of [`stream_across_a_park`]: the request's body arrives
    /// in two writes, and the second is sent only once the streamed head and
    /// its first chunk have been read.
    ///
    /// `opened` is called in between, which is the one moment a case can ask
    /// anything of a request whose answer has started and whose program has not
    /// finished.
    fn read_the_head_then_finish(
        addr: std::net::SocketAddr,
        opened: impl FnOnce() + Send + 'static,
    ) -> std::thread::JoinHandle<String> {
        std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .write_all(
                    b"POST /export HTTP/1.1\r\nHost: localhost\r\nContent-Length: 8\r\n\
                      Connection: close\r\n\r\nabcd",
                )
                .expect("the write failed");
            let mut seen = Vec::new();
            let mut buffer = [0_u8; 512];
            while !String::from_utf8_lossy(&seen).contains("opened;") {
                let read = socket
                    .read(&mut buffer)
                    .expect("the response could not be read");
                assert!(
                    read > 0,
                    "the connection closed before the streamed head arrived: {}",
                    String::from_utf8_lossy(&seen)
                );
                seen.extend_from_slice(&buffer[..read]);
            }
            opened();
            socket.write_all(b"efgh").expect("the second write failed");
            let mut rest = String::new();
            socket
                .read_to_string(&mut rest)
                .expect("the rest of the response could not be read");
            String::from_utf8_lossy(&seen).into_owned() + &rest
        })
    }

    /// Goal prose stage 3 item 12, as the ordering it names: **a streaming
    /// response returns the head as soon as the door is called**, with the
    /// isolate that opened it still running.
    ///
    /// The assertion is causal rather than timed. The program's last chunk is
    /// written after a pull that only the client's second write can satisfy,
    /// and the client sends that write only after it has read the head — so a
    /// response carrying both chunks is one whose head went out while the
    /// isolate was still going. A door that waited for the completion, as every
    /// other reply on this connection does, would deadlock here instead of
    /// answering late, which is the sharp end of `rule:concurrency/a-stream-that-outlives-its-request-is-a-connection`.
    ///
    /// The head is asserted whole, because it is the half no completion can
    /// carry any more: the status and the content type the request had declared
    /// when it opened the stream, and the chunked framing that
    /// [`Answer::size_hint`] leaves `hyper` to choose.
    #[test]
    fn a_streaming_response_answers_its_head_before_the_isolate_ends() {
        let listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let client = read_the_head_then_finish(addr, || {});
        let answer = served_by(listener, &stream_across_a_park(), client);
        let head = answer.to_ascii_lowercase();

        assert!(
            answer.starts_with("HTTP/1.1 201 "),
            "the head did not carry the status the request had declared: {answer}"
        );
        assert!(
            head.contains("content-type: text/csv"),
            "the head did not declare what the stream was opened at: {answer}"
        );
        assert!(
            head.contains("transfer-encoding: chunked"),
            "a body of no exact size was not chunked: {answer}"
        );
        assert!(
            answer.contains("opened;") && answer.contains("closed"),
            "the body did not carry both chunks: {answer}"
        );
    }

    /// A stub isolate whose end the case decides, recording whether it was
    /// joined or thrown away.
    #[derive(Debug)]
    struct Writing {
        ended: Rc<Cell<bool>>,
        joined: Rc<Cell<bool>>,
    }

    impl Running for Writing {
        fn join(self: Box<Self>, _ctx: &mut Ctx) -> Completion {
            self.joined.set(true);
            Completion {
                ok: true,
                value: Value::null(),
                output: Vec::new(),
                content_type: None,
                file_body: None,
                status: None,
                headers: Vec::new(),
                error: None,
                wall: None,
                trace: Vec::new(),
            }
        }

        fn finished(&self) -> bool {
            self.ended.get()
        }

        fn abandon(self: Box<Self>) {}
    }

    /// Goal prose stage 3 item 12's other half: the isolate is joined when the
    /// body ends, and not before.
    ///
    /// **Asserted at the one line that decides it** rather than over a socket,
    /// because the failure it guards has no observable answer: joining early is
    /// not a slow path but a deadlock — `nvs_host::Running::join` parks until
    /// the child has ended, the child cannot end until the chunks it is parked
    /// on have been taken, and the task that would take them is the one that
    /// just parked. A case that reached for that over a connection would assert
    /// nothing and hang.
    ///
    /// The place under the in-flight ceiling is asserted with it, because the
    /// two are one act: what is held while the body is being written is the
    /// isolate *and* its place, and what is given back is both.
    #[test]
    fn the_isolate_is_joined_when_the_body_ends_and_not_before() {
        let ended = Rc::new(Cell::new(false));
        let joined = Rc::new(Cell::new(false));
        let admission = Admission::new(&Ceiling::of(&Capacity {
            configured: 4,
            per_request: None,
            budget: None,
        }));
        let writing: RefCell<Option<Streamed<'_>>> = RefCell::new(Some(Streamed {
            peer: Peer {
                running: Some(Box::new(Writing {
                    ended: Rc::clone(&ended),
                    joined: Rc::clone(&joined),
                })),
                supply: None,
                cancels: false,
                grace: nvs_config::app::DISCONNECT_GRACE,
                tree: Ctx::buffered().safepoint_view(),
                draining: Draining::detached(),
                seen: crate::io::DrainSeen::default(),
                period: Duration::ZERO,
            },
            _place: admission.admit().expect("a free place under the ceiling"),
            // Nobody is recording this one: what it asserts is the join, and a
            // graph derived from a fixture's empty completion would be a second
            // claim in a case about one.
            recording: None,
        }));
        let mut ctx = Ctx::buffered();

        joined_when_ended(&writing, &mut ctx);
        assert!(
            !joined.get() && writing.borrow().is_some(),
            "an isolate still writing its body was joined"
        );
        assert_eq!(
            admission.in_flight(),
            1,
            "a request still writing its body gave its place back"
        );

        ended.set(true);
        joined_when_ended(&writing, &mut ctx);
        assert!(
            joined.get(),
            "the isolate was never joined once its body had ended"
        );
        assert!(
            writing.borrow().is_none(),
            "the joined isolate was left on the connection"
        );
        assert_eq!(
            admission.in_flight(),
            0,
            "the place was not given back when the body ended"
        );
    }

    /// Goal prose stage 3 item 13: **the request's own budget still bounds a
    /// streaming response**, and none of `crate::bounds`' connection numbers do.
    ///
    /// `rule:http-server/a-requests-blast-radius-is-bounded-at-four-tiers`'s
    /// tier C is the claim, and the hole it would otherwise have is specific: a
    /// request that answers its head early leaves the service future early, so
    /// a place given back there would stop counting a request that is still
    /// spending a core. A peer that reads one byte a minute would then be a
    /// route to unbounded concurrency, which is exactly the ceiling's job.
    ///
    /// Asked from the client thread at the one moment it is answerable — the
    /// head read, the program still parked — and again on the main thread once
    /// the loop has ended, because a place held forever fails the same rule
    /// from the other side. What is *not* asserted is any of § 7's connection
    /// bounds: a streaming request frames no connection and takes no
    /// `crate::bounds::Slot`, the send timeout it writes under being the one
    /// number of the connection's it meets at all.
    #[test]
    fn a_streaming_request_isolate_is_still_bounded_by_the_requests_own_ceiling() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let admission = Arc::new(Admission::new(&Ceiling::of(&Capacity {
            configured: 10_000,
            per_request: None,
            budget: None,
        })));
        let held = Arc::new(AtomicUsize::new(usize::MAX));
        let client = {
            let counted = Arc::clone(&admission);
            let observed = Arc::clone(&held);
            read_the_head_then_finish(addr, move || {
                observed.store(counted.in_flight(), Ordering::Relaxed);
            })
        };
        let serving = Serving::new(
            Arc::clone(&admission),
            Arc::new(Secure::default()),
            Arc::new(Trusted::none()),
            Arc::new(Cors::default()),
            Arc::default(),
        );

        let answer = served_under(listener, &stream_across_a_park(), client, serving);
        assert!(
            answer.contains("closed"),
            "the streamed body never finished: {answer}"
        );
        assert_eq!(
            held.load(Ordering::Relaxed),
            1,
            "a request whose head had gone out was no longer counted in flight"
        );
        assert_eq!(
            admission.in_flight(),
            0,
            "the place a streamed answer held was never given back"
        );
    }

    /// M7's acceptance paragraph, its third clause: **a request whose isolates
    /// are still running when the client disconnects leaves none of them
    /// behind.**
    ///
    /// **The isolate is parked when the peer goes away**, which is what makes
    /// this a case about a request still running rather than about one that
    /// happened to finish first: the client promises a hundred bytes, sends
    /// four and closes, so the program is inside `next_chunk` waiting for the
    /// other ninety-six. The body is also where the disconnect is *noticed* —
    /// the read side of `hyper`'s own loop is what learns the peer is gone
    /// while the service future is still `Pending`.
    ///
    /// **What the door does with it is fail the park, not cut the frame**, and
    /// that is asserted here rather than assumed: the supply dies with the
    /// connection, `next_chunk` answers `Err`, and the program returns through
    /// its own end. An isolate parked on a channel whose sender it holds
    /// itself is not a request this case can use: it wedged this fixture's
    /// core for three minutes, which is the playbook's bullet.
    ///
    /// **Asserted as an ordering rather than as a final state**, which is the
    /// only reading of it that is not vacuous: every task on this core is
    /// dropped when the scheduler is, so "the isolate is gone afterwards" holds
    /// just as well for a door that left it running. What is asserted is that
    /// the isolate was released *before* the accept loop returned. `hyper`
    /// keeps a request in flight after its body failed, so this request ends
    /// through its own return, and the connection writes its answer to a
    /// socket that is gone.
    #[test]
    fn a_client_disconnect_leaves_no_isolate_behind() {
        /// What happened, in the order it happened.
        type Log = Arc<std::sync::Mutex<Vec<&'static str>>>;

        fn note(log: &Log, what: &'static str) {
            log.lock().expect("a poisoned log").push(what);
        }

        /// Files the isolate's release, which is the drop of everything its
        /// program captured — so it fires on the cancellation path and on an
        /// ordinary end alike, and it is the *order* that tells them apart.
        struct Released(Log);

        impl Drop for Released {
            fn drop(&mut self) {
                note(&self.0, "the isolate was released");
            }
        }

        let log: Log = Arc::new(std::sync::Mutex::new(Vec::new()));
        let handler = {
            let log = Arc::clone(&log);
            Rc::new(move |request: Request<Incoming>, _origin: Origin| {
                let mut inbound = nvs_runtime::Inbound::new(
                    request.method().as_str(),
                    request.uri().path(),
                    request.uri().query().unwrap_or(""),
                );
                let (head, incoming) = request.into_parts();
                let supply = match crate::body::of(&head.headers, incoming) {
                    crate::body::Arrived::Streaming(supply, pull) => {
                        inbound.set_body(pull);
                        Some(supply)
                    }
                    crate::body::Arrived::Absent | crate::body::Arrived::TooLarge => None,
                };
                let released = Released(Arc::clone(&log));
                let log = Arc::clone(&log);
                let program: Program = Box::new(move |child: &mut Ctx, _args| {
                    let _held = &released;
                    note(&log, "the isolate started");
                    let inbound = child
                        .inbound_mut()
                        .expect("the isolate ran with no request in front of it");
                    let body = inbound.body().expect("a request that promised a body");
                    // The park this case is about: ninety-six of the hundred
                    // bytes the head promised are never sent, so the isolate is
                    // still inside this loop when the peer goes away.
                    note(
                        &log,
                        loop {
                            match body.next_chunk() {
                                Ok(Some(_chunk)) => {}
                                Ok(None) => break "the body ended",
                                Err(_refused) => break "the body failed",
                            }
                        },
                    );
                    note(&log, "the isolate finished");
                    Value::null()
                });
                Reply::Run(
                    Isolate::new(program, Value::null(), Output::Capture).answering(inbound),
                    supply,
                )
            })
        };

        let mut listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");
        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .write_all(
                    b"POST /forever HTTP/1.1\r\nHost: localhost\r\nContent-Length: 100\r\n\r\nabcd",
                )
                .expect("the write failed");
            // Long enough for the isolate to have started and parked, so the
            // disconnect lands on a request that is genuinely still running.
            std::thread::sleep(Duration::from_millis(50));
            drop(socket);
        });

        let ended = Arc::clone(&log);
        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
            serve_on_this_core(
                &mut listener,
                &handler,
                Waits::default(),
                &wide_open(),
                &Draining::detached(),
                |_note| {},
                || ControlFlow::Break(()),
            )
            .expect("the accept loop failed");
            note(&ended, "the accept loop returned");
        });
        nvs_host::run_until_idle(&mut sched).expect("the loop failed");
        client.join().expect("the client thread panicked");

        let events = log.lock().expect("a poisoned log").clone();
        assert!(
            events.contains(&"the isolate started"),
            "the isolate never ran, so there was nothing to leave behind: {events:?}"
        );
        assert!(
            events.contains(&"the body failed"),
            "a request parked on a body its peer never sent was not told the peer had gone: \
             {events:?}"
        );
        assert!(
            !events.contains(&"the body ended"),
            "the isolate was handed a complete body, so this case never tested a disconnect: \
             {events:?}"
        );
        let released = events
            .iter()
            .position(|event| *event == "the isolate was released")
            .expect("the isolate was still held when the core was torn down");
        let returned = events
            .iter()
            .position(|event| *event == "the accept loop returned")
            .expect("the accept loop never returned, so the connection outlived its client");
        assert!(
            released < returned,
            "the isolate outlived the connection that owned it: {events:?}"
        );
    }

    /// The same property across the fan-out: a client that goes away leaves its
    /// isolate behind on **no** core — not on the core that served it, and not
    /// on any of the cores that never saw it
    /// (`rule:http-server/the-accept-fan-out-is-one-worker-per-core`).
    ///
    /// [`a_client_disconnect_leaves_no_isolate_behind`] is the same disconnect
    /// on one core and owns why the park is where it is and why the assertion is
    /// an *ordering*. What a fleet adds is the half a single core cannot state:
    /// the teardown belongs to the core that accepted the connection, so a
    /// neighbour serving its own request neither takes part in it nor is torn
    /// down by it. Both cores are asserted, and each against its own loop's
    /// return — a release filed after every core had stopped would say nothing
    /// about which core let go of what.
    #[test]
    fn a_disconnected_clients_isolates_are_left_behind_on_no_core() {
        /// What happened, on which core, in the order it happened.
        type Log = Arc<std::sync::Mutex<Vec<String>>>;

        fn note(log: &Log, core: &str, what: &str) {
            log.lock()
                .expect("a poisoned log")
                .push(format!("{core}: {what}"));
        }

        /// Files the isolate's release, which is the drop of everything its
        /// program captured — so it fires on the cancellation path and on an
        /// ordinary end alike, and it is the *order* that tells them apart.
        struct Released(Log, &'static str);

        impl Drop for Released {
            fn drop(&mut self) {
                note(&self.0, self.1, "the isolate was released");
            }
        }

        /// One worker, serving one connection whose program parks on the body
        /// its peer promised. Both cores run this: what differs is what their
        /// clients go on to send.
        fn one_core(
            listener: std::net::TcpListener,
            core: &'static str,
            log: Log,
        ) -> impl FnOnce(&mut nvs_host::Scheduler) + Send + 'static {
            move |sched| {
                let mut listener = NvsListener::from_std(listener)
                    .expect("the OS refused a non-blocking listener");
                let _installed = nvs_host::reactor::install(
                    nvs_host::Reactor::new().expect("the OS refused a poll"),
                );
                let handler = {
                    let log = Arc::clone(&log);
                    Rc::new(move |request: Request<Incoming>, _origin: Origin| {
                        let mut inbound = nvs_runtime::Inbound::new(
                            request.method().as_str(),
                            request.uri().path(),
                            request.uri().query().unwrap_or(""),
                        );
                        let (head, incoming) = request.into_parts();
                        let supply = match crate::body::of(&head.headers, incoming) {
                            crate::body::Arrived::Streaming(supply, pull) => {
                                inbound.set_body(pull);
                                Some(supply)
                            }
                            crate::body::Arrived::Absent | crate::body::Arrived::TooLarge => None,
                        };
                        let released = Released(Arc::clone(&log), core);
                        let log = Arc::clone(&log);
                        let program: Program = Box::new(move |child: &mut Ctx, _args| {
                            let _held = &released;
                            note(&log, core, "the isolate started");
                            let inbound = child
                                .inbound_mut()
                                .expect("the isolate ran with no request in front of it");
                            let body = inbound.body().expect("a request that promised a body");
                            note(
                                &log,
                                core,
                                loop {
                                    match body.next_chunk() {
                                        Ok(Some(_chunk)) => {}
                                        Ok(None) => break "the body ended",
                                        Err(_refused) => break "the body failed",
                                    }
                                },
                            );
                            Value::null()
                        });
                        Reply::Run(
                            Isolate::new(program, Value::null(), Output::Capture)
                                .answering(inbound),
                            supply,
                        )
                    })
                };
                let ended = Arc::clone(&log);
                sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
                    serve_on_this_core(
                        &mut listener,
                        &handler,
                        Waits::default(),
                        &wide_open(),
                        &Draining::detached(),
                        |_note| {},
                        || ControlFlow::Break(()),
                    )
                    .expect("the accept loop failed");
                    note(&ended, core, "the accept loop returned");
                });
                run_the_core(sched);
            }
        }

        let cpus = nvs_host::cpus();
        let Some(first_cpu) = cpus.first().copied() else {
            // A host that enumerates no CPU is served from the boot thread, and
            // the one-core case above is the whole of this property there.
            return;
        };
        let second_cpu = cpus.get(1).copied().unwrap_or(first_cpu);

        let log: Log = Arc::new(std::sync::Mutex::new(Vec::new()));
        let leaving = std::net::TcpListener::bind("127.0.0.1:0").expect("the OS refused a port");
        let leaving_addr = leaving
            .local_addr()
            .expect("a bound listener had no address");
        let staying = std::net::TcpListener::bind("127.0.0.1:0").expect("the OS refused a port");
        let staying_addr = staying
            .local_addr()
            .expect("a bound listener had no address");

        let left = nvs_host::Worker::spawn(
            first_cpu,
            one_core(leaving, "the served core", Arc::clone(&log)),
        )
        .expect("the OS refused a worker thread");
        let kept = nvs_host::Worker::spawn(
            second_cpu,
            one_core(staying, "the neighbouring core", Arc::clone(&log)),
        )
        .expect("the OS refused a worker thread");

        // The client that goes away: a hundred bytes promised, four sent, and
        // then the socket closed while the program is inside `next_chunk`
        // waiting for the other ninety-six.
        let gone = std::thread::spawn(move || {
            let mut socket =
                TcpStream::connect(leaving_addr).expect("the loopback refused a connection");
            socket
                .write_all(
                    b"POST /forever HTTP/1.1\r\nHost: localhost\r\nContent-Length: 100\r\n\r\nabcd",
                )
                .expect("the write failed");
            // Long enough for the isolate to have started and parked, so the
            // disconnect lands on a request that is genuinely still running.
            std::thread::sleep(Duration::from_millis(50));
            drop(socket);
        });

        // The neighbour's own client, which sends everything it promised: what
        // the core that lost a peer does must be its own and not the fleet's.
        let mut neighbour =
            TcpStream::connect(staying_addr).expect("the loopback refused a connection");
        neighbour
            .set_read_timeout(Some(CLIENT_PATIENCE))
            .expect("the socket refused a read timeout");
        neighbour
            .write_all(
                b"POST /done HTTP/1.1\r\nHost: localhost\r\nContent-Length: 4\r\nConnection: close\
                  \r\n\r\nabcd",
            )
            .expect("the write failed");
        let mut answer = String::new();
        neighbour
            .read_to_string(&mut answer)
            .expect("the response could not be read");

        gone.join().expect("the client thread panicked");
        left.join().expect("the served core panicked");
        kept.join().expect("the neighbouring core panicked");

        let events = log.lock().expect("a poisoned log").clone();
        let at = |what: &str| events.iter().position(|event| event == what);
        assert!(
            at("the served core: the isolate started").is_some(),
            "the isolate never ran, so there was nothing to leave behind: {events:?}"
        );
        assert!(
            at("the served core: the body failed").is_some()
                && at("the served core: the body ended").is_none(),
            "a request parked on a body its peer never sent was not told the peer had gone, so \
             this case never tested a disconnect: {events:?}"
        );
        let released = at("the served core: the isolate was released")
            .expect("the isolate was still held when its core was torn down");
        let returned = at("the served core: the accept loop returned")
            .expect("the accept loop never returned, so the connection outlived its client");
        assert!(
            released < returned,
            "the isolate outlived the connection that owned it: {events:?}"
        );

        // The neighbour: its own request ran to its own end, and the release it
        // filed is its own connection's rather than a share of the teardown next
        // door.
        assert!(
            answer.starts_with("HTTP/1.1 200 OK\r\n"),
            "the neighbouring core did not answer its own client: {answer}"
        );
        assert!(
            at("the neighbouring core: the body ended").is_some(),
            "the neighbouring core's request was failed by a peer that was not its own: {events:?}"
        );
        let next_door = at("the neighbouring core: the isolate was released")
            .expect("the neighbouring core was still holding an isolate when it was torn down");
        let stopped = at("the neighbouring core: the accept loop returned")
            .expect("the neighbouring core never returned");
        assert!(
            next_door < stopped,
            "the neighbouring core let go of its isolate only when it stopped: {events:?}"
        );
    }

    /// What one served request did after its client went away, in order.
    type Notes = Arc<std::sync::Mutex<Vec<String>>>;

    fn noted(notes: &Notes, what: impl Into<String>) {
        notes.lock().expect("a poisoned log").push(what.into());
    }

    /// Serves one `POST` with an empty body on `written`'s tree, whose client
    /// closes while the program runs `then`, and answers with what happened.
    ///
    /// The body is empty because `hyper` watches for the close only once the
    /// whole request is read: that EOF ends the connection future and drops
    /// the request's [`Peer`]. `then` is the work a request goes on with after
    /// its client left (`rule:http-server/a-request-outlives-a-client-that-goes-away`).
    /// The last note is always `the accept loop returned`, so a case reads an
    /// ordering against the connection's end and not only a final state.
    fn after_a_disconnect(
        written: &str,
        then: impl FnOnce(&mut Ctx, &Notes, &Admission) + 'static,
    ) -> Vec<String> {
        after_a_disconnect_under(written, Waits::default(), &Draining::detached(), then)
    }

    /// [`after_a_disconnect`] under `waits` and `draining`, for a case that
    /// begins a drain or reads its period.
    fn after_a_disconnect_under(
        written: &str,
        waits: Waits,
        draining: &Draining,
        then: impl FnOnce(&mut Ctx, &Notes, &Admission) + 'static,
    ) -> Vec<String> {
        let draining = draining.clone();
        let serving = booted_on(written);
        let admission = Arc::clone(&serving.admission);
        let notes: Notes = Arc::new(std::sync::Mutex::new(Vec::new()));
        type Then = Box<dyn FnOnce(&mut Ctx, &Notes, &Admission)>;
        let then: RefCell<Option<Then>> = RefCell::new(Some(Box::new(then)));
        let handler = {
            let notes = Arc::clone(&notes);
            Rc::new(move |request: Request<Incoming>, _origin: Origin| {
                let mut inbound = nvs_runtime::Inbound::new(
                    request.method().as_str(),
                    request.uri().path(),
                    request.uri().query().unwrap_or(""),
                );
                let (head, incoming) = request.into_parts();
                let supply = match crate::body::of(&head.headers, incoming) {
                    crate::body::Arrived::Streaming(supply, pull) => {
                        inbound.set_body(pull);
                        Some(supply)
                    }
                    crate::body::Arrived::Absent | crate::body::Arrived::TooLarge => None,
                };
                let notes = Arc::clone(&notes);
                let admission = Arc::clone(&admission);
                let then = then.borrow_mut().take().expect("one request per case");
                // Noted when the program's captures are dropped, which is its
                // end and its teardown alike: a cancelled task runs no more of
                // its own code.
                struct Stopped(Notes);
                impl Drop for Stopped {
                    fn drop(&mut self) {
                        noted(&self.0, "the request stopped");
                    }
                }
                let stopped = Stopped(Arc::clone(&notes));
                let program: Program = Box::new(move |child: &mut Ctx, _args| {
                    let _stopped = &stopped;
                    then(child, &notes, &admission);
                    Value::null()
                });
                Reply::Run(
                    Isolate::new(program, Value::null(), Output::Capture).answering(inbound),
                    supply,
                )
            })
        };

        let mut listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");
        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .write_all(
                    b"POST /orders HTTP/1.1\r\nHost: example.com\r\nContent-Length: 0\r\n\r\n",
                )
                .expect("the write failed");
            std::thread::sleep(Duration::from_millis(30));
            drop(socket);
        });

        let ended = Arc::clone(&notes);
        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
            serve_on_this_core(
                &mut listener,
                &handler,
                waits,
                &serving,
                &draining,
                |_note| {},
                || ControlFlow::Break(()),
            )
            .expect("the accept loop failed");
            noted(&ended, "the accept loop returned");
        });
        nvs_host::run_until_idle(&mut sched).expect("the loop failed");
        client.join().expect("the client thread panicked");
        notes.lock().expect("a poisoned log").clone()
    }

    /// Two writes with a wait between them, the shape of a script that saves
    /// an order and then its items. A cancelled task stops at the wait.
    fn two_writes(_ctx: &mut Ctx, notes: &Notes, _admission: &Admission) {
        noted(notes, "the first write");
        if matches!(
            nvs_host::sleep(Duration::from_millis(100)),
            Woken::Cancelled
        ) {
            noted(notes, "cancelled");
            return;
        }
        noted(notes, "the second write");
    }

    /// `rule:http-server/a-request-outlives-a-client-that-goes-away`'s
    /// default: a request whose client left runs to its end, and the
    /// connection's task does not return before it has.
    // covers: tools:server/when-the-client-goes-away
    #[test]
    fn a_disconnected_request_finishes_both_of_its_writes() {
        let notes = after_a_disconnect("", two_writes);
        let at = |what: &str| notes.iter().position(|note| note == what);
        let second = at("the second write")
            .unwrap_or_else(|| panic!("the request stopped between its two writes: {notes:?}"));
        let returned = at("the accept loop returned").expect("the accept loop never returned");
        assert!(
            second < returned,
            "the connection ended before the request it was serving: {notes:?}"
        );
    }

    /// A method listed in `cancel_on_disconnect` is cancelled at the drop, and
    /// the cancellation still waits for the request to stop.
    // covers: tools:server/when-the-client-goes-away
    #[test]
    fn a_disconnected_request_of_a_listed_method_is_cancelled() {
        let notes = after_a_disconnect("[limits]\ncancel_on_disconnect = [\"POST\"]\n", two_writes);
        assert!(
            !notes.iter().any(|note| note == "the second write"),
            "a listed method ran on after its client left: {notes:?}"
        );
        let at = |what: &str| notes.iter().position(|note| note == what);
        let finished = at("the request stopped").expect("the request never stopped");
        let returned = at("the accept loop returned").expect("the accept loop never returned");
        assert!(
            finished < returned,
            "the connection ended before the request it cancelled had stopped: {notes:?}"
        );
    }

    /// `rule:http-server/admission-is-arithmetic-not-a-number`'s place stays
    /// taken while a request runs on without its client, so a client that
    /// connects and leaves again cannot start unbounded work.
    #[test]
    fn a_disconnected_request_keeps_its_admission_place_until_it_ends() {
        let notes = after_a_disconnect("", |_ctx, notes, admission| {
            let _ = nvs_host::sleep(Duration::from_millis(100));
            noted(notes, format!("in flight: {}", admission.in_flight()));
        });
        assert!(
            notes.iter().any(|note| note == "in flight: 1"),
            "a request whose client left gave its place back before it ended: {notes:?}"
        );
    }

    /// A grace far shorter than the waits below, so a case reads which side of
    /// it the request ended on.
    const SHORT_GRACE: &str = "[limits]\ndisconnect_grace = \"50ms\"\n";

    /// Waits `for_how_long`, and answers whether the wait ran to its end. A
    /// cancelled request is torn down where it parks, so it never sees `false`.
    fn waited(for_how_long: Duration) -> bool {
        !matches!(nvs_host::sleep(for_how_long), Woken::Cancelled)
    }

    /// Moves the request's own `wall_time`, as `Core\Config::set` and
    /// `::restore` do: `None` restores it to what the file says.
    fn wall_time(ctx: &mut Ctx, to: Option<&str>) {
        let config = ctx
            .config_mut()
            .expect("a served request has a configuration");
        match to {
            Some(value) => assert!(config.set("wall_time", value), "`wall_time` was refused"),
            None => config.restore("wall_time"),
        }
        ctx.refresh_limits();
    }

    /// A request with no `wall_time` runs on after its client left for
    /// `disconnect_grace` and no longer, and is then cancelled.
    // covers: tools:server/when-the-client-goes-away
    #[test]
    fn a_disconnected_request_with_no_wall_time_is_cancelled_at_its_grace() {
        let notes = after_a_disconnect(SHORT_GRACE, |_ctx, notes, _admission| {
            if waited(Duration::from_secs(5)) {
                noted(notes, "ran past its grace");
            }
        });
        assert!(
            !notes.iter().any(|note| note == "ran past its grace"),
            "a request with no `wall_time` outlived its grace: {notes:?}"
        );
        let at = |what: &str| notes.iter().position(|note| note == what);
        let stopped = at("the request stopped").expect("the request never stopped");
        let returned = at("the accept loop returned").expect("the accept loop never returned");
        assert!(
            stopped < returned,
            "the grace did not cancel the request before the connection ended: {notes:?}"
        );
    }

    /// A request with a `wall_time` has that one limit, so the grace does not
    /// cut it.
    #[test]
    fn a_disconnected_request_with_a_wall_time_runs_past_the_grace() {
        let written = format!("{SHORT_GRACE}wall_time = \"10s\"\n");
        let notes = after_a_disconnect(&written, |_ctx, notes, _admission| {
            if waited(Duration::from_millis(300)) {
                noted(notes, "ran past its grace");
            }
        });
        assert!(
            notes.iter().any(|note| note == "ran past its grace"),
            "the grace cut a request that has a `wall_time`: {notes:?}"
        );
    }

    /// `wall_time` is a `Runtime` key: a request that clears its own after the
    /// grace has passed is bounded by the grace from then on.
    #[test]
    fn a_disconnected_request_that_clears_its_wall_time_is_cancelled_at_its_grace() {
        let notes = after_a_disconnect(SHORT_GRACE, |ctx, notes, _admission| {
            wall_time(ctx, Some("10s"));
            if !waited(Duration::from_millis(300)) {
                return;
            }
            noted(notes, "ran past its grace");
            wall_time(ctx, None);
            if waited(Duration::from_secs(5)) {
                noted(notes, "ran on with no bound");
            }
        });
        assert!(
            notes.iter().any(|note| note == "ran past its grace"),
            "the grace cut the request while it had a `wall_time`: {notes:?}"
        );
        let at = |what: &str| notes.iter().position(|note| note == what);
        let stopped = at("the request stopped").expect("the request never stopped");
        let returned = at("the accept loop returned").expect("the accept loop never returned");
        assert!(
            !notes.iter().any(|note| note == "ran on with no bound") && stopped < returned,
            "a request that cleared its `wall_time` was not cancelled: {notes:?}"
        );
    }

    /// A drain that begins while a request runs on without its client waits
    /// for it as for an attached one: a request that ends inside the drain
    /// period runs to its end, and one that does not is cut at the period's
    /// end. The second run is the one a drain nobody woke the connection for
    /// would leave running for its whole wait.
    #[test]
    fn the_drain_waits_for_a_disconnected_request() {
        let under_a_drain = |for_how_long: Duration| {
            let draining = Draining::detached();
            let begun = draining.clone();
            let waits = Waits {
                drain: Duration::from_millis(300),
                ..Waits::default()
            };
            after_a_disconnect_under("", waits, &draining, move |_ctx, notes, _admission| {
                if !waited(Duration::from_millis(100)) {
                    return;
                }
                begun.begin();
                if waited(for_how_long) {
                    noted(notes, "ran to its end under the drain");
                }
            })
        };

        let notes = under_a_drain(Duration::from_millis(50));
        let at = |what: &str| notes.iter().position(|note| note == what);
        let ended = at("ran to its end under the drain")
            .unwrap_or_else(|| panic!("the drain cut a request inside its period: {notes:?}"));
        let returned = at("the accept loop returned").expect("the accept loop never returned");
        assert!(
            ended < returned,
            "the connection ended before the request the drain waited for: {notes:?}"
        );

        let notes = under_a_drain(Duration::from_secs(5));
        let at = |what: &str| notes.iter().position(|note| note == what);
        assert!(
            at("ran to its end under the drain").is_none(),
            "a request whose client left outlived the drain period: {notes:?}"
        );
        let stopped = at("the request stopped").expect("the request never stopped");
        let returned = at("the accept loop returned").expect("the accept loop never returned");
        assert!(
            stopped < returned,
            "the drain did not cut the request before the connection ended: {notes:?}"
        );
    }

    /// `rule:concurrency/after-response-outlives-the-connection`'s work runs
    /// for a request whose client left, as for one whose client stayed, and
    /// the end of the connection does not cancel it while it waits.
    #[test]
    fn a_disconnected_request_runs_its_after_response_work() {
        static RAN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        struct WaitsAndRecords;
        impl nvs_runtime::NativeBody for WaitsAndRecords {
            fn run(_ctx: &mut Ctx) {
                if waited(Duration::from_millis(50)) {
                    RAN.store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }
        }

        let notes = after_a_disconnect("", |ctx, notes, _admission| {
            assert_eq!(
                ctx.defer(nvs_runtime::native_closure::<WaitsAndRecords>(), 0),
                Ok(()),
                "the request's queue refused a registration"
            );
            if waited(Duration::from_millis(100)) {
                noted(notes, "the request ended");
            }
        });
        assert!(
            notes.iter().any(|note| note == "the request ended"),
            "the request did not run to its end: {notes:?}"
        );
        assert!(
            RAN.load(std::sync::atomic::Ordering::Relaxed),
            "a request whose client left did not run its after-response work: {notes:?}"
        );
    }

    /// `rule:core-classes/html-later`'s `Core\Response::slotted()` works while
    /// the main script runs and throws `LogicError` once it has ended — from a
    /// `later` slot and from after-response work alike.
    #[test]
    fn slotted_after_the_main_script_ended_throws() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static IN_MAIN: AtomicUsize = AtomicUsize::new(0);
        static REFUSED: AtomicUsize = AtomicUsize::new(0);
        struct AsksForSlotting;
        impl nvs_runtime::NativeBody for AsksForSlotting {
            fn run(ctx: &mut Ctx) {
                if matches!(
                    ctx.make_slotted(),
                    Err(nvs_runtime::Fault::Thrown(
                        nvs_runtime::ThrownClass::Logic,
                        _
                    ))
                ) {
                    REFUSED.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        let listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");
        let handler = Rc::new(|request: Request<Incoming>, _origin: Origin| {
            let inbound = nvs_runtime::Inbound::new(
                request.method().as_str(),
                request.uri().path(),
                request.uri().query().unwrap_or(""),
            );
            let program: Program = Box::new(|ctx: &mut Ctx, _args| {
                if ctx.make_slotted().is_ok() && ctx.is_slotted() {
                    IN_MAIN.fetch_add(1, Ordering::Relaxed);
                }
                let marker = ctx.register_later(
                    nvs_runtime::native_closure::<AsksForSlotting>(),
                    b"",
                    b"",
                    None,
                );
                ctx.write_output(&marker)
                    .expect("a captured body refused a write");
                ctx.defer(nvs_runtime::native_closure::<AsksForSlotting>(), 0)
                    .expect("the request's queue refused a registration");
                Value::null()
            });
            Reply::Run(
                Isolate::new(program, Value::null(), Output::Capture).answering(inbound),
                None,
            )
        });
        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .write_all(b"GET /page HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .expect("the write failed");
            let mut answer = String::new();
            socket
                .read_to_string(&mut answer)
                .expect("the response could not be read");
            answer
        });

        let answer = served_by(listener, &handler, client);
        assert!(
            answer.starts_with("HTTP/1.1 200"),
            "the request did not answer: {answer}"
        );
        assert_eq!(
            IN_MAIN.load(Ordering::Relaxed),
            1,
            "the main script could not make its response slotted"
        );
        assert_eq!(
            REFUSED.load(Ordering::Relaxed),
            2,
            "a slot or after-response work made the response slotted after the main script ended"
        );
    }

    /// A request that carried no body leaves the carrier with none to read —
    /// `Inbound::body` answering `None` is "there was no body", which is the
    /// distinction RFC 9110 § 8.6 draws and what a `Core\Request` member reports
    /// differently from an empty one.
    #[test]
    fn a_request_with_no_body_reaches_the_isolate_carrying_none() {
        let listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .write_all(b"GET /upload HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .expect("the write failed");
            let mut answer = String::new();
            socket
                .read_to_string(&mut answer)
                .expect("the response could not be read");
            answer
        });

        let answer = served_by(listener, &echo_the_body(), client);
        assert!(
            answer.ends_with("no body"),
            "a bodiless request did not reach the program as one: {answer}"
        );
    }

    /// The honest oversized client is refused at the door: a declared length
    /// already over [`crate::body::UPLOAD_TOTAL`] is answered `413` with no
    /// program having been asked for.
    ///
    /// `rule:http-server/request-body-and-upload-total-are-two-caps`'s "before dispatch" is two facts and neither implies the
    /// other, so both are asserted: the peer's status, and a counter only a
    /// program that ran could have moved. A refusal taken *after* dispatch
    /// would answer `413` just the same and would already have allocated the
    /// isolate the ADR says the client never reaches.
    ///
    /// The request carries no body bytes at all, which is the case rather than
    /// a shortcut for a 256 MiB write: the refusal is read off the header, so a
    /// server that waited for what it was promised hangs here instead of
    /// answering.
    #[test]
    fn an_upload_total_over_the_cap_is_refused_before_dispatch() {
        let listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let declared = crate::body::UPLOAD_TOTAL + 1;
        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .write_all(
                    format!(
                        "POST /upload HTTP/1.1\r\nHost: localhost\r\n\
                         Content-Length: {declared}\r\nConnection: close\r\n\r\n"
                    )
                    .as_bytes(),
                )
                .expect("the write failed");
            let mut answer = String::new();
            socket
                .read_to_string(&mut answer)
                .expect("the response could not be read");
            answer
        });

        // The door `nvs-cli` writes, in the two lines of it this case is about:
        // classify the body first, and only then ask for a program.
        let dispatched = Rc::new(Cell::new(0_usize));
        let handler = {
            let dispatched = Rc::clone(&dispatched);
            Rc::new(move |request: Request<Incoming>, _origin: Origin| {
                let (head, incoming) = request.into_parts();
                match crate::body::of(&head.headers, incoming) {
                    crate::body::Arrived::TooLarge => Reply::too_large(),
                    crate::body::Arrived::Absent | crate::body::Arrived::Streaming(..) => {
                        let dispatched = Rc::clone(&dispatched);
                        let program: Program = Box::new(move |child: &mut Ctx, _args| {
                            dispatched.set(dispatched.get() + 1);
                            child.write_output(b"dispatched").expect("a buffer");
                            Value::null()
                        });
                        Reply::run(Isolate::new(program, Value::null(), Output::Capture))
                    }
                }
            })
        };

        let answer = served_by(listener, &handler, client);
        assert!(
            answer.starts_with("HTTP/1.1 413"),
            "a body declaring more than upload_total was not refused: {answer}"
        );
        assert_eq!(
            dispatched.get(),
            0,
            "the oversized request reached a program before it was refused: {answer}"
        );
    }

    /// A handler whose program reads its body to the end and reports what it
    /// **weighed** rather than what it read: the byte count, the last bytes to
    /// arrive, and the high-water mark of everything this thread held while the
    /// body was crossing.
    ///
    /// Nothing here accumulates the body, and that is the whole difference from
    /// [`echo_the_body`], which appends every chunk to a string: a program
    /// written that way holds the body whole by itself, so the reading it took
    /// would be its own and never the door's.
    ///
    /// `ceiling` is [`Ctx::set_memory_limit`], the way an isolate holding no
    /// configuration gets the `[limits] memory` a configured request reads from
    /// its snapshot — and [`Ctx::memory_breach`] afterwards is the same
    /// arithmetic `nvs_runtime`'s safepoint poll makes, which a hand-written
    /// program has no safepoint to make for it.
    fn weigh_the_body(ceiling: usize) -> Rc<impl Fn(Request<Incoming>, Origin) -> Reply> {
        Rc::new(move |request: Request<Incoming>, _origin: Origin| {
            let mut inbound = nvs_runtime::Inbound::new(
                request.method().as_str(),
                request.uri().path(),
                request.uri().query().unwrap_or(""),
            );
            let (head, incoming) = request.into_parts();
            let supply = match crate::body::of(&head.headers, incoming) {
                crate::body::Arrived::Streaming(supply, pull) => {
                    inbound.set_body(pull);
                    Some(supply)
                }
                crate::body::Arrived::Absent | crate::body::Arrived::TooLarge => None,
            };
            let program: Program = Box::new(move |child: &mut Ctx, _args| {
                /// The last bytes kept — room for the closing boundary and no
                /// more, because a program holding the tail of a body this size
                /// would be answering its own question.
                const TAIL: usize = 64;

                child.set_memory_limit(ceiling);
                let base = nvs_runtime::budget::live_bytes();
                let mut peak = base;
                let mut carried = 0_usize;
                let mut tail: Vec<u8> = Vec::new();
                let mut failed = None;
                {
                    let inbound = child
                        .inbound_mut()
                        .expect("the isolate ran with no request in front of it");
                    let body = inbound.body().expect("the request carried no body");
                    loop {
                        match body.next_chunk() {
                            Ok(Some(chunk)) => {
                                carried += chunk.len();
                                tail.extend_from_slice(chunk);
                                if tail.len() > TAIL {
                                    tail.drain(..tail.len() - TAIL);
                                }
                                // Sampled here rather than after the loop: what
                                // the door holds is at its highest *during* the
                                // crossing, and a reading taken once the body
                                // has ended is a reading of nothing.
                                peak = peak.max(nvs_runtime::budget::live_bytes());
                            }
                            Ok(None) => break,
                            Err(message) => {
                                failed = Some(message.to_string());
                                break;
                            }
                        }
                    }
                }
                // A breach is a `FATAL` and never a throw — `rule:errors/on-limit`, which
                // `Ctx::memory_breach`'s own doc names — so the other arm is
                // here to be exhaustive rather than because it can happen.
                let breach = match (failed, child.memory_breach()) {
                    (Some(message), _) => format!("failed: {message}"),
                    (None, Some(nvs_runtime::Fault::Fatal(message))) => message.to_string(),
                    (None, Some(other)) => format!("{other:?}"),
                    (None, None) => "none".to_owned(),
                };
                // `tail` last, because it is the only field carrying bytes the
                // client chose: everything a case parses is left of it.
                let said = format!(
                    "carried={carried} held={held} breach={breach} tail={}",
                    String::from_utf8_lossy(&tail),
                    held = peak - base,
                );
                child.write_output(said.as_bytes()).expect("a buffer");
                Value::null()
            });
            Reply::Run(
                Isolate::new(program, Value::null(), Output::Capture).answering(inbound),
                supply,
            )
        })
    }

    /// `rule:http-server/an-upload-is-received-only-through-files`'s
    /// load-bearing case, in M7's own words: a multipart body far larger than
    /// any in-memory bound is received **in full at bounded resident memory**,
    /// asserted against a high-water mark rather than against the request
    /// merely succeeding.
    ///
    /// **The high-water mark is the case.** A door that read the whole 32 MiB
    /// into one buffer and handed it over answers `200` with the same byte
    /// count, so every assertion but the peak is one a buffering server passes.
    /// What is sampled is [`nvs_runtime::budget::live_bytes`] *inside the pull
    /// loop* — the same reading
    /// [`the_upgrading_requests_arena_is_released_while_the_connection_is_open`]
    /// takes, which is per thread and so counts `hyper`'s own read buffer
    /// beside the isolate's arena. Both halves of
    /// [`crate::body`]'s cell are on this one thread by construction, so
    /// nothing the crossing holds is outside the number.
    ///
    /// **The `[limits]` ceiling is the second half, and it is the one an
    /// operator writes.** The request is held to 4 MiB and receives 32 MiB
    /// inside it: [`Ctx::memory_breach`] is what a configured request's
    /// safepoint poll would have raised, and it stays `None`.
    ///
    /// **The body is a real multipart message and the parse is deliberately not
    /// here.** `nvs-stdlib` is not a dependency of this crate, so
    /// `Core\Request::files()` cannot be reached from a `-p nvs-server` test at
    /// all; what this crate owns is the crossing, and what the shape buys is
    /// that the bytes asserted at the end are the closing boundary rather than
    /// a hundredth megabyte of filler. The parse's own bounded-memory case
    /// belongs beside `nvs_stdlib::multipart`.
    #[test]
    fn a_multipart_body_far_over_the_memory_bound_is_received_at_bounded_resident_memory() {
        /// The file part's payload — far past both the ceiling below and
        /// anything `hyper` buffers, so a door that held the body whole crosses
        /// the bound by an order of magnitude rather than by a margin.
        const PAYLOAD: usize = 32 << 20;
        /// One `write_all` of the client's. Nothing depends on the size: the
        /// door reads what the wire gives it, and this only keeps the client
        /// from building 32 MiB of its own to send.
        const BLOCK: usize = 64 * 1024;
        /// The request's `[limits] memory`, roomy in absolute terms and a
        /// thirty-second of the body.
        const CEILING: usize = 4 << 20;
        /// What the crossing may hold at its peak. Measured at **90 KiB** here
        /// — a 371st of the body — and the bound is far above that on purpose:
        /// `hyper`'s h1 read buffer is the largest thing inside the reading and
        /// its own ceiling is 400 KiB, so a platform whose reads fill it, and a
        /// realloc holding both halves while it grows, must stay inside a bound
        /// this case is not otherwise about. It is still a sixteenth of the
        /// body, which is the claim.
        const BOUND: isize = 2 << 20;

        let boundary = "novis-multipart-boundary";
        let opening = format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; \
             filename=\"big.bin\"\r\nContent-Type: application/octet-stream\r\n\r\n"
        );
        let closing = format!("\r\n--{boundary}--\r\n");
        let length = opening.len() + PAYLOAD + closing.len();

        let listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let sent = closing.clone();
        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .write_all(
                    format!(
                        "POST /upload HTTP/1.1\r\nHost: localhost\r\n\
                         Content-Type: multipart/form-data; boundary={boundary}\r\n\
                         Content-Length: {length}\r\nConnection: close\r\n\r\n{opening}"
                    )
                    .as_bytes(),
                )
                .expect("the write failed");
            let block = vec![b'n'; BLOCK];
            for _ in 0..PAYLOAD / BLOCK {
                socket.write_all(&block).expect("a payload write failed");
            }
            socket.write_all(sent.as_bytes()).expect("the tail failed");
            let mut answer = String::new();
            socket
                .read_to_string(&mut answer)
                .expect("the response could not be read");
            answer
        });

        let handler = weigh_the_body(CEILING);
        let answer = served_by(listener, &handler, client);

        assert!(
            answer.contains(&format!("carried={length}")),
            "the body did not reach the program in full: {} bytes were sent",
            length
        );
        assert!(
            answer.ends_with(&closing),
            "the bytes the program saw last were not the closing boundary: {answer}"
        );
        assert!(
            answer.contains("breach=none"),
            "receiving the body crossed the request's own memory ceiling: {answer}"
        );

        let held: isize = answer
            .split("held=")
            .nth(1)
            .and_then(|rest| rest.split(' ').next())
            .expect("the program reported no high-water mark")
            .parse()
            .expect("the high-water mark was not a number");
        assert!(
            held <= BOUND,
            "the crossing held {held} bytes at its peak while carrying {length}, \
             which is past the {BOUND} this body is streamed inside"
        );
    }

    /// One row of M7's state-bleed suite: a kind of state a run can leave
    /// behind, and what the run after it can see of it.
    ///
    /// The two halves are `fn` pointers rather than closures on purpose — a row
    /// that captured anything would be able to carry the state itself, and the
    /// suite would then be asserting its own fixture.
    struct Bleed {
        /// What is being asked about, and the left half of the answer line.
        what: &'static str,
        /// Run in the first run — the one that must leave no trace.
        plant: fn(&mut Ctx),
        /// Run in the second. [`NOTHING`] is the only passing answer, and
        /// anything else is reported verbatim because *what* bled says more
        /// than a bool does.
        probe: fn(&mut Ctx) -> String,
    }

    /// The marker the suite plants, distinctive enough that finding it anywhere
    /// in the second run's answer is itself the failure.
    const BLED: &str = "bled-c0ffee";

    /// What the planting run leaves in its own response buffer — [`BLED`] plus
    /// a suffix, so a client reading until the first run's *body* cannot stop
    /// on the header carrying the same marker.
    const BLED_OUTPUT: &str = "bled-c0ffee-output";

    /// The only passing answer, for every row and both boundaries.
    const NOTHING: &str = "nothing";

    /// M7's state-bleed rows: the per-request state a program can reach, one
    /// row per kind, each planted by a run that then ends.
    ///
    /// **Memory is deliberately not a row here.** A run's arena is the one
    /// piece of state a child isolate shares by design —
    /// `rule:security/isolate-shares-nothing`'s "spends its
    /// parent's budget" — so a row asserting a fresh reading would fail the
    /// isolate arm for obeying the ADR. What must not survive is the *finished*
    /// run's arena, and that is
    /// [`the_upgrading_requests_arena_is_released_while_the_connection_is_open`]
    /// beside
    /// [`a_multipart_body_far_over_the_memory_bound_is_received_at_bounded_resident_memory`].
    const SUITE: &[Bleed] = &[
        Bleed {
            what: "the request carrier",
            // Planted by the door rather than by the run: a program cannot put
            // a query or a header on its own carrier, which is exactly what
            // makes this the row a shared `Inbound` would fail.
            plant: |_run| {},
            probe: |run| {
                let Some(inbound) = run.inbound() else {
                    return NOTHING.to_owned();
                };
                let mut seen = String::new();
                if inbound.path().contains(BLED) {
                    seen.push_str(" path");
                }
                if inbound.query().contains(BLED) {
                    seen.push_str(" query");
                }
                if inbound
                    .headers()
                    .any(|(_name, value)| String::from_utf8_lossy(value).contains(BLED))
                {
                    seen.push_str(" header");
                }
                if seen.is_empty() {
                    NOTHING.to_owned()
                } else {
                    format!("the first run's{seen}")
                }
            },
        },
        Bleed {
            what: "the request body",
            // Read to the end rather than left: `serve_connection`'s docs say a
            // body no program reads is never drained, so a planting run that
            // ignored it would leave the *wire* dirty and the next request on
            // this connection would never be framed at all — a different
            // failure wearing this one's clothes.
            plant: |run| {
                let Some(inbound) = run.inbound_mut() else {
                    return;
                };
                let Some(body) = inbound.body() else {
                    return;
                };
                while let Ok(Some(_chunk)) = body.next_chunk() {}
            },
            probe: |run| {
                let Some(inbound) = run.inbound_mut() else {
                    return NOTHING.to_owned();
                };
                if !inbound.has_body() {
                    return NOTHING.to_owned();
                }
                let mut carried = 0_usize;
                if let Some(body) = inbound.body() {
                    while let Ok(Some(chunk)) = body.next_chunk() {
                        carried += chunk.len();
                    }
                }
                format!("a carrier holding one, with {carried} byte(s) still to read")
            },
        },
        Bleed {
            what: "the declared response head",
            plant: |run| {
                run.declare_status(418);
                run.declare_content_type("text/x-bled");
                run.declare_header("x-bled", BLED);
            },
            probe: |run| {
                let mut seen = String::new();
                if let Some(status) = run.take_status() {
                    seen.push_str(&format!(" status {status}"));
                }
                if let Some(media) = run.take_content_type() {
                    seen.push_str(&format!(" type {media}"));
                }
                // Counted rather than printed: `DeclaredHeader` is the door's
                // shape and this row is about how many crossed, not which.
                match run.take_headers().len() {
                    0 => {}
                    headers => seen.push_str(&format!(" {headers} header(s)")),
                }
                if seen.is_empty() {
                    NOTHING.to_owned()
                } else {
                    format!("the first run's{seen}")
                }
            },
        },
        Bleed {
            what: "the response buffer",
            plant: |run| {
                run.write_output(BLED_OUTPUT.as_bytes()).expect("a buffer");
            },
            probe: |run| match run.take_buffered_output() {
                Some(bytes) if !bytes.is_empty() => format!("{} byte(s) of it", bytes.len()),
                _ => NOTHING.to_owned(),
            },
        },
    ];

    /// Every row's `plant`, in order — the whole of what the first run does.
    fn plant_the_suite(run: &mut Ctx) {
        for row in SUITE {
            (row.plant)(run);
        }
    }

    /// Every row's `probe`, one answer line each, in the same order.
    fn probe_the_suite(run: &mut Ctx) -> String {
        let mut said = String::new();
        for row in SUITE {
            let answer = (row.probe)(run);
            said.push_str(&format!("{}: {answer}\n", row.what));
        }
        said
    }

    /// The request the planting run is given: the marker in the query, in a
    /// header and in a body, which is the state no program could plant for
    /// itself. Shared by both arms, so the two differ only in what runs second.
    fn planting_request() -> String {
        format!(
            "POST /bleed?leak={BLED} HTTP/1.1\r\nHost: localhost\r\nX-Leak: {BLED}\r\n\
             Content-Length: {}\r\n\r\n{BLED}",
            BLED.len()
        )
    }

    /// The carrier and the connection's half of the body, built the one way
    /// every handler in this module builds them.
    fn carrying(request: Request<Incoming>) -> (nvs_runtime::Inbound, Option<Supply>) {
        let mut inbound = nvs_runtime::Inbound::new(
            request.method().as_str(),
            request.uri().path(),
            request.uri().query().unwrap_or(""),
        );
        let (head, incoming) = request.into_parts();
        for (name, value) in &head.headers {
            inbound.push_header(name.as_str(), value.as_bytes());
        }
        let supply = match crate::body::of(&head.headers, incoming) {
            crate::body::Arrived::Streaming(supply, pull) => {
                inbound.set_body(pull);
                Some(supply)
            }
            crate::body::Arrived::Absent | crate::body::Arrived::TooLarge => None,
        };
        (inbound, supply)
    }

    /// The suite across a **request** boundary: the planting run is one
    /// request, the probing run is the next one down the same connection.
    ///
    /// Returns the second response, whose body is the answer lines.
    fn across_a_request_boundary() -> String {
        let listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .write_all(planting_request().as_bytes())
                .expect("the write failed");
            // Until the planting run's own body, so nothing of the first
            // response is still on the socket when the second is asked for.
            let mut planted = String::new();
            read_until(&mut socket, BLED_OUTPUT, &mut planted);
            socket
                .write_all(b"GET /clean HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .expect("the second write failed");
            let mut answer = String::new();
            socket
                .read_to_string(&mut answer)
                .expect("the response could not be read");
            end_the_loop(addr);
            answer
        });

        let served = Rc::new(Cell::new(0_usize));
        let handler = Rc::new(move |request: Request<Incoming>, _origin: Origin| {
            let (inbound, supply) = carrying(request);
            let first = served.get() == 0;
            served.set(served.get() + 1);
            let program: Program = Box::new(move |child: &mut Ctx, _args| {
                if first {
                    plant_the_suite(child);
                } else {
                    let said = probe_the_suite(child);
                    child.write_output(said.as_bytes()).expect("a buffer");
                }
                Value::null()
            });
            Reply::Run(
                Isolate::new(program, Value::null(), Output::Capture).answering(inbound),
                supply,
            )
        });
        served_across_requests(listener, &handler, client, wide_open())
    }

    /// The same suite across an **isolate** boundary: the planting run is the
    /// request, and the probing run is a child [`Isolate`] it starts before it
    /// ends — `rule:security/isolate-shares-nothing`'s
    /// `spawn script`, which is the same type the door built the request from.
    ///
    /// Returns the response, whose body is the child's answer lines and
    /// nothing else.
    fn across_an_isolate_boundary() -> String {
        let listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            let planting = planting_request().replace("\r\n\r\n", "\r\nConnection: close\r\n\r\n");
            socket
                .write_all(planting.as_bytes())
                .expect("the write failed");
            let mut answer = String::new();
            socket
                .read_to_string(&mut answer)
                .expect("the response could not be read");
            answer
        });

        let handler = Rc::new(|request: Request<Incoming>, _origin: Origin| {
            let (inbound, supply) = carrying(request);
            let program: Program = Box::new(move |child: &mut Ctx, _args| {
                plant_the_suite(child);
                let probing: Program = Box::new(|inner: &mut Ctx, _args| {
                    let said = probe_the_suite(inner);
                    inner.write_output(said.as_bytes()).expect("a buffer");
                    Value::null()
                });
                let done = Isolate::new(probing, Value::null(), Output::Capture)
                    .run(child)
                    .expect("the child isolate refused an argument it was not given");
                // The bytes this run planted are its own and were never a
                // bleed. Dropping them is what leaves both arms' responses
                // carrying the answer lines alone.
                let _planted = child.take_buffered_output();
                child.write_output(&done.output).expect("a buffer");
                Value::null()
            });
            Reply::Run(
                Isolate::new(program, Value::null(), Output::Capture).answering(inbound),
                supply,
            )
        });
        served_by(listener, &handler, client)
    }

    /// One worker of the core boundary: this core's listener, the one
    /// connection it serves, and the half of the suite the caller names.
    ///
    /// The handler is built inside the closure because a core's handler is an
    /// [`Rc`] and never crosses a thread — which is the shape under test, not
    /// an accommodation: what the cores share is compiled program text and
    /// nothing else
    /// (`rule:http-server/the-accept-fan-out-is-one-worker-per-core`).
    fn one_bleeding_core(
        listener: std::net::TcpListener,
        planting: bool,
    ) -> impl FnOnce(&mut nvs_host::Scheduler) + Send + 'static {
        move |sched| {
            let mut listener =
                NvsListener::from_std(listener).expect("the OS refused a non-blocking listener");
            let _installed = nvs_host::reactor::install(
                nvs_host::Reactor::new().expect("the OS refused a poll"),
            );
            let handler = Rc::new(move |request: Request<Incoming>, _origin: Origin| {
                let (inbound, supply) = carrying(request);
                let program: Program = Box::new(move |child: &mut Ctx, _args| {
                    if planting {
                        plant_the_suite(child);
                    } else {
                        let said = probe_the_suite(child);
                        child.write_output(said.as_bytes()).expect("a buffer");
                    }
                    Value::null()
                });
                Reply::Run(
                    Isolate::new(program, Value::null(), Output::Capture).answering(inbound),
                    supply,
                )
            });
            sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
                serve_on_this_core(
                    &mut listener,
                    &handler,
                    Waits::default(),
                    &wide_open(),
                    // This core's own drain and not the process's, which is
                    // one-way for the whole binary.
                    &Draining::detached(),
                    |_note| {},
                    || ControlFlow::Break(()),
                )
                .expect("the accept loop failed");
            });
            run_the_core(sched);
        }
    }

    /// The same suite across a **core** boundary: the planting run is a request
    /// one worker serves, and the probing run is a request the *next* worker
    /// serves, on another pinned thread with a scheduler and a run queue of its
    /// own.
    ///
    /// Two workers on two listeners rather than two connections to one, because
    /// which core the OS hands a connection to is the OS's choice — the
    /// fan-out's own property, and the wrong thing for a case about what
    /// crosses a core to rest on. The planting worker is **joined** before the
    /// probing request is written, so the first run has demonstrably ended and
    /// what it left behind is there to be found.
    ///
    /// Returns the second response, or [`None`] on a host that enumerates no
    /// CPU: such a host is served from the boot thread, so there is no second
    /// core there for the state to be found on.
    fn across_a_core_boundary() -> Option<String> {
        let cpus = nvs_host::cpus();
        let first_cpu = cpus.first().copied()?;
        // One CPU is a fleet of two workers sharing it, which is `nvs serve`'s
        // own answer when a written count is above this machine's parallelism.
        let second_cpu = cpus.get(1).copied().unwrap_or(first_cpu);

        let planting_socket =
            std::net::TcpListener::bind("127.0.0.1:0").expect("the OS refused a port");
        let planting_addr = planting_socket
            .local_addr()
            .expect("a bound listener had no address");
        let probing_socket =
            std::net::TcpListener::bind("127.0.0.1:0").expect("the OS refused a port");
        let probing_addr = probing_socket
            .local_addr()
            .expect("a bound listener had no address");

        let planting_worker =
            nvs_host::Worker::spawn(first_cpu, one_bleeding_core(planting_socket, true))
                .expect("the OS refused a worker thread");
        let probing_worker =
            nvs_host::Worker::spawn(second_cpu, one_bleeding_core(probing_socket, false))
                .expect("the OS refused a worker thread");

        let mut planting =
            TcpStream::connect(planting_addr).expect("the loopback refused a connection");
        planting
            .set_read_timeout(Some(CLIENT_PATIENCE))
            .expect("the socket refused a read timeout");
        planting
            .write_all(
                // Closed after the one request, so this core's tally reaches
                // zero and its worker can be joined below.
                planting_request()
                    .replace("\r\n\r\n", "\r\nConnection: close\r\n\r\n")
                    .as_bytes(),
            )
            .expect("the write failed");
        let mut planted = String::new();
        planting
            .read_to_string(&mut planted)
            .expect("the planting response could not be read");
        assert!(
            planted.contains(BLED_OUTPUT),
            "the planting run left nothing behind to look for: {planted}"
        );
        drop(planting);
        planting_worker.join().expect("the planting core panicked");

        let mut probing =
            TcpStream::connect(probing_addr).expect("the loopback refused a connection");
        probing
            .set_read_timeout(Some(CLIENT_PATIENCE))
            .expect("the socket refused a read timeout");
        probing
            .write_all(b"GET /clean HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .expect("the second write failed");
        let mut answer = String::new();
        probing
            .read_to_string(&mut answer)
            .expect("the response could not be read");
        drop(probing);
        probing_worker.join().expect("the probing core panicked");
        Some(answer)
    }

    /// The answer lines of a response, which is its body: one line per row.
    fn answer_lines(answer: &str) -> &str {
        answer
            .split_once("\r\n\r\n")
            .map(|(_head, body)| body)
            .unwrap_or_else(|| panic!("the response had no body at all: {answer}"))
    }

    /// M7's acceptance paragraph, its second clause: **a state-bleed suite
    /// proves nothing leaks between requests, and the same suite runs across an
    /// isolate boundary.**
    ///
    /// **It is one suite run more than once, and the plan says so in the same
    /// breath**: "which the shared `Isolate` makes a parameterisation rather
    /// than a second suite". [`SUITE`] is the rows; an arm differs only in what
    /// the second run *is* — the next request on the connection, a child
    /// isolate the request starts before it ends,
    /// [`the_state_bleed_suite_passes_across_a_core_boundary`]'s request on
    /// another worker, or
    /// [`the_state_bleed_suite_passes_across_a_connection_boundary`]'s isolate
    /// an upgrade opens once the request has ended. Every run is the same
    /// [`Isolate`] type, which is the
    /// property being spent: a second isolation path would make one of these
    /// arms say nothing about the others.
    ///
    /// **Every row answers a string rather than a bool**, so a failure names
    /// what crossed; [`nothing_bled`] is what every arm's answer is held to.
    ///
    /// **The first run's marker is on the wire, not in the fixture.** Its
    /// query, its header and its body all carry [`BLED`], which is state the
    /// door builds and hands over; a run that could only plant what a program
    /// can reach would leave the carrier — the thing a request-scoped design
    /// is most likely to share — untested.
    #[test]
    fn the_state_bleed_suite_passes_within_a_request_and_across_an_isolate_boundary() {
        for (boundary, answer) in [
            (
                "the next request on the connection",
                across_a_request_boundary(),
            ),
            (
                "a child isolate inside the request",
                across_an_isolate_boundary(),
            ),
        ] {
            nothing_bled(boundary, answer_lines(&answer));
        }
    }

    /// The same suite, run where the second request lands on **another core**:
    /// a worker of its own, pinned to its own CPU, with its own scheduler, run
    /// queue and handler. The rows are unchanged, which is the point — a third
    /// boundary is a parameter of the one suite and not a suite of its own.
    ///
    /// **A core is the boundary a shared `static` crosses that the other two do
    /// not.** State a request leaves in a thread-local is invisible to the next
    /// core by construction, so the arms above would pass over it; what fails
    /// here is state parked somewhere the whole process reaches, which is
    /// exactly what `rule:http-server/the-accept-fan-out-is-one-worker-per-core`'s
    /// "what the cores share is compiled program text and nothing else" forbids
    /// and what `rule:security/isolate-shares-nothing` makes a run's own.
    #[test]
    fn the_state_bleed_suite_passes_across_a_core_boundary() {
        let Some(answer) = across_a_core_boundary() else {
            return;
        };
        nothing_bled("a request on another core", answer_lines(&answer));
    }

    /// The same suite, run where the second run is the **connection isolate**
    /// an upgrade opens: the upgrading request plants every row and fills
    /// § 1's slot, and the isolate in the slot probes them once that request
    /// has ended (`rule:concurrency/a-connection-is-a-root-isolate`).
    ///
    /// The request's marker rides on its query, because an upgradable request
    /// carries no body. The probing run writes nowhere a peer reads: the `101`
    /// is the connection's last response, so its answer lines come back
    /// through `said` and not off the wire.
    #[test]
    fn the_state_bleed_suite_passes_across_a_connection_boundary() {
        assert!(
            BLEEDING_UPGRADE.ends_with(BLED),
            "the upgrading request's query no longer carries the marker"
        );
        let said = Rc::new(RefCell::new(String::new()));
        let handler_said = Rc::clone(&said);
        let seen = upgrade_once(
            move || {
                Rc::new(move |request: Request<Incoming>, _origin: Origin| {
                    let (inbound, supply) = carrying(request);
                    let said = Rc::clone(&handler_said);
                    let program: Program = Box::new(move |child: &mut Ctx, _args| {
                        plant_the_suite(child);
                        let probing: Program = Box::new(move |conn: &mut Ctx, _args| {
                            said.borrow_mut().push_str(&probe_the_suite(conn));
                            Value::null()
                        });
                        assert!(
                            Door::Socket.fill(child, probing),
                            "the upgrading request was offered no slot"
                        );
                        Value::null()
                    });
                    Reply::Run(
                        Isolate::new(program, Value::null(), Output::Capture).answering(inbound),
                        supply,
                    )
                })
            },
            BLEEDING_UPGRADE,
            Door::Socket,
        );
        assert!(
            seen.contains(ACCEPT),
            "the upgrading request was not answered the handshake: {seen}"
        );
        nothing_bled("the connection an upgrade opened", &said.borrow());
    }

    /// The path [`the_state_bleed_suite_passes_across_a_connection_boundary`]
    /// upgrades on, with [`BLED`] in its query.
    const BLEEDING_UPGRADE: &str = "/bleed?leak=bled-c0ffee";

    /// One arm's answer lines, asserted: the row count first, then every row's
    /// own line.
    ///
    /// The count is asserted beside the answers because a row that silently
    /// stopped running would otherwise pass by not contradicting anything — the
    /// row table is the assertion, not the four lines a reader can see. And the
    /// rows that bled are collected rather than asserted one at a time, so a
    /// failure reports every one of them instead of the first in the table.
    fn nothing_bled(boundary: &str, lines: &str) {
        assert_eq!(
            lines.lines().count(),
            SUITE.len(),
            "{boundary}: {} rows ran, and the suite has {}: {lines}",
            lines.lines().count(),
            SUITE.len()
        );
        let bled: Vec<&str> = SUITE
            .iter()
            .filter(|row| {
                let clean = format!("{}: {NOTHING}", row.what);
                !lines.lines().any(|line| line == clean)
            })
            .map(|row| row.what)
            .collect();
        assert!(
            bled.is_empty(),
            "{boundary}: {} bled across it, and the answers were:\n{lines}",
            bled.join(", ")
        );
    }

    /// A handler whose program says which sink it is writing through, so the
    /// response body *is* the carrier's class name.
    fn echo_the_sink() -> Rc<impl Fn(Request<Incoming>, Origin) -> Reply> {
        Rc::new(|request: Request<Incoming>, _origin: Origin| {
            let inbound = nvs_runtime::Inbound::new(
                request.method().as_str(),
                request.uri().path(),
                request.uri().query().unwrap_or(""),
            );
            let program: Program = Box::new(move |child: &mut Ctx, _args| {
                let said = child.carrier();
                child.write_output(said.as_bytes()).expect("a buffer");
                Value::null()
            });
            Reply::run(Isolate::new(program, Value::null(), Output::Capture).answering(inbound))
        })
    }

    /// `rule:tooling/echo-always-has-a-sink`
    /// 's first row, end to end: inside an HTTP request `echo` writes to the
    /// response body, and what carries those bytes is `Core\Html\Markup`. No
    /// call site on this path says so — the isolate was handed a request, and
    /// that is the whole of what attaches the sink.
    ///
    /// The contrast is the load-bearing half, because § 3's direction is the
    /// fail-closed one. Every other context this path runs is a terminal sink's
    /// and stays one: the coroutine the accept loop runs on and the one a
    /// connection is served on both discard what they are handed, and neither
    /// ever runs a line of a program. `nvs_host::isolate`'s
    /// `an_isolates_echo_takes_the_terminal_sinks_neutralization` is the same
    /// rule at the one line that decides it, over the isolate that is *not*
    /// answering a request.
    #[test]
    fn the_html_sink_is_attached_by_a_request_and_by_nothing_else() {
        let listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .write_all(b"GET /page HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .expect("the write failed");
            let mut answer = String::new();
            socket
                .read_to_string(&mut answer)
                .expect("the response could not be read");
            answer
        });

        let answer = served_by(listener, &echo_the_sink(), client);
        assert!(
            answer.ends_with(nvs_runtime::CARRIER_HTML_MARKUP),
            "the program answering a request was not writing through the HTML sink: {answer}"
        );
        assert_eq!(
            Ctx::new(OutputSink::Sink).carrier(),
            nvs_runtime::CARRIER_CLI_TEXT,
            "the context a connection is served on attached the HTML sink"
        );
        assert_eq!(
            Ctx::buffered().carrier(),
            nvs_runtime::CARRIER_CLI_TEXT,
            "a context that is answering nothing attached the HTML sink"
        );
    }

    /// Runs one accept loop on a scheduler of its own until the client thread
    /// above it is done, and answers what that client read.
    fn served_by<H>(
        listener: NvsListener,
        handler: &Rc<H>,
        client: std::thread::JoinHandle<String>,
    ) -> String
    where
        H: Fn(Request<Incoming>, Origin) -> Reply + 'static,
    {
        served_under(listener, handler, client, wide_open())
    }

    /// [`served_by`], under a policy the case names — the one thing a request
    /// cannot state about itself, and what `rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing`'s case is about.
    fn served_under<H>(
        listener: NvsListener,
        handler: &Rc<H>,
        client: std::thread::JoinHandle<String>,
        serving: Serving,
    ) -> String
    where
        H: Fn(Request<Incoming>, Origin) -> Reply + 'static,
    {
        served_until(
            listener,
            handler,
            client,
            serving,
            || ControlFlow::Break(()),
        )
    }

    /// [`served_under`] for a client that sends a second request on the
    /// connection it already has. The loop's tail begins the drain the moment
    /// it stops accepting, and a drain closes a connection idle between
    /// requests at once (`rule:concurrency/a-drain-closes-a-connection-cleanly`),
    /// so a loop that broke after the one accept would close the connection
    /// between the two requests. The client ends the loop instead, with
    /// [`end_the_loop`] once its last answer is in.
    fn served_across_requests<H>(
        listener: NvsListener,
        handler: &Rc<H>,
        client: std::thread::JoinHandle<String>,
        serving: Serving,
    ) -> String
    where
        H: Fn(Request<Incoming>, Origin) -> Reply + 'static,
    {
        served_until(
            listener,
            handler,
            client,
            serving,
            until_the_loop_is_ended(),
        )
    }

    /// One more connection, made once a client is done with its own and
    /// dropped unread: what un-parks an accept loop asking
    /// [`until_the_loop_is_ended`], which answers `Break` on it. The
    /// connection it spawns reads end of stream and ends by itself.
    fn end_the_loop(addr: std::net::SocketAddr) {
        drop(TcpStream::connect(addr).expect("the loopback refused the closing connection"));
    }

    /// The `keep_serving` of a case whose client calls [`end_the_loop`]:
    /// `Continue` after the client's own connection, `Break` after the one
    /// that ends the loop.
    fn until_the_loop_is_ended() -> impl FnMut() -> ControlFlow<()> {
        let mut accepted = 0_usize;
        move || {
            accepted += 1;
            if accepted < 2 {
                ControlFlow::Continue(())
            } else {
                ControlFlow::Break(())
            }
        }
    }

    /// [`served_under`], with the `keep_serving` the case names.
    fn served_until<H>(
        mut listener: NvsListener,
        handler: &Rc<H>,
        client: std::thread::JoinHandle<String>,
        serving: Serving,
        keep_serving: impl FnMut() -> ControlFlow<()> + 'static,
    ) -> String
    where
        H: Fn(Request<Incoming>, Origin) -> Reply + 'static,
    {
        let handler = Rc::clone(handler);
        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
            serve_on_this_core(
                &mut listener,
                &handler,
                Waits::default(),
                &serving,
                &Draining::detached(),
                |_note| {},
                keep_serving,
            )
            .expect("the accept loop failed");
        });
        nvs_host::run_until_idle(&mut sched).expect("the loop failed");
        client.join().expect("the client thread panicked")
    }

    /// A valve every case but the last one is not about, beside `rule:http-server/secure-headers-with-nothing-written`'s
    /// shipped header set: § 5's own default ceiling with no memory budget to
    /// divide it against, which is what an unconfigured tree resolves to.
    fn wide_open() -> Serving {
        Serving::new(
            Arc::new(Admission::new(&Ceiling::of(&Capacity {
                configured: 10_000,
                per_request: None,
                budget: None,
            }))),
            Arc::new(Secure::default()),
            // `rule:http-server/trusted-proxies-is-empty-and-empty-reads-nothing`'s default: nothing written, so no forwarded header
            // is read and every request's peer is its own client.
            Arc::new(Trusted::none()),
            // `rule:http-server/cors-is-closed-until-origins-are-named`'s default: no origin named, so nothing crosses and no
            // CORS header is emitted at all.
            Arc::new(Cors::default()),
            // The configuration of a host with no configuration file anywhere,
            // which is what `nvs_config::Snapshot`'s own `Default` is for: it
            // grants nothing and states no ceiling, so a case about the framing
            // is not also a case about a tree.
            Arc::default(),
        )
    }

    /// [`wide_open`]'s valve over a tree a boot resolved, for the case that is
    /// about the configuration itself.
    ///
    /// **Both halves of the snapshot are filled from `written`**, because the
    /// two are read by different readers: a `[limits]` key is answered off the
    /// table, and everything typed — capabilities, `[http]`, `[server]` — off
    /// the tree beside it. A snapshot carrying one of them is a fixture that
    /// passes a case about the half it filled and says nothing about the other.
    fn booted_on(written: &str) -> Serving {
        let snapshot = nvs_config::Snapshot {
            config: toml::from_str(written).expect("the tree deserializes"),
            table: written.parse().expect("the tree is TOML"),
            ..Default::default()
        };
        Serving::new(
            Arc::new(Admission::new(&Ceiling::of(&Capacity {
                configured: 10_000,
                per_request: None,
                budget: None,
            }))),
            Arc::new(Secure::default()),
            Arc::new(Trusted::none()),
            Arc::new(Cors::default()),
            Arc::new(snapshot),
        )
    }

    /// `rule:config/the-config-is-an-immutable-snapshot` on the served path: the
    /// tree this instance booted on is what a request reads, and it is written
    /// to the context the answering isolate starts on.
    ///
    /// Asserted through `[limits] cpu_time`, which is the same read
    /// `Ctx::cpu_limit` takes — `nvs_config::Request::get` off the snapshot's
    /// own table — so a request that can print a ceiling here is a request a
    /// watchdog can charge against one
    /// (`rule:http-server/a-wedged-core-is-detected-by-its-deadline`). What the
    /// case fails as is not a wrong number but an absent tree: a context nobody
    /// configured answers `None` to every `[limits]` key and denies every
    /// capability an entry asks for, which is one request served under no
    /// configuration at all rather than under this instance's.
    #[test]
    fn a_served_request_reads_the_tree_the_instance_booted_on() {
        let listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");
        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .expect("the write failed");
            let mut answer = String::new();
            socket
                .read_to_string(&mut answer)
                .expect("the response could not be read");
            answer
        });
        // The request's own reading of its own context, which is the only place
        // the answer can be taken from: what a program holds is the child the
        // isolate built, so a tree that reached the connection and no further
        // prints the same absence as one that never arrived.
        let handler = Rc::new(|_request: Request<Incoming>, _origin: Origin| {
            let program: Program = Box::new(move |child: &mut Ctx, _args| {
                let said = match child.config().and_then(|config| config.get("cpu_time")) {
                    Some(written) => format!("cpu_time={written}"),
                    None => "no tree".to_owned(),
                };
                child.write_output(said.as_bytes()).expect("a buffer");
                Value::null()
            });
            Reply::run(Isolate::new(program, Value::null(), Output::Capture))
        });
        let answer = served_under(
            listener,
            &handler,
            client,
            booted_on("[limits]\ncpu_time = \"7s\"\n"),
        );
        assert!(
            answer.ends_with("cpu_time=7s"),
            "the tree this instance booted on did not reach the request: {answer}"
        );
    }

    /// Reads until `needle` has arrived, so a test can stop in the middle of a
    /// keep-alive connection without parsing the framing itself.
    ///
    /// **It stops on the needle's last byte and takes nothing past it**, which
    /// is why it reads one byte at a time rather than in chunks. What follows a
    /// handshake on these sockets is the server's own framing, and a chunked
    /// read is free to return it in the same buffer as the head — bytes this
    /// helper would then hold, out of reach of the `tungstenite::WebSocket` the
    /// caller builds over the socket next, which starts with an empty buffer of
    /// its own. The peer reads EOF where a close frame was, and calls that a
    /// reset. A loaded machine is what makes it happen: the server writes both
    /// before the client thread is scheduled for its first read, so the two
    /// arrive coalesced. The server side of the same seam answers it by handing
    /// `Framed::new` the bytes it over-read (`crate::socket`'s `Prefixed`);
    /// here there is nowhere to hand them, so none are taken.
    fn read_until(socket: &mut TcpStream, needle: &str, seen: &mut String) {
        let mut buffer = seen.as_bytes().to_vec();
        while !buffer
            .windows(needle.len())
            .any(|at| at == needle.as_bytes())
        {
            let mut byte = [0_u8; 1];
            let read = socket.read(&mut byte).expect("the read failed");
            assert!(
                read > 0,
                "the connection closed before {needle:?}: {}",
                String::from_utf8_lossy(&buffer)
            );
            buffer.push(byte[0]);
        }
        *seen = String::from_utf8_lossy(&buffer).into_owned();
    }

    /// The whole seam end to end: a socket in, `hyper`'s framing over the
    /// parking stream, a coroutine per connection, and an answer out.
    #[test]
    fn one_connection_gets_one_response() {
        let mut listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .write_all(b"GET /hello HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .expect("the write failed");
            let mut answer = String::new();
            socket
                .read_to_string(&mut answer)
                .expect("the response could not be read");
            answer
        });

        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
            serve_on_this_core(
                &mut listener,
                &echo_the_path(),
                Waits::default(),
                &wide_open(),
                &Draining::detached(),
                |_note| {},
                || ControlFlow::Break(()),
            )
            .expect("the accept loop failed");
        });
        nvs_host::run_until_idle(&mut sched).expect("the loop failed");

        let answer = client.join().expect("the client thread panicked");
        assert!(
            answer.starts_with("HTTP/1.1 200 OK\r\n"),
            "the connection did not answer with a response: {answer}"
        );
        assert!(
            answer.to_ascii_lowercase().contains("content-length: 12"),
            "an answer whose length was known was not sent with one: {answer}"
        );
        assert!(
            answer.ends_with("hello /hello"),
            "the response did not carry the handler's body: {answer}"
        );
    }

    /// A handler over a one-row route table, so that the request it answers
    /// carries `rule:routing/matched-once-before-the-handler`'s match — which
    /// is the only thing a declared name can be read off.
    ///
    /// The table is built here rather than handed in because it is the door's
    /// own: [`crate::route::take`] runs where the unit and the request are both
    /// in hand, which in a server is this closure.
    fn echo_under_a_named_route() -> Rc<impl Fn(Request<Incoming>, Origin) -> Reply> {
        let routes = Rc::new(nvs_runtime::routes::Routes::new(vec![
            nvs_runtime::routes::Route::new(
                "Get",
                "/products/{id}",
                Some("products.show".to_owned()),
                "App\\Products::show",
                None,
                vec![nvs_runtime::routes::Capture {
                    name: "id".to_owned(),
                    conv: nvs_runtime::routes::CaptureConv::Uint,
                }],
            ),
        ]));
        Rc::new(move |request: Request<Incoming>, _origin: Origin| {
            let mut inbound = nvs_runtime::Inbound::new(
                request.method().as_str(),
                request.uri().path(),
                request.uri().query().unwrap_or(""),
            );
            crate::route::take(&routes, &mut inbound);
            let program: Program = Box::new(|child: &mut Ctx, _args| {
                child.write_output(b"ok").expect("a buffer");
                Value::null()
            });
            Reply::run(Isolate::new(program, Value::null(), Output::Capture).answering(inbound))
        })
    }

    /// `rule:observability/a-registry-is-per-core-and-nothing-reads-it` on the
    /// served path: the core that accepts builds the registry `[metrics]` asks
    /// for, and the request it answers is counted on that registry under
    /// `rule:observability/route-label-is-the-declared-name`'s declared name.
    ///
    /// **Both series and the label together**, because each is a different
    /// failure: a counter with no histogram beside it is a door that recorded
    /// half of what `rule:observability/default-series` promises, and a count
    /// under the empty label is a door that reached the registry without
    /// reaching the match — the one thing that cannot be re-derived once
    /// `Isolate::start` has taken the carrier.
    ///
    /// The registry is read back on **this** thread and that is the assertion's
    /// own point rather than a convenience of the fixture: `run_until_idle`
    /// drives the core on the caller's thread, so the cell the counts landed in
    /// is the serving core's and no other's.
    #[test]
    fn every_serving_core_owns_a_registry_and_counts_each_request_under_its_route_label() {
        let mut listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .write_all(
                    b"GET /products/42 HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
                )
                .expect("the write failed");
            let mut answer = String::new();
            socket
                .read_to_string(&mut answer)
                .expect("the response could not be read");
            answer
        });

        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
            serve_on_this_core(
                &mut listener,
                &echo_under_a_named_route(),
                Waits::default(),
                &booted_on("[metrics]\nexporter = \"prometheus\"\n"),
                &Draining::detached(),
                |_note| {},
                || ControlFlow::Break(()),
            )
            .expect("the accept loop failed");
        });
        nvs_host::run_until_idle(&mut sched).expect("the loop failed");

        let answer = client.join().expect("the client thread panicked");
        assert!(
            answer.starts_with("HTTP/1.1 200 OK\r\n"),
            "the connection did not answer with a response: {answer}"
        );

        let registry = crate::metrics::on_this_core()
            .expect("a core serving a tree that names an exporter built a registry");
        let labels = [
            ("method", "GET"),
            ("status", "200"),
            ("route", "products.show"),
        ];
        assert_eq!(
            registry.read("nvs_requests_total", &labels),
            Some(&crate::metrics::Value::Counter(1)),
            "the request was not counted under its route's declared name"
        );
        let Some(crate::metrics::Value::Histogram(took)) =
            registry.read("nvs_request_duration_seconds", &labels)
        else {
            panic!("the duration series exists beside the counter");
        };
        assert_eq!(took.count, 1);
    }

    /// `rule:http-server/a-unix-socket-listener`'s claim that a request over
    /// the socket is byte-identical to the same request over TCP, asserted as
    /// the same case over the other family.
    ///
    /// It is the whole of what [`Listening`] and `nvs_host::NvsConnection`
    /// exist for, and the only test that runs their Unix arms: everything past
    /// the accept — `hyper`'s framing, the coroutine per connection, the
    /// handler, the answer — is reached through the same two functions as
    /// above, so a variant whose `poll_read` or `poll_write` went to the wrong
    /// socket would fail here and nowhere else. The trust half is not asserted
    /// here but in `crate::forwarded`, where the arrival is read.
    #[cfg(unix)]
    #[test]
    fn one_connection_over_a_unix_socket_gets_the_same_response() {
        let path = std::env::temp_dir().join(format!(
            "nvs-serve-unix-{}-{:?}.sock",
            std::process::id(),
            std::thread::current().id()
        ));
        drop(std::fs::remove_file(&path));
        let mut listener =
            nvs_host::NvsUnixListener::bind(&path).expect("the OS refused a socket path");

        let asked = path.clone();
        let client = std::thread::spawn(move || {
            let mut socket = std::os::unix::net::UnixStream::connect(&asked)
                .expect("the socket refused a connection");
            socket
                .write_all(b"GET /hello HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .expect("the write failed");
            let mut answer = String::new();
            socket
                .read_to_string(&mut answer)
                .expect("the response could not be read");
            answer
        });

        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
            serve_on_this_core(
                &mut listener,
                &echo_the_path(),
                Waits::default(),
                &wide_open(),
                &Draining::detached(),
                |_note| {},
                || ControlFlow::Break(()),
            )
            .expect("the accept loop failed");
        });
        nvs_host::run_until_idle(&mut sched).expect("the loop failed");

        let answer = client.join().expect("the client thread panicked");
        drop(std::fs::remove_file(&path));
        assert!(
            answer.starts_with("HTTP/1.1 200 OK\r\n") && answer.ends_with("hello /hello"),
            "the same request over a socket path was answered differently: {answer}"
        );
    }

    /// How long a smuggling case waits for the second response it asserts never
    /// arrives, on a connection the server is keeping alive.
    ///
    /// Short on purpose, because every row spends it: loopback delivery of a
    /// response already written is microseconds, so this is three orders of
    /// magnitude of headroom, and a machine too loaded to answer inside it is a
    /// machine that would not have produced the second response either.
    const SMUGGLED_PATIENCE: Duration = Duration::from_millis(500);

    /// One connection, the exact bytes a case names, and everything the peer
    /// read back before the server closed or the wait ran out.
    ///
    /// **The client neither asks for `Connection: close` nor half-closes**, and
    /// that is the whole reason this is not [`served_by`] with a literal in it.
    /// Either one ends the connection on the *client's* say-so, and then a door
    /// that had answered a request nobody made would look exactly like a door
    /// that had not: the count this suite asserts on would be one either way.
    /// Measured — the half-closing draft of this fixture read one response back
    /// from a genuinely pipelined pair. So the connection is left open and the
    /// absence of a second response is a bounded wait instead.
    fn read_back(raw: &[u8]) -> String {
        let listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");
        let payload = raw.to_vec();
        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .set_read_timeout(Some(SMUGGLED_PATIENCE))
                .expect("the socket refused a read timeout");
            socket.write_all(&payload).expect("the write failed");
            let mut seen = Vec::new();
            let mut chunk = [0_u8; 4096];
            while let Ok(read) = socket.read(&mut chunk) {
                if read == 0 {
                    break;
                }
                seen.extend_from_slice(&chunk[..read]);
            }
            String::from_utf8_lossy(&seen).into_owned()
        });
        served_by(listener, &echo_the_path(), client)
    }

    /// How many responses one connection carried back.
    fn responses(answer: &str) -> usize {
        answer.matches("HTTP/1.1 ").count()
    }

    /// M7's request smuggling suite — `rule:http-server/two-deployments-and-nothing-a-proxy-owns`'s last paragraph, which is
    /// the one place that ADR does *not* delegate to the proxy: smuggling is a
    /// proxy/origin parser differential, so the deployment that always has a
    /// proxy in front is exactly the one where a lenient origin is dangerous,
    /// and `rule:errors/ambiguous-input-refused`
    /// stands whole here.
    ///
    /// **Every row is one payload down one connection, and the property is the
    /// same for all of them: the peer gets back exactly one response, and it is
    /// never the smuggled request's.** That is asserted by *counting* status
    /// lines rather than by looking for `/smuggled` alone — a door that answered
    /// the hidden request with a `400` would pass the search and fail the count,
    /// and the count is what a proxy in front would be desynchronised by.
    ///
    /// **The three tables differ only in where the ambiguity sits**, and each
    /// asserts the thing that makes the peer's next request unsmuggleable: a
    /// head this door cannot read is refused before the handler; a head two
    /// parsers would frame differently is answered and the connection does not
    /// survive it; a head that is unambiguous is answered even though the body
    /// framing behind it then falls apart, and the bytes after that die with
    /// the connection. **No row lets a second request through**, which is the
    /// only thing a proxy in front can be desynchronised by.
    #[test]
    fn the_request_smuggling_suite_passes() {
        // Ambiguous in the head, and refused there: one framing spelled twice
        // and disagreeing with itself, or a spelling only one of two parsers
        // would see at all.
        let refused: [(&str, &[u8]); 6] = [
            (
                "two lengths that disagree",
                b"POST /outer HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\nContent-Length: 46\r\n\r\nGET /smuggled HTTP/1.1\r\nHost: localhost\r\n\r\n",
            ),
            (
                "one length header carrying two values",
                b"POST /outer HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0, 46\r\n\r\nGET /smuggled HTTP/1.1\r\nHost: localhost\r\n\r\n",
            ),
            (
                "a length that is not a number",
                b"POST /outer HTTP/1.1\r\nHost: localhost\r\nContent-Length: +46\r\n\r\nGET /smuggled HTTP/1.1\r\nHost: localhost\r\n\r\n",
            ),
            (
                "an encoding a proxy might read as chunked",
                b"POST /outer HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: xchunked\r\n\r\n0\r\n\r\nGET /smuggled HTTP/1.1\r\nHost: localhost\r\n\r\n",
            ),
            (
                "a space before the colon",
                b"POST /outer HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding : chunked\r\nContent-Length: 46\r\n\r\nGET /smuggled HTTP/1.1\r\nHost: localhost\r\n\r\n",
            ),
            (
                "a fold hiding a second length",
                b"POST /outer HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\n\tContent-Length: 46\r\n\r\nGET /smuggled HTTP/1.1\r\nHost: localhost\r\n\r\n",
            ),
        ];
        for (technique, payload) in refused {
            let answer = read_back(payload);
            assert_eq!(
                responses(&answer),
                1,
                "{technique}: the peer got more than the one refusal it is owed: {answer:?}"
            );
            assert!(
                answer.starts_with("HTTP/1.1 400 "),
                "{technique}: an ambiguous head was repaired rather than refused: {answer:?}"
            );
            assert!(
                !answer.contains("hello /outer"),
                "{technique}: a head nobody could frame reached the handler: {answer:?}"
            );
            assert!(
                !answer.contains("hello /smuggled"),
                "{technique}: the hidden request was answered: {answer:?}"
            );
        }

        // Two framings at once, which is the pair every published technique is
        // built on. RFC 9112 *disambiguates* this one rather than refusing it —
        // the chunked encoding wins and the length is ignored — and `hyper`
        // follows it, so these two rows are a measured divergence from ADR
        // 0095's "refused, never repaired" and are asserted as one rather than
        // left out of the suite. What makes it safe is the second half of the
        // same rule, and it is what these rows are really about: **the
        // connection does not survive the message**, so the bytes the proxy
        // read as a request of their own are never read as one here.
        let disambiguated: [(&str, &[u8]); 2] = [
            (
                "a length and a chunked encoding at once",
                b"POST /outer HTTP/1.1\r\nHost: localhost\r\nContent-Length: 6\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\nGET /smuggled HTTP/1.1\r\nHost: localhost\r\n\r\n",
            ),
            (
                "a chunked encoding and a length, in the other order",
                b"POST /outer HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\nContent-Length: 4\r\n\r\n0\r\n\r\nGET /smuggled HTTP/1.1\r\nHost: localhost\r\n\r\n",
            ),
        ];
        for (technique, payload) in disambiguated {
            let answer = read_back(payload);
            assert_eq!(
                responses(&answer),
                1,
                "{technique}: the peer got a second response: {answer:?}"
            );
            assert!(
                answer.to_ascii_lowercase().contains("connection: close"),
                "{technique}: the door kept a connection two parsers frame differently: {answer:?}"
            );
            assert!(
                !answer.contains("hello /smuggled"),
                "{technique}: the hidden request was answered: {answer:?}"
            );
        }

        // Unambiguous in the head and broken behind it: the outer request is
        // owed an answer by the time those bytes are read, so what is asserted
        // is only that they never become a request of their own.
        let broken: [(&str, &[u8]); 2] = [
            (
                "a chunk size that is not hexadecimal",
                b"POST /outer HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\n\r\n0x2c\r\nGET /smuggled HTTP/1.1\r\nHost: localhost\r\n\r\n",
            ),
            (
                "a chunked terminator ended with a bare LF",
                b"POST /outer HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\n\r\n0\n\nGET /smuggled HTTP/1.1\r\nHost: localhost\r\n\r\n",
            ),
        ];
        for (technique, payload) in broken {
            let answer = read_back(payload);
            assert_eq!(
                responses(&answer),
                1,
                "{technique}: the peer got a second response: {answer:?}"
            );
            assert!(
                !answer.contains("hello /smuggled"),
                "{technique}: the hidden request was answered: {answer:?}"
            );
        }

        // The control, and the reason the counting above is worth anything: two
        // requests the peer really did make, down one connection, are answered
        // twice. A door that answered nothing after the first response would
        // pass all ten rows above and fail here.
        let pipelined = read_back(
            b"GET /first HTTP/1.1\r\nHost: localhost\r\n\r\nGET /second HTTP/1.1\r\nHost: localhost\r\n\r\n",
        );
        assert_eq!(
            responses(&pipelined),
            2,
            "two pipelined requests were not both answered: {pipelined:?}"
        );
        assert!(
            pipelined.contains("hello /first") && pipelined.contains("hello /second"),
            "a pipelined pair was answered out of its own bodies: {pipelined:?}"
        );
    }

    /// `rule:concurrency/one-future-per-connection`'s second property, over the wire: **a future is polled only
    /// on the stack that owns it**, and the stack that owns it is the
    /// connection's own coroutine — so a drive that parked on the parking
    /// stream comes back on the *same* task rather than wherever an executor's
    /// next free worker happened to pick it up.
    ///
    /// Asserted as an **identity across a park**, which is what separates this
    /// from [`a_second_request_on_one_connection_is_answered_after_a_park`]:
    /// that case pins that the drive resumes at all, this one pins where. Both
    /// ids are read inside the handler, which `hyper` calls from inside the
    /// connection future's own `poll`, so what they name is the stack that poll
    /// ran on — and the third assertion is § 1's first property from the other
    /// end, that the stack is not the accept loop's.
    #[test]
    fn a_connection_future_is_driven_by_block_on_over_the_parking_stream() {
        let mut listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .set_read_timeout(Some(CLIENT_PATIENCE))
                .expect("the socket refused a read timeout");
            let mut seen = String::new();
            socket
                .write_all(b"GET /first HTTP/1.1\r\nHost: localhost\r\n\r\n")
                .expect("the write failed");
            read_until(&mut socket, "hello /first", &mut seen);
            // The drive is parked on the keep-alive read at this point, with
            // nothing on this core to poll it: what ends the park is this head
            // arriving on the parking stream and nothing else.
            socket
                .write_all(b"GET /second HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .expect("the write failed");
            socket
                .read_to_string(&mut seen)
                .expect("the response could not be read");
            end_the_loop(addr);
            seen
        });

        // One entry per request `hyper` asked the handler for, in order.
        let polled_on: Rc<RefCell<Vec<nvs_host::TaskId>>> = Rc::new(RefCell::new(Vec::new()));
        let handler = Rc::new({
            let polled_on = Rc::clone(&polled_on);
            move |request: Request<Incoming>, _origin: Origin| {
                polled_on.borrow_mut().push(
                    nvs_host::current_task().expect("the connection future was polled off a task"),
                );
                let path = request.uri().path().to_owned();
                let program: Program = Box::new(move |child: &mut Ctx, _args| {
                    child
                        .write_output(format!("hello {path}").as_bytes())
                        .expect("a buffer");
                    Value::null()
                });
                Reply::run(Isolate::new(program, Value::null(), Output::Capture))
            }
        });

        let accepting: Rc<Cell<Option<nvs_host::TaskId>>> = Rc::new(Cell::new(None));
        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, {
            let accepting = Rc::clone(&accepting);
            move |_ctx| {
                accepting.set(nvs_host::current_task());
                serve_on_this_core(
                    &mut listener,
                    &handler,
                    Waits::default(),
                    &wide_open(),
                    &Draining::detached(),
                    |_note| {},
                    until_the_loop_is_ended(),
                )
                .expect("the accept loop failed");
            }
        });
        nvs_host::run_until_idle(&mut sched).expect("the loop failed");

        let answer = client.join().expect("the client thread panicked");
        let polled_on = polled_on.borrow();
        assert_eq!(
            polled_on.len(),
            2,
            "the handler was not asked once per request: {polled_on:?}"
        );
        assert_eq!(
            polled_on[0], polled_on[1],
            "the drive came back on a different stack than the one it parked on: {polled_on:?}"
        );
        assert_ne!(
            Some(polled_on[0]),
            accepting.get(),
            "the connection was polled on the accept loop's own task"
        );
        assert!(
            answer.ends_with("hello /second"),
            "the drive did not answer the request that ended its park: {answer}"
        );
    }

    /// `rule:concurrency/one-future-per-connection`'s third property: **`Pending` suspends the task and not the
    /// thread**, which is `rule:http-server/a-core-is-never-blocked-on-a-syscall`'s rule stated about this seam. One
    /// core, two connections, and the second is answered in full while the
    /// first's drive is parked half-way through a request head.
    ///
    /// One client thread, so the ordering is the case's rather than a race: the
    /// half-sent head is on the wire before the second socket opens, and its
    /// remainder goes out only once the second has been answered. The read
    /// timeout is what makes a blocked core *fail* here instead of hanging the
    /// suite — an assertion nobody reaches is worth nothing.
    ///
    #[test]
    fn a_connection_never_blocks_the_core_it_runs_on() {
        let mut listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let client = std::thread::spawn(move || {
            let mut parked = TcpStream::connect(addr).expect("the loopback refused a connection");
            parked
                .set_read_timeout(Some(CLIENT_PATIENCE))
                .expect("the socket refused a read timeout");
            // A head with no blank line after it: the drive reads what arrived,
            // answers `Pending`, and parks this connection's coroutine.
            parked
                .write_all(b"GET /parked HTTP/1.1\r\nHost: localhost\r\n")
                .expect("the write failed");

            let mut served = TcpStream::connect(addr).expect("the loopback refused a connection");
            served
                .set_read_timeout(Some(CLIENT_PATIENCE))
                .expect("the socket refused a read timeout");
            served
                .write_all(b"GET /served HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .expect("the write failed");
            let mut second = String::new();
            served
                .read_to_string(&mut second)
                .expect("a core holding a parked connection never answered the second one");

            parked
                .write_all(b"Connection: close\r\n\r\n")
                .expect("the write failed");
            let mut first = String::new();
            parked
                .read_to_string(&mut first)
                .expect("the parked connection was never answered");
            (first, second)
        });

        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
            let accepted = Cell::new(0_usize);
            serve_on_this_core(
                &mut listener,
                &echo_the_path(),
                Waits::default(),
                &wide_open(),
                &Draining::detached(),
                |_note| {},
                || {
                    accepted.set(accepted.get() + 1);
                    if accepted.get() < 2 {
                        ControlFlow::Continue(())
                    } else {
                        ControlFlow::Break(())
                    }
                },
            )
            .expect("the accept loop failed");
        });
        run_the_core(&mut sched);

        let (first, second) = client.join().expect("the client thread panicked");
        assert!(
            second.ends_with("hello /served"),
            "the core did not serve its other connection while one was parked: {second}"
        );
        assert!(
            first.ends_with("hello /parked"),
            "the parked connection did not answer once its head arrived: {first}"
        );
    }

    /// Runs `connections` sequential requests through one accept loop and
    /// answers how many tasks the whole run finished.
    ///
    /// Sequential on purpose: each socket is answered before the next opens, so
    /// the loop's own `keep_serving` counter is what decides when it stops
    /// accepting rather than the order the OS hands connections over in.
    fn tasks_finished<H>(connections: usize, handler: &Rc<H>) -> usize
    where
        H: Fn(Request<Incoming>, Origin) -> Reply + 'static,
    {
        let mut listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let client = std::thread::spawn(move || {
            for _ in 0..connections {
                let mut socket =
                    TcpStream::connect(addr).expect("the loopback refused a connection");
                socket
                    .set_read_timeout(Some(CLIENT_PATIENCE))
                    .expect("the socket refused a read timeout");
                socket
                    .write_all(
                        b"GET /counted HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
                    )
                    .expect("the write failed");
                let mut answer = String::new();
                socket
                    .read_to_string(&mut answer)
                    .expect("the response could not be read");
                assert!(
                    answer.starts_with("HTTP/1.1 "),
                    "a counted connection was not answered: {answer}"
                );
            }
        });

        let handler = Rc::clone(handler);
        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
            let accepted = Cell::new(0_usize);
            serve_on_this_core(
                &mut listener,
                &handler,
                Waits::default(),
                &wide_open(),
                &Draining::detached(),
                |_note| {},
                || {
                    accepted.set(accepted.get() + 1);
                    if accepted.get() < connections {
                        ControlFlow::Continue(())
                    } else {
                        ControlFlow::Break(())
                    }
                },
            )
            .expect("the accept loop failed");
        });
        let report = run_the_core(&mut sched);
        client.join().expect("the client thread panicked");
        assert_eq!(
            sched.tracked_tasks(),
            0,
            "the run left a task behind: `rule:concurrency/nothing-is-still-running-when-a-call-returns`"
        );
        report.finished
    }

    /// `rule:concurrency/one-future-per-connection`'s first property, counted rather than read off a line:
    /// **there is no queue of futures and no spawn.** Serving a connection
    /// costs exactly one task — the coroutine that drives its future — so a
    /// second connection costs exactly one more, and `hyper` contributed none
    /// of them.
    ///
    /// The third measurement is the other side of the same bound, and it is the
    /// distinction § 1 draws: a *request* does cost a second task, because an
    /// [`Isolate`] asks the **scheduler** for one exactly as `Core\Task` does.
    /// What the seam may not do is hand one out itself, and a count that only
    /// ever went up by one per connection could not tell the two apart.
    #[test]
    fn no_task_is_spawned_to_serve_a_connection() {
        let a_status = Rc::new(|_request: Request<Incoming>, _origin: Origin| Reply::not_found());
        let one = tasks_finished(1, &a_status);
        let two = tasks_finished(2, &a_status);
        assert_eq!(
            one, 2,
            "one connection cost more than the accept loop and its own coroutine"
        );
        assert_eq!(
            two - one,
            1,
            "a second connection cost more than one more task: {one} then {two}"
        );
        assert_eq!(
            tasks_finished(1, &echo_the_path()),
            one + 1,
            "a request's isolate is the one task a connection may add, and it was not"
        );
    }

    /// Stage 2's item 2, and the reason it is an item rather than an
    /// assumption: a request runs as [`Isolate`] — the type `spawn script`
    /// runs under `rule:security/isolate-shares-nothing` — on a task of its own, so M7's state-bleed suite
    /// parameterises one isolation mechanism instead of proving something about
    /// two.
    ///
    /// Three ids, all different, which is the *tree* rather than a nesting of
    /// calls: the accept loop's, the connection's, and the one the program ran
    /// on. A request run inside the connection future's own `poll` would report
    /// the second twice — and it is precisely what [`Peer`] exists not to do,
    /// since a connection that cannot go round its dispatcher's loop cannot
    /// read the body its own request is waiting for.
    ///
    #[test]
    fn a_request_is_the_root_isolate_of_a_request_tree() {
        let mut listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .set_read_timeout(Some(CLIENT_PATIENCE))
                .expect("the socket refused a read timeout");
            socket
                .write_all(b"GET /rooted HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .expect("the write failed");
            let mut answer = String::new();
            socket
                .read_to_string(&mut answer)
                .expect("the response could not be read");
            answer
        });

        let connection: Rc<Cell<Option<nvs_host::TaskId>>> = Rc::new(Cell::new(None));
        let request: Rc<Cell<Option<nvs_host::TaskId>>> = Rc::new(Cell::new(None));
        let handler = Rc::new({
            let connection = Rc::clone(&connection);
            let request = Rc::clone(&request);
            move |_request: Request<Incoming>, _origin: Origin| {
                connection.set(nvs_host::current_task());
                let ran_on = Rc::clone(&request);
                let program: Program = Box::new(move |child: &mut Ctx, _args| {
                    ran_on.set(nvs_host::current_task());
                    child.write_output(b"rooted").expect("a buffer");
                    Value::null()
                });
                Reply::run(Isolate::new(program, Value::null(), Output::Capture))
            }
        });

        let accepting: Rc<Cell<Option<nvs_host::TaskId>>> = Rc::new(Cell::new(None));
        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, {
            let accepting = Rc::clone(&accepting);
            move |_ctx| {
                accepting.set(nvs_host::current_task());
                serve_on_this_core(
                    &mut listener,
                    &handler,
                    Waits::default(),
                    &wide_open(),
                    &Draining::detached(),
                    |_note| {},
                    || ControlFlow::Break(()),
                )
                .expect("the accept loop failed");
            }
        });
        nvs_host::run_until_idle(&mut sched).expect("the loop failed");

        let answer = client.join().expect("the client thread panicked");
        let accepting = accepting.get().expect("the accept loop ran off a task");
        let connection = connection.get().expect("the handler ran off a task");
        let request = request.get().expect("the request's program never ran");
        assert_ne!(
            connection, accepting,
            "the connection was served on the accept loop's own task"
        );
        assert_ne!(
            request, connection,
            "the request ran inside the connection future's poll rather than beside it"
        );
        assert_ne!(
            request, accepting,
            "the request ran on the accept loop's own task"
        );
        assert!(
            answer.ends_with("rooted"),
            "the isolate's own output did not come back as the body: {answer}"
        );
        assert_eq!(
            sched.tracked_tasks(),
            0,
            "the request tree outlived the request it belonged to: `rule:concurrency/nothing-is-still-running-when-a-call-returns`"
        );
    }

    /// `rule:http-server/secure-headers-with-nothing-written` over the wire: a server nobody configured answers with the
    /// secure set, on a response a program wrote and never asked for them.
    ///
    /// Deliberately one connection rather than one per [`Reply`] branch — the
    /// set is filled at the single point every response leaves by, so a second
    /// case over a `404` would assert the same line twice. What the set *is*,
    /// and the override that leaves a program's own header alone, are
    /// [`crate::secure`]'s cases; this one is that the wiring happened at all.
    #[test]
    fn a_response_carries_section_ones_shipped_headers() {
        let mut listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .write_all(b"GET /hello HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .expect("the write failed");
            let mut answer = String::new();
            socket
                .read_to_string(&mut answer)
                .expect("the response could not be read");
            answer
        });

        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
            serve_on_this_core(
                &mut listener,
                &echo_the_path(),
                Waits::default(),
                &wide_open(),
                &Draining::detached(),
                |_note| {},
                || ControlFlow::Break(()),
            )
            .expect("the accept loop failed");
        });
        nvs_host::run_until_idle(&mut sched).expect("the loop failed");

        let answer = client.join().expect("the client thread panicked");
        let sent = answer.to_ascii_lowercase();
        for line in [
            "x-content-type-options: nosniff",
            "content-security-policy: frame-ancestors 'none'",
            "referrer-policy: strict-origin-when-cross-origin",
        ] {
            assert!(
                sent.contains(line),
                "§ 1's `{line}` was not on the response: {answer}"
            );
        }
        assert!(
            !sent.contains("strict-transport-security"),
            "HSTS reached a plaintext connection, which § 1 sends it on nothing but an \
             `https` effective scheme: {answer}"
        );
    }

    /// `rule:http-server/two-deployments-and-nothing-a-proxy-owns`'s compression
    /// row, over the wire: a peer that offers every encoding there is gets back
    /// exactly what a peer that offered none would get.
    ///
    /// One connection, for [`a_response_carries_section_ones_shipped_headers`]'s
    /// reason — a response leaves by a single point, so a second case over
    /// another [`Reply`] branch would assert the same absence twice. The length
    /// is asserted beside the header name because those are the two ways a body
    /// could arrive encoded: a `Content-Encoding` nobody asked the server for,
    /// and bytes that are not the handler's own under no header at all.
    /// `Vary: Accept-Encoding` is asserted absent with them, because a response
    /// path that negotiates has to say it varies on what it negotiated over, so
    /// that line is the tell that the wiring exists and happened to choose the
    /// identity encoding on this request.
    ///
    /// A program that compresses its own body with `Core\Compress` is untouched
    /// by this: what the row forbids is the response path doing it implicitly,
    /// where the bound the bytes were decompressed under would be a policy the
    /// server picked rather than the request's own.
    #[test]
    fn the_server_still_sets_no_content_encoding_of_its_own() {
        let listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .write_all(
                    b"GET /hello HTTP/1.1\r\nHost: localhost\r\n\
                      Accept-Encoding: gzip, deflate, br, zstd\r\nConnection: close\r\n\r\n",
                )
                .expect("the write failed");
            let mut answer = String::new();
            socket
                .read_to_string(&mut answer)
                .expect("the response could not be read");
            answer
        });

        let answer = served_by(listener, &echo_the_path(), client);
        let sent = answer.to_ascii_lowercase();
        assert!(
            sent.starts_with("http/1.1 200 ok\r\n"),
            "the request the assertions below read was not answered: {answer}"
        );
        assert!(
            !sent.contains("content-encoding"),
            "the response path encoded a body nobody asked it to: {answer}"
        );
        assert!(
            !sent.contains("vary: accept-encoding"),
            "the response negotiated over the encodings it was offered: {answer}"
        );
        assert!(
            sent.contains("content-length: 12"),
            "the handler's twelve bytes did not arrive as twelve bytes: {answer}"
        );
        assert!(
            answer.ends_with("hello /hello"),
            "the body was not the handler's own bytes: {answer}"
        );
    }

    /// `rule:http-server/cors-is-closed-until-origins-are-named`'s closed default, over the wire: a peer that named an origin
    /// is told nothing about whether it may read the answer, because
    /// `[http.cors] origins` names nobody.
    ///
    /// The assertion is over the whole `access-control-` prefix rather than over
    /// `Access-Control-Allow-Origin` alone, because "no CORS header is emitted at
    /// all" is what § 2 means by closed: a response that withheld the allow line
    /// while still sending an exposed-header list or a `max-age` would pass the
    /// narrow assertion having told a browser something nobody configured it to
    /// say. The `200` and the echoed path are asserted with it so that a request
    /// refused for some other reason cannot pass by carrying no headers at all.
    #[test]
    fn cors_is_closed_with_nothing_configured() {
        let listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .write_all(
                    b"GET /hello HTTP/1.1\r\nHost: localhost\r\n\
                      Origin: https://elsewhere.example\r\nConnection: close\r\n\r\n",
                )
                .expect("the write failed");
            let mut answer = String::new();
            socket
                .read_to_string(&mut answer)
                .expect("the response could not be read");
            answer
        });

        let answer = served_by(listener, &echo_the_path(), client);
        let sent = answer.to_ascii_lowercase();
        assert!(
            sent.starts_with("http/1.1 200 ok"),
            "the cross-origin request was not the one this case is about: {answer}"
        );
        assert!(
            sent.contains("hello /hello"),
            "the request was not answered by the program that ran it: {answer}"
        );
        assert!(
            !sent.contains("access-control"),
            "§ 2 emits no CORS header at all with `origins = []`, and this answer carried \
             one: {answer}"
        );
    }

    /// `rule:http-server/cors-is-closed-until-origins-are-named`'s open half over the wire: a tree that named `https://allowed.example`
    /// tells that origin it may read the answer, tells every other origin nothing, and marks
    /// **both** answers as varying by `Origin`.
    ///
    /// The two requests are one case rather than two because what is being pinned is that they
    /// differ in the allow line and *agree* on the `Vary` — asserted apart, the refused half
    /// passes a server that never varies anything and the allowed half passes one that varies
    /// only what it allowed, which is the cache bug [`crate::cors`]'s module doc is about. The
    /// header names are read off the wire because this is the assertion that the door writes
    /// what the policy decided; `cors::tests` is where the decision itself is pinned.
    #[test]
    fn a_named_origin_crosses_and_every_answer_says_it_varies() {
        for (origin, allowed) in [
            ("https://allowed.example", true),
            ("https://other.example", false),
        ] {
            let listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
                .expect("the OS refused a port");
            let addr = listener
                .local_addr()
                .expect("a bound listener had no address");

            let client = std::thread::spawn(move || {
                let mut socket =
                    TcpStream::connect(addr).expect("the loopback refused a connection");
                socket
                    .write_all(
                        format!(
                            "GET /hello HTTP/1.1\r\nHost: localhost\r\n\
                             Origin: {origin}\r\nConnection: close\r\n\r\n"
                        )
                        .as_bytes(),
                    )
                    .expect("the write failed");
                let mut answer = String::new();
                socket
                    .read_to_string(&mut answer)
                    .expect("the response could not be read");
                answer
            });

            let answer = served_under(listener, &echo_the_path(), client, naming_one_origin());
            let sent = answer.to_ascii_lowercase();
            assert!(
                sent.starts_with("http/1.1 200 ok"),
                "the cross-origin request was not the one this case is about: {answer}"
            );
            assert_eq!(
                sent.contains("access-control-allow-origin: https://allowed.example"),
                allowed,
                "`{origin}` was answered by the wrong half of `[http.cors] origins`: {answer}"
            );
            assert!(
                sent.contains("vary: origin"),
                "an answer an open policy could have varied did not say so: {answer}"
            );
        }
    }

    /// [`wide_open`] with one origin named, which is the only difference between § 2's two
    /// halves as a request meets them.
    fn naming_one_origin() -> Serving {
        let http = nvs_config::tree::Http {
            cors: Some(nvs_config::tree::HttpCors {
                origins: Some(vec!["https://allowed.example".to_owned()]),
                ..nvs_config::tree::HttpCors::default()
            }),
            ..nvs_config::tree::Http::default()
        };
        Serving::new(
            Arc::new(Admission::new(&Ceiling::of(&Capacity {
                configured: 10_000,
                per_request: None,
                budget: None,
            }))),
            Arc::new(Secure::default()),
            Arc::new(Trusted::none()),
            Arc::new(Cors::of(Some(&http))),
            Arc::default(),
        )
    }

    /// `rule:http-server/cors-is-closed-until-origins-are-named`'s other half over the wire: a preflight is answered `403`
    /// with `[http.cors] origins` naming nobody, and **no program is asked**.
    ///
    /// The flag is the half of the case that is not the status. A `403` a handler
    /// produced and a `403` the policy above it took are the same three bytes on
    /// the wire and are not the same guarantee — [`crate::cors`]'s reason for
    /// refusing here is that a preflight nobody configured must select no mount
    /// and allocate no isolate — so what is asserted is that the handler was
    /// never reached at all. § 1's set is asserted on it beside that, because a
    /// refusal is a response and is owed the same policy as the request it
    /// refused.
    #[test]
    fn a_preflight_is_refused_before_any_program_runs() {
        let listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        // Set by the handler, and the point of the case is that it stays false.
        let asked = Rc::new(Cell::new(false));
        let handler = Rc::new({
            let asked = Rc::clone(&asked);
            move |_request: Request<Incoming>, _origin: Origin| {
                asked.set(true);
                Reply::status(StatusCode::OK)
            }
        });

        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .write_all(
                    b"OPTIONS /widgets HTTP/1.1\r\nHost: localhost\r\n\
                      Origin: https://elsewhere.example\r\n\
                      Access-Control-Request-Method: POST\r\nConnection: close\r\n\r\n",
                )
                .expect("the write failed");
            let mut answer = String::new();
            socket
                .read_to_string(&mut answer)
                .expect("the response could not be read");
            answer
        });

        let answer = served_by(listener, &handler, client);
        assert!(
            answer.starts_with("HTTP/1.1 403 Forbidden"),
            "§ 2 answers a preflight `403` while no origin is named: {answer}"
        );
        assert!(
            !asked.get(),
            "the preflight reached the handler, so the refusal was taken below the policy \
             that owns it: {answer}"
        );
        assert!(
            answer
                .to_ascii_lowercase()
                .contains("x-content-type-options: nosniff"),
            "§ 1's set did not reach a response this server wrote: {answer}"
        );
    }

    /// `rule:http-server/cors-is-closed-until-origins-are-named`'s granting half over the wire: a named origin's preflight is
    /// answered `204` with what the block configures, and **no program is asked**
    /// for that answer either.
    ///
    /// The flag is again the load-bearing half. `403` and `204` are two answers
    /// to the same question, and the reason the second one may not come from a
    /// handler is the reason the first may not: § 2 configures the whole answer,
    /// so an application producing it would be answering a question already
    /// decided above it — and the two could disagree. What is asserted beside it
    /// is that the shipped `methods` and `max_age` reach the wire, `[http.cors]`
    /// naming nothing but an origin; `cors::tests` is where each line's own value
    /// is pinned.
    #[test]
    fn a_named_origins_preflight_is_granted_before_any_program_runs() {
        let listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let asked = Rc::new(Cell::new(false));
        let handler = Rc::new({
            let asked = Rc::clone(&asked);
            move |_request: Request<Incoming>, _origin: Origin| {
                asked.set(true);
                Reply::status(StatusCode::OK)
            }
        });

        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .write_all(
                    b"OPTIONS /widgets HTTP/1.1\r\nHost: localhost\r\n\
                      Origin: https://allowed.example\r\n\
                      Access-Control-Request-Method: POST\r\nConnection: close\r\n\r\n",
                )
                .expect("the write failed");
            let mut answer = String::new();
            socket
                .read_to_string(&mut answer)
                .expect("the response could not be read");
            answer
        });

        let answer = served_under(listener, &handler, client, naming_one_origin());
        let sent = answer.to_ascii_lowercase();
        assert!(
            answer.starts_with("HTTP/1.1 204 No Content"),
            "§ 2 answers a named origin's preflight `204`: {answer}"
        );
        assert!(
            !asked.get(),
            "the preflight reached the handler, so the grant was taken below the policy \
             that owns it: {answer}"
        );
        for line in [
            "access-control-allow-origin: https://allowed.example",
            "access-control-allow-methods: get, head, post",
            "access-control-max-age: 600",
            "vary: origin",
        ] {
            assert!(
                sent.contains(line),
                "the grant did not carry `{line}`: {answer}"
            );
        }
    }

    /// `rule:http-server/the-server-block-is-boot-class`'s probe across a shutdown: `200` while the loop is
    /// accepting, `503` from the moment it stops, both from one run and one
    /// handler.
    ///
    /// The changeover is the whole endpoint. A probe pinned at `200` is
    /// indistinguishable from this one for as long as the server is up, and the
    /// answer that matters is the one given while the process is draining —
    /// that is what a proxy takes an instance out of rotation on, and answering
    /// it after the last connection has gone would report the one state nobody
    /// can act on.
    ///
    /// The two connections have to be sequential — the first is answered `200`
    /// only because the loop yields to its child before accepting the second,
    /// and a second socket already waiting in the backlog would be accepted
    /// without that yield, so both requests would see the drain. That leaves a
    /// window this case cannot close from the outside: between the two, the
    /// loop is parked in `accept` with nothing else runnable, which is exactly
    /// the state one `run_until_idle` call can read as idle. [`run_the_core`]
    /// is what closes it, and the client's read deadline is what turns a core
    /// that stopped anyway into a failure rather than a wait.
    #[test]
    fn is_draining_answers_during_a_graceful_shutdown() {
        let mut listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let client = std::thread::spawn(move || {
            let probe = || {
                let mut socket = TcpStream::connect(addr).expect("the loopback refused a socket");
                // `connect` succeeds against a bound listener whether or not
                // anything ever accepts — the socket simply waits in the
                // backlog — so a loop that stopped early leaves the read below
                // with no answer and no end of file to end it on, and an
                // unbounded read there hangs the suite rather than failing it.
                socket
                    .set_read_timeout(Some(CLIENT_PATIENCE))
                    .expect("the socket refused a read timeout");
                socket
                    .write_all(
                        b"GET /healthz HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
                    )
                    .expect("the write failed");
                let mut answer = String::new();
                socket
                    .read_to_string(&mut answer)
                    .expect("the response could not be read");
                answer
            };
            // Sequential, and the second only after the first has been answered
            // in full: the connection the shutdown lands on is one this loop has
            // not accepted yet when the first is served.
            (probe(), probe())
        });

        let draining = Draining::detached();
        let handler = Rc::new({
            let draining = draining.clone();
            move |request: Request<Incoming>, _origin: Origin| {
                assert_eq!(request.uri().path(), "/healthz");
                Reply::health(&draining)
            }
        });
        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, {
            let draining = draining.clone();
            move |_ctx| {
                let accepted = Cell::new(0_usize);
                serve_on_this_core(
                    &mut listener,
                    &handler,
                    Waits::default(),
                    &wide_open(),
                    &draining,
                    |_note| {},
                    // One more connection, and then the shutdown — which is the
                    // seam `nvs ctl` will pull rather than a second mechanism.
                    || {
                        accepted.set(accepted.get() + 1);
                        if accepted.get() < 2 {
                            ControlFlow::Continue(())
                        } else {
                            ControlFlow::Break(())
                        }
                    },
                )
                .expect("the accept loop failed");
            }
        });
        // Driven rather than run once, per [`run_the_core`]: the accept loop is
        // parked in `accept` between the two probes with nothing else runnable,
        // and a single call can read that as idle. Driven before the join,
        // because from there the same failure reads as a client that went
        // unanswered and says nothing about which side stopped.
        run_the_core(&mut sched);

        let (accepting, shutting_down) = client.join().expect("the client thread panicked");
        assert!(
            accepting.starts_with("HTTP/1.1 200 OK\r\n"),
            "a server that was still accepting did not answer the probe `200`: {accepting}"
        );
        assert!(
            shutting_down.starts_with("HTTP/1.1 503 Service Unavailable\r\n"),
            "a draining server did not answer the probe `503`: {shutting_down}"
        );
        // The state outlives the loop that set it, so a second core's handler
        // and an application both read the same shutdown.
        assert!(
            draining.is_draining(),
            "the loop returned without marking the drain"
        );
    }

    /// The drain is the **process's** bit, so a worker that took its own handle
    /// answers what every other worker answers: a probe is a fact about the
    /// instance a proxy is deciding about, and not about which core happened to
    /// take the connection carrying it
    /// (`rule:http-server/the-accept-fan-out-is-one-worker-per-core`).
    ///
    /// Each handle is constructed on its own thread, because that is how a
    /// worker takes one — [`Draining::process`] called there rather than a clone
    /// handed down from the boot. Sharing that comes from the constructor is the
    /// property under test; a clone would be asserting [`nvs_runtime::drain`]'s
    /// own case a second time.
    ///
    /// **This is the only test in this binary that begins the process drain, and
    /// a drain is one-way.** Every other case here takes [`Draining::detached`],
    /// which is what that constructor exists for.
    #[test]
    fn is_draining_answers_the_same_on_every_core() {
        let first = std::thread::spawn(Draining::process)
            .join()
            .expect("a core panicked taking its drain handle");
        let second = std::thread::spawn(Draining::process)
            .join()
            .expect("a core panicked taking its drain handle");
        assert!(
            !first.is_draining(),
            "a server that has not stopped accepting reported a drain"
        );
        assert!(
            !second.is_draining(),
            "a server that has not stopped accepting reported a drain"
        );

        // One core's accept loop reaching its tail, which is the only writer
        // there is: a shutdown drains the process, so the core that gets here
        // first is answering for all of them.
        first.begin();
        assert!(
            second.is_draining(),
            "a second core answered from a bit of its own, so the probe's answer would depend on \
             which core the proxy reached"
        );
        // The reader that was handed no handle at all: `Core\Server::isDraining()`
        // is the same bit, so an application answers what the probe answers
        // whichever core it is running on.
        assert!(
            nvs_runtime::drain::is_draining(),
            "the process's drain was invisible to the reader an application uses"
        );
        assert!(
            !Draining::detached().is_draining(),
            "a server of its own reported the process's drain as its"
        );
    }

    /// A core's in-flight tally is its own, so the process ends when the **last**
    /// of them reaches zero: a worker whose connections are all finished returns
    /// while one still serving holds the process up alone
    /// (`rule:http-server/the-accept-fan-out-is-one-worker-per-core`).
    ///
    /// Two workers on two listeners rather than two handles on one, because
    /// which core the OS hands a connection to is the OS's choice — that is the
    /// fan-out's own property and the wrong thing for a case about the tally to
    /// rest on. Both halves are asserted: the first core returns with the second
    /// still holding a connection, and the second returns only once that
    /// connection has ended.
    #[test]
    fn the_process_exits_when_the_last_cores_in_flight_count_reaches_zero() {
        /// One worker: this core's listener, one connection accepted, and then
        /// the tail that is under test. `accepted` reports the hand-over and
        /// `returned` the tally reaching zero, so the test can tell those two
        /// moments apart from the outside.
        fn one_core(
            listener: std::net::TcpListener,
            accepted: std::sync::mpsc::Sender<()>,
            returned: std::sync::mpsc::Sender<()>,
        ) -> impl FnOnce(&mut nvs_host::Scheduler) + Send + 'static {
            move |sched| {
                // The socket is bound before any worker exists and each core
                // takes its own handle from there, which is what makes this the
                // fleet's shape rather than a second binding.
                let mut listener = NvsListener::from_std(listener)
                    .expect("the OS refused a non-blocking listener");
                let _installed = nvs_host::reactor::install(
                    nvs_host::Reactor::new().expect("the OS refused a poll"),
                );
                let handler = Rc::new(|_request: Request<Incoming>, _origin: Origin| {
                    Reply::status(StatusCode::OK)
                });
                sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
                    serve_on_this_core(
                        &mut listener,
                        &handler,
                        Waits::default(),
                        &wide_open(),
                        // This core's own drain and not the process's: what is
                        // under test is when the loop returns, and the process
                        // bit is one-way for the whole binary.
                        &Draining::detached(),
                        |_note| {},
                        || {
                            let _ = accepted.send(());
                            ControlFlow::Break(())
                        },
                    )
                    .expect("the accept loop failed");
                    // The loop's tail let go, so this core's tally is zero and
                    // its worker thread is about to end.
                    let _ = returned.send(());
                });
                run_the_core(sched);
            }
        }

        let cpus = nvs_host::cpus();
        let Some(first_cpu) = cpus.first().copied() else {
            // A host that enumerates no CPU is served from the boot thread, so
            // there is no second tally there for this to be about.
            return;
        };
        // One CPU is a fleet of two workers sharing it, which is `nvs serve`'s
        // own answer when a written count is above this machine's parallelism.
        let second_cpu = cpus.get(1).copied().unwrap_or(first_cpu);

        let first_socket =
            std::net::TcpListener::bind("127.0.0.1:0").expect("the OS refused a port");
        let first_addr = first_socket
            .local_addr()
            .expect("a bound listener had no address");
        let second_socket =
            std::net::TcpListener::bind("127.0.0.1:0").expect("the OS refused a port");
        let second_addr = second_socket
            .local_addr()
            .expect("a bound listener had no address");

        let (first_handed_over, _first_accepted) = std::sync::mpsc::channel();
        let (first_at_zero, first_returned) = std::sync::mpsc::channel();
        let (second_handed_over, second_accepted) = std::sync::mpsc::channel();
        let (second_at_zero, second_returned) = std::sync::mpsc::channel();

        let first_worker = nvs_host::Worker::spawn(
            first_cpu,
            one_core(first_socket, first_handed_over, first_at_zero),
        )
        .expect("the OS refused a worker thread");
        let second_worker = nvs_host::Worker::spawn(
            second_cpu,
            one_core(second_socket, second_handed_over, second_at_zero),
        )
        .expect("the OS refused a worker thread");

        // The second core's connection first, and nothing written on it: it is
        // in flight from the accept until this socket closes, which is the whole
        // window the assertion below needs to exist.
        let held = TcpStream::connect(second_addr).expect("the loopback refused a socket");
        second_accepted
            .recv_timeout(CLIENT_PATIENCE)
            .expect("the second core never accepted the connection it is meant to be holding");

        // The first core's connection, answered and then closed by the client:
        // that core's tally is back to zero and its loop returns.
        let mut client = TcpStream::connect(first_addr).expect("the loopback refused a socket");
        client
            .set_read_timeout(Some(CLIENT_PATIENCE))
            .expect("the socket refused a read timeout");
        client
            .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .expect("the write failed");
        let mut answer = String::new();
        client
            .read_to_string(&mut answer)
            .expect("the response could not be read");
        assert!(
            answer.starts_with("HTTP/1.1 200 OK\r\n"),
            "the first core did not answer its one connection: {answer}"
        );
        drop(client);
        first_returned
            .recv_timeout(CLIENT_PATIENCE)
            .expect("the first core never returned, though its only connection had finished");

        // The one this case exists for. The first core is done and the second is
        // not, so a process that exited on a tally would already be gone with a
        // connection still being served.
        assert!(
            matches!(
                second_returned.try_recv(),
                Err(std::sync::mpsc::TryRecvError::Empty)
            ),
            "a core returned with a connection still in flight, so the process would exit on the \
             first core's tally rather than on the last one's"
        );

        drop(held);
        second_returned
            .recv_timeout(CLIENT_PATIENCE)
            .expect("the last core never returned once the connection it held had ended");
        // Every worker joined is the process exiting, and it took both.
        first_worker.join().expect("the first worker panicked");
        second_worker.join().expect("the last worker panicked");
    }

    /// The question the seam is actually built around: a connection that has
    /// answered and is waiting for the next request **parks the coroutine** —
    /// `poll_read` answers `Pending` having armed the reactor — and the readiness
    /// that ends the park resumes the drive rather than the stack inside
    /// `hyper`. A second answer over one socket is that, asserted.
    #[test]
    fn a_second_request_on_one_connection_is_answered_after_a_park() {
        let mut listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .write_all(b"GET /one HTTP/1.1\r\nHost: localhost\r\n\r\n")
                .expect("the write failed");
            let mut seen = String::new();
            // The second request is written only once the first answer is in,
            // so the connection has demonstrably had nothing to read in
            // between. Pipelining both would assert a single pass instead.
            read_until(&mut socket, "hello /one", &mut seen);
            socket
                .write_all(b"GET /two HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .expect("the second write failed");
            socket
                .read_to_string(&mut seen)
                .expect("the second response could not be read");
            end_the_loop(addr);
            seen
        });

        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
            serve_on_this_core(
                &mut listener,
                &echo_the_path(),
                Waits::default(),
                &wide_open(),
                &Draining::detached(),
                |_note| {},
                until_the_loop_is_ended(),
            )
            .expect("the accept loop failed");
        });
        nvs_host::run_until_idle(&mut sched).expect("the loop failed");

        let seen = client.join().expect("the client thread panicked");
        assert!(
            seen.contains("hello /one") && seen.contains("hello /two"),
            "one connection did not answer twice: {seen}"
        );
        assert_eq!(
            seen.matches("HTTP/1.1 200 OK").count(),
            2,
            "the two answers were not two responses: {seen}"
        );
    }

    /// The isolate answering this one hands the core back twice before it says
    /// anything, so the request cannot be finished inside the poll that started
    /// it.
    fn echo_after_two_parks() -> Rc<impl Fn(Request<Incoming>, Origin) -> Reply> {
        Rc::new(|_request: Request<Incoming>, _origin: Origin| {
            let program: Program = Box::new(move |child: &mut Ctx, _args| {
                for _ in 0..2 {
                    // `Yielded` rather than `Parked`: nothing is going to wake
                    // this child, so what it is standing on is the scheduler's
                    // run queue — a request that is *running* and unfinished,
                    // which is the state the service future has to survive.
                    suspend_current(Waiting::Yielded);
                }
                child
                    .write_output(b"answered after two parks")
                    .expect("a buffer");
                Value::null()
            });
            Reply::run(Isolate::new(program, Value::null(), Output::Capture))
        })
    }

    /// A request that is not finished when its poll ends is still answered:
    /// the isolate runs as a **peer task**, the service future answers
    /// `Pending` until [`nvs_host::Running::finished`] says otherwise, and the
    /// child's own end is what wakes this connection back into the poll that
    /// collects it.
    ///
    /// Over a socket rather than against the future directly, because the half
    /// that can break is the wake: a service that answered `Pending` with
    /// nothing arranging a re-poll leaves the connection open and silent, which
    /// is exactly what a client sees here if it regresses. The keep-alive case
    /// below is the other half of the same mechanism — `hyper` skips its
    /// post-response read after a request that parked, so the phase the clock
    /// reads is the loop's to move.
    #[test]
    fn a_request_that_parks_is_answered_when_its_isolate_ends() {
        let mut listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .set_read_timeout(Some(CLIENT_PATIENCE))
                .expect("the socket refused a read timeout");
            socket
                .write_all(b"GET /slow HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .expect("the write failed");
            let mut answer = String::new();
            let read = socket.read_to_string(&mut answer);
            (read.is_ok(), answer)
        });

        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
            serve_on_this_core(
                &mut listener,
                &echo_after_two_parks(),
                Waits::default(),
                &wide_open(),
                &Draining::detached(),
                |_note| {},
                || ControlFlow::Break(()),
            )
            .expect("the accept loop failed");
        });
        nvs_host::run_until_idle(&mut sched).expect("the loop failed");

        let (read, answer) = client.join().expect("the client thread panicked");
        assert!(read, "the connection never answered: {answer}");
        assert!(
            answer.starts_with("HTTP/1.1 200 OK\r\n"),
            "a request that parked did not answer with a response: {answer}"
        );
        assert!(
            answer.ends_with("answered after two parks"),
            "the response did not carry what the isolate echoed after its parks: {answer}"
        );
    }

    /// A request that fails is a **response**, not a dropped connection: ADR
    /// 0006's failure is a value, so a request that gave up still leaves the
    /// connection able to answer, and what it answers is `answer`'s decision —
    /// a status and no body.
    ///
    /// A panic is the failure a test at this level can raise without a
    /// compiler in front of it; `nvs_host::isolate`'s own tests are the home of
    /// why that arrives as `ok = false` rather than as an unwind through the
    /// parent.
    #[test]
    fn a_request_that_fails_is_a_response_and_not_a_dropped_connection() {
        let mut listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .write_all(b"GET /boom HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .expect("the write failed");
            let mut answer = String::new();
            socket
                .read_to_string(&mut answer)
                .expect("the response could not be read");
            answer
        });

        let handler = Rc::new(|_request: Request<Incoming>, _origin: Origin| {
            let program: Program =
                Box::new(|_: &mut Ctx, _args| panic!("the request gave up loudly"));
            Reply::run(Isolate::new(program, Value::null(), Output::Capture))
        });

        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
            serve_on_this_core(
                &mut listener,
                &handler,
                Waits::default(),
                &wide_open(),
                &Draining::detached(),
                |_note| {},
                || ControlFlow::Break(()),
            )
            .expect("the accept loop failed");
        });
        nvs_host::run_until_idle(&mut sched).expect("the loop failed");

        let answer = client.join().expect("the client thread panicked");
        assert!(
            answer.starts_with("HTTP/1.1 500 Internal Server Error\r\n"),
            "a failed request did not answer with a status: {answer}"
        );
        assert!(
            answer.to_ascii_lowercase().contains("content-length: 0"),
            "a failed request answered with a body: {answer}"
        );
    }

    /// `rule:http-server/the-server-block-is-boot-class`'s header wait, as the connection it ends: a peer that opens
    /// a socket and says nothing holds a coroutine to hear nothing, and the
    /// clock is the only thing that can notice. Asserted as a **closed**
    /// connection rather than as a status — [`crate::io`]'s docs § *The clock*
    /// own why a peer that never framed a request is owed no response.
    #[test]
    fn a_connection_that_sends_no_head_is_closed_by_the_header_wait() {
        let mut listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .set_read_timeout(Some(CLIENT_PATIENCE))
                .expect("the socket refused a read timeout");
            let mut seen = String::new();
            (socket.read_to_string(&mut seen).is_ok(), seen)
        });

        let waits = Waits {
            header: Duration::from_millis(60),
            ..Waits::default()
        };
        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
            serve_on_this_core(
                &mut listener,
                &echo_the_path(),
                waits,
                &wide_open(),
                &Draining::detached(),
                |_note| {},
                || ControlFlow::Break(()),
            )
            .expect("the accept loop failed");
        });
        nvs_host::run_until_idle(&mut sched).expect("the loop failed");

        let (closed, seen) = client.join().expect("the client thread panicked");
        assert!(
            closed,
            "a connection that said nothing outlived its header wait"
        );
        assert!(
            seen.is_empty(),
            "a connection that framed no request was answered anyway: {seen}"
        );
    }

    /// The keep-alive wait, which is the one that proves the phase machine
    /// **moves**: this connection is answered under the header wait and then
    /// closed under a different one. The header wait is left far above the
    /// client's own patience on purpose, so a machine stuck in
    /// [`Phase::Head`](crate::io::Phase::Head) fails the assertion rather than
    /// passing it slowly.
    #[test]
    fn an_idle_kept_alive_connection_is_closed_by_the_keepalive_wait() {
        let mut listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .set_read_timeout(Some(CLIENT_PATIENCE))
                .expect("the socket refused a read timeout");
            // No `Connection: close`, so the connection is the server's to keep
            // and the client asks for nothing more.
            socket
                .write_all(b"GET /one HTTP/1.1\r\nHost: localhost\r\n\r\n")
                .expect("the write failed");
            let mut seen = String::new();
            (socket.read_to_string(&mut seen).is_ok(), seen)
        });

        let waits = Waits {
            header: Duration::from_secs(30),
            keepalive: Duration::from_millis(60),
            ..Waits::default()
        };
        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
            serve_on_this_core(
                &mut listener,
                &echo_the_path(),
                waits,
                &wide_open(),
                &Draining::detached(),
                |_note| {},
                || ControlFlow::Break(()),
            )
            .expect("the accept loop failed");
        });
        nvs_host::run_until_idle(&mut sched).expect("the loop failed");

        let (closed, seen) = client.join().expect("the client thread panicked");
        assert!(
            seen.contains("hello /one"),
            "the connection was closed before it answered: {seen}"
        );
        assert!(
            closed,
            "an idle kept-alive connection was still open a header wait later: {seen}"
        );
    }

    /// [`serve_connection`]'s drive, on `ConnectionIo::ending_at_drain`'s
    /// wake: a drain closes a connection that is idle between requests the
    /// moment it sees the drain — not when its keep-alive wait ends, and not
    /// at the drain period's end either. Both are far longer here than the
    /// case is allowed to take, so a drain that waited either one out fails by
    /// the clock rather than passing slowly.
    ///
    /// The drain is the accept loop's own: `keep_serving` breaks after the one
    /// connection, and the loop's tail begins the drain before that connection
    /// has even been answered — so the case also holds that a request already
    /// moving is served to its end and only the idle wait after it is cut.
    #[test]
    fn an_idle_kept_alive_connection_is_closed_when_the_drain_begins() {
        let mut listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .set_read_timeout(Some(CLIENT_PATIENCE))
                .expect("the socket refused a read timeout");
            socket
                .write_all(b"GET /one HTTP/1.1\r\nHost: localhost\r\n\r\n")
                .expect("the write failed");
            let began = Instant::now();
            let mut seen = String::new();
            let closed = socket.read_to_string(&mut seen).is_ok();
            (closed, seen, began.elapsed())
        });

        let waits = Waits {
            header: Duration::from_secs(30),
            keepalive: Duration::from_secs(30),
            drain: Duration::from_secs(30),
            ..Waits::default()
        };
        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
            serve_on_this_core(
                &mut listener,
                &echo_the_path(),
                waits,
                &wide_open(),
                &Draining::detached(),
                |_note| {},
                || ControlFlow::Break(()),
            )
            .expect("the accept loop failed");
        });
        nvs_host::run_until_idle(&mut sched).expect("the loop failed");

        let (closed, seen, took) = client.join().expect("the client thread panicked");
        assert!(
            seen.contains("hello /one"),
            "the request in flight when the drain began was not answered: {seen}"
        );
        assert!(
            closed && took < Duration::from_secs(5),
            "the drain waited out a period on an idle connection: closed after {took:?}"
        );
    }

    /// `rule:concurrency/a-drain-closes-a-connection-cleanly`'s bound on work
    /// in progress, for a request whose program is still running: at the drain
    /// period's end the request tree is cancelled, the client is answered
    /// `503` with `Connection: close`, and the accept loop's drain returns.
    ///
    /// The program sleeps far longer than [`CLIENT_PATIENCE`], so a drain that
    /// waited for it fails by the clock rather than passing slowly. The drain
    /// is the accept loop's own, begun once the one connection is accepted.
    #[test]
    fn a_request_still_running_at_the_drain_periods_end_is_answered_503() {
        let mut listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .set_read_timeout(Some(CLIENT_PATIENCE))
                .expect("the socket refused a read timeout");
            socket
                .write_all(b"GET /slow HTTP/1.1\r\nHost: localhost\r\n\r\n")
                .expect("the write failed");
            let began = Instant::now();
            let mut seen = String::new();
            let closed = socket.read_to_string(&mut seen).is_ok();
            (closed, seen, began.elapsed())
        });

        let sleeps_past_the_drain = Rc::new(|_request: Request<Incoming>, _origin: Origin| {
            let program: Program = Box::new(|_child: &mut Ctx, _args| {
                let _ = nvs_host::sleep(Duration::from_secs(600));
                Value::null()
            });
            Reply::run(Isolate::new(program, Value::null(), Output::Capture))
        });
        let waits = Waits {
            drain: Duration::from_millis(300),
            ..Waits::default()
        };
        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
            serve_on_this_core(
                &mut listener,
                &sleeps_past_the_drain,
                waits,
                &wide_open(),
                &Draining::detached(),
                |_note| {},
                || ControlFlow::Break(()),
            )
            .expect("the accept loop failed");
        });
        nvs_host::run_until_idle(&mut sched).expect("the loop failed");

        let (closed, seen, took) = client.join().expect("the client thread panicked");
        assert!(
            seen.starts_with("HTTP/1.1 503 Service Unavailable\r\n"),
            "a request still running at the drain period's end was not answered 503 \
             (closed {closed} after {took:?}): {seen}"
        );
        assert!(
            seen.to_ascii_lowercase().contains("connection: close"),
            "the 503 did not tell the client the connection closes: {seen}"
        );
        assert!(
            closed && took < Duration::from_secs(5),
            "the drain waited for the running request: closed after {took:?}"
        );
    }

    /// `rule:concurrency/a-drain-closes-a-connection-cleanly`'s bound on work
    /// in progress, for a response already being streamed: at the drain
    /// period's end the connection closes with no terminating chunk, so the
    /// client cannot read the cut body as a whole one.
    ///
    /// The program writes one chunk and then sleeps far longer than
    /// [`CLIENT_PATIENCE`], so a drain that waited for it fails by the clock.
    #[test]
    fn a_response_still_streaming_at_the_drain_periods_end_is_cut_without_its_last_chunk() {
        let mut listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .set_read_timeout(Some(CLIENT_PATIENCE))
                .expect("the socket refused a read timeout");
            socket
                .write_all(b"GET /export HTTP/1.1\r\nHost: localhost\r\n\r\n")
                .expect("the write failed");
            let began = Instant::now();
            let mut seen = Vec::new();
            let mut buffer = [0_u8; 512];
            // An aborted connection may arrive as a reset rather than an end,
            // and both are a close. Only the read timeout is not.
            let closed = loop {
                match socket.read(&mut buffer) {
                    Ok(0) => break true,
                    Ok(read) => seen.extend_from_slice(&buffer[..read]),
                    Err(error) => {
                        break !matches!(
                            error.kind(),
                            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                        );
                    }
                }
            };
            (
                closed,
                String::from_utf8_lossy(&seen).into_owned(),
                began.elapsed(),
            )
        });

        let streams_past_the_drain = Rc::new(|request: Request<Incoming>, _origin: Origin| {
            let inbound =
                nvs_runtime::Inbound::new(request.method().as_str(), request.uri().path(), "");
            let program: Program = Box::new(|child: &mut Ctx, _args| {
                let cell = child
                    .inbound()
                    .and_then(nvs_runtime::Inbound::response_stream_slot)
                    .cloned()
                    .expect("a served request was offered no response-stream cell");
                let emit = cell
                    .open("text/plain", None, Vec::new())
                    .expect("the first stream on a request opens");
                child.set_body_stream(emit);
                child
                    .body_stream()
                    .expect("the writing half was just set")
                    .send(b"opened;".to_vec())
                    .expect("the first chunk goes into an empty cell");
                let _ = nvs_host::sleep(Duration::from_secs(600));
                Value::null()
            });
            Reply::Run(
                Isolate::new(program, Value::null(), Output::Capture).answering(inbound),
                None,
            )
        });
        let waits = Waits {
            drain: Duration::from_millis(300),
            ..Waits::default()
        };
        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
            serve_on_this_core(
                &mut listener,
                &streams_past_the_drain,
                waits,
                &wide_open(),
                &Draining::detached(),
                |_note| {},
                || ControlFlow::Break(()),
            )
            .expect("the accept loop failed");
        });
        nvs_host::run_until_idle(&mut sched).expect("the loop failed");

        let (closed, seen, took) = client.join().expect("the client thread panicked");
        assert!(
            seen.starts_with("HTTP/1.1 200 OK\r\n") && seen.contains("opened;"),
            "the stream's head and first chunk were not written before the cut: {seen}"
        );
        assert!(
            !seen.ends_with("0\r\n\r\n"),
            "a stream cut by the drain ended with its terminating chunk: {seen}"
        );
        assert!(
            closed && took < Duration::from_secs(5),
            "the drain waited for the streaming request: closed after {took:?}"
        );
    }

    /// `rule:concurrency/a-drain-closes-a-connection-cleanly`'s bound on work
    /// in progress, for a whole response still being written: a client that
    /// reads it slowly but steadily does not keep the accept loop's drain open
    /// past the drain period.
    ///
    /// The body is far larger than the loopback's socket buffers, and the
    /// client reads a little at a time, so every read restarts the connection's
    /// idle wait. What is timed is the server's drain and not what the client
    /// reads, because the kernel still sends what it was given after the
    /// connection closes. A platform whose socket takes the whole body in one
    /// write, as Windows does, has nothing left to cut.
    ///
    /// This is the outcome, and `crate::io`'s tests are the bound: `hyper`
    /// reads while the write is parked, which ends the response phase early
    /// (`crate::io::ConnectionIo::closing`), so this case cannot tell which of
    /// the two waits the drain ended.
    #[test]
    fn a_whole_response_still_being_written_at_the_drain_periods_end_is_cut() {
        const WHOLE: usize = 32 * 1024 * 1024;
        let mut listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let served = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let client = std::thread::spawn({
            let served = Arc::clone(&served);
            move || {
                let mut socket =
                    TcpStream::connect(addr).expect("the loopback refused a connection");
                socket
                    .set_read_timeout(Some(CLIENT_PATIENCE))
                    .expect("the socket refused a read timeout");
                socket
                    .write_all(b"GET /export HTTP/1.1\r\nHost: localhost\r\n\r\n")
                    .expect("the write failed");
                let began = Instant::now();
                let mut head = Vec::new();
                let mut buffer = [0_u8; 4096];
                // Slowly, until the server's drain has ended or the client
                // runs out of patience. Closing the socket then is what ends a
                // write the drain did not end.
                while !served.load(std::sync::atomic::Ordering::Acquire)
                    && began.elapsed() < CLIENT_PATIENCE
                {
                    match socket.read(&mut buffer) {
                        Ok(0) | Err(_) => break,
                        Ok(read) => {
                            if head.len() < 512 {
                                head.extend_from_slice(&buffer[..read]);
                            }
                            std::thread::sleep(Duration::from_millis(10));
                        }
                    }
                }
                String::from_utf8_lossy(&head).into_owned()
            }
        });

        let answers_with_a_large_body = Rc::new(|_request: Request<Incoming>, _origin: Origin| {
            Reply::Done(Response::new(Answer::new(Bytes::from(vec![b'x'; WHOLE]))))
        });
        let waits = Waits {
            drain: Duration::from_millis(300),
            ..Waits::default()
        };
        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
            serve_on_this_core(
                &mut listener,
                &answers_with_a_large_body,
                waits,
                &wide_open(),
                &Draining::detached(),
                |_note| {},
                || ControlFlow::Break(()),
            )
            .expect("the accept loop failed");
        });
        let began = Instant::now();
        run_the_core(&mut sched);
        let took = began.elapsed();
        served.store(true, std::sync::atomic::Ordering::Release);

        let head = client.join().expect("the client thread panicked");
        assert!(
            head.starts_with("HTTP/1.1 200 OK\r\n"),
            "the response's head was not written before the cut: {head}"
        );
        assert!(
            took < Duration::from_secs(5),
            "the drain waited for the response being written: it ended after {took:?}"
        );
    }

    /// `rule:http-server/the-server-block-is-boot-class`'s valve, and the load-bearing half of it is the **order**:
    /// the handler is never asked, so no mount was selected, no isolate was
    /// allocated and no Novis code ran for a request the ceiling refused. A
    /// case that only asserted the `503` would pass just as well against a cap
    /// that allocated in order to refuse, which is the cap § 5 says does not
    /// protect what it exists to protect.
    #[test]
    fn an_over_capacity_request_is_refused_with_503_before_it_is_allocated() {
        let mut listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let admission = Arc::new(Admission::new(&Ceiling::of(&Capacity {
            configured: 1,
            per_request: None,
            budget: None,
        })));
        // The same count the loop is given, held here so the guard below can
        // outlive the handle the accept task takes.
        let serving = Serving::new(
            Arc::clone(&admission),
            Arc::new(Secure::default()),
            Arc::new(Trusted::none()),
            Arc::new(Cors::default()),
            Arc::default(),
        );
        // The one place this valve has, taken and held for the whole run: the
        // request below therefore arrives *at* the ceiling, which is the state
        // the assertions are about and not a race to reproduce.
        let _at_the_ceiling = admission.admit().expect("a fresh valve refused");

        // Set by the handler, and the point of the case is that it stays false.
        let asked = Rc::new(Cell::new(false));
        let handler = Rc::new({
            let asked = Rc::clone(&asked);
            move |_request: Request<Incoming>, _origin: Origin| {
                asked.set(true);
                Reply::not_found()
            }
        });

        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .set_read_timeout(Some(CLIENT_PATIENCE))
                .expect("the socket refused a read timeout");
            socket
                .write_all(b"GET /busy HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .expect("the write failed");
            let mut answer = String::new();
            socket
                .read_to_string(&mut answer)
                .expect("the response could not be read");
            answer
        });

        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
            serve_on_this_core(
                &mut listener,
                &handler,
                Waits::default(),
                &serving,
                &Draining::detached(),
                |_note| {},
                || ControlFlow::Break(()),
            )
            .expect("the accept loop failed");
        });
        nvs_host::run_until_idle(&mut sched).expect("the loop failed");

        let answer = client.join().expect("the client thread panicked");
        assert!(
            answer.starts_with("HTTP/1.1 503 Service Unavailable\r\n"),
            "a request at the ceiling was not refused: {answer}"
        );
        assert!(
            answer.to_ascii_lowercase().contains("retry-after: 1"),
            "the refusal gave a proxy nothing to fail over on: {answer}"
        );
        assert!(
            !asked.get(),
            "the handler was asked for a request the ceiling had already refused"
        );
    }

    /// One worker under a valve the whole fleet shares: this core's listener,
    /// the connections it accepts before its loop's tail, and a report on every
    /// request the handler is *asked* for — which is exactly the requests the
    /// ceiling admitted, because the valve is asked before the handler is.
    ///
    /// The program reads its request's body to the end, so a client still owing
    /// bytes parks the run there: a place is taken before the handler and given
    /// back when the answer exists, and a body that has not all arrived is what
    /// holds one open for as long as a test needs it.
    fn one_admitting_core(
        listener: std::net::TcpListener,
        serving: Serving,
        connections: usize,
        admitted: std::sync::mpsc::Sender<()>,
    ) -> impl FnOnce(&mut nvs_host::Scheduler) + Send + 'static {
        move |sched| {
            let mut listener =
                NvsListener::from_std(listener).expect("the OS refused a non-blocking listener");
            let _installed = nvs_host::reactor::install(
                nvs_host::Reactor::new().expect("the OS refused a poll"),
            );
            let handler = Rc::new(move |request: Request<Incoming>, _origin: Origin| {
                let _ = admitted.send(());
                let (inbound, supply) = carrying(request);
                let program: Program = Box::new(|run: &mut Ctx, _args| {
                    if let Some(inbound) = run.inbound_mut()
                        && let Some(body) = inbound.body()
                    {
                        while let Ok(Some(_chunk)) = body.next_chunk() {}
                    }
                    run.write_output(b"served").expect("a buffer");
                    Value::null()
                });
                Reply::Run(
                    Isolate::new(program, Value::null(), Output::Capture).answering(inbound),
                    supply,
                )
            });
            sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
                let left = Cell::new(connections);
                serve_on_this_core(
                    &mut listener,
                    &handler,
                    Waits::default(),
                    &serving,
                    &Draining::detached(),
                    |_note| {},
                    || {
                        left.set(left.get() - 1);
                        if left.get() == 0 {
                            ControlFlow::Break(())
                        } else {
                            ControlFlow::Continue(())
                        }
                    },
                )
                .expect("the accept loop failed");
            });
            run_the_core(sched);
        }
    }

    /// `rule:http-server/admission-is-arithmetic-not-a-number`'s counter is
    /// **one relaxed atomic for the process**, so what it guards is the fleet's
    /// ceiling and never a share of it handed out per core.
    ///
    /// Both halves are asserted here, and the first is the one a per-core
    /// counter would pass on its own: one core takes *both* of the two places
    /// while its neighbour has served nothing, which a ceiling divided by the
    /// core count would have refused. Only then does a request on that idle
    /// core meet the `503` — a core with nothing in flight of its own can be
    /// refusing against no count but the one the other core filled.
    ///
    /// Two places rather than more because two is the smallest ceiling that can
    /// tell those two shapes apart, and each is held by a body three bytes
    /// short: nothing here waits on a timing window.
    #[test]
    fn the_in_flight_ceiling_is_fleet_wide_so_a_hot_core_cannot_refuse_while_neighbours_idle() {
        /// A request whose body is three bytes short, so its run parks holding
        /// its place in the count until the rest of the body is written.
        fn holding(addr: std::net::SocketAddr) -> TcpStream {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .set_read_timeout(Some(CLIENT_PATIENCE))
                .expect("the socket refused a read timeout");
            socket
                .write_all(
                    b"POST /hot HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\
                      Content-Length: 5\r\n\r\nab",
                )
                .expect("the write failed");
            socket
        }

        let cpus = nvs_host::cpus();
        let Some(first_cpu) = cpus.first().copied() else {
            // A host that enumerates no CPU is served from the boot thread, so
            // there is no second core there to share a count with.
            return;
        };
        let second_cpu = cpus.get(1).copied().unwrap_or(first_cpu);

        let admission = Arc::new(Admission::new(&Ceiling::of(&Capacity {
            configured: 2,
            per_request: None,
            budget: None,
        })));
        // One valve, cloned into both workers, which is what `nvs serve` hands
        // a fleet: the `Arc` is the count, and a clone of it is not a second.
        let serving = Serving::new(
            Arc::clone(&admission),
            Arc::new(Secure::default()),
            Arc::new(Trusted::none()),
            Arc::new(Cors::default()),
            Arc::default(),
        );

        let hot_socket = std::net::TcpListener::bind("127.0.0.1:0").expect("the OS refused a port");
        let hot_addr = hot_socket
            .local_addr()
            .expect("a bound listener had no address");
        let idle_socket =
            std::net::TcpListener::bind("127.0.0.1:0").expect("the OS refused a port");
        let idle_addr = idle_socket
            .local_addr()
            .expect("a bound listener had no address");

        let (hot_asked, hot_admitted) = std::sync::mpsc::channel();
        let (idle_asked, idle_admitted) = std::sync::mpsc::channel();

        let hot_worker = nvs_host::Worker::spawn(
            first_cpu,
            one_admitting_core(hot_socket, serving.clone(), 2, hot_asked),
        )
        .expect("the OS refused a worker thread");
        let idle_worker = nvs_host::Worker::spawn(
            second_cpu,
            one_admitting_core(idle_socket, serving, 1, idle_asked),
        )
        .expect("the OS refused a worker thread");

        let mut first = holding(hot_addr);
        hot_admitted
            .recv_timeout(CLIENT_PATIENCE)
            .expect("the hot core never admitted the first request of the fleet's two");
        let mut second = holding(hot_addr);
        hot_admitted.recv_timeout(CLIENT_PATIENCE).expect(
            "one core did not get both of the fleet's places, so the ceiling had been divided \
             per core and a hot core refuses while its neighbours idle",
        );

        let mut refused = TcpStream::connect(idle_addr).expect("the loopback refused a connection");
        refused
            .set_read_timeout(Some(CLIENT_PATIENCE))
            .expect("the socket refused a read timeout");
        refused
            .write_all(b"GET /neighbour HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
            .expect("the write failed");
        let mut answer = String::new();
        refused
            .read_to_string(&mut answer)
            .expect("the response could not be read");
        assert!(
            answer.starts_with("HTTP/1.1 503 Service Unavailable\r\n"),
            "a core answered a request the fleet had no place left for, so it was counting \
             against a ceiling of its own: {answer}"
        );
        // `Ok` is the one answer that fails. The sender lives in the idle
        // core's handler, and that core serves one connection, so by the time
        // the `503` has been read to its end the core may already have left
        // its loop and dropped it: a disconnected channel says the handler is
        // gone, not that it was asked.
        assert!(
            idle_admitted.try_recv().is_err(),
            "the handler was asked for a request the fleet's ceiling had already refused"
        );
        assert_eq!(
            admission.in_flight(),
            2,
            "the fleet's count is not the two places its one hot core is holding"
        );

        // The bodies both runs are still owed: each answer is one place given
        // back, and a core's loop returns once its own tally reaches zero.
        for (which, socket) in [("first", &mut first), ("second", &mut second)] {
            socket
                .write_all(b"cde")
                .expect("the rest of the body could not be written");
            let mut answer = String::new();
            socket
                .read_to_string(&mut answer)
                .expect("the response could not be read");
            assert!(
                answer.starts_with("HTTP/1.1 200 OK\r\n"),
                "the hot core's {which} request was admitted and then not answered: {answer}"
            );
        }
        drop(first);
        drop(second);
        drop(refused);
        hot_worker.join().expect("the hot core panicked");
        idle_worker.join().expect("the idle core panicked");
    }

    /// One core that registers with the fleet's watchdog and then keeps
    /// serving: the control the case needs, because "the report named the
    /// wedged core" says nothing unless a second core was watched under the
    /// same watchdog and went unreported.
    ///
    /// It registers itself, from its own thread and right after installing its
    /// reactor, which is where a `DeadlineView` comes from and what
    /// `nvs-cli`'s `serve_on_worker` does at the same point.
    fn one_watched_core(
        watchdog: Arc<nvs_host::Watchdog>,
        cpu: nvs_host::CpuId,
        listener: std::net::TcpListener,
        connections: usize,
    ) -> impl FnOnce(&mut nvs_host::Scheduler) + Send + 'static {
        move |sched| {
            let mut listener =
                NvsListener::from_std(listener).expect("the OS refused a non-blocking listener");
            let _installed = nvs_host::reactor::install(
                nvs_host::Reactor::new().expect("the OS refused a poll"),
            );
            let _watched = watchdog.register(
                cpu,
                nvs_host::reactor::with_current(|reactor| reactor.deadline_view())
                    .expect("the reactor was installed on the line above"),
            );
            let handler = Rc::new(move |request: Request<Incoming>, _origin: Origin| {
                let (inbound, supply) = carrying(request);
                let program: Program = Box::new(|run: &mut Ctx, _args| {
                    run.write_output(b"served").expect("a buffer");
                    Value::null()
                });
                Reply::Run(
                    Isolate::new(program, Value::null(), Output::Capture).answering(inbound),
                    supply,
                )
            });
            sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
                let left = Cell::new(connections);
                serve_on_this_core(
                    &mut listener,
                    &handler,
                    Waits::default(),
                    &wide_open(),
                    &Draining::detached(),
                    |_note| {},
                    || {
                        left.set(left.get() - 1);
                        if left.get() == 0 {
                            ControlFlow::Break(())
                        } else {
                            ControlFlow::Continue(())
                        }
                    },
                )
                .expect("the accept loop failed");
            });
            run_the_core(sched);
        }
    }

    /// One core that arms a deadline and then stops turning, which is the fault
    /// `rule:http-server/a-wedged-core-is-detected-by-its-deadline` exists for
    /// and the one no other bound in this file answers.
    ///
    /// The wedge is a blocking read on its task's own stack: a coroutine that
    /// does not yield is a thread that does not poll, so the deadline it filed
    /// one line earlier stays published and no later turn takes it back. That
    /// is a state rather than a race — the core is held there until the test
    /// drops the other end.
    fn one_wedged_core(
        watchdog: Arc<nvs_host::Watchdog>,
        cpu: nvs_host::CpuId,
        wedged: std::sync::mpsc::Sender<()>,
        released: std::sync::mpsc::Receiver<()>,
    ) -> impl FnOnce(&mut nvs_host::Scheduler) + Send + 'static {
        move |sched| {
            let _installed = nvs_host::reactor::install(
                nvs_host::Reactor::new().expect("the OS refused a poll"),
            );
            let _watched = watchdog.register(
                cpu,
                nvs_host::reactor::with_current(|reactor| reactor.deadline_view())
                    .expect("the reactor was installed on the line above"),
            );
            sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
                let id = nvs_host::current_task().expect("a task always runs on a core");
                nvs_host::reactor::with_current(|reactor| {
                    reactor.timers().arm(id, Instant::now());
                })
                .expect("the reactor was installed above");
                let _ = wedged.send(());
                let _ = released.recv();
            });
            run_the_core(sched);
        }
    }

    /// `rule:http-server/a-wedged-core-is-detected-by-its-deadline`'s detector
    /// is **one watchdog for the process with one entry per running core**, so
    /// what it reports is a core rather than the fleet.
    ///
    /// Both halves are asserted against two workers registered with the same
    /// watchdog. One arms a deadline and stops turning; the report names that
    /// core's CPU and no second report arrives for the neighbour, which is
    /// answering its deadlines and is watched by the same thread. The
    /// neighbour keeps serving throughout — before the report and after it —
    /// which is the other half: `rule:http-server/a-wedged-core-is-shed-never-killed`
    /// bounds firing to a record, so a stall must cost the fleet a core and
    /// never the process.
    #[test]
    fn the_watchdog_fires_per_worker_and_a_stalled_core_does_not_stall_the_fleet() {
        /// Short enough that this case is not a wait, and still far longer than
        /// the gap between two of a turning core's polls.
        const MARGIN: Duration = Duration::from_millis(150);
        /// Read every core this often, which sets the detection latency and
        /// nothing else.
        const SWEEP: Duration = Duration::from_millis(10);

        /// One request on its own connection, answered in full.
        fn ask(addr: std::net::SocketAddr, path: &str) -> String {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .set_read_timeout(Some(CLIENT_PATIENCE))
                .expect("the socket refused a read timeout");
            socket
                .write_all(
                    format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                        .as_bytes(),
                )
                .expect("the write failed");
            let mut answer = String::new();
            socket
                .read_to_string(&mut answer)
                .expect("the response could not be read");
            answer
        }

        let cpus = nvs_host::cpus();
        let (Some(serving_cpu), Some(wedged_cpu)) = (cpus.first().copied(), cpus.get(1).copied())
        else {
            // A report carries the CPU its core was pinned to and no other
            // name, so on a host with fewer than two both workers would
            // register under one and "which core was reported" is not a
            // question this fixture could ask.
            return;
        };

        let (reported, stalls) = std::sync::mpsc::channel();
        // A sink of its own rather than `Watchdog::new`'s floor, because what
        // is under test is which core is reported and how long it took, and
        // both of those are the record's fields rather than its wording.
        let watchdog = Arc::new(nvs_host::Watchdog::with(MARGIN, SWEEP, move |stall| {
            let _ = reported.send(*stall);
        }));

        let socket = std::net::TcpListener::bind("127.0.0.1:0").expect("the OS refused a port");
        let addr = socket
            .local_addr()
            .expect("a bound listener had no address");
        let (wedged, is_wedged) = std::sync::mpsc::channel();
        let (release, released) = std::sync::mpsc::channel();

        let serving_worker = nvs_host::Worker::spawn(
            serving_cpu,
            one_watched_core(Arc::clone(&watchdog), serving_cpu, socket, 2),
        )
        .expect("the OS refused a worker thread");
        let wedged_worker = nvs_host::Worker::spawn(
            wedged_cpu,
            one_wedged_core(Arc::clone(&watchdog), wedged_cpu, wedged, released),
        )
        .expect("the OS refused a worker thread");

        is_wedged
            .recv_timeout(CLIENT_PATIENCE)
            .expect("the second core never reached the deadline it was to stop turning on");
        let answer = ask(addr, "/while-wedged");
        assert!(
            answer.starts_with("HTTP/1.1 200 OK\r\n"),
            "a core that was not the wedged one stopped answering, so a stall costs the fleet \
             rather than a core: {answer}"
        );
        assert_eq!(
            watchdog.watching(),
            2,
            "the fleet is not watched one entry per running core"
        );

        let stall = stalls
            .recv_timeout(CLIENT_PATIENCE)
            .expect("a core that had stopped turning was never reported");
        assert_eq!(
            stall.cpu.raw(),
            wedged_cpu.raw(),
            "the report named a core that was answering its deadlines"
        );
        assert!(
            stall.overdue_by >= MARGIN,
            "a core was reported before it was overdue by the margin: {:?}",
            stall.overdue_by
        );
        assert!(
            matches!(
                stalls.recv_timeout(MARGIN * 3),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout)
            ),
            "a second report arrived: either the turning core was reported as wedged, or one \
             episode was reported once per sweep"
        );

        let after = ask(addr, "/after-the-report");
        assert!(
            after.starts_with("HTTP/1.1 200 OK\r\n"),
            "firing did more than report: the neighbouring core stopped serving too: {after}"
        );

        // The wedge is held until here on purpose: every assertion above is
        // made while the core really is stuck, and dropping the sender is what
        // lets its turn end.
        drop(release);
        wedged_worker.join().expect("the wedged core panicked");
        serving_worker.join().expect("the serving core panicked");
    }

    /// The condition, spelled the way the OS spells it on this platform.
    fn exhausted() -> io::Error {
        io::Error::from_raw_os_error(EXHAUSTED[0])
    }

    /// `rule:http-server/the-accept-loop-backs-off` asks for a backoff on descriptor exhaustion and on nothing
    /// else, so the bound is asserted on both sides: the error the loop must
    /// wait out, and a neighbouring `accept` failure it must still end on. A
    /// backoff that widened to every error would look right against the first
    /// half alone, and would turn a listener whose socket died into a process
    /// that sleeps forever instead of reporting.
    #[test]
    fn descriptor_exhaustion_is_waited_out_and_no_other_failure_is() {
        let now = Instant::now();
        let mut backoff = AcceptBackoff::default();

        let (wait, note) = backoff
            .after(&exhausted(), now)
            .expect("descriptor exhaustion was not recognised");
        assert_eq!(wait, FIRST_WAIT, "the first wait is the short one");
        assert!(note.is_some(), "the first exhaustion went unreported");

        for other in [
            io::Error::from(io::ErrorKind::ConnectionAborted),
            io::Error::from(io::ErrorKind::PermissionDenied),
            io::Error::other("the socket is gone"),
        ] {
            assert!(
                backoff.after(&other, now).is_none(),
                "the loop would have slept on an error it must end on: {other}"
            );
        }
    }

    /// The wait doubles to a ceiling and no further, and an accept puts it
    /// back. Asserted by walking the whole episode rather than by reading one
    /// step off, because a backoff that doubled without a bound and one that
    /// never left its first wait both answer plausibly at any single step.
    #[test]
    fn the_wait_doubles_to_a_ceiling_and_an_accept_puts_it_back() {
        let now = Instant::now();
        let mut backoff = AcceptBackoff::default();
        let mut waits = Vec::new();
        for _ in 0..12 {
            let (wait, _) = backoff.after(&exhausted(), now).expect("still exhausted");
            waits.push(wait);
        }

        assert_eq!(waits[0], FIRST_WAIT, "the episode opened at the wrong wait");
        assert!(
            waits.windows(2).all(|pair| pair[1] >= pair[0]),
            "a wait went backwards inside one episode: {waits:?}"
        );
        assert!(
            waits.iter().all(|wait| *wait <= LONGEST_WAIT),
            "the backoff went past its ceiling: {waits:?}"
        );
        assert_eq!(
            waits.last(),
            Some(&LONGEST_WAIT),
            "twelve doublings did not reach the ceiling: {waits:?}"
        );

        backoff.accepted();
        let (wait, _) = backoff.after(&exhausted(), now).expect("still exhausted");
        assert_eq!(
            wait, FIRST_WAIT,
            "an accepted connection did not end the episode"
        );
    }

    /// § 8's second half: once per window, not once per attempt — the reason
    /// the section gives for the wait is the disk it would otherwise fill.
    /// Asserted by **counting** the notes over a long episode, because a
    /// backoff that reports every time still produces a correct-looking line at
    /// each individual attempt.
    #[test]
    fn the_note_is_once_per_window_and_not_once_per_attempt() {
        let opened = Instant::now();
        let mut backoff = AcceptBackoff::default();
        let inside = (0..200)
            .filter_map(|attempt| {
                let now = opened + REPORT_WINDOW / 400 * attempt;
                backoff.after(&exhausted(), now).expect("still exhausted").1
            })
            .count();
        assert_eq!(inside, 1, "the window reported {inside} times");

        let (_, next) = backoff
            .after(&exhausted(), opened + REPORT_WINDOW)
            .expect("still exhausted");
        let note = next.expect("the next window went unreported");
        assert!(
            note.contains("file descriptors"),
            "the note does not say what the condition is: {note}"
        );

        // A recovery closes the window with it: the next episode is a new fact
        // and reports at once, rather than being swallowed by a window the
        // previous one opened.
        backoff.accepted();
        let (_, reopened) = backoff
            .after(&exhausted(), opened + REPORT_WINDOW)
            .expect("still exhausted");
        assert!(
            reopened.is_some(),
            "an episode after a recovery was swallowed by the previous window"
        );
    }

    /// A backoff is **one core's** episode: the state lives in
    /// [`serve_on_this_core`]'s own frame, so a fleet holds one per worker and a
    /// shortage on one core is not something another core has to live inside
    /// (`rule:http-server/the-accept-fan-out-is-one-worker-per-core`, which
    /// gives a core its accept loop and the backoff that loop applies).
    ///
    /// Three assertions, because a single fleet-wide value would look right at
    /// any one of them alone: a neighbour's first exhaustion is still its
    /// *first* wait however deep another core's episode has gone, its own line
    /// is not swallowed by a window another core opened, and a neighbour
    /// recovering does not end the episode of the core that is still short. The
    /// wait itself costs the neighbour nothing on top of that, because a core
    /// parks on [`nvs_host::sleep`] on its own task rather than blocking its
    /// thread — the accept loop's own line, and the reason a shortage is not
    /// paid for by the connections already accepted.
    #[test]
    fn the_accept_backoff_runs_per_core_and_one_cores_backoff_does_not_stall_another() {
        let opened = Instant::now();
        // Every step inside one report window, so what silences a line is only
        // ever the window and never the clock.
        let step = |attempt| opened + REPORT_WINDOW / 400 * attempt;
        let mut short = AcceptBackoff::default();
        let mut neighbour = AcceptBackoff::default();

        // One core walks a whole episode down to its ceiling.
        let mut episode = Vec::new();
        for attempt in 0..12 {
            let (wait, note) = short
                .after(&exhausted(), step(attempt))
                .expect("still exhausted");
            episode.push((wait, note.is_some()));
        }
        assert_eq!(
            episode.last().map(|(wait, _)| *wait),
            Some(LONGEST_WAIT),
            "the episode never reached the ceiling this case is about: {episode:?}"
        );
        assert_eq!(
            episode.iter().filter(|(_, reported)| *reported).count(),
            1,
            "one core's episode reported more than once inside one window: {episode:?}"
        );

        let (wait, note) = neighbour
            .after(&exhausted(), step(12))
            .expect("still exhausted");
        assert_eq!(
            wait, FIRST_WAIT,
            "a core inherited a wait from an episode that was not its own, so one core's shortage \
             would delay every other core's next accept"
        );
        assert!(
            note.is_some(),
            "a core's first exhaustion was swallowed by a window another core had opened, so a \
             shortage could reach the whole fleet and be reported by one of them"
        );

        // And the same fact from the other side: a core accepting again says
        // nothing about a core that is still out of descriptors.
        neighbour.accepted();
        let (wait, _) = short
            .after(&exhausted(), step(13))
            .expect("still exhausted");
        assert_eq!(
            wait, LONGEST_WAIT,
            "one core's accepted connection ended another core's episode, so a shortage would be \
             retried at full speed on the core still inside it"
        );
    }

    /// A bound listener and a client that asks for `path` once and reads until
    /// the server closes, which `Connection: close` is what makes it do.
    fn one_get(path: &str) -> (NvsListener, std::thread::JoinHandle<String>) {
        let listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");
        let asked = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");

        let client = std::thread::spawn(move || {
            let mut socket = TcpStream::connect(addr).expect("the loopback refused a connection");
            socket
                .write_all(asked.as_bytes())
                .expect("the write failed");
            let mut answer = String::new();
            socket
                .read_to_string(&mut answer)
                .expect("the response could not be read");
            answer
        });
        (listener, client)
    }

    /// Answers one request with `body` and gives back what the client read.
    ///
    /// The body is built by the caller on **this** thread, because neither half
    /// of a stream's cell is `Send` and the accept loop runs on this thread
    /// too — the client is the only thread here, and it holds a socket.
    fn answered_with(path: &str, body: Answer) -> String {
        let (listener, client) = one_get(path);
        let held = RefCell::new(Some(body));
        let handler = Rc::new(move |_request: Request<Incoming>, _origin: Origin| {
            Reply::Done(Response::new(
                held.borrow_mut()
                    .take()
                    .expect("the case sends one request"),
            ))
        });
        served_by(listener, &handler, client)
    }

    /// The whole arm, unchanged by the streaming one existing: an exact
    /// [`Body::size_hint`] is what makes `hyper` frame a response with a
    /// `Content-Length`.
    #[test]
    fn a_whole_body_reports_an_exact_size_and_is_sent_with_content_length() {
        let body = Answer::new(Bytes::from_static(b"twelve bytes"));
        assert_eq!(
            body.size_hint().exact(),
            Some(12),
            "a body this server holds whole did not report the length it knows"
        );

        let answer = answered_with("/whole", body).to_ascii_lowercase();
        assert!(
            answer.contains("content-length: 12"),
            "an answer whose length was known was not sent with one: {answer}"
        );
        assert!(
            !answer.contains("transfer-encoding"),
            "a body of known length was chunked: {answer}"
        );
        assert!(
            answer.ends_with("twelve bytes"),
            "the response did not carry the body: {answer}"
        );
    }

    /// The streaming arm, against the same client: no exact size, so `hyper`
    /// chunks it, and the chunk framing is the proof rather than the header
    /// alone.
    ///
    /// The writer here finishes before the response is answered, which is a
    /// stream with nothing to wait for — the parking half is
    /// [`nvs_runtime::stream`]'s own to prove, and what this case is about is
    /// the framing a body with no length gets.
    #[test]
    fn a_streaming_body_reports_no_exact_size_and_is_chunked() {
        let (mut emit, body) = Answer::stream(&crate::bounds::Connection::default());
        emit.send(b"over time".to_vec()).expect("the cell is empty");
        drop(emit);
        assert_eq!(
            body.size_hint().exact(),
            None,
            "a body still being written reported a length nobody knows yet"
        );

        let answer = answered_with("/stream", body);
        assert!(
            answer
                .to_ascii_lowercase()
                .contains("transfer-encoding: chunked"),
            "a body of unknown length was not chunked: {answer}"
        );
        assert!(
            !answer.to_ascii_lowercase().contains("content-length"),
            "a body of unknown length was sent with a length: {answer}"
        );
        assert!(
            answer.ends_with("9\r\nover time\r\n0\r\n\r\n"),
            "the chunk the writer sent was not framed as one: {answer}"
        );
    }

    /// [`crate::statics`]'s cases are assertions about this method, so the arm
    /// it reads has to stay the one it always read. A stream answers empty
    /// rather than a chunk in flight: the bytes are the connection's to frame
    /// once, and a reader here would take them out of the response.
    #[test]
    fn answer_bytes_still_answers_the_whole_variants_buffer() {
        assert_eq!(
            Answer::new(Bytes::from_static(b"held whole")).bytes(),
            b"held whole"
        );
        assert!(
            Answer::empty().bytes().is_empty(),
            "a body with nothing in it answered bytes"
        );

        let (mut emit, streaming) = Answer::stream(&crate::bounds::Connection::default());
        emit.send(b"in flight".to_vec()).expect("the cell is empty");
        assert!(
            streaming.bytes().is_empty(),
            "a streaming body answered bytes only the connection may take"
        );
    }

    /// `rule:concurrency/connection-bounds-are-finite`'s send bound, on the
    /// body direction: a peer that has stopped reading is a bound met and a
    /// defined close, not a writer parked for as long as the client cares to
    /// leave it.
    ///
    /// The [`Answer`] here is the stalled reader — it holds the draining half
    /// and nothing ever polls it — and the writer runs as a real task, so the
    /// wait is the scheduler's own park against the deadline this connection's
    /// bounds set.
    #[test]
    fn a_stalled_reader_closes_the_stream_at_the_send_timeout_and_reports_as_that() {
        let bounds = crate::bounds::Connection {
            send: Duration::from_millis(5),
            ..crate::bounds::Connection::default()
        };
        let reported = Rc::new(RefCell::new(None));
        let told = Rc::clone(&reported);

        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
            let (mut emit, stalled) = Answer::stream(&bounds);
            emit.send(b"first".to_vec()).expect("the cell is empty");
            *told.borrow_mut() = Some(
                emit.send(b"second".to_vec())
                    .expect_err("nothing has taken the first chunk"),
            );
            drop(stalled);
        });
        nvs_host::run_until_idle(&mut sched).expect("the loop failed");

        assert_eq!(
            reported.borrow().as_deref(),
            Some(nvs_runtime::stream::SEND_TIMED_OUT),
            "a write nothing was reading did not end as the bound it met"
        );
    }

    /// The bounds an event stream opens under, at the wait a server actually
    /// serves under: every case below asserts a bound of `bounds` and never the
    /// keep-alive, so the beat is derived from the default and is far away.
    fn an_event_stream_over(bounds: &crate::bounds::Connection) -> (stream::Emit, Answer) {
        an_event_stream_draining(bounds, &Draining::detached())
    }

    /// [`an_event_stream_over`], on a named drain rather than one nothing ever
    /// begins — a server draining beside the process's own is
    /// [`Draining::detached`]'s whole case.
    fn an_event_stream_draining(
        bounds: &crate::bounds::Connection,
        draining: &Draining,
    ) -> (stream::Emit, Answer) {
        let (emit, body) = Answer::stream(bounds);
        let body = body.as_an_event_stream(
            bounds,
            draining,
            Waits::default().write_idle,
            Rc::new(Cell::new(None)),
        );
        (emit, body)
    }

    /// The frames one poll pass takes off a body, until it parks or ends.
    ///
    /// A pass and not a loop, because every case below asserts what a *poll*
    /// answered: the keep-alive, the opening hint and both closes are decided
    /// there, so a helper that drove the body to exhaustion would be asserting
    /// the sum of several polls.
    /// Takes the reconnection block an event stream opens with, so that a case
    /// about a bound is not also a case about the hint.
    fn past_the_opening(body: &mut Answer) {
        let Poll::Ready(Some(hint)) = polled(body) else {
            panic!("an event stream's first frame is its reconnection hint")
        };
        assert!(
            hint.starts_with(b"retry: "),
            "a stream opened with something other than its hint: {hint:?}"
        );
    }

    fn polled(body: &mut Answer) -> Poll<Option<Bytes>> {
        let mut cx = Context::from_waker(std::task::Waker::noop());
        Pin::new(body).poll_frame(&mut cx).map(|frame| {
            frame.map(|frame| {
                frame
                    .expect("this body never fails")
                    .into_data()
                    .expect("a response body carries data frames and no trailers")
            })
        })
    }

    /// `rule:concurrency/connection-bounds-are-finite`'s table read for what is
    /// *not* in it: **`Connection::idle` is the one bound an event stream does
    /// not arm**, and a stream whose idle window is long past goes on being
    /// written.
    ///
    /// The window here is nothing at all, so anything that armed it would have
    /// met it before this first poll. What the body answers instead is
    /// `Pending` — nothing to send, and no beat owed for half a response wait —
    /// which is a stream still open. `crate::bounds::EventStream` owns why the
    /// absence is the design: that bound closes a connection whose peer stopped
    /// speaking, and this door's peer never speaks, so arming it would close
    /// every healthy stream at the first window.
    #[test]
    fn the_idle_bound_is_not_armed_for_an_event_stream() {
        let (_emit, mut body) = an_event_stream_over(&crate::bounds::Connection {
            idle: Duration::ZERO,
            ..crate::bounds::Connection::default()
        });
        past_the_opening(&mut body);

        assert!(
            matches!(polled(&mut body), Poll::Pending),
            "a stream ended on the bound its peer would have had to speak to meet"
        );
    }

    /// The other half of the same table: **`Connection::lifetime` is armed, and
    /// it closes a stream however busy it was.**
    ///
    /// The word *however* is the claim, so the body has a chunk waiting on the
    /// poll that ends it: a lifetime read after the cell would be a bound the
    /// busiest stream never meets, one chunk at a time. What ends is the body
    /// and not the connection — `Poll::Ready(None)` is what leaves `hyper`
    /// writing the terminating chunk, so the peer reads a stream that finished
    /// rather than a reset, and reconnects to a server that is still serving.
    #[test]
    fn an_event_stream_is_closed_at_its_lifetime_however_busy_it_was() {
        let (mut emit, mut body) = an_event_stream_over(&crate::bounds::Connection {
            // A lifetime of nothing at all is one already met, which is the
            // state this case needs and which no default ever is.
            lifetime: Duration::ZERO,
            ..crate::bounds::Connection::default()
        });
        emit.send(
            nvs_runtime::sse::Event::carrying(b"still sending")
                .frame()
                .expect("a payload with nothing in it to refuse"),
        )
        .expect("the cell is empty");
        // Even a stream whose lifetime was over before it opened tells the
        // client when to come back, which is the order `poll_frame` reads its
        // two cases in.
        past_the_opening(&mut body);

        assert!(
            matches!(polled(&mut body), Poll::Ready(None)),
            "a stream past its lifetime wrote the chunk it was holding"
        );
    }

    /// § 7's third bullet on the door with no close frame: **a drained server
    /// ends an event stream's body**, and a client reading a stream that
    /// finished reconnects where one reading a reset has nothing to go on.
    ///
    /// Three readings in order
    /// (`rule:concurrency/a-drain-closes-a-connection-cleanly`): a stream on a
    /// server still serving is open; an event waiting in the cell when the
    /// drain begins is still written, since a stream that is writing is work
    /// in progress and the period is its; and a stream with nothing to write
    /// ends on the poll that finds it idle, without waiting the period out —
    /// the period here is far longer than the case is allowed to take, so a
    /// drain that waited it out fails on the reading rather than passing
    /// slowly.
    #[test]
    fn a_drain_ends_the_body_cleanly_rather_than_resetting_the_connection() {
        const PERIOD: Duration = Duration::from_secs(30);
        let draining = Draining::detached();
        let (mut emit, mut body) = an_event_stream_draining(
            &crate::bounds::Connection {
                drain: PERIOD,
                ..crate::bounds::Connection::default()
            },
            &draining,
        );
        past_the_opening(&mut body);
        assert!(
            matches!(polled(&mut body), Poll::Pending),
            "a stream was ended by a server that had not begun draining"
        );

        emit.send(b"in flight".to_vec()).expect("the cell is empty");
        draining.begin();
        assert!(
            matches!(polled(&mut body), Poll::Ready(Some(_))),
            "an event waiting when the drain began was not written"
        );
        assert!(
            matches!(polled(&mut body), Poll::Ready(None)),
            "a drained stream with nothing to write was kept open"
        );
    }

    /// The other half of the same bullet: **a stream that keeps writing is
    /// bounded by the period**, and ends at the period's end however busy it
    /// is — an event in the cell past that instant is not written.
    #[test]
    fn a_drained_stream_still_writing_ends_at_the_periods_end() {
        const PERIOD: Duration = Duration::from_millis(20);
        let draining = Draining::detached();
        let (mut emit, mut body) = an_event_stream_draining(
            &crate::bounds::Connection {
                drain: PERIOD,
                ..crate::bounds::Connection::default()
            },
            &draining,
        );
        past_the_opening(&mut body);

        draining.begin();
        emit.send(b"first".to_vec()).expect("the cell is empty");
        assert!(
            matches!(polled(&mut body), Poll::Ready(Some(_))),
            "a stream writing when the drain began was ended at the bit"
        );

        std::thread::sleep(PERIOD * 2);
        emit.send(b"second".to_vec()).expect("the cell is empty");
        assert!(
            matches!(polled(&mut body), Poll::Ready(None)),
            "a drained stream was still being written past its period"
        );
    }

    /// `rule:concurrency/connection-bounds-are-finite`'s reconnection hint:
    /// **every event stream opens with a `retry:` block, and no two streams are
    /// told the same wait.**
    ///
    /// The second half is what the draw is for — a constant hands a drained
    /// fleet one instant to come back at — so it is asserted over a handful of
    /// streams rather than over two: two draws from a band a couple of thousand
    /// milliseconds wide collide once in a couple of thousand runs, and a case
    /// that flakes that often is one a later session deletes. Every draw is
    /// also read against the band itself, since a spread wider than the field
    /// is a wait an operator did not write.
    #[test]
    fn a_retry_line_is_written_at_open_and_is_not_the_same_for_two_streams() {
        const STREAMS: usize = 12;
        let bounds = crate::bounds::Connection::default();
        let spread = bounds.reconnect / 3;
        let band =
            (bounds.reconnect - spread).as_millis()..=(bounds.reconnect + spread).as_millis();

        let mut drawn = std::collections::BTreeSet::new();
        for _ in 0..STREAMS {
            let (_emit, mut body) = an_event_stream_over(&bounds);
            let Poll::Ready(Some(hint)) = polled(&mut body) else {
                panic!("an event stream's first frame is its reconnection hint")
            };
            let millis = std::str::from_utf8(&hint)
                .ok()
                .and_then(|block| block.strip_prefix("retry: "))
                .and_then(|block| block.strip_suffix("\n\n"))
                .and_then(|wait| wait.parse::<u128>().ok())
                .unwrap_or_else(|| panic!("a reconnection block spells one wait: {hint:?}"));
            assert!(
                band.contains(&millis),
                "a stream was told to come back in {millis}ms, outside {band:?}"
            );
            drawn.insert(millis);
        }

        assert!(
            drawn.len() > 1,
            "{STREAMS} streams were all told the same wait: {drawn:?}"
        );
    }

    /// The same table's message bound, on the door that has no frames:
    /// **one event is the whole message**, so an event over the bound never
    /// reaches the cell the connection frames from.
    ///
    /// Read twice, because a bound that refused everything would pass the first
    /// half alone: the event inside it goes through on the same half afterwards.
    /// That second reading is also the one that says a refusal leaves the stream
    /// writable — nothing arrived from a peer and no event was half-written, so
    /// this is a program handing over too much and not a connection to close.
    #[test]
    fn an_event_larger_than_the_message_bound_is_refused() {
        const MESSAGE: usize = 64;
        let (mut emit, _body) = Answer::stream(&crate::bounds::Connection {
            frame: MESSAGE,
            message: MESSAGE,
            ..crate::bounds::Connection::default()
        });

        let oversized = nvs_runtime::sse::Event::carrying(&[b'x'; MESSAGE * 2])
            .frame()
            .expect("a payload with nothing in it to refuse");
        let refused = emit
            .send(oversized)
            .expect_err("an event twice the connection's message bound");
        assert!(
            refused.contains(nvs_runtime::stream::CHUNK_TOO_LARGE),
            "an event over the bound was refused as something else: {refused}"
        );

        emit.send(
            nvs_runtime::sse::Event::carrying(b"small enough")
                .frame()
                .expect("a payload with nothing in it to refuse"),
        )
        .expect("an event inside the bound, on a stream the refusal did not close");
    }
}
