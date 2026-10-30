//! `hyper`'s two IO traits over each transport this server answers on: the
//! parking stream a request arrives over, and — in [`Nonblocking`], whose own
//! doc is the whole of it — a plain non-blocking stream driven off every core.
//! Everything below is the first one.
//!
//! `rule:concurrency/one-future-per-connection`
//! is this module's specification. The whole of the adapter is one sentence:
//! **try the syscall, and on `WouldBlock` arm the reactor and answer
//! `Pending`** — never suspend, because a suspend inside a `poll` is that
//! rule's declined alternative and the deadlock it names. The coroutine would
//! park with the connection future's borrow held and the drive that owns the
//! waker never reached, so the readiness that ends the park would resume a
//! stack standing in the middle of `hyper` rather than in the loop that
//! re-polls it. [`nvs_host::NvsStream::poll_read`] is the half of the stream
//! that stops one step short for exactly this, and
//! [`nvs_host::block_on()`] is where the park it declines to do actually happens.
//!
//! # The waker is not how this connection is woken
//!
//! `Context` is taken and not used, which looks like a bug and is the design.
//! The reactor wakes a task **by id** — it queues the id on the core that
//! parked and pokes that core's poller — so socket readiness never travels
//! through a `Waker` at all. What the waker in `block_on`'s `Context` exists
//! for is the other kind of wake: a `hyper` upgrade holding a clone past the
//! call, or anything else that concludes progress is possible without the
//! socket saying so. Both routes end in the same place, which is why the drive
//! clears its flag before each poll rather than trusting either one.
//!
//! Two things follow, and both are load-bearing. The adapter may not store the
//! waker, because a stored waker would be a second, staler route to the same
//! task. And a `Pending` from here is only honest because
//! [`nvs_host::NvsStream::poll_read`] armed the reactor *before* it answered —
//! `rule:concurrency/the-reactor-reports-readiness`'s rule 1, which is why the arming lives in `nvs-host` beside the
//! registration it touches and not in this module.
//!
//! # What the copy spends, and why it is taken
//!
//! `hyper` hands a read an uninitialised [`ReadBufCursor`], and the way to fill
//! one without initialising it first is an `unsafe` cast of
//! `&mut [MaybeUninit<u8>]` to `&mut [u8]` — handing a syscall a buffer the
//! compiler believes may be read before it is written. This crate inherits the
//! workspace's `unsafe_code = "forbid"` rather than taking another exception
//! to it, so the read goes into a zeroed stack buffer and is copied into the
//! cursor with [`ReadBufCursor::put_slice`].
//!
//! # The clock, and why it lives here
//!
//! `rule:http-server/four-idle-waits-all-finite`'s waits are the stream's own
//! deadline — `hyper` knows nothing about them. Three are **idle** waits, so a
//! slow 2 GB upload completes while a stalled socket does not, and the head's
//! is a **total**, so a peer that trickles a head one byte at a time is closed
//! when `header_timeout` passes. Between the body's end and the answer
//! ([`Phase::Run`]) no wait is on the stream at all: the program is running,
//! the request's own `wall_time` bounds it, and the read `hyper` makes there
//! is a probe for the client's EOF, which an `Ok(0)` still reports. Three
//! mechanisms, all of them in this module because this is the only code that
//! sees a byte move:
//!
//! - **Arm on entry.** Every poll below sets the stream's deadline to
//!   `now + the wait the current [`Phase`] names`, once per phase rather than
//!   once per poll. Re-arming on every poll would be a bound that never fires:
//!   the poll that follows a deadline's own expiry is exactly the one that has
//!   to see it passed, and it would instead push it forward.
//! - **Refresh on progress.** A poll that actually moved bytes re-arms the same
//!   phase, which is what makes the body, write and keep-alive waits idle. A
//!   `Pending` refreshes nothing.
//! - **Fix the head's end once.** A head starts at accept, or at the first byte
//!   after a keep-alive wait, and its deadline is fixed then
//!   (`ConnectionIo::head_ends`). Progress does not refresh it, and a drain
//!   that begins mid-head can only bring it closer.
//!
//! **Not every phase change is visible here.** A first
//! byte after a response ends the keep-alive wait, and a read attempted while a
//! response was being written is `hyper` going back for the next request — but
//! the end of a request *head* or *body* is a framing fact only `hyper` has,
//! so the connection loop sets [`Phase::Body`], [`Phase::Run`] and
//! [`Phase::Write`] through the [`ConnectionIo::phase`] handle. What this module cannot see it is told, and
//! it is told by the one place that knows.
//!
//! **That first rule needs the loop's help for a request that parked**, and
//! `crate::serve`'s service owns the reason: `hyper` skips its own
//! post-response read whenever its read side is already blocked, which it is
//! for every request whose service answered `Pending`, so the read this module
//! reads the end of a response off would not happen at all. The service wakes
//! its connection once more instead of this module guessing.
//!
//! An expired wait surfaces as [`std::io::ErrorKind::TimedOut`] out of the
//! poll, which `hyper` ends the connection on. **A timed-out connection is
//! closed and not answered**: a peer that has not finished a request head is
//! owed no status, and one that has stopped reading is by definition not
//! reading a `408` either.
//!
//! **What that spends**, per `rule:programs/memory-priority`:
//! [`SCRATCH`] bytes of heap per core, zeroed once at the core's first read,
//! and one `memcpy` of at most that much per readable poll. The buffer is the
//! core's rather than the read's, so a read touches no fresh stack and zeroes
//! nothing, and it is not the connection's, because no read parks while it
//! holds the buffer ([`Scratch`]). A connection spends none of it.
//! AGENTS.md's ordering puts security above latency, and a bounded `memcpy` on
//! an L1-resident buffer is the cheap side of that trade; if a benchmark ever
//! says otherwise, the thing to change is the buffer's size, and only then the
//! `forbid`.

