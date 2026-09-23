//! `rule:concurrency/connection-bounds-are-finite`'s
//! connection bounds: the numbers an open WebSocket is held inside, every one
//! of them finite before anything is configured.
//!
//! § 7's first bullet lists them, and its load-bearing word is
//! *nothing*: "every bound is finite with nothing configured". A connection is
//! the one thing this server holds that no request ever ends, so a bound left
//! to a later configuration pass is a bound that is absent on every deployment
//! that did not know to write it — which is the shape `rule:programs/memory-priority`'s priority 1
//! refuses. [`Connection::default`] is therefore the whole answer a deployment
//! that writes nothing is served under, and the test at the foot of this
//! module is what says so:
//! it destructures the struct, so a bound added here without a finite default
//! fails to compile rather than shipping open.
//!
//! # Which of the bounds are here
//!
//! The ones a door arms. [`crate::socket::Framed`] reads this struct once at
//! the `101` and every wait, every frame and every message on that descriptor
//! is inside it from there. [`EventStream`] reads the same struct on the other
//! door, where there is no codec and no peer speaking: a lifetime, a drain
//! period and a reconnection hint, and — one layer down, at
//! `nvs_runtime::stream::open` — a message, which on that door is one event.
//! Which field each door leaves alone is stated where it is left alone, because
//! an absent bound is the one thing a list of bounds cannot show.
//! [`Connection::drain`] is a field here without belonging to that bullet — it
//! is § 7's *third* bullet, the period after which a draining server ends an
//! event stream that is still being written, and it lives here because it is
//! one more instant the same framing layer arms a wait by. The bounds § 7
//! names that are not fields here are owned elsewhere on purpose:
//!
//! - **Connections per process** is [`Slot`], counted here because the resource
//!   is the process's rather than a core's, and taken at the moment the socket
//!   is framed. It is deliberately *not* [`crate::admit`]'s ceiling: that one
//!   counts requests in flight and a request that upgraded has ended, so a
//!   connection holding a coroutine, 128 KiB of codec buffer and a root isolate
//!   would be charged to nothing. The two numbers are the same magnitude for
//!   the same reason — `rule:http-server/admission-is-arithmetic-not-a-number`'s arithmetic sizes a slot against what one
//!   isolate may hold, and § 1 gives a connection isolate exactly that budget.
//! - **An event stream's keep-alive** is [`Heartbeat`], derived rather than
//!   configured: what closes an idle event stream is the response wait
//!   `nvs_config::server::Waits::write_idle` and not any field here, so the
//!   interval that keeps one open is read off that wait
//!   ([`heartbeat`]) and is a number no operator writes twice.
//! - **Connections per tenant** is not a bound of this server's at all, which
//!   is what § 7's parenthetical says: `rule:core-classes/ratelimit-two-members`
//!   applies to the upgrade request "like any other", so a per-tenant ceiling is
//!   the rate limit an application already declares on the route that upgrades.
//!   A second one here would be a second policy over the same request.
//!
//! # What it spends, and what it costs a program
//!
//! Nothing per connection: the struct is copied field for field into
//! [`crate::socket::Framed`] at the `101`, and the deadline it arms is the one
//! `nvs_host::NvsStream` already carries for every wait. The cost is paid in
//! *behaviour* instead, and it is worth stating plainly because it is
//! observable: a connection that says nothing for [`Connection::idle`] is
//! closed, and one that has been open for [`Connection::lifetime`] is closed
//! however busy it was. An application that wants a longer-lived connection
//! sends anything at all — a ping is a frame, and § 3's loop never sees one.
//!
//! # What a deployment may write, and what it may not
//!
//! Six of these fields are `[server.connection]` keys — `max_open`,
//! `max_frame`, `max_message`, `idle_timeout`, `max_lifetime` and
//! `send_timeout` — read by `nvs_config::server::connection_bounds_for` beside
//! [`nvs_config::server::waits_for`]'s, and applied over the defaults below by
//! [`Connection::configured`]. A key left out keeps its default and a key
//! written as `false` or as zero is refused at boot, so configuring this block
//! can move a bound but cannot remove one — which is § 7's property stated
//! against a file rather than against a rebuild.
//!
//! [`Connection::drain`], [`Connection::subscriber_queue`] and
//! [`Connection::reconnect`] have no key, each for its own reason and none of
//! them an omission. The drain period is sized against one round trip on a
//! proxied network and a connection has no in-flight request to finish, so
//! there is nothing a deployment knows about it that this server does not;
//! the queue depth is `nvs_runtime::INBOX_CAP` and belongs to the runtime that
//! holds the queue; and the reconnection hint is drawn per stream, so a
//! written base would be the one number here an operator could set and not
//! observe.

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use rand::RngExt;

use crate::serve::Draining;

