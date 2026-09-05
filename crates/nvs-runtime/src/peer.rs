//! The peer a connection isolate talks to —
//! [ADR 0083](/docs/adr/0083-persistent-connections-are-isolates.md) § 1's
//! socket, as the one thing that crosses from the server into the isolate.
//!
//! § 1 moves the socket into the root isolate the upgrade opened. The socket
//! itself is a `nvs_host::NvsTcp` with RFC 6455's framing over it, and neither
//! of those names exists here: this crate is underneath both, and the isolate's
//! [`Ctx`](crate::Ctx) is what has to carry the thing. So what crosses is a **trait
//! object** — the same seam shape [`crate::host`] uses for the scheduler, and
//! for the same reason. `nvs_server::socket` implements it over the stream the
//! upgrade handed back, `Core\Socket` calls it from inside the isolate, and
//! neither has to name the other's crate.
//!
//! # What a frame is, and what it is not
//!
//! Two payloads, because RFC 6455 has two — text and binary — and a `Core`
//! member is what decides which one a Novis value becomes. Control frames are
//! **absent on purpose**: a ping, a pong and a close are the framing layer's
//! own conversation, answered by the implementation before a caller ever sees
//! a frame, and a program that could send a close out of order would be able
//! to desynchronise a handshake this seam exists to keep. Closing is
//! [`PeerSocket::close`] and nothing finer.
//!
//! [`PeerSocket::receive`] answering `None` is the peer having closed, which is
//! the `null` § 3's loop reads as its condition. It is not an error, and an
//! implementation that reported one for an orderly close would turn every
//! ordinary disconnect into a throw.
//!
//! **A received payload is untrusted input** — § 3 says `tainted`, and
//! [ADR 0024](/docs/adr/0024-taint-tracking-for-injection-sinks.md) § 3 is what
//! that qualifier costs a caller. Nothing here can apply it: a qualifier is a
//! *signature*'s, so the `Core\Socket` row that hands the payload to a program
//! is where it is declared, and this seam carries bytes.
//!
//! # Decision: the queue is bounded, and the overflow closes the subscriber
//!
//! ADR 0083 § 4's priority-1 rule: each subscriber's queue is capped, the
//! publisher is never blocked, and an overflow closes *that* subscriber. An
//! [`Inbox`] therefore holds [`INBOX_CAP`] deliveries and refuses the next one,
//! and the refusal is **sticky** — [`Inbox::overflowed`] stays true once it has
//! been raised, because the fact it records is not "the queue is full now" but
//! "this connection missed a value it had subscribed to", which draining cannot
//! undo.
//!
//! **Who closes is the question this shape answers.** The publisher cannot: the
//! peer is a field of the *subscriber's* [`Ctx`](crate::Ctx), which is another
//! isolate's stack frame and may be on another core, and a publisher reaching
//! into it would be the shared mutable state § 4 declines. The [`Inbox`] is the
//! one object both sides already hold, so the overflow is raised on it and the
//! subscriber's own next `receive()` obeys — it closes its peer with
//! [`Closing::SlowSubscriber`]'s code and answers § 3's `null`, which is the
//! condition the connection loop already ends on. `nvs_stdlib::socket` owns
//! that half; nothing about the publisher's call changes, which is what "never
//! blocked" means here.
//!
//! [`slow_subscribers_closed`] is § 4's "a metric increments", as a per-core
//! count. **Known gap:** nothing exports it yet — it is not one of
//! [ADR 0076](/docs/adr/0076-observability-export.md) § 1's series, and giving
//! it one is `nvs_server::metrics`' edit, in the crate that owns the registry.
//! The count is maintained either way, so the series is a wiring change rather
//! than an instrumentation one.
//!
//! # What it spends
//!
//! Up to [`INBOX_CAP`] deliveries per **live connection that subscribed to
//! something** — a `Box<str>` and a 16-byte value each, so about 8 KiB of
//! queue slots, plus the published copies themselves, which are the publisher's
//! allocations and are bounded by its own `[limits] memory`. It is
//! O(live connections), never O(publishes), which is the whole point of the
//! cap: a subscriber that stops reading is closed rather than accumulated.
//!
//! Beyond that: one boxed trait object per connection isolate, and one buffer per frame in
//! flight — a `String` or a `Vec<u8>` the caller is handed and then owns.
//! Neither is O(frames received): a frame is released with the value it became.
//! The implementation's own read buffer is charged to the connection, which is
//! ADR 0004's "attributable, O(in-flight)" with a connection as the unit.

/// One RFC 6455 payload, in either direction.
///
/// The module doc owns why there are two variants and no control frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PeerFrame {
    /// A text frame's payload, which RFC 6455 requires to be valid UTF-8 —
    /// so the framing layer has already checked it and a caller never has to.
    Text(String),
    /// A binary frame's payload, which is bytes and is not checked.
    Binary(Vec<u8>),
}

