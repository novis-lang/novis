//! PRELOGIN and the in-band TLS upgrade: what encryption the server says it
//! will accept, and the [`Tunnel`] that carries the handshake over the framing
//! rather than over the socket.
//!
//! The tunnel is the shape TDS forces and no other driver here has. TLS is
//! negotiated *inside* PRELOGIN packets, so the `rustls` session a live
//! connection ends up holding was handshaken over a stream that adds and strips
//! eight-byte headers, and [`TunnelEnd`] is what stops it doing that once the
//! handshake is finished. A server that would leave the session in plaintext is
//! refused, per [ADR 0067 § 3](/docs/adr/0067-core-db.md).

use super::*;

/// The protocol version this driver speaks, as LOGIN7's `TDSVersion` field
/// spells it: TDS 7.4.
///
/// PRELOGIN's `VERSION` option is documented as the *client's* version and no
/// server acts on it, so sending the protocol version rather than a build
/// number of Novis's own says the one thing about this client that an operator
/// reading `sys.dm_exec_connections` could act on.
pub const TDS_VERSION: u32 = 0x7400_0004;

/// The `VERSION` option: `UL_VERSION` and `US_SUBBUILD`, six bytes.
pub(super) const PL_VERSION: u8 = 0x00;
/// The `ENCRYPTION` option: one byte, an [`Encryption`] on either side.
pub(super) const PL_ENCRYPTION: u8 = 0x01;
/// Ends the option table. It is a token with no offset and no length, so it is
/// one byte where every entry before it is five.
pub(super) const PL_TERMINATOR: u8 = 0xFF;
/// One option entry: the token, then a big-endian offset and length.
pub(super) const PL_ENTRY: u16 = 5;
/// How much data the `VERSION` option points at.
pub(super) const PL_VERSION_LEN: u16 = 6;
/// How much data the `ENCRYPTION` option points at.
pub(super) const PL_ENCRYPTION_LEN: u16 = 1;
/// Where the first blob starts: two entries and the terminator.
pub(super) const PL_VERSION_AT: u16 = PL_ENTRY * 2 + 1;
/// Where the second blob starts.
pub(super) const PL_ENCRYPTION_AT: u16 = PL_VERSION_AT + PL_VERSION_LEN;

/// What a PRELOGIN's `ENCRYPTION` option says, on either side of the exchange.
///
/// One enum for both halves because MS-TDS gives them one vocabulary: a server
/// answers with the byte a client would have sent, plus [`Encryption::Required`]
/// which only it may send. Novis writes [`Encryption::On`] and nothing else —
/// [ADR 0067 § 3](/docs/adr/0067-core-db.md) has no spelling for an
/// unencrypted database connection — so the client half is a constant and only
/// the answer is ever parsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encryption {
    /// Encrypt the login and speak plaintext afterwards. **Refused**: it is
    /// exactly the shape § 3 names when it calls `sslmode=prefer` a hole, and
    /// on this protocol the credential is the *only* thing it would protect.
    Off,
    /// Encrypt the whole session. What this driver asks for, and one of the two
    /// answers it accepts.
    On,
    /// This side cannot encrypt at all. **Refused**, and a server saying it is a
    /// server Novis has no way to reach.
    NotSupported,
    /// The server requires encryption. Only a server sends it, and it is the
    /// other answer this driver accepts.
    Required,
}

impl Encryption {
    /// The byte this writes into a PRELOGIN option.
    ///
    /// A `match` rather than a `#[repr(u8)]` cast, for [`PacketType::byte`]'s
    /// reason: the numbers appear once each and in one direction only.
    #[must_use]
    pub const fn byte(self) -> u8 {
        match self {
            Encryption::Off => 0x00,
            Encryption::On => 0x01,
            Encryption::NotSupported => 0x02,
            Encryption::Required => 0x03,
        }
    }

    /// The setting that byte names, or `None` for one MS-TDS does not define.
    #[must_use]
    pub const fn from_byte(byte: u8) -> Option<Encryption> {
        match byte {
            0x00 => Some(Encryption::Off),
            0x01 => Some(Encryption::On),
            0x02 => Some(Encryption::NotSupported),
            0x03 => Some(Encryption::Required),
            _ => None,
        }
    }

    /// Whether this answer means every byte after the handshake is encrypted.
    #[must_use]
    pub const fn covers_the_session(self) -> bool {
        matches!(self, Encryption::On | Encryption::Required)
    }
}