use std::cell::Cell;
use std::io;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use hyper::rt::{Read, ReadBufCursor, Write};
use nvs_config::Waits;
use nvs_host::NvsConnection;
use nvs_runtime::DrainWake;

use crate::Draining;
use crate::heads::{Heads, Scanner};

/// The size of the scratch buffer every read of a connection copies through.
///
/// 8 KiB is a socket read's ordinary size and comfortably more than an h1
/// request head, so the common connection — a head, then a small body — is one
/// pass through here. It is a *ceiling* and not a target: a poll asks for
/// `min(what hyper has room for, this)`, so a cursor with less room than this
/// costs less than this.
pub const SCRATCH: usize = 8 * 1024;

/// Which of `rule:http-server/the-server-block-is-boot-class`'s waits bounds this connection right now.
///
/// A connection is always in exactly one of these, starting in [`Phase::Head`]
/// from the moment it is accepted. Every phase but one is bounded by a socket
/// wait, and that one, [`Phase::Run`], is bounded by the request's own
/// `wall_time`: there is no unbounded state to fall into, which is the whole of
/// `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`'s headline
/// stated as a type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    /// Reading a request head — `header_timeout`, the one wait that is a total
    /// for the whole head rather than an idle wait.
    Head,
    /// Reading a request body — `body_idle_timeout`. Set by the connection
    /// loop, because framing a head is `hyper`'s knowledge and not this
    /// module's.
    Body,
    /// The program runs, with its body read to the end or with none — no
    /// socket wait. Set by the connection loop when `hyper` reports the body's
    /// end, or at the head for a request with no body.
    ///
    /// `hyper` reads here while the service is pending, to see a client that
    /// closes (`mid_message_detect_eof`). An idle wait on that read would close
    /// a program that works for longer than the wait with no reply at all, so
    /// the request's `wall_time` is the bound, and the probe's `Ok(0)` still
    /// ends the connection.
    Run,
    /// Writing a response — `write_idle_timeout`. Set by the connection loop
    /// when the request has been answered.
    Write,
    /// Idle between one response and the next request's first byte —
    /// `keepalive_timeout`.
    ///
    /// The phase a drain closes at once: `crate::serve`'s drive asks `hyper`
    /// to shut down once this phase is reached under a drain, which closes an
    /// idle connection on that poll. The drain period's end
    /// ([`ConnectionIo::ending_at_drain`]) is the backstop.
    KeepAlive,
}

/// When one connection first saw the drain, which is when its drain period
/// starts (`rule:concurrency/a-drain-closes-a-connection-cleanly`: the period
/// starts when a connection first sees the drain, not when the drain began).
///
/// One cell shared by the adapter and the request running on the connection,
/// so the waits and the request are cut at the same instant.
#[derive(Clone, Debug, Default)]
pub struct DrainSeen(Rc<Cell<Option<Instant>>>);

impl DrainSeen {
    /// The instant this connection's drain period ends. The first call is the
    /// moment the connection saw the drain, so it starts the period.
    #[must_use]
    pub fn period_ends(&self, period: Duration) -> Instant {
        let seen = self.0.get().unwrap_or_else(|| {
            let now = Instant::now();
            self.0.set(Some(now));
            now
        });
        seen + period
    }
}

impl Phase {
    /// The socket wait this phase is bounded by, or `None` for [`Phase::Run`],
    /// which the request's own `wall_time` bounds.
    fn wait_in(self, waits: &Waits) -> Option<Duration> {
        match self {
            Phase::Head => Some(waits.header),
            Phase::Body => Some(waits.body_idle),
            Phase::Run => None,
            Phase::Write => Some(waits.write_idle),
            Phase::KeepAlive => Some(waits.keepalive),
        }
    }

