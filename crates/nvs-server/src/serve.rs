//! The accept loop: one listening socket, one coroutine per connection, and one
//! `hyper` connection future driven on that coroutine's own stack.
//!
//! [ADR 0138](../../../docs/adr/0138-a-connection-future-is-driven-by-the-coroutine-that-owns-it.md)
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
//! its child ([`nvs_host::spawn_child`], ADR 0072 § 1's "each is a child of the
//! calling task"). That is not a convenience: it is what makes a connection
//! cancellable with the server, and it is the tree
//! [ADR 0072](../../../docs/adr/0072-core-task-structured-concurrency.md) § 4
//! reads when it has to prove nothing is still running. A loop that spawned
//! roots would have to grow its own registry of live connections and its own
//! shutdown, both of which the task tree already is.
//!
//! **What it spends**, per [ADR 0004](../../../docs/adr/0004-memory-for-simplicity.md):
//! one coroutine stack and one `hyper` connection state per connection being
//! served, plus the accepting task's own, plus — while a request is actually
//! running on one of them — that request's isolate, which is one `Ctx` and one
//! pooled task stack under
//! [ADR 0116](../../../docs/adr/0116-an-isolates-arena-is-an-ownership-root.md)
//! § 3's accounting. O(in-flight) at both levels: nothing is held per connection
//! already closed or per request already answered.
//!
//! # What this module does not decide yet
//!
//! - **No routing inside this loop.** The handler is still the caller's
//!   function; what [`crate::mount`] gives it is
//!   [ADR 0097](../../../docs/adr/0097-development-server-and-proxied-origin.md)
//!   § 4's five steps to answer with, and [`Reply`] is the two things this loop
//!   can do with one. Nothing here reads a path from request bytes — that
//!   module's docs own the one place a remainder meets a filesystem, and § 2's
//!   rule with it.
//! - **No response policy for a request that *ran*, beyond a status.** A
//!   [`crate::mount::What::Static`] selection is answered in full by
//!   [`crate::statics`] — § 4's `ETag`, `Range` and MIME policy, one policy in
//!   both deployments — and this loop only carries the [`Reply`] back. What a
//!   *program's* response may say is the next paragraph.
//! - **No response policy beyond a status.** A request that ran answers `200`
//!   carrying what it echoed, and one that did not answers `500` carrying
//!   nothing; `answer`'s own docs are the home of that second call.
//!   [ADR 0092](../../../docs/adr/0092-one-diagnostic-record-three-renderings.md)
//!   § 3's rendering of a failure into a development response is the
//!   configuration slice's, because a mode is what decides it and this loop has
//!   not been given one.
//!   [ADR 0074](../../../docs/adr/0074-http-defaults-safe-and-finite.md) is not
//!   on that list: § 1's header set is filled into every response this loop
//!   writes ([`crate::secure`]) and § 2's closed CORS refuses a preflight above
//!   the handler ([`crate::cors`]), neither of which a mode changes.
//! - **The accept loop backs off.** ADR 0097 § 5's last process-wide bound, as
//!   [ADR 0106](../../../docs/adr/0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md)
//!   § 8 states it: descriptor exhaustion is the one `accept` failure the next
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
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use hyper::body::{Body, Bytes, Frame, Incoming, SizeHint};
use hyper::header::{self, HeaderName, HeaderValue};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use nvs_config::Waits;
use nvs_host::{
    Completion, Isolate, NvsListener, NvsTcp, Running, Waiting, Wake, block_on, spawn_child,
    suspend_current,
};
use nvs_runtime::host::Woken;
use nvs_runtime::{Ctx, Drain, OutputSink, TaskRoot};

use crate::ConnectionIo;
use crate::admit::Admission;
use crate::body::Supply;
use crate::cors::Cors;
use crate::forwarded::{Arrival, Origin, Trusted};
use crate::io::Phase;
use crate::secure::{Scheme, Secure};

/// A response body this server already holds in full, sent as one frame.
///
/// A type of ours rather than `http-body-util`'s `Full`, and that is a
/// dependency not taken rather than a wheel reinvented: what a Novis response
/// carries is the output an isolate produced
/// ([ADR 0088](../../../docs/adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md)
/// § 3's table binds `echo` to the response body), which is a buffer the runtime
/// hands over whole. A crate whose job is to adapt streams would be carried for
/// the one case that never streams.
///
/// The exact [`Body::size_hint`] is what makes `hyper` send a `Content-Length`
/// rather than chunk a body whose length is already known.
#[derive(Debug)]
pub struct Answer(Option<Bytes>);

impl Answer {
    /// The body a caller already has the bytes of.
    #[must_use]
    pub fn new(bytes: impl Into<Bytes>) -> Self {
        Self(Some(bytes.into()))
    }

