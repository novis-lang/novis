//! Packet framing: the eight-byte header in front of every message, the split
//! a message longer than the negotiated packet size takes on the way out, and
//! the reassembly it takes on the way back.
//!
//! [`Codec`] is the whole of it and it touches no stream — [`decode`] is a pure
//! function of the bytes that have arrived, so every case below is written
//! without a socket. The header's length field is a **big-endian** `u16`
//! counting those eight bytes as well as the payload, and it is the one field
//! in this driver written that way; [`mod@super`]'s own doc owns why.

use super::*;

/// The message types this driver speaks.
///
/// Closed on purpose, and it is not MS-TDS's whole roster: `SSPI` (0x11) and
/// the federated authentication token (0x08) belong to Windows integrated and
/// Azure AD authentication, which this driver does not offer, and a server
/// sending one is answering a login this driver did not write. [`decode`]
/// therefore refuses an unknown type byte rather than carrying it as a number,
/// so that a message this driver cannot act on fails at the frame instead of
/// inside a parser that would read the payload as tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketType {
    /// A statement as text — ADR 0067 § 1 allows this for the commands that
    /// bind nothing, never for a caller's SQL.
    SqlBatch,
    /// A procedure call, which is what a prepared statement's execution is on
    /// this backend, and what ADR 0067 § 13's `sp_reset_connection` is.
    Rpc,
    /// A server's answer to either of the above: the token stream.
    TabularResult,
    /// Cancel whatever is running. Its payload is empty, so the message is the
    /// header alone.
    Attention,
    /// `BEGIN`/`COMMIT`/`ROLLBACK` and ADR 0067 § 7's savepoints, which are a
    /// request of their own here rather than text.
    TransactionManager,
    /// The credential, once the socket is encrypted.
    Login7,
    /// The version and capability exchange, and the tunnel ADR 0067 § 3's TLS
    /// handshake rides inside.
    PreLogin,
}

impl PacketType {
    /// The byte this writes into a header.
    ///
    /// A `match` rather than a `#[repr(u8)]` cast, so that the numbers appear
    /// once each and in one direction only — the reverse is [`PacketType::from_byte`]
    /// and the two are held together by `a_packet_types_byte_round_trips`.
    #[must_use]
    pub const fn byte(self) -> u8 {
        match self {
            PacketType::SqlBatch => 0x01,
            PacketType::Rpc => 0x03,
            PacketType::TabularResult => 0x04,
            PacketType::Attention => 0x06,
            PacketType::TransactionManager => 0x0E,
            PacketType::Login7 => 0x10,
            PacketType::PreLogin => 0x12,
        }
    }

    /// The type that byte names, or `None` for one this driver does not speak.
    #[must_use]
    pub const fn from_byte(byte: u8) -> Option<PacketType> {
        match byte {
            0x01 => Some(PacketType::SqlBatch),
            0x03 => Some(PacketType::Rpc),
            0x04 => Some(PacketType::TabularResult),
            0x06 => Some(PacketType::Attention),
            0x0E => Some(PacketType::TransactionManager),
            0x10 => Some(PacketType::Login7),
            0x12 => Some(PacketType::PreLogin),
            _ => None,
        }
    }
}

/// The header's second byte: a set of bits, not an enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Status(pub(super) u8);

impl Status {
    /// A packet with more of its message behind it.
    pub const NORMAL: Status = Status(0x00);
    /// The last packet of a message. [`Wire::read_message`] stops on it, and
    /// [`Codec::encode`] sets it on the last packet it writes and no other.
    pub const EOM: Status = Status(0x01);
    /// The request this packet belongs to has been abandoned; the server
    /// discards it. Sent with [`Status::EOM`] as the tail of a cancelled
    /// message.
    pub const IGNORE: Status = Status(0x02);
    /// ADR 0067 § 13's reset, as a bit rather than as a statement: the batch or
    /// procedure call carrying it resets the session before it runs, which is
    /// `sp_reset_connection` said in the header rather than asked for by name.
    /// It rides the **first** packet of that message — see [`Codec::encode`].
    ///
    /// **[`reset_session`] spends a message of its own on it rather than
    /// letting it ride the next real one.** § 13 makes the reset a security
    /// boundary that a failed reset destroys the connection over, and a bit
    /// riding the next request cannot be proven until that request — belonging
    /// to the program this connection was just handed to — has already run on
    /// it. The cheaper spelling is cheaper only because it moves the proof to
    /// the wrong side of the boundary.
    pub const RESET_CONNECTION: Status = Status(0x08);
    /// [`Status::RESET_CONNECTION`] for a connection inside a transaction the
    /// caller wants kept. Novis never sets it: § 13's reset happens at release,
    /// where an open transaction is a connection that is not poolable rather
    /// than one to preserve.
    pub const RESET_CONNECTION_SKIP_TRAN: Status = Status(0x10);