    /// Whether the drain period's end also ends this phase's wait.
    ///
    /// Every phase but [`Phase::Body`] and [`Phase::Run`]. There the request is
    /// running, and `crate::serve`'s service cuts it at the period's end and
    /// answers `503`; a wait cut here would close the connection before that
    /// answer is written. A response being written is cut at the period's end
    /// whether it is buffered or streamed, and a streamed one is also ended by
    /// its own body at the same instant (`crate::serve::DrainCut`).
    fn ends_at_the_drain(self) -> bool {
        !matches!(self, Phase::Body | Phase::Run)
    }
}

/// One accepted connection, as the two traits `hyper` drives it through.
///
/// Concrete over [`NvsConnection`] rather than generic over the stream:
/// `rule:http-server/two-deployments-and-nothing-a-proxy-owns` gives this
/// listener no TLS, so the only thing a connection here can differ in is which
/// family accepted it — and that enum carries it. A type parameter would put
/// `mio`'s `Source` in this crate's public signatures, and would carry it out
/// through `hyper`'s service into the upgrade path, to say the same thing.
#[derive(Debug)]
pub struct ConnectionIo {
    stream: NvsConnection,
    /// The numbers, fixed for this connection's life: they are the snapshot's
    /// that was published when it was accepted
    /// (`rule:http-server/the-server-block-is-boot-class`), so a reload reaches
    /// the next connection and never moves them under this one.
    waits: Waits,
    /// The phase in force, shared with the connection loop: the module doc
    /// § *The clock* says which changes each side can see.
    phase: Rc<Cell<Phase>>,
    /// The phase the stream's deadline was last armed for, and whether the
    /// drain bounded it, so that a poll which changed nothing does not push
    /// the deadline it is about to be judged against forward — and a poll
    /// that is the first to see the drain does re-arm without progress.
    armed: Option<(Phase, bool)>,
    /// The instant the head being read must be complete by, fixed when that
    /// head starts: at accept, or at the first byte after a keep-alive wait.
    head_ends: Option<Instant>,
    /// When this connection first saw the drain — [`DrainSeen`].
    drain_seen: DrainSeen,
    /// The instant the drain ends the response being written, fixed when that
    /// response is first armed under a drain — [`ConnectionIo::closing`].
    response_ends: Option<Instant>,
    /// The drain that bounds this connection's waits, or `None` for a
    /// connection nothing drains — [`ConnectionIo::ending_at_drain`].
    draining: Option<Draining>,
    /// The wake that re-polls this connection when that drain begins, held for
    /// the connection's life: what it wakes is a task parked in the reactor,
    /// and a wake is only a hint, so the poll it provokes is where the drain is
    /// read ([`nvs_host::wake_at_drain`]).
    woken_at_drain: Option<DrainWake>,
    /// The reader of every request byte on its way into `hyper`
    /// ([`crate::heads`]).
    scanner: Scanner,
    /// Whether the last write found the socket full, shared with the
    /// connection loop: [`ConnectionIo::write_parked`].
    write_parked: Rc<Cell<bool>>,
}

impl ConnectionIo {
    /// Takes over an accepted connection, bounded by `waits`.
    #[must_use]
    pub fn new(stream: NvsConnection, waits: Waits) -> Self {
        Self {
            stream,
            waits,
            phase: Rc::new(Cell::new(Phase::Head)),
            armed: None,
            head_ends: None,
            drain_seen: DrainSeen::default(),
            response_ends: None,
            draining: None,
            woken_at_drain: None,
            scanner: Scanner::default(),
            write_parked: Rc::new(Cell::new(false)),
        }
    }

    /// What [`crate::heads`] found in this connection's request heads, for
    /// the service that answers its requests.
    ///
    /// Shared for [`ConnectionIo::phase`]'s reason: this value is `hyper`'s by
    /// the time there is a request to ask about.
    #[must_use]
    pub fn heads(&self) -> Rc<Heads> {
        self.scanner.heads()
    }

