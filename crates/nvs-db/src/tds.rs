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
//! ordinary TDS packets; [`login7_request`], the credential-carrying message
//! that rides that session and is the reason § 3's TLS is not optional here;
//! and [`Tokens`], which reads that message's answer as far as `LOGINACK`,
//! `ENVCHANGE`, `ERROR`, `INFO` and `DONE`.
//!
//! **The handshake now runs**: [`TdsConn::connect`] opens the socket and
//! [`login`] sends the credential and reads what came back, so a `TdsConn`
//! holds a live encrypted wire and the framing the server's `ENVCHANGE`
//! settled. **A statement's answer reads back too**: [`read_rows`] takes a wire
//! with a request already on it and answers a [`TdsRows`], which streams `ROW`,
//! `NBCROW` and `PLP` over [`Wire::read_packet`] with the remainder held across
//! the packet boundary — the reader the first section says [`Tokens`] cannot be.
//!
//! What is not here is what puts a request on the wire in the first place: ADR
//! 0067 § 1's cache over `sp_prepexec`, § 5's parameters as an RPC's arguments,
//! and § 13's reset. Two tokens wait on those rather than on a reader —
//! `RETURNSTATUS` and `RETURNVALUE`, which only a procedure call can produce —
//! and [`TdsRows::step`] names their byte in its refusal until it does.

use std::cell::Cell;
use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::ops::Range;
use std::path::Path;
use std::rc::Rc;
use std::time::Instant;

use nvs_config::tree::Database;
use nvs_host::net::NvsTcp;
use nvs_host::tls::NvsTls;

use crate::conn::{
    BlockError, ColumnType, DbErrorKind, Driver, ServerError, State, TdsConn, written_value,
};
use crate::span::QuerySpan;
use crate::sql::{Dialect, StatementCache, statement_cache_for, time_zone_for};

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

/// LOGIN7's fixed header, and therefore where its first blob starts.
///
/// Twelve scalar fields (36 bytes), twelve offset/length pairs (48), the
/// six-byte `ClientID` and `cbSSPILong`: 94, none of which varies with what
/// this driver has to say. Every offset in the message is counted from the
/// start of the *message*, so this is also the smallest value any of them may
/// hold — a pair pointing below it points into the header that holds it.
const LOGIN7_HEADER: u16 = 94;

/// The longest any of LOGIN7's variable-length fields may be, in characters.
///
/// MS-TDS's own limit on `cchUserName`, `cchPassword`, `cchDatabase` and the
/// rest, and it is checked here rather than left to the server: an over-long
/// field comes back as a login failure that names nothing, where the thing an
/// operator has to fix is the `[db.<name>]` block that wrote it. The length
/// field is a `u16` and would hold far more, so this is the protocol's rule
/// and not the field's width.
const MAX_FIELD_CHARS: u16 = 128;

/// What LOGIN7 XORs each nibble-swapped password byte with.
///
/// MS-TDS calls the result an encrypted password. It is not one — the
/// transformation is fixed, public and its own inverse — which is why
/// [`TdsTarget::password`] says ADR 0067 § 3's TLS is not optional on this
/// backend in the way it merely defaults elsewhere.
const PASSWORD_XOR: u8 = 0xA5;

/// What this client calls itself, in both `AppName` and `CltIntName`.
///
/// One string in the two fields because on this driver they are the same fact:
/// there is no application name a `[db.<name>]` block carries, and the
/// interface is this crate either way. It is what
/// `sys.dm_exec_sessions.program_name` shows, which is where a SQL Server
/// operator looks to find out whose connection a session is.
const CLIENT_NAME: &str = "Novis";

/// `ClientLCID`: US English.
///
/// Not zero, which is not a locale at all. Nothing this driver parses depends
/// on it — [ADR 0067 § 8](../../../docs/adr/0067-core-db.md) normalises on the
/// error *number* and never on the message text — so all it decides is which
/// language a server writes a message Novis will only ever log.
const CLIENT_LCID: u32 = 0x0409;

/// `fUseDB`: the server says so when the database changes under this
/// connection.
///
/// A `USE` also changes the collation the server describes columns with, so a
/// client that had not asked for the notification would be decoding against a
/// collation it could not know had moved. The notification is an `ENVCHANGE`
/// token, which is the slice after this one.
const OPT1_USE_DB_NOTIFY: u8 = 0x20;

/// `fDatabase`: failing to open [`TdsTarget::database`] fails the login.
///
/// The decision this message owes. MS-TDS's other reading is a *warning*,
/// which leaves the connection open in whichever database the server made this
/// login's default. [`TdsTarget::database`] is required precisely so that a
/// connection cannot mean whatever that was, and a warning would give the
/// field away at the last moment: § 13's pool would then hold connections
/// whose database depends on server-side state no pool key covers, and a
/// request would read the right rows or the wrong ones depending on how the
/// login was provisioned.
const OPT1_INIT_DB_FATAL: u8 = 0x40;

/// `fLanguage`: the same reading for the language, which this driver never
/// names.
///
/// It sends an empty `Language`, so there is no initial change to fail. The
/// bit is set anyway because the alternative reading is "warn and carry on",
/// and a login that half-succeeded is not a state anything above wants to
/// discover later.
const OPT2_INIT_LANG_FATAL: u8 = 0x01;

/// `fODBC`, which is not about ODBC: it is how a client asks for the ANSI
/// session defaults.
///
/// The server answers it by setting `ANSI_DEFAULTS` on, `IMPLICIT_TRANSACTIONS`
/// off, `TEXTSIZE` to its maximum and `ROWCOUNT` to unlimited, and two of those
/// are load-bearing. Implicit transactions off is what makes [ADR 0067
/// § 7](../../../docs/adr/0067-core-db.md)'s closure the only thing that ever
/// opens a transaction on this connection — with them on, a bare `SELECT`
/// opens one nothing commits, and § 13's reset would be destroying a connection
/// per request. `ROWCOUNT` unlimited is what stops a server-side default from
/// silently truncating a result set.
const OPT2_ODBC: u8 = 0x02;

/// `fUnknownCollationHandling`: this client accepts a collation newer than the
/// ones TDS 7.0 knew about.
///
/// Nothing here reads a collation — § 9's map decodes a column by its type —
/// so what the bit buys is that the server is never pushed into describing one
/// the older, lossier way on this client's account.
const OPT3_UNKNOWN_COLLATION: u8 = 0x08;

/// `OptionFlags1`.
///
/// Every other bit in it — byte order, character set, float format, dump/load
/// — has exactly one reading this driver could mean, and zero is that reading:
/// little-endian, the ASCII family, IEEE 754, and no BCP.
const OPTION_FLAGS_1: u8 = OPT1_USE_DB_NOTIFY | OPT1_INIT_DB_FATAL;

/// `OptionFlags2`.
///
/// `fUserType` stays zero — an ordinary login, not a replication or
/// remote-user one — and `fIntegratedSecurity` stays off, which is what makes
/// the `Password` field the credential rather than an SSPI blob this driver
/// has no way to produce.
const OPTION_FLAGS_2: u8 = OPT2_INIT_LANG_FATAL | OPT2_ODBC;

/// `TypeFlags`: an ordinary SQL client, no OLE DB behaviour, and **not**
/// `fReadOnlyIntent` — an availability-group routing hint no `[db.<name>]`
/// field asks for and which this driver would therefore be inventing.
const TYPE_FLAGS: u8 = 0x00;

/// `OptionFlags3`.
///
/// `fChangePassword` and `fUserInstance` name features this driver does not
/// offer, and `fExtension` is the only way to send a `FeatureExt` block, which
/// nothing here has anything to put in.
const OPTION_FLAGS_3: u8 = OPT3_UNKNOWN_COLLATION;

/// One variable-length LOGIN7 field: its characters appended to `blobs`, and
/// the offset/length pair the fixed header carries for it.
///
/// **The pair's two numbers are in different units**, which is the trap this
/// function exists to have exactly once: the offset is *bytes* from the start
/// of the message, the length is *characters*, so a field at `(at, n)` spans
/// `2n` bytes. Both are little-endian, unlike PRELOGIN's table in the same
/// conversation. A supplementary character is two UTF-16 code units and
/// therefore costs two of [`MAX_FIELD_CHARS`], which is the protocol's
/// accounting and not this driver's.
///
/// `obfuscate` is LOGIN7's nibble swap and XOR, which only the password takes.
///
/// # Errors
///
/// `InvalidInput` for a field past [`MAX_FIELD_CHARS`], naming the key whose
/// `[db.<name>]` value is too long rather than the protocol field it fills.
fn placed(
    blobs: &mut Vec<u8>,
    text: &str,
    field: &'static str,
    obfuscate: bool,
) -> io::Result<[u8; 4]> {
    let characters = text.encode_utf16().count();
    let length = u16::try_from(characters)
        .ok()
        .filter(|&n| n <= MAX_FIELD_CHARS)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "this connection's {field} is {characters} characters, past the \
                     {MAX_FIELD_CHARS} MS-TDS gives a LOGIN7 field"
                ),
            )
        })?;

    let at = LOGIN7_HEADER
        + u16::try_from(blobs.len())
            .expect("six non-empty fields of at most 128 characters fit a u16 offset");

    blobs.extend(text.encode_utf16().flat_map(u16::to_le_bytes).map(|byte| {
        if obfuscate {
            // The nibble swap, which on one byte is a rotation by four.
            byte.rotate_left(4) ^ PASSWORD_XOR
        } else {
            byte
        }
    }));

    let mut pair = [0; 4];
    pair[..2].copy_from_slice(&at.to_le_bytes());
    pair[2..].copy_from_slice(&length.to_le_bytes());
    Ok(pair)
}

/// The LOGIN7 message this driver sends, as a payload for [`Wire::send`].
///
/// The whole of what a SQL Server connection is: who is logging in, with what,
/// into which database, and how the session behaves once it is open. It is
/// built from [`TdsTarget`] alone — nothing here reads the environment, so two
/// hosts running one `[db.<name>]` block send byte-identical logins.
///
/// **Everything an eavesdropper would want is in it**, and only the password
/// is disguised at all: the nibble swap and [`PASSWORD_XOR`] are public and
/// reversible, so this message is a credential in the clear unless
/// [`negotiate_tls`] has already run. That is the whole reason § 3's TLS is
/// mandatory rather than defaulted on this backend. The returned buffer holds
/// that credential and nothing zeroes it — deliberately, since the plaintext
/// it was built from lives in the configuration tree for the process's life,
/// and zeroing the copy while the source stays would be a gesture rather than
/// a defence.
///
/// `packet_size` is what the connection is currently framing at
/// ([`Codec::packet_size`]); the server may answer with a different one in an
/// `ENVCHANGE` token, which is what [`Codec::set_packet_size`] is for.
///
/// Five of the twelve fields go out empty and each is a decision. `HostName`
/// is the *client's* machine name, which nothing approved this driver to read
/// and which no server acts on; `Language` is empty so the server's own
/// default governs, and § 8 reads error numbers rather than message text;
/// `SSPI` belongs to integrated security, which [`OPTION_FLAGS_2`] turns off;
/// `AtchDBFile` attaches a database file by path, which is a capability
/// nothing in ADR 0067 grants; and `ChangePassword` changes the login's
/// password as a side effect of connecting. `ClientID` is a six-byte MAC
/// address and goes out as zeroes for `HostName`'s reason — it is a stable
/// identifier for the machine, sent to buy nothing.
///
/// # Errors
///
/// [`placed`]'s: `InvalidInput` for a field past [`MAX_FIELD_CHARS`].
pub fn login7_request(target: &TdsTarget<'_>, packet_size: u16) -> io::Result<Vec<u8>> {
    // In the fixed header's own order, because each pair's offset is where the
    // previous field's characters ended.
    let mut blobs = Vec::new();
    let host_name = placed(&mut blobs, "", "client host name", false)?;
    let user = placed(&mut blobs, target.user, "user", false)?;
    let password = placed(&mut blobs, target.password, "password", true)?;
    let app_name = placed(&mut blobs, CLIENT_NAME, "application name", false)?;
    let server_name = placed(&mut blobs, target.host, "host", false)?;
    let extension = placed(&mut blobs, "", "extension", false)?;
    let interface = placed(&mut blobs, CLIENT_NAME, "interface name", false)?;
    let language = placed(&mut blobs, "", "language", false)?;
    let database = placed(&mut blobs, target.database, "database", false)?;
    let sspi = placed(&mut blobs, "", "SSPI blob", false)?;
    let attached_file = placed(&mut blobs, "", "attached database file", false)?;
    let new_password = placed(&mut blobs, "", "new password", false)?;

    let total = usize::from(LOGIN7_HEADER) + blobs.len();
    let length =
        u32::try_from(total).expect("twelve fields of at most 128 characters fit a u32 length");

    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(&length.to_le_bytes());
    out.extend_from_slice(&TDS_VERSION.to_le_bytes());
    out.extend_from_slice(&u32::from(packet_size).to_le_bytes());
    // `ClientProgVer` and `ClientPID`. The version PRELOGIN already sent is the
    // one fact worth telling a server about this client, and a process id is
    // the host's business.
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    // `ConnectionID`, which only a connection being resumed carries.
    out.extend_from_slice(&0u32.to_le_bytes());
    out.push(OPTION_FLAGS_1);
    out.push(OPTION_FLAGS_2);
    out.push(TYPE_FLAGS);
    out.push(OPTION_FLAGS_3);
    // `ClientTimZone`. MS-TDS documents it as unused, and this is the one
    // driver with nowhere to send [`TdsTarget::time_zone`] anyway: § 9's zone
    // decodes rows here rather than configuring a session.
    out.extend_from_slice(&0i32.to_le_bytes());
    out.extend_from_slice(&CLIENT_LCID.to_le_bytes());

    for pair in [
        host_name,
        user,
        password,
        app_name,
        server_name,
        extension,
        interface,
        language,
        database,
    ] {
        out.extend_from_slice(&pair);
    }
    out.extend_from_slice(&[0; 6]);
    for pair in [sspi, attached_file, new_password] {
        out.extend_from_slice(&pair);
    }
    // `cbSSPILong`, the 32-bit length an SSPI blob past 65535 bytes would need.
    out.extend_from_slice(&0u32.to_le_bytes());

    debug_assert_eq!(
        out.len(),
        usize::from(LOGIN7_HEADER),
        "every offset above was counted against this width"
    );
    out.extend_from_slice(&blobs);
    Ok(out)
}

/// `ERROR`: the server refused something, and said which condition in a number
/// [`kind_of`] normalises.
const TOKEN_ERROR: u8 = 0xAA;
/// `INFO`: the same layout with nothing refused. A login collects several of
/// them for a change of database context alone, so a reader that did not know
/// the token would fail on an ordinary success.
const TOKEN_INFO: u8 = 0xAB;
/// `LOGINACK`: the login succeeded, and this is what the server is.
const TOKEN_LOGIN_ACK: u8 = 0xAD;
/// `ENVCHANGE`: one session property moved, and both values are given.
const TOKEN_ENV_CHANGE: u8 = 0xE3;
/// `COLMETADATA`: the shape of the rows that follow, one [`TdsColumn`] each.
const TOKEN_COL_METADATA: u8 = 0x81;
/// `ROW`: one row, every column's value in `COLMETADATA`'s order.
///
/// Read by [`TdsRows`] and refused by [`Tokens`], which is not a gap in the
/// second: a row is the one token whose extent cannot be known without the
/// column list, so a reader holding a message and nothing else has no way to
/// step over one.
const TOKEN_ROW: u8 = 0xD1;
/// `NBCROW`: a [`TOKEN_ROW`] whose null columns are a bitmap in front of the
/// values instead of a length each.
const TOKEN_NBC_ROW: u8 = 0xD2;
/// `RETURNSTATUS`: the integer a stored procedure returned, which only an RPC
/// can produce.
///
/// `sp_prepexec` answers `0` for a statement it prepared and executed, and a
/// non-zero value for one it refused — which the `ERROR` beside it has already
/// said, in a sentence [`kind_of`] can normalise. So this is read to step over
/// it rather than acted on.
const TOKEN_RETURN_STATUS: u8 = 0x79;
/// `RETURNVALUE`: one of a procedure's output parameters, coming back.
///
/// § 1's whole reason for calling `sp_prepexec` rather than `sp_executesql`:
/// the handle the server allocated for the statement arrives in one of these,
/// and the statement cache's key maps to it.
const TOKEN_RETURN_VALUE: u8 = 0xAC;
/// `DONE`: the end of one statement's answer.
const TOKEN_DONE: u8 = 0xFD;
/// `DONEPROC`: `DONE` for a stored procedure, which § 13's `sp_reset_connection`
/// and § 1's `sp_prepexec` both are.
const TOKEN_DONE_PROC: u8 = 0xFE;
/// `DONEINPROC`: `DONE` for one statement *inside* a procedure, and the one of
/// the three that is never the last token of a message.
const TOKEN_DONE_IN_PROC: u8 = 0xFF;

/// `DONE`'s `Status`: another set of results follows this one.
const DONE_MORE: u16 = 0x0001;
/// `DONE`'s `Status`: the statement this ends did not complete.
const DONE_ERROR: u16 = 0x0002;
/// `DONE`'s `Status`: the row count means something. Without it the field is a
/// number the server did not intend to report, which is not the same as zero.
const DONE_COUNT: u16 = 0x0010;

/// `ENVCHANGE` type 1: the database this session is in.
const ENV_DATABASE: u8 = 1;
/// `ENVCHANGE` type 2: the session's language.
const ENV_LANGUAGE: u8 = 2;
/// `ENVCHANGE` type 3: the session's character set, which TDS 7.4 does not use.
const ENV_CHARSET: u8 = 3;
/// `ENVCHANGE` type 4: the packet size, as decimal digits rather than a number.
const ENV_PACKET_SIZE: u8 = 4;

/// The severity at which SQL Server ends the connection rather than the
/// statement.
///
/// Documented as such by the server and not a driver convention: at 20 and
/// above the server closes the socket, so the read after one is an end of file
/// whatever this driver decides. [`kind_of`] answers
/// [`DbErrorKind::ConnectionLost`] for any unnamed number at this class for
/// that reason — the connection really is gone, and § 13's pool must destroy it
/// rather than reset it.
const FATAL_CLASS: u8 = 20;

/// The § 8 kind a SQL Server error number means.
///
/// **Keyed on the number alone**, where [`crate::pg`]'s table is keyed on the
/// `SQLSTATE` and [`crate::mysql`]'s reads one as a fallback: TDS has no
/// `SQLSTATE` field at all. The five characters PDO reports for this backend
/// are ODBC's invention, mapped from the number by the driver, so a table keyed
/// on them here would be keyed on a value this driver had to make up first.
///
/// Two rows of it are worth arguing:
///
/// - **`547` is both a foreign key and a `CHECK`**, and SQL Server merges them
///   into one number on purpose — only the message text separates them, and
///   matching on message text is the thing § 8 exists to stop. It normalises as
///   [`DbErrorKind::ForeignKeyViolation`], the far commoner reading, and
///   § 8's `driverCode` still carries the number for a caller that needs the
///   difference. [`DbErrorKind::CheckViolation`] is therefore **unreachable on
///   this backend**, which is exactly what § 8 means when it says some
///   boundaries are driver-dependent.
/// - **`4060` is a `Permission`**, not a `Syntax` the way MySQL's "unknown
///   database" is. SQL Server deliberately answers a database that does not
///   exist and one this login may not open with the same refusal, so the only
///   half that is always true of it is that the login could not open it. It is
///   also the refusal [`OPT1_INIT_DB_FATAL`] asks for: with the other reading
///   of that bit this arrives as a warning and the connection opens somewhere
///   else.
///
/// Everything unnamed is [`DbErrorKind::Other`] rather than a guess, as on the
/// other three drivers, except that a class of [`FATAL_CLASS`] or more is
/// [`DbErrorKind::ConnectionLost`] whatever the number: at that severity the
/// server has already closed the socket.
pub(crate) fn kind_of(number: u32, class: u8) -> DbErrorKind {
    match number {
        // A unique index and a primary key, which are two numbers for one
        // condition.
        2601 | 2627 => DbErrorKind::UniqueViolation,
        547 => DbErrorKind::ForeignKeyViolation,
        515 => DbErrorKind::NotNullViolation,
        // The deadlock victim: SQL Server rolled this transaction back whole,
        // so § 7's closure re-runs from nothing.
        1205 => DbErrorKind::Deadlock,
        // Snapshot isolation could not serialise this transaction against a
        // concurrent one, and aborted it.
        3960 | 3961 => DbErrorKind::SerializationFailure,
        // A lock wait that ran out, which is `Timeout` and **not** `Deadlock`
        // for `crate::mysql`'s reason: nothing was rolled back, so re-running
        // the closure would run its earlier statements again inside a
        // transaction that is still open.
        1222 => DbErrorKind::Timeout,
        // A malformed statement, and the "no such thing" numbers § 8 makes one
        // kind with it: an undefined object and a syntax error are the same bug
        // to a caller.
        102 | 156 | 207 | 208 | 2812 | 4104 => DbErrorKind::Syntax,
        // Denied: to the server, to a database, to an object, to a column, or
        // at the login itself.
        229 | 230 | 262 | 300 | 916 | 4060 | 18456 => DbErrorKind::Permission,
        _ if class >= FATAL_CLASS => DbErrorKind::ConnectionLost,
        _ => DbErrorKind::Other,
    }
}

/// An `ERROR` or `INFO` token: one thing the server has to say.
///
/// The same seven fields carry both, which is the protocol's doing and not a
/// convenience taken here — [`Token::Error`] and [`Token::Info`] are what say
/// whether anything was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerMessage {
    /// The server's own error number, which is § 8's `driverCode` and the key
    /// [`kind_of`] reads.
    pub number: u32,
    /// Which of the places that raise this number raised it. Not normalised by
    /// anything: it is what a support article asks for.
    pub state: u8,
    /// The severity. Above 10 is a refusal, and [`FATAL_CLASS`] or more is one
    /// that took the connection with it.
    pub class: u8,
    /// The server's own sentence, in the language its login default chose.
    pub message: String,
    /// Which server said it, as that server knows its own name.
    pub server: String,
    /// The procedure it was raised in, empty for a statement sent directly.
    pub procedure: String,
    /// The line within that procedure or batch.
    pub line: u32,
}

impl ServerMessage {
    /// Whether this ended the connection as well as the statement.
    #[must_use]
    pub const fn is_fatal(&self) -> bool {
        self.class >= FATAL_CLASS
    }

    /// § 8's kind for this message, from [`kind_of`].
    #[must_use]
    pub fn kind(&self) -> DbErrorKind {
        kind_of(self.number, self.class)
    }
}

/// A `LOGINACK` token: the login succeeded, and this is what answered it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginAck {
    /// The TDS version the server will speak, which is the one thing here a
    /// driver could act on. It should be the [`TDS_VERSION`] LOGIN7 asked for;
    /// a server entitled to answer with a lower one is a server this driver
    /// would have to have a second dialect for.
    pub tds_version: u32,
    /// What the server calls its own program — `Microsoft SQL Server`.
    pub program: String,
    /// Its major, minor and build numbers, which is the version an operator
    /// reads.
    pub version: (u8, u8, u16),
}

/// An `ENVCHANGE` token: one session property, before and after.
///
/// The variants are the four TDS 7.4 spells as text. Everything else —
/// collation, the transaction descriptors, the routing answer an Azure failover
/// sends — is [`EnvChange::Other`] carrying its type byte, because the value
/// halves of those are bytes rather than characters and nothing here has a use
/// for them yet. They are skipped by the token's own declared length, so an
/// unread type never costs the reader its place in the stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvChange {
    /// The database this session is in. Novis reads it because § 13's pool key
    /// is a promise about which database a pooled connection is in, and a `USE`
    /// is the one thing that could break it.
    Database {
        /// What it was.
        from: String,
        /// What it now is.
        to: String,
    },
    /// The session's language.
    Language {
        /// What it was.
        from: String,
        /// What it now is.
        to: String,
    },
    /// The session's character set, which a TDS 7.4 server does not send.
    Charset {
        /// What it was.
        from: String,
        /// What it now is.
        to: String,
    },
    /// The packet size, which is the answer to what LOGIN7 asked for and is
    /// what [`Codec::set_packet_size`] takes.
    PacketSize {
        /// The size in force from the *next* packet onwards.
        to: u16,
    },
    /// A type this driver does not read, by its type byte.
    Other(u8),
}

/// One token of a response stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// The server refused something: [`ServerMessage::kind`] is § 8's kind.
    Error(ServerMessage),
    /// The server said something and refused nothing.
    Info(ServerMessage),
    /// The login succeeded.
    LoginAck(LoginAck),
    /// A session property moved.
    Env(EnvChange),
    /// The shape of the rows that follow, in the order the server sends their
    /// values.
    ///
    /// Empty where the server answered `0xFFFF` — *no metadata*, which is what
    /// a statement with no result set at all sends. A reader has nothing to
    /// distinguish that from a result set of no columns, which no server sends,
    /// so the two are one case here rather than a variant that is never taken.
    Columns(Vec<TdsColumn>),
    /// A procedure returned, and this is what it returned —
    /// [`TOKEN_RETURN_STATUS`] owns why nothing acts on the number.
    ReturnStatus(i32),
    /// One of a procedure's output parameters came back.
    ReturnValue(ReturnValue),
    /// A statement's answer ended.
    Done(Done),
}

