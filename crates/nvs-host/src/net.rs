//! The parking stream: a plain `Read` and `Write` that hands the core back
//! instead of blocking it, over TCP or over a Unix-domain socket —
//! [`NvsListener`], the accepting half that parks on the same four functions,
//! and [`NvsUdp`], which parks on them a whole datagram at a time.
//!
//! `rule:concurrency/try-the-syscall-then-park`
//! is this module's specification, and its one sentence is the whole shape:
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
//! What that spends, per `rule:programs/memory-priority`:
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
//! private `ask_connected`'s doc. What bounds it is the next section, and
//! [`NvsTcp::connect_timeout`] is its spelling.
//!
//! [`NvsTcp::connect_racing`] asks that question of several addresses at once:
//! an attempt per address started [`ATTEMPT_DELAY`] apart without waiting for
//! the previous one to answer, every one of them in flight together, the first
//! whose handshake is up kept and the rest dropped — which closes them — all
//! inside one budget. It is how an outbound call falls back across the
//! addresses a name resolved to
//! (`rule:http-server/an-outbound-call-tries-every-approved-address`), and the
//! set's own order is the caller's: interleaving the families is a property of
//! the set handed in, not of the waiting.
//!
//! # Every wait is bounded by a clock, not by a wake
//!
//! A stream carries an optional deadline ([`NvsTcp::set_deadline`]) and every
//! wait in this module is bounded by it: parking files it with the core's
//! [`Timers`](crate::timer::Timers) beside the reactor registration, and the
//! blocking path off a core hands it to its own poll as a timeout. Coming back
//! out, **the clock decides and never the wake** — `rule:concurrency/the-parking-contract` rule 2 means a
//! resume may be some other descriptor this task holds, and a poll that
//! returned because its timeout expired is the same `Ok(())` as one that
//! returned with an event. Past the deadline a caller gets
//! `io::ErrorKind::TimedOut` through the ordinary `Read`/`Write` return, which
//! is the only error channel those traits have and is why this needs no type
//! of its own.
//!
//! **A task holds one timer entry and a stream has two interests**, so the
//! entry is lifted by the interest that filed it and by no other. A connection
//! future polls the readable half, gets `Pending`, and polls the writable half
//! in the same pass; a `Ready` there that lifted the deadline would leave the
//! task parked on readiness alone, which is an idle timeout a write silently
//! cancels. `NvsStream::timed` is where that is kept, and
//! `nvs_server::io`'s § *The clock* is the caller it was found by.
//!
//! A *deadline* and not a per-call duration, deliberately:
//! `rule:http-server/no-spelling-for-an-unbounded-wait`
//! bounds an operation, and a duration re-read on each wait would push the
//! bound out again every time the peer sent one more byte — an unbounded wait
//! wearing a timeout's spelling. What it spends is nothing per stream that has
//! no deadline and one `Instant` for one that does; the reactor-side entry is
//! `timer.rs`'s, O(tasks currently waiting on one).
//!
//! # One type over the source, not one type per socket family
//!
//! The Unix-domain sibling is `NvsUnix`, and it is *this* type: [`NvsStream`]
//! is generic over what it parks on and each family is a type alias over it.
//! Decided and recorded here rather than in an ADR, under the goal's standing
//! decisions; what it was chosen over is a second concrete type carrying its
//! own copy of the four functions that do the waiting.
//!
//! [`NvsUdp`] is the third alias and the one that shows what the generic is
//! actually over: it is not a stream at all, so it carries no `Read` and no
//! `Write`, and what it shares is the *waiting* — the same registration, the
//! same deadline, the same suspend — under `send_to` and `recv_from` instead.
//!
//! Those four are `wait_until_ready`, `arm`, `unregister` and
//! `block_until_ready`, and between them they hold every rule in this module:
//! register *then* yield, with the reactor borrow already dropped; a wake that
//! decides nothing, because the tokens are task ids; a registration kept across
//! parks; a descriptor given up before it can be offered to a second poller.
//! None of those is looked up when it is needed — each is remembered or it is
//! not — so a copy is a second place for all four to go stale, and a fix
//! landing in one of the two is silence in the other. That is priority 2
//! against priority 4, and `AGENTS.md`'s ordering says how that goes.
//!
//! The cost is the type *name*: a mismatch reads `NvsStream<TcpStream>` rather
//! than `NvsTcp`. The aliases bound it — no caller writes the generic
//! form — and it is the whole cost, because a generic monomorphizes and the run
//! time is unchanged. The alternative that would have cost something is `&mut
//! dyn Source`: a vtable on the parking path, buying nothing the generic does
//! not already give. Per stream the footprint is identical; what is spent is
//! code size, two instantiations of four small functions
//! (`rule:programs/memory-priority`).
//!
//! [`NvsAcceptor`] is that decision reached from the other side. An accepting
//! socket waits on `READABLE` for a connection exactly as a stream waits on it
//! for a byte, so it *holds* an [`NvsStream`] over `mio`'s listener rather than
//! carrying a fifth copy of the waiting, and it is generic over what it accepts
//! for the same reason the stream is: [`NvsListener`] and `NvsUnixListener` are
//! its two aliases. Why it is a wrapper at all, rather than one more alias of
//! [`NvsStream`], is its own doc.
//!
//! What stays per family is what is genuinely per family, and it is only the
//! *address*: a `SocketAddr` on one side, a path on the other, so `connect` is
//! the socket constructor plus a call to `connected`, and `peer_addr` returns
//! two different types. `Read` and `Write` are shared under one added bound,
//! and so are `finish_connecting` and `connected`, through the private
//! `Connecting` trait — both families answer `take_error`, and a local connect
//! is in flight too when the listener's backlog is full.

use std::io::{self, Read, Write};
use std::net::SocketAddr;
// `mio::Poll` is a poller and `std::task::Poll` is an answer; both are spelled
// `Poll` and this module names them in adjacent functions, so the poller takes
// the alias — it appears once, in `block_until_ready`.
use std::task::Poll;
use std::time::{Duration, Instant};

use mio::event::Source;
use mio::{Events, Poll as MioPoll, Token};

use crate::reactor::{self, Interest, Reactor};
use crate::scheduler::{TaskId, Waiting, current_task, suspend_current};

/// A stream whose `Read` and `Write` park the task instead of blocking the
/// thread.
///
/// Created from a connected socket — an accept loop's, or a `std` one this
/// switches to non-blocking. Everything about the waiting is underneath the two
/// standard traits, so what holds this is ordinary byte-oriented code.
///
/// `S` is the thing being waited on, and each socket family is an alias rather
/// than a type of its own: [`NvsTcp`] is the TCP one. This module's docs § *One
/// type over the source* own that decision.
#[derive(Debug)]
pub struct NvsStream<S: Source> {
    inner: S,
    /// The task this stream's kernel registration is filed under and what it is
    /// armed for, or `None` while it holds none.
    ///
    /// Kept across parks rather than torn down per wait — this module's docs say
    /// what that buys — and the interest is stored with it so that a repeat park
    /// on the same one can tell that it has nothing to do.
    registered: Option<(TaskId, Interest)>,
    /// The instant every wait on this stream is bounded by, or `None` for a
    /// stream nothing is waiting on the clock for.
    ///
    /// On the stream rather than on each call because `Read` and `Write` have
    /// nowhere to pass one, which is the same reason the parking is under them
    /// rather than beside them.
    deadline: Option<Instant>,
    /// The interest whose `Pending` filed this stream's deadline with the core's
    /// timers, or `None` when no filing of this stream's is outstanding.
    ///
    /// A task holds **one** timer entry, and one stream can have both interests
    /// in flight at once: a connection future polls the readable half, gets
    /// `Pending`, and then polls the writable half in the same pass. Without
    /// this, the second poll's `Ready` would lift the first one's deadline and
    /// the task would park on readiness alone with nothing left to wake it on
    /// the clock — an idle timeout a write silently cancels. So the entry is
    /// lifted by the interest that filed it and by no other.
    timed: Option<Interest>,
}

/// The parking stream over TCP — what an accept loop on a listening port and
/// what [`NvsTcp::connect`] hand back.
pub type NvsTcp = NvsStream<mio::net::TcpStream>;

/// How far apart [`NvsTcp::connect_racing`] starts its attempts.
///
/// RFC 8305's own recommendation, and a constant rather than a directive
/// because it trades against nothing an operator can see — ADR 0180 § 14.
/// Shorter spends a second socket on a set whose first address was about to
/// answer; longer leaves a call sitting on a black hole for most of its budget.
pub const ATTEMPT_DELAY: Duration = Duration::from_millis(250);

impl<S: Source> NvsStream<S> {
    /// Wraps an already non-blocking socket, which is what an accept loop has.
    #[must_use]
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            registered: None,
            deadline: None,
            timed: None,
        }
    }

    /// Bounds every wait on this stream by `at`, or lifts the bound.
    ///
    /// Takes effect from the next wait: a park already in flight belongs to a
    /// task that is not running, so there is no call here to change its mind.
    pub fn set_deadline(&mut self, at: Option<Instant>) {
        self.deadline = at;
    }

    /// The instant every wait on this stream is bounded by, if any.
    #[must_use]
    pub fn deadline(&self) -> Option<Instant> {
        self.deadline
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
}

/// The clock a wait is bounded by, reached through whatever the socket has been
/// wrapped in.
///
/// [`NvsStream::set_deadline`] above is the whole of the mechanism and this
/// trait adds nothing to it. What it adds is **reach**: a caller that wants to
/// bound one exchange holds the socket under a wrapper or two — a `rustls`
/// session, and on SQL Server a framing tunnel underneath that session — and
/// only the innermost stream ever waits. Without a trait, each layer's own
/// crate would have to name the layer below it, which is exactly the coupling
/// [`crate::tls`]'s generic transport parameter exists to avoid.
///
/// The inherent methods stay where they are: they are what a caller holding the
/// socket, or a plain `NvsTls<NvsTcp>`, already writes. This is the same two
/// methods, forwarded down a stack whose shape the forwarder does not know.
pub trait Deadline {
    /// Bounds every wait underneath this wrapper by `at`, or lifts the bound.
    ///
    /// Takes effect from the next wait, exactly as [`NvsStream::set_deadline`]
    /// does — the bound belongs to the stream and not to a call.
    fn set_deadline(&mut self, at: Option<Instant>);

