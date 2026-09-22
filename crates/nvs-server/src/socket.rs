//! `rule:concurrency/a-connection-is-a-root-isolate`'s
//! framing: the connection `hyper` handed back, as the peer a connection
//! isolate holds.
//!
//! Everything above this module is HTTP and everything below it is a socket.
//! What happens in between is one exchange — the `101` [`crate::serve`] writes
//! and the frames this module reads and writes over the same descriptor — and
//! the two halves are together here because the accept key and the codec are
//! the same crate's, so a server that answered one and framed the other would
//! be two implementations of one RFC.
//!
//! # `tungstenite`, synchronous, with no adapter
//!
//! The goal's standing decision, and the workspace `Cargo.toml`'s entry is
//! where it is argued. What makes a *synchronous* codec the right one here is
//! [`nvs_host::NvsConnection`]: its `Read` and `Write` park the coroutine instead of
//! blocking the core, so a `read()` that finds nothing suspends the isolate's
//! own task and resumes inside the same call. § 3's straight-line `while (var
//! $msg = $conn->receive())` is what that buys, and it is why nothing in this
//! module has a `poll`, a waker or a state machine — [`crate::io`]'s adapter
//! exists because `hyper` is `async`, and this one is not.
//!
//! # What `hyper` had already read
//!
//! An upgrade hands back the stream **and** whatever bytes were read past the
//! request head, which for a peer that wrote its first frame without waiting
//! for the `101` is a frame. [`Prefixed`] is the whole of that: a buffer that
//! is drained before the descriptor is touched, and then never again. Dropping
//! those bytes instead would lose exactly one frame, at the start of every
//! connection whose client pipelined — a fault that reproduces on nobody's
//! machine and everybody's load balancer.
//!
//! # What it spends
//!
//! **128 KiB per open connection**, which is `tungstenite`'s own input buffer
//! (`READ_BUF_LEN`) and by far the largest thing here — the codec reads whole
//! chunks off the socket rather than a frame header at a time, which is the
//! trade AGENTS.md's ordering asks for: bytes moved are a latency question and
//! footprint is the last thing spent. It is charged to the connection, bounded
//! by `rule:concurrency/a-connection-is-a-root-isolate`'s own `[limits] memory` (8 MiB in that section's example, so
//! under 2% of it) and O(connections in flight) rather than O(frames served).
//! The prefix beside it is whatever `hyper` had already read — a frame at the
//! most, released the first time the codec drains it.
//!
//! # The clock, and who winds it
//!
//! [`crate::io::ConnectionIo::into_stream`] hands the stream over with **no**
//! deadline on purpose — the response wait it was under is not the bound a
//! WebSocket wants — and this module is what arms the new one. Every wait a
//! connection takes is bounded here rather than in the isolate above it,
//! because `nvs_host::NvsStream` carries one deadline for the descriptor and
//! this is the last object that holds the descriptor. [`crate::bounds`] is
//! where the numbers and their reasoning live; what happens at each of them is
//! [`Framed`]'s own doc.
//!
//! A timeout is therefore **not** an error a program sees on `receive()`: an
//! idle or expired connection is closed with its code and answers `None`, which
//! is § 3's `null` and the condition every connection loop already ends on. A
//! `send` that times out is the one that throws, because § 3 says it does and
//! because a program that could not tell a delivered frame from an abandoned
//! one has no way to be correct.

use std::io::{Read, Write};
use std::time::Instant;

use nvs_host::NvsConnection;
use nvs_runtime::{Closing, PeerError, PeerFrame, PeerSocket};
use tungstenite::Message;
use tungstenite::protocol::WebSocketConfig;
use tungstenite::protocol::frame::CloseFrame;
use tungstenite::protocol::frame::coding::CloseCode;
use tungstenite::protocol::{Role, WebSocket};

use crate::bounds::{Connection, Slot};
use crate::serve::Draining;

/// RFC 6455's `Sec-WebSocket-Accept` for the key the opening carried.
///
/// One line of this module's crate rather than a `sha1` and a `base64` of our
/// own: the value is a hash of the peer's key and a constant GUID, and the
/// place that gets it wrong is not the arithmetic but the constant.
#[must_use]
pub fn accept_key(request_key: &[u8]) -> String {
    tungstenite::handshake::derive_accept_key(request_key)
}

/// The stream a WebSocket is framed over: what `hyper` read past the request
/// head first, and then the socket.
///
/// A type rather than a check at every read, because the prefix is empty for
/// every connection but the one that pipelined and a branch that is almost
/// never taken is exactly the branch that is never tested. Here it is the same
/// code path either way.
#[derive(Debug)]
pub struct Prefixed {
    /// What `hyper` had already read, and how much of it has been handed on.
    /// Emptied once and never refilled.
    read: std::io::Cursor<Vec<u8>>,
    /// The connection itself, from the first byte the prefix does not answer.
    stream: NvsConnection,
}

