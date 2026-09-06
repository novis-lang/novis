//! [`Wire`] — one connection's framed stream, which is what every exchange in
//! this driver reads and writes instead of the socket.
//!
//! [`Wire::read_message`] concatenates packets until [`Status::EOM`] arrives and
//! is what a handshake uses; [`Wire::read_packet`] is what the row path uses,
//! because a result set is bounded by the table rather than by the protocol.
//! [`mod@super`]'s doc owns the choice between them.

use super::*;

/// A TDS conversation over one stream.
///
/// [`crate::mysql`]'s `Wire` and for its reasons: one place owns the framing
/// and the buffered remainder, so no caller can write half a packet, and the
/// default type parameter is the stream a real connection has while a test
/// substitutes anything that reads and writes.
///
/// That default carries a [`Tunnel`] where the other drivers' carry the socket
/// itself, and it is not decoration: § 3's TLS handshake is *tunnelled* here,
/// so the `rustls` session a live connection ends up holding was handshaken
/// over the framing rather than over the socket. [`TdsConn::connect`] is where
/// that type is built, and [`negotiate_tls`] is what answers it.
pub struct Wire<S: Read + Write = NvsTls<Tunnel<NvsTcp>>> {
    stream: S,
    inbox: Vec<u8>,
    codec: Codec,
    /// `rule:core-classes/db-transactions`'s open transaction, as the descriptor every request has to
    /// name it by, and zero where none is open.
    ///
    /// **Here rather than beside [`crate::conn::TdsConn::depth`], for the
    /// reason the packet size is here**: it is a property of this session on
    /// this stream, written by an `ENVCHANGE` the server sent and read by the
    /// next request's `ALL_HEADERS`, and every function on either side of that
    /// already holds the wire. A `Cell` on the connection would be the same
    /// value threaded through ten signatures that have it in hand — and one of
    /// them, [`TdsRows`], reads its tokens with nothing but the wire borrowed.
    /// The depth stays on the connection because § 7's *nesting* is this
    /// driver's own accounting; the descriptor is the server's.
    descriptor: u64,
}

impl<S: Read + Write> std::fmt::Debug for Wire<S> {
    /// How much is buffered, and nothing about the stream: a `rustls` session
    /// holds key material, and a buffered packet is one request's data.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Wire")
            .field("buffered", &self.inbox.len())
            .finish_non_exhaustive()
    }
}

impl<S: Read + Write> Wire<S> {
    /// A wire over a stream that is already connected, framing at
    /// [`DEFAULT_PACKET_SIZE`].
    pub fn new(stream: S) -> Wire<S> {
        Wire {
            stream,
            inbox: Vec::new(),
            codec: Codec::default(),
            descriptor: NO_TRANSACTION,
        }
    }

    /// The same conversation over an encrypted stream.
    ///
    /// [`crate::mysql`]'s `upgrade` exactly, and for a sharper version of its
    /// reason: on this protocol the TLS handshake is itself carried in
    /// [`PacketType::PreLogin`] packets, so the codec and the inbox must
    /// survive an upgrade that happens *between* two messages of one
    /// conversation. `wrap` receives the plaintext stream and answers the
    /// encrypted one, which is the only shape that works — the session is built
    /// from this wire's own socket.
    ///
    /// # Errors
    ///
    /// Whatever `wrap` reported.
    pub fn upgrade<T: Read + Write>(
        self,
        wrap: impl FnOnce(S) -> io::Result<T>,
    ) -> io::Result<Wire<T>> {
        Ok(Wire {
            stream: wrap(self.stream)?,
            inbox: self.inbox,
            codec: self.codec,
            descriptor: self.descriptor,
        })
    }

    /// Bounds every wait on this wire by `at`, or lifts the bound.
    ///
    /// [`crate::pg`]'s `set_deadline` for `rule:core-classes/db-statement-members`'s statement deadline, and
    /// the one driver where the forwarding is two hops rather than one: this
    /// session sits on a [`Tunnel`](super::prelogin::Tunnel), which sits on the
    /// socket, and `nvs_host::net::Deadline` is what carries the instant down
    /// both of them.
    pub fn set_deadline(&mut self, at: Option<std::time::Instant>)
    where
        S: nvs_host::net::Deadline,
    {
        self.stream.set_deadline(at);
    }

    /// The open transaction's descriptor, for the `ALL_HEADERS` of the request
    /// about to go out — zero where no transaction is open.
    #[must_use]
    pub fn descriptor(&self) -> u64 {
        self.descriptor
    }