    /// The instant every wait underneath it is bounded by, if any.
    fn deadline(&self) -> Option<Instant>;
}

impl<S: Source> Deadline for NvsStream<S> {
    fn set_deadline(&mut self, at: Option<Instant>) {
        self.deadline = at;
    }

    fn deadline(&self) -> Option<Instant> {
        self.deadline
    }
}

impl NvsStream<mio::net::TcpStream> {
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
    /// The private `finish_connecting` owns which two questions do that and why
    /// the obvious ones do not.
    ///
    /// This takes no deadline, so a connect to a black hole waits as long as
    /// the platform's own connect timeout, which is minutes.
    /// [`NvsTcp::connect_timeout`] is the bounded spelling and is what a
    /// caller that did not choose the address should reach for.
    ///
    /// # Errors
    ///
    /// The platform refused the socket or the address, or the connection itself
    /// failed — refused, unreachable, or reset while it was being established.
    pub fn connect(addr: SocketAddr) -> io::Result<Self> {
        connected(mio::net::TcpStream::connect(addr)?, None)
    }

    /// [`NvsTcp::connect`], giving up with `TimedOut` if the handshake is not
    /// up within `after`.
    ///
    /// `rule:http-server/no-spelling-for-an-unbounded-wait`'s `connect_timeout`, at the level that can actually
    /// enforce it. The bound is on the **handshake** and is lifted before the
    /// stream is handed back, so a later read takes whatever deadline its
    /// caller sets and not the leftover of getting here — the two are separate
    /// budgets in that section and they are separate here.
    ///
    /// The clock starts at the `connect` syscall rather than at the first
    /// park, because a platform that spends the whole budget inside its own
    /// resolution of the address has spent the caller's budget either way.
    ///
    /// # Errors
    ///
    /// Everything [`NvsTcp::connect`] reports, plus `TimedOut` when the
    /// handshake was still in flight at `after`.
    pub fn connect_timeout(addr: SocketAddr, after: Duration) -> io::Result<Self> {
        // Before the syscall, per the paragraph above: the platform's own time
        // inside `connect` is the caller's budget too.
        let at = Instant::now() + after;
        connected(mio::net::TcpStream::connect(addr)?, Some(at))
    }

    /// Connects to whichever of `addrs` comes up first, under one `within`
    /// budget for the whole set.
    ///
    /// `rule:http-server/an-outbound-call-tries-every-approved-address`'s
    /// fallback, at the level that can hold more than one connect in flight.
    /// Attempts start in the order given, [`ATTEMPT_DELAY`] apart and without
    /// waiting for the previous one to answer; the first socket whose handshake
    /// is up is the stream that comes back, and the rest are dropped, which
    /// closes them. An address that fails brings its successor's start forward
    /// rather than leaving the set idle until the delay is up, so a set of dead
    /// addresses is walked as fast as they refuse.
    ///
    /// The budget is [`NvsTcp::connect_timeout`]'s and is spent by the set
    /// rather than by each address, so falling back introduces no new bound; it
    /// is lifted before the stream is handed back for that method's reason. A
    /// set of one *is* [`NvsTcp::connect_timeout`], called.
    ///
    /// # Errors
    ///
    /// One error naming every address and what it answered, because an error
    /// naming only the last one sends the reader to the wrong host. Its kind is
    /// `TimedOut` when the budget ran out under an address that never answered,
    /// and otherwise the first address's own failure. An empty set is
    /// `InvalidInput`: there is nothing this could have connected to.
    pub fn connect_racing(addrs: &[SocketAddr], within: Duration) -> io::Result<Self> {
        if addrs.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "there is no address to connect to",
            ));
        }
        if let [only] = addrs {
            return Self::connect_timeout(*only, within);
        }
        // Before the first syscall, per `connect_timeout`'s paragraph: the
        // platform's own time inside `connect` is the caller's budget too.
        let at = Instant::now() + within;
        let mut inflight: Vec<(usize, Self)> = Vec::new();
        let mut failures: Vec<(usize, io::Error)> = Vec::new();
        let mut started = 0;
        let mut due = Instant::now();
        while Instant::now() < at {
            while started < addrs.len() && Instant::now() >= due {
                due = Instant::now() + ATTEMPT_DELAY;
                match mio::net::TcpStream::connect(addrs[started]) {
                    Ok(socket) => inflight.push((started, Self::new(socket))),
                    // The constructor spent none of the delay it just set, and
                    // there is nothing in flight to pace against, so the next
                    // address is due now.
                    Err(err) => {
                        failures.push((started, err));
                        due = Instant::now();
                    }
                }
                started += 1;
            }
            let mut lost = false;
            let mut asking = 0;
            while asking < inflight.len() {
                match ask_connected(&mut inflight[asking].1.inner) {
                    // Everything still in `inflight` is dropped on the way out,
                    // and a dropped stream closes its socket.
                    Ok(true) => return Ok(inflight.swap_remove(asking).1),
                    Ok(false) => asking += 1,
                    Err(err) => {
                        failures.push((inflight.swap_remove(asking).0, err));
                        due = Instant::now();
                        lost = true;
                    }
                }
            }
            if lost && started < addrs.len() {
                continue;
            }
            if inflight.is_empty() {
                break;
            }
            // Nothing here will answer sooner than the next attempt is due, and
            // with the set exhausted nothing bounds the wait but the budget.
            let until = if started < addrs.len() {
                due.min(at)
            } else {
                at
            };
            wait_any_writable(&mut inflight, until)?;
        }
        Err(every_attempt_failed(addrs, &failures))
    }

    /// The address at the other end.
    ///
    /// # Errors
    ///
    /// The platform's answer for a socket that is no longer connected.
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.inner.peer_addr()
    }
}

/// The accepting half: a listening socket whose accept parks the task instead
/// of blocking the core it is running on.
///
/// This is a wrapper around [`NvsStream`] over `mio`'s listener and
/// deliberately not a type of its own. Waiting for a connection *is* waiting
/// for readiness — the same registration, the same task, the same deadline —
/// so a second type would mean a fifth copy of the four functions this module's
/// docs § *One type over the source* keeps in one place. What the wrapper buys
/// is the name and the surface: `NvsStream<TcpListener>` carries a `Read` and a
/// `Write` bound it can never satisfy, an acceptor carries `accept` and nothing
/// else, and a caller cannot reach for the wrong one by accident.
///
/// It is generic for [`NvsStream`]'s own reason: what differs between the
/// socket families is only the *address* — the peer an accept answers with, and
/// the socket a bind names — so [`NvsListener`] and `NvsUnixListener` are two
/// aliases over one accept, reached through [`Accepting`].
///
/// What comes out of [`Self::accept`] is a parking stream of the same family:
/// `mio` hands back an already non-blocking socket, which is the state
/// [`NvsStream::new`] documents as its input, so an accepted connection needs no
/// mode change on its way in.
///
/// Both halves of the accept are here — [`Self::accept`] parks the coroutine
/// and [`Self::poll_accept`] answers `Poll::Pending` — for the reason
/// [`NvsStream::poll_read`] gives: a poll may not suspend, and an accept loop
/// written as a coroutine has no reason to go through a future.
#[derive(Debug)]
pub struct NvsAcceptor<L: Accepting>(NvsStream<L>);

/// What an accept answers with: the parking stream of this acceptor's family,
/// and the address the connection arrived from.
///
/// A name because the pair is written four times over two functions and reads
/// as two projections at every one of them.
pub type Accepted<L> = (NvsStream<<L as Accepting>::Stream>, <L as Accepting>::Peer);

/// The accepting socket over TCP — what a server that binds a port holds.
pub type NvsListener = NvsAcceptor<mio::net::TcpListener>;

/// The accepting socket over a Unix-domain path — `NvsUnix`'s listening twin,
/// and Unix only for the reason that alias gives.
#[cfg(unix)]
pub type NvsUnixListener = NvsAcceptor<mio::net::UnixListener>;

/// A source that answers connections, and the whole of what differs between the
/// families: the socket an accept hands back, and how the peer is spelled.
///
/// The [`Connecting`] decision read from the other side — the waiting is shared
/// and only the address is per family, so this trait carries the address and
/// nothing else.
pub trait Accepting: Source {
    /// The socket an accept hands back, already non-blocking.
    type Stream: Source;

    /// How this family spells the address a connection arrived from.
    type Peer;

    /// Takes the next connection off the backlog without waiting for one.
    ///
    /// # Errors
    ///
    /// The platform's, `WouldBlock` included: waiting is [`NvsAcceptor`]'s job
    /// and never the socket's.
    fn accept(&self) -> io::Result<(Self::Stream, Self::Peer)>;
}

impl Accepting for mio::net::TcpListener {
    type Stream = mio::net::TcpStream;
    type Peer = SocketAddr;

    fn accept(&self) -> io::Result<(Self::Stream, Self::Peer)> {
        mio::net::TcpListener::accept(self)
    }
}

#[cfg(unix)]
impl Accepting for mio::net::UnixListener {
    type Stream = mio::net::UnixStream;
    type Peer = std::os::unix::net::SocketAddr;

    fn accept(&self) -> io::Result<(Self::Stream, Self::Peer)> {
        mio::net::UnixListener::accept(self)
    }
}

impl NvsListener {
    /// Binds a listening socket to `addr`.
    ///
    /// # Errors
    ///
    /// The platform refused the address — already in use, or not one of this
    /// host's.
    pub fn bind(addr: SocketAddr) -> io::Result<Self> {
        Ok(Self(NvsStream::new(mio::net::TcpListener::bind(addr)?)))
    }

    /// Takes over a `std` listener, switching it to non-blocking first.
    ///
    /// [`NvsTcp::from_std`] for the accepting side, and the spelling a server
    /// that binds once and accepts on several cores needs: the socket is bound
    /// — and its options chosen — before any core exists, and each core takes
    /// its own handle on the descriptor from there.
    ///
    /// # Errors
    ///
    /// The platform refused the mode change.
    pub fn from_std(listener: std::net::TcpListener) -> io::Result<Self> {
        listener.set_nonblocking(true)?;
        Ok(Self(NvsStream::new(mio::net::TcpListener::from_std(
            listener,
        ))))
    }

