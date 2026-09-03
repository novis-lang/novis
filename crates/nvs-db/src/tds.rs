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
//! # The header is big-endian, and almost nothing after it is
//!
//! The eight bytes are type, status, **length as a big-endian `u16` counting
//! the header itself**, SPID, packet id, window. Every length prefix inside a
//! payload — LOGIN7's offsets, the token stream's — is little-endian instead.
//! The mismatch is the protocol's and it is the one thing in this module worth
//! reading the constant for rather than the code: a driver that writes the
//! header the way it writes everything else emits packets whose length is a
//! plausible number, and the server answers by closing the socket.
//!
//! **PRELOGIN is the exception inside the exception.** Its option table's
//! offsets and lengths are big-endian like the header, while the version blob
//! that table points at is little-endian like everything else, so one message
//! carries both conventions. A little-endian `11` reads as 2816 and points off
//! the end of a message the server then declines to answer, which is why
//! `prelogin_request` says it again where the bytes are written.
//!
//! The length counting the header is the second half of the same trap. A
//! packet's payload is `length - 8`, so an eight-byte header with no payload is
//! a legal, complete message — which is exactly what
//! [`PacketType::Attention`] is.
//!
//! # What is here, and what is not yet
//!
//! Framing; the block that says which server to frame *to*, since [`TdsTarget`]
//! reads a `[db.<name>]` block the way every other driver's target does; and
//! the handshake as far as encryption — [`prelogin`] asks for it and
//! [`negotiate_tls`] tunnels ADR 0067 § 3's TLS handshake inside PRELOGIN
//! packets, after which the socket is an ordinary [`NvsTls`] stream carrying
//! ordinary TDS packets.
//!
//! LOGIN7 and the token stream are the slices after this one, and each of them
//! is a payload handed to [`Wire::send`]. There is still no connection: nothing
//! calls [`negotiate_tls`] yet, because the thing that would — `TdsConn` in
//! [`mod@crate::conn`] — has no wire to hold until LOGIN7 can fill one.

use std::cell::Cell;
use std::io::{self, Read, Write};
use std::ops::Range;
use std::path::Path;
use std::rc::Rc;

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
fn fill<S: Read>(stream: &mut S, inbox: &mut Vec<u8>) -> io::Result<usize> {
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

/// The protocol version this driver speaks, as LOGIN7's `TDSVersion` field
/// spells it: TDS 7.4.
///
/// PRELOGIN's `VERSION` option is documented as the *client's* version and no
/// server acts on it, so sending the protocol version rather than a build
/// number of Novis's own says the one thing about this client that an operator
/// reading `sys.dm_exec_connections` could act on.
pub const TDS_VERSION: u32 = 0x7400_0004;

/// The `VERSION` option: `UL_VERSION` and `US_SUBBUILD`, six bytes.
const PL_VERSION: u8 = 0x00;
/// The `ENCRYPTION` option: one byte, an [`Encryption`] on either side.
const PL_ENCRYPTION: u8 = 0x01;
/// Ends the option table. It is a token with no offset and no length, so it is
/// one byte where every entry before it is five.
const PL_TERMINATOR: u8 = 0xFF;
/// One option entry: the token, then a big-endian offset and length.
const PL_ENTRY: u16 = 5;
/// How much data the `VERSION` option points at.
const PL_VERSION_LEN: u16 = 6;
/// How much data the `ENCRYPTION` option points at.
const PL_ENCRYPTION_LEN: u16 = 1;
/// Where the first blob starts: two entries and the terminator.
const PL_VERSION_AT: u16 = PL_ENTRY * 2 + 1;
/// Where the second blob starts.
const PL_ENCRYPTION_AT: u16 = PL_VERSION_AT + PL_VERSION_LEN;

/// What a PRELOGIN's `ENCRYPTION` option says, on either side of the exchange.
///
/// One enum for both halves because MS-TDS gives them one vocabulary: a server
/// answers with the byte a client would have sent, plus [`Encryption::Required`]
/// which only it may send. Novis writes [`Encryption::On`] and nothing else —
/// [ADR 0067 § 3](../../../docs/adr/0067-core-db.md) has no spelling for an
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
fn prelogin_request() -> Vec<u8> {
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
fn encryption_of(payload: &[u8]) -> io::Result<Encryption> {
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
             the login in plaintext; ADR 0067 § 3 has no spelling for that",
            offered.byte()
        )));
    }
    Ok(())
}

/// The TLS handshake's records, carried as [`PacketType::PreLogin`] payloads for
/// as long as the handshake lasts.
///
/// [ADR 0067 § 3](../../../docs/adr/0067-core-db.md)'s TLS arrives one layer
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
fn take_front(held: &mut Vec<u8>, buf: &mut [u8]) -> usize {
    let take = held.len().min(buf.len());
    buf[..take].copy_from_slice(&held[..take]);
    held.drain(..take);
    take
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
/// The whole of ADR 0067 § 3 on this backend, and it takes the wire by value
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

    /// A PRELOGIN response carrying one `ENCRYPTION` option, as a server writes
    /// it: the entry, the terminator, then the byte the entry points at.
    fn prelogin_answer(encryption: u8) -> Vec<u8> {
        let mut payload = vec![PL_ENCRYPTION];
        // One entry, then the terminator, so the byte is at offset 6.
        payload.extend_from_slice(&(PL_ENTRY + 1).to_be_bytes());
        payload.extend_from_slice(&PL_ENCRYPTION_LEN.to_be_bytes());
        payload.push(PL_TERMINATOR);
        payload.push(encryption);
        packet(PacketType::TabularResult, Status::EOM, 1, &payload)
    }

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