    /// Whether every bit of `other` is set here.
    #[must_use]
    pub const fn contains(self, other: Status) -> bool {
        self.0 & other.0 == other.0
    }

    /// The byte itself, for the tests that assert on a header.
    #[must_use]
    pub const fn bits(self) -> u8 {
        self.0
    }
}

impl std::ops::BitOr for Status {
    type Output = Status;

    fn bitor(self, other: Status) -> Status {
        Status(self.0 | other.0)
    }
}

/// Where one packet sits inside a buffer, decided without moving any of it.
///
/// [`decode`] answers in offsets rather than in slices so that it borrows
/// nothing: the caller owns the buffer, copies the body out and drops `total`
/// bytes off the front, and a decoder that had borrowed could not be asked for
/// the second packet until the first was dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Framed {
    /// The header's first byte, resolved.
    pub kind: PacketType,
    /// The header's second byte.
    pub status: Status,
    /// Where the payload is — `HEADER..total`, and empty for an
    /// [`PacketType::Attention`].
    pub body: Range<usize>,
    /// Header plus payload: what the caller consumes from the front of the
    /// buffer, and what the header's length field says.
    pub total: usize,
}

/// One packet, read off a wire.
///
/// Distinct from [`Message`] by the status byte, which a reassembled message no
/// longer has one of: its packets carried several, and the only one that meant
/// anything to the reader was the [`Status::EOM`] that ended it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packet {
    /// Which message type this packet belongs to.
    pub kind: PacketType,
    /// This packet's own status bits.
    pub status: Status,
    /// The bytes after the header.
    pub payload: Vec<u8>,
}

/// One whole message: every packet's payload, concatenated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// The type every packet of it declared — [`Wire::read_message`] refuses a
    /// continuation that disagrees.
    pub kind: PacketType,
    /// The concatenation, which is what a parser reads.
    pub payload: Vec<u8>,
}

/// The framing itself, with no stream in it.
///
/// Its only state is the size it splits at, because the packet id restarts at 1
/// for each message: a codec that carried the counter between messages would
/// have to be reset at every boundary, and forgetting to is a bug the server
/// cannot report, since MS-TDS says the field is ignored on arrival. Numbering
/// it correctly anyway costs nothing and is what a packet capture is read
/// against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Codec {
    packet_size: u16,
}

impl Default for Codec {
    /// Framing at [`DEFAULT_PACKET_SIZE`], which is what PRELOGIN is written
    /// against.
    fn default() -> Codec {
        Codec {
            packet_size: DEFAULT_PACKET_SIZE,
        }
    }
}

impl Codec {
    /// The size this splits outgoing messages at.
    #[must_use]
    pub const fn packet_size(self) -> u16 {
        self.packet_size
    }

    /// Frame at the size the server agreed to in LOGIN7's answer.
    ///
    /// Only the write side has one. An incoming packet is whatever length its
    /// own header claims, up to the `u16` that field is, and a server is
    /// entitled to answer a request for one size by using another — so
    /// [`decode`] never consults this.
    ///
    /// # Errors
    ///
    /// `InvalidInput` outside MS-TDS's negotiable 512..=32767.
    pub fn set_packet_size(&mut self, size: u16) -> io::Result<()> {
        if !(MIN_PACKET_SIZE..=MAX_PACKET_SIZE).contains(&size) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "TDS packet size {size} is outside the negotiable range \
                     {MIN_PACKET_SIZE}..={MAX_PACKET_SIZE}"
                ),
            ));
        }
        self.packet_size = size;
        Ok(())
    }

    /// Appends `payload` to `out` as one whole message: as many packets as the
    /// packet size needs, headers and all.
    ///
    /// `status` is the caller's *extra* bits and it rides the **first** packet
    /// alone, which is where MS-TDS specifies [`Status::RESET_CONNECTION`] and
    /// is the reading a server acts on. [`Status::EOM`] is this function's to
    /// set and it lands on the last packet alone; a caller passing
    /// [`Status::NORMAL`] — the ordinary case — gets a message whose bits are
    /// entirely decided here.
    ///
    /// An empty payload is one packet with an empty body, not zero packets:
    /// that is a legal message and [`PacketType::Attention`] is nothing else.
    pub fn encode(self, kind: PacketType, status: Status, payload: &[u8], out: &mut Vec<u8>) {
        let limit = usize::from(self.packet_size) - HEADER;
        let mut id: u8 = 1;
        let mut rest = payload;
        loop {
            let (chunk, tail) = rest.split_at(rest.len().min(limit));
            let first = id == 1;
            let last = tail.is_empty();

            let mut bits = if first { status } else { Status::NORMAL };
            if last {
                bits = bits | Status::EOM;
            }
            // Cannot truncate: `chunk` is at most `packet_size - HEADER`.
            let length = u16::try_from(HEADER + chunk.len())
                .expect("a packet is at most `packet_size`, which is a u16");

            out.push(kind.byte());
            out.push(bits.bits());
            out.extend_from_slice(&length.to_be_bytes());
            // SPID and window are the server's fields; a client writes zero.
            out.extend_from_slice(&0u16.to_be_bytes());
            out.push(id);
            out.push(0);
            out.extend_from_slice(chunk);

            if last {
                return;
            }
            id = id.wrapping_add(1);
            rest = tail;
        }
    }
}