    /// The address this socket is listening on.
    ///
    /// # Errors
    ///
    /// The platform's answer for a socket it no longer holds.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.0.inner.local_addr()
    }
}

#[cfg(unix)]
impl NvsUnixListener {
    /// Binds a listening socket at `path`, which must not exist yet.
    ///
    /// The path is created by the bind and is **not** removed by the drop, which
    /// is the platform's behaviour and not a choice made here: a caller that
    /// unlinks it is unlinking whatever holds the name now, which need not be
    /// this socket.
    ///
    /// # Errors
    ///
    /// The platform refused the path — it exists, the directory does not, no
    /// permission, or a name too long for `sun_path`.
    pub fn bind(path: impl AsRef<std::path::Path>) -> io::Result<Self> {
        Ok(Self(NvsStream::new(mio::net::UnixListener::bind(path)?)))
    }

    /// Takes over a `std` listener, switching it to non-blocking first —
    /// [`NvsListener::from_std`], for a socket bound before this core existed.
    ///
    /// # Errors
    ///
    /// The platform refused the mode change.
    pub fn from_std(listener: std::os::unix::net::UnixListener) -> io::Result<Self> {
        listener.set_nonblocking(true)?;
        Ok(Self(NvsStream::new(mio::net::UnixListener::from_std(
            listener,
        ))))
    }

    /// The address this socket is listening on, which for a local socket is a
    /// path, an abstract name, or unnamed.
    ///
    /// # Errors
    ///
    /// The platform's answer for a socket it no longer holds.
    pub fn local_addr(&self) -> io::Result<std::os::unix::net::SocketAddr> {
        self.0.inner.local_addr()
    }
}

impl<L: Accepting> NvsAcceptor<L> {
    /// Bounds every wait on this listener by `at`, or lifts the bound —
    /// [`NvsStream::set_deadline`], which owns what a deadline is.
    pub fn set_deadline(&mut self, at: Option<Instant>) {
        self.0.set_deadline(at);
    }

    /// Whether this listener currently holds a registration with its core's
    /// reactor — [`NvsStream::is_parked_on`], for the test that asserts the
    /// optimistic order.
    #[must_use]
    pub fn is_parked_on(&self) -> bool {
        self.0.is_parked_on()
    }

    /// Accepts the next connection, parking the task while there is none.
    ///
    /// `rule:concurrency/try-the-syscall-then-park` in the order everything here takes it: try the syscall, and
    /// only on `WouldBlock` register and suspend. A listener with a full
    /// backlog therefore accepts a burst without touching the reactor once.
    ///
    /// # Errors
    ///
    /// The platform's, or `TimedOut` once this listener's deadline has passed.
    /// **A connection that died in the backlog is not one of them**: the peer
    /// reset it before this side ever held it, there is no connection for a
    /// caller to report the failure against, and an accept loop that stopped on
    /// one would be a listener any peer could close by connecting and resetting.
    /// It is retried here, exactly as `Interrupted` is.
    pub fn accept(&mut self) -> io::Result<Accepted<L>> {
        loop {
            match self.0.inner.accept() {
                Ok((stream, peer)) => return Ok((NvsStream::new(stream), peer)),
                Err(err) if lost_in_the_backlog(&err) => {}
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                    self.0.wait_until_ready(Interest::READABLE)?;
                }
                Err(err) => return Err(err),
            }
        }
    }

    /// [`Self::accept`]'s three steps, stopped one short: try the syscall, and
    /// on `WouldBlock` arm the reactor and answer `Pending` rather than suspend.
    ///
    /// The half a `poll` may call, and [`NvsStream::poll_read`]'s doc is why the
    /// difference matters. The arming happens *here* rather than in whatever
    /// adapter is driving the poll — `rule:concurrency/the-reactor-reports-readiness` rule 1 — so the wake has somewhere
    /// to be recorded before there is anything to record.
    ///
    /// # Errors
    ///
    /// [`Self::accept`]'s, on the same terms.
    pub fn poll_accept(&mut self) -> Poll<io::Result<Accepted<L>>> {
        loop {
            match self.0.inner.accept() {
                Ok((stream, peer)) => {
                    return self
                        .0
                        .answer(Interest::READABLE, Ok((NvsStream::new(stream), peer)));
                }
                Err(err) if lost_in_the_backlog(&err) => {}
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                    match self.0.arm_only(Interest::READABLE) {
                        Ok(true) => return Poll::Pending,
                        Ok(false) => {}
                        Err(err) => return self.0.answer(Interest::READABLE, Err(err)),
                    }
                }
                Err(err) => return self.0.answer(Interest::READABLE, Err(err)),
            }
        }
    }
}

/// Whether an accept failed for a reason that belongs to no connection: the
/// signal case, and the peer that reset while it was still in the backlog.
///
/// One function because both accept paths have to agree, and
/// [`NvsListener::accept`]'s `# Errors` owns why these are retried rather than
/// reported. The reset spelling differs by platform — `ECONNABORTED` where the
/// Unixes report it, `WSAECONNRESET` where Windows does — so both kinds are
/// named and neither is `#[cfg]`-ed.
fn lost_in_the_backlog(err: &io::Error) -> bool {
    matches!(
        err.kind(),
        io::ErrorKind::Interrupted
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::ConnectionReset
    )
}

/// The parking datagram socket — the same four functions, waited on a whole
/// message at a time.
///
/// An alias like [`NvsTcp`], for the reason this module's docs § *One type over
/// the source* gives: what a datagram changes is the syscall each wait wraps and
/// not the waiting. What it does not get is [`Read`] and [`Write`], because a
/// datagram socket has no stream to read — every send names an address and every
/// receive answers with one, which is why the two members below are inherent
/// rather than the two traits everything else here is reached through.
pub type NvsUdp = NvsStream<mio::net::UdpSocket>;

impl NvsStream<mio::net::UdpSocket> {
    /// Binds a datagram socket to `addr`.
    ///
    /// # Errors
    ///
    /// The platform refused the address — already in use, or not one of this
    /// host's.
    pub fn bind(addr: SocketAddr) -> io::Result<Self> {
        Ok(Self::new(mio::net::UdpSocket::bind(addr)?))
    }

    /// Takes over a `std` socket, switching it to non-blocking first.
    ///
    /// [`NvsTcp::from_std`]'s case, reached the same way: the socket is bound —
    /// and its options chosen — before the core that will wait on it exists.
    ///
    /// # Errors
    ///
    /// The platform refused the mode change.
    pub fn from_std(socket: std::net::UdpSocket) -> io::Result<Self> {
        socket.set_nonblocking(true)?;
        Ok(Self::new(mio::net::UdpSocket::from_std(socket)))
    }

    /// The address this socket is bound to.
    ///
    /// # Errors
    ///
    /// The platform's answer for a socket it no longer holds.
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.inner.local_addr()
    }

    /// Sends one datagram to `target`, parking the task while the socket will
    /// not take it.
    ///
    /// `rule:concurrency/try-the-syscall-then-park` in the order everything here
    /// takes it, and a send is the case that most often needs no reactor at all:
    /// a socket whose transmit queue has room answers immediately, and the queue
    /// is only full under a burst.
    ///
    /// One call is one datagram and there is no partial send: a message too
    /// large for the path is refused rather than split, so the returned count is
    /// `buf.len()` on every success and is returned rather than asserted because
    /// it is what the platform said.
    ///
    /// # Errors
    ///
    /// The socket's own — a message over the maximum size among them — or
    /// `TimedOut` once this socket's deadline has passed.
    pub fn send_to(&mut self, buf: &[u8], target: SocketAddr) -> io::Result<usize> {
        loop {
            match self.inner.send_to(buf, target) {
                Ok(sent) => return Ok(sent),
                Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                    self.wait_until_ready(Interest::WRITABLE)?;
                }
                Err(err) => return Err(err),
            }
        }
    }

    /// Receives one datagram, parking the task while none is waiting, and
    /// answers what arrived together with who sent it.
    ///
    /// A datagram is delivered whole or not at all, and a buffer shorter than
    /// the message is where the platforms part company: the Unixes keep the
    /// bytes that fit and drop the rest, while **Windows refuses the call**
    /// with `WSAEMSGSIZE` and hands back neither a count nor a sender. Neither
    /// is bent here — this is the syscall's own answer, and a wrapper that
    /// invented a count for the Windows case would be inventing the sender
    /// beside it.
    ///
    /// A caller that needs one behaviour on both hosts therefore hands this a
    /// buffer no datagram can overflow and does its own cutting.
    /// `Core\Net\Datagram::receive` is the caller in this tree that does, and
    /// `nvs_stdlib::net`'s `DATAGRAM_CEILING` is the home of that reading.
    ///
    /// # Errors
    ///
    /// The socket's own, or `TimedOut` once this socket's deadline has passed.
    /// Windows adds one that has nothing to do with the wait: after a datagram
    /// this socket sent draws an ICMP port-unreachable, the *next* receive
    /// reports `ConnectionReset` for it. It is reported rather than swallowed —
    /// a datagram socket has no connection to reset, so the error is genuinely
    /// about a send and belongs to the caller that made it.
    pub fn recv_from(&mut self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        loop {
            match self.inner.recv_from(buf) {
                Ok(arrived) => return Ok(arrived),
                Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                    self.wait_until_ready(Interest::READABLE)?;
                }
                Err(err) => return Err(err),
            }
        }
    }
}

/// The parking stream over a Unix-domain socket — the same contract, the same
/// four functions, a local address.
///
/// Unix only, and there is no Windows fallback here on purpose: `AF_UNIX` does
/// exist on Windows now, but `mio` does not carry it, so a caller that needs a
/// local transport on both platforms uses [`NvsTcp`] on loopback rather than
/// getting a type that compiles and cannot be polled.
#[cfg(unix)]
pub type NvsUnix = NvsStream<mio::net::UnixStream>;

#[cfg(unix)]
impl NvsStream<mio::net::UnixStream> {
    /// Takes over a `std` socket, switching it to non-blocking first.
    ///
    /// # Errors
    ///
    /// The platform refused the mode change.
    pub fn from_std(stream: std::os::unix::net::UnixStream) -> io::Result<Self> {
        stream.set_nonblocking(true)?;
        Ok(Self::new(mio::net::UnixStream::from_std(stream)))
    }

