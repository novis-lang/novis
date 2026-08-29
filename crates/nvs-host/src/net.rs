//! The parking TCP stream: a plain `Read` and `Write` that hands the core back
//! instead of blocking it.
//!
//! [ADR 0115](../../../docs/adr/0115-the-reactor-reports-readiness-and-a-stream-that-would-block-parks.md)
//! § 3 is this module's specification, and its one sentence is the whole shape:
//! **issue the syscall; on success return; on `WouldBlock` register, suspend,
//! and loop.** Optimistic and not pessimistic, because the first read of an
//! accepted connection almost always finds the request bytes already there —
//! they arrived with the connection — and a poll-then-read order would pay the
//! microsecond-class registration on every one of those. § 3's cost table is
//! that argument in full.
//!
//! [`NvsTcp`] implements nothing but [`std::io::Read`] and [`std::io::Write`].
//! That is the point: a helper reads a socket with the call it would use on a
//! blocking one, `rustls` drives it without knowing what it is, and no caller up
//! the chain is marked. How the stream reaches its core from inside `&mut self`,
//! having been handed a buffer and nothing else, is [`crate::reactor`]'s module
//! doc — the thread-local route, and what it was chosen over.
//!
//! # The registration is kept, not taken down per park
//!
//! § 3's last line, and it is where the repeat-park cost goes. A stream that
//! parks a second time on the same interest issues **no syscall at all**: the
//! poller is level-triggered and the registration is still filed under this
//! task, so it already reports what the new wait needs. Changing interest —
//! a `read` after a `write` — costs one `reregister`, never a fresh `register`.
//! What that spends, per [ADR 0004](../../../docs/adr/0004-memory-for-simplicity.md):
//! one kernel registration per *stream a task is holding*, released by
//! [`Drop`] and swept by `Reactor::retire` when the task ends. O(in-flight).
//!
//! # Off a core, it blocks — and that is not the rule being bent
//!
//! The goal's standing decision is that a blocking-looking read never blocks
//! **the core**. Reached from `nvs run`, or from a unit test, there is no core:
//! [`crate::current_task`] answers `None`, or no reactor is installed, and there
//! is no coroutine to suspend and no neighbour to starve. The stream waits on
//! its own descriptor with a poll of its own rather than handing the caller a
//! `WouldBlock` it has no way to wait on — a `Read` that can return "not yet"
//! is not the `Read` this module promises. That path creates one poll per wait
//! and is deliberately not optimized: it is the path with no throughput target.
//!
//! # Connecting is the same shape, with one extra question
//!
//! [`NvsTcp::connect`] parks on `WRITABLE` like everything else here, and then
//! asks what the readiness *meant*: a refused connection is writable too, so a
//! connect that only waited would hand back a stream that is dead and looks
//! fine. The two questions that answer that soundly on both platforms — and
//! why `peer_addr`, which is the obvious one, is not among them — are the
//! private `finish_connecting`'s doc. It carries no deadline; bounding it is
//! the reactor's timer wait, which does not exist yet.
//!
//! **Still outstanding:** a Unix-domain sibling of this type, for the same
//! parking contract over a local socket.

use std::io::{self, Read, Write};
use std::net::SocketAddr;

use mio::{Events, Poll, Token};

use crate::reactor::{self, Interest, Reactor};
use crate::scheduler::{TaskId, Waiting, current_task, suspend_current};

/// A TCP stream whose `Read` and `Write` park the task instead of blocking the
/// thread.
///
/// Created from a connected socket — an accept loop's, or a `std` one this
/// switches to non-blocking. Everything about the waiting is underneath the two
/// standard traits, so what holds this is ordinary byte-oriented code.
#[derive(Debug)]
pub struct NvsTcp {
    inner: mio::net::TcpStream,
    /// The task this stream's kernel registration is filed under and what it is
    /// armed for, or `None` while it holds none.
    ///
    /// Kept across parks rather than torn down per wait — this module's docs say
    /// what that buys — and the interest is stored with it so that a repeat park
    /// on the same one can tell that it has nothing to do.
    registered: Option<(TaskId, Interest)>,
}

impl NvsTcp {
    /// Wraps an already non-blocking socket, which is what an accept loop has.
    #[must_use]
    pub fn new(inner: mio::net::TcpStream) -> Self {
        Self {
            inner,
            registered: None,
        }
    }