    /// No body at all — a `204`, a `304`, or the answer to a `HEAD`.
    #[must_use]
    pub fn empty() -> Self {
        Self(None)
    }

    /// The bytes this body carries, empty where it carries none.
    ///
    /// A Novis response body is one buffer this crate already holds whole, so
    /// reading it needs no poll and no `Context`. That is what makes
    /// [`crate::statics`]'s cases assertions about *bytes* — the one range that
    /// was asked for, and the empty body of a `304` — rather than about a status
    /// and a header pair that happen to look right.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        self.0.as_deref().unwrap_or_default()
    }
}

impl Body for Answer {
    type Data = Bytes;
    type Error = Infallible;

    /// The one frame, then the end of the stream.
    fn poll_frame(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        Poll::Ready(self.0.take().map(|bytes| Ok(Frame::data(bytes))))
    }

    fn is_end_stream(&self) -> bool {
        self.0.is_none()
    }

    fn size_hint(&self) -> SizeHint {
        let len = self.0.as_ref().map_or(0, Bytes::len);
        // Widening on every target this builds for; the fallible spelling is
        // here because the lint policy has no exception for a cast that happens
        // to be safe.
        SizeHint::with_exact(u64::try_from(len).unwrap_or(u64::MAX))
    }
}

/// What a handler answers one request with — [ADR 0097]'s § 4 outcomes, as the
/// two things this loop can do with them.
///
/// A handler used to answer with an [`Isolate`] and nothing else, which was
/// [ADR 0097] § 4 with only step 5 in it. Step 1's third arrow is a `404` and
/// step 3 is a file's bytes, and neither is a program: they are responses this
/// server already holds in full, so the type says so rather than a handler
/// inventing an isolate whose only job is to `echo` a status.
///
/// [ADR 0097]: ../../../docs/adr/0097-development-server-and-proxied-origin.md
#[derive(Debug)]
pub enum Reply {
    /// Run this isolate as a child of the connection, and answer with what it
    /// echoed — § 4 steps 4 and 5, and [ADR 0088]'s table.
    ///
    /// **The second field is the connection's half of that request's body**, and
    /// it comes back out of the handler because it may not travel with the
    /// isolate: `hyper`'s [`Incoming`] is polled with the *connection's*
    /// context, and the isolate is a different task
    /// ([`crate::body`]'s module docs are the whole argument). `None` where the
    /// request carried no body, which is also what leaves
    /// [`nvs_runtime::Inbound::has_body`] false.
    ///
    /// [ADR 0088]: ../../../docs/adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md
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
    /// [ADR 0105](../../../docs/adr/0105-an-uploaded-file-is-a-stream-and-there-is-one-way-to-receive-it.md)
    /// § 5 puts this refusal in the server rather than in each consumer, and
    /// before dispatch rather than after it, so that the honest oversized client
    /// never reaches application code and nothing has been allocated to tell it
    /// so. A peer that declares no length is bounded on the wire instead, by the
    /// supplier that is the only thing counting bytes.
    #[must_use]
    pub fn too_large() -> Self {
        Self::status(StatusCode::PAYLOAD_TOO_LARGE)
    }

    /// [ADR 0097] § 4 step 1's third arrow — no mount covers the request, so
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

    /// [ADR 0097] § 5's probe, answered by a server that is accepting: `200`
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

/// Whether this server has stopped accepting — ADR 0097 § 5's drain, as the one
/// bit a probe and an application both read.
///
/// **The accept loop sets it and nothing else does.** A drain begins when
/// [`serve_on_this_core`]'s `keep_serving` seam says to stop, and the loop's own
/// tail is the drain itself: it stops accepting, marks this, and parks until the
/// connections already handed over have finished. Marking it at the caller
/// instead would be the fail-open direction — a caller that forgot leaves the
/// probe answering `200` for a process whose socket is already closed, which is
/// exactly the window a proxy uses this endpoint to avoid.
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
}

/// What every connection this server hands over is served under: ADR 0097 § 5's
/// valve and [ADR 0074](../../../docs/adr/0074-http-defaults-safe-and-finite.md)
/// § 1's header set.
///
/// One argument rather than two because these are the *shared* half of a
/// connection's context — an [`Arc`] each, boot-fixed, so every core answers
/// under the one valve and the one policy. `waits` stays a value beside it for
/// exactly that reason: it is [`Copy`], and § 5 makes it `Boot`-class so a
/// connection carries its own copy rather than a handle somebody could move
/// under it.
#[derive(Clone, Debug)]
pub struct Serving {
    /// § 5's in-flight ceiling, asked before the handler is.
    admission: Arc<Admission>,
    /// ADR 0074 § 1's header set, filled into every response this loop writes.
    secure: Arc<Secure>,
    /// ADR 0097 § 6's `[server] trusted_proxies`, resolved: who may assert a
    /// client address or a scheme. Empty is the default and means no forwarded
    /// header is read at all — [`crate::forwarded`] owns that difference.
    trusted: Arc<Trusted>,
    /// ADR 0074 § 2's cross-origin policy, asked of a preflight before the
    /// handler is. Closed is the default — [`crate::cors`] owns what that means
    /// and why the refusal is taken here rather than in an application.
    cors: Arc<Cors>,
}