impl Prefixed {
    /// Bounds every wait on the descriptor by `at`.
    ///
    /// The prefix is not bounded and does not need to be: it is bytes already
    /// in memory, so a read that the cursor answers takes no wait at all.
    fn set_deadline(&mut self, at: Instant) {
        self.stream.set_deadline(Some(at));
    }

    /// [`NvsConnection::cut_at_drain`] on the descriptor. The prefix is bytes
    /// already in memory, and no wait is ever taken on it.
    fn cut_at_drain(&mut self, drain: &nvs_runtime::Drain) -> Option<nvs_runtime::DrainWake> {
        self.stream.cut_at_drain(drain)
    }

    /// Whether the prefix still has bytes nobody has read.
    fn buffered(&self) -> bool {
        // Widening: a cursor's position is a `u64` and the buffer is a `Vec`,
        // so the comparison is between the two spellings of one length.
        self.read.position() < self.read.get_ref().len() as u64
    }
}

impl Read for Prefixed {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.buffered() {
            return self.read.read(buf);
        }
        self.stream.read(buf)
    }
}

impl Write for Prefixed {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.stream.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.stream.flush()
    }
}

/// The framed connection, as `rule:concurrency/a-connection-is-a-root-isolate`
/// 's peer.
///
/// It owns the descriptor from the `101` onwards: dropping it closes the
/// socket, which is what makes a connection isolate's teardown the connection's
/// end with no second path to keep in step. It owns § 7's clock for the same
/// reason — the descriptor's deadline is one field of one stream, and this is
/// the last object that holds it.
#[derive(Debug)]
pub struct Framed {
    /// The codec, already carrying § 7's two size bounds as its own config.
    socket: WebSocket<Prefixed>,
    /// § 7's numbers for this connection, copied at the `101`.
    bounds: Connection,
    /// When [`Connection::lifetime`] runs out, computed once so that every wait
    /// is capped by the same instant rather than by a duration re-measured from
    /// whenever it was asked.
    expires_at: Instant,
    /// This connection's place under [`Connection::max_open`], or `None` for a
    /// connection the process had no room for. Held here so the count falls
    /// exactly when the descriptor does.
    slot: Option<Slot>,
    /// The drain of the server that accepted this connection — § 7's third
    /// bullet, read by [`PeerSocket::receive`] and by nothing else here.
    ///
    /// A handle on the *server's* bit rather than the process's, because a
    /// process may run more than one accept loop and only the one that took
    /// this connection is entitled to end it
    /// ([`nvs_runtime::drain`] owns that distinction).
    draining: Draining,
    /// The wake that ends a `receive` parked on the peer the moment the drain
    /// begins, registered by the first `receive` on the task the program runs
    /// on and held for the connection's life ([`PeerSocket::receive`]).
    cut: Option<nvs_runtime::DrainWake>,
}

impl Framed {
    /// Takes the stream an upgrade handed back and frames it, server-side,
    /// inside `bounds`.
    ///
    /// `Role::Server` is not a detail: it decides that outgoing frames are
    /// unmasked and that an incoming unmasked frame is a protocol error, which
    /// is RFC 6455's own asymmetry and the half a peer cannot lie its way out
    /// of.
    ///
    /// **The size bounds are handed to the codec and the time bounds are not**,
    /// because `tungstenite` is what reassembles a message and this module is
    /// what owns the descriptor's clock. Passing a config at all is the point:
    /// the crate's own defaults are 16 MiB and 64 MiB, and
    /// [`Connection::default`] is where it is argued that a connection given
    /// 8 MiB may not be handed either.
    ///
    /// A connection this process has no room for is framed anyway and reports
    /// [`Self::admitted`] as `false`. Refusing before framing would leave the
    /// caller a raw descriptor and no way to say why it is closing, and § 7
    /// asks for a defined code rather than a reset.
    #[must_use]
    pub fn new(
        stream: NvsConnection,
        already_read: Vec<u8>,
        bounds: Connection,
        draining: Draining,
    ) -> Self {
        let prefixed = Prefixed {
            read: std::io::Cursor::new(already_read),
            stream,
        };
        let config = WebSocketConfig::default()
            .max_frame_size(Some(bounds.frame))
            .max_message_size(Some(bounds.message));
        Self {
            socket: WebSocket::from_raw_socket(prefixed, Role::Server, Some(config)),
            bounds,
            expires_at: Instant::now() + bounds.lifetime,
            slot: Slot::take(bounds.max_open),
            draining,
            cut: None,
        }
    }

