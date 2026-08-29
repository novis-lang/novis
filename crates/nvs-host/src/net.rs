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
//! **Still outstanding:** connecting. [`NvsTcp::from_std`] and
//! [`NvsTcp::new`] take a socket that is already connected, which is what an
//! accept loop hands over; a parking `connect` is one `WRITABLE` wait plus a
//! `take_error` check, and it lands with the accept loop that needs it.

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