/// `rule:concurrency/connection-bounds-are-finite`'s
/// bounds on one open connection.
///
/// Copied per connection rather than shared, for `nvs_config::server::Waits`'
/// reason: these are boot-class, and a connection already open keeps the
/// numbers it was accepted under across a reload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Connection {
    /// How many connections this process may hold open at once.
    pub max_open: u64,
    /// The largest frame payload the codec will accept, in bytes.
    pub frame: usize,
    /// The largest message — frames reassembled — the codec will accept, in
    /// bytes. Never below [`frame`](Self::frame): a message is at least one.
    pub message: usize,
    /// How long a connection may go without a frame from its peer before it is
    /// closed.
    pub idle: Duration,
    /// How long a connection may stay open at all, however busy.
    pub lifetime: Duration,
    /// How long one `send` may take before it throws, which is § 3's "throws on
    /// the send timeout rather than waiting forever".
    pub send: Duration,
    /// § 7's third bullet: how long an event stream that is still writing
    /// keeps being written after its server has begun draining, before its
    /// body is ended. A stream with nothing to write, and a WebSocket whose
    /// program is waiting on its peer, close the moment they see the drain and
    /// never wait this out
    /// (`rule:concurrency/a-drain-closes-a-connection-cleanly`);
    /// [`EventStream`] owns when the period starts.
    pub drain: Duration,
    /// § 4's per-subscriber delivery queue, restated here so that "every bound"
    /// has one place to be read off. The number is
    /// [`nvs_runtime::INBOX_CAP`]'s and this field is a copy of it, because the
    /// queue is the runtime's and the bound is § 7's.
    pub subscriber_queue: usize,
    /// How long a client is told to wait before reconnecting an event stream,
    /// before the draw — [`reconnect_hint`] is what a stream is actually
    /// opened with, and owns why it is drawn at all.
    ///
    /// A bound on the *client* rather than on this process, and the only field
    /// here that is: what it holds down is the rate a drained fleet comes back
    /// at, which is a number no connection of this server's is open to read.
    pub reconnect: Duration,
}

impl Default for Connection {
    /// § 7's numbers, chosen against § 1's own worked example rather than
    /// against the codec's defaults.
    ///
    /// The example in § 2 gives a connection `{memory: 8mb, idle: 5m}`, and
    /// that is the whole of where these come from. `idle` is transcribed from
    /// it. [`frame`](Connection::frame) and [`message`](Connection::message)
    /// are **below** it on purpose: `tungstenite`'s own defaults are 16 MiB and
    /// 64 MiB, either of which is a single peer frame an 8 MiB connection
    /// cannot hold, so leaving them would make the codec's bound and the
    /// isolate's budget disagree and let the second one be discovered as an
    /// out-of-memory. [`send`](Connection::send) matches
    /// `nvs_config::server::Waits::write_idle`, since a peer that stopped
    /// reading costs the same thing on either protocol.
    ///
    /// [`lifetime`](Connection::lifetime) is the one number no ADR writes, and
    /// a day is picked rather than derived: it is long enough that no
    /// application notices it and short enough that a process cannot accumulate
    /// connections older than a deploy. § 7's own drain closes them sooner on
    /// any host that reloads, so this bound is what catches the host that never
    /// does.
    ///
    /// [`drain`](Connection::drain) is the other number no ADR writes, and a
    /// second is picked for what the period is *for*: an event stream that is
    /// writing when its server stops has no request to finish — that is what
    /// separates it from the drain
    /// `rule:http-server/the-server-block-is-boot-class` gives an HTTP
    /// connection — so what the period buys is the event already in the cell
    /// and the one behind it, which is one round trip on any network an origin
    /// is proxied over. Longer would hold a deploy open for clients that are
    /// going to reconnect to the next instance anyway, and a stream with
    /// nothing to write never waits it out, so the cost of the second is at
    /// most a second of shutdown and nothing else.
    /// [`reconnect`](Connection::reconnect) is the third number no ADR writes,
    /// and three seconds is taken from the client rather than picked: it is
    /// what a browser's `EventSource` waits with no `retry:` field at all, so
    /// emitting it changes how a fleet is *spread* without changing how long
    /// any one client is away. The spreading is [`reconnect_hint`]'s.
    fn default() -> Self {
        Self {
            max_open: 10_000,
            frame: 1 << 20,
            message: 4 << 20,
            idle: Duration::from_secs(5 * 60),
            lifetime: Duration::from_secs(24 * 60 * 60),
            send: Duration::from_secs(30),
            drain: Duration::from_secs(1),
            subscriber_queue: nvs_runtime::INBOX_CAP,
            reconnect: Duration::from_secs(3),
        }
    }
}

impl Connection {
    /// [`Self::default`]'s bounds, with whatever the tree's `[server.connection]` block wrote over
    /// them.
    ///
    /// The block arrives already parsed and already refused — `nvs_config::server`'s
    /// `connection_bounds_for` is what reads the keys, says which were written and rejects the
    /// magnitudes a bound cannot have, so what is left here is the one thing that crate cannot do:
    /// know what the unwritten bounds are. That division is why it hands back overrides rather than
    /// numbers, and it is what keeps [`Self::default`] the only place any of these is stated.
    ///
    /// One written value is changed on the way in, because the struct's own invariant outranks the
    /// file: a [`message`](Self::message) below [`frame`](Self::frame) is raised to it. A message
    /// is at least one frame, so the pair as written would close every connection that sent a full
    /// one — and refusing the block instead would make a deployment that lowered a single bound
    /// fail to start over a combination it can be given the nearest working reading of.
    /// The three fields the block cannot reach — [`drain`](Self::drain),
    /// [`subscriber_queue`](Self::subscriber_queue) and [`reconnect`](Self::reconnect) — are named
    /// in this module's header with why each is the server's own number rather than an operator's.
    #[must_use]
    pub fn configured(written: nvs_config::ConnectionBounds) -> Self {
        let shipped = Self::default();
        let frame = written.frame.map_or(shipped.frame, saturating);
        Self {
            max_open: written.max_open.unwrap_or(shipped.max_open),
            frame,
            message: written
                .message
                .map_or(shipped.message, saturating)
                .max(frame),
            idle: written.idle.unwrap_or(shipped.idle),
            lifetime: written.lifetime.unwrap_or(shipped.lifetime),
            send: written.send.unwrap_or(shipped.send),
            ..shipped
        }
    }
}