/// A `RETURNVALUE` token: one output parameter of the procedure that was
/// called, named as the procedure declares it.
///
/// The value is owned rather than borrowed, unlike every other token's bytes,
/// because [`TdsRows`] parses its tokens out of a buffer it then refills over —
/// and this is the one token whose *payload* a caller keeps past the parse. It
/// is at most a handle: § 1 calls one procedure and asks for one integer back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnValue {
    /// Which parameter of the call this is, one-based, as the server counted
    /// it. Read for a caller that asked for more than one output parameter;
    /// § 1 asks for a single one and reads [`ReturnValue::as_i32`] instead.
    pub ordinal: u16,
    /// The parameter's name as the procedure declares it — `@handle` for
    /// `sp_prepexec`'s. A caller that sent the parameter unnamed still gets
    /// this, since it is the *procedure's* spelling and not the request's.
    pub name: String,
    /// What the server said the value's type is.
    pub type_info: TypeInfo,
    /// The value's bytes, or `None` for SQL `NULL` — which is what an output
    /// parameter a procedure never assigned comes back as.
    pub value: Option<Vec<u8>>,
}

impl ReturnValue {
    /// The value as a four-byte little-endian integer, or `None` where it is
    /// null or is not four bytes wide.
    ///
    /// The one shape § 1 reads: `sp_prepexec` hands its handle back as an
    /// `INTN` of four bytes. Narrower is not silently widened — a server that
    /// answered a two-byte handle is one this driver has no account of, and
    /// reading it as a number would be inventing the two bytes it did not send.
    #[must_use]
    pub fn as_i32(&self) -> Option<i32> {
        let bytes: [u8; 4] = self.value.as_deref()?.try_into().ok()?;
        Some(i32::from_le_bytes(bytes))
    }
}

/// A `DONE`, `DONEPROC` or `DONEINPROC` token: one statement's answer ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Done {
    /// The status bits, read through the three methods below rather than
    /// matched on directly.
    pub status: u16,
    /// Which command this ends, which no caller here reads: the token stream is
    /// already in order.
    pub command: u16,
    /// Whether this was a `DONEINPROC` — one statement *inside* a procedure
    /// ending, rather than the answer.
    ///
    /// Not a status bit, which is why it is a field beside them rather than a
    /// method over `status`: the three ends are three token *bytes*, and only
    /// this one is guaranteed to have something behind it. A reader that
    /// stopped on it would end an `sp_prepexec` answer before the
    /// `RETURNVALUE` carrying § 1's handle, which is sent after the rows.
    pub in_proc: bool,
    /// The row count, meaningful only where [`Done::counted`] says so.
    pub rows: u64,
}

impl Done {
    /// Whether another statement's answer follows this one in the same stream.
    #[must_use]
    pub const fn more(self) -> bool {
        self.status & DONE_MORE != 0
    }

    /// Whether the statement this ends failed.
    ///
    /// The bit is the *only* place a failure is reported for a statement whose
    /// `ERROR` token a caller chose not to keep — a reader that watched for
    /// `ERROR` alone and then trusted the `DONE` would report a rolled-back
    /// batch as a success.
    #[must_use]
    pub const fn failed(self) -> bool {
        self.status & DONE_ERROR != 0
    }

    /// Whether [`Done::rows`] is a count the server meant to report.
    ///
    /// Without the bit the field is whatever the server left there, which is
    /// not the same as zero: `execute` answering 0 for a statement that
    /// affected rows and never counted them would be a wrong number rather than
    /// a missing one.
    #[must_use]
    pub const fn counted(self) -> bool {
        self.status & DONE_COUNT != 0
    }
}

/// `COLMETADATA`'s count, and `TYPE_INFO`'s two-byte length, when what is meant
/// is *there is none*: no result set at all in the first, `MAX` — a value that
/// arrives in [`Length::Partial`]'s chunks — in the second.
const NO_LENGTH: u16 = 0xFFFF;

/// A [`Length::Long`] value's length, when what is meant is `NULL`.
const NO_LENGTH_LONG: u32 = 0xFFFF_FFFF;

/// The eight bytes of row timestamp a `text`, `ntext` or `image` value carries
/// between its text pointer and its length. Read to step over: it says when the
/// row was last written, which nothing here asks.
const TEXT_TIMESTAMP: usize = 8;

/// A `PLP` value's declared total, when what is meant is `NULL`.
const PLP_NULL: u64 = 0xFFFF_FFFF_FFFF_FFFF;

/// A `PLP` value's declared total, when the server does not know it yet: the
/// chunks say how long it was, and the terminator is the only end.
const PLP_UNKNOWN: u64 = 0xFFFF_FFFF_FFFF_FFFE;

/// How much of a [`PLP_UNKNOWN`]-free value is reserved up front.
///
/// The declared total is the server's word about bytes that are not on the wire
/// yet, the same position [`Tokens::columns`]' own cap is in: reserving two
/// gigabytes because a `varbinary(max)` said so is a per-row allocation an
/// answer of one packet can ask for.
const PLP_RESERVE: usize = 64 * 1024;

/// `COLMETADATA`'s `Flags`: the column may be null. The other fifteen bits are
/// what the server would say about updatability, identity and sparseness, and
/// nothing in this driver asks.
const COLUMN_NULLABLE: u16 = 0x0001;

/// `NULLTYPE`: a column with no type, which is what `SELECT NULL` has.
const TY_NULL: u8 = 0x1F;
/// `INT1TYPE`: `tinyint`, and the one integer SQL Server does not sign.
const TY_INT1: u8 = 0x30;
/// `BITTYPE`: `bit`.
const TY_BIT: u8 = 0x32;
/// `INT2TYPE`: `smallint`.
const TY_INT2: u8 = 0x34;
/// `INT4TYPE`: `int`.
const TY_INT4: u8 = 0x38;
/// `DATETIM4TYPE`: `smalldatetime`.
const TY_DATETIME4: u8 = 0x3A;
/// `FLT4TYPE`: `real`.
const TY_FLT4: u8 = 0x3B;
/// `MONEYTYPE`: `money`, four decimal places in a scaled 64-bit integer.
const TY_MONEY: u8 = 0x3C;
/// `DATETIMETYPE`: `datetime`.
const TY_DATETIME: u8 = 0x3D;
/// `FLT8TYPE`: `float`.
const TY_FLT8: u8 = 0x3E;
/// `MONEY4TYPE`: `smallmoney`.
const TY_MONEY4: u8 = 0x7A;
/// `INT8TYPE`: `bigint`.
const TY_INT8: u8 = 0x7F;
/// `GUIDTYPE`: `uniqueidentifier`.
const TY_GUID: u8 = 0x24;
/// `INTNTYPE`: any of the four integers where the column is nullable, its width
/// in the declared length rather than in the type byte.
const TY_INTN: u8 = 0x26;
/// `BITNTYPE`: a nullable `bit`.
const TY_BITN: u8 = 0x68;
/// `DECIMALNTYPE`: `decimal`, carrying its own precision and scale.
const TY_DECIMALN: u8 = 0x6A;
/// `NUMERICNTYPE`: `numeric`, which SQL Server stores identically.
const TY_NUMERICN: u8 = 0x6C;
/// `FLTNTYPE`: a nullable `real` or `float`.
const TY_FLTN: u8 = 0x6D;
/// `MONEYNTYPE`: a nullable `money` or `smallmoney`.
const TY_MONEYN: u8 = 0x6E;
/// `DATETIMNTYPE`: a nullable `datetime` or `smalldatetime`.
const TY_DATETIMEN: u8 = 0x6F;
/// `DATENTYPE`: `date`, whose `TYPE_INFO` carries nothing at all — three bytes
/// is the only width it has.
const TY_DATEN: u8 = 0x28;
/// `TIMENTYPE`: `time`, whose `TYPE_INFO` carries a scale and no length.
const TY_TIMEN: u8 = 0x29;
/// `DATETIME2NTYPE`: `datetime2`, a `TY_TIMEN` with three bytes of date after
/// it.
const TY_DATETIME2N: u8 = 0x2A;
/// `DATETIMEOFFSETNTYPE`: `datetimeoffset`, a `TY_DATETIME2N` with two bytes of
/// offset after it — the one SQL Server type that carries its own zone.
const TY_DATETIMEOFFSETN: u8 = 0x2B;
/// `BIGVARBINTYPE`: `varbinary(n)`, or `varbinary(max)` at [`NO_LENGTH`].
const TY_BIGVARBINARY: u8 = 0xA5;
/// `BIGVARCHRTYPE`: `varchar(n)`, or `varchar(max)` at [`NO_LENGTH`].
const TY_BIGVARCHAR: u8 = 0xA7;
/// `BIGBINARYTYPE`: `binary(n)`.
const TY_BIGBINARY: u8 = 0xAD;
/// `BIGCHARTYPE`: `char(n)`.
const TY_BIGCHAR: u8 = 0xAF;
/// `NVARCHARTYPE`: `nvarchar(n)`, or `nvarchar(max)` at [`NO_LENGTH`].
const TY_NVARCHAR: u8 = 0xE7;
/// `NCHARTYPE`: `nchar(n)`.
const TY_NCHAR: u8 = 0xEF;
/// `IMAGETYPE`: `image`, deprecated since 2005 and still on disk everywhere.
const TY_IMAGE: u8 = 0x22;
/// `TEXTTYPE`: `text`, likewise.
const TY_TEXT: u8 = 0x23;
/// `NTEXTTYPE`: `ntext`, likewise.
const TY_NTEXT: u8 = 0x63;
/// `SSVARIANTTYPE`: `sql_variant`, a value carrying its own type description in
/// front of itself.
const TY_VARIANT: u8 = 0x62;
/// `UDTTYPE`: a CLR type, which is what `geometry`, `geography` and
/// `hierarchyid` arrive as.
const TY_UDT: u8 = 0xF0;
/// `XMLTYPE`: `xml`, optionally naming the schema collection it is bound to.
const TY_XML: u8 = 0xF1;

/// The three bytes a `date` occupies, and the date half of a `datetime2`.
const DATE_BYTES: u8 = 3;
/// The two bytes of minute offset a `datetimeoffset` carries after its
/// `datetime2` half.
const OFFSET_BYTES: u8 = 2;

/// How one column's values carry their own length, which is the whole of what
/// a row reader needs from a type it does not otherwise understand.
///
/// MS-TDS states this as four families — `FIXEDLENTYPE`, `BYTELEN`, `USHORTLEN`
/// and `LONGLEN` — plus `PARTLEN` for the `MAX` types, and the families are not
/// derivable from the type byte in any shorter way than the table
/// [`Tokens::type_info`] writes out. Every variant but [`Length::Fixed`] has a
/// null form, which is why a fixed-width nullable column arrives as its `…N`
/// type instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Length {
    /// Exactly this many bytes, with no length in front of them and no null
    /// form: the type byte is the width.
    Fixed(usize),
    /// One byte of length, at most this many, and `0` is `NULL`.
    Byte(u8),
    /// Two bytes of length, at most this many, and [`NO_LENGTH`] is `NULL`.
    Short(u16),
    /// Four bytes of length, at most this many, and [`NO_LENGTH_LONG`] is
    /// `NULL`.
    ///
    /// `text`, `ntext` and `image` put a text pointer and a timestamp in front
    /// of that length and are `NULL` where the pointer is empty; `sql_variant`
    /// does not — it is `NULL` at a length of **zero** as well, since a
    /// `sql_variant` carries its own type description and a value of no bytes
    /// at all is not one. The row reader tells them apart by [`TypeInfo::id`],
    /// because nothing else about the two is different.
    Long(u32),
    /// `PLP`: an eight-byte total length or an unknown-length sentinel, then
    /// chunks until an empty one. Every `MAX` type, `xml` and every CLR type.
    Partial,
}

/// One column's `TYPE_INFO`: what the server said its type is, in the server's
/// own vocabulary.
///
/// Deliberately not a Novis type — [`TdsColumn::column_type`] is the only thing
/// here that has an opinion about that, and [ADR 0067
/// § 9](../../../docs/adr/0067-core-db.md)'s decode into a value belongs to
/// `nvs-stdlib`, which is the crate that can allocate a `Core\Time\DateTime`.
/// The fields are all four things a `TYPE_INFO` can carry, and a type that
/// carries none of them leaves them at their zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeInfo {
    /// The type byte, which is the identity of the type and the only thing
    /// [`TdsColumn::column_type`] reads.
    pub id: u8,
    /// How a value of this type is measured on the wire.
    pub length: Length,
    /// A `decimal` or `numeric` column's declared precision, and `0` for every
    /// other type.
    pub precision: u8,
    /// A `decimal`/`numeric` column's declared scale, or the fractional-second
    /// digits of a `time`, `datetime2` or `datetimeoffset` — the same field
    /// because the wire spells them with one byte in the same place.
    pub scale: u8,
    /// The five raw bytes of the column's `COLLATION`, for the six character
    /// types that carry one: an LCID and flags in four little-endian bytes,
    /// then a sort id.
    ///
    /// Kept raw and unparsed because the one thing a reader will ever want from
    /// it is the code page of a non-Unicode column, and that question belongs
    /// to the row path rather than here.
    pub collation: Option<[u8; 5]>,
}

/// One column of a result set, as `COLMETADATA` described it.
///
/// The shape [`crate::PgColumn`] has, for the reason its doc gives: a driver's
/// column carries the *server's* type description and never a Novis type. What
/// differs is that TDS describes a type structurally rather than by a catalog
/// id, so the description is [`TypeInfo`] and not an integer this driver would
/// have to hold a table for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TdsColumn {
    /// The column's label, as the server wrote it.
    pub name: String,
    /// The user-defined type id, `0` for every built-in type. This driver reads
    /// it only to skip it: a UDT's own name is in [`TypeInfo`].
    pub user_type: u32,
    /// The `Flags` field, read through [`TdsColumn::nullable`] rather than
    /// matched on.
    pub flags: u16,
    /// What the server said the column's type is.
    pub type_info: TypeInfo,
}

impl TdsColumn {
    /// Whether the server said this column may be null.
    ///
    /// Not what makes a value optional to a program: every column reads back as
    /// `?T` because a value can be `NULL` whatever the schema says, and a
    /// server computes this bit for an expression rather than reading it off a
    /// table.
    #[must_use]
    pub const fn nullable(&self) -> bool {
        self.flags & COLUMN_NULLABLE != 0
    }

    /// The column's declared type, as spec § 18's `ColumnType` names it — what
    /// `Core\Db\Rows::columns` answers for this column.
    ///
    /// It reads the type byte and nothing else, which is the whole difference
    /// from [`crate::PgColumn::column_type`]: PostgreSQL has one OID for `bit`
    /// and `bit varying` and needs the modifier to find [ADR 0067
    /// § 9](../../../docs/adr/0067-core-db.md)'s `BIT(1)` row, while SQL
    /// Server's `bit` *is* one bit — `BIT(n>1)` is not a type it has — so the
    /// byte is the answer and the width is never consulted.
    ///
    /// Two more of § 9's rows fall out of the same table. `uniqueidentifier` is
    /// [`ColumnType::Uuid`] rather than [`ColumnType::Bytes`], unlike MySQL's
    /// `BINARY(16)`, because it is a type of its own here. And nothing answers
    /// [`ColumnType::Json`]: SQL Server through 2022 stores JSON in an
    /// `nvarchar` with a `CHECK` constraint, so a JSON column *is* a text
    /// column and § 9's rule that JSON is never auto-decoded is what makes that
    /// the honest answer rather than a lost one.
    ///
    /// [`ColumnType::Uint`] is likewise unreachable: `tinyint` is the only
    /// unsigned integer SQL Server has and it fits an `int` with room to spare,
    /// so § 9's `uint` row is MySQL's and PostgreSQL's alone.
    #[must_use]
    pub const fn column_type(&self) -> ColumnType {
        match self.type_info.id {
            TY_INT1 | TY_INT2 | TY_INT4 | TY_INT8 | TY_INTN => ColumnType::Int,
            TY_BIT | TY_BITN => ColumnType::Bool,
            TY_FLT4 | TY_FLT8 | TY_FLTN => ColumnType::Float,
            TY_MONEY | TY_MONEY4 | TY_MONEYN | TY_DECIMALN | TY_NUMERICN => ColumnType::Decimal,
            TY_BIGCHAR | TY_BIGVARCHAR | TY_NCHAR | TY_NVARCHAR | TY_TEXT | TY_NTEXT => {
                ColumnType::Text
            }
            TY_BIGBINARY | TY_BIGVARBINARY | TY_IMAGE => ColumnType::Bytes,
            TY_GUID => ColumnType::Uuid,
            TY_DATEN => ColumnType::Date,
            TY_TIMEN => ColumnType::Time,
            TY_DATETIME | TY_DATETIME4 | TY_DATETIMEN | TY_DATETIME2N => ColumnType::DateTime,
            TY_DATETIMEOFFSETN => ColumnType::Instant,
            // `sql_variant`, `xml`, every CLR type — `geometry` among them —
            // and the column that has no type at all: § 9's last row, which
            // reads as a `tainted string`.
            _ => ColumnType::Other,
        }
    }
}

/// A reader over one message's tokens.
///
/// Sans-IO like the rest of this module: it borrows a payload and never touches
/// a stream, so every shape below is tested without a socket. The payload it is
/// given is one whole message — [`Wire::read_message`] — because a token is cut
/// at whatever offset the packet size lands on and this reader holds no
/// remainder of its own. The row path is the one that cannot afford that, and
/// it is [`TdsRows`], which holds the remainder itself and hands *this* reader
/// the tokens between the rows — see [`Tokens::consumed`].
///
/// Not an `Iterator`: every step can fail, and an `Iterator<Item = Result<_>>`
/// would let a `for` loop walk past a malformed token by ignoring the item it
/// was handed.
#[derive(Debug)]
pub struct Tokens<'a> {
    payload: &'a [u8],
    at: usize,
}