impl Serving {
    /// The four, as a boot resolves them.
    #[must_use]
    pub fn new(
        admission: Arc<Admission>,
        secure: Arc<Secure>,
        trusted: Arc<Trusted>,
        cors: Arc<Cors>,
    ) -> Self {
        Self {
            admission,
            secure,
            trusted,
            cors,
        }
    }
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
/// its request still running — `hyper` giving up on the connection, or this
/// task being torn down under it — and a drop that simply released the handle
/// would leave an isolate running with nothing left that could prove it
/// finished, which is
/// [ADR 0072](../../../docs/adr/0072-core-task-structured-concurrency.md) § 4
/// gone rather than kept. So the drop **abandons**: cancel, then wait.
/// [`nvs_host::Running::abandon`] owns both halves and the one case that may
/// not wait.
struct Peer(Option<Box<dyn Running>>);

impl Peer {
    /// Whether the request has ended, asked without waiting for it.
    fn finished(&self) -> bool {
        self.0.as_ref().is_none_or(|running| running.finished())
    }

    /// Takes the answer, once the request has ended.
    ///
    /// `None` only for a `Peer` already collected, which the one caller cannot
    /// reach — worth an answer rather than a panic all the same, since what it
    /// would cost a future caller is one `500` instead of a connection.
    fn collect(&mut self, ctx: &mut Ctx) -> Option<Completion> {
        self.0.take().map(|running| running.join(ctx))
    }
}

impl Drop for Peer {
    fn drop(&mut self) {
        if let Some(running) = self.0.take() {
            running.abandon();
        }
    }
}

/// Drives one accepted connection to completion on the calling coroutine.
///
/// The whole of ADR 0138 § 1: one future, on this task's own stack, polled by
/// [`nvs_host::block_on()`]. `hyper` with `http1` and `server` alone spawns
/// nothing, so there is no executor to install and no second scheduler to
/// reconcile with [`nvs_host::Scheduler`].
///
/// `handler` is asked once per request for the [`Reply`] that request is. Where
/// that is a program it is an
/// [ADR 0006](../../../docs/adr/0006-isolated-script-execution.md) [`Isolate`] —
/// the same type `spawn script` runs, and deliberately **not** a second isolation
/// path, since M7's state-bleed suite is a parameterisation of one mechanism and
/// would prove nothing about two of them. Where it is already a response
/// (ADR 0097 § 4's `404`, or a file), nothing is run for it at all.
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
/// and it is also ADR 0072 § 4's cancellation.
///
/// ADR 0138 § 1 is what makes answering `Pending` cheap rather than an
/// executor: the connection future is driven on this coroutine's own stack, so
/// the park is [`nvs_host::block_on()`]'s and the resume lands back inside the
/// same poll, with the core having served its other connections in between.
/// Nothing re-enters while it is parked, because a suspended task runs nothing
/// at all, which is what gives the borrow below one borrower by construction.
///
/// `ctx` is the connection task's, and that makes it the root of this
/// connection's request tree
/// ([ADR 0072](../../../docs/adr/0072-core-task-structured-concurrency.md)
/// § 1): a request's isolate is a child of the connection, so a client that
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
/// [ADR 0105](../../../docs/adr/0105-an-uploaded-file-is-a-stream-and-there-is-one-way-to-receive-it.md)
/// § 5's cap bounds what a program *asks* for, and draining what it did not ask
/// for would spend the same bytes with nobody having wanted them.
///
/// **`serving` carries § 5's valve, and it is asked before `handler` is.** A
/// request over the ceiling is answered with [`crate::admit::over_capacity`] and
/// nothing is started for it; [`crate::admit`]'s own docs own the order and why
/// it is the whole of the guarantee.
///
/// **It carries ADR 0074 § 1's header set too, and this function is the one
/// place that set is applied.** Every response that leaves here goes through
/// [`Secure::fill`] — a program's, a mount table's `404`, a static file's and
/// § 5's `503` alike — and it *fills* rather than overwrites, which is what
/// leaves `Core\Response::setHeader` its override on one response.
/// [`crate::secure`]'s own docs own that direction, and the effective scheme
/// this passes.
///
/// **`waits` is the clock, and it is a parameter and not a default.** ADR 0097
/// § 5's four waits bound this connection from the moment it is accepted, and
/// [`crate::io`]'s § *The clock* is where they are actually enforced; what this
/// function owns is the one phase change no adapter can see, which is that
/// `hyper` framing a head ends the header wait and answering the request starts
/// the write one.
///
/// # Errors
///
/// `hyper`'s own for this connection: a peer that spoke something other than
/// h1, a socket that failed under it, a request it refused to frame, or a wait
/// that expired under it. A cancelled task is not one of them — the drive
/// answers `None` and this reports `Ok`, because the connection ended for a
/// reason its caller already knows about. **A request's own failure is not one
/// either**: ADR 0006's failure is a value, so it becomes a response instead.
pub fn serve_connection<H>(
    stream: NvsTcp,
    arrival: Arrival,
    ctx: &mut Ctx,
    handler: &H,
    waits: Waits,
    serving: &Serving,
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
    let io = ConnectionIo::new(stream, waits);
    // Taken before the adapter is handed to `hyper`, because that is the last
    // moment anything on this side can reach it.
    let phase = io.phase();
    let phase = &phase;
    let service = service_fn(move |request: Request<Incoming>| async move {
        // A head that framed is a head that arrived: what this connection is
        // waiting for from here is the body, and then nothing until the answer
        // exists.
        phase.set(Phase::Body);
        // ADR 0097 § 6, and it is asked here rather than once per connection
        // because what asserts it is a *header*: one connection carries many
        // requests and a proxy writes the line on each. Before the valve below,
        // for the reason the valve is before the handler — this reads borrowed
        // header bytes and allocates nothing, and a `503` that dropped HSTS
        // behind a TLS-terminating proxy would be answering with less policy
        // than the request it refused was owed.
        //
        // ADR 0074 § 1's effective scheme is `https` here and nowhere else:
        // Novis terminates no TLS (ADR 0097 § 1), so a trusted proxy's
        // `X-Forwarded-Proto` is the only thing that can assert it.
        let origin = match crate::forwarded::walk(arrival, &serving.trusted, request.headers()) {
            Ok(origin) => origin,
            // § 6's one refusal: the walk stopped on a token that was about to
            // become the client address and is not one.
            Err(crate::forwarded::Unusable) => {
                phase.set(Phase::Write);
                let mut refused = unusable_forward();
                serving.secure.fill(refused.headers_mut(), Scheme::Http);
                return Ok::<_, Infallible>(refused);
            }
        };
        // The whole `Origin` goes to the handler below, because the carrier a
        // request reaches its program through is built there — this loop never
        // holds one — and both of the walk's answers belong on it
        // (`nvs_runtime::Inbound::set_peer`). What stays here is the scheme,
        // read again for ADR 0074 § 1's header set at the end of this closure:
        // an `Origin` is `Copy`, so the two readings are one decision.
        //
        // `origin.ignored_address_header()` is § 6's one `Warn` and still has
        // nowhere to go: it goes wherever this loop's other reports go once it
        // has been given a log, and it changes nothing about what is served.
        let scheme = origin.scheme();
        // ADR 0097 § 5, and this line is the *order* rather than the number:
        // the ceiling is asked before the handler is, so a refused request has
        // selected no mount, allocated no isolate, compiled nothing and run no
        // Novis code. A valve that allocated in order to refuse would not
        // protect what it exists to protect. The guard lives to the end of this
        // closure, which is the whole of what "in flight" means here — the
        // answer exists by then.
        let Some(_in_flight) = serving.admission.admit() else {
            phase.set(Phase::Write);
            let mut refused = crate::admit::over_capacity();
            serving.secure.fill(refused.headers_mut(), scheme);
            return Ok::<_, Infallible>(refused);
        };
        // ADR 0074 § 2, asked here for the reason the valve above it is: a
        // preflight selects no mount, allocates no isolate and runs no Novis
        // code, under an open policy exactly as under a closed one — § 2
        // configures the whole answer, so an application asked to produce it
        // would be answering a question already decided above it. Under the
        // valve rather than over it, because the ceiling is what protects the
        // process and a `503` is the answer a server at capacity owes every
        // request, whatever it was going to ask. [`crate::cors`] owns which
        // status the policy gives and what it grants with it.
        if let Some(preflight) = serving.cors.preflight(request.method(), request.headers()) {
            phase.set(Phase::Write);
            let mut permitted = Response::new(Answer::empty());
            *permitted.status_mut() = preflight.status();
            serving.secure.fill(permitted.headers_mut(), scheme);
            preflight.fill(permitted.headers_mut());
            return Ok::<_, Infallible>(permitted);
        }
        // § 2's other half, decided here because the handler below takes the
        // request by value and the `Origin` it must read is on it. The valve's
        // own `503` carries none of this on purpose: it is the door's answer
        // rather than the policy's, it carries no CORS header under any policy,
        // and saying it varied by an origin nothing looked at would be a claim
        // about an answer the policy never produced. [`crate::cors`] owns the
        // rest, including why a cache is what `Vary` is for.
        let crossing = serving.cors.answer(request.headers());
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
            Reply::Run(isolate, mut supply) => {
                // A statement of its own, because the borrow a `match`
                // scrutinee takes lives to the end of the whole `match` — and
                // the arm below borrows the same context again to collect.
                let started = isolate.start(&mut ctx.borrow_mut());
                match started {
                    Ok(running) => {
                        let mut peer = Peer(Some(running));
                        let mut parked = false;
                        // No waker is registered, and that is the seam rather
                        // than an omission: what ends this wait is the
                        // isolate's own end waking the **task** that started it
                        // (`Isolate::start` takes this connection's `Wake`),
                        // and ADR 0138 § 1's loop re-polls whatever the task
                        // was resumed for. A waker stored here would be a
                        // second route to the same resume.
                        std::future::poll_fn(|cx| {
                            // The one thing this wait does besides ask: a
                            // request parked on a pull has published that it
                            // wants a chunk and woken this task, and `cx` is
                            // the context `hyper`'s `Incoming` has to be
                            // polled with. Ahead of the question, so that a
                            // request whose last act is to read its body is
                            // answered on this poll rather than one later.
                            if let Some(supply) = supply.as_mut() {
                                supply.pump(cx);
                            }
                            if peer.finished() {
                                Poll::Ready(())
                            } else {
                                parked = true;
                                Poll::Pending
                            }
                        })
                        .await;
                        // A request that made this future answer `Pending` left
                        // `hyper`'s read side blocked on the head it speculated
                        // about while the isolate ran — and a blocked read side
                        // is the one case `Conn::maybe_notify` skips its
                        // post-response read in. That read is what ends
                        // `Phase::Write` and starts the keep-alive clock
                        // (`crate::io`'s § *The clock*), so without a second
                        // poll an idle connection would sit under the write
                        // wait instead of § 5's keep-alive one. One wake is the
                        // whole fix: ADR 0138 § 2's loop re-polls with the
                        // response written and the dispatcher idle, which is
                        // where `hyper` reads again.
                        if parked {
                            std::future::poll_fn(|cx| {
                                cx.waker().wake_by_ref();
                                Poll::Ready(())
                            })
                            .await;
                        }
                        // Nothing left to wait for, so this join does not park.
                        peer.collect(&mut ctx.borrow_mut())
                            .map_or_else(failed, answer)
                    }
                    // The *argument* had no meaning on the other side, so no
                    // request was ever started. Everywhere else that is the
                    // parent's to raise; here the parent is a connection with
                    // nobody to raise it in, so it is one more `500`.
                    Err(_refused) => failed(),
                }
            }
        };
        // The request took as long as it took — a request's own runtime is
        // ADR 0106's ceiling and not a socket wait — and what remains on this
        // connection is a peer reading what it asked for.
        phase.set(Phase::Write);
        // Last, and once for every path above: ADR 0074 § 1's set is what this
        // response carries beside whatever wrote it, and filling leaves a name
        // the answer already spelled for itself exactly as it is. § 2's answer
        // is written on the same terms and at the same point, so a response has
        // one place where policy reaches it rather than two.
        serving.secure.fill(answered.headers_mut(), scheme);
        crossing.fill(answered.headers_mut());
        Ok(answered)
    });
    let connection = http1::Builder::new().serve_connection(io, service);
    block_on(connection).unwrap_or(Ok(()))
}