    /// Connects to the socket bound at `path`, parking rather than blocking
    /// while the connect is in flight.
    ///
    /// A local connect is usually up by the first question — there is no
    /// handshake to wait for — but it is *not* always: a listener whose backlog
    /// is full answers "in progress" exactly as a remote one does, which is why
    /// this waits through the same `finish_connecting` rather than trusting the
    /// syscall's own answer. [`NvsTcp::connect`] is that reasoning's home.
    ///
    /// # Errors
    ///
    /// The platform refused the socket or the path — no such file, no
    /// permission, a path too long for `sun_path` — or nothing is listening.
    pub fn connect(path: impl AsRef<std::path::Path>) -> io::Result<Self> {
        connected(mio::net::UnixStream::connect(path)?, None)
    }

    /// [`NvsUnix::connect`], giving up with `TimedOut` if it is not up within
    /// `after`.
    ///
    /// Here for the backlog-full case above, which is the only way a local
    /// connect waits on anything. [`NvsTcp::connect_timeout`] owns what the
    /// bound covers and when it is lifted.
    ///
    /// # Errors
    ///
    /// Everything [`NvsUnix::connect`] reports, plus `TimedOut`.
    pub fn connect_timeout(path: impl AsRef<std::path::Path>, after: Duration) -> io::Result<Self> {
        let at = Instant::now() + after;
        connected(mio::net::UnixStream::connect(path)?, Some(at))
    }

    /// The address at the other end, which for a local socket is a path, an
    /// abstract name, or unnamed.
    ///
    /// # Errors
    ///
    /// The platform's answer for a socket that is no longer connected.
    pub fn peer_addr(&self) -> io::Result<std::os::unix::net::SocketAddr> {
        self.inner.peer_addr()
    }
}

/// Wraps a socket whose connect is in flight and waits it out, bounded by `at`.
///
/// One function for every family, because only the *address* is per family: a
/// `connect` here is the socket constructor and this call. The bound is on the
/// handshake alone and is lifted before the stream is returned, which is
/// [`NvsTcp::connect_timeout`]'s paragraph and the reason the deadline is set
/// and cleared here rather than left on the stream.
fn connected<S: Connecting>(inner: S, at: Option<Instant>) -> io::Result<NvsStream<S>> {
    let mut stream = NvsStream::new(inner);
    stream.set_deadline(at);
    let outcome = finish_connecting(&mut stream);
    stream.set_deadline(None);
    outcome?;
    Ok(stream)
}

/// A source whose in-flight connect can be waited out by `finish_connecting`.
///
/// Two methods rather than a bound on `mio`'s own types, because the pair below
/// is the whole of what completing a connect needs and every socket family has
/// it: `take_error` is where a refusal lands, and `Write` is the zero-length
/// write that asks whether the handshake is up. That is why `connect` is the
/// only thing left per family — the *address* differs, the waiting does not.
trait Connecting: Source + Write {
    /// The pending error on the socket, taken.
    ///
    /// # Errors
    ///
    /// The platform refused to answer for this descriptor.
    fn take_error(&self) -> io::Result<Option<io::Error>>;
}

impl Connecting for mio::net::TcpStream {
    fn take_error(&self) -> io::Result<Option<io::Error>> {
        mio::net::TcpStream::take_error(self)
    }
}

#[cfg(unix)]
impl Connecting for mio::net::UnixStream {
    fn take_error(&self) -> io::Result<Option<io::Error>> {
        mio::net::UnixStream::take_error(self)
    }
}

/// Asks, without waiting, whether an in-flight connect has finished: `true`
/// once the handshake is up, `false` while it is still in flight.
///
/// A free function rather than a method, because an inherent `impl` bounded by
/// a private trait makes a public type carry a bound nothing outside this
/// module can name — `private_bounds`, and the alternative is publishing a
/// trait that exists only to be these two questions.
///
/// The two questions it asks are the ones that are *sound on both platforms*,
/// which the obvious pair is not:
///
/// - **`SO_ERROR`**, because a failed connect reports writable exactly as a
///   completed one does, and on Windows this is the only place the refusal
///   ever appears.
/// - **A zero-length write**, because it is the portable "is this socket
///   connected yet" question: `Ok` on a connected socket having sent nothing,
///   `NotConnected`/`WouldBlock` while the handshake is still in flight, and on
///   Linux the refusal itself. `peer_addr` is what `mio`'s own example asks and
///   it **cannot be used here**: on Windows it answers `Ok(the target address)`
///   for a socket whose connect has not started succeeding and never will, so a
///   connect built on it reports success for a stream that is dead.
///
/// Being sound rather than merely usual matters because a wake names nothing
/// for either caller: [`suspend_current`] can return for a reason that is not
/// this socket — the reactor's tokens are task ids, so any other descriptor the
/// task holds wakes it too (`rule:concurrency/the-parking-contract` rule 2) —
/// and [`NvsTcp::connect_racing`] waits on one token for a whole set of
/// sockets. A completion test that is only right when the wake was this
/// socket's would hand back an unconnected stream on both paths.
///
/// # Errors
///
/// The connection's own failure — refused, unreachable, reset while it was
/// being established — or the platform refusing to answer for the descriptor.
fn ask_connected<S: Connecting>(inner: &mut S) -> io::Result<bool> {
    if let Some(err) = inner.take_error()? {
        return Err(err);
    }
    match inner.write(&[]) {
        Ok(_) => Ok(true),
        // Not an error, just "not yet" — which is rule 2's shape asked of a
        // different question.
        Err(err)
            if matches!(
                err.kind(),
                io::ErrorKind::NotConnected | io::ErrorKind::WouldBlock
            ) =>
        {
            Ok(false)
        }
        Err(err) => Err(err),
    }
}

/// Waits out one in-flight connect, asking [`ask_connected`] on each turn.
///
/// The same optimistic order as [`Read::read`], and it earns it: a loopback
/// connect is routinely up before this is first asked, on both platforms
/// measured, so the common case pays no registration at all.
fn finish_connecting<S: Connecting>(stream: &mut NvsStream<S>) -> io::Result<()> {
    loop {
        if ask_connected(&mut stream.inner)? {
            return Ok(());
        }
        stream.wait_until_ready(Interest::WRITABLE)?;
    }
}

/// Waits until one of `inflight` may have finished connecting, or until `at`.
///
/// [`NvsStream::wait_until_ready`] over a set instead of over one stream, and
/// the same rules hold in the same order: register every socket *then* yield,
/// with the reactor borrow already dropped; one timer entry, because the wait
/// belongs to the task and not to any member of the set; and a wake that
/// decides nothing, since the tokens are task ids and this one names no member
/// at all. The caller asks each socket again on the way round.
fn wait_any_writable(inflight: &mut [(usize, NvsTcp)], at: Instant) -> io::Result<()> {
    let parked = match current_task() {
        None => false,
        Some(me) => match reactor::with_current(|reactor| -> io::Result<()> {
            for (_, stream) in &mut *inflight {
                stream.arm(reactor, me, Interest::WRITABLE)?;
            }
            reactor.timers().arm(me, at);
            Ok(())
        }) {
            None => false,
            Some(armed) => {
                armed?;
                let resumed = suspend_current(Waiting::Parked);
                reactor::with_current(|reactor| reactor.timers().disarm(me));
                if resumed.cancelled() {
                    // `wait_until_ready`'s paragraph: a cancellation is
                    // delivered once, so the answer has to be terminal.
                    return Err(io::Error::new(
                        io::ErrorKind::ConnectionAborted,
                        "the task was cancelled",
                    ));
                }
                resumed.suspended()
            }
        },
    };
    if parked {
        return Ok(());
    }
    block_any_writable(inflight, at)
}

/// Blocks this thread on the whole set, for the case where there is no core to
/// hand back — [`NvsStream::block_until_ready`] with one token per attempt.
fn block_any_writable(inflight: &mut [(usize, NvsTcp)], at: Instant) -> io::Result<()> {
    // A descriptor in two pollers at once is refused on some platforms, so a
    // socket armed on a core gives that registration up before this one.
    for (_, stream) in &mut *inflight {
        stream.unregister();
    }
    let mut poll = MioPoll::new()?;
    let mut events = Events::with_capacity(inflight.len().max(1));
    let mut registered = 0;
    let outcome = inflight
        .iter_mut()
        .enumerate()
        .try_for_each(|(slot, (_, stream))| {
            let filed =
                poll.registry()
                    .register(&mut stream.inner, Token(slot), Interest::WRITABLE);
            if filed.is_ok() {
                registered += 1;
            }
            filed
        })
        .and_then(|()| {
            poll.poll(
                &mut events,
                Some(at.saturating_duration_since(Instant::now())),
            )
        });
    // Best-effort, and the drop of `poll` is what actually guarantees no
    // descriptor is left in it.
    for (_, stream) in inflight.iter_mut().take(registered) {
        let _ = poll.registry().deregister(&mut stream.inner);
    }
    match outcome {
        Ok(()) => Ok(()),
        // A signal reports "nothing yet", exactly as `Reactor::poll` treats it:
        // the caller asks each socket again.
        Err(err) if err.kind() == io::ErrorKind::Interrupted => Ok(()),
        Err(err) => Err(err),
    }
}

/// The one error a race that reached nothing reports, naming every address.
///
/// One error rather than the last attempt's, because an error naming only the
/// last address sends the reader to the wrong host
/// (`rule:http-server/an-outbound-call-tries-every-approved-address`). An
/// address with no failure of its own is one the budget ran out under, so it
/// reports the clock — and that is what makes the whole error `TimedOut` rather
/// than whatever the addresses that did answer said.
fn every_attempt_failed(addrs: &[SocketAddr], failures: &[(usize, io::Error)]) -> io::Error {
    let mut kind = None;
    let mut named = Vec::with_capacity(addrs.len());
    for (slot, addr) in addrs.iter().enumerate() {
        match failures.iter().find(|(failed, _)| *failed == slot) {
            Some((_, err)) => {
                kind = kind.or_else(|| Some(err.kind()));
                named.push(format!("{addr}: {err}"));
            }
            None => {
                kind = Some(io::ErrorKind::TimedOut);
                named.push(format!("{addr}: {}", timed_out()));
            }
        }
    }
    io::Error::new(
        kind.unwrap_or(io::ErrorKind::TimedOut),
        format!("no approved address connected: {}", named.join("; ")),
    )
}