/// One value published to a topic this connection subscribed to, queued for
/// the next [`PeerSocket::receive`] the isolate performs.
///
/// ADR 0083 § 3's **second source**. It lives here rather than in `nvs-stdlib`
/// for the reason [`PeerSocket`] does: the queue it waits in is the isolate's,
/// so [`Ctx::deliver`](crate::Ctx::deliver) has to be able to name the type,
/// and a `Ctx` cannot name a type that crate declares. § 4's bus is what
/// pushes one — `nvs_stdlib::topic`'s `publish` — and it reaches this queue
/// through the [`Inbox`] handle the subscriber table holds rather than through
/// a context it has no way to name.
///
/// **The value is one owned reference.** Whoever takes a `Delivery` out of the
/// queue owes [`Self::into_value`] and, if it does not hand the reference on,
/// a release — the same convention `Ctx::set_isolate_argument` keeps for the
/// other value a context holds. There is deliberately no `Drop`: a release
/// needs the context that allocated it, and a `Delivery` does not carry one.
#[derive(Debug)]
pub struct Delivery {
    /// The topic's name, which is what tells a delivery from a peer frame at
    /// the one member that answers both (§ 3).
    topic: Box<str>,
    /// The published value, already copied across the boundary by the
    /// publisher — [ADR 0023](/docs/adr/0023-clone-serialize-and-cross-boundary-copy.md)
    /// § 2, as § 4 requires of anything the bus hands a subscriber.
    value: crate::Value,
}

impl Delivery {
    /// Builds one from the topic it arrived on and the value that crossed.
    ///
    /// Takes over the value's reference.
    #[must_use]
    pub fn new(topic: impl Into<Box<str>>, value: crate::Value) -> Self {
        Self {
            topic: topic.into(),
            value,
        }
    }

    /// The topic's name.
    #[must_use]
    pub fn topic(&self) -> &str {
        &self.topic
    }

    /// The value, handing the reference on to the caller.
    #[must_use]
    pub fn into_value(self) -> crate::Value {
        self.value
    }
}

/// How many deliveries one connection's queue holds before the next is refused
/// and that connection is closed.
///
/// The module doc owns why an overflow closes the subscriber and what the queue
/// spends. The number is a *count* rather than a size, because the bytes are
/// already bounded twice over: a published value is a graph copy the publisher
/// allocated under its own `[limits] memory`, and the connection holding them
/// has a memory limit of its own (§ 1). 256 is far more than the gap between a
/// publish and the `receive()` that drains it — a connection that is running at
/// all empties its whole queue at every wait — so reaching it means this
/// subscriber has not run for 256 publishes, which is § 4's "slow subscriber"
/// and not a burst.
pub const INBOX_CAP: usize = 256;

