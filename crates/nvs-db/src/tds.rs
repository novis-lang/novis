//! SQL Server: TDS 7.4's packet framing, which is an eight-byte header in front
//! of every message, the split a message longer than the negotiated packet size
//! takes on the way out, and the reassembly it takes on the way back.
//!
//! [ADR 0132 § 5](../../../docs/adr/0132-a-driver-is-a-sans-io-codec-over-the-parking-stream.md)
//! gives each driver its own state machine, its own error table and its own
//! codec, and here the codec is *ours*: the borrowed half every other driver in
//! this crate has does not exist for TDS. The only implementation in Rust is
//! async over `futures-io` with its TLS behind `tokio-rustls`, and this goal's
//! standing decisions refuse a runtime that spawns — so the choice is not
//! between writing this and borrowing it, it is between writing this and having
//! no SQL Server driver. What that buys back is that the framing below is a
//! pure function of its arguments and is tested without a socket.
//!
//! # A message is packets, and a token can straddle two of them
//!
//! Every other driver here reads a packet and hands it to a parser, because
//! both of the protocols they speak align a message with a packet. TDS does
//! not: a response is a stream of tokens cut into packets at whatever offset
//! the packet size lands on, so the last token in a packet is routinely half of
//! one. A caller therefore cannot parse a packet, and [`Wire::read_packet`] is
//! deliberately not the method a driver reaches for — [`Wire::read_message`] is,
//! and it concatenates until the packet carrying [`Status::EOM`] arrives.
//!
//! That costs a whole message in memory, so it is capped at [`MAX_MESSAGE`] and
//! the row path will not use it: a result set is bounded by the table, not by
//! the protocol, and reading one is [`Wire::read_packet`] in a loop with the
//! token parser holding the remainder across the boundary. Framing is what this
//! module owes that reader, and it is why the packet-level method is public
//! rather than private with the message-level one wrapping it.
//!
//! # The header is big-endian, and nothing after it is
//!
//! The eight bytes are type, status, **length as a big-endian `u16` counting
//! the header itself**, SPID, packet id, window. Every field inside a payload —
//! every length prefix in PRELOGIN, LOGIN7 and the token stream — is little-
//! endian instead. The mismatch is the protocol's and it is the one thing in
//! this module worth reading the constant for rather than the code: a driver
//! that writes the header the way it writes everything else emits packets whose
//! length is a plausible number, and the server answers by closing the socket.
//!
//! The length counting the header is the second half of the same trap. A
//! packet's payload is `length - 8`, so an eight-byte header with no payload is
//! a legal, complete message — which is exactly what
//! [`PacketType::Attention`] is.
//!
//! # What is here, and what is not yet
//!
//! Framing, and the block that says which server to frame *to*: [`TdsTarget`]
//! reads a `[db.<name>]` block the way every other driver's target does, and it
//! is here rather than with a connection because there is no connection yet.
//! PRELOGIN, ADR 0067 § 3's TLS tunnelled inside PRELOGIN packets, LOGIN7 and
//! the token stream are the slices after this one, and each of them is a
//! payload handed to [`Wire::send`].

use std::io::{self, Read, Write};
use std::ops::Range;
use std::path::Path;

use nvs_config::tree::Database;
use nvs_host::net::NvsTcp;
use nvs_host::tls::NvsTls;

use crate::conn::{BlockError, Driver, written_value};
use crate::sql::{statement_cache_for, time_zone_for};

/// The header in front of every packet: type, status, length, SPID, packet id,
/// window.
///
/// The length field counts these eight bytes as well as the payload, so this is
/// both the header's width and the smallest legal value that field may hold.
pub const HEADER: usize = 8;

/// What a connection frames with before LOGIN7 negotiates anything.
///
/// MS-TDS's own default, and the size PRELOGIN is written against: the
/// negotiation that could raise it happens inside a message that must already
/// have been framed.
pub const DEFAULT_PACKET_SIZE: u16 = 4096;

/// The smallest packet size LOGIN7 may ask for.
const MIN_PACKET_SIZE: u16 = 512;

/// The largest packet size LOGIN7 may ask for.
///
/// Not `u16::MAX`, which the length field would hold: the negotiable range
/// stops here, and a client that asked for more would be answered with
/// something else and have to notice.
const MAX_PACKET_SIZE: u16 = 32_767;