/// The PRELOGIN message this driver sends, as a payload for [`Wire::send`].
///
/// Two options and the terminator: the version, which a server records and
/// never acts on, and the encryption request, which is the whole point of the
/// message. MS-TDS lets a client offer `INSTOPT`, `THREADID`, `MARS` and the
/// federated-authentication options as well; each of those either names a
/// feature this driver does not have or asks a question whose answer is already
/// in the `[db.<name>]` block, and an option a server has to parse in order to
/// ignore is not free.
///
/// **The offsets and lengths are big-endian and are counted from the start of
/// this payload**, not of the packet — the module doc's second trap. The
/// version blob they point at is little-endian, in the same eighteen bytes.
pub(super) fn prelogin_request() -> Vec<u8> {
    let mut out = Vec::with_capacity(usize::from(PL_ENCRYPTION_AT + PL_ENCRYPTION_LEN));

    out.push(PL_VERSION);
    out.extend_from_slice(&PL_VERSION_AT.to_be_bytes());
    out.extend_from_slice(&PL_VERSION_LEN.to_be_bytes());

    out.push(PL_ENCRYPTION);
    out.extend_from_slice(&PL_ENCRYPTION_AT.to_be_bytes());
    out.extend_from_slice(&PL_ENCRYPTION_LEN.to_be_bytes());

    out.push(PL_TERMINATOR);

    out.extend_from_slice(&TDS_VERSION.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());

    out.push(Encryption::On.byte());
    out
}

/// The `ENCRYPTION` byte of a PRELOGIN response.
///
/// Walks the option table rather than reading a fixed position: a server orders
/// its options as it likes and sends the ones it has, so the only thing this
/// driver may assume about the answer is the table's own shape.
///
/// # Errors
///
/// `InvalidData` for a table that runs off the end of the message, for a data
/// range pointing outside it, for a response naming no `ENCRYPTION` option at
/// all — which is a server that has said nothing about whether this connection
/// would be encrypted — and for a byte that is none of MS-TDS's four.
pub(super) fn encryption_of(payload: &[u8]) -> io::Result<Encryption> {
    let entry = usize::from(PL_ENTRY);
    let mut at = 0;

    while payload.get(at).copied() != Some(PL_TERMINATOR) {
        let Some(option) = payload.get(at..at + entry) else {
            return Err(malformed(
                "a PRELOGIN response's option table ran off the end of the message before its \
                 terminator"
                    .to_string(),
            ));
        };
        let offset = usize::from(u16::from_be_bytes([option[1], option[2]]));
        let length = usize::from(u16::from_be_bytes([option[3], option[4]]));

        if option[0] == PL_ENCRYPTION {
            let Some(&byte) = payload.get(offset..offset + length).and_then(<[u8]>::first) else {
                return Err(malformed(format!(
                    "a PRELOGIN response's ENCRYPTION option is {length} byte(s) at offset \
                     {offset}, which does not resolve to a byte of the {}-byte message",
                    payload.len()
                )));
            };
            return Encryption::from_byte(byte).ok_or_else(|| {
                malformed(format!(
                    "a PRELOGIN response's ENCRYPTION option is 0x{byte:02X}, which is none of \
                     MS-TDS's four settings"
                ))
            });
        }

        at += entry;
    }

    Err(malformed(
        "a PRELOGIN response named no ENCRYPTION option, so nothing in it says whether this \
         connection would be encrypted"
            .to_string(),
    ))
}

/// Sends PRELOGIN and reads the answer, refusing a server that will not encrypt
/// the whole session.
///
/// This exchange is the only thing on a SQL Server connection that ever happens
/// in the clear, and it carries a version and a request — no credential, no
/// database name, nothing an eavesdropper did not already learn from the
/// address. Everything after it rides [`negotiate_tls`]'s session.
///
/// A server answers PRELOGIN with a packet of type **`TabularResult`**, not
/// with one of its own type, which is a wrinkle of the protocol rather than of
/// this driver; anything else is the stream out of sync and is refused here
/// rather than parsed as an option table.
///
/// # Errors
///
/// [`Wire::read_packet`]'s, plus `InvalidData` for an answer of the wrong
/// packet type, for a malformed option table ([`encryption_of`]), and for a
/// server offering anything but [`Encryption::On`] or [`Encryption::Required`].
pub fn prelogin<S: Read + Write>(wire: &mut Wire<S>) -> io::Result<()> {
    wire.send(PacketType::PreLogin, Status::NORMAL, &prelogin_request())?;

    let answer = wire.read_message()?;
    if answer.kind != PacketType::TabularResult {
        return Err(malformed(format!(
            "a PRELOGIN answer arrived as packet type 0x{:02X}, not the 0x{:02X} MS-TDS answers \
             one with",
            answer.kind.byte(),
            PacketType::TabularResult.byte()
        )));
    }

    let offered = encryption_of(&answer.payload)?;
    if !offered.covers_the_session() {
        return Err(malformed(format!(
            "this server answered PRELOGIN with encryption 0x{:02X}, which leaves everything after \
             the login in plaintext; `rule:core-classes/db-capabilities` has no spelling for that",
            offered.byte()
        )));
    }
    Ok(())
}