thread_local! {
    /// How many subscribers this core has closed for overflowing — § 4's "a
    /// metric increments", read back by [`slow_subscribers_closed`].
    static SLOW_SUBSCRIBERS_CLOSED: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// How many subscribers **this core** has closed for missing a delivery.
///
/// ADR 0083 § 4's metric, as the number an exporter would read. It is a core's
/// own count and never a process-wide one, which is where
/// [ADR 0076](/docs/adr/0076-observability-export.md) § 7 already charges a
/// series; the module doc records that nothing exports it yet.
#[must_use]
pub fn slow_subscribers_closed() -> u64 {
    SLOW_SUBSCRIBERS_CLOSED.with(std::cell::Cell::get)
}

/// The queue one connection's deliveries wait in — ADR 0083 § 3's second
/// source, as a thing two owners can hold.
///
/// [`Ctx::deliver`](crate::Ctx::deliver) fills it and
/// [`Ctx::take_delivery`](crate::Ctx::take_delivery) drains it, which is all a
/// connection needs. It is a separate allocation, behind an
/// [`Rc`](std::rc::Rc), for the *other* owner: § 4's subscriber table has to be
/// able to reach a subscriber's queue from outside that subscriber's own call
/// stack, and a `Ctx` is a stack frame's — it cannot be named from a table that
/// outlives any one isolate. So the queue moves out of the context and the
/// context keeps a handle onto it.
///
/// **The table holds a [`Weak`](std::rc::Weak) and the context the only
/// strong**, which is what keeps § 4's bookkeeping O(live connections) rather
/// than O(connections served): the isolate ending drops the last strong
/// reference, and a subscription to a connection that is gone is a dead entry
/// the next walk of that topic drops. Nothing has to unsubscribe on the way
/// out, which matters because a connection that ended at a limit or a fatal
/// error runs no more of its own code.
///
/// **Every queued [`Delivery`] holds one owned reference** and this type has no
/// [`Drop`], for the reason `Delivery` has none: releasing a value needs the
/// context that allocated it. `Ctx`'s own `Drop` is where what no `receive()`
/// reached is given back.
#[derive(Debug, Default)]
pub struct Inbox {
    /// Arrival order, drained from the front. A [`RefCell`] rather than a
    /// `&mut` because the two owners reach it at unrelated moments, and never
    /// at the same one: the runtime is thread-per-core and neither a publish
    /// nor a `receive()` holds the borrow across a suspension point.
    queue: std::cell::RefCell<std::collections::VecDeque<Delivery>>,
    /// Whether this subscriber has already missed a value — the module doc's
    /// bound, and why the flag is sticky rather than derived from the length.
    overflowed: std::cell::Cell<bool>,
}

impl Inbox {
    /// Queues one delivery, taking over its value's reference.
    ///
    /// **Answers the delivery back when the queue is full**, having raised the
    /// overflow: this type cannot release a value — [`Delivery`] says why — so
    /// the refused one goes back to the caller, which is the publisher and does
    /// have the context that allocated it.
    #[must_use = "a refused delivery still owns a reference the caller has to release"]
    pub fn push(&self, delivery: Delivery) -> Option<Delivery> {
        let mut queue = self.queue.borrow_mut();
        if queue.len() >= INBOX_CAP {
            drop(queue);
            self.note_overflow();
            return Some(delivery);
        }
        queue.push_back(delivery);
        None
    }

    /// Whether one more delivery would be taken.
    ///
    /// Asked by the fan-out *before* it makes a copy, so that a subscriber
    /// already being closed costs the publisher no allocation at all — which is
    /// the case § 4 is written for, a fan-out to ten thousand clients where one
    /// has stopped reading.
    #[must_use]
    pub fn has_room(&self) -> bool {
        self.queue.borrow().len() < INBOX_CAP
    }

    /// Raises the overflow, counting the subscriber the first time.
    ///
    /// Idempotent, and the count is only moved on the transition: a topic
    /// publishing a hundred more values to a connection that is already being
    /// closed has lost one subscriber, not a hundred.
    pub fn note_overflow(&self) {
        if !self.overflowed.replace(true) {
            SLOW_SUBSCRIBERS_CLOSED.with(|closed| closed.set(closed.get().saturating_add(1)));
        }
    }

    /// Whether this connection missed a delivery and is therefore to be closed.
    ///
    /// Read by the subscriber's own `receive()` and by nothing else — the
    /// module doc owns why the close is the subscriber's rather than the
    /// publisher's.
    #[must_use]
    pub fn overflowed(&self) -> bool {
        self.overflowed.get()
    }

    /// The oldest queued delivery, handing its reference to the caller.
    #[must_use]
    pub fn pop(&self) -> Option<Delivery> {
        self.queue.borrow_mut().pop_front()
    }

    /// How many deliveries are waiting — § 4's bounded queue reads this, and
    /// so does a test asserting what a publish reached.
    #[must_use]
    pub fn len(&self) -> usize {
        self.queue.borrow().len()
    }

    /// Whether nothing is waiting.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.queue.borrow().is_empty()
    }
}

/// What went wrong on the socket, as the one thing a `Core` member turns into a
/// throw.
///
/// A message and nothing else. The alternative — an enum of RFC 6455's and
/// `std::io`'s failure modes — would be a taxonomy every caller in this tree
/// then has to map back onto one `RuntimeError`, since ADR 0083 § 3 gives a
/// program exactly two outcomes for a `send`: it buffered, or it threw.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeerError(Box<str>);

impl PeerError {
    /// Builds one from whatever the framing layer said.
    #[must_use]
    pub fn new(message: impl Into<Box<str>>) -> Self {
        Self(message.into())
    }

    /// What to report, already in the shape a throw's message takes.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PeerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The socket a connection isolate holds, as the operations ADR 0083 § 3
/// describes and no more.
///
/// **Blocking is the contract.** Both operations may suspend the coroutine they
/// are called on — the implementation is a synchronous codec over
/// `nvs_host::NvsStream`, which parks the task rather than the core — and § 3's
/// straight-line loop is what that buys. An implementation that answered
/// "would block" would be handing the caller a state machine to write.
pub trait PeerSocket: std::fmt::Debug {
    /// The next frame from the peer, suspending until one arrives.
    ///
    /// `Ok(None)` is the peer having closed, which is § 3's `null`.
    ///
    /// # Errors
    ///
    /// The socket failed, or the peer sent something RFC 6455 does not allow.
    /// Either way this connection is over: the isolate is what a fatal error
    /// tears down (§ 3), and a caller that saw one may not call again.
    fn receive(&mut self) -> Result<Option<PeerFrame>, PeerError>;