impl<'a> Tokens<'a> {
    /// A reader over one message's payload.
    #[must_use]
    pub const fn over(payload: &'a [u8]) -> Tokens<'a> {
        Tokens { payload, at: 0 }
    }

    /// The next token, or `None` where the message has ended.
    ///
    /// # Errors
    ///
    /// `InvalidData` for a token this driver does not read yet — naming the
    /// byte, since that is the whole of what a reader can say about it — and
    /// for any token whose fields run past the end of the message.
    pub fn next_token(&mut self) -> io::Result<Option<Token>> {
        let Some(&kind) = self.payload.get(self.at) else {
            return Ok(None);
        };
        self.at += 1;

        let token = match kind {
            TOKEN_ERROR => Token::Error(self.server_message("ERROR")?),
            TOKEN_INFO => Token::Info(self.server_message("INFO")?),
            TOKEN_LOGIN_ACK => Token::LoginAck(self.login_ack()?),
            TOKEN_ENV_CHANGE => Token::Env(self.env_change()?),
            TOKEN_COL_METADATA => Token::Columns(self.columns()?),
            TOKEN_RETURN_STATUS => Token::ReturnStatus(self.signed("RETURNSTATUS")?),
            TOKEN_RETURN_VALUE => Token::ReturnValue(self.return_value()?),
            TOKEN_DONE | TOKEN_DONE_PROC | TOKEN_DONE_IN_PROC => {
                Token::Done(self.done(kind == TOKEN_DONE_IN_PROC)?)
            }
            other => {
                return Err(malformed(format!(
                    "a TDS response carried token 0x{other:02X}, which this driver does not read"
                )));
            }
        };
        Ok(Some(token))
    }

    /// How far the payload has been read, which is how many bytes the tokens
    /// answered so far occupied.
    ///
    /// [`TdsRows`] is the caller: it parses a token out of a buffer it filled
    /// from packets and needs to know what to drop off the front, and the token
    /// itself says nothing about its own extent — three of the six carry a
    /// length, `DONE` is fixed, and `COLMETADATA` is neither.
    #[must_use]
    pub const fn consumed(&self) -> usize {
        self.at
    }

    /// `n` bytes, or the refusal that says which field ran off the end.
    fn take(&mut self, n: usize, what: &'static str) -> io::Result<&'a [u8]> {
        let Some(bytes) = self.payload.get(self.at..self.at + n) else {
            return Err(malformed(format!(
                "a TDS token's {what} wanted {n} byte(s) at offset {}, past the end of a {}-byte \
                 message",
                self.at,
                self.payload.len()
            )));
        };
        self.at += n;
        Ok(bytes)
    }

    /// One byte.
    fn byte(&mut self, what: &'static str) -> io::Result<u8> {
        Ok(self.take(1, what)?[0])
    }

    /// A little-endian `u16`.
    fn short(&mut self, what: &'static str) -> io::Result<u16> {
        let bytes = self.take(2, what)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    /// A little-endian `u32`.
    fn long(&mut self, what: &'static str) -> io::Result<u32> {
        let bytes = self.take(4, what)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// A little-endian `i32`.
    ///
    /// `RETURNSTATUS` is the one signed field this driver reads: a procedure
    /// returns whatever integer it likes, and the convention is that a negative
    /// one is a failure.
    fn signed(&mut self, what: &'static str) -> io::Result<i32> {
        let bytes = self.take(4, what)?;
        Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// `characters` UCS-2LE characters, as a `String`.
    ///
    /// Lossy on an unpaired surrogate, which is the one thing a UCS-2 field can
    /// hold that Novis's own strings cannot ([ADR 0009](../../../docs/adr/0009-string-and-bytes.md)
    /// makes a `string` valid UTF-8). Refusing a message because the server's
    /// *prose* was ill-formed would turn a reportable error into an
    /// unreportable one.
    fn characters(&mut self, characters: usize, what: &'static str) -> io::Result<String> {
        let bytes = self.take(characters * 2, what)?;
        let units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|two| u16::from_le_bytes([two[0], two[1]]))
            .collect();
        Ok(String::from_utf16_lossy(&units))
    }

    /// A `B_VARCHAR`: a one-byte character count, then the characters.
    fn b_varchar(&mut self, what: &'static str) -> io::Result<String> {
        let characters = usize::from(self.byte(what)?);
        self.characters(characters, what)
    }

    /// A `US_VARCHAR`: a two-byte character count, then the characters.
    fn us_varchar(&mut self, what: &'static str) -> io::Result<String> {
        let characters = usize::from(self.short(what)?);
        self.characters(characters, what)
    }

    /// A length-prefixed token's end, from the `u16` length that opens it.
    ///
    /// Every variable-length token below is parsed field by field and then
    /// *positioned* by this, rather than trusted to have consumed exactly the
    /// right number of bytes. That is what makes a field MS-TDS adds to the end
    /// of a token in a later version a thing this driver skips rather than
    /// reads as the next token's type byte.
    fn bounded(&mut self, what: &'static str) -> io::Result<usize> {
        let length = usize::from(self.short(what)?);
        let end = self.at + length;
        if end > self.payload.len() {
            return Err(malformed(format!(
                "a TDS {what} token claims {length} byte(s) at offset {}, past the end of a \
                 {}-byte message",
                self.at,
                self.payload.len()
            )));
        }
        Ok(end)
    }

    /// An `ERROR` or `INFO` token, which have one layout.
    fn server_message(&mut self, what: &'static str) -> io::Result<ServerMessage> {
        let end = self.bounded(what)?;
        let message = ServerMessage {
            number: self.long(what)?,
            state: self.byte(what)?,
            class: self.byte(what)?,
            message: self.us_varchar(what)?,
            server: self.b_varchar(what)?,
            procedure: self.b_varchar(what)?,
            line: self.long(what)?,
        };
        self.finish(end, what)?;
        Ok(message)
    }

    /// A `LOGINACK` token.
    fn login_ack(&mut self) -> io::Result<LoginAck> {
        let end = self.bounded("LOGINACK")?;
        // `Interface`, which says which of the two SQL dialects the server will
        // accept and is the same answer for every server this driver can talk
        // to at all.
        self.byte("LOGINACK")?;
        let ack = LoginAck {
            tds_version: self.long("LOGINACK")?,
            program: self.b_varchar("LOGINACK")?,
            version: (
                self.byte("LOGINACK")?,
                self.byte("LOGINACK")?,
                // The build number is two bytes written high half first, which
                // is the one number in this token that is not little-endian.
                u16::from_be_bytes([self.byte("LOGINACK")?, self.byte("LOGINACK")?]),
            ),
        };
        self.finish(end, "LOGINACK")?;
        Ok(ack)
    }

    /// An `ENVCHANGE` token: the type, then a new value and an old one whose
    /// shape depends on it.
    fn env_change(&mut self) -> io::Result<EnvChange> {
        let end = self.bounded("ENVCHANGE")?;
        let kind = self.byte("ENVCHANGE")?;
        let change = match kind {
            ENV_DATABASE | ENV_LANGUAGE | ENV_CHARSET | ENV_PACKET_SIZE => {
                let to = self.b_varchar("ENVCHANGE")?;
                let from = self.b_varchar("ENVCHANGE")?;
                match kind {
                    ENV_DATABASE => EnvChange::Database { from, to },
                    ENV_LANGUAGE => EnvChange::Language { from, to },
                    ENV_CHARSET => EnvChange::Charset { from, to },
                    _ => EnvChange::PacketSize {
                        to: to.parse().map_err(|_| {
                            malformed(format!(
                                "a TDS ENVCHANGE answered the packet size with {to:?}, which is \
                                 not a size"
                            ))
                        })?,
                    },
                }
            }
            other => EnvChange::Other(other),
        };
        // Deliberately unconditional: an unread type is skipped by the token's
        // own length rather than by walking value halves this driver has no
        // parser for.
        self.at = end;
        Ok(change)
    }

    /// A `DONE`, `DONEPROC` or `DONEINPROC` token: twelve fixed bytes with no
    /// length in front of them.
    fn done(&mut self, in_proc: bool) -> io::Result<Done> {
        let status = self.short("DONE")?;
        let command = self.short("DONE")?;
        let rows = self.take(8, "DONE")?;
        Ok(Done {
            status,
            command,
            in_proc,
            rows: u64::from_le_bytes(rows.try_into().expect("eight bytes")),
        })
    }

    /// A `COLMETADATA` token: a column count, then that many descriptions in
    /// the order the values will arrive.
    ///
    /// The one token here that carries neither a length nor a fixed size, so a
    /// field that runs off the end is caught by [`Tokens::take`] rather than by
    /// [`Tokens::finish`]. That costs nothing here and is what lets the row
    /// path reuse this reader unchanged over a buffer it filled from packets:
    /// this parses bytes, and where they came from is not its question.
    ///
    /// The `CekTable` a column-encryption login would put between the count and
    /// the first column is deliberately not read. LOGIN7 does not ask for that
    /// feature ([`login7_request`]), so the field is never sent, and a parser for a
    /// shape this driver cannot receive is untestable by construction.
    fn columns(&mut self) -> io::Result<Vec<TdsColumn>> {
        let count = self.short("COLMETADATA")?;
        if count == NO_LENGTH {
            return Ok(Vec::new());
        }
        // Capped, because the count is the server's word and the columns are
        // not on the wire yet: SQL Server's own ceiling on a `SELECT` is 4,096,
        // and a claim past it grows the vector instead of preallocating for it.
        let mut columns = Vec::with_capacity(usize::from(count).min(4096));
        for _ in 0..count {
            let user_type = self.long("COLMETADATA")?;
            let flags = self.short("COLMETADATA")?;
            let type_info = self.type_info()?;
            if matches!(type_info.id, TY_TEXT | TY_NTEXT | TY_IMAGE) {
                // The `TableName` only those three carry: a part count, then
                // that many `US_VARCHAR`s naming the table the value is stored
                // in. Read to skip it — a text pointer is what fetches the
                // value — and field by field, since it is variable.
                let parts = self.byte("COLMETADATA")?;
                for _ in 0..parts {
                    self.us_varchar("COLMETADATA")?;
                }
            }
            columns.push(TdsColumn {
                name: self.b_varchar("COLMETADATA")?,
                user_type,
                flags,
                type_info,
            });
        }
        Ok(columns)
    }

    /// A `RETURNVALUE`: which output parameter it is, what type the server gave
    /// it, and its bytes.
    ///
    /// The `Flags` field is read and dropped. It describes the *column* an
    /// output parameter would have if it were one — nullable, updateable — and
    /// a value that came back has already answered the only question here,
    /// which is whether it is null.
    fn return_value(&mut self) -> io::Result<ReturnValue> {
        const WHAT: &str = "RETURNVALUE";

        let ordinal = self.short(WHAT)?;
        let name = self.b_varchar(WHAT)?;
        // `Status`: 0x01 for an RPC's output parameter and 0x02 for a
        // user-defined function's return value. Nothing here calls a UDF, and a
        // caller that did would read the same bytes either way.
        self.byte(WHAT)?;
        self.long(WHAT)?;
        self.short(WHAT)?;
        let type_info = self.type_info()?;
        let value = self.value(type_info)?;
        Ok(ReturnValue {
            ordinal,
            name,
            type_info,
            value,
        })
    }

    /// One value, measured by its own `TYPE_INFO`, or `None` for SQL `NULL`.
    ///
    /// [`TdsRows::value`]'s table over a payload that is all there, rather than
    /// over a buffer that refills — which is the whole of the difference, and
    /// the reason the two are not one function. That one copies into a row's
    /// bytes as chunks arrive so a `PLP` value never sits in the buffer twice;
    /// this one is reading a message [`Wire::read_message`] has already
    /// assembled, so there is no second copy to avoid. It is also the narrower
    /// job: a `RETURNVALUE` is the only token that reaches here, and § 1 asks
    /// for a four-byte handle.
    fn value(&mut self, info: TypeInfo) -> io::Result<Option<Vec<u8>>> {
        const WHAT: &str = "value";

        if info.id == TY_NULL {
            return Ok(None);
        }
        let length = match info.length {
            Length::Fixed(width) => width,
            Length::Byte(_) => match self.byte(WHAT)? {
                0 => return Ok(None),
                length => usize::from(length),
            },
            Length::Short(_) => match self.short(WHAT)? {
                NO_LENGTH => return Ok(None),
                length => usize::from(length),
            },
            Length::Long(_) => {
                if matches!(info.id, TY_TEXT | TY_NTEXT | TY_IMAGE) {
                    let pointer = self.byte(WHAT)?;
                    if pointer == 0 {
                        return Ok(None);
                    }
                    self.take(usize::from(pointer) + TEXT_TIMESTAMP, WHAT)?;
                }
                let declared = self.long(WHAT)?;
                if declared == NO_LENGTH_LONG || (declared == 0 && info.id == TY_VARIANT) {
                    return Ok(None);
                }
                as_usize(declared)?
            }
            Length::Partial => return self.partial(),
        };
        Ok(Some(self.take(length, WHAT)?.to_vec()))
    }

    /// A `PLP` value: a declared total or a sentinel, then chunks until an empty
    /// one, joined.
    ///
    /// The total is checked against what the chunks carried for
    /// [`TdsRows::partial_value`]'s reason — it is the server's word about bytes
    /// it had not sent yet, and a value that disagrees with it is a message this
    /// reader cannot prove its position in.
    fn partial(&mut self) -> io::Result<Option<Vec<u8>>> {
        const WHAT: &str = "PLP value";

        let bytes = self.take(8, WHAT)?;
        let total = u64::from_le_bytes(bytes.try_into().expect("eight bytes, as asked for"));
        if total == PLP_NULL {
            return Ok(None);
        }
        let mut out = Vec::new();
        loop {
            let chunk = self.long(WHAT)?;
            if chunk == 0 {
                break;
            }
            out.extend_from_slice(self.take(as_usize(chunk)?, WHAT)?);
        }
        let read = out.len() as u64;
        if total != PLP_UNKNOWN && read != total {
            return Err(malformed(format!(
                "a TDS PLP value declared {total} byte(s) and its chunks carried {read}"
            )));
        }
        Ok(Some(out))
    }

    /// A `TYPE_INFO`: the type byte, then whatever that type declares about
    /// itself.
    ///
    /// MS-TDS § 2.2.5.4.1's tables written as a `match`, and there is no
    /// shorter form of them: which [`Length`] family a type belongs to is a
    /// property of the byte and not of any range it falls in — `0x28` is
    /// measured by a byte while `0x27` two below it is a type from another
    /// decade.
    ///
    /// # Errors
    ///
    /// `InvalidData` for a type byte this driver does not read, naming it. TDS
    /// 4.2's `CHAR` (0x2F), `VARCHAR` (0x27), `BINARY` (0x2D), `VARBINARY`
    /// (0x25), `DECIMAL` (0x37) and `NUMERIC` (0x3F) are among them on purpose:
    /// a 7.4 server sends the `BIG…`/`…N` spelling in every case, so an arm for
    /// one would be a guess at a layout this driver can never observe. Also for
    /// a fractional-second scale past the seven digits SQL Server's three time
    /// types hold, which is a width this reader would otherwise have to invent.
    fn type_info(&mut self) -> io::Result<TypeInfo> {
        const WHAT: &str = "TYPE_INFO";

        let id = self.byte(WHAT)?;
        let mut info = TypeInfo {
            id,
            length: Length::Fixed(0),
            precision: 0,
            scale: 0,
            collation: None,
        };
        match id {
            // A column with no type carries no value either, and the zero above
            // is already that.
            TY_NULL => {}
            TY_INT1 | TY_BIT => info.length = Length::Fixed(1),
            TY_INT2 => info.length = Length::Fixed(2),
            TY_INT4 | TY_DATETIME4 | TY_FLT4 | TY_MONEY4 => info.length = Length::Fixed(4),
            TY_MONEY | TY_DATETIME | TY_FLT8 | TY_INT8 => info.length = Length::Fixed(8),
            TY_GUID | TY_INTN | TY_BITN | TY_FLTN | TY_MONEYN | TY_DATETIMEN => {
                info.length = Length::Byte(self.byte(WHAT)?);
            }
            TY_DECIMALN | TY_NUMERICN => {
                info.length = Length::Byte(self.byte(WHAT)?);
                info.precision = self.byte(WHAT)?;
                info.scale = self.byte(WHAT)?;
            }
            // `date` declares nothing at all: three bytes is the only width it
            // has, and it is still measured by a byte because a null one is
            // measured as none.
            TY_DATEN => info.length = Length::Byte(DATE_BYTES),
            TY_TIMEN | TY_DATETIME2N | TY_DATETIMEOFFSETN => {
                let scale = self.byte(WHAT)?;
                info.scale = scale;
                let seconds = match scale {
                    0..=2 => 3,
                    3..=4 => 4,
                    5..=7 => 5,
                    past => {
                        return Err(malformed(format!(
                            "a TDS TYPE_INFO gave type 0x{id:02X} a scale of {past}, past the \
                             seven fractional-second digits SQL Server holds"
                        )));
                    }
                };
                info.length = Length::Byte(match id {
                    TY_TIMEN => seconds,
                    TY_DATETIME2N => seconds + DATE_BYTES,
                    _ => seconds + DATE_BYTES + OFFSET_BYTES,
                });
            }
            TY_BIGBINARY | TY_BIGVARBINARY => info.length = self.declared_length(WHAT)?,
            TY_BIGCHAR | TY_BIGVARCHAR | TY_NCHAR | TY_NVARCHAR => {
                info.length = self.declared_length(WHAT)?;
                info.collation = Some(self.collation(WHAT)?);
            }
            TY_IMAGE | TY_VARIANT => info.length = Length::Long(self.long(WHAT)?),
            TY_TEXT | TY_NTEXT => {
                info.length = Length::Long(self.long(WHAT)?);
                info.collation = Some(self.collation(WHAT)?);
            }
            TY_XML => {
                // `SchemaPresent`, then the three names a bound column carries.
                // Read to skip: an `xml` value arrives the same way either way.
                if self.byte(WHAT)? != 0 {
                    self.b_varchar(WHAT)?;
                    self.b_varchar(WHAT)?;
                    self.us_varchar(WHAT)?;
                }
                info.length = Length::Partial;
            }
            TY_UDT => {
                // The declared maximum, then the four names a CLR type is
                // identified by — none of which changes how its bytes are read.
                self.short(WHAT)?;
                self.b_varchar(WHAT)?;
                self.b_varchar(WHAT)?;
                self.b_varchar(WHAT)?;
                self.us_varchar(WHAT)?;
                info.length = Length::Partial;
            }
            other => {
                return Err(malformed(format!(
                    "a TDS TYPE_INFO described a value as type 0x{other:02X}, which this driver \
                     does not read"
                )));
            }
        }
        Ok(info)
    }

    /// A `USHORTLEN` type's declared width, which is [`Length::Partial`] where
    /// the type is a `MAX` one.
    fn declared_length(&mut self, what: &'static str) -> io::Result<Length> {
        let declared = self.short(what)?;
        Ok(if declared == NO_LENGTH {
            Length::Partial
        } else {
            Length::Short(declared)
        })
    }

    /// A `COLLATION`, kept as the five bytes the server wrote.
    fn collation(&mut self, what: &'static str) -> io::Result<[u8; 5]> {
        Ok(self
            .take(5, what)?
            .try_into()
            .expect("five bytes, as asked for"))
    }

    /// Positions the reader at a token's declared end, refusing one whose
    /// fields already read past it.
    fn finish(&mut self, end: usize, what: &'static str) -> io::Result<()> {
        if self.at > end {
            return Err(malformed(format!(
                "a TDS {what} token's fields read {} byte(s) past the length it declared",
                self.at - end
            )));
        }
        self.at = end;
        Ok(())
    }
}

/// What [`ServerError::backend`] calls this one, and what its rendered sentence
/// therefore opens with.
///
/// The `[db.<name>]` block's own spelling, so an operator reading a refusal and
/// an operator reading the configuration that produced it are reading one word
/// — and deliberately not [`Driver::matrix_name`], which is a harness key
/// [`ServerError::backend`]'s own doc refuses to tie this to.
const BACKEND: &str = "sqlserver";

/// The server's refusal, as the `io::Error` every entry point here answers
/// with.
///
/// [`crate::mysql`]'s `server_refusal` and [`crate::pg`]'s `server_error` with
/// this backend's two absences. TDS sends **no `SQLSTATE`**, so
/// [`ServerError::sql_state`] is empty — that field's own doc owns what the
/// empty string means, and inventing ODBC's five characters here would fill the
/// slot a program reads with a value this driver made up, which is what
/// [`kind_of`] refuses to key its table on for the same reason. And its
/// severity is a number rather than a word, so [`ServerMessage::is_fatal`] is
/// what picks between the two words [`ServerError::severity`] is documented to
/// carry.
///
/// `constraint` is `None` on every refusal this backend words: SQL Server names
/// the index or the key inside the sentence and in no field of its own, and
/// § 8's rule against matching on message text is exactly the rule that stops
/// this driver taking it from there.
pub(crate) fn server_refusal(message: &ServerMessage) -> io::Error {
    io::Error::other(ServerError {
        kind: message.kind(),
        sql_state: String::new(),
        severity: String::from(if message.is_fatal() { "FATAL" } else { "ERROR" }),
        message: message.message.clone(),
        constraint: None,
        driver_code: Some(message.number),
        backend: BACKEND,
    })
}

/// LOGIN7 out over an encrypted wire, and the token stream that answers it read
/// to its end.
///
/// A free function generic in the stream for [`negotiate_tls`]'s reason: a
/// method on [`TdsConn`] could only be reached through a real socket and a real
/// certificate, and this is the half worth a unit test.
///
/// **The whole answer is read before anything acts on it.** The packet size an
/// `ENVCHANGE` announces is in force from the packet *after* the one that
/// carried it, and a refused login sends its `ERROR` before the `DONE` that
/// ends the message, so a reader that applied either mid-stream would be acting
/// on a message it had not finished. The first `ERROR` is the one kept: what
/// follows it is the server describing the consequences of the first, and the
/// `DONE` that ends a failed login says only that something failed.
///
/// The [`LoginAck`] is answered rather than kept, because the one thing on it a
/// driver could act on is the dialect — and this checks that itself: a server
/// answering a version other than [`TDS_VERSION`] is one this driver would need
/// a second set of readers for, and reading its tokens as 7.4's would be
/// guessing at the bytes rather than refusing them.
///
/// # Errors
///
/// [`login7_request`]'s `InvalidInput` for a field past [`MAX_FIELD_CHARS`]; an
/// `Other` carrying a [`ServerError`] for the login the server refused —
/// [`DbErrorKind::Permission`] for the usual one, since `18456` is what a wrong
/// password, an unknown login and a database this login may not open all
/// arrive as; `InvalidData` for an answer that is not a token stream, for one
/// that neither accepts nor refuses, for a dialect this driver does not speak
/// and for a packet size outside [`Codec::set_packet_size`]'s range; and
/// whatever the stream reported.
pub fn login<S: Read + Write>(wire: &mut Wire<S>, target: &TdsTarget<'_>) -> io::Result<LoginAck> {
    let request = login7_request(target, wire.framing().packet_size())?;
    wire.send(PacketType::Login7, Status::NORMAL, &request)?;

    let answer = wire.read_message()?;
    if answer.kind != PacketType::TabularResult {
        return Err(malformed(format!(
            "a LOGIN7 was answered with a packet of type 0x{:02X}",
            answer.kind.byte()
        )));
    }

    let mut tokens = Tokens::over(&answer.payload);
    let mut refusal = None;
    let mut ack = None;
    let mut packet_size = None;
    while let Some(token) = tokens.next_token()? {
        match token {
            Token::Error(message) if refusal.is_none() => refusal = Some(message),
            Token::LoginAck(answered) => ack = Some(answered),
            Token::Env(EnvChange::PacketSize { to }) => packet_size = Some(to),
            // Exhaustive rather than a wildcard, so a token this driver learns
            // to read is a decision here and not a silent omission. A login
            // answers with no result set, so `Columns` is one of them.
            Token::Error(_)
            | Token::Info(_)
            | Token::Env(_)
            | Token::Columns(_)
            | Token::ReturnStatus(_)
            | Token::ReturnValue(_)
            | Token::Done(_) => {}
        }
    }

    if let Some(message) = refusal {
        return Err(server_refusal(&message));
    }
    let Some(ack) = ack else {
        return Err(malformed(String::from(
            "a LOGIN7 was answered without either a LOGINACK or an ERROR",
        )));
    };
    if ack.tds_version != TDS_VERSION {
        return Err(malformed(format!(
            "the server answered a TDS 7.4 login speaking 0x{:08X}, which this driver has no readers for",
            ack.tds_version
        )));
    }
    if let Some(size) = packet_size {
        wire.codec().set_packet_size(size)?;
    }
    Ok(ack)
}

/// A statement's result, and the rows still to come out of it.
///
/// [`crate::MySqlRows`]' shape, and it is the same borrow for the same reason:
/// the handle holds the wire and the busy state, so the connection is unusable
/// for anything else until the stream ends — [ADR 0067
/// § 4](../../../docs/adr/0067-core-db.md)'s one-statement-at-a-time rule
/// enforced by the type system rather than by a check every caller has to
/// remember. A statement with no result set answers one of these too, already
/// ended: [`TdsRows::columns`] is empty and [`TdsRows::next_row`] is `None` on
/// the first call.
///
/// # What it holds that the other drivers' do not
///
/// **The remainder.** MySQL and PostgreSQL align a row with a packet, so their
/// readers own nothing between calls; a TDS row is cut wherever the packet size
/// lands, so this one buffers what a packet carried past the token it was
/// parsing and drops what it has read off the front on the next refill. The
/// buffer is therefore about a packet plus the token in hand, and it is
/// deliberately *not* the answer: a `PLP` value's chunks are copied into the
/// row as they arrive rather than accumulated here, so a `varbinary(max)`
/// column costs its own size and not its size twice.
///
/// **A token reader it hands the between-row tokens to.** [`Tokens`] parses
/// every token in a response but the two row ones, and reusing it is what keeps
/// `ERROR`, `INFO`, `ENVCHANGE`, `COLMETADATA` and `DONE` written once. It
/// parses a slice, so it is offered the buffer and asked again with more of it
/// when the token was not all there — [`TdsRows::token`] owns why that is
/// cheaper than it looks.
pub struct TdsRows<'a, S: Read + Write = NvsTls<Tunnel<NvsTcp>>> {
    wire: &'a mut Wire<S>,
    state: &'a Cell<State>,
    /// `COLMETADATA`'s columns, in the order their values arrive, and empty for
    /// a statement that returned no result set.
    columns: Vec<TdsColumn>,
    /// Bytes of the token stream that have arrived and are not parsed yet.
    buffer: Vec<u8>,
    /// How much of [`TdsRows::buffer`] is behind the reader. The prefix is
    /// dropped at the next refill rather than at every read, which is what
    /// keeps a row of many small columns from being a `drain` per column.
    at: usize,
    /// Whether the packet carrying [`Status::EOM`] has been read, so no further
    /// packet belongs to this answer.
    last: bool,
    /// Rows handed back so far.
    rows: u64,
    /// The last `DONE` that set [`Done::counted`], which is the count a
    /// statement with no result set reports.
    counted: Option<u64>,
    /// Whether the stream has ended, by any of its three ends: a `DONE`, the
    /// server's refusal, or a decode this driver will not continue past.
    ended: bool,
    /// The last `RETURNVALUE` the answer carried, which for § 1's `sp_prepexec`
    /// is the handle the server allocated.
    ///
    /// The *last* rather than all of them because a procedure this driver calls
    /// declares one output parameter, and a `Vec` for a list that is one long
    /// would be an allocation on every statement. It arrives near the end of
    /// the stream, after the rows, so a caller reads it once
    /// [`TdsRows::next_row`] has answered `None` — see [`TdsRows::returned`].
    returned: Option<ReturnValue>,
    /// [ADR 0067 § 1](../../../docs/adr/0067-core-db.md)'s cache and the key
    /// this answer's handle belongs under, for a `sp_prepexec` whose plan is to
    /// be kept; `None` for every other answer, which is most of them.
    ///
    /// The stream borrows the cache rather than the caller filing it afterwards
    /// — [`Filing`] owns why.
    filing: Option<Filing<'a>>,
    /// [ADR 0067 § 11](../../../docs/adr/0067-core-db.md)'s trace event for this
    /// statement, opened when the request went out and ended by whatever ends
    /// the stream — [`crate::MySqlRows`]' field, for [`crate::span`]'s reasons.
    span: QuerySpan,
}

impl<S: Read + Write> std::fmt::Debug for TdsRows<'_, S> {
    /// The shape of the result and where the wire is, and nothing that arrived.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TdsRows")
            .field("columns", &self.columns.len())
            .field("state", &self.state.get())
            .finish_non_exhaustive()
    }
}

impl<S: Read + Write> TdsRows<'_, S> {
    /// The result set's columns, empty for a statement that returned none.
    #[must_use]
    pub fn columns(&self) -> &[TdsColumn] {
        &self.columns
    }

    /// Column `index`'s Novis type, per [`TdsColumn::column_type`], or `None`
    /// where the result set has no such column.
    #[must_use]
    pub fn column_type(&self, index: usize) -> Option<ColumnType> {
        self.columns.get(index).map(TdsColumn::column_type)
    }

    /// [ADR 0067 § 11](../../../docs/adr/0067-core-db.md)'s trace event for this
    /// statement.
    ///
    /// Borrowed rather than taken, for [`crate::PgRows::span`]'s reason.
    #[must_use]
    pub fn span(&self) -> &QuerySpan {
        &self.span
    }

    /// Names the `[db.<name>]` block this statement ran on, for the layer that
    /// resolved it — [`QuerySpan::name`] owns why the driver cannot.
    pub fn name_connection(&mut self, connection: &str) {
        self.span.name(connection);
    }

    /// [ADR 0067 § 4](../../../docs/adr/0067-core-db.md)'s affected-row count,
    /// once the stream has ended.
    ///
    /// [`crate::MySqlRows::affected`]'s two numbers under one name — the rows
    /// that came back for a statement with a result set, and what the server
    /// counted for one without — with one difference this backend forces:
    /// **`None` is two facts here rather than one.** A stream still running has
    /// no count yet, and a `DONE` that did not set [`Done::counted`] reported
    /// none at all, which [`Done::counted`]'s own doc explains is not the same
    /// as zero. The caller that must tell them apart has already been told
    /// which: [`TdsRows::next_row`] answered `None`.
    ///
    /// There is no `lastId` beside it. SQL Server puts no generated key in the
    /// token stream at all — `SCOPE_IDENTITY()` is a statement a caller writes —
    /// so this driver has nothing to answer with, and PostgreSQL's `RETURNING`
    /// is the same absence.
    #[must_use]
    pub fn affected(&self) -> Option<u64> {
        if !self.ended {
            return None;
        }
        if self.columns.is_empty() {
            self.counted
        } else {
            Some(self.rows)
        }
    }

    /// The output parameter the procedure came back with, once the stream has
    /// reached it.
    ///
    /// For § 1's `sp_prepexec` that is the statement handle, and it arrives
    /// **after** the rows: the server sends `RETURNVALUE` next to the
    /// `RETURNSTATUS` that ends the procedure, so a caller filing the handle in
    /// the statement cache does it when the stream has ended and not when it
    /// opened. A statement that was never a procedure call answers `None`.
    #[must_use]
    pub fn returned(&self) -> Option<&ReturnValue> {
        self.returned.as_ref()
    }

    /// The next row, or `None` once the stream has ended.
    ///
    /// Deliberately not `Iterator::next`, for [`crate::PgRows::next_row`]'s
    /// reason: every call can fail, and an `Option` would have to swallow it.
    /// Ending the stream is what returns the connection to [`State::Idle`].
    ///
    /// # Errors
    ///
    /// The server's own error, which still ends the stream cleanly and leaves
    /// the connection idle; `InvalidData` for a token stream that does not add
    /// up against the columns, for a second result set, and for an answer that
    /// ended without a `DONE`, all of which poison the connection; and whatever
    /// the stream reported.
    pub fn next_row(&mut self) -> io::Result<Option<TdsRow>> {
        // The state is the only bookkeeping: anything that ended this stream
        // has already left `State::Streaming`.
        if self.state.get() != State::Streaming {
            return Ok(None);
        }
        match self.step() {
            Ok(row) => Ok(row),
            Err(e) => Err(poison_on_read(self.state, e)),
        }
    }

    /// Reads tokens until a row arrives or the answer ends.
    ///
    /// The refusal is collected rather than returned where it is found: an
    /// `ERROR` is followed by the `DONE` that ends the statement, and a reader
    /// that returned at the `ERROR` would leave those bytes on the wire and
    /// poison a connection the server had left perfectly usable. Rows that
    /// arrive after one are read and dropped for the same reason — the wire has
    /// to be walked past them either way, and there is nobody to hand them to
    /// once the statement has failed.
    fn step(&mut self) -> io::Result<Option<TdsRow>> {
        let mut refusal = None;
        loop {
            match self.peek()? {
                Some(kind @ (TOKEN_ROW | TOKEN_NBC_ROW)) => {
                    self.at += 1;
                    let row = self.row(kind == TOKEN_NBC_ROW)?;
                    if refusal.is_some() {
                        continue;
                    }
                    self.rows += 1;
                    self.span.row();
                    return Ok(Some(row));
                }
                Some(_) => match self.token()? {
                    Some(Token::Info(_) | Token::Env(_) | Token::ReturnStatus(_)) => {}
                    Some(Token::ReturnValue(returned)) => self.returned = Some(returned),
                    Some(Token::Error(message)) => {
                        if refusal.is_none() {
                            refusal = Some(message);
                        }
                    }
                    Some(Token::Done(done)) => {
                        if done.counted() {
                            self.counted = Some(done.rows);
                        }
                        // `DONEINPROC` is never the last token of an answer —
                        // § 1's `sp_prepexec` sends the `RETURNVALUE` carrying
                        // its handle after one — and a `DONE` that says more
                        // results follow is a batch this surface has nowhere to
                        // put: the second `COLMETADATA` below is where that is
                        // refused, once the tokens between here and it have been
                        // read rather than abandoned mid-packet.
                        if done.more() || done.in_proc {
                            continue;
                        }
                        self.end(refusal.is_some())?;
                        return match refusal {
                            Some(message) => Err(server_refusal(&message)),
                            None => Ok(None),
                        };
                    }
                    Some(Token::Columns(_)) => {
                        return Err(malformed(String::from(
                            "the server has a second result set for this statement, and ADR 0067 \
                             § 4's one-statement-at-a-time surface has nowhere to put it",
                        )));
                    }
                    Some(Token::LoginAck(_)) => {
                        return Err(malformed(String::from(
                            "a TDS statement was answered with a LOGINACK, which is a stream out \
                             of sync rather than a login",
                        )));
                    }
                    None => return Err(no_done()),
                },
                None => return Err(no_done()),
            }
        }
    }