/// How much of one reassembled message this driver will hold.
///
/// The same ceiling `crate::mysql` puts on a packet, for the same reason: a
/// message's length is the server's to choose, and a bound that is the
/// protocol's own `u16` per packet with no limit on the count is no bound at
/// all. A handshake message is kilobytes; anything that could legitimately
/// approach this is a result set, which [`Wire::read_packet`] streams instead.
pub const MAX_MESSAGE: usize = 16 * 1024 * 1024;

/// How much is asked of the stream when the inbox does not hold a whole packet.
const READ_CHUNK: usize = 16 * 1024;

/// One `[db.<name>]` block, read as the facts a LOGIN7 message carries.
///
/// [`crate::MySqlTarget`]'s and [`crate::PgTarget`]'s twin, and the twinning is
/// the point: [ADR 0067 § 2](../../../docs/adr/0067-core-db.md) makes a block a
/// discriminated union on `driver`, so a server block holds the same fields
/// whichever driver reads it and the refusals are one vocabulary —
/// [`BlockError`], in [`mod@crate::conn`] for that reason.
///
/// **The target borrows the block and copies nothing**, for the reason
/// [`crate::MySqlTarget`] gives at length: owning these four strings would put
/// a second copy of the password — a `secret` at the language level (§ 3) — in
/// a struct nothing zeroes.
///
/// The address is not here. `host` is the name the server's certificate is
/// checked against; resolving it to a [`std::net::SocketAddr`] belongs to
/// whoever checked the `db.connect` capability, and a driver that re-resolved
/// the name would be connecting somewhere nobody approved.
pub struct TdsTarget<'a> {
    /// The name the server's certificate is checked against.
    pub host: &'a str,
    /// The login to authenticate as. ADR 0067 § 3 accepts `tainted` here
    /// freely: LOGIN7 carries it as a length-prefixed field, never as parsed
    /// text.
    pub user: &'a str,
    /// The password. On this protocol it goes out *as* a password rather than
    /// as a challenge response — obfuscated by LOGIN7's nibble swap, which is
    /// not encryption — which is why ADR 0067 § 3's TLS is not optional here in
    /// the way it merely defaults elsewhere.
    pub password: &'a str,
    /// The database LOGIN7 selects.
    ///
    /// Required, though the protocol would let it be omitted: a login with no
    /// database named lands in whichever one the *server* made that login's
    /// default, so an omitted field would make a connection's meaning depend on
    /// state no `[db.<name>]` block records and § 13's pool key does not cover.
    pub database: &'a str,
    /// The PEM bundle whose anchors this server's certificate is verified
    /// against, or the compiled-in Mozilla set where the block names none.
    ///
    /// Not a way to turn verification off — ADR 0067 § 3 has no spelling for
    /// that. What it changes is *whose* certificates are believed.
    pub tls_ca_file: Option<&'a Path>,
    /// The zone a `datetime2` or `datetime` off this connection is read in, as
    /// a whole number of seconds east of UTC.
    ///
    /// [ADR 0067 § 9](../../../docs/adr/0067-core-db.md)'s declared zone, read
    /// by the same `time_zone_for` the other drivers go through — **and this is
    /// the one driver that cannot send it**. SQL Server has no session time
    /// zone: `AT TIME ZONE` is an expression, and `SET` has no such setting, so
    /// there is nothing to send it to. The declared zone therefore governs
    /// decoding alone, and `GETDATE()` stays the server's own clock in the
    /// server's own zone. § 9's sentence about the server agreeing is a MySQL
    /// and PostgreSQL fact, not a property of the field.
    pub time_zone: i32,
    /// How many prepared statements this connection may keep alive on the
    /// server, [ADR 0067 § 1](../../../docs/adr/0067-core-db.md)'s
    /// `statement_cache`, through the reader every other driver's block goes
    /// through.
    pub statement_cache: usize,
}

