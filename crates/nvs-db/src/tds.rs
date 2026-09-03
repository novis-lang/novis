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
//! **Nothing sends any of it yet.** `TdsConn` in [`mod@crate::conn`] is a
//! busy-state flag with no wire in it, so the sequencing that would call
//! [`negotiate_tls`], write a LOGIN7 and read its tokens has nowhere to keep
//! the connection it opened. That is the next slice, and after it the result
//! set: [`Tokens`] reads a whole message and the row path cannot, for the
//! reason the first section gives.

use std::cell::Cell;
use std::io::{self, Read, Write};
use std::ops::Range;
use std::path::Path;
use std::rc::Rc;

use nvs_config::tree::Database;
use nvs_host::net::NvsTcp;
use nvs_host::tls::NvsTls;

use crate::conn::{BlockError, DbErrorKind, Driver, written_value};
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
    /// A statement's answer ended.
    Done(Done),
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

/// A reader over one message's tokens.
///
/// Sans-IO like the rest of this module: it borrows a payload and never touches
/// a stream, so every shape below is tested without a socket. The payload it is
/// given is one whole message — [`Wire::read_message`] — because a token is cut
/// at whatever offset the packet size lands on and this reader holds no
/// remainder of its own. The row path is the one that cannot afford that, and
/// it is a later slice's reader over [`Wire::read_packet`].
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
            TOKEN_DONE | TOKEN_DONE_PROC | TOKEN_DONE_IN_PROC => Token::Done(self.done()?),
            other => {
                return Err(malformed(format!(
                    "a TDS response carried token 0x{other:02X}, which this driver does not read"
                )));
            }
        };
        Ok(Some(token))
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
    fn done(&mut self) -> io::Result<Done> {
        let status = self.short("DONE")?;
        let command = self.short("DONE")?;
        let rows = self.take(8, "DONE")?;
        Ok(Done {
            status,
            command,
            rows: u64::from_le_bytes(rows.try_into().expect("eight bytes")),
        })
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

        let refused = tokens(&[0x81, 0, 0]).expect_err("COLMETADATA is a later slice's");
        assert!(
            refused.to_string().contains("0x81"),
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
}