/// A written size as a length this process can hold, saturating at [`usize::MAX`].
///
/// The saturation is unreachable on any host this server runs on and is not a policy: a 64-bit
/// target has no size a `u64` can spell and a `usize` cannot, and a smaller one cannot allocate the
/// buffer the larger number asked for either way. Refusing at boot instead would make a deployment
/// that moved one binary to a 32-bit host fail to start over a bound it was already going to hit.
fn saturating(bytes: u64) -> usize {
    usize::try_from(bytes).unwrap_or(usize::MAX)
}

/// The connections this process currently holds open.
///
/// A process-wide relaxed counter, for [`crate::admit`]'s reason stated one
/// resource over: a per-core share would let one core refuse while its
/// neighbours idle, and relaxed is enough because the count is a ceiling on a
/// long-lived thing rather than a sequence anything orders itself against. A
/// `static` rather than a field threaded down from the accept loop because a
/// connection is framed inside `hyper`'s own teardown, three frames below the
/// last place a per-core value is in hand — and because the resource being
/// bounded is the process's memory, which no per-core object speaks for.
static OPEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// One connection's place under [`Connection::max_open`], released on drop.
///
/// Held by [`crate::socket::Framed`], so the count falls when the descriptor
/// closes and by no other route — there is no path on which a connection ends
/// without its codec being dropped, which is exactly why the framing object is
/// the one that carries this.
#[derive(Debug)]
pub struct Slot {
    /// The count this place belongs to, which is [`OPEN`] for every connection
    /// this server frames. Named rather than assumed so that a test can count
    /// against its own and not race the process's — a ceiling is only
    /// assertable at a number small enough to reach, and reaching it in a
    /// shared count would make the assertion depend on which other test is
    /// running.
    count: &'static std::sync::atomic::AtomicU64,
}

impl Slot {
    /// Takes a place under `ceiling`, or answers `None` at it.
    #[must_use]
    pub fn take(ceiling: u64) -> Option<Self> {
        Self::take_from(&OPEN, ceiling)
    }

    /// [`Self::take`], against a named count.
    ///
    /// Reads and increments in one `fetch_add`, then gives the place back when
    /// it was over — the same shape [`crate::admit::Admission::admit`] uses and
    /// for the same reason: a compare-and-swap loop would order a count that
    /// nothing reads twice, and a transient one-over under a race is one
    /// connection, not a breach of the memory arithmetic that sized the number.
    fn take_from(count: &'static std::sync::atomic::AtomicU64, ceiling: u64) -> Option<Self> {
        use std::sync::atomic::Ordering;
        if count.fetch_add(1, Ordering::Relaxed) >= ceiling {
            count.fetch_sub(1, Ordering::Relaxed);
            return None;
        }
        Some(Self { count })
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        self.count
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }
}

/// The shortest interval an idle event stream is written a keep-alive at.
///
/// A floor and not a target: the derivation below halves the response wait, and
/// a deployment that shortened that wait for its own reasons would otherwise
/// buy a stream that writes a comment several times a second for as long as it
/// is open. What it costs where it binds is one three-byte frame a second on a
/// stream that is saying nothing, which is `rule:programs/memory-priority`'s
/// priority 3 spent on priority 1's behalf — the alternative is the connection
/// closing under a program that is working correctly.
pub const HEARTBEAT_FLOOR: Duration = Duration::from_secs(1);

/// How often an event stream that has sent nothing writes
/// `nvs_runtime::sse::KEEPALIVE`, read off the wait that would otherwise close
/// it.
///
/// **Half the wait, and never below [`HEARTBEAT_FLOOR`] while the wait leaves
/// room for it.** Halving is what makes the beat a bound rather than a race: a
/// stream whose beat and whose wait were the same number would be writing its
/// keep-alive at the instant the connection is already being closed for not
/// having written one, and one scheduling delay either way decides which
/// happens. The floor is the other direction, and it yields: where the wait is
/// itself at or under a second there is no interval that is both a second and
/// under the wait, and what an event stream needs is the second of those — a
/// beat at or past the wait is not a beat at all.
///
/// So this is **strictly under `write_idle` at every duration the `[server]`
/// parser can produce** (`nvs_config::server::waits_for`, which refuses zero
/// and accepts every other magnitude), and that is the property
/// [`wake_at`] rests on rather than a nicety.
#[must_use]
pub fn heartbeat(write_idle: Duration) -> Duration {
    let half = write_idle / 2;
    if half < HEARTBEAT_FLOOR && HEARTBEAT_FLOOR < write_idle {
        HEARTBEAT_FLOOR
    } else {
        half
    }
}