impl<'a> TdsTarget<'a> {
    /// One `[db.<name>]` block as this driver's target, or why it is not one.
    ///
    /// # Errors
    ///
    /// [`BlockError`], in the order the checks run and the order
    /// [`crate::MySqlTarget::resolve`] runs them: the `driver` first, since a
    /// block belonging to another backend resolved here would send LOGIN7 to a
    /// server that cannot answer it; then a field belonging to another driver;
    /// then the four LOGIN7 sends, each by its own key; then § 9's zone.
    pub fn resolve(block: &'a Database) -> Result<TdsTarget<'a>, BlockError<'a>> {
        let written = block.driver.as_deref().ok_or(BlockError::NoDriver)?;
        match Driver::from_config_name(written) {
            Some(Driver::SqlServer) => {}
            Some(driver) => {
                return Err(BlockError::OtherDriver {
                    written,
                    driver,
                    expected: Driver::SqlServer,
                });
            }
            None => return Err(BlockError::UnknownDriver { written }),
        }

        if block.path.is_some() {
            return Err(BlockError::Unusable {
                field: "path",
                expected: Driver::SqlServer,
            });
        }

        let password = match (block.password.as_deref(), block.password_file.is_some()) {
            // `nvs_config::secret` materializes the file's content into
            // `password` and leaves `password_file` set, so a block with the
            // file and no value is one that never went through that pass.
            (None, true) => return Err(BlockError::SecretUnread),
            // A password may legitimately be spaces, so this one field is empty
            // only when it is *empty* — the rule `written_value`'s doc owns.
            (Some(""), _) => return Err(BlockError::Blank { field: "password" }),
            (Some(password), _) => password,
            (None, false) => {
                return Err(BlockError::Missing {
                    field: "password",
                    expected: Driver::SqlServer,
                });
            }
        };

        let host = written_value(block.host.as_deref(), "host", Driver::SqlServer)?;
        let user = written_value(block.user.as_deref(), "user", Driver::SqlServer)?;
        let database = written_value(block.database.as_deref(), "database", Driver::SqlServer)?;

        let Some(time_zone) = time_zone_for(block) else {
            return Err(BlockError::TimeZone {
                // `time_zone_for` answers `Some(0)` for an absent field, so
                // reaching here means the block wrote one.
                written: block.time_zone.as_deref().unwrap_or_default(),
            });
        };

        Ok(TdsTarget {
            host,
            user,
            password,
            database,
            // Absolute and trust-checked by `nvs_config::db` before the block
            // reached here, so there is nothing for this resolver to decide:
            // written or not written is the whole of it.
            tls_ca_file: block.tls_ca_file.as_deref().map(Path::new),
            time_zone,
            statement_cache: statement_cache_for(block),
        })
    }
}

impl std::fmt::Debug for TdsTarget<'_> {
    /// Everything but the password, which is a `secret` at the language level
    /// (ADR 0067 § 3) and is not printed in any rendering.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TdsTarget")
            .field("host", &self.host)
            .field("user", &self.user)
            .field("database", &self.database)
            .field("time_zone", &self.time_zone)
            .field("statement_cache", &self.statement_cache)
            .finish_non_exhaustive()
    }
}

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
pub struct Status(u8);

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
    /// ADR 0067 § 13's reset, as a bit rather than as a statement: the next
    /// batch or procedure call resets the session before it runs, which is
    /// `sp_reset_connection` without the round trip of asking for it. It rides
    /// the **first** packet of that message — see [`Codec::encode`].
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
fn malformed(what: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, what)
}