/// The response one finished request is.
///
/// [ADR 0088](../../../docs/adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md)
/// § 3's table in code: inside an HTTP request `echo` writes to the response
/// body, and an isolate's `echo` reaches its own capture buffer, so
/// [`Completion::output`] **is** the body and no call site had to name a
/// format.
///
/// A request that failed answers `500` **with no body at all**, discarding
/// whatever it echoed before it failed. That is a decision rather than an
/// omission: a page rendered half-way is worse than none, and the failure
/// itself reaches a response only where a mode says it may
/// ([ADR 0092](../../../docs/adr/0092-one-diagnostic-record-three-renderings.md)
/// § 3's HTML rendering of a `Throwable`, in development), which is the
/// configuration slice's. Until there is a mode to ask, the fail-closed answer
/// is the status and nothing else.
/// What a response carries when nothing declared otherwise —
/// [ADR 0088](../../../docs/adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md)
/// § 4's last bullet, which is what makes ADR 0049's inline-HTML page shape
/// work with no ceremony.
const ECHOED: &str = "text/html; charset=utf-8";

/// What a declaration that is not a header value becomes.
///
/// [ADR 0097](../../../docs/adr/0097-development-server-and-proxied-origin.md)
/// § 4 already names this for a static file's unknown extension, and it is the
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
    // ADR 0088 § 4: the request's own body member said what these bytes are,
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
    // Last, and that ordering is the whole of what `setHeader` means: spec
    // § 15 gives a program an override of a policy-owned header on one
    // response, so what the program set is written after everything this
    // server wrote for itself. ADR 0074 § 4 is why the policy is not
    // narrowing-only, and the reverse order would leave the member with no
    // effect on exactly the headers it exists to change.
    //
    // A pair this crate cannot spell is dropped rather than answered with:
    // `Core\Response::setHeader` refuses a name that is not a token and a value
    // outside printable ASCII at the member, so nothing a program can write
    // arrives here — this is [`UNSPELLABLE`]'s arrangement again, kept as a
    // layer below rather than reduced to a comment about one.
    //
    // `insert` and `append` are the two halves of the row's own
    // `DeclaredHeader::append`, and this is the layer that would otherwise
    // collapse a repeated name: `insert` replaces every value already under it,
    // which is what an override is and what a second `Set-Cookie` must not
    // meet. Applying the rows in order is safe because `Ctx::declare_header`
    // holds a replacing row ahead of every appending one for its name.
    for declared in done.headers {
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
    response
}