    /// Ends the stream: freezes the span, and hands the connection back where
    /// the answer really did end at a packet boundary.
    ///
    /// The check is the whole of what makes the connection reusable. A `DONE`
    /// with bytes behind it, or one in a packet that never carried
    /// [`Status::EOM`], is an answer this driver has read only part of, and
    /// returning that wire to the pool is one request reading another's.
    ///
    /// # Errors
    ///
    /// `InvalidData` where the answer did not end where the `DONE` said it did.
    /// [`TdsRows::next_row`] poisons the connection on it.
    fn end(&mut self, refused: bool) -> io::Result<()> {
        self.ended = true;
        // No affected count on a refusal, as `crate::MySqlRows::next_row` does
        // it: § 11 gives a span no success field to lose, so a refused statement
        // reports the rows that did arrive and the error is the caller's own
        // return value.
        let affected = if refused { None } else { self.affected() };
        self.span.finished(affected);
        // § 1's cache is filed here and nowhere else, because here is the first
        // moment the handle exists: `sp_prepexec` writes it into a `RETURNVALUE`
        // that arrives after the rows, so a caller filing it would have to be
        // told to read `returned()` and would be free to forget. A refusal files
        // nothing — the server compiled no plan there is a number for — and
        // neither does a stream that ended any other way, which leaves that plan
        // alive on the server until the connection closes rather than filed
        // under a handle this side never read.
        let filing = self.filing.take().filter(|_| !refused);
        if let (Some(filing), Some(handle)) =
            (filing, self.returned.as_ref().and_then(ReturnValue::as_i32))
        {
            let plan = TdsPlan {
                handle,
                declared: filing.declared,
            };
            filing.cache.commit(&filing.sql, filing.arity, plan);
        }
        if !self.last || self.at < self.buffer.len() {
            return Err(malformed(format!(
                "a TDS answer carried {} byte(s) after the DONE that ended it",
                self.buffer.len() - self.at
            )));
        }
        self.state.set(State::Idle);
        Ok(())
    }

    /// The next token's type byte, without consuming it, or `None` where the
    /// answer has ended.
    fn peek(&mut self) -> io::Result<Option<u8>> {
        while self.at == self.buffer.len() {
            if self.last {
                return Ok(None);
            }
            self.fill()?;
        }
        Ok(Some(self.buffer[self.at]))
    }

    /// The next token, parsed by [`Tokens`] over the buffer this reader filled.
    ///
    /// **A failed parse is retried with more bytes rather than reported**, until
    /// the packet carrying [`Status::EOM`] has arrived. `Tokens` reads a slice
    /// and cannot say whether it ran out of bytes or ran into a bad one, and
    /// telling the two apart from the outside would mean a second table of
    /// every token's extent — `COLMETADATA`, which is the one a result set
    /// opens with, does not carry one at all. Retrying costs a re-parse of a
    /// token that is at most tens of kilobytes and only while it is incomplete;
    /// a malformed token is refused with the same message one packet later, and
    /// [`MAX_MESSAGE`] in [`TdsRows::fill`] is what stops a stream that never
    /// parses from growing without one.
    ///
    /// Rows never reach here: [`TdsRows::step`] reads the type byte first, and
    /// a row is the one token whose extent needs the column list.
    fn token(&mut self) -> io::Result<Option<Token>> {
        loop {
            let (parsed, consumed) = {
                let mut tokens = Tokens::over(&self.buffer[self.at..]);
                let parsed = tokens.next_token();
                (parsed, tokens.consumed())
            };
            match parsed {
                Ok(Some(token)) => {
                    self.at += consumed;
                    return Ok(Some(token));
                }
                Ok(None) if self.last => return Ok(None),
                Err(e) if self.last => return Err(e),
                _ => self.fill()?,
            }
        }
    }

    /// One row's values, against the columns that measure them.
    ///
    /// `nbc` is [`TOKEN_NBC_ROW`]: a bitmap of one bit per column, least
    /// significant bit first, and a set bit is a column that is `NULL` and
    /// carries no bytes at all. It is not an optimisation this reader may
    /// decline — the server sends whichever form it likes and the two are read
    /// differently — and it is the only place a null arrives without the type's
    /// own null form.
    fn row(&mut self, nbc: bool) -> io::Result<TdsRow> {
        let mut nulls = Vec::new();
        if nbc {
            let bitmap = self.columns.len().div_ceil(8);
            self.copy(bitmap, &mut nulls)?;
        }

        let mut bytes = Vec::new();
        let mut values = Vec::with_capacity(self.columns.len());
        for index in 0..self.columns.len() {
            if nbc && nulls[index / 8] & (1 << (index % 8)) != 0 {
                values.push(None);
                continue;
            }
            // Copied out because `TypeInfo` is `Copy` and the read below is a
            // `&mut self`: the alternative is a clone of the whole column list
            // per row.
            let info = self.columns[index].type_info;
            let start = bytes.len();
            let present = self.value(index, info, &mut bytes)?;
            values.push(present.then_some(start..bytes.len()));
        }
        Ok(TdsRow { bytes, values })
    }

    /// One column's value, appended to `bytes`, and whether it was there at all
    /// — `false` is SQL `NULL`.
    ///
    /// [`Length`] is the whole of the type knowledge here: this reads what the
    /// column said its values are measured by and never what they mean, which
    /// is [ADR 0067 § 9](../../../docs/adr/0067-core-db.md)'s decode and belongs
    /// to the crate that can allocate a `Core\Time\DateTime`.
    fn value(&mut self, index: usize, info: TypeInfo, bytes: &mut Vec<u8>) -> io::Result<bool> {
        // A `NULLTYPE` column carries no value and has no value to carry: the
        // one thing `SELECT NULL` can be is null, and reading it as a present
        // value of no bytes would make § 9's `?T` depend on the type byte a
        // layer up.
        if info.id == TY_NULL {
            return Ok(false);
        }
        match info.length {
            Length::Fixed(width) => {
                self.copy(width, bytes)?;
                Ok(true)
            }
            Length::Byte(most) => {
                let length = self.byte()?;
                if length == 0 {
                    return Ok(false);
                }
                self.bounded(u32::from(length), u32::from(most), index, info)?;
                self.copy(usize::from(length), bytes)?;
                Ok(true)
            }
            Length::Short(most) => {
                let length = self.short()?;
                if length == NO_LENGTH {
                    return Ok(false);
                }
                self.bounded(u32::from(length), u32::from(most), index, info)?;
                self.copy(usize::from(length), bytes)?;
                Ok(true)
            }
            Length::Long(most) => self.long_value(index, info, most, bytes),
            Length::Partial => self.partial_value(index, info, bytes),
        }
    }

    /// A `LONGLEN` value: the three deprecated types' text pointer, then the
    /// four-byte length [`Length::Long`] describes.
    fn long_value(
        &mut self,
        index: usize,
        info: TypeInfo,
        most: u32,
        bytes: &mut Vec<u8>,
    ) -> io::Result<bool> {
        if matches!(info.id, TY_TEXT | TY_NTEXT | TY_IMAGE) {
            let pointer = self.byte()?;
            if pointer == 0 {
                return Ok(false);
            }
            self.skip(usize::from(pointer) + TEXT_TIMESTAMP)?;
        }
        let length = self.long()?;
        // Zero is `NULL` only for `sql_variant`, which the type byte decides:
        // an empty `text` is a value, and it arrived through a pointer that was
        // there.
        if length == NO_LENGTH_LONG || (length == 0 && info.id == TY_VARIANT) {
            return Ok(false);
        }
        self.bounded(length, most, index, info)?;
        self.copy(as_usize(length)?, bytes)?;
        Ok(true)
    }

    /// A `PLP` value: a declared total or a sentinel, then chunks until an empty
    /// one.
    ///
    /// The chunks are copied straight into the row, so the buffer never holds
    /// the value and a `varbinary(max)` costs its own size once. The declared
    /// total is checked against what arrived rather than trusted: it is the
    /// server's word about bytes that had not been sent yet, and a value that
    /// disagrees with it is a stream this driver cannot prove the position of.
    fn partial_value(
        &mut self,
        index: usize,
        info: TypeInfo,
        bytes: &mut Vec<u8>,
    ) -> io::Result<bool> {
        let total = self.quad()?;
        if total == PLP_NULL {
            return Ok(false);
        }
        let start = bytes.len();
        if total != PLP_UNKNOWN {
            bytes.reserve(as_usize(total)?.min(PLP_RESERVE));
        }
        loop {
            let chunk = self.long()?;
            if chunk == 0 {
                break;
            }
            self.copy(as_usize(chunk)?, bytes)?;
        }
        let read = (bytes.len() - start) as u64;
        if total != PLP_UNKNOWN && read != total {
            return Err(malformed(format!(
                "a TDS PLP value for column {index} (type 0x{:02X}) declared {total} byte(s) and \
                 its chunks carried {read}",
                info.id
            )));
        }
        Ok(true)
    }

    /// Refuses a value wider than the column it belongs to.
    ///
    /// The declared width is the server's own description of the column two
    /// tokens earlier, so a value past it is the reader having lost its place
    /// in the stream — the one error worth catching before it is read as the
    /// next column's length.
    fn bounded(&self, length: u32, most: u32, index: usize, info: TypeInfo) -> io::Result<()> {
        if length > most {
            return Err(malformed(format!(
                "a TDS row's column {index} (type 0x{:02X}) carried {length} byte(s), past the \
                 {most} its COLMETADATA declared",
                info.id
            )));
        }
        Ok(())
    }

    /// Reads one more packet of this answer into the buffer, dropping what is
    /// already behind the reader.
    ///
    /// # Errors
    ///
    /// `InvalidData` for a packet that is not a tabular result, for an answer
    /// whose last packet left a token unfinished, and for a single token past
    /// [`MAX_MESSAGE`] — the same ceiling [`Wire::read_message`] holds a whole
    /// message to, and here it bounds one token rather than the answer, since a
    /// row's values leave the buffer as they arrive.
    fn fill(&mut self) -> io::Result<()> {
        if self.last {
            return Err(malformed(String::from(
                "a TDS answer's last packet ended in the middle of a token",
            )));
        }
        let packet = self.wire.read_packet()?;
        if packet.kind != PacketType::TabularResult {
            return Err(malformed(format!(
                "a TDS answer continued with a packet of type 0x{:02X}, which is not a tabular \
                 result",
                packet.kind.byte()
            )));
        }
        self.last = packet.status.contains(Status::EOM);
        if self.at > 0 {
            self.buffer.drain(..self.at);
            self.at = 0;
        }
        if self.buffer.len() + packet.payload.len() > MAX_MESSAGE {
            return Err(malformed(format!(
                "a TDS token grew past this driver's {MAX_MESSAGE}-byte ceiling"
            )));
        }
        self.buffer.extend_from_slice(&packet.payload);
        Ok(())
    }

    /// Ensures the buffer holds `n` bytes the reader has not read.
    ///
    /// Only ever asked for a field, never for a value: a value is copied out
    /// with [`TdsRows::copy`], which spans packets instead of demanding they be
    /// contiguous — a `PLP` chunk is four bytes of length away from being
    /// two gigabytes.
    fn need(&mut self, n: usize) -> io::Result<()> {
        while self.buffer.len() - self.at < n {
            self.fill()?;
        }
        Ok(())
    }

    /// One byte of a token's own fields.
    fn byte(&mut self) -> io::Result<u8> {
        self.need(1)?;
        let byte = self.buffer[self.at];
        self.at += 1;
        Ok(byte)
    }

    /// Two bytes, little-endian, as everything inside a payload is.
    fn short(&mut self) -> io::Result<u16> {
        self.need(2)?;
        let short = u16::from_le_bytes([self.buffer[self.at], self.buffer[self.at + 1]]);
        self.at += 2;
        Ok(short)
    }

    /// Four bytes, little-endian.
    fn long(&mut self) -> io::Result<u32> {
        self.need(4)?;
        let long = u32::from_le_bytes(
            self.buffer[self.at..self.at + 4]
                .try_into()
                .expect("four bytes, as asked for"),
        );
        self.at += 4;
        Ok(long)
    }

    /// Eight bytes, little-endian: a `PLP` value's declared total.
    fn quad(&mut self) -> io::Result<u64> {
        self.need(8)?;
        let quad = u64::from_le_bytes(
            self.buffer[self.at..self.at + 8]
                .try_into()
                .expect("eight bytes, as asked for"),
        );
        self.at += 8;
        Ok(quad)
    }

    /// Copies `n` bytes out of the stream, reading packets as it needs them.
    fn copy(&mut self, n: usize, out: &mut Vec<u8>) -> io::Result<()> {
        let mut left = n;
        while left > 0 {
            if self.at == self.buffer.len() {
                self.fill()?;
                continue;
            }
            let take = left.min(self.buffer.len() - self.at);
            out.extend_from_slice(&self.buffer[self.at..self.at + take]);
            self.at += take;
            left -= take;
        }
        Ok(())
    }

    /// Steps over `n` bytes, reading packets as it needs them.
    fn skip(&mut self, n: usize) -> io::Result<()> {
        let mut left = n;
        while left > 0 {
            if self.at == self.buffer.len() {
                self.fill()?;
                continue;
            }
            let take = left.min(self.buffer.len() - self.at);
            self.at += take;
            left -= take;
        }
        Ok(())
    }
}

/// One row, already read out of the packets it was framed in.
///
/// **Eager, and one allocation for the values rather than one each.**
/// [`crate::PgRow`] is lazy because a `DataRow` is a run of length-prefixed
/// bodies inside one packet it already owns; a TDS row is neither — a value's
/// width comes from its column's declared type, so finding column five means
/// measuring columns zero to four, and the bytes may have arrived in three
/// packets that no longer exist. Once that walk is unavoidable, keeping the
/// result is free and re-walking per column is what would cost.
///
/// The values are the server's bytes, undecoded: [ADR 0067
/// § 9](../../../docs/adr/0067-core-db.md)'s table is applied by `nvs-stdlib`
/// against [`TdsColumn::type_info`], which is the boundary
/// [`crate::PgColumn::decode`] sits on for the other driver.
pub struct TdsRow {
    /// Every present value's bytes, end to end.
    bytes: Vec<u8>,
    /// Where each column's value is in [`TdsRow::bytes`], and `None` for a
    /// column that was `NULL`.
    values: Vec<Option<Range<usize>>>,
}

impl std::fmt::Debug for TdsRow {
    /// How many columns, and none of their values: a row in flight is one
    /// request's data — the rule [`Wire`]'s own rendering follows.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TdsRow")
            .field("columns", &self.values.len())
            .finish_non_exhaustive()
    }
}

impl TdsRow {
    /// How many columns this row has, which is the result set's column count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Whether the row has no columns at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Column `index`'s bytes.
    ///
    /// **The two `None`s are two different facts**, which is why they are
    /// nested rather than flattened: the outer one is a column the result set
    /// does not have, and the inner one is SQL `NULL` — § 9's `?T`, and the
    /// only one of the two a program can see.
    #[must_use]
    pub fn column(&self, index: usize) -> Option<Option<&[u8]>> {
        self.values
            .get(index)
            .map(|range| range.clone().map(|range| &self.bytes[range]))
    }
}

/// Reads a statement's answer as far as its shape: the tokens in front of the
/// first row, and the `COLMETADATA` that says what a row is.
///
/// A free function generic in the stream for [`login`]'s reason: a method on
/// [`TdsConn`] could only be reached through a real socket and a real
/// certificate. The request is already on the wire when this is called — which
/// of § 1's two ways it was written is the caller's business, and this reads the
/// same tokens either way.
///
/// # Errors
///
/// An `Other` carrying a [`ServerError`] for a statement the server refused;
/// `InvalidData` for an answer that is not a token stream and for one that ends
/// without either a `COLMETADATA` or a `DONE`; and whatever the stream reported.
/// Everything but the refusal poisons the connection.
pub fn read_rows<'a, S: Read + Write>(
    wire: &'a mut Wire<S>,
    state: &'a Cell<State>,
    span: QuerySpan,
) -> io::Result<TdsRows<'a, S>> {
    read_answer(wire, state, span, None)
}

/// The cache entry an answer is about to complete: [ADR 0067
/// § 1](../../../docs/adr/0067-core-db.md)'s key, and the cache to file the
/// handle in once the token carrying it arrives.
///
/// **The stream holds this rather than the caller** because `sp_prepexec`'s
/// handle is a `RETURNVALUE` that arrives *after* the rows, and a statement with
/// no result set has already ended by the time [`read_rows`] returns — so a
/// caller filing it afterwards would file nothing on exactly the statements a
/// cache is worth the most on. [`TdsRows::end`] is the one place it lands.
#[derive(Debug)]
struct Filing<'a> {
    /// Where the handle goes.
    cache: &'a mut StatementCache<TdsPlan>,
    /// § 1's key, first half: the statement as written.
    sql: String,
    /// § 1's key, second half: how many markers § 5's rewrite left in it.
    arity: usize,
    /// The `@params` the plan is being compiled against — [`TdsPlan::declared`]
    /// owns why a plan is not usable without it.
    declared: Rc<str>,
}

/// [`read_rows`], plus the cache entry a `sp_prepexec` answer completes.
fn read_answer<'a, S: Read + Write>(
    wire: &'a mut Wire<S>,
    state: &'a Cell<State>,
    span: QuerySpan,
    filing: Option<Filing<'a>>,
) -> io::Result<TdsRows<'a, S>> {
    state.set(State::Streaming);
    let mut rows = TdsRows {
        wire,
        state,
        columns: Vec::new(),
        buffer: Vec::new(),
        at: 0,
        last: false,
        rows: 0,
        counted: None,
        ended: false,
        returned: None,
        filing,
        span,
    };
    match rows.shape() {
        Ok(()) => Ok(rows),
        Err(e) => Err(poison_on_read(state, e)),
    }
}

impl<S: Read + Write> TdsRows<'_, S> {
    /// The tokens up to and including the `COLMETADATA`, or the `DONE` of a
    /// statement that has no result set to describe.
    fn shape(&mut self) -> io::Result<()> {
        let mut refusal = None;
        loop {
            match self.token()? {
                Some(Token::Columns(columns)) => {
                    self.columns = columns;
                    return Ok(());
                }
                Some(Token::Info(_) | Token::Env(_) | Token::ReturnStatus(_)) => {}
                Some(Token::ReturnValue(returned)) => self.returned = Some(returned),
                Some(Token::Error(message)) => {
                    if refusal.is_none() {
                        refusal = Some(message);
                    }
                }
                Some(Token::Done(done)) => {
                    if done.counted() {
                        self.counted = Some(done.rows);
                    }
                    // [`TdsRows::step`]'s test, for its reasons.
                    if done.more() || done.in_proc {
                        continue;
                    }
                    self.end(refusal.is_some())?;
                    return match refusal {
                        Some(message) => Err(server_refusal(&message)),
                        None => Ok(()),
                    };
                }
                Some(Token::LoginAck(_)) => {
                    return Err(malformed(String::from(
                        "a TDS statement was answered with a LOGINACK, which is a stream out of \
                         sync rather than a login",
                    )));
                }
                None => return Err(no_done()),
            }
        }
    }
}

/// `sp_prepexec`'s procedure id — MS-TDS § 2.2.6.6's `Sp_PrepExec`.
///
/// Called by number rather than by name. The id is the shorter form of
/// `NameLenProcID` and it saves the server the lookup as well as the bytes,
/// which is the whole difference between the two spellings.
const PROC_SP_PREPEXEC: u16 = 13;

/// `sp_execute`'s procedure id — MS-TDS § 2.2.6.6's `Sp_Execute`.
///
/// [ADR 0067 § 1](../../../docs/adr/0067-core-db.md)'s cached re-execution: the
/// handle [`PROC_SP_PREPEXEC`] answered with and the values, and no SQL on the
/// wire at all.
const PROC_SP_EXECUTE: u16 = 12;

/// `sp_unprepare`'s procedure id — MS-TDS § 2.2.6.6's `Sp_Unprepare`.
///
/// What an eviction sends, so the server never holds more plans than the
/// block's `statement_cache` allows.
const PROC_SP_UNPREPARE: u16 = 15;

/// The batch [`Status::RESET_CONNECTION`] rides.
///
/// The bit resets the session *before* the message carrying it is processed, so
/// there has to be a message — and this is the smallest well-formed one whose
/// answer proves the reset landed. It deliberately sets nothing: anything this
/// batch did would be session state the reset had just finished removing.
const RESET_STATEMENT: &str = "select 1";

/// `NameLenProcID`'s first field when what follows is a procedure *id* rather
/// than a name.
const PROC_ID_SWITCH: u16 = 0xFFFF;

/// `ALL_HEADERS`'s `TotalLength`, which counts itself: its own four bytes plus
/// the eighteen of the one header every request here carries.
const ALL_HEADERS_BYTES: u32 = 22;

/// The transaction-descriptor header's own length, likewise counting itself.
const TRANSACTION_HEADER_BYTES: u32 = 18;

/// `HeaderType` for that header, which is the only one of the three a request
/// may carry that TDS 7.2 and later require.
const HEADER_TRANSACTION: u16 = 0x0002;

/// `StatusFlags` for a parameter the server may write back —
/// `sp_prepexec`'s `@handle`, and nothing else this driver sends.
const PARAM_BY_REF: u8 = 0x01;

/// `INTN`'s declared width for the one integer parameter this driver sends.
const INTN_BYTES: u8 = 4;

/// The widest `nvarchar` that is not a `MAX` one, in characters. A value past
/// it is declared and sent as `nvarchar(max)`, which arrives in `PLP` chunks.
const NVARCHAR_CHARS: u16 = 4000;

/// A parameter's `COLLATION`, all zeroes, which is TDS's spelling of *the
/// server's own default*.
///
/// The field decides how a value **compares**, never how it is carried:
/// `nvarchar` is UCS-2 whatever the collation says. Sending anything else would
/// be this driver deciding a sort order for someone else's database.
const NO_COLLATION: [u8; 5] = [0; 5];

/// [ADR 0067 § 1](../../../docs/adr/0067-core-db.md)'s prepare and execute in
/// one message: `sp_prepexec`, carrying § 5's rewritten SQL and its parameters
/// as the procedure's own arguments.
///
/// **`sp_prepexec` rather than `sp_executesql` because § 1 keeps a statement
/// cache.** Both send the SQL and the parameters in one round trip; only this
/// one hands back a handle, and the handle is what a later `sp_execute` costs
/// nothing to run. The first execution of a statement is therefore one round
/// trip here — where § 1 records two for MySQL — and a cached one is one as
/// well, with no SQL on the wire at all.
///
/// **Every parameter goes out as `nvarchar` and the server casts it**, which is
/// [`crate::mysql::execute`]'s `MYSQL_TYPE_VAR_STRING` account reached through a
/// different protocol: the values arrive here already encoded as text by the
/// layer that knows what they are, and no byte of one is ever parsed as SQL —
/// § 1's no-emulated-prepares rule holds as a property of this function, since
/// `sql` is a separate argument to the procedure and never a string a value is
/// spliced into. The `@params` declaration beside it is what makes the cast the
/// *server's* decision rather than a guess: it names each marker's type, and
/// [`Dialect::marker`] is asked for the names so that the declaration and the
/// rewritten SQL cannot drift apart.
///
/// **A parameter that is not UTF-8 is refused rather than reinterpreted.** § 9's
/// `bytes` maps to `varbinary`, and `nvarchar` → `varbinary` on SQL Server is a
/// reinterpretation of UCS-2 code units rather than a parse — so a binary
/// parameter needs an encoding of its own, which is the encoder's slice and not
/// this one's. Refusing is what keeps that gap visible instead of silently
/// storing the wrong bytes.
///
/// # Errors
///
/// `InvalidInput` for a parameter that is not UTF-8, and for one whose UCS-2
/// form is past [`MAX_MESSAGE`].
pub fn sp_prepexec_request(sql: &str, params: &[Option<&[u8]>]) -> io::Result<Vec<u8>> {
    let bound = bind(params)?;
    prepexec_request(sql, &bound, declarations(&bound).as_deref())
}

/// Every bound value as the UCS-2 [`text_param`] writes, or the refusal that
/// names the marker one of them was bound at.
///
/// Done once per statement and before anything is written, so a refusal costs
/// no bytes on the wire and neither of § 1's two request shapes has to repeat
/// the conversion to decide which of them is being sent.
///
/// # Errors
///
/// `InvalidInput` for a value that is not UTF-8 — [`text_of`]'s refusal.
fn bind(params: &[Option<&[u8]>]) -> io::Result<Vec<Option<Vec<u8>>>> {
    let mut bound = Vec::with_capacity(params.len());
    for (index, value) in params.iter().enumerate() {
        bound.push(match value {
            None => None,
            Some(bytes) => Some(ucs2_of(text_of(bytes, index + 1)?)),
        });
    }
    Ok(bound)
}