    /// Takes over a `std` socket, switching it to non-blocking first.
    ///
    /// # Errors
    ///
    /// The platform refused the mode change, or refused to hand back the
    /// descriptor.
    pub fn from_std(stream: std::net::TcpStream) -> io::Result<Self> {
        stream.set_nonblocking(true)?;
        Ok(Self::new(mio::net::TcpStream::from_std(stream)))
    }

    /// Connects to `addr`, parking rather than blocking while the handshake is
    /// in flight.
    ///
    /// The socket is non-blocking from its first syscall, so the platform
    /// answers "in progress" and it is *readiness*, not the `connect` call, that
    /// says how it ended. **Readiness alone does not mean it succeeded**: a
    /// refused connection reports writable exactly as a completed one does, so
    /// what a caller gets back here is a stream that has been *asked* whether
    /// it is connected, and otherwise the connection's own failure —
    /// `ConnectionRefused` — rather than something writable and dead.
    /// `NvsTcp::finish_connecting` owns which two questions do that and why
    /// the obvious ones do not.
    ///
    /// This takes no deadline. A connect to a black hole waits as long as the
    /// platform's own connect timeout, which is minutes; bounding it is the
    /// reactor's timer wait, and until that exists no caller can ask for less.
    ///
    /// # Errors
    ///
    /// The platform refused the socket or the address, or the connection itself
    /// failed — refused, unreachable, or reset while it was being established.
    pub fn connect(addr: SocketAddr) -> io::Result<Self> {
        let mut stream = Self::new(mio::net::TcpStream::connect(addr)?);
        stream.finish_connecting()?;
        Ok(stream)
    }

    /// Waits out an in-flight connect, turning readiness into the answer.
    ///
    /// The same optimistic order as [`Read::read`], and it earns it: a loopback
    /// connect is routinely up before this is first asked, on both platforms
    /// measured, so the common case pays no registration at all.
    ///
    /// The two questions it asks on each turn are the ones that are *sound on
    /// both platforms*, which the obvious pair is not:
    ///
    /// - **`SO_ERROR`**, because a failed connect reports writable exactly as a
    ///   completed one does, and on Windows this is the only place the refusal
    ///   ever appears.
    /// - **A zero-length write**, because it is the portable "is this socket
    ///   connected yet" question: `Ok` on a connected socket having sent
    ///   nothing, `NotConnected`/`WouldBlock` while the handshake is still in
    ///   flight, and on Linux the refusal itself. `peer_addr` is what `mio`'s
    ///   own example asks and it **cannot be used here**: on Windows it answers
    ///   `Ok(the target address)` for a socket whose connect has not started
    ///   succeeding and never will, so a connect built on it reports success
    ///   for a stream that is dead.
    ///
    /// Being sound rather than merely usual matters because [`suspend_current`]
    /// can return for a reason that is not this stream — the reactor's tokens
    /// are task ids, so any other descriptor this task holds wakes it here too
    /// (ADR 0115 § 2 rule 2). A completion test that is only right when the
    /// wake was ours would hand back an unconnected stream on that path.
    fn finish_connecting(&mut self) -> io::Result<()> {
        loop {
            if let Some(err) = self.inner.take_error()? {
                return Err(err);
            }
            match self.inner.write(&[]) {
                Ok(_) => return Ok(()),
                // Not an error, just "not yet" — which is rule 2's shape asked
                // of a different question.
                Err(err)
                    if matches!(
                        err.kind(),
                        io::ErrorKind::NotConnected | io::ErrorKind::WouldBlock
                    ) => {}
                Err(err) => return Err(err),
            }
            self.wait_until_ready(Interest::WRITABLE)?;
        }
    }