    /// Whether this process had a place for this connection under
    /// [`Connection::max_open`].
    ///
    /// A caller that reads `false` owes the peer a
    /// [`Closing::AtCapacity`] close and must start no isolate: the whole
    /// saving of the ceiling is the isolate that is not allocated.
    #[must_use]
    pub fn admitted(&self) -> bool {
        self.slot.is_some()
    }

    /// Tells the peer [`Closing::AtCapacity`] and drops the descriptor.
    ///
    /// An inherent method rather than the caller reaching for
    /// [`nvs_runtime::PeerSocket::close`], because this is the one close taken
    /// by a caller that never hands the socket to an isolate — and importing
    /// the whole seam trait into [`crate::serve`] to spell one refusal would
    /// put `receive` and `send` in scope in the module that must never call
    /// either.
    pub fn refuse(mut self) {
        self.close(Closing::AtCapacity);
    }

    /// Bounds the next wait by `window`, or by the lifetime where that is
    /// sooner.
    ///
    /// The lifetime cap is what makes [`Connection::lifetime`] a bound at all:
    /// a connection that speaks every minute would otherwise re-arm the idle
    /// window forever and never reach its own expiry. The drain is not an
    /// instant here: it ends a parked `receive` through the cut
    /// [`PeerSocket::receive`] registers, and a `send` is never cut — § 3
    /// makes a send timeout the one failure a program has to see, and telling
    /// it the server is going away is `receive`'s job rather than a
    /// half-written frame's.
    fn arm(&mut self, window: std::time::Duration) {
        let at = (Instant::now() + window).min(self.expires_at);
        self.socket.get_mut().set_deadline(at);
    }

    /// Which of § 7's two clocks a `TimedOut` was, or `None` for an error that
    /// is not one.
    ///
    /// Read off the *deadlines* rather than off which window was armed, because
    /// [`Self::arm`] hands the stream one instant and the stream reports one
    /// kind — so the question "was that the lifetime" is answered by asking the
    /// lifetime, and everything else is the idle window by construction.
    fn expiry(&self, error: &tungstenite::Error) -> Option<Closing> {
        let tungstenite::Error::Io(io) = error else {
            return None;
        };
        if io.kind() != std::io::ErrorKind::TimedOut {
            return None;
        }
        Some(if Instant::now() >= self.expires_at {
            Closing::Expired
        } else {
            Closing::Idle
        })
    }
}

/// Whatever the codec or the socket said, in the one shape the seam carries.
fn failed(error: &tungstenite::Error) -> PeerError {
    PeerError::new(error.to_string())
}