/// A request's `ALL_HEADERS` and `NameLenProcID`, up to the first argument.
fn rpc_header(proc_id: u16) -> Vec<u8> {
    let mut out = Vec::new();
    all_headers(&mut out);
    out.extend_from_slice(&PROC_ID_SWITCH.to_le_bytes());
    out.extend_from_slice(&proc_id.to_le_bytes());
    // `OptionFlags`: neither `fWithRecomp` nor the two metadata ones. A
    // recompile on every execution is the opposite of what § 1's cache is for.
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

/// [`sp_prepexec_request`] over values already bound and a declaration already
/// derived, which is what [`start_statement`] is holding by the time it knows
/// this is the shape to send.
fn prepexec_request(
    sql: &str,
    bound: &[Option<Vec<u8>>],
    declared: Option<&str>,
) -> io::Result<Vec<u8>> {
    let mut out = rpc_header(PROC_SP_PREPEXEC);
    // `@handle`, sent null and by reference: the server allocates the plan and
    // writes the number back in the `RETURNVALUE` [`TdsRows::returned`] keeps.
    int_param(&mut out, true, None);
    let declaration = declared.map(ucs2_of);
    text_param(&mut out, declaration.as_deref(), "@params")?;
    text_param(&mut out, Some(&ucs2_of(sql)), "@stmt")?;
    values(&mut out, bound)?;
    Ok(out)
}

/// § 1's cached re-execution: `sp_execute`, naming the plan the server already
/// holds and carrying the same values in the same order.
///
/// **No SQL and no `@params`.** The plan already knows both, which is the whole
/// of what a hit buys on this protocol — a four-byte handle where the first
/// execution carried the statement. The declaration the plan was compiled
/// against is therefore not this call's to change, which is why
/// [`TdsPlan::declared`] is compared before one is sent.
///
/// # Errors
///
/// As [`text_param`], for the same values.
fn execute_request(handle: i32, bound: &[Option<Vec<u8>>]) -> io::Result<Vec<u8>> {
    let mut out = rpc_header(PROC_SP_EXECUTE);
    // By value rather than by reference: this one is read and never written
    // back, so there is nothing for the server to return.
    int_param(&mut out, false, Some(handle));
    values(&mut out, bound)?;
    Ok(out)
}

/// § 1's eviction: `sp_unprepare`, dropping one plan the server is holding.
///
/// Sent as a message of its own and read to its `DONEPROC`, unlike MySQL's
/// `COM_STMT_CLOSE`, which answers nothing. Every TDS request has an answer, so
/// one left unread would be taken as the *next* statement's — this is the
/// protocol's difference and not a choice, and it is why an eviction costs a
/// round trip here and none there.
fn unprepare_request(handle: i32) -> Vec<u8> {
    let mut out = rpc_header(PROC_SP_UNPREPARE);
    int_param(&mut out, false, Some(handle));
    out
}

/// The batch [`RESET_STATEMENT`] goes out as, carrying no headers of its own
/// beyond the transaction descriptor every request needs.
fn reset_request() -> Vec<u8> {
    let mut out = Vec::new();
    all_headers(&mut out);
    out.extend_from_slice(&ucs2_of(RESET_STATEMENT));
    out
}

/// One argument per marker, in § 5's order, for either of the two procedures
/// that take them.
///
/// # Errors
///
/// As [`text_param`].
fn values(out: &mut Vec<u8>, bound: &[Option<Vec<u8>>]) -> io::Result<()> {
    for (index, value) in bound.iter().enumerate() {
        text_param(out, value.as_deref(), &Dialect::SqlServer.marker(index + 1))?;
    }
    Ok(())
}

/// One bound value as text, or the refusal that names the marker it was bound
/// at.
fn text_of(value: &[u8], marker: usize) -> io::Result<&str> {
    std::str::from_utf8(value).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "the value bound at {} is not UTF-8, and this driver sends every parameter as \
                 `nvarchar` — ADR 0067 § 9's `bytes` needs an encoding of its own here rather \
                 than a reinterpretation of these bytes as UCS-2",
                Dialect::SqlServer.marker(marker)
            ),
        )
    })
}

/// Text as UCS-2LE, which is the one form TDS carries a character value in.
fn ucs2_of(text: &str) -> Vec<u8> {
    text.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

/// Whether a value needs the `MAX` form — the one decision `@params` and the
/// parameter itself have to agree on, so it is asked once and here.
fn is_wide(ucs2: &[u8]) -> bool {
    ucs2.len() > usize::from(NVARCHAR_CHARS) * 2
}

/// A request's `ALL_HEADERS`: the transaction descriptor TDS 7.2 and later
/// require, and nothing else.
fn all_headers(out: &mut Vec<u8>) {
    out.extend_from_slice(&ALL_HEADERS_BYTES.to_le_bytes());
    out.extend_from_slice(&TRANSACTION_HEADER_BYTES.to_le_bytes());
    out.extend_from_slice(&HEADER_TRANSACTION.to_le_bytes());
    // The transaction this request enlists in, and zero is *none*. § 7's
    // transactions are statements on this connection rather than the descriptor
    // MARS needs, so there is never a number to name here.
    out.extend_from_slice(&0u64.to_le_bytes());
    // `OutstandingRequestCount`: one, which is all § 4 ever allows in flight.
    out.extend_from_slice(&1u32.to_le_bytes());
}

/// `sp_prepexec`'s `@params`: § 5's markers with the type each is sent as, or
/// `None` for a statement that binds nothing — which the procedure reads as a
/// plan with no parameters, and an empty string would not.
fn declarations(bound: &[Option<Vec<u8>>]) -> Option<String> {
    if bound.is_empty() {
        return None;
    }
    let mut out = String::new();
    for (index, value) in bound.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(&Dialect::SqlServer.marker(index + 1));
        // A null is declared narrow: it carries no characters, so nothing about
        // it wants the `MAX` form, and the plan a later execution of the same
        // statement reuses is the one compiled against the narrow declaration.
        match value {
            Some(ucs2) if is_wide(ucs2) => out.push_str(" nvarchar(max)"),
            _ => out.push_str(&format!(" nvarchar({NVARCHAR_CHARS})")),
        }
    }
    Some(out)
}

/// A parameter's `ParamMetaData` up to its `TYPE_INFO`.
///
/// The name is always empty. A procedure's arguments are read positionally when
/// they are unnamed, and naming them would put SQL Server's own spelling of
/// `sp_prepexec`'s parameters in this driver — a thing a release is free to
/// change and this driver would have no way to notice.
fn param_header(out: &mut Vec<u8>, by_ref: bool) {
    out.push(0);
    out.push(if by_ref { PARAM_BY_REF } else { 0 });
}

/// An `INTN` argument: four bytes, or `NULL` at a length of zero.
fn int_param(out: &mut Vec<u8>, by_ref: bool, value: Option<i32>) {
    param_header(out, by_ref);
    out.push(TY_INTN);
    out.push(INTN_BYTES);
    match value {
        None => out.push(0),
        Some(number) => {
            out.push(INTN_BYTES);
            out.extend_from_slice(&number.to_le_bytes());
        }
    }
}

/// An `NVARCHAR` argument, in whichever of its two forms the value fits:
/// `USHORTLEN` up to [`NVARCHAR_CHARS`], and `PLP` past it.
///
/// `what` is the marker or procedure parameter the value was bound at, for the
/// refusal alone — the request itself sends no names.
///
/// # Errors
///
/// `InvalidInput` for a value past [`MAX_MESSAGE`], which is the ceiling this
/// driver reads a message to and therefore the widest one it is willing to
/// write.
fn text_param(out: &mut Vec<u8>, value: Option<&[u8]>, what: &str) -> io::Result<()> {
    param_header(out, false);
    out.push(TY_NVARCHAR);

    let wide = value.is_some_and(is_wide);
    if wide {
        out.extend_from_slice(&NO_LENGTH.to_le_bytes());
    } else {
        out.extend_from_slice(&(NVARCHAR_CHARS * 2).to_le_bytes());
    }
    out.extend_from_slice(&NO_COLLATION);

    let Some(ucs2) = value else {
        // The null of whichever form the type declared, which are two different
        // sentinels rather than one.
        if wide {
            out.extend_from_slice(&PLP_NULL.to_le_bytes());
        } else {
            out.extend_from_slice(&NO_LENGTH.to_le_bytes());
        }
        return Ok(());
    };
    if ucs2.len() > MAX_MESSAGE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "the value bound at {what} is {} byte(s) of UCS-2, past the {MAX_MESSAGE} a TDS \
                 message holds",
                ucs2.len()
            ),
        ));
    }
    let length = u32::try_from(ucs2.len()).expect("checked against MAX_MESSAGE above");
    if wide {
        // One chunk and its terminator. The value is in hand, so the declared
        // total is the truth rather than `PLP_UNKNOWN`, and a server reading it
        // can size its buffer once.
        out.extend_from_slice(&u64::from(length).to_le_bytes());
        out.extend_from_slice(&length.to_le_bytes());
        out.extend_from_slice(ucs2);
        out.extend_from_slice(&0u32.to_le_bytes());
    } else {
        let short = u16::try_from(length).expect("narrower than NVARCHAR_CHARS characters");
        out.extend_from_slice(&short.to_le_bytes());
        out.extend_from_slice(ucs2);
    }
    Ok(())
}

/// One plan this connection has the server holding, as [ADR 0067
/// § 1](../../../docs/adr/0067-core-db.md)'s cache records it.
///
/// The handle alone would be [`crate::mysql::Prepared`]'s twin. It is not
/// enough here, and the second field is why.
#[derive(Debug, Clone)]
pub struct TdsPlan {
    /// The number `sp_execute` and `sp_unprepare` name — the `@handle`
    /// `sp_prepexec` wrote back.
    pub handle: i32,
    /// The `@params` declaration this plan was compiled against, empty for a
    /// statement that binds nothing.
    ///
    /// **§ 1's key is one component short on this protocol.** [`declarations`]
    /// widens a marker to `nvarchar(max)` for a value past [`NVARCHAR_CHARS`],
    /// so one statement run first with a short value and then with a long one
    /// wants two different plans under a key — the SQL text and the arity —
    /// that cannot tell them apart. A long value bound against a plan declared
    /// narrow is *truncated* by SQL Server rather than refused, which is silent
    /// data loss, so this is carried and compared and a mismatch is a miss that
    /// unprepares the plan it did not fit. The key itself is left alone: § 1
    /// states it once, for four drivers, and this is one driver's reason to
    /// reject a hit rather than a fifth way to spell the key.
    ///
    /// `Rc<str>` because [`StatementCache::lookup`] clones the handle on every
    /// hit, and a hit is the path the cache exists for.
    declared: Rc<str>,
}

/// [ADR 0067 §§ 1 and 4](../../../docs/adr/0067-core-db.md)'s one statement,
/// end to end: the RPC out, and the token stream that answers it.
///
/// [`crate::mysql::start_statement`]'s shape and its reasons — free and generic
/// in the stream so a unit test can script a server for it, and taking the busy
/// state by reference so § 4's one-statement-at-a-time rule is enforced here
/// rather than by each caller remembering to.
///
/// **§ 1's cache decides which of two requests goes out.** A hit sends
/// [`execute_request`] — the handle and the values, no SQL — and files nothing,
/// since the plan is already recorded. A miss sends [`prepexec_request`] and
/// hands the stream a [`Filing`], because the handle it will be recorded under
/// arrives in a `RETURNVALUE` after the rows. An eviction and a rejected hit
/// both send [`unprepare_request`] *first*, so the server never holds more
/// plans than `statement_cache` allows, not even for the length of one round
/// trip — `crate::mysql`'s `cached_statement` ordering, for its reason.
///
/// # Errors
///
/// `InvalidInput` for a statement written to a connection that is not idle and
/// for [`bind`]'s and [`text_param`]'s refusals — none of which touches the
/// wire, so none poisons the connection; otherwise as [`read_rows`], including
/// for an eviction's own answer. A write that failed part-way leaves the
/// connection [`State::Poisoned`], because a half-written packet is not a
/// boundary anything can be found from.
pub fn start_statement<'a, S: Read + Write>(
    wire: &'a mut Wire<S>,
    state: &'a Cell<State>,
    cache: &'a mut StatementCache<TdsPlan>,
    sql: &str,
    params: &[Option<&[u8]>],
) -> io::Result<TdsRows<'a, S>> {
    if !state.get().may_start_statement() {
        return Err(crate::pg::second_statement(state));
    }

    // ADR 0067 § 11's span, opened before the request goes out and handed `sql`
    // and never `params` — `crate::span`'s module doc owns why that is a
    // signature rather than a rule.
    let span = QuerySpan::opened(Driver::SqlServer, sql);
    let bound = bind(params)?;
    let declared = declarations(&bound);
    let declared: Rc<str> = Rc::from(declared.as_deref().unwrap_or_default());

    let mut stale = None;
    match cache.lookup(sql, params.len()) {
        Some(plan) if plan.declared == declared => {
            let request = execute_request(plan.handle, &bound)?;
            send_request(wire, state, PacketType::Rpc, Status::NORMAL, &request)?;
            return read_answer(wire, state, span, None);
        }
        // A hit whose plan was compiled against a different declaration — see
        // `TdsPlan::declared`. The entry is dropped rather than shadowed so the
        // cache never holds two under one key, and the plan is unprepared
        // because nothing else will ever name it again.
        Some(plan) => {
            cache.forget(sql, params.len());
            stale = Some(plan.handle);
        }
        None => {}
    }

    // Built before anything is written, so a value this driver will not send
    // costs neither an eviction nor a byte on the wire.
    let request = prepexec_request(sql, &bound, cache_declaration(&declared))?;
    if let Some(handle) = stale.or_else(|| cache.make_room().map(|plan| plan.handle)) {
        send_request(
            wire,
            state,
            PacketType::Rpc,
            Status::NORMAL,
            &unprepare_request(handle),
        )?;
        drain(wire, state)?;
    }
    send_request(wire, state, PacketType::Rpc, Status::NORMAL, &request)?;
    read_answer(
        wire,
        state,
        span,
        Some(Filing {
            cache,
            sql: sql.to_owned(),
            arity: params.len(),
            declared,
        }),
    )
}

/// The `@params` a declaration string stands for: `None` where it is empty,
/// which is the statement that binds nothing and which the procedure reads as a
/// plan with no parameters where an empty string would not.
fn cache_declaration(declared: &Rc<str>) -> Option<&str> {
    (!declared.is_empty()).then(|| &**declared)
}

/// [ADR 0067 § 13](../../../docs/adr/0067-core-db.md)'s reset, and the cache it
/// takes with it.
///
/// **`sp_reset_connection` as [`Status::RESET_CONNECTION`] on a message of its
/// own**, which is MS-TDS's own spelling of that procedure and is why nothing
/// here names it: the bit resets the session before the message carrying it is
/// processed, and [`RESET_STATEMENT`] is the smallest well-formed message there
/// is to carry it. Its answer is what proves the reset landed, which is what
/// § 13 asks of a reset and what a bit riding the *next* request could not give
/// — that request already belongs to the program the connection was handed to.
///
/// [`crate::mysql::reset_session`]'s twin with one half missing: TDS has no
/// session time zone to send again, [`TdsTarget::time_zone`] owns why, so the
/// cache is the whole of what has to follow the reset. It is emptied here
/// rather than by whoever pools the connection, for that function's reason —
/// the plans are gone from the server the moment this answers, and a cache
/// still naming them would bind the next request against handles this session
/// no longer has.
///
/// Free and generic in the stream for this crate's usual reason: a
/// `Wire<NvsTls<Tunnel<NvsTcp>>>` needs a socket and a certificate that no unit
/// test has.
///
/// # Errors
///
/// As [`read_rows`], for the batch the bit rode in on. § 13 destroys the
/// connection on any of them, so what the cache holds on that path is nobody's
/// business.
pub fn reset_session<S: Read + Write>(
    wire: &mut Wire<S>,
    state: &Cell<State>,
    cache: &mut StatementCache<TdsPlan>,
) -> io::Result<()> {
    send_request(
        wire,
        state,
        PacketType::SqlBatch,
        Status::RESET_CONNECTION,
        &reset_request(),
    )?;
    drain(wire, state)?;
    cache.clear();
    Ok(())
}

/// Writes one request and says where a failed write leaves the connection.
///
/// # Errors
///
/// Whatever the write reported, with the connection [`State::Poisoned`]: a
/// half-written packet is not a boundary anything can be found from.
fn send_request<S: Read + Write>(
    wire: &mut Wire<S>,
    state: &Cell<State>,
    kind: PacketType,
    status: Status,
    request: &[u8],
) -> io::Result<()> {
    state.set(State::Executing);
    if let Err(e) = wire.send(kind, status, request) {
        state.set(State::Poisoned);
        return Err(e);
    }
    Ok(())
}

/// Reads an answer this driver sent for its own reasons to its end, and throws
/// it away.
///
/// The span it opens is dropped with the stream on purpose: § 11's trace event
/// is one statement a *program* ran, and neither an eviction nor a reset is
/// one. Reading to the end is not optional — a TDS answer left on the wire is
/// read as the next statement's.
///
/// # Errors
///
/// As [`read_rows`] and [`TdsRows::next_row`].
fn drain<S: Read + Write>(wire: &mut Wire<S>, state: &Cell<State>) -> io::Result<()> {
    let mut rows = read_answer(wire, state, QuerySpan::opened(Driver::SqlServer, ""), None)?;
    while rows.next_row()?.is_some() {}
    Ok(())
}

/// The refusal for an answer the server stopped sending without ending it.
fn no_done() -> io::Error {
    malformed(String::from(
        "a TDS answer ended without the DONE that says which statement finished and what it \
         counted",
    ))
}

/// A length the protocol writes as a `u32` or a `u64`, as an index into memory.
///
/// Fallible because both are wider than a `usize` on a 32-bit host, and a
/// `PLP` chunk is four bytes of the server's choosing: the cast that cannot
/// fail on this project's targets is still the cast that truncates on one of
/// them, and a truncated length reads the next value as part of this one.
fn as_usize(length: impl TryInto<usize>) -> io::Result<usize> {
    length.try_into().map_err(|_| {
        malformed(String::from(
            "a TDS value declared a length this host cannot hold in memory",
        ))
    })
}

/// Where a failed read leaves the connection.
///
/// [`crate::mysql`]'s `poison_on_write` and its split, which its own doc owns: a
/// [`ServerError`] arrived whole and left the wire at a boundary, so the next
/// statement on it is fine, and anything else reached here with the stream in a
/// position nothing has proven.
fn poison_on_read(state: &Cell<State>, error: io::Error) -> io::Error {
    if ServerError::of(&error).is_some() {
        state.set(State::Idle);
    } else {
        state.set(State::Poisoned);
    }
    error
}

impl TdsConn {
    /// [ADR 0067 § 3](../../../docs/adr/0067-core-db.md)'s handshake end to
    /// end: a socket, PRELOGIN, the TLS session tunnelled inside it, LOGIN7 and
    /// the tokens that answer it.
    ///
    /// [`crate::MySqlConn::connect`]'s shape and its rules. `addr` is where to
    /// connect and `target.host` is only the name the certificate is checked
    /// against: resolving one to the other belongs to whoever checked
    /// `db.connect`, and a driver that re-resolved the name would be connecting
    /// somewhere nobody approved. `deadline` covers the whole handshake rather
    /// than each step, because what a caller bounds is how long opening a
    /// connection may take.
    ///
    /// Unlike the other three drivers this one sends nothing after the login.
    /// There is no charset to force — TDS carries text as UCS-2 and § 9's rows
    /// decode from that — and no session time zone to set, which
    /// [`TdsTarget::time_zone`] owns; LOGIN7's own option flags carry the ANSI
    /// defaults § 7's closure rests on, so the connection is usable the moment
    /// the login is acknowledged.
    ///
    /// # Errors
    ///
    /// [`prelogin`]'s `InvalidData` for a server that would leave the session
    /// in plaintext, [`negotiate_tls`]'s for a certificate that does not verify
    /// against `target.tls_ca_file`'s anchors, [`login`]'s for the login the
    /// server refused, `TimedOut` when the deadline passes, and whatever the
    /// socket itself reported.
    pub fn connect(
        addr: SocketAddr,
        target: &TdsTarget<'_>,
        deadline: Option<Instant>,
    ) -> io::Result<TdsConn> {
        let mut tcp = match deadline {
            Some(at) => {
                NvsTcp::connect_timeout(addr, at.saturating_duration_since(Instant::now()))?
            }
            None => NvsTcp::connect(addr)?,
        };
        tcp.set_deadline(deadline);

        let mut wire = negotiate_tls(Wire::new(tcp), target.host, target.tls_ca_file)?;
        login(&mut wire, target)?;

        Ok(TdsConn {
            wire,
            state: Cell::new(State::Idle),
            // § 9's zone-less row is decoded a layer up, where the target is
            // gone — see the field.
            time_zone: target.time_zone,
            // § 1's capacity is the `[db.<name>]` block's, already read by
            // `statement_cache_for` and carried here on the target.
            cache: StatementCache::new(target.statement_cache),
        })
    }

    /// [ADR 0067 § 13](../../../docs/adr/0067-core-db.md)'s reset, before this
    /// connection may be handed to another request.
    ///
    /// Takes `self` by value for [`crate::MySqlConn::reset`]'s reason: a reset
    /// that failed must not be able to hand a connection back, and a signature
    /// that borrowed would let a caller ignore the `Err` and pool it anyway.
    ///
    /// # Errors
    ///
    /// As [`reset_session`]. The connection is consumed either way.
    pub fn reset(mut self) -> io::Result<TdsConn> {
        reset_session(&mut self.wire, &self.state, &mut self.cache)?;
        Ok(self)
    }

    /// The zone a `datetime` or `datetime2` off this connection is read in, as
    /// seconds east of UTC.
    ///
    /// [`crate::MySqlConn::time_zone`]'s twin with one difference this method
    /// cannot show: no server was told. [`TdsTarget::time_zone`] owns why SQL
    /// Server has nowhere to be told.
    #[must_use]
    pub fn time_zone(&self) -> i32 {
        self.time_zone
    }