/// How far either side of [`Connection::reconnect`] a stream's own hint may be
/// drawn: a third of it.
///
/// Wide enough that a fleet reconnecting over a two-second band is no longer a
/// fleet arriving at one instant, and narrow enough that the hint still reads
/// as the number an operator wrote. Full jitter — uniform from zero — is
/// `crate::db`'s shape one repository over and the wrong one here: this wait is
/// not a backoff retried against a contended resource but the single gap before
/// a client comes back, and a client drawn near zero reconnects into the
/// restart it was told to wait out.
const RECONNECT_SPREAD: u32 = 3;

/// The reconnection wait one event stream is opened with: `base`, spread by up
/// to a third either way.
///
/// **Drawn per stream, and that is the whole point.** Every client of a drained
/// instance is told to come back at the same moment by a constant, so the
/// instance that replaces it takes the whole fleet in one arrival —
/// `rule:http-server/retry-is-opt-in-jittered-and-closed` is the same argument
/// on the outbound side, and § 7's drain is what makes it reachable here: a
/// deploy drains every connection this process holds at once.
///
/// **What it spends**, per `rule:programs/memory-priority`: one draw from
/// `rand::rng()` per open event stream, at priority 3 and on priority 1's
/// behalf — a self-inflicted arrival spike is an availability failure and not a
/// latency one.
#[must_use]
pub fn reconnect_hint(base: Duration) -> Duration {
    let spread = base / RECONNECT_SPREAD;
    let Some(most) = base.checked_add(spread) else {
        return base;
    };
    // Nanoseconds rather than the `Duration` range `rand` could sample
    // directly, because the band is small by construction and the cast is what
    // keeps the draw one integer wide.
    let low = u64::try_from((base - spread).as_nanos()).unwrap_or(u64::MAX);
    let high = u64::try_from(most.as_nanos()).unwrap_or(u64::MAX);
    Duration::from_nanos(rand::rng().random_range(low..=high))
}

/// Where a connection loop reads the instant its event stream next owes a byte,
/// and `None` for a connection writing no event stream.
///
/// A cell shared between the response body and the loop driving the connection,
/// because the two halves of one beat are a `hyper` connection apart: the body
/// is the only thing that knows a poll found nothing to send, and the loop is
/// the only place a deadline can be filed *after* the whole poll pass. That
/// ordering is the whole reason this is not a method on [`Heartbeat`] — see
/// [`wake_at`].
pub type NextBeat = Rc<Cell<Option<Instant>>>;

/// One open event stream's keep-alive clock: how often it owes a byte, and when
/// the next one falls due.
///
/// Held by the response body `crate::serve::Answer` an event stream is written
/// through, because that is the one object that sees both halves — what the
/// program sent and what the connection is about to answer `Pending` with. A
/// beat is owed only where nothing else moved, so every chunk the program sends
/// is a beat ([`Heartbeat::moved`]) and a stream under load never writes one.
///
/// **What it spends**, per `rule:programs/memory-priority`: one cell and two
/// words per open event stream, and one clock read per poll of its body.
/// Nothing per event, and nothing at all on a response that is not one.
#[derive(Debug)]
pub struct Heartbeat {
    /// The interval, from [`heartbeat`] and fixed for this stream's life —
    /// `[server] write_idle_timeout` is `Boot`-class, so a reload does not move
    /// it under a stream already open.
    every: Duration,
    /// When the next beat falls due, pushed forward by every byte that goes out
    /// for any reason, and published for the loop on every change.
    due_at: NextBeat,
}

impl Heartbeat {
    /// The clock an event stream opens with, derived from the wait its
    /// connection writes responses under and published into the loop's cell.
    #[must_use]
    pub fn derived_from(write_idle: Duration, due_at: NextBeat) -> Self {
        let every = heartbeat(write_idle);
        due_at.set(Some(Instant::now() + every));
        Self { every, due_at }
    }

    /// Whether a beat is owed now, starting the next interval where it is.
    ///
    /// Asked on the poll that would otherwise answer `Pending`, so a stream
    /// with a chunk in hand never reaches it.
    pub fn due(&mut self) -> bool {
        let now = Instant::now();
        if self.due_at.get().is_some_and(|at| now < at) {
            return false;
        }
        self.due_at.set(Some(now + self.every));
        true
    }

    /// A byte went out for its own reason, which is everything a beat would
    /// have bought.
    pub fn moved(&mut self) {
        self.due_at.set(Some(Instant::now() + self.every));
    }

    /// The stream is over, so the loop stops waking for it.
    ///
    /// Left unsaid, the cell would keep asking for a wake every interval for as
    /// long as the connection lives — a keep-alive for a response that ended.
    pub fn ended(&mut self) {
        self.due_at.set(None);
    }