/// The TLS handshake's records, carried as [`PacketType::PreLogin`] payloads for
/// as long as the handshake lasts.
///
/// [ADR 0067 § 3](/docs/adr/0067-core-db.md)'s TLS arrives one layer
/// lower here than on any other backend. MySQL and PostgreSQL each write a short
/// plaintext request and then hand the raw socket to `rustls`; SQL Server
/// tunnels the handshake itself, so every `ClientHello`, certificate and
/// `Finished` is the payload of a TDS packet, and only once the handshake is
/// complete do records go on the socket as they are. The bytes `rustls`
/// produces are therefore not the bytes the socket carries, and this adapter is
/// what reconciles them — `nvs_host::tls`'s own docs name it, and it is why
/// [`NvsTls`] is generic over its transport at all.
///
/// **It outlives the tunnel it is.** After [`TunnelEnd::handshake_done`] every
/// call passes straight through, at the cost of one `Cell` read on a path that
/// has just encrypted or decrypted a record. Taking the adapter back out would
/// mean handing `rustls` a different stream than the one it handshook over,
/// which `StreamOwned` has no spelling for.
pub struct Tunnel<S: Read + Write> {
    stream: S,
    /// Whether the framing is still on. Shared with the [`TunnelEnd`] the caller
    /// kept, because `rustls` owns the tunnel by the time the handshake returns
    /// and hands nothing back. An `Rc` rather than an atomic: a connection
    /// belongs to one core, which is this crate's standing assumption and
    /// `nvs-host`'s `!Send` scheduler's guarantee.
    framing: Rc<Cell<bool>>,
    /// Handshake bytes `rustls` has written and not yet flushed. One flush is
    /// one TDS message, which is the right boundary because `rustls` flushes
    /// after each write flight and before it reads.
    outbox: Vec<u8>,
    /// What has arrived off the socket and is not yet a whole packet.
    inbox: Vec<u8>,
    /// Packet payloads not yet handed to `rustls`.
    plain: Vec<u8>,
}

impl<S: Read + Write> std::fmt::Debug for Tunnel<S> {
    /// Whether the framing is on and how much is buffered. Not the stream and
    /// not the buffers: those hold a handshake's own material.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tunnel")
            .field("framing", &self.framing.get())
            .field("buffered", &(self.inbox.len() + self.plain.len()))
            .finish_non_exhaustive()
    }
}

impl<S: Read + Write> Tunnel<S> {
    /// A tunnel over the plaintext socket, framing from its first byte.
    pub fn wrapping(stream: S) -> Tunnel<S> {
        Tunnel {
            stream,
            framing: Rc::new(Cell::new(true)),
            outbox: Vec::new(),
            inbox: Vec::new(),
            plain: Vec::new(),
        }
    }

    /// The handle that ends the framing, taken before `rustls` takes the tunnel.
    #[must_use]
    pub fn end(&self) -> TunnelEnd {
        TunnelEnd(Rc::clone(&self.framing))
    }
}

/// The handle that ends a [`Tunnel`]'s framing.
///
/// Taken before `rustls` takes the tunnel and consumed when the handshake
/// returns. `self` rather than `&self` so that a tunnel cannot be reopened, and
/// so that a handle dropped without being called is visibly a handshake that
/// never finished rather than a connection that quietly kept framing.
#[derive(Debug)]
pub struct TunnelEnd(Rc<Cell<bool>>);

impl TunnelEnd {
    /// Stops the framing: from here the socket carries TLS records as they are.
    pub fn handshake_done(self) {
        self.0.set(false);
    }
}

/// Moves as much of `held` as fits into `buf`, and drops what it moved.
pub(super) fn take_front(held: &mut Vec<u8>, buf: &mut [u8]) -> usize {
    let take = held.len().min(buf.len());
    buf[..take].copy_from_slice(&held[..take]);
    held.drain(..take);
    take
}