    /// Bounds this connection's head, response and keep-alive waits by the
    /// drain period once `draining` begins, and re-polls a connection already
    /// parked so the drive that owns it reads the drain at once.
    ///
    /// `rule:concurrency/a-drain-closes-a-connection-cleanly` has the period
    /// bound work in progress, so a head still arriving or a response still
    /// being written at its end ends in `TimedOut` like any other expired wait
    /// ([`ConnectionIo::closing`] says which response is given longer, and
    /// [`Phase::ends_at_the_drain`] why the body wait is not cut here). An idle
    /// connection closes sooner than that: the re-poll is what makes its close
    /// prompt, because `crate::serve`'s drive reads [`Phase::KeepAlive`] and
    /// the drain on every poll and asks `hyper` to shut the connection down,
    /// and `hyper` closes an idle one on that same poll. That phase is also
    /// read off `hyper`'s end-of-stream probe during a streaming body, which
    /// is why the close is `hyper`'s to take and not this adapter's: only
    /// `hyper` knows whether a response is still moving. A request whose
    /// program is still running moves no bytes, so no wait here ends it;
    /// `crate::serve`'s service cuts it at the same period through
    /// [`ConnectionIo::drain_seen`]. Called on the task that will drive the
    /// connection, because the wake is issued against that task; a drain that
    /// has already begun registers nothing, and the first poll reads the bit
    /// and arms the same bound.
    ///
    /// What it spends: one registration on the process's drain per connection,
    /// released with this adapter, and one atomic load per arming.
    #[must_use]
    pub fn ending_at_drain(mut self, draining: &Draining) -> Self {
        self.woken_at_drain = nvs_host::wake_at_drain(draining.bit());
        self.draining = Some(draining.clone());
        self
    }

    /// The handle the connection loop moves between the head, the body and the
    /// response.
    ///
    /// A shared cell rather than a method on this type, because by the time
    /// there is a request to frame this value has been moved into `hyper` and
    /// the loop can no longer reach it. Setting it is not itself an arming: the
    /// next poll notices the phase changed and re-arms, which is the only
    /// moment at which a wait can honestly start.
    #[must_use]
    pub fn phase(&self) -> Rc<Cell<Phase>> {
        Rc::clone(&self.phase)
    }

    /// The handle on when this connection first saw the drain, for the
    /// request the connection loop runs on it.
    ///
    /// Shared rather than copied for [`ConnectionIo::phase`]'s reason, and so
    /// that the adapter and the request read one period and not two.
    #[must_use]
    pub fn drain_seen(&self) -> DrainSeen {
        self.drain_seen.clone()
    }

    /// The handle on whether the last write found the socket full.
    ///
    /// `hyper` returns from a poll with a response still owed only when that
    /// response's body has no chunk ready or a write is waiting for room, and
    /// in the second case this is `true`. A whole body or a file always has
    /// its next chunk ready. So once the service has answered with one, a poll
    /// that ends with this `false` is one whose response is all on the wire,
    /// and `crate::serve` gives that response's in-flight place back there.
    /// The phase cannot say this: `hyper` may not read again until the next
    /// request's bytes arrive. Shared for [`ConnectionIo::phase`]'s reason.
    #[must_use]
    pub fn write_parked(&self) -> Rc<Cell<bool>> {
        Rc::clone(&self.write_parked)
    }

    /// Puts the current phase's wait on the stream, or refreshes it.
    ///
    /// `progressed` is what makes a wait idle rather than total: bytes moved,
    /// so the clock starts again. Without it the arming is once per phase, for
    /// the module doc's reason — the poll after an expiry is the one that has
    /// to see it. The head is the exception: its end is fixed when the head
    /// starts, and neither progress nor a drain beginning moves it later.
    fn arm(&mut self, progressed: bool) {
        let phase = self.phase.get();
        let starts = self.armed.map(|(was, _)| was) != Some(phase);
        let closing = self.closing(phase);
        let armed = (phase, closing.is_some());
        if !progressed && self.armed == Some(armed) {
            return;
        }
        self.armed = Some(armed);
        let now = Instant::now();
        let mut at = if phase == Phase::Head {
            let ends = match self.head_ends {
                Some(ends) if !starts => ends,
                _ => now + self.waits.header,
            };
            self.head_ends = Some(ends);
            Some(ends)
        } else {
            phase.wait_in(&self.waits).map(|wait| now + wait)
        };
        if let Some(closing) = closing {
            at = Some(at.map_or(closing, |at| at.min(closing)));
        }
        self.stream_mut().set_deadline(at);
    }

    /// The instant the drain ends this phase's wait at the latest, or `None`
    /// while nothing drains it or the phase is one the drain does not end
    /// ([`Phase::ends_at_the_drain`]).
    ///
    /// That instant is the drain period's end, and once the period is over it
    /// is in the past, which is `TimedOut` on the next park. A response is the
    /// exception, because one can begin after the period is over: the `503`
    /// `crate::serve`'s service answers a request cut at the period's end is
    /// written at that instant. A response that begins once the period is
    /// over is given one `write_idle_timeout` from its first arming, in total,
    /// so a `503` that parks once is still written and a slow reader still
    /// cannot keep the connection open.
    fn closing(&mut self, phase: Phase) -> Option<Instant> {
        if !phase.ends_at_the_drain() || !self.draining.as_ref().is_some_and(Draining::is_draining)
        {
            return None;
        }
        let period_ends = self.drain_seen.period_ends(self.waits.drain);
        if phase != Phase::Write {
            return Some(period_ends);
        }
        if self.armed != Some((Phase::Write, true)) {
            let now = Instant::now();
            self.response_ends = Some(if period_ends > now {
                period_ends
            } else {
                now + self.waits.write_idle
            });
        }
        self.response_ends
    }

