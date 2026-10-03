//! `hyper`'s two IO traits over each transport this server answers on: the
//! parking stream a request arrives over, and — in [`Nonblocking`], whose own
//! doc is the whole of it — the plain stream the control endpoint is answered
//! on. Everything below is the first one.
//!
//! `rule:concurrency/one-future-per-connection`
//! is this module's specification. The whole of the adapter is one sentence:
//! **try the syscall, and on `WouldBlock` arm the reactor and answer
//! `Pending`** — never suspend, because a suspend inside a `poll` is that
//! ADR's rejected alternative and the deadlock it names. The coroutine would
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
//! `rule:http-server/the-server-block-is-boot-class`
//! 's waits are the stream's own deadline — `hyper` knows nothing about
//! them — and they are **idle** waits rather than totals, so a slow 2 GB upload
//! completes while a stalled socket does not. That is one rule and two
//! mechanisms, both of them in this module because this is the only code that
//! sees a byte move:
//!
//! - **Arm on entry.** Every poll below sets the stream's deadline to
//!   `now + the wait the current [`Phase`] names`, once per phase rather than
//!   once per poll. Re-arming on every poll would be a bound that never fires:
//!   the poll that follows a deadline's own expiry is exactly the one that has
//!   to see it passed, and it would instead push it forward.
//! - **Refresh on progress.** A poll that actually moved bytes re-arms the same
//!   phase, which is what makes the wait idle. A `Pending` refreshes nothing.
//!
//! **Not every phase change is visible here.** A first
//! byte after a response ends the keep-alive wait, and a read attempted while a
//! response was being written is `hyper` going back for the next request — but
//! the end of a request *head* is a framing fact only `hyper` has, so the
//! connection loop sets [`Phase::Body`] and [`Phase::Write`] through the
//! [`ConnectionIo::phase`] handle. What this module cannot see it is told, and
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
//! [`SCRATCH`] bytes of the accepting coroutine's own stack while a read is in
//! flight, and one `memcpy` of at most that much per readable poll. Per
//! connection and only while it is polling, so O(in-flight) and nothing held
//! between polls. AGENTS.md's ordering puts security and simplicity above
//! priority 3, and a bounded `memcpy` on an L1-resident buffer is the cheap
//! side of that trade; if a benchmark ever says otherwise, the thing to change
//! is the buffer's size, and only then the `forbid`.
//!
//! # Known gaps
//!
//! Each gap is a record, and `bun nv gaps --module crates/nvs-server/src/io.rs` lists them.

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