    /// The size this connection's messages are split at, which is what LOGIN7's
    /// answer settled.
    ///
    /// A server is free to answer a request for one size by using another, and
    /// the `ENVCHANGE` that says so is the only place it appears — so this is
    /// read off the framing rather than off the block that asked.
    #[must_use]
    pub fn packet_size(&self) -> u16 {
        self.wire.framing().packet_size()
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

    /// Where in LOGIN7's fixed header each offset/length pair sits, by the name
    /// MS-TDS gives it.
    ///
    /// `login7_request` writes the pairs in order and never names a position,
    /// which is the cheap way to build one and the expensive way to read one
    /// back: a pair written a slot out of place is a login the *server*
    /// refuses, with a message about the field it landed in. Asserting through
    /// this table is what makes that fail here instead.
    const PAIRS: [(&str, usize); 12] = [
        ("HostName", 36),
        ("UserName", 40),
        ("Password", 44),
        ("AppName", 48),
        ("ServerName", 52),
        ("Unused", 56),
        ("CltIntName", 60),
        ("Language", 64),
        ("Database", 68),
        // `ClientID`'s six bytes sit between these two.
        ("SSPI", 78),
        ("AtchDBFile", 82),
        ("ChangePassword", 86),
    ];

    /// The offset and the character count one pair holds.
    fn pair(message: &[u8], at: usize) -> (usize, usize) {
        (
            usize::from(u16::from_le_bytes([message[at], message[at + 1]])),
            usize::from(u16::from_le_bytes([message[at + 2], message[at + 3]])),
        )
    }

    /// The characters one pair points at, read back as a `String`.
    fn field_at(message: &[u8], at: usize) -> String {
        let (offset, characters) = pair(message, at);
        let units: Vec<u16> = message[offset..offset + characters * 2]
            .chunks_exact(2)
            .map(|two| u16::from_le_bytes([two[0], two[1]]))
            .collect();
        String::from_utf16(&units).expect("this driver writes back what it was handed")
    }

    /// The `[db.<name>]` block above, as the target LOGIN7 is built from.
    fn login7(block: &Database) -> Vec<u8> {
        let target = TdsTarget::resolve(block).expect("a complete block resolves");
        login7_request(&target, DEFAULT_PACKET_SIZE).expect("every field of it fits")
    }

    #[test]
    fn a_login7_carries_the_targets_four_fields_and_says_how_long_it_is() {
        let message = login7(&block());

        assert_eq!(
            usize::try_from(u32::from_le_bytes(
                message[0..4].try_into().expect("four bytes")
            ))
            .expect("a length this small"),
            message.len(),
            "the length field counts the whole message, itself included"
        );
        assert_eq!(message[4..8], TDS_VERSION.to_le_bytes());
        assert_eq!(message[8..12], u32::from(DEFAULT_PACKET_SIZE).to_le_bytes());

        assert_eq!(field_at(&message, 40), "sa");
        assert_eq!(
            field_at(&message, 52),
            "mssql.test",
            "ServerName is the host"
        );
        assert_eq!(field_at(&message, 68), "novis_test");
        assert_eq!(field_at(&message, 48), CLIENT_NAME);
        assert_eq!(field_at(&message, 60), CLIENT_NAME);

        for name in [
            "HostName",
            "Language",
            "SSPI",
            "AtchDBFile",
            "ChangePassword",
        ] {
            let at = PAIRS
                .iter()
                .find(|(field, _)| *field == name)
                .expect("the table names it")
                .1;
            assert_eq!(pair(&message, at).1, 0, "{name} goes out empty");
        }
    }

    /// Every pair points inside the message and past the header that holds it.
    ///
    /// Counted over the whole table rather than read off the fields that carry
    /// text: a pair whose offset is short by the six bytes of `ClientID` still
    /// points at plausible characters, and only the sweep says which one of the
    /// twelve moved.
    #[test]
    fn a_login7s_offsets_all_point_past_its_header_and_inside_the_message() {
        let message = login7(&block());

        assert_eq!(
            pair(&message, 36).0,
            usize::from(LOGIN7_HEADER),
            "the first blob starts where the fixed header ends"
        );
        for (name, at) in PAIRS {
            let (offset, characters) = pair(&message, at);
            assert!(
                offset >= usize::from(LOGIN7_HEADER),
                "{name} points into the header at {offset}"
            );
            assert!(
                offset + characters * 2 <= message.len(),
                "{name} spans past the end of a {}-byte message",
                message.len()
            );
        }
    }

    /// § 3's reason the TLS is not optional, asserted from the wire side.
    #[test]
    fn a_password_is_nibble_swapped_and_xored_and_is_nowhere_in_the_message_in_the_clear() {
        let message = login7(&block());
        let (offset, characters) = pair(&message, 44);
        let sent = &message[offset..offset + characters * 2];

        let plain: Vec<u8> = "hunter2"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        assert_eq!(characters, 7, "a length in characters, not in bytes");
        assert_ne!(sent, plain.as_slice());

        // The transform is its own inverse, which is the whole of what MS-TDS
        // calls encryption here.
        let back: Vec<u8> = sent
            .iter()
            .map(|byte| (byte ^ PASSWORD_XOR).rotate_left(4))
            .collect();
        assert_eq!(back, plain);

        for spelling in [plain.as_slice(), b"hunter2".as_slice()] {
            assert!(
                !message
                    .windows(spelling.len())
                    .any(|window| window == spelling),
                "the password appears in the message unobscured"
            );
        }
    }

    /// The option flags, and the one this slice had to decide: a failed initial
    /// database is fatal, so a login never lands in whichever database the
    /// server made this login's default.
    #[test]
    fn a_failed_initial_database_is_fatal_rather_than_a_warning() {
        let message = login7(&block());

        assert_eq!(
            message[24] & OPT1_INIT_DB_FATAL,
            OPT1_INIT_DB_FATAL,
            "a warning would make the connection's database server-side state"
        );
        assert_eq!(
            message[25] & OPT2_ODBC,
            OPT2_ODBC,
            "implicit transactions off is what makes § 7's closure the only transaction"
        );
        assert_eq!(message[24], OPTION_FLAGS_1);
        assert_eq!(message[25], OPTION_FLAGS_2);
        assert_eq!(message[26], TYPE_FLAGS);
        assert_eq!(message[27], OPTION_FLAGS_3);

        assert_eq!(
            message[28..32],
            0i32.to_le_bytes(),
            "the block declares +02:00 and § 9's zone still decodes rows rather than \
             configuring a session there is no setting for"
        );
    }

    /// Both sides of the field bound, and the units it counts in.
    #[test]
    fn a_field_one_character_past_the_protocols_limit_is_refused_by_its_key() {
        let mut block = block();
        let limit = usize::from(MAX_FIELD_CHARS);

        block.database = Some("d".repeat(limit));
        assert_eq!(pair(&login7(&block), 68).1, limit, "the last one that fits");

        // Half as many supplementary characters spend the same allowance: the
        // length is code units, and one of these is two of them.
        block.database = Some("🦀".repeat(limit / 2));
        assert_eq!(pair(&login7(&block), 68).1, limit);

        block.database = Some("d".repeat(limit + 1));
        let target = TdsTarget::resolve(&block).expect("an over-long name is still a name");
        let refused =
            login7_request(&target, DEFAULT_PACKET_SIZE).expect_err("one character past the limit");
        assert_eq!(refused.kind(), io::ErrorKind::InvalidInput);
        assert!(
            refused.to_string().contains("database"),
            "the refusal names the key an operator has to fix, not the protocol field"
        );
    }

    /// A `B_VARCHAR` as a server writes one.
    fn b_varchar(text: &str) -> Vec<u8> {
        let mut out = vec![u8::try_from(text.encode_utf16().count()).expect("a short field")];
        out.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
        out
    }

    /// A token, given its type byte and the body its `u16` length covers.
    fn token(kind: u8, body: &[u8]) -> Vec<u8> {
        let mut out = vec![kind];
        out.extend_from_slice(
            &u16::try_from(body.len())
                .expect("a short token")
                .to_le_bytes(),
        );
        out.extend_from_slice(body);
        out
    }

    /// An `ERROR` or `INFO` token as a server writes one.
    fn message_token(kind: u8, number: u32, class: u8, text: &str) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(&number.to_le_bytes());
        body.push(1);
        body.push(class);
        body.extend_from_slice(
            &u16::try_from(text.encode_utf16().count())
                .expect("a short sentence")
                .to_le_bytes(),
        );
        body.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
        body.extend_from_slice(&b_varchar("mssql.test"));
        body.extend_from_slice(&b_varchar(""));
        body.extend_from_slice(&7u32.to_le_bytes());
        token(kind, &body)
    }

    /// An `ENVCHANGE` token over two `B_VARCHAR` halves.
    fn env_token(kind: u8, to: &str, from: &str) -> Vec<u8> {
        let mut body = vec![kind];
        body.extend_from_slice(&b_varchar(to));
        body.extend_from_slice(&b_varchar(from));
        token(TOKEN_ENV_CHANGE, &body)
    }

    /// A `LOGINACK` as SQL Server 2022 writes one.
    fn login_ack_token() -> Vec<u8> {
        let mut body = vec![1];
        body.extend_from_slice(&TDS_VERSION.to_le_bytes());
        body.extend_from_slice(&b_varchar("Microsoft SQL Server"));
        body.extend_from_slice(&[16, 0]);
        body.extend_from_slice(&4035u16.to_be_bytes());
        token(TOKEN_LOGIN_ACK, &body)
    }

    /// A `DONE`, which carries no length at all.
    fn done_token(status: u16, rows: u64) -> Vec<u8> {
        let mut out = vec![TOKEN_DONE];
        out.extend_from_slice(&status.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&rows.to_le_bytes());
        out
    }

    /// A `DONEINPROC` or a `DONEPROC`, which are the same twelve bytes under
    /// another type byte.
    fn done_kind(kind: u8, status: u16, rows: u64) -> Vec<u8> {
        let mut out = done_token(status, rows);
        out[0] = kind;
        out
    }

    /// Every token of a message, or the refusal one of them was.
    fn tokens(payload: &[u8]) -> io::Result<Vec<Token>> {
        let mut reader = Tokens::over(payload);
        let mut out = Vec::new();
        while let Some(token) = reader.next_token()? {
            out.push(token);
        }
        Ok(out)
    }

    #[test]
    fn a_login_answer_reads_back_as_the_tokens_the_server_wrote() {
        let mut payload = env_token(ENV_DATABASE, "novis_test", "master");
        payload.extend_from_slice(&message_token(
            TOKEN_INFO,
            5701,
            0,
            "Changed database context to 'novis_test'.",
        ));
        payload.extend_from_slice(&env_token(ENV_PACKET_SIZE, "8192", "4096"));
        payload.extend_from_slice(&login_ack_token());
        payload.extend_from_slice(&done_token(DONE_COUNT, 0));

        let read = tokens(&payload).expect("a login answer this driver can read");
        assert_eq!(read.len(), 5);
        assert_eq!(
            read[0],
            Token::Env(EnvChange::Database {
                from: "master".to_owned(),
                to: "novis_test".to_owned(),
            }),
            "OptionFlags1's fUseDB is what asks for this token"
        );
        let Token::Info(info) = &read[1] else {
            panic!("the second token is an INFO");
        };
        assert_eq!(info.number, 5701);
        assert_eq!(info.server, "mssql.test");
        assert_eq!(info.line, 7);
        assert!(!info.is_fatal());
        assert_eq!(read[2], Token::Env(EnvChange::PacketSize { to: 8192 }));
        assert_eq!(
            read[3],
            Token::LoginAck(LoginAck {
                tds_version: TDS_VERSION,
                program: "Microsoft SQL Server".to_owned(),
                version: (16, 0, 4035),
            }),
            "the version LOGIN7 asked for is the one that comes back"
        );
        let Token::Done(done) = &read[4] else {
            panic!("the last token is a DONE");
        };
        assert!(done.counted() && !done.more() && !done.failed());
    }

    /// § 8's kinds, over the numbers this backend raises them as.
    ///
    /// One question of every row rather than a case each, so a table that grew
    /// a row meaning something else fails here: the numbers are the whole of
    /// what a caller branches on, since TDS sends no `SQLSTATE` to fall back to.
    #[test]
    fn a_sql_server_error_normalises_by_its_number_and_a_fatal_class_is_a_lost_connection() {
        for (number, kind) in [
            (2601, DbErrorKind::UniqueViolation),
            (2627, DbErrorKind::UniqueViolation),
            (547, DbErrorKind::ForeignKeyViolation),
            (515, DbErrorKind::NotNullViolation),
            (1205, DbErrorKind::Deadlock),
            (3960, DbErrorKind::SerializationFailure),
            (1222, DbErrorKind::Timeout),
            (208, DbErrorKind::Syntax),
            (4060, DbErrorKind::Permission),
            (18456, DbErrorKind::Permission),
            (8152, DbErrorKind::Other),
        ] {
            assert_eq!(kind_of(number, 16), kind, "error {number}");
        }

        assert!(
            kind_of(1205, 13).is_retryable() && kind_of(3960, 16).is_retryable(),
            "§ 7 re-runs a closure over these two and nothing else"
        );
        assert!(
            !kind_of(1222, 16).is_retryable(),
            "a lock timeout rolled nothing back, so the closure's earlier statements are still \
             in the transaction"
        );
        assert_eq!(
            kind_of(4001, FATAL_CLASS),
            DbErrorKind::ConnectionLost,
            "at this severity the server has already closed the socket"
        );
        assert_eq!(kind_of(4001, FATAL_CLASS - 1), DbErrorKind::Other);
    }

    #[test]
    fn an_error_token_carries_what_the_server_said_and_which_kind_it_is() {
        let payload = message_token(
            TOKEN_ERROR,
            2627,
            14,
            "Violation of PRIMARY KEY constraint 'PK_orders'.",
        );
        let read = tokens(&payload).expect("an error token is a token like any other");
        let [Token::Error(error)] = read.as_slice() else {
            panic!("one ERROR token");
        };
        assert_eq!(error.number, 2627);
        assert_eq!(error.state, 1);
        assert_eq!(error.kind(), DbErrorKind::UniqueViolation);
        assert!(error.message.contains("PK_orders"));
        assert!(
            !error.is_fatal(),
            "a constraint refuses a statement, not the connection"
        );
    }

    /// An unread `ENVCHANGE` type costs the reader nothing, because the token's
    /// own length is what skips it.
    #[test]
    fn an_envchange_this_driver_does_not_read_is_skipped_by_its_length() {
        // Type 8, `BEGIN TRANSACTION`, whose two halves are bytes rather than
        // characters — so a reader that walked the value halves would be
        // reading a transaction descriptor as a character count.
        let mut payload = token(TOKEN_ENV_CHANGE, &[8, 8, 1, 2, 3, 4, 5, 6, 7, 8, 0]);
        payload.extend_from_slice(&done_token(0, 0));

        let read = tokens(&payload).expect("an unread type is not a malformed message");
        assert_eq!(read[0], Token::Env(EnvChange::Other(8)));
        assert_eq!(read.len(), 2, "the DONE after it is still found");
    }

    /// A field or a token that runs off the end says so, rather than reading
    /// whatever followed it.
    #[test]
    fn a_token_that_runs_past_the_end_of_the_message_is_refused() {
        let whole = message_token(TOKEN_ERROR, 2627, 14, "Violation");
        for cut in 1..whole.len() {
            let refused = tokens(&whole[..cut]).expect_err("half a token is not a token");
            assert_eq!(refused.kind(), io::ErrorKind::InvalidData, "cut at {cut}");
        }

        // A length that promises more than the message holds is the same
        // refusal one step earlier, and it is the one a `take` per field would
        // miss: every field of this token is present.
        let mut lying = whole.clone();
        lying[1] = 0xFF;
        let refused = tokens(&lying).expect_err("a token cannot be longer than its message");
        assert_eq!(refused.kind(), io::ErrorKind::InvalidData);

        let refused = tokens(&[0xA9, 0, 0]).expect_err("ORDER is a token nothing here reads");
        assert!(
            refused.to_string().contains("0xA9"),
            "an unread token names the byte, which is the whole of what it can say"
        );
    }

    /// The packet size arrives as digits, and a server that wrote something
    /// else is not answered with a plausible size.
    #[test]
    fn a_packet_size_envchange_is_the_number_the_server_wrote() {
        let read = tokens(&env_token(ENV_PACKET_SIZE, "16384", "4096"))
            .expect("digits are what this token carries");
        assert_eq!(read[0], Token::Env(EnvChange::PacketSize { to: 16384 }));

        let refused = tokens(&env_token(ENV_PACKET_SIZE, "large", "4096"))
            .expect_err("a size that is not a number");
        assert_eq!(refused.kind(), io::ErrorKind::InvalidData);
    }

    /// One `TabularResult` message carrying `payload`, as a server writes it.
    fn answer(payload: &[u8]) -> Vec<u8> {
        packet(PacketType::TabularResult, Status::EOM, 1, payload)
    }

    /// A wire over a server that answers one login and says nothing else.
    fn logging_in(payload: &[u8]) -> Wire<Script> {
        Wire::new(Script::answering(answer(payload), READ_CHUNK))
    }

    #[test]
    fn a_login_sends_login7_and_the_servers_answer_settles_the_framing() {
        let block = block();
        let target = TdsTarget::resolve(&block).expect("a complete block resolves");

        let mut payload = env_token(ENV_DATABASE, "novis_test", "master");
        payload.extend_from_slice(&env_token(ENV_PACKET_SIZE, "8192", "4096"));
        payload.extend_from_slice(&login_ack_token());
        payload.extend_from_slice(&done_token(0, 0));
        let mut wire = logging_in(&payload);

        let ack = login(&mut wire, &target).expect("a login this server accepted");
        assert_eq!(ack.tds_version, TDS_VERSION);
        assert_eq!(ack.version, (16, 0, 4035));

        assert_eq!(
            wire.peer().sent,
            packet(PacketType::Login7, Status::EOM, 1, &login7(&block)),
            "what went out is the message `login7_request` built, framed whole"
        );
        assert_eq!(
            wire.framing().packet_size(),
            8192,
            "the ENVCHANGE is the only place a server says what size it will \
             actually use, so the framing follows it rather than what LOGIN7 asked"
        );
    }

    /// § 8's four fields for a refusal this backend words, all of them at once.
    ///
    /// Both absences are asserted here rather than left to a reader's
    /// assumption: TDS sends no `SQLSTATE`, and no constraint name outside the
    /// sentence. A driver that filled either from ODBC's table or from the
    /// message text would still answer the right kind and would fail here.
    #[test]
    fn a_refused_login_is_the_servers_own_refusal_with_its_number_and_no_sqlstate() {
        let block = block();
        let target = TdsTarget::resolve(&block).expect("a complete block resolves");

        let mut payload = message_token(TOKEN_ERROR, 18456, 14, "Login failed for user 'novis'.");
        payload.extend_from_slice(&done_token(DONE_ERROR, 0));
        let mut wire = logging_in(&payload);

        let refused = login(&mut wire, &target).expect_err("this login was refused");
        let server = ServerError::of(&refused).expect("a refusal the server worded");
        assert_eq!(server.kind, DbErrorKind::Permission);
        assert_eq!(
            server.driver_code,
            Some(18456),
            "the number is the whole of what a caller branches on past § 8's kind"
        );
        assert_eq!(server.sql_state, "", "TDS has no such field to fill");
        assert_eq!(server.severity, "ERROR");
        assert_eq!(server.constraint, None);
        assert_eq!(
            refused.to_string(),
            "sqlserver ERROR: Login failed for user 'novis'.",
            "an absent SQLSTATE is omitted from the sentence, not rendered empty"
        );

        // A `THROW` past a `u16`, which is the width this field used to be and
        // the reason it is not any more: the whole user-defined range is above
        // it, so a narrower field would answer `None` for every error an
        // application raised itself.
        let mut raised = message_token(TOKEN_ERROR, 90_001, 16, "the application said no");
        raised.extend_from_slice(&done_token(DONE_ERROR, 0));
        let mut wire = logging_in(&raised);
        let refused = login(&mut wire, &target).expect_err("this login was refused");
        let server = ServerError::of(&refused).expect("a refusal the server worded");
        assert_eq!(server.driver_code, Some(90_001));
        assert_eq!(server.kind, DbErrorKind::Other);

        // At `FATAL_CLASS` the server has already closed the socket, so § 13's
        // pool must destroy this connection rather than reset it — which is
        // what the kind says and what the severity has to agree with.
        let mut fatal = message_token(TOKEN_ERROR, 9001, 21, "the log is not available");
        fatal.extend_from_slice(&done_token(DONE_ERROR, 0));
        let mut wire = logging_in(&fatal);
        let refused = login(&mut wire, &target).expect_err("this login was refused");
        let server = ServerError::of(&refused).expect("a refusal the server worded");
        assert_eq!(server.kind, DbErrorKind::ConnectionLost);
        assert_eq!(server.severity, "FATAL");
    }

    /// The three answers that are neither an acceptance nor a refusal.
    ///
    /// Each is a stream this driver could read *something* out of and must not:
    /// a login that ended with no verdict, a server speaking another dialect,
    /// and an answer that is not a token stream at all. The first is the one a
    /// reader trusting `LOGINACK`'s absence to mean failure would get wrong.
    #[test]
    fn a_login_answer_that_decides_nothing_is_refused_rather_than_assumed() {
        let block = block();
        let target = TdsTarget::resolve(&block).expect("a complete block resolves");

        let mut nothing = message_token(TOKEN_INFO, 5701, 0, "Changed database context.");
        nothing.extend_from_slice(&done_token(0, 0));
        let mut wire = logging_in(&nothing);
        let refused = login(&mut wire, &target).expect_err("nothing here accepted the login");
        assert_eq!(refused.kind(), io::ErrorKind::InvalidData);
        assert!(refused.to_string().contains("LOGINACK"), "{refused}");
        assert!(
            ServerError::of(&refused).is_none(),
            "the server refused nothing, so this is a protocol failure and not \
             a condition § 8 normalises"
        );

        // TDS 7.3, which a server before SQL Server 2012 answers with. Its
        // token stream is close enough to read wrongly and not close enough to
        // read right.
        // The version is the four bytes after the token byte, its length and
        // the interface byte — `login_ack_token` is what says so.
        let mut older = login_ack_token();
        older[4..8].copy_from_slice(&0x730B_0003u32.to_le_bytes());
        older.extend_from_slice(&done_token(0, 0));
        let mut wire = logging_in(&older);
        let refused = login(&mut wire, &target).expect_err("this driver speaks 7.4 alone");
        assert!(refused.to_string().contains("0x730B0003"), "{refused}");

        // The server answering a LOGIN7 with a PRELOGIN: the stream is out of
        // sync, and the tokens would be read out of whatever arrived instead.
        let mut wire = Wire::new(Script::answering(
            packet(PacketType::PreLogin, Status::EOM, 1, &[PL_TERMINATOR]),
            READ_CHUNK,
        ));
        let refused = login(&mut wire, &target).expect_err("0x12 answers nothing here");
        assert!(refused.to_string().contains("type 0x12"), "{refused}");
    }

    /// A `US_VARCHAR` as a server writes one.
    fn us_varchar(text: &str) -> Vec<u8> {
        let mut out = u16::try_from(text.encode_utf16().count())
            .expect("a short field")
            .to_le_bytes()
            .to_vec();
        out.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
        out
    }

    /// A `COLLATION` as a server writes one — `Latin1_General_CI_AS` — which
    /// this driver keeps and does not read apart.
    const COLLATION: [u8; 5] = [0x09, 0x04, 0xD0, 0x00, 0x34];

    /// The declared maximum of every large-object type.
    const MAX_LOB: u32 = 0x7FFF_FFFF;

    /// One column of a `COLMETADATA`, from its `TYPE_INFO` outwards. A `text`,
    /// `ntext` or `image` column's `TableName` belongs on the end of
    /// `type_info`, which is where the wire puts it.
    fn column(type_info: &[u8], name: &str, flags: u16) -> Vec<u8> {
        let mut out = 0u32.to_le_bytes().to_vec();
        out.extend_from_slice(&flags.to_le_bytes());
        out.extend_from_slice(type_info);
        out.extend_from_slice(&b_varchar(name));
        out
    }

    /// A `COLMETADATA` token over the columns `column` built.
    fn col_metadata(columns: &[Vec<u8>]) -> Vec<u8> {
        let mut out = vec![TOKEN_COL_METADATA];
        out.extend_from_slice(
            &u16::try_from(columns.len())
                .expect("a few columns")
                .to_le_bytes(),
        );
        for one in columns {
            out.extend_from_slice(one);
        }
        out
    }

    /// A `USHORTLEN` character type's `TYPE_INFO`, which carries a collation.
    fn char_type(id: u8, declared: u16) -> Vec<u8> {
        let mut out = vec![id];
        out.extend_from_slice(&declared.to_le_bytes());
        out.extend_from_slice(&COLLATION);
        out
    }

    /// A `USHORTLEN` binary type's, which does not.
    fn binary_type(id: u8, declared: u16) -> Vec<u8> {
        let mut out = vec![id];
        out.extend_from_slice(&declared.to_le_bytes());
        out
    }

    /// A `LONGLEN` type's, with the collation and the `TableName` for the three
    /// types that carry them.
    fn long_type(id: u8, declared: u32) -> Vec<u8> {
        let mut out = vec![id];
        out.extend_from_slice(&declared.to_le_bytes());
        if matches!(id, TY_TEXT | TY_NTEXT) {
            out.extend_from_slice(&COLLATION);
        }
        if matches!(id, TY_TEXT | TY_NTEXT | TY_IMAGE) {
            out.push(1);
            out.extend_from_slice(&us_varchar("notes"));
        }
        out
    }

    /// A CLR type's `TYPE_INFO`: `geometry`, as SQL Server describes one.
    fn udt_type() -> Vec<u8> {
        let mut out = vec![TY_UDT];
        out.extend_from_slice(&NO_LENGTH.to_le_bytes());
        out.extend_from_slice(&b_varchar("novis_test"));
        out.extend_from_slice(&b_varchar("sys"));
        out.extend_from_slice(&b_varchar("geometry"));
        out.extend_from_slice(&us_varchar(
            "Microsoft.SqlServer.Types.SqlGeometry, Microsoft.SqlServer.Types",
        ));
        out
    }

    /// The one column a `TYPE_INFO` describes, or the refusal reading it was.
    fn one_column(type_info: &[u8]) -> io::Result<TdsColumn> {
        let payload = col_metadata(&[column(type_info, "c", COLUMN_NULLABLE)]);
        let mut read = tokens(&payload)?;
        let Some(Token::Columns(mut columns)) = read.pop() else {
            panic!("a COLMETADATA reads back as one");
        };
        Ok(columns.pop().expect("the one column that was described"))
    }

    /// A `TYPE_INFO` for every type byte this driver reads, with the length
    /// family MS-TDS gives it and the § 9 row it classifies as. The two sweeps
    /// below share it, which is what makes them ask about one table.
    fn every_type_info() -> Vec<(Vec<u8>, Length, ColumnType)> {
        vec![
            (vec![TY_NULL], Length::Fixed(0), ColumnType::Other),
            (vec![TY_INT1], Length::Fixed(1), ColumnType::Int),
            (vec![TY_INT2], Length::Fixed(2), ColumnType::Int),
            (vec![TY_INT4], Length::Fixed(4), ColumnType::Int),
            (vec![TY_INT8], Length::Fixed(8), ColumnType::Int),
            (vec![TY_INTN, 4], Length::Byte(4), ColumnType::Int),
            (vec![TY_BIT], Length::Fixed(1), ColumnType::Bool),
            (vec![TY_BITN, 1], Length::Byte(1), ColumnType::Bool),
            (vec![TY_FLT4], Length::Fixed(4), ColumnType::Float),
            (vec![TY_FLT8], Length::Fixed(8), ColumnType::Float),
            (vec![TY_FLTN, 8], Length::Byte(8), ColumnType::Float),
            (vec![TY_MONEY], Length::Fixed(8), ColumnType::Decimal),
            (vec![TY_MONEY4], Length::Fixed(4), ColumnType::Decimal),
            (vec![TY_MONEYN, 8], Length::Byte(8), ColumnType::Decimal),
            (
                vec![TY_DECIMALN, 17, 38, 4],
                Length::Byte(17),
                ColumnType::Decimal,
            ),
            (
                vec![TY_NUMERICN, 9, 18, 2],
                Length::Byte(9),
                ColumnType::Decimal,
            ),
            (
                char_type(TY_BIGCHAR, 10),
                Length::Short(10),
                ColumnType::Text,
            ),
            (
                char_type(TY_BIGVARCHAR, 8000),
                Length::Short(8000),
                ColumnType::Text,
            ),
            (
                char_type(TY_BIGVARCHAR, NO_LENGTH),
                Length::Partial,
                ColumnType::Text,
            ),
            (char_type(TY_NCHAR, 20), Length::Short(20), ColumnType::Text),
            (
                char_type(TY_NVARCHAR, 100),
                Length::Short(100),
                ColumnType::Text,
            ),
            (
                char_type(TY_NVARCHAR, NO_LENGTH),
                Length::Partial,
                ColumnType::Text,
            ),
            (
                binary_type(TY_BIGBINARY, 16),
                Length::Short(16),
                ColumnType::Bytes,
            ),
            (
                binary_type(TY_BIGVARBINARY, 900),
                Length::Short(900),
                ColumnType::Bytes,
            ),
            (
                binary_type(TY_BIGVARBINARY, NO_LENGTH),
                Length::Partial,
                ColumnType::Bytes,
            ),
            (
                long_type(TY_TEXT, MAX_LOB),
                Length::Long(MAX_LOB),
                ColumnType::Text,
            ),
            (
                long_type(TY_NTEXT, MAX_LOB),
                Length::Long(MAX_LOB),
                ColumnType::Text,
            ),
            (
                long_type(TY_IMAGE, MAX_LOB),
                Length::Long(MAX_LOB),
                ColumnType::Bytes,
            ),
            (
                long_type(TY_VARIANT, 8009),
                Length::Long(8009),
                ColumnType::Other,
            ),
            (vec![TY_GUID, 16], Length::Byte(16), ColumnType::Uuid),
            (vec![TY_DATEN], Length::Byte(3), ColumnType::Date),
            (vec![TY_TIMEN, 7], Length::Byte(5), ColumnType::Time),
            (vec![TY_DATETIME4], Length::Fixed(4), ColumnType::DateTime),
            (vec![TY_DATETIME], Length::Fixed(8), ColumnType::DateTime),
            (vec![TY_DATETIMEN, 8], Length::Byte(8), ColumnType::DateTime),
            (
                vec![TY_DATETIME2N, 7],
                Length::Byte(8),
                ColumnType::DateTime,
            ),
            (
                vec![TY_DATETIMEOFFSETN, 7],
                Length::Byte(10),
                ColumnType::Instant,
            ),
            (vec![TY_XML, 0], Length::Partial, ColumnType::Other),
            (udt_type(), Length::Partial, ColumnType::Other),
        ]
    }

