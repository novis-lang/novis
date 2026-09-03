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
//! - **No mount table and no routing.** The handler is the caller's function,
//!   and [ADR 0097](../../../docs/adr/0097-development-server-and-proxied-origin.md)
//!   § 4's five steps are the slice that puts a table in front of it. § 2's rule
//!   is kept trivially in the meantime: nothing here reads a path from request
//!   bytes at all.
//! - **No response policy beyond a status.** A request that ran answers `200`
//!   carrying what it echoed, and one that did not answers `500` carrying
//!   nothing; `answer`'s own docs are the home of that second call.
//!   [ADR 0074](../../../docs/adr/0074-http-defaults-safe-and-finite.md) § 2's
//!   secure headers and
//!   [ADR 0092](../../../docs/adr/0092-one-diagnostic-record-three-renderings.md)
//!   § 3's rendering of a failure into a development response are both the
//!   configuration slice's, because a mode is what decides them and this loop
//!   has not been given one.
//! - **No deadline on a connection.** [`crate::io::ConnectionIo::stream_mut`]
//!   exists for exactly that and nothing calls it here, so a peer that opens a
//!   socket and says nothing holds a coroutine until it goes away.
//!   [ADR 0074](../../../docs/adr/0074-http-defaults-safe-and-finite.md) § 5's
//!   waits are the `[server]` configuration this loop has not been given, and
//!   until it has, nothing user-reachable starts this loop — `nvs serve` is the
//!   slice that changes both at once.
//! - **One core.** The name says so: a listener bound once and handed to several
//!   cores through [`nvs_host::NvsListener::from_std`] is the fan-out, and it is
//!   the same loop on each of them.

use std::cell::{Cell, RefCell};
use std::convert::Infallible;
use std::io;
use std::ops::ControlFlow;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use hyper::body::{Body, Bytes, Frame, Incoming, SizeHint};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use nvs_host::{
    Completion, Isolate, NvsListener, NvsTcp, Waiting, Wake, block_on, spawn_child, suspend_current,
};
use nvs_runtime::{Ctx, OutputSink, TaskRoot};

use crate::ConnectionIo;

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