    /// The address at the other end.
    ///
    /// # Errors
    ///
    /// The platform's answer for a socket that is no longer connected.
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.inner.peer_addr()
    }

    /// Whether this stream currently holds a registration with its core's
    /// reactor.
    ///
    /// Here because it is what a test asserting § 3's optimistic order has to
    /// ask: a first read that found buffered bytes must never have touched the
    /// reactor at all.
    #[must_use]
    pub fn is_parked_on(&self) -> bool {
        self.registered.is_some()
    }

    /// Waits until `interest` is satisfiable, giving the core back if there is
    /// one to give back.
    fn wait_until_ready(&mut self, interest: Interest) -> io::Result<()> {
        let parked = match current_task() {
            None => false,
            Some(me) => match reactor::with_current(|reactor| self.arm(reactor, me, interest)) {
                None => false,
                Some(armed) => {
                    // Rule 1: registered, and the borrow above is already
                    // dropped, *then* the yield. Never the other way round.
                    armed?;
                    suspend_current(Waiting::Parked)
                }
            },
        };
        if parked {
            // Rule 2: a wake is a hint. Returning here sends the caller back
            // round its own retry loop rather than promising the syscall will
            // now succeed.
            return Ok(());
        }
        self.block_until_ready(interest)
    }

    /// Files this stream's interest with `reactor` under the running task.
    fn arm(&mut self, reactor: &mut Reactor, me: TaskId, interest: Interest) -> io::Result<()> {
        match self.registered {
            Some((task, armed)) if task == me => {
                if armed == interest {
                    // The cheapest park there is: a level-triggered poller with
                    // a live registration already reports what this wait needs.
                    return Ok(());
                }
                reactor.reregister(&mut self.inner, me, interest)?;
            }
            Some((other, _)) => {
                // A stream that changed hands between tasks. Dropping the old
                // entry before taking the new one keeps the reactor's per-task
                // count honest; both tasks are on this core, because nothing
                // here is `Send`.
                reactor.deregister(&mut self.inner, other)?;
                reactor.register(&mut self.inner, me, interest)?;
            }
            None => reactor.register(&mut self.inner, me, interest)?,
        }
        self.registered = Some((me, interest));
        Ok(())
    }

    /// Drops the reactor-side registration, if this stream holds one.
    fn unregister(&mut self) {
        let Some((task, _)) = self.registered.take() else {
            return;
        };
        // A refusal here is the platform saying it does not hold the descriptor,
        // which is the state this call wanted; there is nothing a caller could
        // do with it, and `Drop` has nowhere to report it anyway.
        let _ = reactor::with_current(|reactor| reactor.deregister(&mut self.inner, task));
    }

    /// Blocks this thread on the descriptor, for the case where there is no core
    /// to hand back.
    ///
    /// This module's docs say why blocking is right here and not a bent rule.
    fn block_until_ready(&mut self, interest: Interest) -> io::Result<()> {
        // A descriptor in two pollers at once is refused on some platforms, so
        // a stream that was registered on a core and is now being used off one
        // gives that registration up first.
        self.unregister();

        let mut poll = Poll::new()?;
        poll.registry()
            .register(&mut self.inner, Token(0), interest)?;
        let mut events = Events::with_capacity(1);
        let outcome = poll.poll(&mut events, None);
        // Best-effort, and the drop of `poll` below is what actually guarantees
        // the descriptor is left in no poller.
        let _ = poll.registry().deregister(&mut self.inner);
        match outcome {
            // A signal reports "nothing yet", exactly as `Reactor::poll` treats
            // it: the caller's retry loop asks the socket again.
            Ok(()) => Ok(()),
            Err(err) if err.kind() == io::ErrorKind::Interrupted => Ok(()),
            Err(err) => Err(err),
        }
    }
}

impl Drop for NvsTcp {
    /// Gives the registration back to the reactor.
    ///
    /// `Reactor::retire` already sweeps the table when the task ends; this is
    /// the case that module's docs name alongside it — a stream dropped while
    /// its task runs on, whose kernel registration would otherwise sit under a
    /// descriptor that is about to close.
    fn drop(&mut self) {
        self.unregister();
    }
}

impl Read for NvsTcp {
    /// ADR 0115 § 3, in order: try, return, and only then park.
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            match self.inner.read(buf) {
                Ok(read) => return Ok(read),
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                    self.wait_until_ready(Interest::READABLE)?;
                }
                Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
                Err(err) => return Err(err),
            }
        }
    }
}