    /// What an [`EnvChange::Transaction`] said the open transaction now is.
    ///
    /// Written by whoever reads the token — [`TdsRows`] on a statement's
    /// answer, [`reset_session`] on the reset's — rather than by the code that
    /// sent the `BEGIN`: the descriptor is the server's answer and not a
    /// consequence of the command, and a `BEGIN` inside a batch this driver did
    /// not write would move it just the same.
    pub fn set_descriptor(&mut self, descriptor: u64) {
        self.descriptor = descriptor;
    }

    /// The framing, for the negotiation that resizes it.
    pub fn codec(&mut self) -> &mut Codec {
        &mut self.codec
    }

    /// The framing as it now stands.
    ///
    /// A [`Codec`] is `Copy` and holds one number, so this hands back the value
    /// rather than a borrow: a caller reading the packet size while it also
    /// holds the message it read is the ordinary case, and a shared borrow of
    /// the wire would make it the awkward one.
    #[must_use]
    pub fn framing(&self) -> Codec {
        self.codec
    }

    /// The stream underneath, for the tests that assert on what was written to
    /// it. There is no non-test reader: a driver talks through the three
    /// methods below.
    #[cfg(test)]
    pub(super) fn peer(&self) -> &S {
        &self.stream
    }

    /// Frames `payload` as a whole message, writes all of it, and flushes.
    ///
    /// One function because a half-written message is the shape `rule:core-classes/db-connection-busy-state`
    /// calls poison, and `write_all` is the only spelling that cannot leave
    /// one. `status` is the extra bits of [`Codec::encode`]; the ordinary
    /// caller passes [`Status::NORMAL`].
    ///
    /// # Errors
    ///
    /// Whatever the stream reported.
    pub fn send(&mut self, kind: PacketType, status: Status, payload: &[u8]) -> io::Result<()> {
        let mut out = Vec::with_capacity(payload.len() + HEADER);
        self.codec.encode(kind, status, payload, &mut out);
        self.stream.write_all(&out)?;
        self.stream.flush()
    }

    /// Reads until the inbox holds a whole packet, and takes it.
    ///
    /// The primitive, and the one a result set is read with: bytes that arrived
    /// after this packet stay buffered for the next call. A driver reading a
    /// handshake message wants [`Wire::read_message`] instead — a TDS token is
    /// cut at whatever offset the packet size lands on, so a packet is not a
    /// unit anything above framing can parse on its own.
    ///
    /// # Errors
    ///
    /// `UnexpectedEof` when the peer closed mid-packet, `InvalidData` for a
    /// header [`decode`] refuses, and whatever the stream reported.
    pub fn read_packet(&mut self) -> io::Result<Packet> {
        loop {
            if let Some(framed) = decode(&self.inbox)? {
                let payload = self.inbox[framed.body].to_vec();
                self.inbox.drain(..framed.total);
                return Ok(Packet {
                    kind: framed.kind,
                    status: framed.status,
                    payload,
                });
            }

            if fill(&mut self.stream, &mut self.inbox)? == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "the server closed the connection part-way through a TDS packet",
                ));
            }
        }
    }

    /// Reads packets until one carries [`Status::EOM`], and answers their
    /// payloads concatenated.
    ///
    /// # Errors
    ///
    /// As [`Wire::read_packet`], plus `InvalidData` for a continuation packet
    /// whose type disagrees with the message's — which is the stream out of
    /// sync rather than a message worth parsing — and for a message past
    /// [`MAX_MESSAGE`].
    pub fn read_message(&mut self) -> io::Result<Message> {
        let first = self.read_packet()?;
        let kind = first.kind;
        let mut status = first.status;
        let mut payload = first.payload;

        while !status.contains(Status::EOM) {
            let next = self.read_packet()?;
            if next.kind != kind {
                return Err(malformed(format!(
                    "a TDS message of type 0x{:02X} continued with a packet of type 0x{:02X}",
                    kind.byte(),
                    next.kind.byte()
                )));
            }
            if payload.len() + next.payload.len() > MAX_MESSAGE {
                return Err(malformed(format!(
                    "a TDS message grew past this driver's {MAX_MESSAGE}-byte ceiling"
                )));
            }
            payload.extend_from_slice(&next.payload);
            status = next.status;
        }

        Ok(Message { kind, payload })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tds::testing::*;

    #[test]
    fn what_send_writes_is_what_the_codec_framed() {
        let mut wire = Wire::new(Script::silent());
        wire.send(PacketType::PreLogin, Status::NORMAL, b"abc")
            .expect("the script accepts every write");

        let mut expected = Vec::new();
        Codec::default().encode(PacketType::PreLogin, Status::NORMAL, b"abc", &mut expected);
        assert_eq!(wire.peer().sent, expected);
    }
}