impl<S: Source> NvsStream<S> {
    /// Waits until `interest` is satisfiable or this stream's deadline has
    /// passed, giving the core back if there is one to give back.
    ///
    /// The deadline is filed with the core's timers *beside* the reactor
    /// registration and taken back out again when the wait ends, however it
    /// ended: `timer.rs` holds one entry per task that is currently waiting on
    /// one, and a task woken by readiness is no longer that task.
    fn wait_until_ready(&mut self, interest: Interest) -> io::Result<()> {
        let deadline = self.deadline;
        if deadline.is_some_and(|at| Instant::now() >= at) {
            // Nothing to wait for: the budget was already spent by an earlier
            // wait on the same stream, or by the caller before it got here.
            return Err(timed_out());
        }
        let parked = match current_task() {
            None => false,
            Some(me) => match reactor::with_current(|reactor| {
                let armed = self.arm(reactor, me, interest);
                if armed.is_ok()
                    && let Some(at) = deadline
                {
                    reactor.timers().arm(me, at);
                }
                armed
            }) {
                None => false,
                Some(armed) => {
                    // Rule 1: registered, and the borrow above is already
                    // dropped, *then* the yield. Never the other way round.
                    armed?;
                    let resumed = suspend_current(Waiting::Parked);
                    if deadline.is_some() {
                        // This path files nothing on `timed` — it takes its own
                        // entry back out here, where the wait has plainly ended
                        // — but a poll on the other interest may have left one,
                        // and it is the same entry.
                        self.timed = None;
                        reactor::with_current(|reactor| reactor.timers().disarm(me));
                    }
                    if resumed.cancelled() {
                        // The wait is over and the syscall is not ready, so the
                        // only honest answer is an error — and it has to be a
                        // terminal one, because a cancellation is delivered
                        // once and a retrying caller would park again with
                        // nothing coming. `Interrupted` is the kind that reads
                        // right and is exactly the one `Read::read_exact` and
                        // friends retry, so it is not that one.
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::ConnectionAborted,
                            "the task was cancelled",
                        ));
                    }
                    resumed.suspended()
                }
            },
        };
        if parked {
            // Rule 2: a wake is a hint, so what ends this wait is the clock and
            // not the resume — the reactor's tokens are task ids, and any other
            // descriptor this task holds wakes it here too. Short of the
            // deadline the caller goes back round its own retry loop rather
            // than being promised the syscall will now succeed.
            return match deadline {
                Some(at) if Instant::now() >= at => Err(timed_out()),
                _ => Ok(()),
            };
        }
        self.block_until_ready(interest, deadline)
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

    /// [`Read::read`]'s three steps, stopped one short: try the syscall, and on
    /// `WouldBlock` arm the reactor and answer `Pending` rather than suspend.
    ///
    /// This is the half a `poll` may call, and the difference is the whole of
    /// `rule:concurrency/a-wake-never-moves-a-task`
    /// 's rejected alternative. Suspending *inside* a poll parks the
    /// coroutine with the future's borrow still held and the drive that owns
    /// the waker never reached, so the readiness that ends the park resumes a
    /// stack that is standing in the middle of `hyper` rather than in the loop
    /// that would re-poll it. Answering `Pending` hands the decision back up to
    /// [`crate::block_on()`], which parks on its own stack with its permission
    /// installed — one park per drive, not one per byte.
    ///
    /// The registration is armed *before* the answer, which is `rule:concurrency/the-reactor-reports-readiness`'s
    /// rule 1 in the shape a poll can keep it: the wake has somewhere to be
    /// recorded before there is anything to record.
    ///
    ///
    /// # Errors
    ///
    /// The socket's own, or `TimedOut` once this stream's deadline has passed.
    pub fn poll_read(&mut self, buf: &mut [u8]) -> Poll<io::Result<usize>>
    where
        S: Read,
    {
        loop {
            match self.inner.read(buf) {
                Ok(read) => return self.answer(Interest::READABLE, Ok(read)),
                Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                    match self.arm_only(Interest::READABLE) {
                        Ok(true) => return Poll::Pending,
                        Ok(false) => {}
                        Err(err) => return self.answer(Interest::READABLE, Err(err)),
                    }
                }
                Err(err) => return self.answer(Interest::READABLE, Err(err)),
            }
        }
    }

    /// [`Self::poll_read`] on the other interest.
    ///
    /// # Errors
    ///
    /// The socket's own, or `TimedOut` once this stream's deadline has passed.
    pub fn poll_write(&mut self, buf: &[u8]) -> Poll<io::Result<usize>>
    where
        S: Write,
    {
        loop {
            match self.inner.write(buf) {
                Ok(written) => return self.answer(Interest::WRITABLE, Ok(written)),
                Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                    match self.arm_only(Interest::WRITABLE) {
                        Ok(true) => return Poll::Pending,
                        Ok(false) => {}
                        Err(err) => return self.answer(Interest::WRITABLE, Err(err)),
                    }
                }
                Err(err) => return self.answer(Interest::WRITABLE, Err(err)),
            }
        }
    }

    /// Ends a poll on `waited_on`, lifting the deadline that interest may have
    /// filed with the core's timers on its way to a `Pending`.
    ///
    /// The parking path takes its timer entry back out on the far side of the
    /// suspend, where the wait has plainly ended. A poll has no far side — the
    /// resume lands in [`crate::block_on()`]'s loop and arrives back here as an
    /// ordinary call — so the entry is lifted by whichever poll answers
    /// `Ready`. Same rule, stated where this shape can keep it.
    ///
    /// **Only the interest that filed it lifts it**, which is [`Self::timed`]'s
    /// whole reason: the other half of this stream may still be parked on the
    /// same clock, and a `Ready` there would otherwise leave that park with
    /// nothing to end it.
    fn answer<T>(&mut self, waited_on: Interest, outcome: io::Result<T>) -> Poll<io::Result<T>> {
        if self.timed == Some(waited_on)
            && let Some(me) = current_task()
        {
            self.timed = None;
            reactor::with_current(|reactor| reactor.timers().disarm(me));
        }
        Poll::Ready(outcome)
    }

    /// Files `interest` with this core's reactor without waiting on it.
    ///
    /// `true` means armed, and the caller may answer `Pending`. `false` is the
    /// case this module's docs § *Off a core, it blocks* owns: there is no
    /// reactor to arrange a wake with, [`crate::block_on()`] would park a thread
    /// nothing is going to unpark, so the wait happens here on the descriptor's
    /// own poll and the caller goes back round its retry loop.
    fn arm_only(&mut self, interest: Interest) -> io::Result<bool> {
        let deadline = self.deadline;
        if deadline.is_some_and(|at| Instant::now() >= at) {
            return Err(timed_out());
        }
        if let Some(me) = current_task()
            && let Some(armed) = reactor::with_current(|reactor| {
                let armed = self.arm(reactor, me, interest);
                if armed.is_ok()
                    && let Some(at) = deadline
                {
                    reactor.timers().arm(me, at);
                    // Whose entry it is, so that the other interest's `Ready`
                    // does not lift it — [`Self::answer`] owns that rule.
                    self.timed = Some(interest);
                }
                armed
            })
        {
            armed?;
            return Ok(true);
        }
        self.block_until_ready(interest, deadline)?;
        Ok(false)
    }

    /// Drops the reactor-side registration, if this stream holds one.
    fn unregister(&mut self) {
        // Whatever this stream filed with the timers is no longer its to lift:
        // the registration the filing belonged to is going away with it.
        self.timed = None;
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
    /// The deadline is this poll's own timeout, which is the same mechanism the
    /// parking path gets from the reactor and is why both answer on the clock.
    fn block_until_ready(
        &mut self,
        interest: Interest,
        deadline: Option<Instant>,
    ) -> io::Result<()> {
        // A descriptor in two pollers at once is refused on some platforms, so
        // a stream that was registered on a core and is now being used off one
        // gives that registration up first.
        self.unregister();

        let mut poll = MioPoll::new()?;
        poll.registry()
            .register(&mut self.inner, Token(0), interest)?;
        let mut events = Events::with_capacity(1);
        let timeout = deadline.map(|at| at.saturating_duration_since(Instant::now()));
        let outcome = poll.poll(&mut events, timeout);
        // Best-effort, and the drop of `poll` below is what actually guarantees
        // the descriptor is left in no poller.
        let _ = poll.registry().deregister(&mut self.inner);
        match outcome {
            // A signal reports "nothing yet", exactly as `Reactor::poll` treats
            // it: the caller's retry loop asks the socket again.
            Ok(()) => {}
            Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
            Err(err) => return Err(err),
        }
        // A poll that ran out of time and one that came back with an event are
        // the same `Ok(())`, so the clock is asked here exactly as it is on the
        // parking path.
        if deadline.is_some_and(|at| Instant::now() >= at) {
            return Err(timed_out());
        }
        Ok(())
    }
}

/// The one error a wait past its deadline reports.
///
/// `io::ErrorKind::TimedOut` and not a type of this crate's own, because what
/// holds an [`NvsTcp`] is ordinary `Read`/`Write` code — `rustls` among it —
/// and `io::Error` is the only thing it can be handed.
fn timed_out() -> io::Error {
    io::Error::new(
        io::ErrorKind::TimedOut,
        "the deadline passed before the socket was ready",
    )
}

impl<S: Source> Drop for NvsStream<S> {
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

impl<S: Source + Read> Read for NvsStream<S> {
    /// `rule:concurrency/try-the-syscall-then-park`, in order: try, return, and only then park.
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

impl<S: Source + Write> Write for NvsStream<S> {
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
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

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

    /// The accepting half's § 3, and the same question the first read asks: a
    /// connection already in the backlog is handed over having touched nothing.
    #[test]
    fn an_accept_that_finds_a_connection_waiting_never_touches_the_reactor() {
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));
        let mut listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");
        // The handshake finishes inside `connect` on loopback, so by the time
        // this returns the connection is in the backlog and the accept below is
        // in the position it is written for — no sleep, and nothing assumed.
        let client = std::net::TcpStream::connect(addr).expect("the loopback refused a connection");
        let client_addr = client
            .local_addr()
            .expect("a connected socket had no address");