    /// The stream underneath, for the caller that has to set a deadline on it.
    ///
    /// `rule:http-server/the-server-block-is-boot-class`'s idle waits are the stream's own bound and not something
    /// `hyper` knows about, so [`ConnectionIo::arm`] reaches through here to
    /// move them between the request head, the body and the response — and so
    /// does a caller that has taken the stream back for an upgrade and owns the
    /// clock from then on.
    pub fn stream_mut(&mut self) -> &mut NvsConnection {
        &mut self.stream
    }

    /// Gives the connection back, which is what closes it.
    ///
    /// `hyper` finishing with a connection is not the same event as the socket
    /// closing: [`Write::poll_shutdown`] below says the writing is done, and
    /// the FIN goes out when this is dropped. Handing the stream back is how a
    /// caller keeps it past that — an upgrade to a WebSocket, which
    /// `rule:concurrency/a-connection-is-a-root-isolate`
    /// makes an isolate over the same descriptor.
    ///
    /// The deadline goes with it: what bounds a WebSocket is that isolate's own
    /// wait and not the response wait this connection happened to be in when
    /// the upgrade was agreed, so the stream is handed over unbounded and the
    /// caller arms it.
    #[must_use]
    pub fn into_stream(mut self) -> NvsConnection {
        self.stream.set_deadline(None);
        self.stream
    }
}

impl Read for ConnectionIo {
    /// The module's one sentence, on the readable interest — and the two phase
    /// changes a read is the evidence for.
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        mut cursor: ReadBufCursor<'_>,
    ) -> Poll<io::Result<()>> {
        // A read attempted in the response phase is taken as the end of that
        // response, because `hyper` goes back for the next request once it is
        // finished writing. Two shapes read early: a body `hyper` drains after
        // answering, and the probe `hyper` makes while a large write is
        // parked. The consequence is the keep-alive wait in place of the write
        // wait, which with the defaults is the longer one. Under a drain both
        // end at the drain period's end ([`ConnectionIo::closing`]).
        if self.phase.get() == Phase::Write {
            self.phase.set(Phase::KeepAlive);
        }
        self.arm(false);
        let want = cursor.remaining().min(SCRATCH);
        if want == 0 {
            // `hyper` has nowhere to put anything, so a syscall here could only
            // read zero bytes and be mistaken for the peer's end of stream.
            return Poll::Ready(Ok(()));
        }
        let this = &mut *self;
        CORE_SCRATCH.with(|scratch| {
            let mut scratch = scratch.borrow_mut();
            let scratch = &mut scratch.get_or_insert_with(Scratch::new).0;
            match this.stream.poll_read(&mut scratch[..want]) {
                Poll::Ready(Ok(read)) => {
                    // The first byte after a response is the next request's head,
                    // which ends the keep-alive wait and fixes that head's deadline.
                    // In the head itself this arming moves nothing.
                    if read > 0 {
                        if this.phase.get() == Phase::KeepAlive {
                            this.phase.set(Phase::Head);
                        }
                        this.arm(true);
                    }
                    this.scanner.feed(&scratch[..read]);
                    cursor.put_slice(&scratch[..read]);
                    Poll::Ready(Ok(()))
                }
                Poll::Ready(Err(err)) => Poll::Ready(Err(err)),
                Poll::Pending => Poll::Pending,
            }
        })
    }
}

thread_local! {
    /// This core's [`Scratch`], made by its first connection's first read.
    static CORE_SCRATCH: std::cell::RefCell<Option<Scratch>> =
        const { std::cell::RefCell::new(None) };
}

/// The [`SCRATCH`] bytes every read on one core lands in before it is copied
/// into `hyper`'s cursor, zeroed once and reused by every connection on the
/// core.
///
/// One per core and not one per connection, because a read fills it and
/// copies out of it without parking: the socket read does not wait, so no
/// other connection's read can run in between. Each read copies only the bytes
/// it just read, so no connection sees another's. Made and freed inside a
/// [`nvs_runtime::budget::Detached`] bracket, because it is the core's and not
/// the request's that happened to read first.
struct Scratch(Box<[u8]>);