impl PeerSocket for Framed {
    /// The next frame the peer sent, parking the isolate's task until one
    /// arrives.
    ///
    /// **A ping is answered and not reported.** `tungstenite` queues the pong
    /// itself and hands the ping up anyway; a program has no use for either, so
    /// this reads again rather than making every loop in every application
    /// filter control frames it did not ask about. A close — orderly, or the
    /// socket ending under us — is `None`, which is § 3's `null`.
    ///
    /// **A ping re-arms the idle window**, which is the arming being inside the
    /// loop rather than above it. That is what makes § 7's idle bound usable at
    /// all: a client with nothing to say keeps its connection by doing what RFC
    /// 6455 already tells it to do, and a client that has genuinely gone is the
    /// one that stops.
    ///
    /// § 7's two timeouts end the connection here rather than reporting one:
    /// the peer is told [`Closing::Idle`] or [`Closing::Expired`] and the
    /// member answers `None`, so a program's loop ends exactly as it does on an
    /// ordinary disconnect. A connection closed on the clock is not a fault of
    /// the program's, and there is nothing for it to catch.
    ///
    /// # § 7's shutdown, and why it is a close taken here
    ///
    /// **A drained server's connections are closed by their own loops, and not
    /// by the accept loop that spawned them.** § 7's third bullet asks a
    /// shutdown and a `nvs ctl reload` to leave the peer "a clean close rather
    /// than a reset", and both ways of reaching one from outside are refused.
    /// Cancelling the connection isolate tears its task down at its next
    /// safepoint (`nvs_host::scheduler::cancel_task`), which is the path a
    /// panicking isolate already takes and which `nvs_host::isolate`'s close
    /// branch records as giving the peer exactly the reset this bullet exists
    /// to replace. And there is no second handle to write a frame through: the
    /// socket moved into the isolate at the `101`, and on one core the only
    /// task that may touch it is the one holding it. So the close is taken
    /// here, by the program's own `receive`, and it is
    /// [`Closing::ShuttingDown`].
    ///
    /// **It is taken the moment the drain is seen, not after a period**
    /// (`rule:concurrency/a-drain-closes-a-connection-cleanly`): a program in
    /// `receive` is waiting for its peer and has nothing to finish, so a
    /// period would hold the stop for a connection doing nothing. What the
    /// drain does not cut is a program between one `receive` and the next —
    /// its `send` completes on its own clock, and this call is where it then
    /// learns the server is going away — so a program mid-step finishes the
    /// step and a program mid-wait ends at once.
    ///
    /// **A `receive` already parked when the drain begins is woken by it.**
    /// The first call registers a cut with the server's drain on the task the
    /// program runs on (`nvs_host::NvsStream::cut_at_drain`): a parked read
    /// ends on its deadline, on readiness or on a cancellation and on nothing
    /// else, and a plain wake would only send it back round its retry loop to
    /// park again. The cut turns that wake into a `WouldBlock` out of the
    /// read, which the codec returns untouched and the loop below reads the
    /// bit on. A drain that began before the first call needs no wake: the
    /// bit is read before the wait is armed.
    fn receive(&mut self) -> Result<Option<PeerFrame>, PeerError> {
        if self.cut.is_none() {
            self.cut = self.socket.get_mut().cut_at_drain(self.draining.bit());
        }
        loop {
            if self.draining.is_draining() {
                self.close(Closing::ShuttingDown);
                return Ok(None);
            }
            self.arm(self.bounds.idle);
            return match self.socket.read() {
                Ok(Message::Text(text)) => Ok(Some(PeerFrame::Text(text.as_str().to_owned()))),
                Ok(Message::Binary(bytes)) => Ok(Some(PeerFrame::Binary(bytes.to_vec()))),
                Ok(Message::Ping(_) | Message::Pong(_)) => continue,
                // `Frame` is a raw frame the codec only produces for a caller
                // that asked for one, which this is not.
                Ok(Message::Close(_) | Message::Frame(_)) => Ok(None),
                Err(tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed) => {
                    Ok(None)
                }
                // The cut ended a parked read: the drain is read at the top of
                // the loop, which is the one place it is acted on.
                Err(tungstenite::Error::Io(io)) if io.kind() == std::io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(error) => match self.expiry(&error) {
                    Some(why) => {
                        self.close(why);
                        Ok(None)
                    }
                    None => Err(failed(&error)),
                },
            };
        }
    }

    /// Sends one frame and flushes it, parking until the bytes are out.
    ///
    /// § 3's *send timeout* is [`Connection::send`], armed here and reported as
    /// an error rather than as a close: this is the one operation whose failure
    /// a program must see, because a peer that stopped reading and a frame that
    /// went out are the same call otherwise.
    ///
    /// # Errors
    ///
    /// The socket failed, or the send timeout expired — including the case
    /// where it was [`Connection::lifetime`] that expired first, since
    /// [`Framed::arm`] caps every wait by it and a connection past its lifetime
    /// may not keep writing.
    fn send(&mut self, frame: PeerFrame) -> Result<(), PeerError> {
        let message = match frame {
            PeerFrame::Text(text) => Message::Text(text.into()),
            PeerFrame::Binary(bytes) => Message::Binary(bytes.into()),
        };
        self.arm(self.bounds.send);
        self.socket.send(message).map_err(|error| failed(&error))
    }

    /// Starts RFC 6455's close handshake, ignoring what it fails with.
    ///
    /// A close that cannot be written is a connection that is already gone, and
    /// the descriptor closes with this object either way.
    ///
    /// The code and the reason are [`Closing`]'s, which is the one place this
    /// tree decides them — a peer told 1008 rather than 1000 is a client that
    /// can log why it will not simply reconnect into the same overflow.
    ///
    /// **The lifetime does not bound this write**, which is why the deadline is
    /// set here rather than through [`Framed::arm`]: the connection being over
    /// is the commonest reason to be closing one, and a close capped by an
    /// instant already in the past would send nothing at all — turning every
    /// [`Closing::Expired`] into the reset § 7 asks this to replace. The send
    /// timeout still applies, so a peer that has stopped reading costs one
    /// window and not a stuck coroutine.
    fn close(&mut self, why: Closing) {
        self.socket
            .get_mut()
            .set_deadline(Instant::now() + self.bounds.send);
        drop(self.socket.close(Some(CloseFrame {
            code: CloseCode::from(why.code()),
            reason: why.reason().into(),
        })));
        drop(self.socket.flush());
    }
}