/// Where the packet at the front of `inbox` is, or `None` while less than all
/// of it has arrived.
///
/// Sans-IO in the literal sense: it reads a buffer and answers offsets, so
/// every framing case this module is tested for is tested without a stream.
///
/// # Errors
///
/// `InvalidData` for a type byte this driver does not speak, and for a length
/// field below [`HEADER`] — which counts itself, so a packet cannot claim to be
/// shorter than its own header without the stream having lost sync.
pub fn decode(inbox: &[u8]) -> io::Result<Option<Framed>> {
    if inbox.len() < HEADER {
        return Ok(None);
    }
    let Some(kind) = PacketType::from_byte(inbox[0]) else {
        return Err(malformed(format!(
            "TDS packet type 0x{:02X} is not one this driver speaks",
            inbox[0]
        )));
    };
    let total = usize::from(u16::from_be_bytes([inbox[2], inbox[3]]));
    if total < HEADER {
        return Err(malformed(format!(
            "TDS packet length {total} is shorter than the {HEADER}-byte header it counts"
        )));
    }
    if inbox.len() < total {
        return Ok(None);
    }
    Ok(Some(Framed {
        kind,
        status: Status(inbox[1]),
        body: HEADER..total,
        total,
    }))
}

/// A protocol failure, as the `io::Error` every driver in this crate reports
/// one as.
pub(super) fn malformed(what: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, what)
}