    /// Brings the next wake forward to `at`, and leaves it where it is
    /// otherwise.
    ///
    /// A bound closer than the beat is one the loop still has to be woken for,
    /// and this cell is the only wake an event stream has: filing a second
    /// deadline would replace it ([`wake_at`]). Never pushes one back, because
    /// the beat is what keeps the connection open and a wake later than it is
    /// the connection closing.
    pub fn no_later_than(&mut self, at: Instant) {
        if self.due_at.get().is_none_or(|due| at < due) {
            self.due_at.set(Some(at));
        }
    }
}

impl Drop for Heartbeat {
    /// A peer that went away mid-stream ends the beat the same way the stream
    /// ending does: this body is what was being kept alive.
    fn drop(&mut self) {
        self.ended();
    }
}

/// One open event stream's bounds: the keep-alive it writes where nothing else
/// moved, and the instant it is closed at however busy it was.
///
/// Held by the response body `crate::serve::Answer` an event stream is written
/// through and absent from every other response, which is where the two
/// spellings of `rule:concurrency/a-stream-that-outlives-its-request-is-a-connection`
/// part here: a streamed response is bounded by the request writing it, and an
/// event stream outlives every ceiling but the connection's, so it is the one
/// body that needs a table of its own to be held inside.
///
/// **What it reads, and the one field it must not.**
/// [`Connection::lifetime`], [`Connection::drain`] and
/// [`Connection::reconnect`] are this struct's; [`Connection::message`] and
/// [`Connection::send`] are read one layer down where the bytes cross
/// (`nvs_runtime::stream::open`). [`Connection::idle`] is **not armed**, and
/// that absence is the design rather
/// than an omission: it closes a connection whose *peer* stopped speaking, and
/// an event stream's peer never speaks — the hand-over took nothing from it
/// (`rule:concurrency/two-doors-one-isolate`) and there is no frame it could
/// send — so arming it would close every healthy stream at the first window.
/// What bounds the quiet on this door is `nvs_config::server::Waits::write_idle`
/// instead, and [`Heartbeat`] is the answer to it.
///
/// **What it spends**, per `rule:programs/memory-priority`: [`Heartbeat`]'s cell
/// and two words, and one instant beside them, per open event stream.
#[derive(Debug)]
pub struct EventStream {
    /// What keeps the connection open under a stream that is saying nothing.
    beat: Heartbeat,
    /// When this stream is closed however busy it has been, which is
    /// [`Connection::lifetime`] from the moment it opened.
    expires_at: Instant,
    /// The reconnection block this stream opens with, held until the first
    /// poll of the body takes it. Drawn once, at [`Self::opened`], because a
    /// hint redrawn per poll would be a different wait every time the client
    /// read one.
    opening: Option<Vec<u8>>,
    /// The drain of the server this stream is being written by — § 7's third
    /// bullet, read on every poll because a bit is the whole of what
    /// `nvs_runtime::Drain` carries.
    draining: Draining,
    /// The drain period this stream is given once it has seen the drain, and
    /// when that period ends. It starts when the stream first *sees* the drain
    /// rather than when the drain began: a bit is the whole of what
    /// `nvs_runtime::Drain` carries, and per stream the instant is a field of
    /// the object about to act on it. Only a stream that is writing is held to
    /// it — [`Self::drain_begun`] is what ends one that is not.
    drain: Duration,
    closing_at: Option<Instant>,
}

impl EventStream {
    /// The bounds a stream opens under: `bounds`' lifetime from now, the beat
    /// derived from the wait its connection writes responses under, and the
    /// reconnection hint drawn for this stream alone.
    #[must_use]
    pub fn opened(
        bounds: &Connection,
        draining: &Draining,
        write_idle: Duration,
        due_at: NextBeat,
    ) -> Self {
        Self {
            beat: Heartbeat::derived_from(write_idle, due_at),
            expires_at: Instant::now() + bounds.lifetime,
            opening: Some(nvs_runtime::sse::reconnect_after(reconnect_hint(
                bounds.reconnect,
            ))),
            draining: draining.clone(),
            drain: bounds.drain,
            closing_at: None,
        }
    }

    /// The bytes this stream owes before any event, taken once.
    ///
    /// A byte that moved like any other, so the beat starts again from it: a
    /// stream that has just written its hint does not owe a keep-alive as well.
    pub fn opening(&mut self) -> Option<Vec<u8>> {
        let opening = self.opening.take()?;
        self.beat.moved();
        Some(opening)
    }

    /// Whether this stream has met a bound that ends it — its lifetime, or the
    /// drain period of a server that has begun shutting down.
    ///
    /// Asked on each poll of the body rather than filed as a deadline of its
    /// own, because [`wake_at`] keeps one instant per connection task and the
    /// beat is already it — a second entry would replace the one the stream is
    /// being kept alive by. Asking is enough for the lifetime on its own: a
    /// stream with nothing to send is polled every beat, and one with something
    /// to send is polled for every chunk. The drain is the case where that is
    /// not enough, since a shutdown waiting a beat for each stream is a
    /// shutdown taking as long as the slowest wait a deployment configured — so
    /// seeing the drain brings the next wake forward to the close it schedules.
    #[must_use]
    pub fn over(&mut self) -> bool {
        Instant::now() >= self.expires_at || self.drained()
    }

