//! `rule:concurrency/connection-bounds-are-finite`'s
//! connection bounds: the numbers an open WebSocket is held inside, every one
//! of them finite before anything is configured.
//!
//! § 7's first bullet lists them, and its load-bearing word is
//! *nothing*: "every bound is finite with nothing configured". A connection is
//! the one thing this server holds that no request ever ends, so a bound left
//! to a later configuration pass is a bound that is absent on every deployment
//! that did not know to write it — which is the shape `rule:programs/memory-priority`'s priority 1
//! refuses. [`Connection::default`] is therefore the whole answer and not a
//! starting point, and the test at the foot of this module is what says so:
//! it destructures the struct, so a bound added here without a finite default
//! fails to compile rather than shipping open.
//!
//! # Which of the bounds are here
//!
//! The ones the framing layer arms: [`crate::socket::Framed`]
//! reads this struct once at the `101` and every wait, every frame and every
//! message on that descriptor is inside it from there.
//! [`Connection::drain`] is a field here without belonging to that bullet — it
//! is § 7's *third* bullet, the period after which a draining server closes a
//! connection, and it lives here because it is one more instant the same
//! framing layer arms a wait by. The bounds § 7 names that are not fields here
//! are owned elsewhere on purpose:
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
//! # Known gap: none of these has a `[server]` key yet
//!
//! § 7 asks for finite, not for configurable, so this module answers § 7 in
//! full. What it does not yet answer is an operator who wants a different
//! number: `nvs_config::tree::Server` has no key for any field here, so
//! changing one is a rebuild. The keys are the obvious follow-on and belong
//! beside [`nvs_config::server::waits_for`]'s, which is where a `[server]`
//! duration is already parsed, refused at zero and given an origin note.
//!
//! — owner: unowned

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

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
    /// § 7's third bullet: how long a connection keeps being served after its
    /// server has begun draining, before it is closed with
    /// [`nvs_runtime::Closing::ShuttingDown`]. `crate::socket`'s `receive` owns
    /// when the period starts and why the close is the connection's own.
    pub drain: Duration,
    /// § 4's per-subscriber delivery queue, restated here so that "every bound"
    /// has one place to be read off. The number is
    /// [`nvs_runtime::INBOX_CAP`]'s and this field is a copy of it, because the
    /// queue is the runtime's and the bound is § 7's.
    pub subscriber_queue: usize,
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
    /// second is picked for what the period is *for*: a connection has no
    /// in-flight request to finish — that is what separates it from the drain
    /// `rule:http-server/the-server-block-is-boot-class` gives an HTTP connection — so what the period buys is the
    /// frame already on the wire and the answer to it, which is one round trip
    /// on any network an origin is proxied over. Longer would hold a deploy
    /// open for clients that are going to reconnect to the next instance
    /// anyway, and the connection is served normally throughout it, so the cost
    /// of the second is a second of shutdown and nothing else.
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
        }
    }
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
    /// `[server]` is `Boot`-class, so a reload does not move it under a stream
    /// already open.
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
}

impl Drop for Heartbeat {
    /// A peer that went away mid-stream ends the beat the same way the stream
    /// ending does: this body is what was being kept alive.
    fn drop(&mut self) {
        self.ended();
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

    use super::{Connection, HEARTBEAT_FLOOR, Slot, heartbeat};

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
        } = Connection::default();

        assert!(max_open > 0, "a process that may hold no connection");
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

    /// The property [`Heartbeat::arm`](super::Heartbeat::arm) rests on: a beat
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