/// The socket's clock, forwarded past the framing.
///
/// This adapter buffers but never waits: every wait a tunnelled handshake takes
/// is the stream underneath issuing a syscall, so the deadline belongs there and
/// this is the one hop that lets [`crate::tds`]'s session name it. It is what
/// makes `NvsTls<Tunnel<NvsTcp>>` bound its exchanges like the other four
/// drivers' `NvsTls<NvsTcp>` does, rather than being the one connection `rule:core-classes/db-statement-members`'s statement deadline could not reach.
impl<S: Read + Write + nvs_host::net::Deadline> nvs_host::net::Deadline for Tunnel<S> {
    fn set_deadline(&mut self, at: Option<std::time::Instant>) {
        self.stream.set_deadline(at);
    }

    fn deadline(&self) -> Option<std::time::Instant> {
        self.stream.deadline()
    }
}

impl<S: Read + Write> Read for Tunnel<S> {
    /// Packet payloads while the tunnel frames, and the socket itself
    /// afterwards.
    ///
    /// Whatever was decoded before the tunnel ended is drained first either way,
    /// and then whatever was buffered but never framed: `Ok(0)` here is end of
    /// file to `rustls`, so a byte answered out of order is a handshake that
    /// fails for a reason no message would name. A packet with an empty body is
    /// not end of file either — it is a legal message — so an empty payload
    /// loops rather than answering zero.
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if !self.plain.is_empty() {
            return Ok(take_front(&mut self.plain, buf));
        }
        if !self.framing.get() {
            if !self.inbox.is_empty() {
                return Ok(take_front(&mut self.inbox, buf));
            }
            return self.stream.read(buf);
        }

        loop {
            if let Some(framed) = decode(&self.inbox)? {
                self.plain.extend_from_slice(&self.inbox[framed.body]);
                self.inbox.drain(..framed.total);
                if self.plain.is_empty() {
                    continue;
                }
                return Ok(take_front(&mut self.plain, buf));
            }

            if fill(&mut self.stream, &mut self.inbox)? == 0 {
                if self.inbox.is_empty() {
                    return Ok(0);
                }
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "the server closed the connection part-way through a tunnelled TDS packet",
                ));
            }
        }
    }
}

impl<S: Read + Write> Write for Tunnel<S> {
    /// Buffers, while the tunnel frames.
    ///
    /// A TDS header states its own packet's length, so nothing can go out until
    /// the flight is complete — and `rustls` marks a flight complete with the
    /// flush below, every time, before it reads anything back.
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if !self.framing.get() {
            return self.stream.write(buf);
        }
        self.outbox.extend_from_slice(buf);
        Ok(buf.len())
    }

    /// Frames the flight as one PRELOGIN message and writes all of it.
    fn flush(&mut self) -> io::Result<()> {
        if self.framing.get() && !self.outbox.is_empty() {
            let mut out = Vec::with_capacity(self.outbox.len() + HEADER);
            Codec::default().encode(PacketType::PreLogin, Status::NORMAL, &self.outbox, &mut out);
            self.outbox.clear();
            self.stream.write_all(&out)?;
        }
        self.stream.flush()
    }
}