impl Write for NvsTcp {
    /// The same three steps as [`Read::read`], on the other interest.
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        loop {
            match self.inner.write(buf) {
                Ok(written) => return Ok(written),
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                    self.wait_until_ready(Interest::WRITABLE)?;
                }
                Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
                Err(err) => return Err(err),
            }
        }
    }

    /// Nothing is buffered on this side, so there is nothing to push.
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reactor::{install, run_until_idle, with_current};
    use crate::scheduler::Scheduler;
    use nvs_runtime::{Ctx, OutputSink, TaskRoot};
    use std::cell::Cell;
    use std::rc::Rc;
    use std::time::Duration;

    fn ctx() -> Ctx {
        Ctx::new(OutputSink::Sink)
    }

    /// A connected pair: the parking half, and the plain `std` half a test
    /// writes to in order to make it readable.
    fn connected_pair() -> (NvsTcp, std::net::TcpStream) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");
        let client = std::net::TcpStream::connect(addr).expect("the loopback refused a connection");
        let (server, _) = listener.accept().expect("the accept failed");
        (
            NvsTcp::from_std(server).expect("the socket refused non-blocking mode"),
            client,
        )
    }

    /// § 3's whole reason for trying first: the common case must not touch the
    /// reactor at all.
    #[test]
    fn a_first_read_that_finds_buffered_bytes_never_touches_the_reactor() {
        let (mut server, mut client) = connected_pair();
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));

        client.write_all(b"hi").expect("the write failed");
        // Waited for here rather than assumed, so that the task below is in the
        // position an accepted connection's first read is normally in: the bytes
        // are already on the socket. Without this the test would park on the
        // loopback's own timing and assert the opposite of what it means to.
        let mut probe = [0_u8; 2];
        while server.inner.peek(&mut probe).is_err() {
            std::thread::sleep(Duration::from_millis(1));
        }

        let touched = Rc::new(Cell::new(true));
        let reported = Rc::clone(&touched);
        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            let mut buf = [0_u8; 8];
            let read = server.read(&mut buf).expect("the read failed");
            assert_eq!(&buf[..read], b"hi");
            reported.set(server.is_parked_on());
        });

        run_until_idle(&mut sched).expect("the loop failed");
        assert!(
            !touched.get(),
            "a read that found its bytes still registered with the reactor"
        );
        assert_eq!(
            with_current(|reactor| reactor.registrations()),
            Some(0),
            "the reactor was touched by a read that did not need it"
        );
    }

    /// The other half: a read with nothing to read parks, the reactor wakes it,
    /// and a second park over the same interest reuses the registration rather
    /// than making a second one.
    #[test]
    fn a_read_with_nothing_to_read_parks_and_reuses_its_registration() {
        let (mut server, mut client) = connected_pair();
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));

        let writer = std::thread::spawn(move || {
            for chunk in [&b"one"[..], &b"two"[..]] {
                std::thread::sleep(Duration::from_millis(20));
                client.write_all(chunk).expect("the write failed");
            }
        });

        let live = Rc::new(Cell::new(0_usize));
        let counted = Rc::clone(&live);
        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            let mut buf = [0_u8; 8];
            for expected in [&b"one"[..], &b"two"[..]] {
                let read = server.read(&mut buf).expect("the read failed");
                assert_eq!(&buf[..read], expected);
                assert!(server.is_parked_on(), "the park left no registration");
                counted.set(
                    counted
                        .get()
                        .max(with_current(|reactor| reactor.registrations()).expect("no reactor")),
                );
            }
        });

        let report = run_until_idle(&mut sched).expect("the loop failed");
        writer.join().expect("the writing thread panicked");
        assert!(
            report.resumes > 1,
            "the read never parked, so nothing about the reactor was exercised"
        );
        assert_eq!(report.finished, 1);
        assert_eq!(
            live.get(),
            1,
            "a repeat park made a second registration instead of keeping one"
        );
        // Rule 3: the task ended, so the loop retired what it held.
        assert_eq!(with_current(|reactor| reactor.registrations()), Some(0));
    }

    #[test]
    fn a_stream_dropped_mid_task_gives_its_registration_back() {
        let (mut server, mut client) = connected_pair();
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));

        let writer = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            client.write_all(b"hi").expect("the write failed");
        });

        let after_drop = Rc::new(Cell::new(usize::MAX));
        let counted = Rc::clone(&after_drop);
        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            let mut buf = [0_u8; 8];
            let read = server.read(&mut buf).expect("the read failed");
            assert_eq!(&buf[..read], b"hi");
            assert_eq!(
                with_current(|reactor| reactor.registrations()),
                Some(1),
                "the park left nothing registered"
            );
            drop(server);
            counted.set(with_current(|reactor| reactor.registrations()).expect("no reactor"));
        });

        run_until_idle(&mut sched).expect("the loop failed");
        writer.join().expect("the writing thread panicked");
        assert_eq!(
            after_drop.get(),
            0,
            "dropping the stream left its registration behind"
        );
    }

    /// A connect on a core reaches a listener, and what comes back is a stream
    /// that knows who it is talking to.
    #[test]
    fn a_connect_on_a_core_reaches_a_listener() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");
        let accepting = std::thread::spawn(move || {
            // Deliberately late: the connect is expected to complete against
            // the backlog, so this is here to keep the socket alive, not to be
            // what completes it.
            std::thread::sleep(Duration::from_millis(20));
            let (mut stream, _) = listener.accept()?;
            // Read rather than just accept, so that what the task got back is
            // asserted to be a *usable* connection and not merely a value.
            let mut buf = [0_u8; 8];
            let read = stream.read(&mut buf)?;
            Ok::<_, io::Error>(buf[..read].to_vec())
        });

        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));
        let peer = Rc::new(Cell::new(None));
        let reported = Rc::clone(&peer);
        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            let mut stream = NvsTcp::connect(addr).expect("the connect failed");
            reported.set(stream.peer_addr().ok());
            stream.write_all(b"hi").expect("the write failed");
        });

        run_until_idle(&mut sched).expect("the loop failed");
        let accepted = accepting.join().expect("the accepting thread panicked");
        assert_eq!(
            accepted.expect("the listener never saw the connection"),
            b"hi",
            "the connected stream did not carry bytes"
        );
        assert_eq!(peer.get(), Some(addr));
        // Rule 3 again: whatever the connect parked on was retired with the
        // task, so a connect leaves the reactor exactly as it found it.
        assert_eq!(with_current(|reactor| reactor.registrations()), Some(0));
    }

    /// Readiness is not success. A refused connection is writable too, so a
    /// `connect` that took the wake as its answer would hand back a dead
    /// stream; the `SO_ERROR` read is what makes this the caller's error.
    #[test]
    fn a_refused_connect_reports_the_refusal_rather_than_a_stream() {
        let addr = {
            let listener =
                std::net::TcpListener::bind("127.0.0.1:0").expect("the OS refused a port");
            listener
                .local_addr()
                .expect("a bound listener had no address")
            // Dropped here: the port is bound by nothing, so the loopback
            // answers the SYN with a reset.
        };

        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));
        // `Option<Result<..>>`, so that a connect still parked when the loop
        // went idle is a different answer from one that succeeded.
        let outcome = Rc::new(Cell::new(None));
        let reported = Rc::clone(&outcome);
        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            reported.set(Some(
                NvsTcp::connect(addr).map(|_| ()).map_err(|err| err.kind()),
            ));
        });

        run_until_idle(&mut sched).expect("the loop failed");
        assert_eq!(
            outcome.get(),
            Some(Err(io::ErrorKind::ConnectionRefused)),
            "a connect to an unbound port did not report the refusal"
        );
        assert_eq!(with_current(|reactor| reactor.registrations()), Some(0));
    }

    /// Off a core there is nothing to hand back, so the connect waits on its
    /// own poll — the same path the reads already take.
    #[test]
    fn a_connect_off_a_core_waits_rather_than_refusing() {
        assert!(current_task().is_none(), "this test must run off a core");
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let stream = NvsTcp::connect(addr).expect("the connect failed");
        assert_eq!(stream.peer_addr().expect("no peer address"), addr);
        assert!(
            !stream.is_parked_on(),
            "a blocking wait left a reactor registration behind"
        );
        drop(listener);
    }

    /// Off a core there is nothing to hand back, so the read waits on its own
    /// descriptor rather than handing the caller a `WouldBlock`.
    #[test]
    fn a_read_off_a_core_waits_rather_than_refusing() {
        let (mut server, mut client) = connected_pair();
        assert!(current_task().is_none(), "this test must run off a core");

        let writer = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            client.write_all(b"hi").expect("the write failed");
        });

        let mut buf = [0_u8; 8];
        let read = server.read(&mut buf).expect("the read failed");
        writer.join().expect("the writing thread panicked");
        assert_eq!(&buf[..read], b"hi");
        assert!(
            !server.is_parked_on(),
            "a blocking wait left a reactor registration behind"
        );
    }
}