/// `400`, carrying nothing — ADR 0097 § 6's one refusal, joining
/// [ADR 0095](../../../docs/adr/0095-ambiguous-input-is-refused-never-repaired.md)
/// § 2's closed list.
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

/// `500`, carrying nothing — `answer`'s docs own why the body is empty.
fn failed() -> Response<Answer> {
    let mut response = Response::new(Answer::empty());
    *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
    response
}

/// Accepts on `listener`, giving every connection its own coroutine, until
/// `keep_serving` breaks or the listener itself fails.
///
/// Called from a task on a core: the connections are its children, which is
/// this module's docs § *One connection per coroutine*. `keep_serving` is asked
/// after each connection has been handed over — a server that runs until the
/// process ends answers `ControlFlow::Continue(())` every time, and a test that
/// wants one connection answers `Break`. It is a callback rather than a flag
/// because what stops a server is a decision the caller owns
/// ([ADR 0078](../../../docs/adr/0078-config-reload-and-control-socket.md)'s
/// control socket is one such caller) and this loop has no business polling for
/// it.
///
/// `admission` is shared with every other core rather than cloned per core,
/// which is ADR 0097 § 5's "counted process-wide": a per-core share would let
/// one hot core refuse while its neighbours idle. Every connection this loop
/// hands over gets a handle on the same count.
///
/// `report` is handed the one line [`AcceptBackoff`] produces per window, for
/// the reason [`crate::admit::Ceiling::clamp_note`] hands its own back as a
/// `String`: this crate is given a socket and not a logger, and the boot that
/// chose where a note goes is the one that owns writing it there.
///
/// `draining` is this loop's own state and it writes it: the moment
/// `keep_serving` says to stop, the drain has begun and § 5's probe answers
/// `503` through [`Reply::health`] — the tail below is that drain. Handed in
/// rather than returned because the handler is built before the loop is, and it
/// is what the probe is answered from.
///
/// `waits` is handed to every connection unchanged and is never re-read: ADR
/// 0097 § 5 makes `[server]` `Boot`-class precisely because `header_timeout`
/// and `keepalive_timeout` apply before any Novis code exists on a connection,
/// so a reload that moved them under a socket already accepted would be a
/// promise two of the four could not keep.
///
/// `serving` is handed to every connection by clone rather than by copy, which
/// is what [`Serving`]'s own docs say it is for: one valve and one header set
/// for the whole process, not one of each per core.
///
/// # Errors
///
/// The listener's own, which ends the whole loop — a listening socket that
/// cannot accept is not a condition the next iteration recovers from. The one
/// exception is descriptor exhaustion, which [`AcceptBackoff`] waits out rather
/// than returning: it is a condition of the process and of every other core's
/// listener too, so ending this loop would turn a transient shortage into a
/// server that stays down after it clears (ADR 0106 § 8).
/// [`nvs_host::NvsListener::accept`] already retries the failures that belong
/// to one connection rather than to the socket. Also `Other` when this is
/// called off a task, because there is then no parent to put a connection
/// under and serving it on this stack would silently be a one-connection
/// server.
pub fn serve_on_this_core<H>(
    listener: &mut NvsListener,
    handler: &Rc<H>,
    waits: Waits,
    serving: &Serving,
    draining: &Draining,
    mut report: impl FnMut(&str),
    mut keep_serving: impl FnMut() -> ControlFlow<()>,
) -> io::Result<()>
where
    H: Fn(Request<Incoming>, Origin) -> Reply + 'static,
{
    // Taken once, and it is also the check that this is a task at all: a wake
    // exists exactly when `spawn_child` has a parent to hang a child off.
    let Some(parent) = Wake::current().map(Rc::new) else {
        return Err(io::Error::other(
            "the accept loop must run as a task on a core",
        ));
    };
    let outstanding = Rc::new(Cell::new(0_usize));
    let mut backoff = AcceptBackoff::default();

    loop {
        let (stream, peer) = match listener.accept() {
            Ok(accepted) => {
                backoff.accepted();
                accepted
            }
            Err(err) => {
                // ADR 0106 § 8. `after` answering `None` is every other
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
        // ADR 0097 § 6's peer, taken from the accept rather than asked of the
        // socket afterwards: this is the one place where who connected is a
        // fact the operating system has just stated, and a `peer_addr` later
        // would be re-deriving it from a descriptor that may already have
        // failed.
        let arrival = Arrival::Tcp(peer.ip());
        let handler = Rc::clone(handler);
        // `Arc`s and not `Rc`s: § 5's valve is counted process-wide and ADR 0074
        // § 1's header set is one policy for the whole server, so what a
        // connection clones is shared by every core rather than by every
        // connection on this one.
        let serving = serving.clone();
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
            // come back as data (ADR 0088 § 3).
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

    // ADR 0097 § 5's drain begins here, and the tail below *is* the drain: this
    // loop has stopped accepting, so from this line the probe answers `503` and
    // a proxy can take the instance out of rotation while the connections
    // already accepted are still being answered. Marked before the park rather
    // than after it, since a drain that announced itself once it was over would
    // report the one state nobody can act on. The child spawned by the last
    // iteration has not run yet — it cannot, until this task parks — so the
    // request on it sees the drain, which is the answer a shutdown wants.
    draining.begin();

    // ADR 0072 § 4, and it is the whole reason this function has a tail: the
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

/// How often an episode is reported. ADR 0106 § 8's own reason for a window
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

/// ADR 0106 § 8's bounded backoff, and the once-per-window note that comes with
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
struct AcceptBackoff {
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
    fn accepted(&mut self) {
        *self = Self::default();
    }

    /// How long to park after `err`, and the line to log when this is the first
    /// exhaustion in a window.
    ///
    /// `None` is every failure that is not descriptor exhaustion — which the
    /// caller ends the loop on, unchanged.
    fn after(&mut self, err: &io::Error, now: Instant) -> Option<(Duration, Option<String>)> {
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
/// A guard rather than a decrement at the end of the body: ADR 0072 § 5's
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

    /// How long a client waits before it gives up and closes, in the two tests
    /// whose subject is a connection the *server* is supposed to close.
    ///
    /// Comfortably above every wait those tests configure and comfortably below
    /// any default, so a phase machine that never armed the wait under test
    /// fails the assertion instead of hanging the suite: the client's own close
    /// is what lets the accept loop finish and the test report.
    const CLIENT_PATIENCE: Duration = Duration::from_secs(10);

    /// One finished request, as [`answer`] takes it.
    fn completed(output: &str, content_type: Option<&str>) -> Completion {
        Completion {
            ok: true,
            value: Value::null(),
            output: output.as_bytes().to_vec(),
            content_type: content_type.map(Into::into),
            status: None,
            headers: Vec::new(),
            error: None,
        }
    }

    /// ADR 0088 § 4: the body member a request used is what decides the
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
    /// owns it. The ordering in [`answer`] is what ADR 0074 § 1's policy set
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

    /// The isolate the first two tests answer with: a program that echoes the
    /// path back, so that a response asserted below is the answer to the
    /// request that asked for it and not merely a well-formed response.
    ///
    /// A closure is the program a test at this level can build — turning a path
    /// into runnable code is `nvs_runtime::script`'s seam and there is no
    /// compiler in this crate — and it writes through `Ctx::write_output`,
    /// which is the buffer a compiled `echo` reaches under ADR 0088 § 3's
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

    /// The handler ADR 0097 § 6's case answers with: the walk's two answers put
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

    /// ADR 0097 § 6's answer reaches the program, and it is the *walk's* answer:
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
    /// only way this answers at all is the shape ADR 0105 § 5 and ADR 0138 § 1
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
    /// ADR 0105 § 5's "before dispatch" is two facts and neither implies the
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

    /// [ADR 0088](../../../docs/adr/0088-a-sink-is-an-instruction-and-the-default-refuses.md)
    /// § 3's first row, end to end: inside an HTTP request `echo` writes to the
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
    /// cannot state about itself, and what ADR 0097 § 6's case is about.
    fn served_under<H>(
        mut listener: NvsListener,
        handler: &Rc<H>,
        client: std::thread::JoinHandle<String>,
        serving: Serving,
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
                || ControlFlow::Break(()),
            )
            .expect("the accept loop failed");
        });
        nvs_host::run_until_idle(&mut sched).expect("the loop failed");
        client.join().expect("the client thread panicked")
    }

    /// A valve every case but the last one is not about, beside ADR 0074 § 1's
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
            // ADR 0097 § 6's default: nothing written, so no forwarded header
            // is read and every request's peer is its own client.
            Arc::new(Trusted::none()),
            // ADR 0074 § 2's default: no origin named, so nothing crosses and no
            // CORS header is emitted at all.
            Arc::new(Cors::default()),
        )
    }

    /// Reads until `needle` has arrived, so a test can stop in the middle of a
    /// keep-alive connection without parsing the framing itself.
    fn read_until(socket: &mut TcpStream, needle: &str, seen: &mut String) {
        while !seen.contains(needle) {
            let mut chunk = [0_u8; 256];
            let read = socket.read(&mut chunk).expect("the read failed");
            assert!(read > 0, "the connection closed before {needle:?}: {seen}");
            seen.push_str(&String::from_utf8_lossy(&chunk[..read]));
        }
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

    /// ADR 0138 § 1's second property, over the wire: **a future is polled only
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
                    || ControlFlow::Break(()),
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

    /// ADR 0138 § 1's third property: **`Pending` suspends the task and not the
    /// thread**, which is [ADR 0106] § 6's rule stated about this seam. One
    /// core, two connections, and the second is answered in full while the
    /// first's drive is parked half-way through a request head.
    ///
    /// One client thread, so the ordering is the case's rather than a race: the
    /// half-sent head is on the wire before the second socket opens, and its
    /// remainder goes out only once the second has been answered. The read
    /// timeout is what makes a blocked core *fail* here instead of hanging the
    /// suite — an assertion nobody reaches is worth nothing.
    ///
    /// [ADR 0106]: ../../../docs/adr/0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md
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
        nvs_host::run_until_idle(&mut sched).expect("the loop failed");

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
        let report = nvs_host::run_until_idle(&mut sched).expect("the loop failed");
        client.join().expect("the client thread panicked");
        assert_eq!(
            sched.tracked_tasks(),
            0,
            "the run left a task behind: ADR 0072 § 4"
        );
        report.finished
    }

    /// ADR 0138 § 1's first property, counted rather than read off a line:
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
    /// runs under [ADR 0006] — on a task of its own, so M7's state-bleed suite
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
    /// [ADR 0006]: ../../../docs/adr/0006-isolated-script-execution.md
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
            "the request tree outlived the request it belonged to: ADR 0072 § 4"
        );
    }

    /// ADR 0074 § 1 over the wire: a server nobody configured answers with the
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

    /// ADR 0074 § 2's closed default, over the wire: a peer that named an origin
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

    /// ADR 0074 § 2's open half over the wire: a tree that named `https://allowed.example`
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
        )
    }

    /// ADR 0074 § 2's other half over the wire: a preflight is answered `403`
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

    /// ADR 0074 § 2's granting half over the wire: a named origin's preflight is
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

    /// ADR 0097 § 5's probe across a shutdown: `200` while the loop is
    /// accepting, `503` from the moment it stops, both from one run and one
    /// handler.
    ///
    /// The changeover is the whole endpoint. A probe pinned at `200` is
    /// indistinguishable from this one for as long as the server is up, and the
    /// answer that matters is the one given while the process is draining —
    /// that is what a proxy takes an instance out of rotation on, and answering
    /// it after the last connection has gone would report the one state nobody
    /// can act on. Nothing here waits on a race: the client's two connections
    /// are sequential, the loop parks in `accept` until the second arrives, and
    /// the child spawned for it cannot run until the loop has broken and marked
    /// the drain.
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
        nvs_host::run_until_idle(&mut sched).expect("the loop failed");

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
                || ControlFlow::Break(()),
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

    /// ADR 0097 § 5's header wait, as the connection it ends: a peer that opens
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

    /// ADR 0097 § 5's valve, and the load-bearing half of it is the **order**:
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

    /// The condition, spelled the way the OS spells it on this platform.
    fn exhausted() -> io::Error {
        io::Error::from_raw_os_error(EXHAUSTED[0])
    }

    /// ADR 0106 § 8 asks for a backoff on descriptor exhaustion and on nothing
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
}