    /// Every type this driver reads is measured the way MS-TDS § 2.2.5.4.1
    /// says, and the count is asserted rather than the rows: a type byte that
    /// gained an arm without gaining a row here fails the last line.
    #[test]
    fn every_type_info_is_measured_the_way_ms_tds_measures_it() {
        let table = every_type_info();
        for (bytes, length, _) in &table {
            let column = one_column(bytes)
                .unwrap_or_else(|refused| panic!("type 0x{:02X}: {refused}", bytes[0]));
            assert_eq!(column.type_info.length, *length, "type 0x{:02X}", bytes[0]);
            assert_eq!(column.type_info.id, bytes[0]);
            assert_eq!(
                column.name, "c",
                "the name is after the TYPE_INFO, so a type read short takes the name with it"
            );
        }

        for family in [
            Length::Fixed(0),
            Length::Byte(0),
            Length::Short(0),
            Length::Long(0),
            Length::Partial,
        ] {
            let sort = std::mem::discriminant(&family);
            assert!(
                table
                    .iter()
                    .any(|(_, length, _)| std::mem::discriminant(length) == sort),
                "no type in the table is measured as {family:?}, so that family is untested"
            );
        }

        let ids: std::collections::HashSet<u8> =
            table.iter().map(|(bytes, _, _)| bytes[0]).collect();
        assert_eq!(
            ids.len(),
            36,
            "every type byte `Tokens::type_info` has an arm for is one row here"
        );
    }

    /// § 9's rows this backend can reach, asserted as a set — so a type that
    /// classified as a plausible neighbour fails here even where its own row
    /// still passes.
    #[test]
    fn every_column_type_this_backend_reaches_is_a_row_of_section_nine() {
        let mut seen: std::collections::HashSet<ColumnType> = std::collections::HashSet::new();
        for (bytes, _, expected) in &every_type_info() {
            let column = one_column(bytes)
                .unwrap_or_else(|refused| panic!("type 0x{:02X}: {refused}", bytes[0]));
            assert_eq!(column.column_type(), *expected, "type 0x{:02X}", bytes[0]);
            seen.insert(*expected);
        }

        assert_eq!(seen.len(), 12, "twelve of § 9's fourteen rows, and these:");
        assert!(
            !seen.contains(&ColumnType::Uint),
            "`tinyint` is the only unsigned integer here and it fits an `int`"
        );
        assert!(
            !seen.contains(&ColumnType::Json),
            "SQL Server stores JSON in an `nvarchar`, so a JSON column is a text column"
        );
    }

    /// A result set's shape is its columns in the order their values will
    /// arrive, and the fields around the `TYPE_INFO` come back with them.
    #[test]
    fn a_colmetadata_describes_its_columns_in_the_order_the_values_arrive() {
        let mut payload = col_metadata(&[
            column(&[TY_INT4], "id", 0),
            column(&char_type(TY_NVARCHAR, 100), "name", COLUMN_NULLABLE),
            column(&long_type(TY_TEXT, MAX_LOB), "note", COLUMN_NULLABLE),
        ]);
        payload.extend_from_slice(&done_token(DONE_COUNT, 3));

        let read = tokens(&payload).expect("a result set this driver can describe");
        assert_eq!(read.len(), 2, "the DONE after the columns is still found");
        let Token::Columns(columns) = &read[0] else {
            panic!("the first token describes the columns");
        };
        assert_eq!(
            columns.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
            ["id", "name", "note"],
            "`note` is the one that proves the TableName is skipped: a reader that \
             did not would name the column after the table"
        );
        assert!(!columns[0].nullable(), "the server said this one cannot be");
        assert!(columns[1].nullable());
        assert_eq!(columns[1].type_info.collation, Some(COLLATION));
        assert_eq!(
            columns[0].type_info.collation, None,
            "an `int` has no collation to carry"
        );
        assert_eq!(columns[2].column_type(), ColumnType::Text);
    }

    /// TDS 4.2's spellings are refused by their byte rather than parsed on a
    /// guess at a layout a 7.4 server never sends.
    #[test]
    fn a_column_type_this_driver_does_not_read_is_named_by_its_byte() {
        for legacy in [0x2Fu8, 0x27, 0x2D, 0x25, 0x37, 0x3F] {
            let refused = one_column(&[legacy, 8]).expect_err("a 4.2 type is not read here");
            assert_eq!(refused.kind(), io::ErrorKind::InvalidData);
            assert!(
                refused.to_string().contains(&format!("0x{legacy:02X}")),
                "{refused}"
            );
        }
    }

    /// Both sides of the bound: seven fractional-second digits is the widest
    /// `time` SQL Server has, and eight is not a narrower one.
    #[test]
    fn a_time_scale_is_read_up_to_seven_digits_and_refused_past_them() {
        for (scale, width) in [(0u8, 3u8), (2, 3), (3, 4), (4, 4), (5, 5), (7, 5)] {
            let column = one_column(&[TY_TIMEN, scale]).expect("a scale SQL Server has");
            assert_eq!(
                column.type_info.length,
                Length::Byte(width),
                "scale {scale}"
            );
            assert_eq!(column.type_info.scale, scale);
        }

        let refused = one_column(&[TY_DATETIME2N, 8]).expect_err("eight digits is not a time");
        assert_eq!(refused.kind(), io::ErrorKind::InvalidData);
        assert!(refused.to_string().contains("scale of 8"), "{refused}");
    }

    /// `0xFFFF` columns is the server saying the statement had no result set at
    /// all, and it is not a count that reads the tokens after it as columns.
    #[test]
    fn a_statement_with_no_result_set_answers_no_columns() {
        let mut payload = vec![TOKEN_COL_METADATA];
        payload.extend_from_slice(&NO_LENGTH.to_le_bytes());
        payload.extend_from_slice(&done_token(DONE_COUNT, 1));

        let read = tokens(&payload).expect("no metadata is not malformed metadata");
        assert_eq!(read[0], Token::Columns(Vec::new()));
        assert_eq!(read.len(), 2, "the DONE is the next token, not a column");
    }

    /// `COLMETADATA` carries no length of its own, so the fields running out is
    /// the only thing between a lying count and a reader walking into the
    /// tokens after it.
    #[test]
    fn a_colmetadata_that_promises_more_columns_than_it_holds_is_refused() {
        let mut payload = vec![TOKEN_COL_METADATA];
        payload.extend_from_slice(&3u16.to_le_bytes());
        payload.extend_from_slice(&column(&[TY_INT4], "id", 0));
        payload.extend_from_slice(&done_token(0, 0));

        let refused = tokens(&payload).expect_err("three columns were promised and one sent");
        assert_eq!(refused.kind(), io::ErrorKind::InvalidData);
    }

    // ---- `ROW`, `NBCROW` and PLP over packets: ADR 0067 §§ 4 and 9 ----------

    /// A wire over a server that answers with these packets in order, only the
    /// last of them ending the message.
    fn answering(packets: &[Vec<u8>]) -> Wire<Script> {
        let mut inbound = Vec::new();
        for (index, payload) in packets.iter().enumerate() {
            let last = index + 1 == packets.len();
            inbound.extend_from_slice(&packet(
                PacketType::TabularResult,
                if last { Status::EOM } else { Status::NORMAL },
                u8::try_from(index + 1).expect("a few packets"),
                payload,
            ));
        }
        Wire::new(Script::answering(inbound, READ_CHUNK))
    }

    /// One payload cut into packets of `at` bytes, which is what a real server
    /// does at whatever offset its packet size lands on.
    fn chunked(payload: &[u8], at: usize) -> Vec<Vec<u8>> {
        payload.chunks(at).map(<[u8]>::to_vec).collect()
    }

    /// The span every case here opens, since none of them is about § 11.
    fn span() -> QuerySpan {
        QuerySpan::opened(Driver::SqlServer, "select 1")
    }

    /// A `ROW` token over values already written the way their columns measure
    /// them.
    fn row_token(values: &[Vec<u8>]) -> Vec<u8> {
        let mut out = vec![TOKEN_ROW];
        for value in values {
            out.extend_from_slice(value);
        }
        out
    }

    /// An `NBCROW`: the null bitmap `nulls` describes, then the values of the
    /// columns it left off.
    fn nbc_row_token(nulls: &[bool], values: &[Vec<u8>]) -> Vec<u8> {
        let mut out = vec![TOKEN_NBC_ROW];
        let mut bitmap = vec![0u8; nulls.len().div_ceil(8)];
        for (index, null) in nulls.iter().enumerate() {
            if *null {
                bitmap[index / 8] |= 1 << (index % 8);
            }
        }
        out.extend_from_slice(&bitmap);
        for value in values {
            out.extend_from_slice(value);
        }
        out
    }

    /// A `BYTELEN` value, and its `NULL` at a length of zero.
    fn byte_value(bytes: &[u8]) -> Vec<u8> {
        let mut out = vec![u8::try_from(bytes.len()).expect("a short value")];
        out.extend_from_slice(bytes);
        out
    }

    /// A `USHORTLEN` value.
    fn short_value(bytes: &[u8]) -> Vec<u8> {
        let mut out = u16::try_from(bytes.len())
            .expect("a short value")
            .to_le_bytes()
            .to_vec();
        out.extend_from_slice(bytes);
        out
    }

    /// A `text`, `ntext` or `image` value: a text pointer, the row timestamp
    /// nothing reads, and then the length.
    fn long_value(bytes: &[u8]) -> Vec<u8> {
        let mut out = vec![2, 0xAB, 0xCD];
        out.extend_from_slice(&[0; TEXT_TIMESTAMP]);
        out.extend_from_slice(
            &u32::try_from(bytes.len())
                .expect("a short value")
                .to_le_bytes(),
        );
        out.extend_from_slice(bytes);
        out
    }

    /// A `PLP` value: a declared total, the chunks, and the empty one that ends
    /// them.
    ///
    /// A `NULL` is the sentinel alone — MS-TDS § 2.2.5.2.3 gives it no chunks to
    /// terminate — which is exactly the four bytes a reader that returned early
    /// would leave on the wire.
    fn plp_value(total: u64, chunks: &[&[u8]]) -> Vec<u8> {
        let mut out = total.to_le_bytes().to_vec();
        if total == PLP_NULL {
            return out;
        }
        for chunk in chunks {
            out.extend_from_slice(
                &u32::try_from(chunk.len())
                    .expect("a short chunk")
                    .to_le_bytes(),
            );
            out.extend_from_slice(chunk);
        }
        out.extend_from_slice(&0u32.to_le_bytes());
        out
    }

    /// The four columns the cases below read rows against, one per length
    /// family: `int`, `varchar(10)`, a nullable `int` and `text`.
    fn four_columns() -> Vec<u8> {
        col_metadata(&[
            column(&[TY_INT4], "id", 0),
            column(&char_type(TY_BIGVARCHAR, 10), "name", COLUMN_NULLABLE),
            column(&[TY_INTN, 4], "score", COLUMN_NULLABLE),
            column(&long_type(TY_TEXT, MAX_LOB), "notes", COLUMN_NULLABLE),
        ])
    }

    /// One row of [`four_columns`], with the third column null.
    fn four_values() -> Vec<u8> {
        row_token(&[
            7i32.to_le_bytes().to_vec(),
            short_value(b"abc"),
            byte_value(&[]),
            long_value(b"hello"),
        ])
    }

    /// Every row of a stream, or the refusal it ended with.
    fn drain(rows: &mut TdsRows<'_, Script>) -> io::Result<Vec<TdsRow>> {
        let mut out = Vec::new();
        while let Some(row) = rows.next_row()? {
            out.push(row);
        }
        Ok(out)
    }

    #[test]
    fn a_row_is_read_back_as_the_values_its_columns_measure() {
        let mut payload = four_columns();
        payload.extend_from_slice(&four_values());
        payload.extend_from_slice(&done_token(DONE_COUNT, 1));

        let mut wire = answering(&[payload]);
        let state = Cell::new(State::Executing);
        let mut rows = read_rows(&mut wire, &state, span()).expect("a described result set");
        assert_eq!(rows.columns().len(), 4);
        assert_eq!(rows.column_type(0), Some(ColumnType::Int));
        assert_eq!(rows.column_type(4), None);
        assert_eq!(rows.affected(), None, "the stream has not ended yet");

        let read = drain(&mut rows).expect("one row and a DONE");
        assert_eq!(read.len(), 1);
        let row = &read[0];
        assert_eq!(row.len(), 4);
        assert_eq!(row.column(0), Some(Some(&7i32.to_le_bytes()[..])));
        assert_eq!(row.column(1), Some(Some(&b"abc"[..])));
        assert_eq!(
            row.column(2),
            Some(None),
            "a BYTELEN length of zero is NULL"
        );
        assert_eq!(row.column(3), Some(Some(&b"hello"[..])));
        assert_eq!(row.column(4), None, "the row has four columns");

        // A result set's `affected` is the rows that came back, which is the
        // number `crate::MySqlRows::affected` answers for the same statement.
        assert_eq!(rows.affected(), Some(1));
        assert_eq!(rows.span().rows(), 1);
        assert_eq!(state.get(), State::Idle);
    }

    #[test]
    fn a_row_cut_at_any_offset_is_read_back_as_one_row() {
        let mut payload = four_columns();
        payload.extend_from_slice(&four_values());
        payload.extend_from_slice(&done_token(DONE_COUNT, 1));

        // Three bytes a packet cuts every field of every token, the COLMETADATA
        // and the DONE included — the case a reader holding no remainder of its
        // own cannot answer at all.
        let mut wire = answering(&chunked(&payload, 3));
        let state = Cell::new(State::Executing);
        let mut rows = read_rows(&mut wire, &state, span()).expect("a described result set");
        let read = drain(&mut rows).expect("one row and a DONE");

        assert_eq!(read.len(), 1);
        assert_eq!(read[0].column(0), Some(Some(&7i32.to_le_bytes()[..])));
        assert_eq!(read[0].column(1), Some(Some(&b"abc"[..])));
        assert_eq!(read[0].column(3), Some(Some(&b"hello"[..])));
        assert_eq!(state.get(), State::Idle);
    }

    #[test]
    fn an_nbcrows_bitmap_is_the_null_of_a_column_that_sent_no_bytes() {
        let mut payload = four_columns();
        // Columns 1 and 3 are in the bitmap and carry nothing at all, where the
        // ROW form would have sent each of them a length.
        payload.extend_from_slice(&nbc_row_token(
            &[false, true, false, true],
            &[7i32.to_le_bytes().to_vec(), byte_value(&[9, 0, 0, 0])],
        ));
        payload.extend_from_slice(&done_token(DONE_COUNT, 1));

        let mut wire = answering(&[payload]);
        let state = Cell::new(State::Executing);
        let mut rows = read_rows(&mut wire, &state, span()).expect("a described result set");
        let read = drain(&mut rows).expect("one row and a DONE");

        assert_eq!(read[0].column(0), Some(Some(&7i32.to_le_bytes()[..])));
        assert_eq!(read[0].column(1), Some(None));
        assert_eq!(read[0].column(2), Some(Some(&[9, 0, 0, 0][..])));
        assert_eq!(read[0].column(3), Some(None));
        assert_eq!(state.get(), State::Idle);
    }

    #[test]
    fn a_plp_value_is_its_chunks_joined_and_the_empty_one_ends_them() {
        let columns = col_metadata(&[
            column(
                &char_type(TY_BIGVARCHAR, NO_LENGTH),
                "known",
                COLUMN_NULLABLE,
            ),
            column(
                &binary_type(TY_BIGVARBINARY, NO_LENGTH),
                "unknown",
                COLUMN_NULLABLE,
            ),
            column(&char_type(TY_NVARCHAR, NO_LENGTH), "empty", COLUMN_NULLABLE),
            column(
                &binary_type(TY_BIGVARBINARY, NO_LENGTH),
                "absent",
                COLUMN_NULLABLE,
            ),
        ]);
        let mut payload = columns;
        payload.extend_from_slice(&row_token(&[
            plp_value(6, &[b"abc", b"def"]),
            plp_value(PLP_UNKNOWN, &[b"one", b"two", b"three"]),
            plp_value(0, &[]),
            plp_value(PLP_NULL, &[]),
        ]));
        payload.extend_from_slice(&done_token(DONE_COUNT, 1));

        let mut wire = answering(&chunked(&payload, 7));
        let state = Cell::new(State::Executing);
        let mut rows = read_rows(&mut wire, &state, span()).expect("a described result set");
        let read = drain(&mut rows).expect("one row and a DONE");

        assert_eq!(read[0].column(0), Some(Some(&b"abcdef"[..])));
        assert_eq!(read[0].column(1), Some(Some(&b"onetwothree"[..])));
        assert_eq!(
            read[0].column(2),
            Some(Some(&b""[..])),
            "a value of no bytes is not a value that was not there"
        );
        assert_eq!(read[0].column(3), Some(None));
        assert_eq!(state.get(), State::Idle);
    }

    #[test]
    fn a_plp_value_that_disagrees_with_its_declared_total_is_refused() {
        let mut payload = col_metadata(&[column(
            &char_type(TY_BIGVARCHAR, NO_LENGTH),
            "text",
            COLUMN_NULLABLE,
        )]);
        payload.extend_from_slice(&row_token(&[plp_value(9, &[b"abc"])]));
        payload.extend_from_slice(&done_token(DONE_COUNT, 1));

        let mut wire = answering(&[payload]);
        let state = Cell::new(State::Executing);
        let mut rows = read_rows(&mut wire, &state, span()).expect("a described result set");
        let refused = drain(&mut rows).expect_err("nine bytes were promised and three sent");

        assert_eq!(refused.kind(), io::ErrorKind::InvalidData);
        assert_eq!(state.get(), State::Poisoned);
    }

    #[test]
    fn a_nulltype_column_is_null_and_carries_nothing() {
        let mut payload = col_metadata(&[
            column(&[TY_NULL], "nothing", COLUMN_NULLABLE),
            column(&[TY_INT4], "id", 0),
        ]);
        payload.extend_from_slice(&row_token(&[Vec::new(), 7i32.to_le_bytes().to_vec()]));
        payload.extend_from_slice(&done_token(DONE_COUNT, 1));

        let mut wire = answering(&[payload]);
        let state = Cell::new(State::Executing);
        let mut rows = read_rows(&mut wire, &state, span()).expect("a described result set");
        let read = drain(&mut rows).expect("one row and a DONE");

        assert_eq!(read[0].column(0), Some(None));
        assert_eq!(read[0].column(1), Some(Some(&7i32.to_le_bytes()[..])));
    }

    #[test]
    fn a_statement_with_no_result_set_is_already_ended_and_reports_what_it_counted() {
        let mut payload = env_token(ENV_DATABASE, "novis_test", "master");
        payload.extend_from_slice(&message_token(
            TOKEN_INFO,
            5701,
            0,
            "Changed database context.",
        ));
        payload.extend_from_slice(&done_token(DONE_COUNT, 4));

        let mut wire = answering(&[payload]);
        let state = Cell::new(State::Executing);
        let mut rows = read_rows(&mut wire, &state, span()).expect("an answer with no result set");

        assert!(rows.columns().is_empty());
        assert_eq!(
            rows.affected(),
            Some(4),
            "the server's own count, not the rows"
        );
        assert!(
            rows.next_row().expect("an ended stream").is_none(),
            "there is nothing to read and the wire is at a boundary"
        );
        assert_eq!(state.get(), State::Idle);
    }

    #[test]
    fn a_done_that_counted_nothing_reports_no_count_rather_than_zero() {
        let mut wire = answering(&[done_token(0, 99)]);
        let state = Cell::new(State::Executing);
        let rows = read_rows(&mut wire, &state, span()).expect("an answer with no result set");

        assert_eq!(
            rows.affected(),
            None,
            "the row count without the count bit is a number the server did not mean"
        );
        assert_eq!(state.get(), State::Idle);
    }

    #[test]
    fn a_refused_statement_is_read_to_its_done_and_leaves_the_connection_idle() {
        let mut payload = four_columns();
        payload.extend_from_slice(&four_values());
        payload.extend_from_slice(&message_token(
            TOKEN_ERROR,
            8134,
            16,
            "Divide by zero error encountered.",
        ));
        payload.extend_from_slice(&done_token(DONE_ERROR, 0));

        let mut wire = answering(&chunked(&payload, 11));
        let state = Cell::new(State::Executing);
        let mut rows = read_rows(&mut wire, &state, span()).expect("a described result set");
        let refused = drain(&mut rows).expect_err("the server refused the statement");

        let error = ServerError::of(&refused).expect("the server's own refusal");
        assert_eq!(error.driver_code, Some(8134));
        assert_eq!(error.kind, DbErrorKind::Other);
        assert_eq!(
            state.get(),
            State::Idle,
            "the ERROR's DONE was read, so the wire is at a boundary"
        );
    }

    #[test]
    fn a_second_result_set_is_refused_rather_than_left_on_the_wire() {
        let mut payload = four_columns();
        payload.extend_from_slice(&four_values());
        payload.extend_from_slice(&done_token(DONE_MORE | DONE_COUNT, 1));
        payload.extend_from_slice(&four_columns());
        payload.extend_from_slice(&done_token(DONE_COUNT, 0));

        let mut wire = answering(&[payload]);
        let state = Cell::new(State::Executing);
        let mut rows = read_rows(&mut wire, &state, span()).expect("a described result set");
        let refused = drain(&mut rows).expect_err("ADR 0067 § 4 has nowhere to put the second");

        assert_eq!(refused.kind(), io::ErrorKind::InvalidData);
        assert!(refused.to_string().contains("second result set"));
        assert_eq!(state.get(), State::Poisoned);
    }

    #[test]
    fn an_answer_whose_last_packet_ends_mid_token_is_refused() {
        let mut payload = four_columns();
        let mut cut = four_values();
        cut.truncate(cut.len() - 3);
        payload.extend_from_slice(&cut);

        let mut wire = answering(&[payload]);
        let state = Cell::new(State::Executing);
        let mut rows = read_rows(&mut wire, &state, span()).expect("a described result set");
        let refused = drain(&mut rows).expect_err("the row's last value never arrived");

        assert_eq!(refused.kind(), io::ErrorKind::InvalidData);
        assert_eq!(state.get(), State::Poisoned);
    }

    #[test]
    fn a_value_wider_than_its_column_declared_is_refused_before_it_is_read() {
        let mut payload = col_metadata(&[column(
            &char_type(TY_BIGVARCHAR, 4),
            "name",
            COLUMN_NULLABLE,
        )]);
        payload.extend_from_slice(&row_token(&[short_value(b"abcdefgh")]));
        payload.extend_from_slice(&done_token(DONE_COUNT, 1));

        let mut wire = answering(&[payload]);
        let state = Cell::new(State::Executing);
        let mut rows = read_rows(&mut wire, &state, span()).expect("a described result set");
        let refused = drain(&mut rows).expect_err("eight bytes in a varchar(4)");

        assert_eq!(refused.kind(), io::ErrorKind::InvalidData);
        assert!(
            refused
                .to_string()
                .contains("past the 4 its COLMETADATA declared")
        );
        assert_eq!(state.get(), State::Poisoned);
    }

    #[test]
    fn every_row_of_a_stream_is_counted_once_on_its_span() {
        let mut payload = four_columns();
        for _ in 0..3 {
            payload.extend_from_slice(&four_values());
        }
        payload.extend_from_slice(&done_token(DONE_COUNT, 3));

        let mut wire = answering(&chunked(&payload, 16));
        let state = Cell::new(State::Executing);
        let mut rows = read_rows(&mut wire, &state, span()).expect("a described result set");
        let read = drain(&mut rows).expect("three rows and a DONE");

        assert_eq!(read.len(), 3);
        assert_eq!(rows.span().rows(), 3);
        assert_eq!(rows.affected(), Some(3));
        assert!(rows.span().duration() > std::time::Duration::ZERO);
    }

