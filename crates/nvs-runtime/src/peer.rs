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
