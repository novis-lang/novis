//! [ADR 0083](/docs/adr/0083-persistent-connections-are-isolates.md) § 1's
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
//! [`nvs_host::NvsTcp`]: its `Read` and `Write` park the coroutine instead of
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
//! by ADR 0083 § 1's own `[limits] memory` (8 MiB in that section's example, so
//! under 2% of it) and O(connections in flight) rather than O(frames served).
//! The prefix beside it is whatever `hyper` had already read — a frame at the
//! most, released the first time the codec drains it.
//!
//! # What is not here yet
//!
//! **The idle bound.** § 1 gives a connection its own `[limits] idle`, and
//! [`crate::io::ConnectionIo::into_stream`] hands the stream over with no
//! deadline on purpose — the response wait it was under is not the bound a
//! WebSocket wants. Nothing arms a new one yet, so a peer that opens a
//! connection and goes silent holds one isolate until it closes: the limit is
//! `Core\Socket`'s slice, together with § 3's send timeout, and this paragraph
//! is what says so out loud until then.

use std::io::{Read, Write};

use nvs_host::NvsTcp;
use nvs_runtime::{Closing, PeerError, PeerFrame, PeerSocket};
use tungstenite::Message;
use tungstenite::protocol::frame::CloseFrame;
use tungstenite::protocol::frame::coding::CloseCode;
use tungstenite::protocol::{Role, WebSocket};

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
    stream: NvsTcp,
}

impl Prefixed {
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

/// The framed connection, as [ADR 0083](/docs/adr/0083-persistent-connections-are-isolates.md)
/// § 1's peer.
///
/// It owns the descriptor from the `101` onwards: dropping it closes the
/// socket, which is what makes a connection isolate's teardown the connection's
/// end with no second path to keep in step.
#[derive(Debug)]
pub struct Framed(WebSocket<Prefixed>);

impl Framed {
    /// Takes the stream an upgrade handed back and frames it, server-side.
    ///
    /// `Role::Server` is not a detail: it decides that outgoing frames are
    /// unmasked and that an incoming unmasked frame is a protocol error, which
    /// is RFC 6455's own asymmetry and the half a peer cannot lie its way out
    /// of.
    #[must_use]
    pub fn new(stream: NvsTcp, already_read: Vec<u8>) -> Self {
        let prefixed = Prefixed {
            read: std::io::Cursor::new(already_read),
            stream,
        };
        Self(WebSocket::from_raw_socket(prefixed, Role::Server, None))
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
    fn receive(&mut self) -> Result<Option<PeerFrame>, PeerError> {
        loop {
            return match self.0.read() {
                Ok(Message::Text(text)) => Ok(Some(PeerFrame::Text(text.as_str().to_owned()))),
                Ok(Message::Binary(bytes)) => Ok(Some(PeerFrame::Binary(bytes.to_vec()))),
                Ok(Message::Ping(_) | Message::Pong(_)) => continue,
                // `Frame` is a raw frame the codec only produces for a caller
                // that asked for one, which this is not.
                Ok(Message::Close(_) | Message::Frame(_)) => Ok(None),
                Err(tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed) => {
                    Ok(None)
                }
                Err(error) => Err(failed(&error)),
            };
        }
    }

    /// Sends one frame and flushes it, parking until the bytes are out.
    ///
    /// # Errors
    ///
    /// The socket failed. § 3's *send timeout* is not among them yet — the
    /// module doc's § *What is not here yet* is where that gap is recorded.
    fn send(&mut self, frame: PeerFrame) -> Result<(), PeerError> {
        let message = match frame {
            PeerFrame::Text(text) => Message::Text(text.into()),
            PeerFrame::Binary(bytes) => Message::Binary(bytes.into()),
        };
        self.0.send(message).map_err(|error| failed(&error))
    }

    /// Starts RFC 6455's close handshake, ignoring what it fails with.
    ///
    /// A close that cannot be written is a connection that is already gone, and
    /// the descriptor closes with this object either way.
    ///
    /// The code and the reason are [`Closing`]'s, which is the one place this
    /// tree decides them — a peer told 1008 rather than 1000 is a client that
    /// can log why it will not simply reconnect into the same overflow.
    fn close(&mut self, why: Closing) {
        drop(self.0.close(Some(CloseFrame {
            code: CloseCode::from(why.code()),
            reason: why.reason().into(),
        })));
        drop(self.0.flush());
    }
}