/// Drives one accepted connection to completion on the calling coroutine.
///
/// The whole of ADR 0138 § 1: one future, on this task's own stack, polled by
/// [`nvs_host::block_on()`]. `hyper` with `http1` and `server` alone spawns
/// nothing, so there is no executor to install and no second scheduler to
/// reconcile with [`nvs_host::Scheduler`].
///
/// `handler` is asked once per request for the [`Isolate`] that request *is*:
/// [ADR 0006](../../../docs/adr/0006-isolated-script-execution.md)'s isolate,
/// the same type `spawn script` runs, and deliberately **not** a second
/// isolation path — M7's state-bleed suite is a parameterisation of one
/// mechanism and would prove nothing about two of them.
///
/// **That isolate runs inside the connection future's poll, and it may park.**
/// That is what ADR 0138 § 1 bought: the future is driven on this coroutine's
/// own stack, so the join's suspend parks the whole stack — `hyper`'s poll
/// frame included — and the resume lands back inside that same poll, with the
/// core having served its other connections in between. Nothing re-enters
/// while it is parked, because a suspended task runs nothing at all, which is
/// what gives the borrow below one borrower by construction.
///
/// `ctx` is the connection task's, and that makes it the root of this
/// connection's request tree
/// ([ADR 0072](../../../docs/adr/0072-core-task-structured-concurrency.md)
/// § 1): a request's isolate is a child of the connection, so a client that
/// goes away takes its request's tasks with it rather than leaving them
/// behind.
///
/// The request is handed to `handler` with its body unread. Nothing in this
/// slice consumes an [`Incoming`], so a request that carried one ends its
/// connection rather than being followed by a second —
/// [ADR 0105](../../../docs/adr/0105-an-uploaded-file-is-a-stream-and-there-is-one-way-to-receive-it.md)'s
/// lazily yielded parts are the slice that reads one.
///
/// # Errors
///
/// `hyper`'s own for this connection: a peer that spoke something other than
/// h1, a socket that failed under it, or a request it refused to frame. A
/// cancelled task is not one of them — the drive answers `None` and this
/// reports `Ok`, because the connection ended for a reason its caller already
/// knows about. **A request's own failure is not one either**: ADR 0006's
/// failure is a value, so it becomes a response instead.
pub fn serve_connection<H>(stream: NvsTcp, ctx: &mut Ctx, handler: &H) -> hyper::Result<()>
where
    H: Fn(Request<Incoming>) -> Isolate,
{
    let ctx = RefCell::new(ctx);
    let service = service_fn(|request: Request<Incoming>| {
        let isolate = handler(request);
        let answered = match isolate.run(&mut ctx.borrow_mut()) {
            Ok(done) => answer(done),
            // The *argument* had no meaning on the other side, so no request
            // was ever started. Everywhere else that is the parent's to raise;
            // here the parent is a connection with nobody to raise it in, so it
            // is one more `500`.
            Err(_refused) => failed(),
        };
        std::future::ready(Ok::<_, Infallible>(answered))
    });
    let connection = http1::Builder::new().serve_connection(ConnectionIo::new(stream), service);
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
fn answer(mut done: Completion) -> Response<Answer> {
    // There is nowhere to move the child's returned value to — a request
    // answers with what it wrote, not with what it returned — and that field
    // carries one reference this crate would otherwise leak. `discard_value`
    // is the safe discharge, which is the whole reason it exists: this crate
    // forbids `unsafe_code`.
    done.discard_value();
    if done.ok {
        Response::new(Answer::new(done.output))
    } else {
        failed()
    }
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
/// # Errors
///
/// The listener's own, which ends the whole loop — a listening socket that
/// cannot accept is not a condition the next iteration recovers from.
/// [`nvs_host::NvsListener::accept`] already retries the failures that belong
/// to one connection rather than to the socket. Also `Other` when this is
/// called off a task, because there is then no parent to put a connection
/// under and serving it on this stack would silently be a one-connection
/// server.
pub fn serve_on_this_core<H>(
    listener: &mut NvsListener,
    handler: &Rc<H>,
    mut keep_serving: impl FnMut() -> ControlFlow<()>,
) -> io::Result<()>
where
    H: Fn(Request<Incoming>) -> Isolate + 'static,
{
    // Taken once, and it is also the check that this is a task at all: a wake
    // exists exactly when `spawn_child` has a parent to hang a child off.
    let Some(parent) = Wake::current().map(Rc::new) else {
        return Err(io::Error::other(
            "the accept loop must run as a task on a core",
        ));
    };
    let outstanding = Rc::new(Cell::new(0_usize));

    loop {
        let (stream, _peer) = listener.accept()?;
        let handler = Rc::clone(handler);
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
            drop(serve_connection(stream, ctx, handler.as_ref()));
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
    use nvs_host::{Output, Program};
    use nvs_runtime::Value;
    use std::io::{Read as _, Write as _};
    use std::net::TcpStream;

    /// The isolate the first two tests answer with: a program that echoes the
    /// path back, so that a response asserted below is the answer to the
    /// request that asked for it and not merely a well-formed response.
    ///
    /// A closure is the program a test at this level can build — turning a path
    /// into runnable code is `nvs_runtime::script`'s seam and there is no
    /// compiler in this crate — and it writes through `Ctx::write_output`,
    /// which is the buffer a compiled `echo` reaches under ADR 0088 § 3's
    /// table.
    fn echo_the_path() -> Rc<impl Fn(Request<Incoming>) -> Isolate> {
        Rc::new(|request: Request<Incoming>| {
            let path = request.uri().path().to_owned();
            let program: Program = Box::new(move |child: &mut Ctx, _args| {
                child
                    .write_output(format!("hello {path}").as_bytes())
                    .expect("a buffer");
                Value::null()
            });
            Isolate::new(program, Value::null(), Output::Capture)
        })
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
            serve_on_this_core(&mut listener, &echo_the_path(), || ControlFlow::Break(()))
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
            serve_on_this_core(&mut listener, &echo_the_path(), || ControlFlow::Break(()))
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

        let handler = Rc::new(|_request: Request<Incoming>| {
            let program: Program =
                Box::new(|_: &mut Ctx, _args| panic!("the request gave up loudly"));
            Isolate::new(program, Value::null(), Output::Capture)
        });

        let mut sched = nvs_host::Scheduler::new();
        let _installed =
            nvs_host::reactor::install(nvs_host::Reactor::new().expect("the OS refused a poll"));
        sched.spawn(Ctx::new(OutputSink::Sink), TaskRoot::Request, move |_ctx| {
            serve_on_this_core(&mut listener, &handler, || ControlFlow::Break(()))
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
}
