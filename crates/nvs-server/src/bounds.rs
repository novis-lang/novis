//! [ADR 0083](/docs/adr/0083-persistent-connections-are-isolates.md) § 7's
//! connection bounds: the numbers an open WebSocket is held inside, every one
//! of them finite before anything is configured.
//!
//! § 7's first bullet is a list of eight, and its load-bearing word is
//! *nothing*: "every bound is finite with nothing configured". A connection is
//! the one thing this server holds that no request ever ends, so a bound left
//! to a later configuration pass is a bound that is absent on every deployment
//! that did not know to write it — which is the shape `rule:programs/memory-priority`'s priority 1
//! refuses. [`Connection::default`] is therefore the whole answer and not a
//! starting point, and the test at the foot of this module is what says so:
//! it destructures the struct, so a bound added here without a finite default
//! fails to compile rather than shipping open.
//!
//! # Which of the eight are here
//!
//! Six, and they are the six the framing layer arms: [`crate::socket::Framed`]
//! reads this struct once at the `101` and every wait, every frame and every
//! message on that descriptor is inside it from there.
//! [`Connection::drain`] is a seventh field and not a seventh of the eight — it
//! is § 7's *third* bullet, the period after which a draining server closes a
//! connection, and it lives here because it is one more instant the same
//! framing layer arms a wait by. The other two of the eight are named by § 7
//! and owned elsewhere on purpose:
//!
//! - **Connections per process** is [`Slot`], counted here because the resource
//!   is the process's rather than a core's, and taken at the moment the socket
//!   is framed. It is deliberately *not* [`crate::admit`]'s ceiling: that one
//!   counts requests in flight and a request that upgraded has ended, so a
//!   connection holding a coroutine, 128 KiB of codec buffer and a root isolate
//!   would be charged to nothing. The two numbers are the same magnitude for
//!   the same reason — ADR 0106 § 13's arithmetic sizes a slot against what one
//!   isolate may hold, and § 1 gives a connection isolate exactly that budget.
//! - **Connections per tenant** is not a bound of this server's at all, which
//!   is what § 7's parenthetical says: [ADR 0075](/docs/adr/0075-core-ratelimit.md)
//!   applies to the upgrade request "like any other", so a per-tenant ceiling is
//!   the rate limit an application already declares on the route that upgrades.
//!   A second one here would be a second policy over the same request.
//!
//! # What it spends, and what it costs a program
//!
//! Nothing per connection: the struct is seven fields copied into
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
//! number: `nvs_config::tree::Server` has no key for any of the seven, so
//! changing one is a rebuild. The keys are the obvious follow-on and belong
//! beside [`nvs_config::server::waits_for`]'s four, which is where a `[server]`
//! duration is already parsed, refused at zero and given an origin note.

use std::time::Duration;

/// [ADR 0083](/docs/adr/0083-persistent-connections-are-isolates.md) § 7's
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
    /// § 7's eight numbers, chosen against § 1's own worked example rather than
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
    /// ADR 0097 § 5 gives an HTTP connection — so what the period buys is the
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

#[cfg(test)]
mod tests {
    use super::{Connection, Slot};

    /// ADR 0083 § 7's first bullet, as the assertion it is: with nothing
    /// configured, every bound a connection is held inside is a finite number.
    ///
    /// Destructured rather than read field by field, so that a bound added to
    /// [`Connection`] without a default fails this test by failing to compile —
    /// which is the only way a list of eight stays a list of eight, and how
    /// § 7's drain period joined it as a field the moment it existed. Two of
    /// § 7's eight are not fields and are asserted beside them: the process
    /// ceiling is
    /// [`Slot`]'s and is exercised here at a ceiling of one, and the per-tenant
    /// bound is ADR 0075's rate limit on the upgrade request, which is an
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
}