    /// Whether the drain period this stream was given has run out, starting it
    /// the first time the drain is seen.
    fn drained(&mut self) -> bool {
        if self.closing_at.is_none() && self.draining.is_draining() {
            let closing_at = Instant::now() + self.drain;
            self.closing_at = Some(closing_at);
            self.beat.no_later_than(closing_at);
        }
        self.closing_at
            .is_some_and(|closing_at| Instant::now() >= closing_at)
    }

    /// Whether the server this stream is written by has begun draining — read
    /// by the body's poll where it found nothing to write, which is where a
    /// stream with nothing to write ends
    /// (`rule:concurrency/a-drain-closes-a-connection-cleanly`).
    #[must_use]
    pub fn drain_begun(&self) -> bool {
        self.draining.is_draining()
    }

    /// Whether a keep-alive is owed now — [`Heartbeat::due`].
    pub fn due(&mut self) -> bool {
        self.beat.due()
    }

    /// A byte went out for its own reason — [`Heartbeat::moved`].
    pub fn moved(&mut self) {
        self.beat.moved();
    }

    /// The stream is over, so the loop stops waking for it —
    /// [`Heartbeat::ended`].
    pub fn ended(&mut self) {
        self.beat.ended();
    }
}

/// Files `at` as this connection task's deadline, so the park that follows a
/// `Pending` ends in time to write the beat that is due then.
///
/// **Called from the connection loop and after the whole poll pass**, which is
/// the ordering the design rests on rather than a detail: the socket files its
/// own deadline from inside a poll (`nvs_host::NvsStream::poll_read`), and
/// `nvs_host::Timers` keeps one deadline per task, so a beat filed mid-pass
/// would be replaced by whichever poll came after it. Filing last makes the
/// beat the entry that survives.
///
/// **Replacing the socket's entry is sound for exactly one reason**:
/// [`heartbeat`] is strictly under `write_idle`, so the beat is always the
/// earlier instant and the wake it arranges always comes first. The response
/// wait is still what closes the connection — the socket re-reads its own
/// deadline on every poll and refuses one that has passed — and a peer that
/// stopped reading is discovered by the write a beat itself is.
///
/// Nothing is disarmed when the stream ends: [`Heartbeat::ended`] empties the
/// cell instead, so no further beat is filed, and the deadline already there
/// costs one extra poll of a connection that is about to be polled anyway.
/// Taking it back would take the socket's own entry with it, and that one is
/// what bounds a park nothing else ends.
pub fn wake_at(at: Instant) {
    let Some(me) = nvs_host::current_task() else {
        return;
    };
    let _ = nvs_host::reactor::with_current(|reactor| reactor.timers().arm(me, at));
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{Connection, HEARTBEAT_FLOOR, Slot, heartbeat, reconnect_hint};

    /// `rule:concurrency/connection-bounds-are-finite`'s first bullet, as the assertion it is: with nothing
    /// configured, every bound a connection is held inside is a finite number.
    ///
    /// Destructured rather than read field by field, so that a bound added to
    /// [`Connection`] without a default fails this test by failing to compile —
    /// which is the only way the list stays complete. The bounds § 7 names
    /// that are not fields are asserted beside them: the process
    /// ceiling is
    /// [`Slot`]'s and is exercised here at a ceiling of one, and the per-tenant
    /// bound is `rule:core-classes/ratelimit-two-members`'s rate limit on the upgrade request, which is an
    /// application's declaration and not a number this server holds.
    #[test]
    fn every_connection_bound_is_finite_with_nothing_configured() {
        let Connection {
            max_open,
            frame,
            message,
            idle,
            lifetime,
            send,
            drain,
            subscriber_queue,
            reconnect,
        } = Connection::default();

        assert!(max_open > 0, "a process that may hold no connection");
        assert!(
            !reconnect.is_zero(),
            "a client told to reconnect immediately is a client told to flood"
        );
        assert!(frame > 0, "a frame bound of zero accepts nothing");
        assert!(
            message >= frame,
            "a message is at least one frame, so {message} may not be under {frame}"
        );
        assert!(!idle.is_zero(), "an idle bound of zero closes on arrival");
        assert!(!lifetime.is_zero(), "a connection allowed no lifetime");
        assert!(!send.is_zero(), "a send that may never be attempted");
        // Finite in the other direction as well, which is the half a `Duration`
        // makes easy to forget: a drain longer than the idle window would be no
        // bound at all, the connection being closed on the idle clock first.
        assert!(
            !drain.is_zero() && drain < idle,
            "a drain period of {drain:?} is not a bound a shutdown reaches, \
             against an idle window of {idle:?}"
        );
        assert_eq!(
            subscriber_queue,
            nvs_runtime::INBOX_CAP,
            "§ 4's queue depth is the runtime's number and this is a copy of it"
        );
        // The two magnitudes § 1's own example is what these are read against: a
        // frame and a message must both fit inside the 8 MiB budget it gives a
        // connection isolate, or the codec's bound is not the one that binds.
        assert!(
            message < 8 << 20,
            "a message the connection's own memory budget cannot hold"
        );

        // The process ceiling, at the smallest number that can be over-run, and
        // against a count of this test's own: the module's own is the live one,
        // and a ceiling of one asserted against it would answer whichever other
        // test in this binary happened to hold a connection open.
        static COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let first = Slot::take_from(&COUNT, 1).expect("the first connection is under a ceiling");
        assert!(
            Slot::take_from(&COUNT, 1).is_none(),
            "a second connection was admitted over a ceiling of one"
        );
        drop(first);
        assert!(
            Slot::take_from(&COUNT, 1).is_some(),
            "the place was not given back when the connection closed"
        );
    }

    /// The six bounds a deployment may move, from the block it writes them in
    /// to the table a connection is then framed inside.
    ///
    /// Written end to end rather than against
    /// `nvs_config::server::connection_bounds_for` alone, because the failure
    /// this guards is the seam: a key that parses, is refused correctly and
    /// then reaches no field is exactly the setting an operator writes and
    /// never observes. Every written number is deliberately unlike the shipped
    /// one, so a mapping that dropped a key — or crossed two of them — fails
    /// here rather than reading as a pass over a value that was already right.
    ///
    /// The three fields the block cannot reach are asserted as *unchanged* by a
    /// block that writes everything it can, which is the only way a key added
    /// to one of them later cannot land silently. The partial block is the
    /// second half of `rule:config/later-wins-and-every-override-is-recorded`'s
    /// per-key override: a written `[server.connection]` is a decision per key
    /// and not one decision about the table.
    #[test]
    fn server_connection_bounds_are_read_from_the_server_block() {
        let shipped = Connection::default();

        let written = configured(
            "[server.connection]\n\
             max_open = 64\n\
             max_frame = 262144\n\
             max_message = 524288\n\
             idle_timeout = \"90s\"\n\
             max_lifetime = \"2h\"\n\
             send_timeout = \"5s\"\n",
        );
        assert_eq!(written.max_open, 64, "the open-connection ceiling");
        assert_eq!(written.frame, 262_144, "the frame bound");
        assert_eq!(written.message, 524_288, "the message bound");
        assert_eq!(written.idle, Duration::from_secs(90), "the idle bound");
        assert_eq!(
            written.lifetime,
            Duration::from_secs(2 * 60 * 60),
            "the lifetime bound"
        );
        assert_eq!(written.send, Duration::from_secs(5), "the send bound");
        assert_eq!(
            (written.drain, written.subscriber_queue, written.reconnect),
            (shipped.drain, shipped.subscriber_queue, shipped.reconnect),
            "a block reached a bound it has no key for"
        );

        // One key, and the other five are the numbers this server ships.
        let partial = configured("[server.connection]\nidle_timeout = \"11s\"\n");
        assert_eq!(partial.idle, Duration::from_secs(11));
        assert_eq!(
            Connection {
                idle: shipped.idle,
                ..partial
            },
            shipped,
            "an unwritten key did not keep its own default"
        );

        // A message under the frame bound is raised to it: the pair as written
        // would close every connection that sent a full frame, which
        // `Connection::configured` owns.
        let raised = configured("[server.connection]\nmax_frame = 262144\nmax_message = 1024\n");
        assert_eq!(raised.message, raised.frame);

        // And neither magnitude that would remove a bound is a value this block
        // has, whichever unit it is written in.
        for text in [
            "[server.connection]\nmax_open = 0\n",
            "[server.connection]\nmax_frame = 0\n",
            "[server.connection]\nidle_timeout = \"0s\"\n",
            "[server.connection]\nmax_lifetime = false\n",
            "[server.connection]\nsend_timeout = false\n",
        ] {
            let refusal = bounds_for(text).expect_err("a bound with no bound in it was accepted");
            assert_eq!(
                refusal.code,
                Some(nvs_diagnostics::code::E_CONNECTION_BOUND_REMOVED),
                "refused under the wrong code: {text:?}"
            );
        }
    }

    /// One fixture block, from its text to the table a connection is framed
    /// inside — the boot path `nvs_cli::serve` takes, with the origins map a
    /// refusal would be pointed at left empty.
    fn configured(text: &str) -> Connection {
        Connection::configured(bounds_for(text).expect("the fixture is a block of bounds"))
    }

    /// [`configured`]'s first half, kept apart so a case can assert the refusal.
    fn bounds_for(text: &str) -> Result<nvs_config::ConnectionBounds, nvs_diagnostics::Diagnostic> {
        let config: nvs_config::Config = toml::from_str(text).expect("the fixture did not parse");
        nvs_config::server::connection_bounds_for(&config, &std::collections::BTreeMap::new())
    }

    /// The same table read for the *other* half of finite: **no default is a
    /// number standing in for "no ceiling"**, and neither is anything derived
    /// from one.
    ///
    /// `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect` refuses
    /// `0` as a spelling for an absent bound, which the case above is; a bound
    /// spelled as the largest number its type can hold is the same defect
    /// written the other way, and it reads in a dump exactly like a bound that
    /// binds. Each field is asserted against what would make it unreachable
    /// rather than against a literal, so a number deliberately changed does not
    /// drag this case along with it — and the two numbers *derived* from the
    /// defaults are read here too, because a field inside its ceiling whose
    /// derivation is not is a bound nothing meets either.
    #[test]
    fn connection_defaults_are_finite_for_every_field() {
        // Longer than any bound here may be and still be one a running process
        // meets: a wait past this is a wait a deployment restarts before it
        // reaches.
        const REACHABLE: Duration = Duration::from_secs(7 * 24 * 60 * 60);

        let Connection {
            max_open,
            frame,
            message,
            idle,
            lifetime,
            send,
            drain,
            subscriber_queue,
            reconnect,
        } = Connection::default();

        // Counted through `try_from` rather than a cast, which is the lint
        // policy's shape and not a worry about the numbers: a bound that will
        // not fit a `u64` is one this assertion would have failed anyway.
        let counted = |bound: usize| u64::try_from(bound).unwrap_or(u64::MAX);
        for (named, bound) in [
            ("max_open", max_open),
            ("frame", counted(frame)),
            ("message", counted(message)),
            ("subscriber_queue", counted(subscriber_queue)),
        ] {
            assert!(
                bound < u64::from(u32::MAX),
                "`{named}` is {bound}, which is a ceiling nothing reaches rather than one that binds"
            );
        }
        for (named, wait) in [
            ("idle", idle),
            ("lifetime", lifetime),
            ("send", send),
            ("drain", drain),
            ("reconnect", reconnect),
        ] {
            assert!(
                wait < REACHABLE,
                "`{named}` is {wait:?}, which is longer than a process runs"
            );
        }

        // The keep-alive, which is derived from a wait rather than written, and
        // the reconnection hint, which is drawn from a field rather than sent
        // as it stands. `heartbeat`'s own cases sweep every wait a parser
        // accepts; what is read here is the pair a deployment gets with nothing
        // configured.
        let served_under = nvs_config::Waits::default().write_idle;
        let beat = heartbeat(served_under);
        assert!(
            !beat.is_zero() && beat < served_under,
            "a beat of {beat:?} is not inside the {served_under:?} wait it exists to stay under"
        );
        let spread = reconnect / 3;
        for _ in 0..64 {
            let hint = reconnect_hint(reconnect);
            assert!(
                hint >= reconnect - spread && hint <= reconnect + spread,
                "a hint of {hint:?} is outside the band around {reconnect:?}"
            );
        }
    }

    /// [`heartbeat`]'s derivation, stated as the two clauses it is: half the
    /// response wait, and not below [`HEARTBEAT_FLOOR`].
    ///
    /// Asserted over every wait the floor leaves room inside, which is every
    /// one above a second — below that the two clauses cannot both hold and the
    /// floor is the one that yields, which is what the test below is the whole
    /// statement of. The default is read from `nvs_config::Waits` rather than
    /// written here, so that a change to the response wait moves this case with
    /// it instead of leaving it asserting a number nothing serves under.
    #[test]
    fn the_heartbeat_is_half_the_write_idle_wait_and_never_below_one_second() {
        let served_under = nvs_config::Waits::default().write_idle;
        assert_eq!(
            heartbeat(served_under),
            served_under / 2,
            "the wait this server writes responses under does not halve"
        );

        for millis in [1_001_u64, 1_500, 1_999, 2_000, 2_001, 3_000, 10_000, 30_000] {
            let wait = Duration::from_millis(millis);
            assert_eq!(
                heartbeat(wait),
                (wait / 2).max(HEARTBEAT_FLOOR),
                "a beat derived from {wait:?} is neither half of it nor the floor"
            );
            assert!(
                heartbeat(wait) >= HEARTBEAT_FLOOR,
                "a stream under {wait:?} beats more often than once a second"
            );
        }
    }

    /// The property [`wake_at`](super::wake_at) rests on: a beat
    /// is strictly earlier than the wait it exists to stay inside, whatever the
    /// `[server]` block wrote.
    ///
    /// A beat at or past the wait would be a connection closed for not writing
    /// the byte it was about to write, and — because the beat is filed as the
    /// connection task's one deadline — a wake arranged for after the instant
    /// it was arranged to beat before. So the sweep runs the tight region a
    /// nanosecond at a time, where the floor and the wait cross, and the whole
    /// magnitude range by doubling: `waits_for` refuses zero and accepts every
    /// other duration it can spell, so those are the values a server can be
    /// serving under.
    #[test]
    fn the_heartbeat_is_strictly_under_the_write_idle_wait_for_every_accepted_value() {
        let mut accepted: Vec<Duration> = (1..=5_000_u64).map(Duration::from_nanos).collect();
        accepted.extend((1..=5_000_u64).map(Duration::from_millis));
        let mut nanos = 1_u64;
        while let Some(doubled) = nanos.checked_mul(2) {
            accepted.push(Duration::from_nanos(nanos));
            nanos = doubled;
        }
        accepted.push(Duration::from_nanos(u64::MAX));

        for wait in accepted {
            let beat = heartbeat(wait);
            assert!(
                beat < wait,
                "a stream under {wait:?} beats every {beat:?}, which is not \
                 inside the wait it has to move a byte before"
            );
            // Zero is reachable at exactly one wait — one nanosecond, where no
            // non-zero interval is under it — and a connection written that is
            // closed by its own deadline before a beat could matter.
            assert!(
                !beat.is_zero() || wait == Duration::from_nanos(1),
                "a beat of no length at all, under a wait of {wait:?}"
            );
        }
    }
}
