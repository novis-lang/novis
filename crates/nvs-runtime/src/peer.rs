//! The peer a connection isolate talks to —
//! [ADR 0083](/docs/adr/0083-persistent-connections-are-isolates.md) § 1's
//! socket, as the one thing that crosses from the server into the isolate.
//!
//! § 1 moves the socket into the root isolate the upgrade opened. The socket
//! itself is a `nvs_host::NvsTcp` with RFC 6455's framing over it, and neither
//! of those names exists here: this crate is underneath both, and the isolate's
//! [`Ctx`] is what has to carry the thing. So what crosses is a **trait
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
//! # What it spends
//!
//! One boxed trait object per connection isolate, and one buffer per frame in
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
/// and a `Ctx` cannot name a type that crate declares. § 4's bus is what will
/// push one; nothing does yet, which is this seam's known gap and
/// [`crate::Ctx::deliver`]'s own doc is where it is written down.
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
}

impl Inbox {
    /// Queues one delivery, taking over its value's reference.
    pub fn push(&self, delivery: Delivery) {
        self.queue.borrow_mut().push_back(delivery);
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
    /// the operation has no error to report and no result to check.
    fn close(&mut self);
}