/// The scratch buffer one read borrows from the coroutine's stack.
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
/// from the moment it is accepted: there is no unbounded state to fall into,
/// which is the whole of `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`'s headline stated as a type.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    /// Reading a request head — `header_timeout`.
    Head,
    /// Reading a request body — `body_idle_timeout`. Set by the connection
    /// loop, because framing a head is `hyper`'s knowledge and not this
    /// module's.
    Body,
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
    /// The wait this phase is bounded by.
    fn wait_in(self, waits: &Waits) -> Duration {
        match self {
            Phase::Head => waits.header,
            Phase::Body => waits.body_idle,
            Phase::Write => waits.write_idle,
            Phase::KeepAlive => waits.keepalive,
        }
    }

    /// Whether a connection in this phase is owed no answer, which makes it
    /// one the drain period may close by ending its wait.
    ///
    /// A peer still sending its head, or idle between requests, has no request
    /// to answer. In [`Phase::Body`] the request is running, and
    /// `crate::serve`'s service cuts it at the period's end and answers `503`.
    /// A wait cut here would close the connection before that answer is
    /// written. [`Phase::Write`] is a response already being written. A
    /// streamed one is cut by its own body at the period's end
    /// (`crate::serve::DrainCut`), and a buffered one finishes under its own
    /// wait.
    fn is_owed_nothing(self) -> bool {
        matches!(self, Phase::Head | Phase::KeepAlive)
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
    /// When this connection first saw the drain — [`DrainSeen`].
    drain_seen: DrainSeen,
    /// The drain that bounds this connection's waits, or `None` for a
    /// connection nothing drains — [`ConnectionIo::ending_at_drain`].
    draining: Option<Draining>,
    /// The wake that re-polls this connection when that drain begins, held for
    /// the connection's life: what it wakes is a task parked in the reactor,
    /// and a wake is only a hint, so the poll it provokes is where the drain is
    /// read ([`nvs_host::wake_at_drain`]).
    woken_at_drain: Option<DrainWake>,
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
            drain_seen: DrainSeen::default(),
            draining: None,
            woken_at_drain: None,
        }
    }

    /// Bounds this connection's head and keep-alive waits by the drain period
    /// once `draining` begins, and re-polls a connection already parked so the
    /// drive that owns it reads the drain at once.
    ///
    /// `rule:concurrency/a-drain-closes-a-connection-cleanly` has the period
    /// bound work in progress, so a head still arriving at its end ends in
    /// `TimedOut` like any other expired wait ([`Phase::is_owed_nothing`] says
    /// why the other two phases are not cut here). An idle
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

    /// Puts the current phase's wait on the stream, or refreshes it.
    ///
    /// `progressed` is what makes a wait idle rather than total: bytes moved,
    /// so the clock starts again. Without it the arming is once per phase, for
    /// the module doc's reason — the poll after an expiry is the one that has
    /// to see it.
    fn arm(&mut self, progressed: bool) {
        let phase = self.phase.get();
        // Under a drain the two phases that owe no answer end at the drain
        // period's end at the latest — already in the past, if the period is
        // over, which is `TimedOut` on the next park. [`Phase::is_owed_nothing`]
        // says why the other two are not cut here.
        let closing = (phase.is_owed_nothing()
            && self.draining.as_ref().is_some_and(Draining::is_draining))
        .then(|| self.drain_seen.period_ends(self.waits.drain));
        let armed = (phase, closing.is_some());
        if !progressed && self.armed == Some(armed) {
            return;
        }
        self.armed = Some(armed);
        let mut at = Instant::now() + phase.wait_in(&self.waits);
        if let Some(closing) = closing {
            at = at.min(closing);
        }
        self.stream_mut().set_deadline(Some(at));
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
        // `hyper` reads again only once it is finished writing, so a read
        // attempted in the response phase *is* the end of that response. The
        // one shape this reads early is a body `hyper` drains after answering,
        // and the consequence there is the longer of two waits on a connection
        // that is demonstrably still moving.
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
        let mut scratch = [0_u8; SCRATCH];
        match self.stream.poll_read(&mut scratch[..want]) {
            Poll::Ready(Ok(read)) => {
                // The first byte after a response is the next request's head,
                // which ends the keep-alive wait and starts the header one.
                if read > 0 {
                    if self.phase.get() == Phase::KeepAlive {
                        self.phase.set(Phase::Head);
                    }
                    self.arm(true);
                }
                cursor.put_slice(&scratch[..read]);
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(err)) => Poll::Ready(Err(err)),
            Poll::Pending => Poll::Pending,
        }
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

/// `hyper`'s two IO traits over a plain non-blocking stream, which is the
/// control endpoint's transport and nothing else.
///
/// Everything above is about a socket that must never block a core, and arms a
/// reactor so that the `Pending` it answers is one somebody will end. This is
/// the same sentence with the reactor taken out: `crate::control`'s endpoint is
/// served on one thread of its own, off every core, and there is no reactor
/// there to arm. So a syscall that would wait answers `Pending` with nothing
/// arranged, and the loop that drives the connection —
/// `crate::control::answer_connection` — is what decides when to ask again.
///
/// **A stream that simply blocked instead would deadlock**, which is why the
/// transport underneath is non-blocking at all: `hyper` polls for the next
/// request *before* it writes the answer to the one it is holding, so a read
/// that waited there would be waiting for a client that is waiting for that
/// answer. `nvs_config::control`'s `Stream` is the half that answers
/// `WouldBlock`, and its doc owns the platform spellings.
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
/// cursor directly is what that would take. A control message is a short head
/// and a short body, so it is one pass.
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

    /// Says the writing is finished; the close is the stream's own drop, which
    /// for the control endpoint is what hands the next client its turn.
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
    const EVERY_PHASE: [Phase; 4] = [Phase::Head, Phase::Body, Phase::Write, Phase::KeepAlive];

    /// `rule:http-server/the-server-block-is-boot-class`'s four waits, from both sides: with nothing configured every
    /// phase is bounded by a finite, non-zero wait, and each phase is bounded by
    /// its **own** one.
    ///
    /// The second half is what a per-phase assertion cannot reach. Four defaults
    /// that happen to be finite say nothing about a phase wired to the wrong
    /// field, and two of § 5's four numbers are equal — `body_idle` and
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
            let wait = phase.wait_in(&defaults);
            assert!(
                wait > Duration::ZERO,
                "{phase:?} is unbounded with nothing configured"
            );
            // Not a tautology about `Duration`: § 5's defaults are seconds, and
            // a phase that reached for an unset field would read as zero rather
            // than as anything near this.
            assert!(
                wait <= Duration::from_secs(300),
                "{phase:?} waits {wait:?}, which is not one of § 5's numbers"
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
        let bounds: Vec<Duration> = EVERY_PHASE
            .iter()
            .map(|phase| phase.wait_in(&distinct))
            .collect();
        assert_eq!(
            bounds,
            vec![
                distinct.header,
                distinct.body_idle,
                distinct.write_idle,
                distinct.keepalive
            ],
            "a phase is bounded by a wait that is not its own"
        );
    }
}