impl Scratch {
    fn new() -> Self {
        let _core = nvs_runtime::budget::Detached::begin();
        Self(vec![0_u8; SCRATCH].into_boxed_slice())
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _core = nvs_runtime::budget::Detached::begin();
        drop(std::mem::take(&mut self.0));
    }
}

impl Write for ConnectionIo {
    /// The module's one sentence, on the writable interest.
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.arm(false);
        let written = self.stream.poll_write(buf);
        self.write_parked.set(written.is_pending());
        if matches!(written, Poll::Ready(Ok(bytes)) if bytes > 0) {
            self.arm(true);
        }
        written
    }

    /// Nothing is buffered on this side, so there is nothing to push.
    ///
    /// The stream writes straight through to the socket — `nvs_host::net`'s
    /// module doc says so of its `flush` for the same reason — so this is
    /// `Ready` rather than a syscall that would do nothing.
    fn poll_flush(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let _ = &mut self.stream;
        Poll::Ready(Ok(()))
    }

    /// Says the writing is finished; the FIN is the drop's.
    ///
    /// A half-close here would take the choice away from
    /// [`ConnectionIo::into_stream`], and an upgraded connection reads and
    /// writes on the same descriptor after `hyper` has let go of it.
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.poll_flush(cx)
    }
}

/// `hyper`'s two IO traits over a plain non-blocking stream, for an exchange
/// driven on a thread of its own rather than on a core.
///
/// Everything above is about a socket that must never block a core, and arms a
/// reactor so that the `Pending` it answers is one somebody will end. This is
/// the same sentence with the reactor taken out: off every core there is no
/// reactor to arm. So a syscall that would wait answers `Pending` with nothing
/// arranged, and the loop that drives the connection is what decides when to
/// ask again — the OTLP exporter's cases drive `post` over one this way.
///
/// **A stream that simply blocked instead could deadlock**, which is why the
/// transport underneath is non-blocking at all: `hyper` may poll for the next
/// message *before* it writes the one it is holding, so a read that waited
/// there would be waiting for a peer that is waiting for that write.
///
/// [`Nonblocking::moved`] is how the drive loop tells a connection that is
/// waiting for its peer from one that is making progress, since every poll of a
/// stalled connection looks the same from outside. The flag is set by any poll
/// that carried a byte and read by the loop, which is the same shape
/// [`ConnectionIo`]'s phase handle has and is there for the same reason: only
/// the code that sees a byte move knows one moved.
///
/// The read is the same zeroed-scratch copy `ConnectionIo`'s is, for the same
/// reason: this crate forbids `unsafe`, and filling `hyper`'s uninitialised
/// cursor directly is what that would take.
#[derive(Debug)]
pub struct Nonblocking<S> {
    /// The transport, which answers `WouldBlock` rather than waiting.
    stream: S,
    /// Set by any poll that moved a byte in either direction.
    moved: Rc<Cell<bool>>,
}

impl<S> Nonblocking<S> {
    /// `stream` as the two traits `hyper` drives a connection over.
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            moved: Rc::new(Cell::new(false)),
        }
    }

    /// The progress flag, for the loop driving this connection: set when a poll
    /// has moved a byte, and cleared by whoever reads it.
    #[must_use]
    pub fn moved(&self) -> Rc<Cell<bool>> {
        Rc::clone(&self.moved)
    }
}

impl<S: io::Read + Unpin> Read for Nonblocking<S> {
    /// One read into the cursor: `Ready` with whatever arrived, or `Pending`
    /// where nothing has.
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        mut cursor: ReadBufCursor<'_>,
    ) -> Poll<io::Result<()>> {
        let want = cursor.remaining().min(SCRATCH);
        if want == 0 {
            // `hyper` has nowhere to put anything, so a syscall here could only
            // read zero bytes and be mistaken for the peer's end of stream.
            return Poll::Ready(Ok(()));
        }
        let mut scratch = [0_u8; SCRATCH];
        match interruptible(|| self.stream.read(&mut scratch[..want])) {
            Ok(read) => {
                if read > 0 {
                    self.moved.set(true);
                }
                cursor.put_slice(&scratch[..read]);
                Poll::Ready(Ok(()))
            }
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => Poll::Pending,
            Err(err) => Poll::Ready(Err(err)),
        }
    }
}

impl<S: io::Write + Unpin> Write for Nonblocking<S> {
    /// One write: `Ready` with what went out, or `Pending` where the peer is
    /// not taking any of it.
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match interruptible(|| self.stream.write(buf)) {
            Ok(wrote) => {
                if wrote > 0 {
                    self.moved.set(true);
                }
                Poll::Ready(Ok(wrote))
            }
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => Poll::Pending,
            Err(err) => Poll::Ready(Err(err)),
        }
    }

    /// The stream's own flush, because this side buffers nothing but the one
    /// underneath it may: an answer is not the client's until it is pushed.
    fn poll_flush(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match interruptible(|| self.stream.flush()) {
            Ok(()) => Poll::Ready(Ok(())),
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => Poll::Pending,
            Err(err) => Poll::Ready(Err(err)),
        }
    }

    /// Says the writing is finished; the close is the stream's own drop.
    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.poll_flush(cx)
    }
}