/// PRELOGIN, then the TLS handshake tunnelled inside it, then an ordinary
/// encrypted wire.
///
/// The whole of `rule:core-classes/db-capabilities` on this backend, and it takes the wire by value
/// for [`Wire::upgrade`]'s reason: the codec and anything still buffered survive
/// the upgrade, because here the encryption starts *between two messages of one
/// conversation* rather than between two conversations.
///
/// A free function generic in the stream rather than a method, so that a test
/// can reach it without a socket and a certificate — the shape `crate::pg`'s
/// sequencing already takes, for the same reason.
///
/// `host` is the name the certificate is checked against and `bundle` the
/// anchors to check it against, both straight off [`TdsTarget`].
///
/// # Errors
///
/// [`prelogin`]'s, plus whatever the handshake reported: `InvalidData` for a
/// certificate that does not verify against those anchors, `TimedOut` for a
/// handshake past the stream's deadline.
pub fn negotiate_tls<S: Read + Write>(
    mut plain: Wire<S>,
    host: &str,
    bundle: Option<&Path>,
) -> io::Result<Wire<NvsTls<Tunnel<S>>>> {
    prelogin(&mut plain)?;

    plain.upgrade(|stream| {
        let tunnel = Tunnel::wrapping(stream);
        let end = tunnel.end();
        let session = match bundle {
            Some(bundle) => NvsTls::over_bundle(tunnel, host, bundle),
            None => NvsTls::over(tunnel, host),
        }?;
        // Only now: every record above was a payload, and every record below is
        // the socket's own.
        end.handshake_done();
        Ok(session)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tds::testing::*;

    #[test]
    fn an_encryption_settings_byte_round_trips() {
        for setting in [
            Encryption::Off,
            Encryption::On,
            Encryption::NotSupported,
            Encryption::Required,
        ] {
            assert_eq!(Encryption::from_byte(setting.byte()), Some(setting));
        }
        assert_eq!(Encryption::from_byte(0x04), None);
    }

    #[test]
    fn a_prelogin_asks_for_encryption_and_names_the_version() {
        let payload = prelogin_request();

        assert_eq!(payload.len(), 18, "two entries, a terminator and two blobs");
        assert_eq!(payload[0], PL_VERSION);
        assert_eq!(u16::from_be_bytes([payload[1], payload[2]]), 11);
        assert_eq!(u16::from_be_bytes([payload[3], payload[4]]), 6);
        assert_eq!(payload[5], PL_ENCRYPTION);
        assert_eq!(u16::from_be_bytes([payload[6], payload[7]]), 17);
        assert_eq!(u16::from_be_bytes([payload[8], payload[9]]), 1);
        assert_eq!(payload[10], PL_TERMINATOR);

        // The version blob is little-endian inside a table that is not: both
        // conventions, in eighteen bytes.
        assert_eq!(payload[11..15], TDS_VERSION.to_le_bytes());
        assert_eq!(u16::from_le_bytes([payload[15], payload[16]]), 0);
        assert_eq!(payload[17], Encryption::On.byte());

        // Each offset really does point inside the message, which is what a
        // little-endian one would not: 11 written the other way round is 2816.
        for at in [1, 6] {
            let offset = usize::from(u16::from_be_bytes([payload[at], payload[at + 1]]));
            assert!(offset < payload.len(), "option at {at} points off the end");
        }
    }

    #[test]
    fn a_prelogin_goes_out_as_one_prelogin_message_and_the_answer_is_read_back() {
        let mut wire = Wire::new(Script::answering(
            prelogin_answer(Encryption::Required.byte()),
            READ_CHUNK,
        ));

        prelogin(&mut wire).expect("a server that requires encryption is one we can use");

        assert_eq!(
            wire.peer().sent,
            packet(PacketType::PreLogin, Status::EOM, 1, &prelogin_request())
        );
    }

    #[test]
    fn a_server_that_would_leave_the_session_in_plaintext_is_refused() {
        for setting in [Encryption::On, Encryption::Required] {
            let mut wire = Wire::new(Script::answering(
                prelogin_answer(setting.byte()),
                READ_CHUNK,
            ));
            prelogin(&mut wire).unwrap_or_else(|e| panic!("{setting:?} is usable, not {e}"));
        }

        // Both refusals are the same refusal: § 3 has one setting and it is not
        // negotiable downwards, so "login only" and "not at all" fail alike.
        for setting in [Encryption::Off, Encryption::NotSupported] {
            let mut wire = Wire::new(Script::answering(
                prelogin_answer(setting.byte()),
                READ_CHUNK,
            ));
            let refused = prelogin(&mut wire).expect_err("a plaintext session is not reachable");
            assert_eq!(refused.kind(), io::ErrorKind::InvalidData);
            assert!(
                refused.to_string().contains("plaintext"),
                "{setting:?} was refused as {refused}"
            );
        }
    }

    #[test]
    fn a_prelogin_answer_this_driver_cannot_read_is_refused_rather_than_assumed() {
        // An answer of the wrong packet type: the stream is out of sync, and
        // the option table would be read out of whatever arrived instead.
        let mut wire = Wire::new(Script::answering(
            packet(PacketType::PreLogin, Status::EOM, 1, &[PL_TERMINATOR]),
            READ_CHUNK,
        ));
        let refused = prelogin(&mut wire).expect_err("0x12 is not what a server answers with");
        assert!(
            refused.to_string().contains("packet type 0x12"),
            "{refused}"
        );

        // A table with no ENCRYPTION option in it at all.
        let missing = encryption_of(&[PL_TERMINATOR]).expect_err("nothing said anything");
        assert!(
            missing.to_string().contains("no ENCRYPTION option"),
            "{missing}"
        );

        // A table that never reaches its terminator.
        let truncated = encryption_of(&[PL_VERSION, 0, 11]).expect_err("half an entry");
        assert!(
            truncated.to_string().contains("ran off the end"),
            "{truncated}"
        );

        // An entry whose data is outside the message. Offset 200 is inside no
        // PRELOGIN anyone sends, and reading it would be reading the heap.
        let outside = encryption_of(&[PL_ENCRYPTION, 0, 200, 0, 1, PL_TERMINATOR])
            .expect_err("the option points past the message");
        assert!(outside.to_string().contains("offset 200"), "{outside}");

        // A byte MS-TDS does not define. Not treated as "some kind of on".
        let unknown = encryption_of(&[PL_ENCRYPTION, 0, 6, 0, 1, PL_TERMINATOR, 0x04])
            .expect_err("0x04 is not one of the four");
        assert!(unknown.to_string().contains("0x04"), "{unknown}");
    }

    #[test]
    fn a_handshake_flight_goes_out_as_one_prelogin_message_and_a_closed_tunnel_adds_no_header() {
        let mut tunnel = Tunnel::wrapping(Script::silent());
        let end = tunnel.end();

        // `rustls` writes a flight in as many calls as it likes; a TDS header
        // states its packet's length, so nothing may go out until the flush.
        tunnel
            .write_all(b"\x16\x03\x01")
            .expect("the script accepts every write");
        tunnel
            .write_all(b"hello")
            .expect("the script accepts every write");
        assert!(
            tunnel.stream.sent.is_empty(),
            "a half-written flight has no length to state"
        );

        tunnel.flush().expect("the script accepts every write");
        assert_eq!(
            tunnel.stream.sent,
            packet(PacketType::PreLogin, Status::EOM, 1, b"\x16\x03\x01hello")
        );

        let framed = tunnel.stream.sent.len();
        end.handshake_done();
        tunnel
            .write_all(b"\x17\x03\x03after")
            .expect("the script accepts every write");
        tunnel.flush().expect("the script accepts every write");
        assert_eq!(
            &tunnel.stream.sent[framed..],
            b"\x17\x03\x03after",
            "an ended tunnel is the socket itself"
        );
    }

    #[test]
    fn a_tunnelled_read_strips_the_headers_and_hands_back_what_follows_them() {
        // A flight split across two packets, then the first record of the
        // encrypted session — which is not framed and may arrive in the same
        // read as the packet before it.
        let mut inbound = packet(PacketType::PreLogin, Status::NORMAL, 1, b"\x16\x03\x03one");
        inbound.extend_from_slice(&packet(PacketType::PreLogin, Status::EOM, 2, b"two"));
        inbound.extend_from_slice(b"\x17\x03\x03raw");

        // Byte at a time is the partial-arrival path; the whole transcript at
        // once is the one that leaves unframed bytes buffered when the tunnel
        // ends. Both must answer the same nine bytes and then the same six.
        for chunk in [1, READ_CHUNK] {
            let mut tunnel = Tunnel::wrapping(Script::answering(inbound.clone(), chunk));
            let end = tunnel.end();

            let mut flight = [0; 9];
            tunnel
                .read_exact(&mut flight)
                .unwrap_or_else(|e| panic!("chunk {chunk} lost the flight: {e}"));
            assert_eq!(&flight, b"\x16\x03\x03onetwo", "chunk {chunk}");

            end.handshake_done();
            let mut rest = [0; 6];
            tunnel
                .read_exact(&mut rest)
                .unwrap_or_else(|e| panic!("chunk {chunk} lost the first record: {e}"));
            assert_eq!(&rest, b"\x17\x03\x03raw", "chunk {chunk}");
        }
    }

    #[test]
    fn a_tunnel_that_loses_the_server_mid_packet_says_so_rather_than_reporting_eof() {
        // `Ok(0)` is end of file to `rustls`, and a handshake that ends in one
        // reports nothing about why. Half a packet is not end of file.
        let half = &packet(PacketType::PreLogin, Status::EOM, 1, b"\x16\x03\x03one")[..7];
        let mut tunnel = Tunnel::wrapping(Script::answering(half.to_vec(), READ_CHUNK));
        let lost = tunnel
            .read(&mut [0; 16])
            .expect_err("half a header is not a message");
        assert_eq!(lost.kind(), io::ErrorKind::UnexpectedEof);

        // A server that closes between packets, however, really is end of file.
        let mut clean = Tunnel::wrapping(Script::silent());
        assert_eq!(clean.read(&mut [0; 16]).expect("a clean close"), 0);
    }
}