/// A TDS conversation over one stream.
///
/// [`crate::mysql`]'s `Wire` and for its reasons: one place owns the framing
/// and the buffered remainder, so no caller can write half a packet, and the
/// default type parameter is the stream a real connection has while a test
/// substitutes anything that reads and writes.
pub struct Wire<S: Read + Write = NvsTls<NvsTcp>> {
    stream: S,
    inbox: Vec<u8>,
    codec: Codec,
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
        })
    }

    /// The framing, for the negotiation that resizes it.
    pub fn codec(&mut self) -> &mut Codec {
        &mut self.codec
    }

    /// The stream underneath, for the tests that assert on what was written to
    /// it. There is no non-test reader: a driver talks through the three
    /// methods below.
    #[cfg(test)]
    fn peer(&self) -> &S {
        &self.stream
    }

    /// Frames `payload` as a whole message, writes all of it, and flushes.
    ///
    /// One function because a half-written message is the shape ADR 0132 § 4
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

            let filled = self.inbox.len();
            self.inbox.resize(filled + READ_CHUNK, 0);
            let read = self.stream.read(&mut self.inbox[filled..]);
            // Restore the length before the `?`: a failed read must not leave
            // the inbox claiming the zeroes it was given room in.
            let read = match read {
                Ok(read) => read,
                Err(e) => {
                    self.inbox.truncate(filled);
                    return Err(e);
                }
            };
            self.inbox.truncate(filled + read);

            if read == 0 {
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

    /// A stream that answers with a canned transcript and keeps what was
    /// written.
    ///
    /// Unlike [`crate::mysql`]'s `Peer` this one does not script a server: at
    /// this layer nothing depends on what the previous message was, so a fixed
    /// transcript asserts everything, and what a scripted server would add is
    /// the sequencing the slices above this one bring.
    ///
    /// `chunk` is the point of it: it caps what one `read` answers, so the
    /// partial-arrival path — the one a real socket takes and a `Cursor` never
    /// does — is exercised by setting it to 1.
    struct Script {
        inbound: Vec<u8>,
        read: usize,
        chunk: usize,
        sent: Vec<u8>,
    }

    impl Script {
        fn answering(inbound: Vec<u8>, chunk: usize) -> Script {
            Script {
                inbound,
                read: 0,
                chunk,
                sent: Vec::new(),
            }
        }

        fn silent() -> Script {
            Script::answering(Vec::new(), READ_CHUNK)
        }
    }

    impl Write for Script {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.sent.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl Read for Script {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let left = &self.inbound[self.read..];
            let take = left.len().min(buf.len()).min(self.chunk);
            buf[..take].copy_from_slice(&left[..take]);
            self.read += take;
            Ok(take)
        }
    }

    /// One packet as a server would write it, for the read-side cases.
    fn packet(kind: PacketType, status: Status, id: u8, payload: &[u8]) -> Vec<u8> {
        let length = u16::try_from(HEADER + payload.len()).expect("a test packet fits");
        let mut out = vec![kind.byte(), status.bits()];
        out.extend_from_slice(&length.to_be_bytes());
        out.extend_from_slice(&[0, 0, id, 0]);
        out.extend_from_slice(payload);
        out
    }

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

    /// A `[db.<name>]` block with every field a SQL Server connection reads, as
    /// an operator writes it and as `nvs_config` hands it over.
    fn block() -> Database {
        Database {
            driver: Some("mssql".to_owned()),
            host: Some("mssql.test".to_owned()),
            user: Some("sa".to_owned()),
            password: Some("hunter2".to_owned()),
            database: Some("novis_test".to_owned()),
            time_zone: Some("+02:00".to_owned()),
            ..Database::default()
        }
    }

    /// A complete block resolves to what LOGIN7 sends, and the two fields with
    /// readers of their own are asserted through them: § 9's zone arrives as
    /// seconds rather than as the text an operator wrote, and an unwritten
    /// `statement_cache` is § 1's default where a written `0` is the cache off.
    #[test]
    fn a_complete_block_resolves_to_the_target_login7_sends() {
        let mut block = block();
        let target = TdsTarget::resolve(&block).expect("a complete block resolves");
        assert_eq!(target.host, "mssql.test");
        assert_eq!(target.user, "sa");
        assert_eq!(target.password, "hunter2");
        assert_eq!(target.database, "novis_test");
        assert_eq!(target.time_zone, 2 * 3600);
        assert!(target.tls_ca_file.is_none());
        assert_eq!(
            target.statement_cache,
            crate::sql::DEFAULT_STATEMENT_CACHE,
            "an unwritten field is § 1's default and not zero"
        );

        block.statement_cache = Some(0);
        assert_eq!(
            TdsTarget::resolve(&block)
                .expect("a block that turns the cache off resolves")
                .statement_cache,
            0,
            "a written `0` is § 1's cache turned off, which is the answer an \
             `unwrap_or_default` reader loses"
        );
        block.statement_cache = None;

        block.time_zone = None;
        assert_eq!(
            TdsTarget::resolve(&block)
                .expect("an unwritten zone is UTC")
                .time_zone,
            0
        );

        block.time_zone = Some("halfway east".to_owned());
        assert_eq!(
            TdsTarget::resolve(&block).unwrap_err(),
            BlockError::TimeZone {
                written: "halfway east"
            }
        );
    }

    /// ADR 0067 § 2's discriminant, from this side of it.
    ///
    /// Every driver's resolver owes this case, and the reason it is not
    /// redundant with `crate::mysql`'s is the direction: what is asserted is
    /// that *this* resolver refuses the other four, so a block an operator
    /// wrote for one backend cannot open a connection that speaks another.
    #[test]
    fn a_block_that_is_not_sql_servers_is_refused() {
        let mut block = block();
        block.driver = None;
        assert_eq!(
            TdsTarget::resolve(&block).unwrap_err(),
            BlockError::NoDriver
        );

        for (written, driver) in [
            ("mysql", Driver::MySql),
            ("mariadb", Driver::MariaDb),
            ("postgres", Driver::Postgres),
            ("sqlite", Driver::Sqlite),
        ] {
            block.driver = Some(written.to_owned());
            assert_eq!(
                TdsTarget::resolve(&block).unwrap_err(),
                BlockError::OtherDriver {
                    written,
                    driver,
                    expected: Driver::SqlServer,
                }
            );
        }

        block.driver = Some("sqlserver".to_owned());
        assert_eq!(
            TdsTarget::resolve(&block).unwrap_err(),
            BlockError::UnknownDriver {
                written: "sqlserver"
            },
            "the one spelling is `mssql`, which is what `tools/db-matrix.py` \
             and `tests/db/compose.yaml` write"
        );
    }

    /// The four fields LOGIN7 sends are refused one by one, each naming its own
    /// key — and `path` is refused as a field of the driver that has one.
    #[test]
    fn a_field_that_is_missing_blank_or_another_drivers_is_named_by_its_key() {
        let mut with_path = block();
        with_path.path = Some("/tmp/novis.db".to_owned());
        assert_eq!(
            TdsTarget::resolve(&with_path).unwrap_err(),
            BlockError::Unusable {
                field: "path",
                expected: Driver::SqlServer,
            }
        );

        for field in ["host", "user", "database"] {
            let mut missing = block();
            *written_field(&mut missing, field) = None;
            assert_eq!(
                TdsTarget::resolve(&missing).unwrap_err(),
                BlockError::Missing {
                    field,
                    expected: Driver::SqlServer,
                }
            );

            let mut blank = block();
            *written_field(&mut blank, field) = Some(String::new());
            assert_eq!(
                TdsTarget::resolve(&blank).unwrap_err(),
                BlockError::Blank { field }
            );
        }
    }

    /// The password is the field with its own arm, so it gets its own case: an
    /// unread `password_file` is a block that never went through
    /// `nvs_config::secret`, and is not the same failure as a missing password.
    #[test]
    fn a_password_that_is_missing_blank_or_unread_is_three_different_refusals() {
        let mut block = block();
        block.password = None;
        assert_eq!(
            TdsTarget::resolve(&block).unwrap_err(),
            BlockError::Missing {
                field: "password",
                expected: Driver::SqlServer,
            }
        );

        block.password_file = Some("/run/secrets/mssql".to_owned());
        assert_eq!(
            TdsTarget::resolve(&block).unwrap_err(),
            BlockError::SecretUnread,
            "the file is named and nothing read it, which is a boot-order bug \
             and not a block missing a credential"
        );

        block.password_file = None;
        block.password = Some(String::new());
        assert_eq!(
            TdsTarget::resolve(&block).unwrap_err(),
            BlockError::Blank { field: "password" }
        );

        block.password = Some("   ".to_owned());
        assert_eq!(
            TdsTarget::resolve(&block)
                .expect("spaces are a password")
                .password,
            "   ",
        );
    }

    /// Which `Option<String>` a field name selects, so the three that resolve
    /// identically are asserted in one loop rather than in three copies.
    fn written_field<'a>(block: &'a mut Database, field: &str) -> &'a mut Option<String> {
        match field {
            "host" => &mut block.host,
            "user" => &mut block.user,
            "database" => &mut block.database,
            other => unreachable!("no case names {other}"),
        }
    }

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