/// Asks `stream` for more bytes, onto the tail of `inbox`, and answers how many
/// arrived — `0` for end of file.
///
/// A free function because both readers in this module need it and their
/// buffers are not the same field: [`Wire`] fills its own inbox and [`Tunnel`]
/// fills the one it strips packet headers off. The length is restored before
/// the error leaves, so a failed read never leaves an inbox claiming the zeroes
/// it was given room in — the bug this is factored out to have exactly once.
///
/// # Errors
///
/// Whatever the stream reported.
pub(super) fn fill<S: Read>(stream: &mut S, inbox: &mut Vec<u8>) -> io::Result<usize> {
    let filled = inbox.len();
    inbox.resize(filled + READ_CHUNK, 0);
    match stream.read(&mut inbox[filled..]) {
        Ok(read) => {
            inbox.truncate(filled + read);
            Ok(read)
        }
        Err(e) => {
            inbox.truncate(filled);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tds::testing::*;

    #[test]
    fn a_packet_types_byte_round_trips() {
        for kind in [
            PacketType::SqlBatch,
            PacketType::Rpc,
            PacketType::TabularResult,
            PacketType::Attention,
            PacketType::TransactionManager,
            PacketType::Login7,
            PacketType::PreLogin,
        ] {
            assert_eq!(PacketType::from_byte(kind.byte()), Some(kind));
        }
    }

    #[test]
    fn a_header_is_eight_bytes_and_its_length_is_big_endian() {
        let mut out = Vec::new();
        Codec::default().encode(PacketType::PreLogin, Status::NORMAL, b"abc", &mut out);
        assert_eq!(
            out,
            vec![
                0x12, 0x01, 0x00, 0x0B, 0x00, 0x00, 0x01, 0x00, b'a', b'b', b'c'
            ],
            "type, status, a big-endian length counting the header, SPID, id, window"
        );
    }

    #[test]
    fn an_empty_message_is_the_header_alone() {
        let mut out = Vec::new();
        Codec::default().encode(PacketType::Attention, Status::NORMAL, &[], &mut out);
        assert_eq!(out, vec![0x06, 0x01, 0x00, 0x08, 0x00, 0x00, 0x01, 0x00]);
    }

    #[test]
    fn a_message_past_the_packet_size_is_split_and_only_its_last_packet_ends_it() {
        let mut codec = Codec::default();
        codec.set_packet_size(512).expect("512 is negotiable");
        let payload = vec![b'x'; 1000];

        let mut out = Vec::new();
        codec.encode(PacketType::SqlBatch, Status::NORMAL, &payload, &mut out);

        assert_eq!(out.len(), 512 + 504, "1000 bytes in bodies of 504 and 496");
        assert_eq!(
            &out[..8],
            &[0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x01, 0x00],
            "the first packet is full, is packet 1, and does not end the message"
        );
        assert_eq!(
            &out[512..520],
            &[0x01, 0x01, 0x01, 0xF8, 0x00, 0x00, 0x02, 0x00],
            "the second is 504 bytes long, is packet 2, and carries EOM"
        );
    }

    #[test]
    fn an_extra_status_bit_rides_the_first_packet_and_eom_the_last() {
        let mut codec = Codec::default();
        codec.set_packet_size(512).expect("512 is negotiable");

        let mut out = Vec::new();
        codec.encode(
            PacketType::Rpc,
            Status::RESET_CONNECTION,
            &vec![b'x'; 1000],
            &mut out,
        );

        assert_eq!(out[1], 0x08, "the reset bit is on the first packet");
        assert_eq!(
            out[512 + 1],
            0x01,
            "and not on the last, which carries EOM alone"
        );
    }

    #[test]
    fn a_packet_size_outside_the_negotiable_range_is_refused() {
        let mut codec = Codec::default();
        for size in [0, MIN_PACKET_SIZE - 1, MAX_PACKET_SIZE + 1] {
            let refused = codec
                .set_packet_size(size)
                .expect_err("outside 512..=32767");
            assert_eq!(refused.kind(), io::ErrorKind::InvalidInput);
        }
        assert_eq!(
            codec.packet_size(),
            DEFAULT_PACKET_SIZE,
            "and changes nothing"
        );
        codec
            .set_packet_size(MAX_PACKET_SIZE)
            .expect("the top of the range is in it");
        assert_eq!(codec.packet_size(), MAX_PACKET_SIZE);
    }

    #[test]
    fn a_partly_arrived_packet_is_not_a_packet_yet() {
        let whole = packet(PacketType::TabularResult, Status::EOM, 1, b"hello");
        assert_eq!(
            decode(&whole[..whole.len() - 1]).expect("a short buffer is not an error"),
            None,
        );
        let framed = decode(&whole)
            .expect("a whole packet decodes")
            .expect("and is there");
        assert_eq!(framed.total, 13);
        assert_eq!(&whole[framed.body], b"hello");
    }

    #[test]
    fn a_message_split_across_packets_is_read_back_as_one_payload() {
        let mut inbound = packet(PacketType::TabularResult, Status::NORMAL, 1, b"one ");
        inbound.extend(packet(PacketType::TabularResult, Status::EOM, 2, b"two"));
        // A byte at a time: a real socket delivers a packet in pieces, and a
        // reader that assumed otherwise passes against any whole-buffer stub.
        let mut wire = Wire::new(Script::answering(inbound, 1));

        let message = wire.read_message().expect("both packets are there");

        assert_eq!(message.kind, PacketType::TabularResult);
        assert_eq!(message.payload, b"one two");
    }

    #[test]
    fn a_continuation_packet_of_another_type_is_refused() {
        let mut inbound = packet(PacketType::TabularResult, Status::NORMAL, 1, b"one ");
        inbound.extend(packet(PacketType::PreLogin, Status::EOM, 2, b"two"));
        let mut wire = Wire::new(Script::answering(inbound, READ_CHUNK));

        let refused = wire.read_message().expect_err("the stream is out of sync");

        assert_eq!(refused.kind(), io::ErrorKind::InvalidData);
        assert!(
            refused.to_string().contains("0x12"),
            "naming the type that arrived: {refused}"
        );
    }

    #[test]
    fn an_unspoken_packet_type_is_refused_at_the_frame() {
        // 0x11 is SSPI: a real TDS type, and the answer to a login this driver
        // does not write.
        let refused = decode(&[0x11, 0x01, 0x00, 0x08, 0x00, 0x00, 0x01, 0x00])
            .expect_err("not a type this driver speaks");
        assert_eq!(refused.kind(), io::ErrorKind::InvalidData);
        assert!(refused.to_string().contains("0x11"), "{refused}");
    }

    #[test]
    fn a_length_shorter_than_the_header_is_refused() {
        let refused = decode(&[0x04, 0x01, 0x00, 0x07, 0x00, 0x00, 0x01, 0x00])
            .expect_err("a packet cannot be shorter than its own header");
        assert_eq!(refused.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn a_stream_that_closes_mid_packet_is_an_unexpected_eof() {
        let whole = packet(PacketType::TabularResult, Status::EOM, 1, b"hello");
        let mut wire = Wire::new(Script::answering(whole[..6].to_vec(), READ_CHUNK));

        let refused = wire.read_packet().expect_err("the header never finished");

        assert_eq!(refused.kind(), io::ErrorKind::UnexpectedEof);
    }
}