/// `call` again for as long as a signal is what stopped it.
///
/// A blocking syscall is the one place `ErrorKind::Interrupted` reaches this
/// crate at all: the parking stream above never waits in the kernel long enough
/// to be interrupted. `hyper` ends a connection on any error it is handed, so
/// retrying here is what keeps a control answer from being lost to a signal the
/// process handled and carried on from.
fn interruptible<T>(mut call: impl FnMut() -> io::Result<T>) -> io::Result<T> {
    loop {
        match call() {
            Err(err) if err.kind() == io::ErrorKind::Interrupted => (),
            other => return other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every state a connection can be in, so the sweep below is over the whole
    /// type rather than over the ones someone remembered. A variant added
    /// without a line here is a variant with no assertion about its wait, and
    /// the `match` in [`Phase::wait_in`] is what makes the omission a compile
    /// error rather than a silent hole.
    const EVERY_PHASE: [Phase; 5] = [
        Phase::Head,
        Phase::Body,
        Phase::Run,
        Phase::Write,
        Phase::KeepAlive,
    ];

    /// `rule:http-server/the-server-block-is-boot-class`'s four waits, from both sides: with nothing configured every
    /// phase but [`Phase::Run`] is bounded by a finite, non-zero wait, and each
    /// phase is bounded by its **own** one. [`Phase::Run`] has none, because
    /// the request's `wall_time` bounds it
    /// (`rule:http-server/four-idle-waits-all-finite`).
    ///
    /// The second half is what a per-phase assertion cannot reach. Four defaults
    /// that happen to be finite say nothing about a phase wired to the wrong
    /// field, and two of the four shipped waits are equal — `body_idle` and
    /// `write_idle` are both `30s` — so a connection reading a body under the
    /// write wait would read as correct against the defaults alone. The sweep
    /// below gives all four distinct values for exactly that reason.
    ///
    /// The boot refusal of a written `0` or `false` is `nvs_config::server`'s
    /// `E0619` and is asserted there, beside the block it reads. What is here is
    /// the other half of the same claim: that having read four finite numbers,
    /// this module bounds every state with one of them.
    #[test]
    fn the_four_idle_timeouts_are_finite() {
        let defaults = Waits::default();
        for phase in EVERY_PHASE {
            let Some(wait) = phase.wait_in(&defaults) else {
                assert_eq!(phase, Phase::Run, "{phase:?} has no wait");
                continue;
            };
            assert!(
                wait > Duration::ZERO,
                "{phase:?} is unbounded with nothing configured"
            );
            // Not a tautology about `Duration`: the shipped waits are seconds, and
            // a phase that reached for an unset field would read as zero rather
            // than as anything near this.
            assert!(
                wait <= Duration::from_secs(300),
                "{phase:?} waits {wait:?}, which is not one of the shipped waits"
            );
        }

        // Four distinct waits, so each phase's answer names the field it is
        // supposed to name and not merely a plausible number.
        let distinct = Waits {
            header: Duration::from_secs(1),
            body_idle: Duration::from_secs(2),
            write_idle: Duration::from_secs(3),
            keepalive: Duration::from_secs(4),
            // Not a phase's wait, and a fifth distinct number so that a phase
            // reaching for it would be read here as a wrong answer.
            drain: Duration::from_secs(5),
        };
        let bounds: Vec<Option<Duration>> = EVERY_PHASE
            .iter()
            .map(|phase| phase.wait_in(&distinct))
            .collect();
        assert_eq!(
            bounds,
            vec![
                Some(distinct.header),
                Some(distinct.body_idle),
                None,
                Some(distinct.write_idle),
                Some(distinct.keepalive)
            ],
            "a phase is bounded by a wait that is not its own"
        );
    }

    /// An adapter over one end of a loopback connection, under a drain that
    /// has begun, with the far end returned so that the connection stays open.
    fn draining_adapter(waits: Waits) -> (ConnectionIo, std::net::TcpStream) {
        let (io, far) = adapter(waits);
        let draining = Draining::detached();
        draining.begin();
        (io.ending_at_drain(&draining), far)
    }

    /// An adapter over one end of a loopback connection that nothing drains,
    /// with the far end returned so that the connection stays open.
    fn adapter(waits: Waits) -> (ConnectionIo, std::net::TcpStream) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("the OS refused a port");
        let near = std::net::TcpStream::connect(
            listener
                .local_addr()
                .expect("a bound listener had no address"),
        )
        .expect("the loopback refused a connection");
        let (far, _) = listener.accept().expect("the accept failed");
        let stream = nvs_host::NvsTcp::from_std(near).expect("the socket refused non-blocking");
        (ConnectionIo::new(NvsConnection::Tcp(stream), waits), far)
    }

    /// `rule:http-server/four-idle-waits-all-finite`'s one total: bytes of a
    /// head arriving do not move the head's deadline, and the next head after
    /// a keep-alive wait gets a deadline of its own. The body wait beside it
    /// is still refreshed by progress.
    #[test]
    fn a_head_keeps_one_deadline_while_its_bytes_arrive() {
        let (mut io, _far) = adapter(Waits::default());
        io.arm(false);
        let head_ends = deadline(&mut io).expect("a head was armed with no deadline");
        std::thread::sleep(Duration::from_millis(5));
        io.arm(true);
        assert_eq!(
            deadline(&mut io),
            Some(head_ends),
            "a byte of the head moved the head's deadline"
        );

        io.phase().set(Phase::Body);
        io.arm(false);
        let body = deadline(&mut io).expect("a body was armed with no deadline");
        std::thread::sleep(Duration::from_millis(5));
        io.arm(true);
        assert!(
            deadline(&mut io).is_some_and(|at| at > body),
            "a byte of the body did not refresh the body's idle wait"
        );

        // The body's end takes the body wait off the stream, and a byte the
        // EOF probe reads while the program runs does not put it back.
        io.phase().set(Phase::Run);
        io.arm(false);
        assert_eq!(deadline(&mut io), None, "a running program kept a socket wait");
        io.arm(true);
        assert_eq!(deadline(&mut io), None, "a probe's byte armed a socket wait");

        io.phase().set(Phase::KeepAlive);
        io.arm(false);
        io.phase().set(Phase::Head);
        io.arm(true);
        assert!(
            deadline(&mut io).is_some_and(|at| at > head_ends),
            "the next request's head kept the first head's deadline"
        );
    }

    /// The deadline the adapter last put on its stream.
    fn deadline(io: &mut ConnectionIo) -> Option<Instant> {
        match io.stream_mut() {
            NvsConnection::Tcp(stream) => stream.deadline(),
            #[cfg(unix)]
            NvsConnection::Unix(stream) => stream.deadline(),
        }
    }

    /// `rule:concurrency/a-drain-closes-a-connection-cleanly`'s bound on a
    /// response being written: under a drain, the write wait ends at the drain
    /// period's end, and bytes moving do not push it past that end.
    ///
    /// This is asserted on the adapter because `hyper` also reads while a
    /// write is parked, and that read ends [`Phase::Write`] early. A case over
    /// a served connection passes through that read whether this bound exists
    /// or not.
    #[test]
    fn a_response_being_written_ends_at_the_drain_periods_end() {
        let waits = Waits {
            drain: Duration::from_millis(300),
            ..Waits::default()
        };
        let (mut io, _far) = draining_adapter(waits);
        io.phase().set(Phase::Write);
        let before = Instant::now();
        io.arm(false);
        let armed = deadline(&mut io).expect("a response under a drain was armed with no deadline");
        assert!(
            armed <= Instant::now() + waits.drain && armed >= before + waits.drain,
            "a response under a drain is bounded by {:?} from now, which is not the drain period",
            armed.saturating_duration_since(before)
        );
        io.arm(true);
        assert_eq!(
            deadline(&mut io),
            Some(armed),
            "a byte written moved the drain period's end"
        );
    }

    /// A response that begins once the drain period is over is given one
    /// `write_idle_timeout` in total. It is the `503` a request cut by the
    /// drain is answered with, so it is written even if it parks once, and
    /// bytes moving do not extend it.
    #[test]
    fn a_response_begun_after_the_drain_period_gets_one_write_wait_in_total() {
        let waits = Waits {
            drain: Duration::ZERO,
            ..Waits::default()
        };
        let (mut io, _far) = draining_adapter(waits);
        // The first arming is the moment this connection saw the drain, and a
        // zero period ends then.
        io.arm(false);
        io.phase().set(Phase::Write);
        let before = Instant::now();
        io.arm(false);
        let armed = deadline(&mut io).expect("a response under a drain was armed with no deadline");
        assert!(
            armed >= before + waits.write_idle,
            "a response begun after the drain period was cut before one write wait: {:?}",
            armed.saturating_duration_since(before)
        );
        std::thread::sleep(Duration::from_millis(5));
        io.arm(true);
        assert_eq!(
            deadline(&mut io),
            Some(armed),
            "a byte written extended the response's one write wait"
        );
    }
}