        let parked = Rc::new(Cell::new(true));
        let reported = Rc::clone(&parked);
        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            let (_stream, peer) = match listener.poll_accept() {
                Poll::Ready(accepted) => accepted.expect("the accept failed"),
                Poll::Pending => panic!("an accept with a connection waiting answered Pending"),
            };
            assert_eq!(peer, client_addr, "the accept named the wrong peer");
            reported.set(listener.is_parked_on());
        });

        run_until_idle(&mut sched).expect("the loop failed");
        drop(client);
        assert!(
            !parked.get(),
            "an accept that found its connection still registered with the reactor"
        );
        assert_eq!(
            with_current(|reactor| reactor.registrations()),
            Some(0),
            "the reactor was touched by an accept that did not need it"
        );
    }

    /// `rule:concurrency/the-reactor-reports-readiness` rule 1, in the shape a poll can keep it: the registration is
    /// filed *before* the `Pending`, so the wake has somewhere to be recorded.
    #[test]
    fn an_accept_with_nothing_to_accept_arms_before_it_answers_pending() {
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));
        let mut listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");

        // Both halves of rule 1 read from inside the task, because the answer
        // and the registration have to be asserted at the same instant: after
        // the task ends there is nothing left to have got wrong.
        let armed = Rc::new(Cell::new(None));
        let reported = Rc::clone(&armed);
        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            assert!(
                matches!(listener.poll_accept(), Poll::Pending),
                "an accept with nothing to accept did not answer Pending"
            );
            reported.set(Some((
                listener.is_parked_on(),
                with_current(|reactor| reactor.registrations()),
            )));
        });

        run_until_idle(&mut sched).expect("the loop failed");
        assert_eq!(
            armed.get(),
            Some((true, Some(1))),
            "the Pending was answered with no registration behind it"
        );
        assert_eq!(
            with_current(|reactor| reactor.registrations()),
            Some(0),
            "the listener's registration outlived the task that made it"
        );
    }

    /// The parking half: an accept with nothing to accept suspends, the reactor
    /// wakes it, and what it hands back is a connection that carries bytes.
    #[test]
    fn an_accept_on_a_core_parks_until_a_connection_arrives() {
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));
        let mut listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let connecting = std::thread::spawn(move || {
            // Late on purpose: the accept has to have parked before this
            // connection exists, or the test asserts the previous one again.
            std::thread::sleep(Duration::from_millis(20));
            let mut client =
                std::net::TcpStream::connect(addr).expect("the loopback refused a connection");
            client.write_all(b"hi").expect("the write failed");
            client
        });

        let got = Rc::new(RefCell::new(Vec::new()));
        let recorded = Rc::clone(&got);
        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            let (mut stream, _) = listener.accept().expect("the accept failed");
            let mut buf = [0_u8; 8];
            let read = stream.read(&mut buf).expect("the read failed");
            *recorded.borrow_mut() = buf[..read].to_vec();
        });

        run_until_idle(&mut sched).expect("the loop failed");
        drop(connecting.join().expect("the connecting thread panicked"));
        assert_eq!(
            &*got.borrow(),
            b"hi",
            "the accepted connection did not carry bytes"
        );
        assert_eq!(
            with_current(|reactor| reactor.registrations()),
            Some(0),
            "the accept's registration outlived its task"
        );
    }

    /// Off a core there is no coroutine to suspend, so the accept waits on its
    /// own poll — this module's § *Off a core, it blocks*, on the other half.
    #[test]
    fn an_accept_off_a_core_waits_rather_than_refusing() {
        assert!(current_task().is_none(), "this test must run off a core");
        let mut listener = NvsListener::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let connecting = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            std::net::TcpStream::connect(addr).expect("the loopback refused a connection")
        });

        let (stream, peer) = listener.accept().expect("the accept failed");
        let client = connecting.join().expect("the connecting thread panicked");
        assert_eq!(
            peer,
            client
                .local_addr()
                .expect("a connected socket had no address"),
            "the accept named the wrong peer"
        );
        assert!(
            !listener.is_parked_on(),
            "a blocking wait left a reactor registration behind"
        );
        drop(stream);
    }

    /// A wait with a deadline behind it ends when the clock says so, and says
    /// so through the only channel `Read` has.
    #[test]
    fn a_read_past_its_deadline_reports_a_timeout() {
        // Held to the end of the test: a closed peer makes the socket readable
        // at EOF, which is a `read` that answers rather than one that waits.
        let (mut server, client) = connected_pair();
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));

        let deadline = Instant::now() + Duration::from_millis(20);
        server.set_deadline(Some(deadline));
        let outcome = Rc::new(Cell::new(None));
        let reported = Rc::clone(&outcome);
        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            let mut buf = [0_u8; 8];
            let kind = server.read(&mut buf).map(|_| ()).map_err(|err| err.kind());
            // Against the deadline itself, not against an instant taken inside
            // the task: the wait is measured from where the clock was set, and
            // charging it the scheduler's start latency makes a loaded machine
            // report an on-time timeout as an early one.
            reported.set(Some((kind, Instant::now() >= deadline)));
        });

        let report = run_until_idle(&mut sched).expect("the loop failed");
        assert_eq!(report.finished, 1);
        assert_eq!(
            outcome.get(),
            Some((Err(io::ErrorKind::TimedOut), true)),
            "a read past its deadline did not report it, or reported it early"
        );
        // The table `timer.rs` keeps exact: the wait is over, so nothing it
        // filed is still filed.
        assert_eq!(with_current(|reactor| reactor.timers().len()), Some(0));
        assert_eq!(with_current(|reactor| reactor.registrations()), Some(0));
        drop(client);
    }

    /// The other half of "the clock decides": a wait that parked and was woken
    /// inside its deadline gets its bytes, and leaves no deadline filed.
    #[test]
    fn a_read_woken_inside_its_deadline_still_gets_its_bytes() {
        let (mut server, mut client) = connected_pair();
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));

        let writer = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            client.write_all(b"hi").expect("the write failed");
        });

        server.set_deadline(Some(Instant::now() + Duration::from_secs(30)));
        let arrived = Rc::new(Cell::new(false));
        let reported = Rc::clone(&arrived);
        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            let mut buf = [0_u8; 8];
            let read = server
                .read(&mut buf)
                .expect("a read inside its deadline failed");
            reported.set(&buf[..read] == b"hi");
        });

        let report = run_until_idle(&mut sched).expect("the loop failed");
        writer.join().expect("the writing thread panicked");
        assert!(arrived.get(), "the bytes did not come back");
        assert!(
            report.resumes > 1,
            "the read never parked, so no deadline was ever armed"
        );
        assert_eq!(
            with_current(|reactor| reactor.timers().len()),
            Some(0),
            "a wait that ended on readiness left its deadline filed"
        );
    }

    /// Off a core the bound is the poll's own timeout, so the blocking path
    /// answers with the same error rather than waiting for a peer that is
    /// never going to say anything.
    #[test]
    fn a_read_off_a_core_is_bounded_by_the_same_deadline() {
        let (mut server, client) = connected_pair();
        assert!(current_task().is_none(), "this test must run off a core");

        server.set_deadline(Some(Instant::now() + Duration::from_millis(20)));
        let start = Instant::now();
        let mut buf = [0_u8; 8];
        let err = server
            .read(&mut buf)
            .expect_err("a read with no bytes and a deadline behind it succeeded");

        assert_eq!(err.kind(), io::ErrorKind::TimedOut);
        assert!(
            start.elapsed() >= Duration::from_millis(20),
            "the blocking wait gave up before its deadline"
        );
        assert!(
            !server.is_parked_on(),
            "a blocking wait left a reactor registration behind"
        );
        drop(client);
    }

    /// The bound is on the handshake: a connect that comes up hands back a
    /// stream with nothing still bounding its reads.
    #[test]
    fn a_connect_under_a_timeout_lifts_the_bound_once_it_is_up() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");

        let stream =
            NvsTcp::connect_timeout(addr, Duration::from_secs(5)).expect("the connect failed");

        assert_eq!(stream.peer_addr().expect("no peer address"), addr);
        assert_eq!(
            stream.deadline(),
            None,
            "the handshake's bound outlived the handshake"
        );
        drop(listener);
    }

    /// A connect to an address that answers nothing is bounded by its own
    /// argument rather than by the platform's, which is minutes.
    ///
    /// The assertion is deliberately about the *clock* and not the error kind:
    /// 192.0.2.1 is documentation address space, so a host with a default route
    /// sends the SYN and hears nothing — the case the bound exists for — while
    /// a host without a route to it is refused before there is anything to wait
    /// on. Both are fast, and fast is the whole claim; the one thing neither
    /// may do is fire the bound early, which the second half checks.
    #[test]
    fn a_connect_that_hears_nothing_gives_up_at_its_bound() {
        let black_hole: SocketAddr = "192.0.2.1:80"
            .parse()
            .expect("a literal address did not parse");

        let start = Instant::now();
        let err = NvsTcp::connect_timeout(black_hole, Duration::from_millis(50))
            .expect_err("a connect to documentation address space succeeded");
        let elapsed = start.elapsed();

        assert!(
            elapsed < Duration::from_secs(5),
            "the connect was not bounded by its argument, and failed with {err}"
        );
        if err.kind() == io::ErrorKind::TimedOut {
            assert!(
                elapsed >= Duration::from_millis(50),
                "the bound fired before it was due"
            );
        }
    }

    /// The fallback the approved set exists for: a dead address does not end
    /// the call, and what comes back is the connection to the one that
    /// answered. The reactor count is the other half of it — every socket the
    /// race started and did not keep gave its registration back with itself.
    #[test]
    fn a_race_past_a_dead_address_answers_on_the_next_one() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("the OS refused a port");
        let live = listener
            .local_addr()
            .expect("a bound listener had no address");
        // Bound after the live one and dropped at once, so the port it names is
        // closed and is not the one the live listener holds.
        let dead = {
            let closing =
                std::net::TcpListener::bind("127.0.0.1:0").expect("the OS refused a port");
            closing
                .local_addr()
                .expect("a bound listener had no address")
        };

        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));
        let peer = Rc::new(Cell::new(None));
        let reported = Rc::clone(&peer);
        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            let stream = NvsTcp::connect_racing(&[dead, live], Duration::from_secs(5))
                .expect("every approved address failed");
            reported.set(stream.peer_addr().ok());
        });

        run_until_idle(&mut sched).expect("the loop failed");
        assert_eq!(peer.get(), Some(live), "the race kept the wrong socket");
        assert_eq!(with_current(|reactor| reactor.registrations()), Some(0));
        drop(listener);
    }

    /// Every address failing is one error naming each of them, because an error
    /// naming only the last one sends the reader to the wrong host.
    #[test]
    fn a_race_that_reaches_nothing_names_every_address() {
        let (first, second) = {
            let one = std::net::TcpListener::bind("127.0.0.1:0").expect("the OS refused a port");
            let two = std::net::TcpListener::bind("127.0.0.1:0").expect("the OS refused a port");
            (
                one.local_addr().expect("a bound listener had no address"),
                two.local_addr().expect("a bound listener had no address"),
            )
        };

        let err = NvsTcp::connect_racing(&[first, second], Duration::from_secs(2))
            .expect_err("a race to two closed ports connected to something");
        let named = err.to_string();
        assert!(
            named.contains(&first.to_string()),
            "the error did not name the first address: {named}"
        );
        assert!(
            named.contains(&second.to_string()),
            "the error did not name the second address: {named}"
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

    /// `rule:concurrency/try-the-syscall-then-park`'s first half, asserted from the core's side rather than the
    /// caller's: the read is driven with `Scheduler::run` alone and no reactor
    /// poll after it, so what the assertions describe is a core that came back.
    /// A blocking read would never have returned from that call at all.
    #[test]
    fn a_socket_read_that_would_block_parks_its_coroutine() {
        let (mut server, mut client) = connected_pair();
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));

        let returned = Rc::new(Cell::new(false));
        let flagged = Rc::clone(&returned);
        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            let mut buf = [0_u8; 8];
            let read = server.read(&mut buf).expect("the read failed");
            assert_eq!(&buf[..read], b"late");
            flagged.set(true);
        });

        let report = sched.run();
        assert_eq!(
            report.finished, 0,
            "a read with nothing to read returned instead of waiting"
        );
        assert_eq!(report.parked, 1, "the read did not park its coroutine");
        assert!(
            !returned.get(),
            "the task ran past its read without any bytes to read"
        );
        assert_eq!(
            with_current(|reactor| reactor.registrations()),
            Some(1),
            "the park filed nothing for the reactor to wake it on"
        );

        // Left running rather than abandoned parked: a test that ends here
        // would assert the park and never that the park is survivable.
        client.write_all(b"late").expect("the write failed");
        run_until_idle(&mut sched).expect("the loop failed");
        assert!(returned.get(), "the parked read never came back");
    }

    /// The other half of the pair, with the wake pinned to its cause: the task
    /// is parked, nothing is ready, and one `Reactor::turn` after the peer
    /// writes is what puts it back on the run queue.
    #[test]
    fn a_parked_coroutine_resumes_when_its_descriptor_is_ready() {
        let (mut server, mut client) = connected_pair();
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));

        let got = Rc::new(RefCell::new(Vec::new()));
        let collected = Rc::clone(&got);
        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            let mut buf = [0_u8; 8];
            let read = server.read(&mut buf).expect("the read failed");
            collected.borrow_mut().extend_from_slice(&buf[..read]);
        });

        assert_eq!(sched.run().parked, 1, "the read did not park");
        assert_eq!(sched.ready_count(), 0, "a parked task was still runnable");

        client.write_all(b"ready").expect("the write failed");
        let woken = with_current(|reactor| reactor.turn(&mut sched))
            .expect("no reactor is installed")
            .expect("the poll failed");
        assert_eq!(woken, 1, "readiness on the descriptor woke no task");
        assert_eq!(
            sched.ready_count(),
            1,
            "the woken task was not put back on the run queue"
        );

        assert_eq!(sched.run().finished, 1, "the resumed task did not finish");
        assert_eq!(
            &*got.borrow(),
            b"ready",
            "the resumed read got the wrong bytes"
        );
    }

    /// Tier B, from `rule:http-server/a-core-is-never-blocked-on-a-syscall`: what a park hands back is the *core*, so a
    /// task that never touched a socket runs to completion while its neighbour
    /// is still waiting on one. The order is recorded rather than inferred —
    /// the neighbour finishes between the parked task's two lines.
    #[test]
    fn a_core_serves_another_task_while_one_is_parked() {
        let (mut server, mut client) = connected_pair();
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));

        let order = Rc::new(RefCell::new(Vec::new()));
        let by_reader = Rc::clone(&order);
        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            by_reader.borrow_mut().push("the read began");
            let mut buf = [0_u8; 8];
            let read = server.read(&mut buf).expect("the read failed");
            assert_eq!(&buf[..read], b"hi");
            by_reader.borrow_mut().push("the read returned");
        });
        let by_neighbour = Rc::clone(&order);
        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            by_neighbour.borrow_mut().push("the neighbour ran");
        });

        let first = sched.run();
        assert_eq!(
            first.finished, 1,
            "the neighbour did not run while the reader was parked"
        );
        assert_eq!(first.parked, 1, "the reader did not park");
        assert_eq!(
            &*order.borrow(),
            &["the read began", "the neighbour ran"],
            "the core stalled inside the read instead of taking the next task"
        );

        client.write_all(b"hi").expect("the write failed");
        run_until_idle(&mut sched).expect("the loop failed");
        assert_eq!(
            &*order.borrow(),
            &["the read began", "the neighbour ran", "the read returned"],
            "the parked reader never finished"
        );
    }

    /// Byte-oriented code that has never heard of a core drives this stream:
    /// the body below is generic over `Read + Write` and reaches for
    /// `BufReader`, which is the whole reason goal `database`'s drivers and a `rustls`
    /// session compose over it. The peer answers late on purpose, so the park
    /// happens *inside* the generic code rather than beside it.
    #[test]
    fn a_stream_satisfies_std_io_read_and_write() {
        /// Nothing in here names this crate. `S` is the only thing it knows.
        fn ask_and_answer<S: Read + Write>(stream: &mut S) -> io::Result<String> {
            use std::io::BufRead;
            stream.write_all(b"ping\n")?;
            stream.flush()?;
            let mut line = String::new();
            std::io::BufReader::new(stream).read_line(&mut line)?;
            Ok(line)
        }

        let (mut server, mut client) = connected_pair();
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));

        let peer = std::thread::spawn(move || {
            let mut heard = [0_u8; 5];
            client
                .read_exact(&mut heard)
                .expect("the peer's read failed");
            assert_eq!(&heard, b"ping\n");
            std::thread::sleep(Duration::from_millis(20));
            client
                .write_all(b"pong\n")
                .expect("the peer's write failed");
        });

        let answer = Rc::new(RefCell::new(String::new()));
        let recorded = Rc::clone(&answer);
        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            // Object-safe as well as generic: a `dyn` pair is what a driver
            // holding a boxed transport has.
            let _: &mut dyn Read = &mut server;
            let _: &mut dyn Write = &mut server;
            *recorded.borrow_mut() = ask_and_answer(&mut server).expect("the exchange failed");
        });

        let report = run_until_idle(&mut sched).expect("the loop failed");
        peer.join().expect("the peer thread panicked");
        assert_eq!(&*answer.borrow(), "pong\n");
        assert!(
            report.resumes > 1,
            "the exchange never parked, so std::io was not driven across a park"
        );
        assert_eq!(report.finished, 1);
    }

    /// `rule:concurrency/try-the-syscall-then-park`'s claim in the strongest form there is: the stream is a
    /// plain `Read`/`Write`, so a protocol implementation that has never heard
    /// of this crate runs over it unchanged. TLS is the witness worth having.
    /// It reads and writes inside a single call; a record is a length-prefixed
    /// frame that a short read *corrupts* rather than merely delays; and
    /// `rustls` treats a `WouldBlock` from the socket as an error to propagate
    /// rather than a park to retry — so a stream that leaked one would fail
    /// here and nowhere else in this module. Nothing in the client half below
    /// names `NvsTcp` except the line that constructs it.
    ///
    /// The peer answers late twice, and the second one is the point. Once
    /// before its handshake flight, so the session parks mid-handshake; then
    /// with the answer's **last byte** held back, so a park lands strictly
    /// inside an application record rather than tidily between two. A stream
    /// that reported a short read as a complete one passes the first and fails
    /// the second.
    #[test]
    fn a_rustls_session_streams_over_it_unmodified() {
        use rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer, ServerName};
        use std::sync::Arc;

        let issued = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])
            .expect("the certificate could not be generated");
        let cert = issued.cert.der().clone();
        let key =
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(issued.signing_key.serialize_der()));

        // The provider is named rather than installed: `install_default` is
        // global to the whole test binary, and this test needs no such reach.
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let server_config = rustls::ServerConfig::builder_with_provider(Arc::clone(&provider))
            .with_safe_default_protocol_versions()
            .expect("the provider refused the default versions")
            .with_no_client_auth()
            .with_single_cert(vec![cert.clone()], key)
            .expect("the certificate and the key did not pair");
        let mut roots = rustls::RootCertStore::empty();
        roots
            .add(cert)
            .expect("the root store refused the certificate");
        let client_config = rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .expect("the provider refused the default versions")
            .with_root_certificates(roots)
            .with_no_client_auth();

        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("the OS refused a port");
        let addr = listener
            .local_addr()
            .expect("a bound listener had no address");
        let peer = std::thread::spawn(move || {
            let (mut sock, _) = listener.accept().expect("the accept failed");
            let mut conn = rustls::ServerConnection::new(Arc::new(server_config))
                .expect("the server config was rejected");
            // Late on purpose: the client is already parked on this flight.
            std::thread::sleep(Duration::from_millis(20));
            let mut heard = [0_u8; 5];
            rustls::Stream::new(&mut conn, &mut sock)
                .read_exact(&mut heard)
                .expect("the peer's read failed");
            assert_eq!(&heard, b"ping\n");

            // Framed by hand instead of through `Stream`, for the one thing a
            // test needs out of it: the record becomes a buffer this thread
            // can cut, and the cut is what puts the park inside it.
            conn.writer()
                .write_all(b"pong\n")
                .expect("the peer's write failed");
            let mut record = Vec::new();
            while conn.wants_write() {
                conn.write_tls(&mut record)
                    .expect("the record could not be framed");
            }
            let (head, tail) = record.split_at(record.len() - 1);
            sock.write_all(head).expect("the peer's first half failed");
            std::thread::sleep(Duration::from_millis(20));
            sock.write_all(tail).expect("the peer's last byte failed");
        });

        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));
        let session = Rc::new(RefCell::new(None));
        let recorded = Rc::clone(&session);
        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            let mut sock = NvsTcp::connect(addr).expect("the connect failed");
            let name = ServerName::try_from("localhost").expect("the name is not a DNS name");
            let mut conn = rustls::ClientConnection::new(Arc::new(client_config), name)
                .expect("the client config was rejected");
            let mut tls = rustls::Stream::new(&mut conn, &mut sock);
            tls.write_all(b"ping\n").expect("the write failed");
            tls.flush().expect("the flush failed");
            let mut heard = [0_u8; 5];
            tls.read_exact(&mut heard).expect("the read failed");
            *recorded.borrow_mut() = Some((
                String::from_utf8(heard.to_vec()).expect("the plaintext was not text"),
                // Not decoration: a session that had skipped verification would
                // have carried the same five bytes.
                conn.peer_certificates().is_some(),
            ));
        });

        let report = run_until_idle(&mut sched).expect("the loop failed");
        peer.join().expect("the peer thread panicked");
        let session = session.borrow();
        let (plaintext, verified) = session.as_ref().expect("the session never completed");
        assert_eq!(plaintext, "pong\n", "the session lost its plaintext");
        assert!(
            *verified,
            "the handshake completed without a verified certificate"
        );
        assert!(
            report.resumes > 1,
            "the session never parked, so nothing was driven across a park"
        );
        assert_eq!(report.finished, 1);
        // Rule 3: the registration went back with the task, TLS or not.
        assert_eq!(with_current(|reactor| reactor.registrations()), Some(0));
    }

    /// A path under the system temporary directory that nothing else in this
    /// binary will pick.
    ///
    /// Short on purpose: `sun_path` is 108 bytes and a bind past it fails with
    /// `InvalidInput`, which reads like a bug in the stream rather than in the
    /// name it was handed.
    #[cfg(unix)]
    fn socket_path(name: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("nvs-{}-{name}.sock", std::process::id()));
        // A previous run that was killed leaves the node behind, and `bind`
        // refuses an existing one with `AddrInUse`.
        let _ = std::fs::remove_file(&path);
        path
    }

    /// The parking contract over a local socket is the *same* contract: this is
    /// the TCP parking test with one type substituted, which is what makes it
    /// worth having — the four functions it exercises are the same four.
    #[cfg(unix)]
    #[test]
    fn a_unix_read_with_nothing_to_read_parks_and_reuses_its_registration() {
        let (server, mut client) =
            std::os::unix::net::UnixStream::pair().expect("the OS refused a socket pair");
        let mut server = NvsUnix::from_std(server).expect("the socket refused non-blocking mode");
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
        assert_eq!(
            live.get(),
            1,
            "a repeat park made a second registration instead of keeping one"
        );
        assert_eq!(with_current(|reactor| reactor.registrations()), Some(0));
    }

    /// A local connect goes through the same `finish_connecting`, so what this
    /// pins is that the completion questions answer for `AF_UNIX` too — and
    /// that the stream handed back carries bytes.
    #[cfg(unix)]
    #[test]
    fn a_unix_connect_on_a_core_reaches_a_listener() {
        let path = socket_path("reaches");
        let listener =
            std::os::unix::net::UnixListener::bind(&path).expect("the OS refused the path");
        let accepting = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept()?;
            let mut buf = [0_u8; 8];
            let read = stream.read(&mut buf)?;
            Ok::<_, io::Error>(buf[..read].to_vec())
        });

        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));
        let target = path.clone();
        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            let mut stream = NvsUnix::connect(&target).expect("the connect failed");
            assert!(stream.peer_addr().is_ok(), "a connected socket had no peer");
            stream.write_all(b"hi").expect("the write failed");
        });

        run_until_idle(&mut sched).expect("the loop failed");
        let accepted = accepting.join().expect("the accepting thread panicked");
        assert_eq!(
            accepted.expect("the listener never saw the connection"),
            b"hi",
            "the connected stream did not carry bytes"
        );
        assert_eq!(with_current(|reactor| reactor.registrations()), Some(0));
        let _ = std::fs::remove_file(&path);
    }

    /// The refusal half of the same question: nothing is bound at the path, so
    /// a stream is exactly what a caller must not get back.
    #[cfg(unix)]
    #[test]
    fn a_unix_connect_to_nothing_reports_the_failure_rather_than_a_stream() {
        let path = socket_path("refused");
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));
        let outcome = Rc::new(Cell::new(None));
        let reported = Rc::clone(&outcome);
        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            reported.set(Some(
                NvsUnix::connect(&path)
                    .map(|_| ())
                    .map_err(|err| err.kind()),
            ));
        });

        run_until_idle(&mut sched).expect("the loop failed");
        assert_eq!(
            outcome.get(),
            Some(Err(io::ErrorKind::NotFound)),
            "a connect to an unbound path did not report the failure"
        );
        assert_eq!(with_current(|reactor| reactor.registrations()), Some(0));
    }

    /// Off a core the local socket takes the blocking path like every other
    /// stream here, and leaves no registration behind.
    #[cfg(unix)]
    #[test]
    fn a_unix_read_off_a_core_waits_rather_than_refusing() {
        let (server, mut client) =
            std::os::unix::net::UnixStream::pair().expect("the OS refused a socket pair");
        let mut server = NvsUnix::from_std(server).expect("the socket refused non-blocking mode");
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

    /// A bound datagram socket and the plain `std` peer a test speaks to it
    /// with.
    fn bound_pair() -> (NvsUdp, std::net::UdpSocket, SocketAddr) {
        let socket = NvsUdp::bind("127.0.0.1:0".parse().expect("a literal address"))
            .expect("the OS refused a port");
        let addr = socket.local_addr().expect("a bound socket had no address");
        let peer = std::net::UdpSocket::bind("127.0.0.1:0").expect("the OS refused a port");
        (socket, peer, addr)
    }

    /// The datagram half of the same contract: a receive with nothing waiting
    /// registers and parks, the reactor is what ends the park, and the reply
    /// goes back out over the socket the park was taken on.
    #[test]
    fn a_udp_socket_registers_with_the_reactor_and_parks_the_coroutine() {
        let (mut socket, peer, addr) = bound_pair();
        let peer_addr = peer.local_addr().expect("a bound socket had no address");
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));

        let registered = Rc::new(Cell::new(0_usize));
        let counted = Rc::clone(&registered);
        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            let mut buf = [0_u8; 16];
            let (read, from) = socket.recv_from(&mut buf).expect("the receive failed");
            assert_eq!(&buf[..read], b"ping");
            assert_eq!(from, peer_addr, "the datagram named the wrong sender");
            counted.set(usize::from(socket.is_parked_on()));
            socket
                .send_to(b"pong", from)
                .expect("the reply could not be sent");
        });

        // The task runs into the receive, finds nothing and parks — which is
        // where the registration has to already exist, `rule:concurrency/the-reactor-reports-readiness`
        // rule 1.
        let turn = sched.run();
        assert_eq!(turn.parked, 1, "a receive with no datagram waiting ran on");
        assert_eq!(
            with_current(|reactor| reactor.registrations()),
            Some(1),
            "the parked receive filed no registration to be woken by"
        );

        peer.send_to(b"ping", addr).expect("the send failed");
        run_until_idle(&mut sched).expect("the loop failed");
        assert_eq!(
            registered.get(),
            1,
            "the receive that came back out of a park had dropped its registration"
        );
        let mut reply = [0_u8; 16];
        let (read, from) = peer.recv_from(&mut reply).expect("no reply arrived");
        assert_eq!(&reply[..read], b"pong");
        assert_eq!(from, addr, "the reply came from the wrong socket");
    }

    /// The park is a suspension and not a retry loop: with the socket empty the
    /// task is off the run queue, nothing resumes it, and the poller waits out
    /// its whole timeout rather than reporting a readiness to spin on.
    #[test]
    fn a_udp_read_with_no_datagram_waiting_suspends_rather_than_spinning() {
        let (mut socket, peer, addr) = bound_pair();
        let mut sched = Scheduler::new();
        let _installed = install(Reactor::new().expect("the OS refused a poll"));

        let reads = Rc::new(Cell::new(0_usize));
        let counted = Rc::clone(&reads);
        sched.spawn(ctx(), TaskRoot::Worker, move |_ctx| {
            let mut buf = [0_u8; 16];
            let (read, _) = socket.recv_from(&mut buf).expect("the receive failed");
            assert_eq!(&buf[..read], b"late");
            counted.set(counted.get() + 1);
        });

        assert_eq!(sched.run().parked, 1, "the receive did not park");
        // Two turns of the loop with the socket still empty. A receive that
        // retried instead of suspending would take a resume in one of them, and
        // a poller reporting a readiness nobody can use would wake a task.
        for _ in 0..2 {
            let turn = sched.run();
            assert_eq!(turn.resumes, 0, "an empty socket resumed the parked task");
            let woken = with_current(|reactor| {
                reactor
                    .poll(Some(Duration::from_millis(5)))
                    .expect("the poll failed")
                    .len()
            })
            .expect("no reactor is installed");
            assert_eq!(woken, 0, "an empty socket reported itself readable");
            assert_eq!(
                sched.ready_count(),
                0,
                "the parked task went back on the run queue"
            );
        }
        assert_eq!(reads.get(), 0, "the receive answered with nothing sent");

        peer.send_to(b"late", addr).expect("the send failed");
        run_until_idle(&mut sched).expect("the loop failed");
        assert_eq!(reads.get(), 1, "the datagram never reached the parked task");
        assert_eq!(
            with_current(|reactor| reactor.registrations()),
            Some(0),
            "the finished task left its registration behind"
        );
    }
}