    /// Sends one frame, suspending until it is buffered.
    ///
    /// # Errors
    ///
    /// The socket failed, or the wait ADR 0074 expired — § 3's "throws on the
    /// send timeout rather than waiting forever".
    fn send(&mut self, frame: PeerFrame) -> Result<(), PeerError>;

    /// Sends the close handshake and stops, ignoring whatever it fails with.
    ///
    /// There is nothing a caller could do with a failure here — the connection
    /// is ending either way, and the descriptor closes with this object — so
    /// the operation has no error to report and no result to check. What the
    /// peer is *told* is [`Closing`]'s code, which is the one thing about a
    /// close a program on the other end can act on.
    fn close(&mut self, why: Closing);
}

/// Why a connection is being closed, as the code RFC 6455 puts on the wire.
///
/// One variant per reason this runtime ever closes a connection from its own
/// side, because a peer that cannot tell them apart cannot decide whether to
/// reconnect — and that decision is the whole of what a close code is for. It
/// is deliberately not a `u16`: a code is a *decision* this crate makes, and an
/// open integer would let each call site invent one.
///
/// **Two variants may share a code, and none may share an action.** The
/// question a client asks is "do I reconnect, and when", so the codes here fall
/// into three answers: 1000 and 1001 mean reconnect now, 1013 means back off
/// first, and 1008 means the reconnect will end the same way unless the client
/// changes what it does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Closing {
    /// The connection ended the way it was meant to — its loop finished, or the
    /// isolate did. RFC 6455's 1000.
    Done,
    /// ADR 0083 § 4: this subscriber's delivery queue overflowed, so it is
    /// closed rather than tolerated. RFC 6455's 1008, *policy violation*, which
    /// is the code for a peer whose behaviour the server will not carry —
    /// specifically not 1001 or 1011, which say the server is going away or
    /// broke, and either would tell a client to reconnect and do it again.
    SlowSubscriber,
    /// ADR 0083 § 7's idle timeout: nothing arrived from this peer for
    /// `nvs_server::bounds::Connection::idle`. RFC 6455's 1001, *going away* —
    /// the peer did nothing wrong and reconnecting is the correct response,
    /// which is what separates this from [`Self::SlowSubscriber`] however
    /// similar the two look from the server's side.
    Idle,
    /// ADR 0083 § 7's total lifetime: this connection has been open for
    /// `nvs_server::bounds::Connection::lifetime`, however busy it was. 1001
    /// for [`Self::Idle`]'s reason, and the reason text is what tells the two
    /// apart in a log.
    Expired,
    /// ADR 0083 § 7's per-process ceiling: this process already holds
    /// `nvs_server::bounds::Connection::max_open` connections, so this one is
    /// closed before its isolate is started. 1013, *try again later*, which is
    /// the one registered code that says the refusal is about load and not
    /// about the request — a client told 1001 here would reconnect immediately
    /// and be refused again.
    AtCapacity,
    /// ADR 0083 § 1: this connection's isolate ended in a failure — a throw
    /// that reached ADR 0020's floor, or one of the `[limits]` values § 1 gives
    /// a connection its own budget of. RFC 6455's 1011, *internal error*.
    ///
    /// **One code for both**, and that is § 1's own wording rather than a
    /// coarsening of it: what it asks for is that a connection exceeding a
    /// limit "is closed with a defined code, the same way a request that
    /// exceeds one is terminated", and a request over its budget and a request
    /// that threw past every handler are one status on the HTTP side too. The
    /// distinction that matters to a client is that the server chose the close
    /// — the alternative being the reset it reads when a process is killed for
    /// the memory it was holding, which is the failure this code exists to say
    /// did *not* happen.
    Faulted,
}

impl Closing {
    /// RFC 6455's status code for this reason.
    #[must_use]
    pub fn code(self) -> u16 {
        match self {
            Self::Done => 1000,
            Self::SlowSubscriber => 1008,
            Self::Idle | Self::Expired => 1001,
            Self::AtCapacity => 1013,
            Self::Faulted => 1011,
        }
    }

    /// The text sent beside the code, which is for a human reading a log.
    #[must_use]
    pub fn reason(self) -> &'static str {
        match self {
            Self::Done => "closing",
            Self::SlowSubscriber => "subscriber too slow to keep up with its topics",
            Self::Idle => "idle for longer than this connection is allowed to be",
            Self::Expired => "open for longer than a connection may stay open",
            Self::AtCapacity => "this server already holds as many connections as it may",
            Self::Faulted => "this connection's isolate ended in a failure",
        }
    }
}