    #[test]
    fn a_statement_the_server_refused_before_describing_anything_is_the_servers_error() {
        let mut payload = message_token(TOKEN_ERROR, 208, 16, "Invalid object name 'nope'.");
        payload.extend_from_slice(&done_token(DONE_ERROR, 0));

        let mut wire = answering(&[payload]);
        let state = Cell::new(State::Executing);
        let refused = read_rows(&mut wire, &state, span()).expect_err("the server refused it");

        let error = ServerError::of(&refused).expect("the server's own refusal");
        assert_eq!(error.driver_code, Some(208));
        assert!(error.sql_state.is_empty(), "TDS sends no SQLSTATE");
        assert_eq!(state.get(), State::Idle);
    }

    // ---- The RPC out, and the two tokens a procedure answers with: §§ 1 and 5 -

    /// One argument of an RPC, read back off the request that carried it.
    #[derive(Debug, PartialEq, Eq)]
    struct SentParam {
        by_ref: bool,
        type_id: u8,
        /// The width the `TYPE_INFO` declared: `NO_LENGTH` is the `MAX` form.
        declared: u16,
        /// The value as text, and `None` for the null of whichever form.
        text: Option<String>,
    }

    /// Walks a request the way a server does: past its headers, then one
    /// argument at a time until the message ends.
    ///
    /// Deliberately a walk rather than a table of offsets — a parameter's extent
    /// depends on the one before it, so an offset asserted by hand would pass on
    /// a request whose *earlier* fields were the wrong width.
    fn sent_rpc(request: &[u8]) -> (u16, Vec<SentParam>) {
        assert_eq!(
            u32::from_le_bytes(request[0..4].try_into().unwrap()),
            ALL_HEADERS_BYTES
        );
        assert_eq!(
            u32::from_le_bytes(request[4..8].try_into().unwrap()),
            TRANSACTION_HEADER_BYTES
        );
        assert_eq!(
            u16::from_le_bytes([request[8], request[9]]),
            HEADER_TRANSACTION
        );
        assert_eq!(request[10..18], [0; 8], "no transaction descriptor");
        assert_eq!(u32::from_le_bytes(request[18..22].try_into().unwrap()), 1);
        assert_eq!(
            u16::from_le_bytes([request[22], request[23]]),
            PROC_ID_SWITCH
        );
        let proc_id = u16::from_le_bytes([request[24], request[25]]);
        assert_eq!(u16::from_le_bytes([request[26], request[27]]), 0);

        let mut at = usize::try_from(ALL_HEADERS_BYTES).expect("twenty-two") + 6;
        let mut params = Vec::new();
        while at < request.len() {
            assert_eq!(request[at], 0, "every argument goes out unnamed");
            let by_ref = request[at + 1] == PARAM_BY_REF;
            let type_id = request[at + 2];
            at += 3;
            let (declared, text) = match type_id {
                TY_INTN => {
                    let declared = u16::from(request[at]);
                    let length = usize::from(request[at + 1]);
                    at += 2 + length;
                    let text = (length > 0).then(|| {
                        let bytes = &request[at - length..at];
                        i32::from_le_bytes(bytes.try_into().unwrap()).to_string()
                    });
                    (declared, text)
                }
                TY_NVARCHAR => {
                    let declared = u16::from_le_bytes([request[at], request[at + 1]]);
                    assert_eq!(request[at + 2..at + 7], NO_COLLATION);
                    at += 7;
                    let mut bytes = Vec::new();
                    let null;
                    if declared == NO_LENGTH {
                        let total = u64::from_le_bytes(request[at..at + 8].try_into().unwrap());
                        at += 8;
                        null = total == PLP_NULL;
                        if !null {
                            loop {
                                let chunk = usize::try_from(u32::from_le_bytes(
                                    request[at..at + 4].try_into().unwrap(),
                                ))
                                .unwrap();
                                at += 4;
                                if chunk == 0 {
                                    break;
                                }
                                bytes.extend_from_slice(&request[at..at + chunk]);
                                at += chunk;
                            }
                            assert_eq!(
                                u64::try_from(bytes.len()).unwrap(),
                                total,
                                "the declared total is the truth, not a sentinel"
                            );
                        }
                    } else {
                        let length =
                            usize::from(u16::from_le_bytes([request[at], request[at + 1]]));
                        at += 2;
                        null = length == usize::from(NO_LENGTH);
                        if !null {
                            bytes.extend_from_slice(&request[at..at + length]);
                            at += length;
                        }
                    }
                    let units: Vec<u16> = bytes
                        .chunks_exact(2)
                        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
                        .collect();
                    (
                        declared,
                        (!null).then(|| String::from_utf16(&units).expect("what we wrote")),
                    )
                }
                other => panic!("this driver sends no argument of type 0x{other:02X}"),
            };
            params.push(SentParam {
                by_ref,
                type_id,
                declared,
                text,
            });
        }
        (proc_id, params)
    }

    #[test]
    fn an_rpc_names_sp_prepexec_and_carries_a_handle_a_declaration_the_sql_and_every_value() {
        let request = sp_prepexec_request(
            "select * from t where a = @p1 and b = @p2",
            &[Some(b"7"), None],
        )
        .expect("two bound values");
        let (proc_id, params) = sent_rpc(&request);

        assert_eq!(proc_id, PROC_SP_PREPEXEC);
        assert_eq!(
            params.len(),
            5,
            "the handle, @params, @stmt, and two values"
        );
        assert_eq!(
            params[0],
            SentParam {
                by_ref: true,
                type_id: TY_INTN,
                declared: u16::from(INTN_BYTES),
                text: None,
            },
            "the handle goes out null and by reference, for the server to fill in"
        );
        assert!(params[1..].iter().all(|param| !param.by_ref));
        assert_eq!(
            params[1].text.as_deref(),
            Some("@p1 nvarchar(4000),@p2 nvarchar(4000)")
        );
        assert_eq!(
            params[2].text.as_deref(),
            Some("select * from t where a = @p1 and b = @p2"),
            "the SQL is an argument to the procedure and never a string a value \
             was spliced into"
        );
        assert_eq!(params[3].text.as_deref(), Some("7"));
        assert_eq!(params[4].text, None, "a bound null is the type's own null");
    }

    /// A statement that binds nothing declares nothing, and `@params` is null
    /// rather than empty: an empty declaration is a plan with a parameter list
    /// the server then finds no parameters for.
    #[test]
    fn a_statement_that_binds_nothing_sends_a_null_declaration_and_no_values() {
        let request = sp_prepexec_request("select 1", &[]).expect("no bound values");
        let (_, params) = sent_rpc(&request);

        assert_eq!(params.len(), 3);
        assert_eq!(params[1].text, None, "@params");
        assert_eq!(params[2].text.as_deref(), Some("select 1"));
    }

    /// Agreement, over § 5's own rewriter: the names `@params` declares are the
    /// names the rewritten SQL uses, because both ask [`Dialect::marker`].
    #[test]
    fn a_declarations_markers_are_the_ones_section_fives_rewriter_wrote() {
        let statement = crate::sql::rewrite(
            "insert into t values (?, ?, ?)",
            crate::sql::Params::Positional(&[
                crate::sql::Binding::One,
                crate::sql::Binding::One,
                crate::sql::Binding::One,
            ]),
            Dialect::SqlServer,
        )
        .expect("three positional markers");

        let request = sp_prepexec_request(&statement.sql, &[Some(b"a"), Some(b"b"), Some(b"c")])
            .expect("three bound values");
        let (_, params) = sent_rpc(&request);
        let declared = params[1].text.clone().expect("a declaration");

        assert_eq!(statement.arity(), 3);
        for marker in declared.split(',') {
            let name = marker.split(' ').next().expect("a name and a type");
            assert!(
                statement.sql.contains(name),
                "{name} is declared and never written: {}",
                statement.sql
            );
        }
    }

    /// Both sides of the bound `sp_prepexec` is told about: 4,000 characters is
    /// the widest `nvarchar` that is not a `MAX` one, and 4,001 is not a
    /// narrower one.
    #[test]
    fn a_value_past_four_thousand_characters_is_declared_and_sent_as_max() {
        for (chars, declared, spelling) in [
            (
                usize::from(NVARCHAR_CHARS),
                NVARCHAR_CHARS * 2,
                "nvarchar(4000)",
            ),
            (usize::from(NVARCHAR_CHARS) + 1, NO_LENGTH, "nvarchar(max)"),
        ] {
            let value = "x".repeat(chars);
            let request =
                sp_prepexec_request("select @p1", &[Some(value.as_bytes())]).expect("one value");
            let (_, params) = sent_rpc(&request);

            assert_eq!(params[1].text.as_deref(), Some(&*format!("@p1 {spelling}")));
            assert_eq!(params[3].declared, declared, "{chars} character(s)");
            assert_eq!(params[3].text.as_deref(), Some(&*value));
        }
    }

    /// A null takes the null of whichever form its declaration named, and the
    /// two are different sentinels rather than one.
    #[test]
    fn a_null_value_is_the_narrow_forms_sentinel_and_a_max_one_is_plps() {
        let request = sp_prepexec_request("select @p1", &[None]).expect("one null");
        let (_, params) = sent_rpc(&request);
        assert_eq!(params[3].declared, NVARCHAR_CHARS * 2);
        assert_eq!(params[3].text, None);

        // The `MAX` half of the same rule, asserted where a value forces it:
        // `@params` and `@stmt` take the same path as a bound value.
        let long = "y".repeat(usize::from(NVARCHAR_CHARS) + 1);
        let request = sp_prepexec_request(&long, &[]).expect("a long statement");
        let (_, params) = sent_rpc(&request);
        assert_eq!(params[2].declared, NO_LENGTH);
        assert_eq!(params[2].text.as_deref(), Some(&*long));
    }

    #[test]
    fn a_parameter_that_is_not_utf8_is_refused_by_the_marker_it_was_bound_at() {
        let refused = sp_prepexec_request("select @p1, @p2", &[Some(b"fine"), Some(&[0xFF, 0xFE])])
            .expect_err("this driver has no binary parameter yet");

        assert_eq!(refused.kind(), io::ErrorKind::InvalidInput);
        assert!(refused.to_string().contains("@p2"), "{refused}");
    }

    /// A `RETURNVALUE` as a server writes one: no length in front of it, so its
    /// extent is the `TYPE_INFO` and nothing else.
    fn return_value_token(name: &str, handle: Option<i32>) -> Vec<u8> {
        let mut out = vec![TOKEN_RETURN_VALUE, 1, 0];
        out.extend_from_slice(&b_varchar(name));
        // `Status` — an RPC's output parameter — then `UserType` and `Flags`.
        out.push(0x01);
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.push(TY_INTN);
        out.push(INTN_BYTES);
        match handle {
            None => out.push(0),
            Some(number) => {
                out.push(INTN_BYTES);
                out.extend_from_slice(&number.to_le_bytes());
            }
        }
        out
    }

    /// A `RETURNSTATUS`: four bytes and no length either.
    fn return_status_token(status: i32) -> Vec<u8> {
        let mut out = vec![TOKEN_RETURN_STATUS];
        out.extend_from_slice(&status.to_le_bytes());
        out
    }

    #[test]
    fn a_procedures_answer_is_read_past_its_two_tokens_and_the_handle_is_kept() {
        let mut payload = four_columns();
        payload.extend_from_slice(&four_values());
        // Where `sp_prepexec` really puts them: the inner statement's own
        // `DONEINPROC`, then the handle and the status, then the `DONEPROC`
        // that ends the procedure. The `DONEINPROC` is deliberately written
        // without `DONE_MORE` — its *type byte* is what says something follows.
        payload.extend_from_slice(&done_kind(TOKEN_DONE_IN_PROC, DONE_COUNT, 1));
        payload.extend_from_slice(&return_value_token("@handle", Some(9)));
        payload.extend_from_slice(&return_status_token(0));
        payload.extend_from_slice(&done_kind(TOKEN_DONE_PROC, 0, 0));

        let mut wire = answering(&[payload]);
        let state = Cell::new(State::Executing);
        let mut rows = read_rows(&mut wire, &state, span()).expect("a described result set");
        assert_eq!(rows.returned(), None, "the handle arrives after the rows");

        let read = drain(&mut rows).expect("one row, then the procedure's own end");
        assert_eq!(read.len(), 1);
        let returned = rows.returned().expect("sp_prepexec's handle");
        assert_eq!(returned.name, "@handle");
        assert_eq!(returned.ordinal, 1);
        assert_eq!(returned.as_i32(), Some(9));
        assert_eq!(state.get(), State::Idle);
    }

    /// The same two tokens on the other side of the `COLMETADATA`, which is
    /// [`TdsRows::shape`] rather than [`TdsRows::step`]: a procedure that
    /// assigned its output parameter before selecting anything sends them
    /// there, and a reader that only knew one of the two paths would refuse it.
    #[test]
    fn the_two_procedure_tokens_are_read_before_the_columns_as_well() {
        let mut payload = return_status_token(0);
        payload.extend_from_slice(&return_value_token("@handle", Some(4)));
        payload.extend_from_slice(&four_columns());
        payload.extend_from_slice(&four_values());
        payload.extend_from_slice(&done_token(DONE_COUNT, 1));

        let mut wire = answering(&[payload]);
        let state = Cell::new(State::Executing);
        let mut rows = read_rows(&mut wire, &state, span()).expect("a described result set");
        assert_eq!(
            rows.returned().and_then(ReturnValue::as_i32),
            Some(4),
            "the handle was there before the columns were"
        );
        assert_eq!(drain(&mut rows).expect("one row").len(), 1);
    }

    /// A null output parameter is `None` and not a zero handle: a procedure
    /// that never assigned one has not allocated a plan number 0.
    #[test]
    fn an_unassigned_output_parameter_is_null_rather_than_a_handle() {
        let mut payload = return_value_token("@handle", None);
        payload.extend_from_slice(&done_token(0, 0));

        let mut wire = answering(&[payload]);
        let state = Cell::new(State::Executing);
        let rows = read_rows(&mut wire, &state, span()).expect("a statement with no result set");
        let returned = rows.returned().expect("the token was there");
        assert_eq!(returned.value, None);
        assert_eq!(returned.as_i32(), None);
    }

    #[test]
    fn a_statement_goes_out_as_one_rpc_message_and_its_rows_come_back() {
        let mut payload = four_columns();
        payload.extend_from_slice(&four_values());
        payload.extend_from_slice(&done_kind(TOKEN_DONE_IN_PROC, DONE_COUNT, 1));
        payload.extend_from_slice(&return_value_token("@handle", Some(3)));
        payload.extend_from_slice(&done_kind(TOKEN_DONE_PROC, 0, 0));

        let mut wire = answering(&[payload]);
        let state = Cell::new(State::Idle);
        let mut cache = plans(2);
        let mut rows = start_statement(
            &mut wire,
            &state,
            &mut cache,
            "select * from t where a = @p1",
            &[Some(b"7")],
        )
        .expect("a described result set");
        assert_eq!(drain(&mut rows).expect("one row").len(), 1);
        assert_eq!(rows.returned().and_then(ReturnValue::as_i32), Some(3));

        let sent = wire.peer().sent.clone();
        assert_eq!(sent[0], PacketType::Rpc.byte());
        assert_eq!(sent[1], Status::EOM.bits(), "one packet, and it ends there");
        assert_eq!(
            usize::from(u16::from_be_bytes([sent[2], sent[3]])),
            sent.len()
        );
        let (proc_id, params) = sent_rpc(&sent[HEADER..]);
        assert_eq!(proc_id, PROC_SP_PREPEXEC);
        assert_eq!(params[3].text.as_deref(), Some("7"));
    }

    /// § 4's one statement at a time, held here rather than by every caller —
    /// and a request this driver would not build never reaches the wire, so the
    /// connection it was refused on is still usable.
    #[test]
    fn a_statement_on_a_busy_connection_is_refused_and_a_bad_parameter_leaves_it_idle() {
        let mut wire = Wire::new(Script::silent());
        let busy = Cell::new(State::Streaming);
        let mut cache = plans(2);
        let refused = start_statement(&mut wire, &busy, &mut cache, "select 1", &[])
            .expect_err("a second statement on one connection");
        assert_eq!(refused.kind(), io::ErrorKind::InvalidInput);

        let idle = Cell::new(State::Idle);
        let refused = start_statement(&mut wire, &idle, &mut cache, "select @p1", &[Some(&[0xFF])])
            .expect_err("a parameter that is not UTF-8");
        assert_eq!(refused.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(idle.get(), State::Idle, "nothing was written");
        assert!(wire.peer().sent.is_empty());
    }

    /// An empty § 1 cache of `capacity` plans.
    fn plans(capacity: usize) -> StatementCache<TdsPlan> {
        StatementCache::new(capacity)
    }

    /// One `Wire` answering a *sequence* of requests, each answer a message of
    /// its own — where [`answering`] builds one answer out of many packets.
    fn answering_each(answers: &[Vec<u8>]) -> Wire<Script> {
        let mut inbound = Vec::new();
        for payload in answers {
            inbound.extend_from_slice(&packet(PacketType::TabularResult, Status::EOM, 1, payload));
        }
        Wire::new(Script::answering(inbound, READ_CHUNK))
    }

    /// Every message the driver flushed, reassembled from the packets it wrote.
    ///
    /// `Script` records one stream of bytes, and § 1's cache is asserted on the
    /// *sequence* of requests rather than on any one of them: which procedure
    /// went out, in what order, is the whole of what a hit and an eviction are.
    /// Messages and not packets, because a `nvarchar(max)` value is longer than
    /// the negotiated packet size and a request counted in packets would make a
    /// long parameter look like three requests. The status kept is the **first**
    /// packet's, which is where [`Status::RESET_CONNECTION`] rides.
    fn flushed(sent: &[u8]) -> Vec<(PacketType, Status, Vec<u8>)> {
        let mut out: Vec<(PacketType, Status, Vec<u8>)> = Vec::new();
        let mut at = 0;
        let mut open = false;
        while at < sent.len() {
            let length = usize::from(u16::from_be_bytes([sent[at + 2], sent[at + 3]]));
            let kind = PacketType::from_byte(sent[at]).expect("a type this driver writes");
            let status = Status(sent[at + 1]);
            let body = &sent[at + HEADER..at + length];
            if open {
                out.last_mut()
                    .expect("a message these bytes continue")
                    .2
                    .extend_from_slice(body);
            } else {
                out.push((kind, status, body.to_vec()));
            }
            open = !status.contains(Status::EOM);
            at += length;
        }
        out
    }

    /// `sp_prepexec`'s answer for a statement with no result set: the inner
    /// statement's own end, then the handle and the status, then the procedure's.
    fn prepexec_answer(handle: i32) -> Vec<u8> {
        let mut out = done_kind(TOKEN_DONE_IN_PROC, DONE_COUNT, 1);
        out.extend_from_slice(&return_value_token("@handle", Some(handle)));
        out.extend_from_slice(&return_status_token(0));
        out.extend_from_slice(&done_kind(TOKEN_DONE_PROC, 0, 0));
        out
    }

    /// What `sp_unprepare` answers: a status and the end of the procedure.
    fn procedure_answer() -> Vec<u8> {
        let mut out = return_status_token(0);
        out.extend_from_slice(&done_kind(TOKEN_DONE_PROC, 0, 0));
        out
    }

    /// ADR 0067 § 13's reset on this backend, both halves of it: the session is
    /// reset through `sp_reset_connection` — which MS-TDS spells as a header bit
    /// rather than as a call — and § 1's cache is emptied with it, because the
    /// reset drops the server's prepared statements.
    ///
    /// The asymmetry § 13 names, from the other side: PostgreSQL's reset is a
    /// command list picked so the cache *survives*, and this one has no such
    /// choice to make. A cache that survived here would bind the next request
    /// against plan handles this session no longer has — a wrong answer rather
    /// than a slow one.
    #[test]
    fn mssql_resets_through_sp_reset_connection_and_loses_its_cache() {
        const SQL: &str = "select a";

        let mut wire = answering_each(&[
            prepexec_answer(9),
            done_token(DONE_COUNT, 1),
            prepexec_answer(11),
        ]);
        let state = Cell::new(State::Idle);
        let mut cache = plans(2);

        drop(
            start_statement(&mut wire, &state, &mut cache, SQL, &[])
                .expect("a statement the server answered"),
        );
        assert_eq!(cache.len(), 1);

        reset_session(&mut wire, &state, &mut cache).expect("a reset the server acknowledged");
        assert!(
            cache.is_empty(),
            "§ 13: the reset drops every prepared statement, so a cache that \
             kept one is naming a plan the server does not have"
        );
        assert_eq!(
            state.get(),
            State::Idle,
            "the reset left the wire at a boundary"
        );

        drop(
            start_statement(&mut wire, &state, &mut cache, SQL, &[])
                .expect("a statement the server answered"),
        );

        let sent = flushed(&wire.peer().sent);
        assert_eq!(sent.len(), 3);
        assert_eq!(sent[0].0, PacketType::Rpc);
        assert_eq!(sent[1].0, PacketType::SqlBatch);
        assert!(
            sent[1].1.contains(Status::RESET_CONNECTION),
            "the reset is the header bit, which is MS-TDS's own sp_reset_connection"
        );
        assert_eq!(sent[2].0, PacketType::Rpc);
        assert_eq!(
            sent_rpc(&sent[2].2).0,
            PROC_SP_PREPEXEC,
            "the same statement after a reset costs the prepare again"
        );
    }

    /// § 1's "cached re-executions cost one round trip", on this protocol: the
    /// second execution names the plan and carries the values, and no byte of
    /// the statement is on the wire.
    #[test]
    fn a_cached_statement_is_sent_as_sp_execute_with_no_sql() {
        const SQL: &str = "select a from t where b = @p1";

        let mut wire = answering_each(&[prepexec_answer(9), done_token(DONE_COUNT, 1)]);
        let state = Cell::new(State::Idle);
        let mut cache = plans(2);

        drop(
            start_statement(&mut wire, &state, &mut cache, SQL, &[Some(b"7")])
                .expect("the first execution, which prepares"),
        );
        drop(
            start_statement(&mut wire, &state, &mut cache, SQL, &[Some(b"8")])
                .expect("the second, which does not"),
        );

        let sent = flushed(&wire.peer().sent);
        assert_eq!(
            sent.len(),
            2,
            "one message each, and the eviction sent none"
        );
        assert_eq!(sent_rpc(&sent[0].2).0, PROC_SP_PREPEXEC);

        let (proc_id, params) = sent_rpc(&sent[1].2);
        assert_eq!(proc_id, PROC_SP_EXECUTE);
        assert_eq!(
            params.len(),
            2,
            "the handle and the one value — no SQL, no @params"
        );
        assert_eq!(params[0].type_id, TY_INTN);
        assert!(
            !params[0].by_ref,
            "the handle is read here, never written back"
        );
        assert_eq!(params[0].text.as_deref(), Some("9"));
        assert_eq!(params[1].text.as_deref(), Some("8"));
        assert_eq!(cache.len(), 1, "one plan, executed twice");
    }

    /// § 1's capacity is what the *server* holds, so the eviction goes out
    /// before the prepare that needed the room and not after it.
    #[test]
    fn an_eviction_unprepares_before_the_prepare_that_needed_the_room() {
        let mut wire = answering_each(&[
            prepexec_answer(9),
            prepexec_answer(10),
            procedure_answer(),
            prepexec_answer(11),
        ]);
        let state = Cell::new(State::Idle);
        let mut cache = plans(2);

        for sql in ["select 1", "select 2", "select 3"] {
            drop(
                start_statement(&mut wire, &state, &mut cache, sql, &[])
                    .expect("a statement the server answered"),
            );
        }

        let sent = flushed(&wire.peer().sent);
        assert_eq!(sent.len(), 4);
        let (proc_id, params) = sent_rpc(&sent[2].2);
        assert_eq!(proc_id, PROC_SP_UNPREPARE);
        assert_eq!(
            params[0].text.as_deref(),
            Some("9"),
            "the plan unprepared is the least recently used one, not the new one"
        );
        assert_eq!(sent_rpc(&sent[3].2).0, PROC_SP_PREPEXEC);
        assert_eq!(cache.len(), 2, "never more plans than the block allowed");
    }

    /// The reason [`TdsPlan`] carries more than a handle: a plan compiled
    /// against `nvarchar(4000)` would *truncate* a longer value rather than
    /// refuse it, so the hit is rejected, the plan unprepared and a new one
    /// compiled against the declaration this execution needs.
    #[test]
    fn a_hit_whose_plan_was_declared_narrower_is_unprepared_rather_than_truncating() {
        const SQL: &str = "select a from t where b = @p1";
        let long = "x".repeat(usize::from(NVARCHAR_CHARS) + 1);

        let mut wire =
            answering_each(&[prepexec_answer(9), procedure_answer(), prepexec_answer(10)]);
        let state = Cell::new(State::Idle);
        let mut cache = plans(4);

        drop(
            start_statement(&mut wire, &state, &mut cache, SQL, &[Some(b"7")])
                .expect("a short value, declared narrow"),
        );
        drop(
            start_statement(&mut wire, &state, &mut cache, SQL, &[Some(long.as_bytes())])
                .expect("a long one, which the narrow plan cannot hold"),
        );

        let sent = flushed(&wire.peer().sent);
        assert_eq!(
            sent.len(),
            3,
            "the hit was rejected, so this one prepares too"
        );
        assert_eq!(sent_rpc(&sent[1].2).0, PROC_SP_UNPREPARE);
        let (proc_id, params) = sent_rpc(&sent[2].2);
        assert_eq!(proc_id, PROC_SP_PREPEXEC);
        assert!(
            params[1]
                .text
                .as_deref()
                .expect("a declaration")
                .contains("nvarchar(max)"),
            "the new plan is compiled against the declaration the value needs"
        );
        assert_eq!(cache.len(), 1, "the stale entry is dropped, not shadowed");
    }
}
