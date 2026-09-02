//! MySQL: reading the server's greeting, upgrading the socket in band,
//! authenticating with a proof rather than a password, forcing the
//! connection's charset to `utf8mb4`, and running one statement over
//! `COM_STMT_PREPARE` and `COM_STMT_EXECUTE`.
//!
//! **The statement path runs to the end of the result set.**
//! [`start_statement`] answers a [`MySqlRows`], and [`MySqlRows::next_row`]
//! takes one binary row packet at a time until the terminator returns the
//! connection to [`State::Idle`]. § 1's statement cache is not here, which is
//! why every statement costs the two round trips § 1 prices it at rather than
//! the one a cache hit would.
//!
//! # A binary row is a null bitmap and then values of no stated width
//!
//! Nothing in a row packet says how long a value is. The width comes from the
//! *column's declared type*, which arrived in the definition packets before the
//! first row, so a row can only be read against the definitions it belongs to —
//! and the fifth column can only be found by decoding the four before it. That
//! is why [`MySqlRow`] is decoded whole where [`crate::PgRow`] slices lazily:
//! the walk is unavoidable, so keeping what it produced is free.
//!
//! The bitmap comes first, one bit per column, and it is offset by two bits
//! because the server writes it — `mysql_common`'s `ServerSide` is that offset,
//! and the same structure written by a *client* in `COM_STMT_EXECUTE` has no
//! offset at all. The two are not interchangeable and a driver that used one
//! for the other reads every row shifted by two columns.
//!
//! [`column_type`] is the other half: § 9's type map, read off a column
//! definition and never off a value. What it does *not* do is turn a value into
//! a Novis one — that is the slice after this, on the boundary
//! [`crate::PgColumn::decode`] already occupies for the other driver.
//!
//! [ADR 0132 § 2](../../../docs/adr/0132-a-driver-is-a-sans-io-codec-over-the-parking-stream.md)
//! decides what is here and what is not. `mysql_common` frames every packet,
//! keeps the sequence-id discipline, and owns each authentication plugin's
//! challenge/response arithmetic — that is the borrowed codec. **The sequencing
//! is ours**: which packet may follow which, where the TLS upgrade sits inside
//! the packet stream, and — the part no codec can decide — *which* of the
//! plugins a server may name this driver will answer at all.
//!
//! # Opening is four exchanges, and MySQL's order is the awkward one
//!
//! PostgreSQL asks to upgrade before it says anything; MySQL speaks first. The
//! server's greeting — its version, its capability bits, the nonce every plugin
//! challenges against — arrives in the clear, before there is any way to
//! upgrade. That is the protocol's shape and not a choice, and it is why this
//! module's plaintext leg reads a packet where [`crate::pg`]'s only writes
//! eight bytes.
//!
//! 1. **The greeting**, in the clear. [`read_greeting`] takes it and nothing
//!    acts on it yet. A greeting is not authenticated: every field in it is a
//!    claim by a peer whose certificate has not been checked, so the only two
//!    things read off it are the ones the next step *must* have — the
//!    capability bits, and the version that decides how a response is framed.
//! 2. **The upgrade.** [`request_tls`] writes an `SSLRequest` — a bare
//!    capability word, the last plaintext this connection carries — and a
//!    server whose greeting does not offer `CLIENT_SSL` is **refused** rather
//!    than answered. [ADR 0067 § 3](../../../docs/adr/0067-core-db.md) makes
//!    `VerifyFull` the default with no spelling for turning it off, and a
//!    handshake that continued in the clear here is exactly PHP's
//!    `sslmode=prefer` under another name.
//! 3. **The handshake response and its auth loop**, over TLS.
//!    [`authenticate`] sends the user, the database, the collation and the
//!    first challenge response together, then answers whatever the server asks
//!    next until it says `OK` or `ERR`.
//! 4. **The session's declared zone**, [`set_session_time_zone`] — one
//!    `COM_QUERY`, because MySQL has no startup-parameter list to carry it the
//!    way PostgreSQL's does. It costs a round trip per *connection*, not per
//!    request; § 13's pool is what makes that the right place to spend it, and
//!    the reset that pool performs is `COM_RESET_CONNECTION`, which clears
//!    session variables — so whatever lands that reset owes this call again
//!    after it.
//!
//! # Two plugins, and the rest are refused
//!
//! This driver authenticates with [`CACHING_SHA2_PASSWORD`] and
//! [`MYSQL_NATIVE_PASSWORD`], and refuses every other plugin by name — the same
//! shape [`crate::pg`] takes with `SCRAM-SHA-256`, and for the same reason.
//! Both of those answer a server's nonce with a *proof*: what leaves this
//! process is a hash the server can check and cannot replay elsewhere. The ones
//! refused are the ones that move the secret itself or a broken digest of it —
//! `mysql_old_password`'s pre-4.1 hash, and `mysql_clear_password`, which
//! exists to hand the password verbatim to a server-side PAM module. MariaDB's
//! `ed25519` and `parsec` belong to MariaDB's own driver, which is a separate
//! roster and not a flag on this one.
//!
//! The refusal matters because the *server* chooses. `AuthSwitchRequest` lets a
//! server name a different plugin after the response has gone out, so a peer
//! that talked this driver into `mysql_clear_password` would be handed the
//! password in exchange for nothing. The gate is therefore applied in both
//! places a plugin can be named, and [`plugin_or_refuse`] is the one function
//! that decides it.
//!
//! `caching_sha2_password`'s cache-miss path does send the password, over TLS,
//! to a server whose certificate has already been verified to
//! [ADR 0067 § 3](../../../docs/adr/0067-core-db.md)'s standard. That is the
//! plugin's design and MySQL 8's default; it is not a downgrade a server can
//! ask for, because the fast path and the full path are the same plugin and the
//! same verified peer.
//!
//! # `LOCAL INFILE` is not a capability this client has
//!
//! [`CLIENT_CAPABILITIES`] does not contain `CLIENT_LOCAL_FILES`, so a
//! conforming server may not ask for a file at all — that is
//! [ADR 0067 § 3](../../../docs/adr/0067-core-db.md)'s first closed hole, and
//! it is closed by an absent bit rather than by a check. The check exists
//! anyway, in [`read_ok`], because the bit is a request and a malicious server
//! is under no obligation to honour it: a `0xFB` response is answered with the
//! empty packet that terminates a transfer having sent no bytes, and then the
//! connection is refused. Nothing on this path ever opens a path, so there is
//! no file for a bug to leak; what the refusal buys is that the *wire* says so
//! too.
//!
//! # UTF-8 is forced by the collation byte, not by a statement
//!
//! `HandshakeResponse` carries the connection's collation, and this driver
//! sends `utf8mb4_general_ci`. That is
//! [ADR 0067 § 3](../../../docs/adr/0067-core-db.md)'s third default — text
//! columns arrive as valid UTF-8 by construction, which is what
//! [ADR 0009](../../../docs/adr/0009-string-and-bytes.md)'s guarantee needs —
//! and it costs nothing, where a `SET NAMES` after the fact would be a round
//! trip and a window in which one was not set.

use std::cell::Cell;
use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use bytes::BytesMut;
use mysql_common::auth::plugins::{
    AuthProc, ChallengeResponsePlugin, Context as AuthContextTrait, Response as AuthResponse,
};
use mysql_common::collations::CollationId;
use mysql_common::constants::{
    CapabilityFlags, ColumnFlags, ColumnType as MyColumnType, StatusFlags,
};
use mysql_common::io::ParseBuf;
use mysql_common::packets::{
    AuthMoreData, AuthPlugin, AuthSwitchRequest, Column, ComStmtExecuteRequestBuilder,
    CommonOkPacket, ErrPacket, HandshakePacket, HandshakeResponse, LocalInfilePacket, NullBitmap,
    OkPacketDeserializer, ResultSetTerminator, SslRequest, StmtPacket,
};
use mysql_common::proto::codec::PacketCodec;
use mysql_common::proto::codec::error::PacketCodecError;
use mysql_common::proto::{MyDeserialize, MySerialize};
use mysql_common::value::{BinValue, ServerSide, Value as MyValue, ValueDeserializer};
use nvs_host::net::NvsTcp;
use nvs_host::tls::NvsTls;

use crate::conn::{ColumnType, MySqlConn, State};

/// MySQL's own port, which an absent `port` in a `[db.<name>]` block means.
///
/// Not a field of [`MySqlTarget`], for the reason it is not one on
/// [`crate::pg::PgTarget`]: a port is half of an address, and the address is
/// resolved by whoever checked the `db.connect` capability. What this crate
/// owes that caller is the number to fall back to.
pub const DEFAULT_PORT: u16 = 3306;

/// The plugin MySQL 8 defaults to, and the first this driver accepts.
pub const CACHING_SHA2_PASSWORD: &str = "caching_sha2_password";

/// The plugin MySQL 5.7 defaults to, and the second this driver accepts.
pub const MYSQL_NATIVE_PASSWORD: &str = "mysql_native_password";

/// `COM_QUERY` — the only text command this driver composes, and it composes
/// exactly one of them ([`set_session_time_zone`]).
const COM_QUERY: u8 = 0x03;

/// `COM_QUIT`, the goodbye a destroyed connection writes.
const COM_QUIT: u8 = 0x01;

/// `COM_RESET_CONNECTION`, ADR 0067 § 13's reset for this backend.
const COM_RESET_CONNECTION: u8 = 0x1F;

/// `COM_STMT_PREPARE` — ADR 0067 § 1's first round trip.
///
/// `COM_STMT_EXECUTE` and `COM_STMT_CLOSE` have no constant beside this one
/// because `mysql_common` builds those two packets header and all, and a second
/// spelling of a byte it already writes is a place for the two to disagree.
const COM_STMT_PREPARE: u8 = 0x16;

/// How much room a read is given when the inbox holds no whole packet.
///
/// [`crate::pg`]'s number and for its reason: a TLS record caps at 16 KiB, so a
/// larger chunk buys no fewer syscalls and a much smaller one costs a syscall
/// per row.
const READ_CHUNK: usize = 16 * 1024;

/// The largest packet payload this client will send or accept, in bytes.
///
/// It is a *ceiling* rather than a size: the codec refuses anything larger
/// instead of allocating it, which is the whole point of naming a number a
/// server does not get to choose. 16 MiB is MySQL's own `max_allowed_packet`
/// default, so a value this client can send is one a stock server accepts.
const MAX_PACKET: u32 = 16 * 1024 * 1024;

/// The collation every connection this driver opens is in —
/// [ADR 0067 § 3](../../../docs/adr/0067-core-db.md)'s forced `utf8mb4`.
const COLLATION: u8 = CollationId::UTF8MB4_GENERAL_CI as u8;

/// What this client claims it can do, and the absences are the interesting
/// half.
///
/// **`CLIENT_LOCAL_FILES` is not here**, which is ADR 0067 § 3's `LOCAL INFILE`
/// closed at the only place it can be closed *before* a server asks — the
/// module doc owns the second half of that rule. **`CLIENT_COMPRESS` and
/// `CLIENT_ZSTD_COMPRESSION_ALGORITHM` are not here** either: the workspace
/// takes `mysql_common` with its compression feature off, so there is no
/// implementation behind the bit, and a compressed stream would in any case be
/// a second framing layer under a TLS session that already has one.
/// **`CLIENT_MULTI_STATEMENTS` is not here**, and that is a security bit: it is
/// what turns one injected `;` into two statements, and ADR 0067 § 1's
/// every-statement-is-prepared has no use for it.
const CLIENT_CAPABILITIES: CapabilityFlags = CapabilityFlags::CLIENT_PROTOCOL_41
    .union(CapabilityFlags::CLIENT_SECURE_CONNECTION)
    .union(CapabilityFlags::CLIENT_PLUGIN_AUTH)
    .union(CapabilityFlags::CLIENT_PLUGIN_AUTH_LENENC_CLIENT_DATA)
    .union(CapabilityFlags::CLIENT_SSL)
    .union(CapabilityFlags::CLIENT_LONG_PASSWORD)
    .union(CapabilityFlags::CLIENT_LONG_FLAG)
    .union(CapabilityFlags::CLIENT_TRANSACTIONS)
    .union(CapabilityFlags::CLIENT_MULTI_RESULTS)
    .union(CapabilityFlags::CLIENT_PS_MULTI_RESULTS)
    .union(CapabilityFlags::CLIENT_DEPRECATE_EOF)
    .union(CapabilityFlags::CLIENT_CONNECT_WITH_DB);

/// The bits a server must offer before any credential is composed.
///
/// `CLIENT_SSL` is in here rather than checked separately because it is the
/// same kind of fact as the other three: without it there is no connection this
/// driver is willing to have, and the refusal should read as one sentence.
const REQUIRED_CAPABILITIES: CapabilityFlags = CapabilityFlags::CLIENT_PROTOCOL_41
    .union(CapabilityFlags::CLIENT_SECURE_CONNECTION)
    .union(CapabilityFlags::CLIENT_PLUGIN_AUTH)
    .union(CapabilityFlags::CLIENT_SSL);

/// Where one MySQL server is, and who to be there.
///
/// [`crate::pg::PgTarget`]'s shape and its split: `host` is the name the
/// certificate must be valid for, and the *address* is not here at all because
/// it was resolved by whoever checked the `db.connect` capability. A driver
/// that re-resolved the name would be connecting somewhere nobody approved.
pub struct MySqlTarget<'a> {
    /// The name the server's certificate is checked against.
    pub host: &'a str,
    /// The user to log in as. ADR 0067 § 3 accepts `tainted` here freely: it is
    /// a length-prefixed protocol field, never parsed text.
    pub user: &'a str,
    /// The user's password, used only to derive a challenge response.
    pub password: &'a str,
    /// The schema to select on connect.
    pub database: &'a str,
    /// The PEM bundle whose anchors this server's certificate is verified
    /// against, or the compiled-in Mozilla set where the block names none.
    ///
    /// Not a way to turn verification off — ADR 0067 § 3 has no spelling for
    /// that. What it changes is *whose* certificates are believed.
    pub tls_ca_file: Option<&'a Path>,
    /// The zone a `DATETIME` or `TIMESTAMP` off this connection is read in, as
    /// a whole number of seconds east of UTC.
    ///
    /// [ADR 0067 § 9](../../../docs/adr/0067-core-db.md)'s declared zone. It is
    /// sent to the server as well as held here, so `CURRENT_TIMESTAMP` and a
    /// decoded column agree about which zone they are in —
    /// [`set_session_time_zone`] is where it goes out, and as a numeric offset
    /// rather than a name because named zones need `mysql.time_zone` populated
    /// and it usually is not.
    pub time_zone: i32,
}

impl std::fmt::Debug for MySqlTarget<'_> {
    /// Everything but the password, which is a `secret` at the language level
    /// (ADR 0067 § 3) and is not printed in any rendering.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MySqlTarget")
            .field("host", &self.host)
            .field("user", &self.user)
            .field("database", &self.database)
            .field("time_zone", &self.time_zone)
            .finish_non_exhaustive()
    }
}

/// A MySQL connection's stream, the bytes read off it that are not yet a whole
/// packet, and the sequence-id state that says which packet is next.
///
/// Generic in the stream so the exchanges below can be asserted against a
/// server that answers rather than against a socket — the playbook's rule for
/// this crate, and the reason every routine here takes a `&mut Wire<S>` instead
/// of being an inherent method on [`MySqlConn`]. A real connection's wire is
/// the `NvsTls<NvsTcp>` the default names.
pub(crate) struct Wire<S: Read + Write = NvsTls<NvsTcp>> {
    stream: S,
    inbox: BytesMut,
    /// `mysql_common`'s framing, and the sequence counter with it. It survives
    /// [`Wire::upgrade`] because MySQL's TLS upgrade happens *inside* the
    /// packet stream: the `SSLRequest` is packet 1 and the handshake response
    /// that follows it, on the encrypted socket, is packet 2. A codec reset at
    /// the upgrade would put the connection out of sync on its first
    /// authenticated byte.
    codec: PacketCodec,
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
    /// A wire over a stream that is already connected.
    ///
    /// Unlike [`crate::pg`]'s, this one is built *before* the upgrade: MySQL
    /// speaks first, so the greeting is read through the codec on a plaintext
    /// socket and the same codec continues once the socket is encrypted.
    fn new(stream: S) -> Wire<S> {
        let mut codec = PacketCodec::default();
        codec.max_allowed_packet = MAX_PACKET as usize;
        Wire {
            stream,
            inbox: BytesMut::new(),
            codec,
        }
    }

    /// The same conversation over an encrypted stream.
    ///
    /// Takes the codec and the inbox across with it — see the field. `wrap`
    /// receives the plaintext stream and answers the encrypted one, which is
    /// the only shape that works: the TLS session is built *from* this wire's
    /// socket, so a signature taking the new stream would need the old one
    /// moved out first and leave a half-consumed `Wire` behind.
    ///
    /// The caller must have flushed the `SSLRequest` first, and must not have
    /// read anything since: whatever is buffered here arrived in the clear.
    ///
    /// # Errors
    ///
    /// Whatever `wrap` reported.
    fn upgrade<T: Read + Write>(
        self,
        wrap: impl FnOnce(S) -> io::Result<T>,
    ) -> io::Result<Wire<T>> {
        Ok(Wire {
            stream: wrap(self.stream)?,
            inbox: self.inbox,
            codec: self.codec,
        })
    }

    /// The stream underneath, for the tests that assert on what was written to
    /// it. There is no non-test reader: a driver talks through the two methods
    /// below, and reaching past them is how a half-packet gets written.
    #[cfg(test)]
    fn peer(&self) -> &S {
        &self.stream
    }

    /// Frames `payload` as the next packet in sequence, writes all of it, and
    /// flushes.
    ///
    /// One function because a half-written packet is the shape ADR 0132 § 4
    /// calls poison, and `write_all` is the only spelling that cannot leave
    /// one.
    ///
    /// # Errors
    ///
    /// `InvalidInput` for a payload past [`MAX_PACKET`], and whatever the
    /// stream reported.
    pub(crate) fn send(&mut self, payload: &[u8]) -> io::Result<()> {
        let mut out = BytesMut::new();
        self.codec
            .encode(&mut &payload[..], &mut out)
            .map_err(codec_failed)?;
        self.stream.write_all(&out)?;
        self.stream.flush()
    }

    /// Reads until the inbox holds a whole packet, and takes its payload.
    ///
    /// Bytes that arrived after this packet — or the head of one that has not
    /// finished arriving — stay buffered for the next call.
    ///
    /// # Errors
    ///
    /// `UnexpectedEof` when the peer closed mid-packet, `InvalidData` for a
    /// sequence id that is not the one owed or a payload past [`MAX_PACKET`],
    /// and whatever the stream reported.
    pub(crate) fn read_packet(&mut self) -> io::Result<Vec<u8>> {
        let mut packet = Vec::new();
        loop {
            if self
                .codec
                .decode(&mut self.inbox, &mut packet)
                .map_err(codec_failed)?
            {
                return Ok(packet);
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
                    "the server closed the connection part-way through a packet",
                ));
            }
        }
    }
}

/// A framing failure, as the `io::Error` every routine here answers with.
///
/// `PacketCodecError` is `mysql_common`'s and does not implement
/// `std::error::Error`, so it is rendered rather than boxed. `PacketsOutOfSync`
/// is the one worth reading twice: it means the server sent a sequence id other
/// than the one owed, which is a wire this connection can no longer locate a
/// boundary in.
fn codec_failed(error: PacketCodecError) -> io::Error {
    match error {
        PacketCodecError::Io(inner) => inner,
        other => io::Error::new(io::ErrorKind::InvalidData, other.to_string()),
    }
}

/// What the server said before anything of ours was sent, kept only as long as
/// it is needed.
///
/// Owned rather than borrowed from the greeting packet because the packet's
/// buffer is gone by the time the response is composed, and because *nothing
/// here is trusted yet*: it arrived in the clear from a peer whose certificate
/// has not been checked. Only the fields the next two steps cannot proceed
/// without are kept.
#[derive(Debug)]
struct Greeting {
    /// The capability bits the server claims, already checked against
    /// [`REQUIRED_CAPABILITIES`].
    capabilities: CapabilityFlags,
    /// The challenge every plugin's response is derived from.
    nonce: Vec<u8>,
    /// The plugin the server wants, as it spelled it.
    plugin: Vec<u8>,
    /// The version, which decides `HandshakeResponse`'s collation width. Absent
    /// from a server whose banner does not parse, in which case the modern
    /// framing is assumed — every server this driver will speak to is past
    /// 5.5.3 by decades.
    server_version: (u16, u16, u16),
}

/// Reads the server's greeting off a plaintext socket.
///
/// # Errors
///
/// `ConnectionRefused` for a server that offers none of
/// [`REQUIRED_CAPABILITIES`] — including TLS — `InvalidData` for a packet that
/// is not a greeting, and the server's own refusal where it declined the
/// connection before greeting at all.
fn read_greeting<S: Read + Write>(wire: &mut Wire<S>) -> io::Result<Greeting> {
    let packet = wire.read_packet()?;
    if packet.first() == Some(&0xFF) {
        return Err(server_refusal(&packet, CapabilityFlags::empty()));
    }

    let handshake = HandshakePacket::deserialize((), &mut ParseBuf(&packet))?;
    let capabilities = handshake.capabilities();
    let missing = REQUIRED_CAPABILITIES.difference(capabilities);
    if !missing.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::ConnectionRefused,
            format!(
                "the server offers no {missing:?}, and this driver needs \
                 {REQUIRED_CAPABILITIES:?}: ADR 0067 § 3 has no spelling for a \
                 connection without TLS or without plugin authentication"
            ),
        ));
    }

    Ok(Greeting {
        capabilities,
        nonce: handshake.nonce(),
        plugin: handshake
            .auth_plugin_name_ref()
            .unwrap_or_default()
            .to_vec(),
        server_version: handshake.server_version_parsed().unwrap_or((5, 5, 3)),
    })
}

/// Writes the `SSLRequest` that turns the rest of this socket into TLS records.
///
/// It is a capability word and nothing else — no user, no database, no
/// credential — because everything identifying is owed to the encrypted half.
///
/// # Errors
///
/// Whatever the stream reported.
fn request_tls<S: Read + Write>(wire: &mut Wire<S>, greeting: &Greeting) -> io::Result<()> {
    let mut payload = Vec::new();
    SslRequest::new(
        CLIENT_CAPABILITIES.intersection(greeting.capabilities),
        MAX_PACKET,
        COLLATION,
    )
    .serialize(&mut payload);
    wire.send(&payload)
}

/// The plugin a name selects, or the refusal that name earns.
///
/// One function because a plugin is named in two places — the greeting, and an
/// `AuthSwitchRequest` that arrives after the response has gone out — and a
/// gate applied at only the first of them is not a gate: the module doc owns
/// why a server choosing the second is the case that matters.
///
/// # Errors
///
/// `ConnectionRefused`, naming what was asked for and what this driver answers.
fn plugin_or_refuse(name: &[u8]) -> io::Result<AuthPlugin<'static>> {
    match AuthPlugin::from_bytes(name) {
        AuthPlugin::CachingSha2Password => Ok(AuthPlugin::CachingSha2Password),
        AuthPlugin::MysqlNativePassword => Ok(AuthPlugin::MysqlNativePassword),
        _ => Err(io::Error::new(
            io::ErrorKind::ConnectionRefused,
            format!(
                "the server asked to authenticate with `{}`, and this driver answers \
                 `{CACHING_SHA2_PASSWORD}` and `{MYSQL_NATIVE_PASSWORD}` only: those are \
                 the two that send a proof rather than the password",
                String::from_utf8_lossy(name)
            ),
        )),
    }
}

/// What a plugin is allowed to know while it composes a response.
///
/// `is_tls_transport` is unconditionally `true` and that is a fact rather than
/// an assertion: [`request_tls`] refused every path to this point that was not
/// encrypted. `server_key_pem` is `None` because the RSA key exchange it feeds
/// is the *unencrypted* transport's answer to the same problem, and there is no
/// unencrypted transport here.
#[derive(Clone, Copy)]
struct AuthContext<'a> {
    password: &'a [u8],
    nonce: &'a [u8],
}

impl AuthContextTrait for AuthContext<'_> {
    fn pass(&self) -> &[u8] {
        self.password
    }

    fn is_ipc_transport(&self) -> bool {
        false
    }

    fn is_tls_transport(&self) -> bool {
        true
    }

    fn scramble(&self) -> &[u8] {
        self.nonce
    }

    fn server_key_pem(&self) -> Option<&[u8]> {
        None
    }
}

/// Sends the handshake response and answers the server until it says `OK`.
///
/// Returns the capability set the two ends agreed on, which every later packet
/// on this connection is decoded against.
///
/// # Errors
///
/// `ConnectionRefused` for a plugin this driver does not answer,
/// `PermissionDenied` for the server's own refusal — a wrong password, a schema
/// that does not exist — carrying its `SQLSTATE` and message, `InvalidData` for
/// a packet the protocol does not allow at that point, and whatever the stream
/// reported.
fn authenticate<S: Read + Write>(
    wire: &mut Wire<S>,
    target: &MySqlTarget<'_>,
    greeting: &Greeting,
) -> io::Result<CapabilityFlags> {
    let capabilities = CLIENT_CAPABILITIES.intersection(greeting.capabilities);
    let context = AuthContext {
        password: target.password.as_bytes(),
        nonce: &greeting.nonce,
    };

    let plugin = plugin_or_refuse(&greeting.plugin)?;
    let mut exchange = AuthProc::init(&plugin)
        .map_err(|e| io::Error::new(io::ErrorKind::ConnectionRefused, e.to_string()))?;
    let first = exchange
        .run(context, &greeting.nonce)
        .map_err(auth_failed)?;

    let mut payload = Vec::new();
    HandshakeResponse::new(
        first.data().map(<[u8]>::to_vec),
        greeting.server_version,
        Some(target.user.as_bytes()),
        Some(target.database.as_bytes()),
        Some(plugin),
        capabilities,
        None,
        MAX_PACKET,
    )
    .serialize(&mut payload);
    wire.send(&payload)?;

    loop {
        let packet = wire.read_packet()?;
        match packet.first() {
            // `OK`, and the only way out of this loop that is a connection.
            Some(0x00) => return Ok(capabilities),
            Some(0xFF) => return Err(server_refusal(&packet, capabilities)),
            // `AuthSwitchRequest`: the server names a different plugin, and the
            // gate applies again — this is the packet the module doc is about.
            Some(0xFE) => {
                let switch = AuthSwitchRequest::deserialize((), &mut ParseBuf(&packet))?;
                let named = plugin_or_refuse(switch.auth_plugin().as_bytes())?;
                let challenge = switch.plugin_data().to_vec();
                exchange = AuthProc::init(&named)
                    .map_err(|e| io::Error::new(io::ErrorKind::ConnectionRefused, e.to_string()))?;
                let step = exchange.run(context, &challenge).map_err(auth_failed)?;
                send_step(wire, &step)?;
            }
            // `AuthMoreData`: the plugin's own next challenge, with the leading
            // `0x01` the server escapes binary data with already stripped by
            // the deserializer — `ChallengeResponsePlugin::run` requires that.
            Some(0x01) => {
                let more = AuthMoreData::deserialize((), &mut ParseBuf(&packet))?;
                let challenge = more.data().to_vec();
                let step = exchange.run(context, &challenge).map_err(auth_failed)?;
                send_step(wire, &step)?;
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "the server sent something that is not an authentication packet",
                ));
            }
        }
    }
}

/// Writes a plugin step's packet, where it has one.
///
/// [`AuthResponse::Last`] with no packet is a plugin that has said everything
/// it will say and is waiting for the server's verdict; writing an empty packet
/// there would be a message the protocol does not expect.
///
/// The import is renamed because `mysql_common` calls a plugin's step a
/// `Response` and this module needed that word for [`Answer`] — what a *command*
/// gets back, which is the other thing a reader here would reach for.
fn send_step<S: Read + Write>(wire: &mut Wire<S>, step: &AuthResponse) -> io::Result<()> {
    match step.data() {
        Some(data) => wire.send(data),
        None => Ok(()),
    }
}

/// A plugin's own refusal, as an `io::Error`.
///
/// These are all client-side arithmetic failures — a challenge of the wrong
/// length, a password the plugin cannot represent — so they are `InvalidData`
/// rather than a permission verdict the server has not yet given.
fn auth_failed(error: mysql_common::auth::plugins::Error) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("the authentication exchange failed: {error}"),
    )
}

/// The server's `ERR` packet, worded as this crate's callers read it.
///
/// `PermissionDenied` rather than `Other` because every refusal that reaches
/// this path during a handshake is one: the credentials, the schema, or the
/// host's right to connect at all. The code and `SQLSTATE` are carried because
/// they are what an operator greps for; ADR 0067 § 8's mapping to a
/// `DbErrorKind` is a table this driver does not have yet.
fn server_refusal(packet: &[u8], capabilities: CapabilityFlags) -> io::Error {
    let Ok(err) = ErrPacket::deserialize(capabilities, &mut ParseBuf(packet)) else {
        return io::Error::new(
            io::ErrorKind::InvalidData,
            "the server sent an error packet that does not decode",
        );
    };
    if !err.is_error() {
        return io::Error::new(
            io::ErrorKind::InvalidData,
            "the server sent a progress report where an error or a result was owed",
        );
    }
    let error = err.server_error();
    let state = error
        .sql_state_ref()
        .map(|s| String::from_utf8_lossy(&s.as_bytes()).into_owned())
        .unwrap_or_else(|| "HY000".to_owned());
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        format!(
            "MySQL {} (SQLSTATE {state}): {}",
            error.error_code(),
            error.message_str()
        ),
    )
}

/// Reads the answer to a command that returns no rows — the handshake's
/// `SET time_zone`, and § 13's `COM_RESET_CONNECTION`.
///
/// [`read_answer`] is the reader and owns every packet shape, including
/// [ADR 0067 § 3](../../../docs/adr/0067-core-db.md)'s `LOCAL INFILE` refusal.
/// What this adds is the *caller's* claim: these commands have no result set,
/// so a column count arriving here is a server answering something other than
/// what was asked, and there is no reader on this path to drain it with. It is
/// refused rather than skipped, because a driver that walked away from a result
/// set it did not expect would leave the wire pointing into the middle of one.
///
/// # Errors
///
/// As [`read_answer`], plus `InvalidData` for a result set where a status was
/// owed.
pub(crate) fn read_ok<S: Read + Write>(
    wire: &mut Wire<S>,
    capabilities: CapabilityFlags,
) -> io::Result<()> {
    match read_answer(wire, capabilities)? {
        Answer::Done { .. } => Ok(()),
        Answer::Columns(count) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "the server answered a command that has no result set with one of {count} \
                 columns, and there is no reader here to drain it"
            ),
        )),
    }
}

/// What the first packet of a command's answer turned out to be.
///
/// An enum rather than two readers because **the caller cannot know which it is
/// about to get**: `COM_STMT_EXECUTE` answers with a status packet for an
/// `INSERT` and with a column count for a `SELECT`, over the same statement
/// handle, and a reader that assumed either one would leave the packet stream
/// pointing at a packet it had already mis-read. MySQL's answers are
/// self-describing in their first byte and nowhere else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    /// A status packet: the command is finished and nothing follows it.
    Done {
        /// Rows the statement changed, as the server counted them — ADR 0067
        /// § 4's `affected`.
        affected: u64,
        /// The `AUTO_INCREMENT` value the statement generated, or `0` for none.
        /// The protocol's own spelling for absence is kept rather than mapped
        /// to an `Option`, because § 4's `lastId` answers `0` for a statement
        /// that generated none and mapping twice would be two decisions.
        last_id: u64,
    },
    /// A result set of this many columns. That many column definition packets
    /// follow it, and [`read_columns`] is what takes them.
    Columns(u16),
}

/// Reads one command's first answer packet, refusing a `LOCAL INFILE` request.
///
/// The four shapes a server may put here, and how they are told apart:
///
/// - `0x00` **and at least seven bytes long** is a status packet. The length is
///   load-bearing: a length-encoded column count of zero is also a `0x00` first
///   byte, and the two are distinguished by nothing else. A result set of no
///   columns is not something a server sends, so the ambiguity is theoretical —
///   but reading the length is free and guessing is how a driver desynchronises.
/// - `0xFF` is the server's own refusal, worded by [`server_refusal`].
/// - `0xFB` is a `LOCAL INFILE` request — see below.
/// - anything else is a bare length-encoded integer: the column count.
///
/// A `0xFE` shorter than nine bytes is the deprecated EOF packet, and this
/// driver negotiates `CLIENT_DEPRECATE_EOF`, so a server that sends one is not
/// speaking the protocol both ends agreed on. That is `InvalidData` rather than
/// a shape to tolerate: the whole point of the flag is that the column
/// definitions are not followed by one, and a driver that accepted both would
/// have no way to know how many packets a result set is.
///
/// # The `LOCAL INFILE` refusal
///
/// The refusal writes the empty packet that terminates a transfer — so the
/// server is told the file is zero bytes long rather than being left waiting —
/// and then refuses the connection. **Nothing on this path can open a file**:
/// there is no filesystem call in this module, and the packet's name is read
/// only to say it in the error.
///
/// # Errors
///
/// `PermissionDenied` for the server's own error and for a `LOCAL INFILE`
/// request, `InvalidData` for anything else.
pub(crate) fn read_answer<S: Read + Write>(
    wire: &mut Wire<S>,
    capabilities: CapabilityFlags,
) -> io::Result<Answer> {
    let packet = wire.read_packet()?;
    match packet.first() {
        Some(0x00) if packet.len() >= 7 => {
            let ok = OkPacketDeserializer::<CommonOkPacket>::deserialize(
                capabilities,
                &mut ParseBuf(&packet),
            )?
            .into_inner();
            Ok(Answer::Done {
                affected: ok.affected_rows(),
                last_id: ok.last_insert_id().unwrap_or(0),
            })
        }
        Some(0xFF) => Err(server_refusal(&packet, capabilities)),
        Some(0xFB) => {
            let named = LocalInfilePacket::deserialize((), &mut ParseBuf(&packet))
                .map(|request| request.file_name_str().into_owned())
                .unwrap_or_else(|_| "an unreadable path".to_owned());
            // The empty packet is the protocol's end-of-transfer, and it is the
            // whole of what this client sends: no path was opened and no byte
            // of one is on the wire.
            wire.send(&[])?;
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!(
                    "the server asked this client to send the local file `{named}`, and \
                     ADR 0067 § 3 turns `LOCAL INFILE` off with no option to enable it: \
                     `CLIENT_LOCAL_FILES` was never offered, so a server that asks is \
                     one this connection stops talking to"
                ),
            ))
        }
        Some(0xFE) if packet.len() < 9 => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "the server sent a deprecated EOF packet on a connection that negotiated \
             `CLIENT_DEPRECATE_EOF`, so the length of a result set is no longer something \
             this driver can count",
        )),
        Some(_) => {
            let count = ParseBuf(&packet)
                .checked_eat_lenenc_int()
                .and_then(|count| u16::try_from(count).ok())
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "the server answered a command with something that is neither a \
                         result nor a status",
                    )
                })?;
            Ok(Answer::Columns(count))
        }
        None => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "the server answered a command with an empty packet",
        )),
    }
}

/// Takes the `count` column definition packets that follow a column count.
///
/// Nothing follows them: `CLIENT_DEPRECATE_EOF` is negotiated, so the first row
/// packet comes straight after the last definition and the count is the whole
/// of how a reader knows where that boundary is.
///
/// # Errors
///
/// `InvalidData` for a packet that is not a column definition, and whatever the
/// stream reported.
pub(crate) fn read_columns<S: Read + Write>(
    wire: &mut Wire<S>,
    count: u16,
) -> io::Result<Vec<Column>> {
    let mut columns = Vec::with_capacity(usize::from(count));
    for _ in 0..count {
        let packet = wire.read_packet()?;
        columns.push(Column::deserialize((), &mut ParseBuf(&packet))?);
    }
    Ok(columns)
}

/// The collation number MySQL gives every binary column, and the only thing
/// that separates a `BLOB` from a `TEXT` or a `BINARY` from a `CHAR`.
///
/// MySQL types both members of each pair the same — `MYSQL_TYPE_BLOB`,
/// `MYSQL_TYPE_STRING` — and puts the difference here, so
/// [ADR 0067 § 9](../../../docs/adr/0067-core-db.md)'s split between `tainted
/// string` and `tainted bytes` is read off the charset or it is not read at
/// all. The number is protocol constant `binary`, fixed since 4.1.
const BINARY_CHARSET: u16 = 63;

/// [ADR 0067 § 9](../../../docs/adr/0067-core-db.md)'s type map, as the Novis
/// type one MySQL column definition declares — what `Core\Db\Column::type`
/// answers for this column.
///
/// **It describes the column and never a value**, which is the whole reason it
/// reads no row: a column whose every row is `NULL` still has a type.
/// [`crate::PgColumn::column_type`] is the same question on the other driver,
/// and [`ColumnType`]'s own doc owns why describing a column and decoding one
/// of its values are two functions rather than one.
///
/// Three of § 9's rows are decided by something other than the type byte, and
/// each is a place where MySQL reuses one code for two column types:
///
/// - **`UNSIGNED` is a flag**, so `BIGINT` and `BIGINT UNSIGNED` are one type
///   code and § 9's `uint` row is the flag rather than the code.
/// - **`ENUM` and `SET` arrive as `MYSQL_TYPE_STRING`** with a flag, never as
///   their own code, and § 9 sends them to different rows — `ENUM` is `tainted
///   string`, `SET` is `array<string>` and so [`ColumnType::Other`].
/// - **`BIT(1)` is a `bool` and `BIT(n>1)` is not**, and the width is in the
///   column length; PostgreSQL draws the identical line off its type modifier.
///
/// `TINYINT(1)` is deliberately **not** `bool`. § 6 says so: MySQL has no
/// boolean of its own, the display width is not a type, and a driver that
/// guessed here would read one application's `0`/`1` flag as a `bool` and
/// another's small integer as one too.
pub(crate) fn column_type(column: &Column) -> ColumnType {
    let flags = column.flags();
    match column.column_type() {
        MyColumnType::MYSQL_TYPE_TINY
        | MyColumnType::MYSQL_TYPE_SHORT
        | MyColumnType::MYSQL_TYPE_INT24
        | MyColumnType::MYSQL_TYPE_LONG
        | MyColumnType::MYSQL_TYPE_LONGLONG
        | MyColumnType::MYSQL_TYPE_YEAR => {
            if flags.contains(ColumnFlags::UNSIGNED_FLAG) {
                ColumnType::Uint
            } else {
                ColumnType::Int
            }
        }
        MyColumnType::MYSQL_TYPE_DECIMAL | MyColumnType::MYSQL_TYPE_NEWDECIMAL => {
            ColumnType::Decimal
        }
        MyColumnType::MYSQL_TYPE_FLOAT | MyColumnType::MYSQL_TYPE_DOUBLE => ColumnType::Float,
        MyColumnType::MYSQL_TYPE_BIT if column.column_length() == 1 => ColumnType::Bool,
        MyColumnType::MYSQL_TYPE_DATE | MyColumnType::MYSQL_TYPE_NEWDATE => ColumnType::Date,
        MyColumnType::MYSQL_TYPE_TIME | MyColumnType::MYSQL_TYPE_TIME2 => ColumnType::Time,
        // § 9's zone-less row, and MySQL's `TIMESTAMP` is in it: the column
        // stores no offset, so what makes it a point in time is the zone
        // `set_session_time_zone` declared rather than anything on the wire.
        MyColumnType::MYSQL_TYPE_DATETIME
        | MyColumnType::MYSQL_TYPE_DATETIME2
        | MyColumnType::MYSQL_TYPE_TIMESTAMP
        | MyColumnType::MYSQL_TYPE_TIMESTAMP2 => ColumnType::DateTime,
        MyColumnType::MYSQL_TYPE_JSON => ColumnType::Json,
        // Both flags ride on a string type code, so they are asked before the
        // text family below and after everything that cannot carry them.
        _ if flags.contains(ColumnFlags::SET_FLAG) => ColumnType::Other,
        _ if flags.contains(ColumnFlags::ENUM_FLAG) => ColumnType::Text,
        MyColumnType::MYSQL_TYPE_VARCHAR
        | MyColumnType::MYSQL_TYPE_VAR_STRING
        | MyColumnType::MYSQL_TYPE_STRING
        | MyColumnType::MYSQL_TYPE_TINY_BLOB
        | MyColumnType::MYSQL_TYPE_MEDIUM_BLOB
        | MyColumnType::MYSQL_TYPE_LONG_BLOB
        | MyColumnType::MYSQL_TYPE_BLOB => {
            if column.character_set() == BINARY_CHARSET {
                ColumnType::Bytes
            } else {
                ColumnType::Text
            }
        }
        // § 9's last row: `BIT(n>1)`, `GEOMETRY`, a vector, and every code this
        // driver has never heard of. `Other` is a true answer rather than a
        // failure — the value still reads as a `tainted string`.
        _ => ColumnType::Other,
    }
}

/// Sends ADR 0067 § 9's declared zone as a session variable.
///
/// A numeric offset and never a zone name, because a name needs the
/// `mysql.time_zone` tables populated and they usually are not — § 9 says so,
/// and this is the one place that rule is spent.
///
/// # Errors
///
/// As [`read_ok`]. A server that refuses this refuses the connection: a
/// connection whose zone is not the declared one would decode every zone-less
/// column wrong, silently.
fn set_session_time_zone<S: Read + Write>(
    wire: &mut Wire<S>,
    capabilities: CapabilityFlags,
    seconds_east: i32,
) -> io::Result<()> {
    // `COM_QUERY`, and the only text statement this driver ever composes: its
    // one interpolated value is a number this process computed, so ADR 0067
    // § 1's no-emulated-prepares rule has nothing to bite on.
    let mut payload = vec![COM_QUERY];
    payload.extend_from_slice(
        format!("SET time_zone = '{}'", offset_literal(seconds_east)).as_bytes(),
    );
    wire.codec.reset_seq_id();
    wire.send(&payload)?;
    read_ok(wire, capabilities)
}

/// A whole number of seconds east of UTC as MySQL's `±HH:MM`.
///
/// Seconds that are not a whole minute are truncated toward zero: MySQL's
/// `time_zone` has no finer resolution, and every zone in use has been on a
/// whole minute since 1972.
fn offset_literal(seconds_east: i32) -> String {
    let sign = if seconds_east < 0 { '-' } else { '+' };
    let minutes = seconds_east.unsigned_abs() / 60;
    format!("{sign}{:02}:{:02}", minutes / 60, minutes % 60)
}

impl MySqlConn {
    /// Opens a connection: TCP, the greeting, § 3's in-band upgrade, TLS, the
    /// authentication exchange, and the declared zone.
    ///
    /// `deadline` bounds the whole of that and not one leg of it — the connect,
    /// the handshake and every authentication round trip share one clock, filed
    /// on the socket where `nvs-host` keeps it. `None` is unbounded and is for
    /// a caller that has its own bound.
    ///
    /// The connection comes back [`State::Idle`]: at a packet boundary, with a
    /// statement allowed.
    ///
    /// # Errors
    ///
    /// `ConnectionRefused` when the server will not upgrade to TLS, offers none
    /// of [`REQUIRED_CAPABILITIES`], or names an authentication plugin this
    /// driver does not answer; `PermissionDenied` for the server's own refusal;
    /// `InvalidData` for a packet the protocol does not allow at that point —
    /// or for a `tls_ca_file` that holds no certificate — `TimedOut` when the
    /// deadline passes, and whatever the socket or the TLS handshake itself
    /// reported.
    pub fn connect(
        addr: SocketAddr,
        target: &MySqlTarget<'_>,
        deadline: Option<Instant>,
    ) -> io::Result<MySqlConn> {
        let mut tcp = match deadline {
            Some(at) => {
                NvsTcp::connect_timeout(addr, at.saturating_duration_since(Instant::now()))?
            }
            None => NvsTcp::connect(addr)?,
        };
        tcp.set_deadline(deadline);

        let mut plain = Wire::new(tcp);
        let greeting = read_greeting(&mut plain)?;
        request_tls(&mut plain, &greeting)?;

        // Whatever is still buffered arrived in the clear and is carried across
        // with the codec: the greeting is the only thing a server may say
        // before the upgrade, and `read_greeting` took it.
        let mut wire = plain.upgrade(|tcp| match target.tls_ca_file {
            Some(bundle) => NvsTls::over_bundle(tcp, target.host, bundle),
            None => NvsTls::over(tcp, target.host),
        })?;

        let capabilities = authenticate(&mut wire, target, &greeting)?;
        set_session_time_zone(&mut wire, capabilities, target.time_zone)?;

        Ok(MySqlConn {
            wire,
            state: Cell::new(State::Idle),
            capabilities,
            // § 9's zone-less row is decoded a layer up, where the target is
            // gone — see the field.
            time_zone: target.time_zone,
        })
    }

    /// The zone a zone-less `DATETIME` or `TIMESTAMP` off this connection is
    /// read in, as seconds east of UTC — the same number the handshake sent the
    /// server.
    #[must_use]
    pub fn time_zone(&self) -> i32 {
        self.time_zone
    }

    /// ADR 0067 § 1's two round trips for one statement, and the columns its
    /// result set turned out to have.
    ///
    /// The two-line delegation the playbook prescribes: [`start_statement`] is
    /// where the sequencing lives, because a method on `MySqlConn` can only be
    /// reached through a real socket and a real certificate and so cannot be
    /// unit-tested at all.
    ///
    /// The result borrows the connection until it ends, which is
    /// [`MySqlRows`]' whole point: ADR 0067 § 4's one-statement-at-a-time rule
    /// is not a check this method performs but a borrow the caller cannot get
    /// around.
    ///
    /// # Errors
    ///
    /// As [`start_statement`].
    pub fn query(
        &mut self,
        sql: &str,
        params: &[Option<&[u8]>],
    ) -> io::Result<MySqlRows<'_, NvsTls<NvsTcp>>> {
        start_statement(&mut self.wire, &self.state, self.capabilities, sql, params)
    }

    /// [ADR 0067 § 13](../../../docs/adr/0067-core-db.md)'s reset, before this
    /// connection may be handed to another request.
    ///
    /// Takes `self` by value for [`crate::conn::PgConn::reset`]'s reason: a reset
    /// that failed must not be able to hand a connection back, and a signature
    /// that borrowed would let a caller ignore the `Err` and pool it anyway.
    ///
    /// # Errors
    ///
    /// As [`read_ok`]. The connection is consumed either way.
    pub fn reset(mut self) -> io::Result<MySqlConn> {
        reset_session(&mut self.wire, self.capabilities, self.time_zone)?;
        Ok(self)
    }
}

impl Drop for MySqlConn {
    /// Says goodbye rather than vanishing — [`say_goodbye`] has the reasoning.
    fn drop(&mut self) {
        say_goodbye(&mut self.wire);
    }
}

/// `COM_RESET_CONNECTION`, and the one session variable it clears that this
/// driver put there.
///
/// § 13 calls this reset "atomic and complete", which is what makes it the
/// right primitive and also what obliges the second half: it drops session
/// variables, so ADR 0067 § 9's declared zone has to be sent again or the next
/// request decodes every zone-less column in UTC while believing otherwise.
/// The prepared-statement cache goes with it too — § 13 names that as the real
/// asymmetry with PostgreSQL, and it is the protocol's rather than a choice.
///
/// Free and generic in the stream for this crate's usual reason: a
/// `Wire<NvsTls<NvsTcp>>` needs a socket and a certificate that no unit test
/// has.
///
/// # Errors
///
/// As [`read_ok`], for either of the two commands.
fn reset_session<S: Read + Write>(
    wire: &mut Wire<S>,
    capabilities: CapabilityFlags,
    seconds_east: i32,
) -> io::Result<()> {
    wire.codec.reset_seq_id();
    wire.send(&[COM_RESET_CONNECTION])?;
    read_ok(wire, capabilities)?;
    set_session_time_zone(wire, capabilities, seconds_east)
}

/// A statement the server holds, and what it will want and give back.
///
/// The two counts are the server's own and not this driver's reading of the
/// SQL: `num_params` is what `COM_STMT_EXECUTE` must supply and `num_columns`
/// is what the *prepare* said the result set would be. The second is not what
/// the execution answers with — a statement whose result set depends on the
/// data returns `0` here and a real count there — so it is carried for the
/// definition packets the prepare itself sends and nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Prepared {
    /// The handle `COM_STMT_EXECUTE` and `COM_STMT_CLOSE` name.
    pub(crate) statement_id: u32,
    /// How many parameters the server will read out of an execution.
    pub(crate) params: u16,
}

/// ADR 0067 § 1's first round trip: `COM_STMT_PREPARE`.
///
/// **This is the round trip § 1 says is recorded rather than hidden.** A
/// statement's first execution in a request costs two — this one and
/// [`execute`] — where PostgreSQL's extended protocol pays nothing extra, and
/// the honest answer to that asymmetry is a statement cache (§ 1, and this
/// crate's own next slice), never an emulated prepare that interpolates the
/// value into the SQL to save a packet.
///
/// The prepare's own column and parameter definition packets are read and
/// dropped. They have to be read — they are packets in the stream and the next
/// command cannot start until the wire is at a boundary — and nothing wants
/// them: what a caller decodes rows against is the *execution's* definitions,
/// which arrive again and are the ones that are true.
///
/// Free and generic in the stream for this crate's usual reason, which
/// `crate::pg`'s `start_statement` states: a `Wire<NvsTls<NvsTcp>>` needs a
/// socket and a certificate that no unit test has.
///
/// # Errors
///
/// As [`read_answer`], plus `InvalidData` for a first packet that is not a
/// `COM_STMT_PREPARE_OK`.
pub(crate) fn prepare<S: Read + Write>(
    wire: &mut Wire<S>,
    capabilities: CapabilityFlags,
    sql: &str,
) -> io::Result<Prepared> {
    let mut payload = vec![COM_STMT_PREPARE];
    payload.extend_from_slice(sql.as_bytes());
    // Every command starts a new packet sequence, and a stale counter is a wire
    // the codec cannot find a boundary in.
    wire.codec.reset_seq_id();
    wire.send(&payload)?;

    let packet = wire.read_packet()?;
    if packet.first() == Some(&0xFF) {
        return Err(server_refusal(&packet, capabilities));
    }
    let stmt = StmtPacket::deserialize((), &mut ParseBuf(&packet)).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "the server answered `COM_STMT_PREPARE` with neither a prepared statement nor \
             an error",
        )
    })?;

    // Read past both definition runs. `CLIENT_DEPRECATE_EOF` is negotiated, so
    // there is no terminator after either one and the counts are the whole of
    // how many packets this is.
    for _ in 0..stmt.num_params() {
        wire.read_packet()?;
    }
    for _ in 0..stmt.num_columns() {
        wire.read_packet()?;
    }

    Ok(Prepared {
        statement_id: stmt.statement_id(),
        params: stmt.num_params(),
    })
}

/// ADR 0067 § 1's second round trip: `COM_STMT_EXECUTE`, over the binary
/// protocol.
///
/// **Every parameter goes in the packet and none of them goes in the SQL.**
/// That is § 1's no-emulated-prepares rule as a property of this function
/// rather than a claim about it: the SQL was sent by [`prepare`] and is not an
/// argument here at all, so there is no string for a value to be spliced into.
/// Values are bound as length-encoded strings and the server casts each to its
/// column's type, which is what the binary protocol does with a
/// `MYSQL_TYPE_VAR_STRING` parameter and is not an escaping decision — no byte
/// of a parameter is ever parsed as SQL.
///
/// `params` is [`crate::pg`]'s shape — `None` is SQL `NULL`, and the null
/// bitmap is where it is written rather than a sentinel in the data.
///
/// # Errors
///
/// `InvalidInput` when the count does not match what the prepare said the
/// statement wants, or when the bound values are too large for one packet;
/// otherwise as [`read_answer`].
pub(crate) fn execute<S: Read + Write>(
    wire: &mut Wire<S>,
    capabilities: CapabilityFlags,
    stmt: Prepared,
    params: &[Option<&[u8]>],
) -> io::Result<Answer> {
    if params.len() != usize::from(stmt.params) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "the statement takes {} parameter(s) and {} were bound — ADR 0067 § 5's \
                 rewriter is what keeps those two numbers equal, and the `inList` expansion \
                 has already moved the arity by the time the SQL reaches here",
                stmt.params,
                params.len()
            ),
        ));
    }

    let bound: Vec<MyValue> = params
        .iter()
        .map(|param| match param {
            Some(bytes) => MyValue::Bytes(bytes.to_vec()),
            None => MyValue::NULL,
        })
        .collect();
    let (request, as_long_data) =
        ComStmtExecuteRequestBuilder::new(stmt.statement_id).build(&bound);
    if as_long_data {
        // `COM_STMT_SEND_LONG_DATA` is the protocol's answer and this driver
        // does not write it: a parameter that large is a value ADR 0067 § 4's
        // one-statement-at-a-time shape has nowhere to stream from, and a
        // refusal an operator can read beats a packet the server rejects.
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "the bound parameters do not fit in one packet, and this driver does not send \
             a parameter in pieces",
        ));
    }
    let mut payload = Vec::new();
    request.serialize(&mut payload);

    wire.codec.reset_seq_id();
    wire.send(&payload)?;
    read_answer(wire, capabilities)
}

/// One statement, end to end: prepare, execute, and stop at the first row.
///
/// The shape is [`crate::pg`]'s `start_statement` and for its reasons — free
/// and generic in the stream so a unit test can script a server for it, and
/// taking the busy state by reference so ADR 0067 § 4's one-statement-at-a-time
/// rule is enforced here rather than by each caller remembering to.
///
/// It stops at the row boundary deliberately. The column definitions are the
/// last thing whose count is known ahead of time, and a binary row decoder is
/// what reads past them — so this returns with the wire pointing at the first
/// row packet and the connection [`State::Streaming`], which is the state § 4
/// refuses a second statement in.
///
/// # Errors
///
/// `InvalidInput` for a statement written to a connection that is not idle, and
/// for [`execute`]'s parameter-count mismatch; otherwise as [`read_answer`]. A
/// write that failed part-way leaves the connection [`State::Poisoned`],
/// because a half-written packet is not a boundary anything can be found from.
pub(crate) fn start_statement<'a, S: Read + Write>(
    wire: &'a mut Wire<S>,
    state: &'a Cell<State>,
    capabilities: CapabilityFlags,
    sql: &str,
    params: &[Option<&[u8]>],
) -> io::Result<MySqlRows<'a, S>> {
    if !state.get().may_start_statement() {
        return Err(crate::pg::second_statement(state));
    }

    state.set(State::Executing);
    let stmt = match prepare(wire, capabilities, sql) {
        Ok(stmt) => stmt,
        Err(e) => return Err(poison_on_write(state, e)),
    };
    let answer = match execute(wire, capabilities, stmt, params) {
        Ok(answer) => answer,
        Err(e) => return Err(poison_on_write(state, e)),
    };

    match answer {
        Answer::Done { affected, last_id } => {
            state.set(State::Idle);
            Ok(MySqlRows {
                wire,
                state,
                capabilities,
                columns: Arc::from(Vec::new()),
                rows: 0,
                affected,
                last_id,
                ended: true,
            })
        }
        Answer::Columns(count) => match read_columns(wire, count) {
            Ok(columns) => {
                state.set(State::Streaming);
                Ok(MySqlRows {
                    wire,
                    state,
                    capabilities,
                    columns: Arc::from(columns),
                    rows: 0,
                    affected: 0,
                    last_id: 0,
                    ended: false,
                })
            }
            Err(e) => Err(poison_on_write(state, e)),
        },
    }
}

/// One row, already decoded out of the packet the wire framed it from.
///
/// **Eager where [`crate::PgRow`] is lazy, and the protocol is what decides
/// that.** PostgreSQL's `DataRow` is a run of length-prefixed bodies, so a
/// column can be sliced out by walking the prefixes and nothing has to be
/// parsed to reach the next one. A binary row has no such prefix: a value's
/// width comes from its *column's declared type*, so finding column five means
/// decoding columns zero to four. Once that walk is unavoidable, keeping the
/// results is free and re-walking per column is what would cost.
///
/// The values are `mysql_common`'s own [`MyValue`], which is
/// [ADR 0132 § 2](../../../docs/adr/0132-a-driver-is-a-sans-io-codec-over-the-parking-stream.md)'s
/// borrowed codec answering in its own vocabulary. Turning one into the Novis
/// value [ADR 0067 § 9](../../../docs/adr/0067-core-db.md)'s table names is the
/// slice after this one — the same boundary [`crate::PgColumn::decode`] sits on
/// for the other driver, and the reason [`column_type`] is here while no
/// `decode` is.
pub struct MySqlRow {
    values: Vec<MyValue>,
}

impl std::fmt::Debug for MySqlRow {
    /// How many columns, and none of their values: a row in flight is one
    /// request's data — the rule [`Wire`]'s own rendering follows.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MySqlRow")
            .field("columns", &self.values.len())
            .finish_non_exhaustive()
    }
}

impl MySqlRow {
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

    /// Column `index`'s value, or `None` where the row has no such column.
    ///
    /// SQL `NULL` is [`MyValue::NULL`] and not this `None`: the two are
    /// different facts — a column that was null, and a column that was never
    /// described — and § 9's `?T` is only the first of them.
    #[must_use]
    pub fn value(&self, index: usize) -> Option<&MyValue> {
        self.values.get(index)
    }
}

/// Decodes one binary row packet against the definitions it belongs to.
///
/// The walk `mysql_common`'s own `RowDeserializer` performs, written out here
/// for one reason: **that deserializer cannot represent § 9's `uint` row.** Its
/// integer path reads the right width and the right signedness and then packs
/// every result into `Value::Int(x as i64)`, so a `BIGINT UNSIGNED` past
/// `i64::MAX` — the half of that range § 9 names `uint` precisely because PHP
/// loses it to a float — comes back negative. The bits survive the cast, so the
/// fix is to put them back under the type the column declared, and doing it
/// here is what keeps [`MySqlRows::column_type`] and [`MySqlRow::value`]
/// agreeing: a column that describes as `uint` yields a `UInt`, with no
/// reinterpretation left for a caller to remember.
///
/// # Errors
///
/// `InvalidData` for a row whose bitmap or values do not add up against the
/// definitions — a packet that is not the row those columns describe.
fn decode_row(columns: &[Column], packet: &[u8]) -> io::Result<MySqlRow> {
    let mut buf = ParseBuf(packet);
    // The `0x00` that said this is a row rather than the end of the stream.
    buf.checked_eat_u8();
    let bitmap =
        NullBitmap::<ServerSide, std::borrow::Cow<'_, [u8]>>::deserialize(columns.len(), &mut buf)?;

    let mut values = Vec::with_capacity(columns.len());
    for (index, column) in columns.iter().enumerate() {
        if bitmap.is_null(index) {
            values.push(MyValue::NULL);
            continue;
        }
        let value = ValueDeserializer::<BinValue>::deserialize(
            (column.column_type(), column.flags()),
            &mut buf,
        )?
        .0;
        values.push(match value {
            MyValue::Int(bits) if column_type(column) == ColumnType::Uint => {
                MyValue::UInt(bits.cast_unsigned())
            }
            other => other,
        });
    }
    Ok(MySqlRow { values })
}

/// A statement's result, and the rows still to come out of it.
///
/// [`crate::PgRows`]' shape, and it is the same borrow for the same reason: the
/// handle holds the wire and the busy state, so the connection is unusable for
/// anything else until the stream ends — which is ADR 0067 § 4's
/// one-statement-at-a-time rule enforced by the type system rather than by a
/// check every caller has to remember.
///
/// A statement with no result set answers one of these too, already ended: its
/// [`Self::next_row`] is `None` on the first call and [`Self::affected`] is the
/// number the server's status packet carried.
pub struct MySqlRows<'a, S: Read + Write = NvsTls<NvsTcp>> {
    wire: &'a mut Wire<S>,
    state: &'a Cell<State>,
    capabilities: CapabilityFlags,
    /// Shared rather than borrowed because `mysql_common`'s binary row
    /// deserializer takes exactly this: an `Arc<[Column]>` per row, which is a
    /// refcount bump and not a copy of the definitions.
    columns: Arc<[Column]>,
    rows: u64,
    affected: u64,
    last_id: u64,
    ended: bool,
}

impl<S: Read + Write> std::fmt::Debug for MySqlRows<'_, S> {
    /// The shape of the result and where the wire is, and nothing that arrived.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MySqlRows")
            .field("columns", &self.columns.len())
            .field("state", &self.state.get())
            .finish_non_exhaustive()
    }
}

impl<S: Read + Write> MySqlRows<'_, S> {
    /// The result set's column definitions, empty for a statement that returns
    /// none.
    #[must_use]
    pub fn columns(&self) -> &[Column] {
        &self.columns
    }

    /// Column `index`'s Novis type, per [`column_type`], or `None` where the
    /// result set has no such column.
    #[must_use]
    pub fn column_type(&self, index: usize) -> Option<ColumnType> {
        self.columns.get(index).map(column_type)
    }

    /// [ADR 0067 § 4](../../../docs/adr/0067-core-db.md)'s affected-row count,
    /// once the stream has ended, and `None` while rows may still arrive.
    ///
    /// Two numbers under one name, and which one it is follows the statement:
    /// for a statement with no result set it is what the server's status packet
    /// said it changed, and for one with a result set it is how many rows came
    /// back. PostgreSQL's `SELECT 2` tag says the second of those, so the two
    /// drivers agree on what `affected` means for a `SELECT` without either of
    /// them inventing a count.
    #[must_use]
    pub fn affected(&self) -> Option<u64> {
        self.ended.then_some(self.affected)
    }

    /// [ADR 0067 § 4](../../../docs/adr/0067-core-db.md)'s `lastId` — the
    /// `AUTO_INCREMENT` value this statement generated, `0` for none — once the
    /// stream has ended.
    ///
    /// MySQL puts it in the status packet, so unlike PostgreSQL it belongs to
    /// the write that produced it with no `RETURNING` clause to ask for. A
    /// statement that returned a result set has none, and answers `0`.
    #[must_use]
    pub fn last_id(&self) -> Option<u64> {
        self.ended.then_some(self.last_id)
    }

    /// The next row, or `None` once the stream has ended.
    ///
    /// **The first byte says which of three things arrived, with no ambiguity
    /// to weigh**, unlike [`read_answer`]'s four shapes: a binary row always
    /// opens `0x00`, the terminator is the `0xFE` status packet
    /// `CLIENT_DEPRECATE_EOF` promises instead of an EOF packet, and `0xFF` is
    /// the server's own error. A length is not read here because none is needed
    /// — the collision `read_answer` reads one for is between a status packet
    /// and a column count, and neither of those can be at this point in the
    /// stream.
    ///
    /// Ending the stream is what returns the connection to [`State::Idle`].
    /// Deliberately not `Iterator::next`, for [`crate::PgRows::next_row`]'s
    /// reason: every call can fail, and an `Option` would have to swallow it.
    ///
    /// # Errors
    ///
    /// The server's own error, which still ends the stream cleanly and leaves
    /// the connection idle; `InvalidData` for a packet the protocol does not
    /// allow here, or for a second result set, which poison it; and whatever
    /// the stream reported.
    pub fn next_row(&mut self) -> io::Result<Option<MySqlRow>> {
        // The state is the only bookkeeping: anything that ended this stream —
        // a terminator, a server error, a poisoning — has already left it.
        if self.state.get() != State::Streaming {
            return Ok(None);
        }

        let packet = match self.wire.read_packet() {
            Ok(packet) => packet,
            Err(e) => return Err(poison_on_write(self.state, e)),
        };
        match packet.first() {
            Some(0x00) => {
                let row = decode_row(&self.columns, &packet)
                    .map_err(|e| poison_on_write(self.state, e))?;
                self.rows += 1;
                Ok(Some(row))
            }
            Some(0xFE) => {
                let terminator = OkPacketDeserializer::<ResultSetTerminator>::deserialize(
                    self.capabilities,
                    &mut ParseBuf(&packet),
                )
                .map_err(|e| poison_on_write(self.state, e))?
                .into_inner();
                self.ended = true;
                self.affected = self.rows;
                if terminator
                    .status_flags()
                    .contains(StatusFlags::SERVER_MORE_RESULTS_EXISTS)
                {
                    // A stored procedure's second result set. There is no
                    // reader here to drain it with and no surface in ADR 0067
                    // § 4 to hand it to, and walking away from packets that are
                    // still coming is what leaves the wire pointing into the
                    // middle of one — [`read_ok`] refuses the same thing for
                    // the same reason.
                    return Err(poison_on_write(
                        self.state,
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            "the server has a second result set for this statement, and ADR \
                             0067 § 4's one-statement-at-a-time surface has nowhere to put it",
                        ),
                    ));
                }
                self.state.set(State::Idle);
                Ok(None)
            }
            Some(0xFF) => {
                // A refused statement is still a statement that ran, and the
                // error packet arrived whole: the wire is at a boundary, so
                // `poison_on_write` leaves the connection idle rather than
                // poisoned.
                self.ended = true;
                self.affected = self.rows;
                Err(poison_on_write(
                    self.state,
                    server_refusal(&packet, self.capabilities),
                ))
            }
            _ => Err(poison_on_write(
                self.state,
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "the server sent something that is neither a binary row, the end of a \
                     result set, nor an error",
                ),
            )),
        }
    }
}

impl<S: Read + Write> Drop for MySqlRows<'_, S> {
    /// Abandonment, and it is the ordinary case rather than an error.
    ///
    /// The rows are coming whether or not anybody reads them, so draining to
    /// the terminator is what returns the connection to the pool instead of
    /// closing it — [`crate::PgRows`]' `Drop` and ADR 0132 § 4's rule for both.
    /// A read that fails on the way poisons the connection through the same
    /// helper every other read here uses, and the loop ends because that
    /// leaves [`State::Streaming`].
    fn drop(&mut self) {
        while self.state.get() == State::Streaming {
            if self.next_row().is_err() {
                break;
            }
        }
    }
}

/// Files a failure against the connection, poisoning it unless the failure is
/// one the server worded.
///
/// The split is the difference between "this statement did not work" and "this
/// wire is no longer a sequence of packets". A `PermissionDenied` is the
/// server's own error packet, which arrived whole and left the connection at a
/// boundary — the next statement on it is fine. Anything else reached here with
/// the stream in a position nothing has proven, and § 4's answer to a boundary
/// that cannot be proven is poison.
fn poison_on_write(state: &Cell<State>, error: io::Error) -> io::Error {
    if error.kind() == io::ErrorKind::PermissionDenied {
        state.set(State::Idle);
    } else {
        state.set(State::Poisoned);
    }
    error
}

/// The last thing a destroyed connection writes: `COM_QUIT`, best effort.
///
/// [`crate::pg`]'s `say_goodbye` and for its reasons: the server closes its own
/// side instead of discovering a reset socket, which is one fewer error line in
/// its log per connection. Best effort by definition — the connection is going
/// away whatever the write reports, so there is nobody left to tell.
fn say_goodbye<S: Read + Write>(wire: &mut Wire<S>) {
    wire.codec.reset_seq_id();
    let _ = wire.send(&[COM_QUIT]);
}

#[cfg(test)]
mod tests {
    use sha2::{Digest, Sha256};

    use super::{
        AuthContext, CLIENT_CAPABILITIES, COLLATION, Greeting, MySqlTarget, MyValue, Prepared,
        State, Wire, authenticate, column_type, execute, offset_literal, read_greeting, read_ok,
        request_tls, start_statement,
    };
    use crate::conn::ColumnType as NovisType;
    use mysql_common::constants::{CapabilityFlags, ColumnFlags, ColumnType, StatusFlags};
    use mysql_common::packets::{Column, ComStmtExecuteRequestBuilder, HandshakePacket};
    use mysql_common::proto::MySerialize;
    use std::cell::Cell;
    use std::io;

    /// The nonce every case challenges with, and the password every case
    /// refuses to find on the wire.
    const NONCE: &[u8; 20] = b"NR3HP:qIYa_9=db?Sd{`";
    const PASSWORD: &str = "correct-horse-battery";

    /// A server that answers the client rather than a script.
    ///
    /// [`crate::pg`]'s `Peer` and for its reason: an authentication exchange's
    /// every packet depends on the one before it, so a canned transcript could
    /// only assert that we send *something*. A flush is the message boundary —
    /// every write in this module is `write_all` then `flush` — so the closure
    /// is handed exactly what the driver considers one send.
    struct Peer<F: FnMut(&[u8]) -> Vec<u8>> {
        server: F,
        pending: Vec<u8>,
        inbound: Vec<u8>,
        read: usize,
        /// Every flushed group, for the assertions that are about what we sent.
        sent: Vec<Vec<u8>>,
    }

    impl<F: FnMut(&[u8]) -> Vec<u8>> Peer<F> {
        fn new(server: F) -> Peer<F> {
            Peer {
                server,
                pending: Vec::new(),
                inbound: Vec::new(),
                read: 0,
                sent: Vec::new(),
            }
        }
    }

    impl<F: FnMut(&[u8]) -> Vec<u8>> io::Write for Peer<F> {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.pending.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            let message = std::mem::take(&mut self.pending);
            self.inbound.extend_from_slice(&(self.server)(&message));
            self.sent.push(message);
            Ok(())
        }
    }

    impl<F: FnMut(&[u8]) -> Vec<u8>> io::Read for Peer<F> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let left = &self.inbound[self.read..];
            let take = left.len().min(buf.len());
            buf[..take].copy_from_slice(&left[..take]);
            self.read += take;
            Ok(take)
        }
    }

    /// One packet: a three-byte little-endian length, the sequence id, the
    /// payload.
    fn packet(seq: u8, payload: &[u8]) -> Vec<u8> {
        let len = u32::try_from(payload.len()).expect("a test packet fits in a u32");
        let mut out = len.to_le_bytes()[..3].to_vec();
        out.push(seq);
        out.extend_from_slice(payload);
        out
    }

    /// A server greeting naming `plugin`, with `capabilities` on top of the set
    /// every modern server offers.
    fn greeting_bytes(plugin: &str, capabilities: CapabilityFlags) -> Vec<u8> {
        // A real server's second scramble field carries a trailing NUL, and
        // the length byte in front of it counts that byte — a 12-byte tail
        // here would make the *plugin name* start one byte late, which is
        // exactly the shape a hand-rolled greeting gets wrong.
        let mut tail = NONCE[8..].to_vec();
        tail.push(0);

        let mut payload = Vec::new();
        HandshakePacket::new(
            10,
            &b"8.0.36"[..],
            42,
            NONCE[..8].try_into().expect("eight bytes of nonce"),
            Some(tail),
            capabilities,
            COLLATION,
            StatusFlags::empty(),
            Some(plugin.as_bytes()),
        )
        .serialize(&mut payload);
        packet(0, &payload)
    }

    /// Everything a stock MySQL 8 offers that this driver cares about.
    fn server_capabilities() -> CapabilityFlags {
        CLIENT_CAPABILITIES.union(CapabilityFlags::CLIENT_LOCAL_FILES)
    }

    /// The target every case authenticates as.
    fn target() -> MySqlTarget<'static> {
        MySqlTarget {
            host: "db.example.internal",
            user: "novis",
            password: PASSWORD,
            database: "shop",
            tls_ca_file: None,
            time_zone: 0,
        }
    }

    /// `caching_sha2_password`'s fast-path proof, computed here rather than
    /// borrowed from the codec: a test that called the same function could pass
    /// on a bug the two shared.
    ///
    /// `XOR(SHA256(password), SHA256(SHA256(SHA256(password)), nonce))`.
    fn expected_proof(password: &[u8], nonce: &[u8]) -> Vec<u8> {
        let once = Sha256::digest(password);
        let twice = Sha256::digest(once);
        let mut hasher = Sha256::new();
        hasher.update(twice);
        hasher.update(nonce);
        let salted = hasher.finalize();
        once.iter().zip(salted).map(|(a, b)| a ^ b).collect()
    }

    /// Every byte this client wrote, flattened — what the security assertions
    /// search.
    fn everything_sent<F: FnMut(&[u8]) -> Vec<u8>>(peer: &Peer<F>) -> Vec<u8> {
        peer.sent.iter().flatten().copied().collect()
    }

    /// ADR 0067 § 3's third default and § 1's proof-not-password rule, in the
    /// one exchange that decides both.
    ///
    /// Four properties of the same handshake response, because they are four
    /// readings of one packet and splitting them would script the same server
    /// four times: the collation is `utf8mb4`, the scramble is the plugin's
    /// proof rather than anything derivable from the password by looking at it,
    /// the password's own bytes are nowhere in what was sent, and
    /// `CLIENT_LOCAL_FILES` is not among the capabilities claimed.
    #[test]
    fn the_mysql_handshake_forces_utf8mb4_and_sends_a_proof_rather_than_the_password() {
        let mut step = 0;
        let mut wire = Wire::new(Peer::new(move |_sent: &[u8]| {
            step += 1;
            match step {
                // The `SSLRequest`, which is answered with nothing: a real
                // server's next words arrive over TLS.
                1 => Vec::new(),
                // The handshake response earns an `OK`, and then the
                // `SET time_zone` does too.
                _ => packet(u8::try_from(step).expect("a small step count") + 1, &[0x00]),
            }
        }));
        wire.inbox.extend_from_slice(&greeting_bytes(
            super::CACHING_SHA2_PASSWORD,
            server_capabilities(),
        ));

        let greeting = read_greeting(&mut wire).expect("the greeting decodes");
        request_tls(&mut wire, &greeting).expect("the upgrade request goes out");
        let agreed = authenticate(&mut wire, &target(), &greeting).expect("the server says OK");

        let response = wire.peer().sent[1].clone();
        // Past the four-byte header, `HandshakeResponse` is a capability word,
        // a max-packet word, then the collation byte.
        let claimed = CapabilityFlags::from_bits_retain(u32::from_le_bytes(
            response[4..8].try_into().expect("a capability word"),
        ));
        assert!(
            !claimed.contains(CapabilityFlags::CLIENT_LOCAL_FILES),
            "the client claimed `CLIENT_LOCAL_FILES` at a server that offered it"
        );
        assert!(
            !agreed.contains(CapabilityFlags::CLIENT_LOCAL_FILES),
            "the agreed capability set carries `CLIENT_LOCAL_FILES`"
        );
        assert_eq!(
            response[12], COLLATION,
            "the connection was opened in a collation that is not utf8mb4"
        );

        let sent = everything_sent(wire.peer());
        assert!(
            !sent
                .windows(PASSWORD.len())
                .any(|w| w == PASSWORD.as_bytes()),
            "the password itself appears in what this client sent"
        );
        let proof = expected_proof(PASSWORD.as_bytes(), NONCE);
        assert!(
            sent.windows(proof.len()).any(|w| w == proof),
            "the handshake response does not carry `caching_sha2_password`'s own proof"
        );
    }

    /// ADR 0067 § 3's TLS default, asserted where it has to hold: **before**
    /// anything identifying is composed.
    ///
    /// The bound is named on both sides — a server offering `CLIENT_SSL` is the
    /// case above, and this is the first one refused — and the refusal is
    /// checked to have cost the client nothing: not a user name, not a schema,
    /// not a proof.
    #[test]
    fn a_server_that_offers_no_tls_is_refused_before_anything_is_sent() {
        let mut wire = Wire::new(Peer::new(|_sent: &[u8]| {
            panic!("the client answered a server that offered no TLS")
        }));
        wire.inbox.extend_from_slice(&greeting_bytes(
            super::CACHING_SHA2_PASSWORD,
            server_capabilities().difference(CapabilityFlags::CLIENT_SSL),
        ));

        let refused = read_greeting(&mut wire).expect_err("a plaintext server is not a connection");
        assert_eq!(refused.kind(), io::ErrorKind::ConnectionRefused);
        assert!(
            wire.peer().sent.is_empty(),
            "the client wrote something to a server it had already refused"
        );
    }

    /// The `AuthSwitchRequest` gate: a server that asks for a
    /// password-sending plugin *after* the response has gone out is refused,
    /// and the password does not follow.
    ///
    /// This is the case the plugin roster exists for. A gate applied only to
    /// the greeting would pass this exchange, because the greeting names a
    /// plugin this driver accepts and the switch names the one it does not.
    #[test]
    fn an_auth_switch_to_a_password_sending_plugin_is_refused() {
        let mut step = 0;
        let mut wire = Wire::new(Peer::new(move |_sent: &[u8]| {
            step += 1;
            match step {
                1 => Vec::new(),
                // `AuthSwitchRequest`: 0xFE, the plugin name, the nonce.
                _ => {
                    let mut body = vec![0xFE];
                    body.extend_from_slice(b"mysql_clear_password\0");
                    body.extend_from_slice(NONCE);
                    body.push(0);
                    packet(3, &body)
                }
            }
        }));
        wire.inbox.extend_from_slice(&greeting_bytes(
            super::MYSQL_NATIVE_PASSWORD,
            server_capabilities(),
        ));

        let greeting = read_greeting(&mut wire).expect("the greeting decodes");
        request_tls(&mut wire, &greeting).expect("the upgrade request goes out");
        let refused = authenticate(&mut wire, &target(), &greeting)
            .expect_err("`mysql_clear_password` is not a plugin this driver answers");

        assert_eq!(refused.kind(), io::ErrorKind::ConnectionRefused);
        assert!(
            refused.to_string().contains("mysql_clear_password"),
            "the refusal does not name the plugin that was asked for: {refused}"
        );
        let sent = everything_sent(wire.peer());
        assert!(
            !sent
                .windows(PASSWORD.len())
                .any(|w| w == PASSWORD.as_bytes()),
            "the password was handed to a server that asked for it in the clear"
        );
    }

    /// ADR 0067 § 3's first closed hole, asserted as a property of the
    /// **client**: a malicious server can send a `LOCAL INFILE` request at any
    /// time, whatever capability the client claimed, so the refusal cannot be
    /// the absent bit alone.
    ///
    /// Two halves, named together because either alone reads green on a broken
    /// client: the call fails, *and* the only thing that went out is the empty
    /// packet the protocol terminates a transfer with — four bytes of header
    /// and no payload. A client that streamed the file would satisfy the first
    /// half by erroring afterwards.
    #[test]
    fn local_infile_is_refused_and_no_file_is_sent() {
        let mut wire = Wire::new(Peer::new(|_sent: &[u8]| Vec::new()));
        // 0xFB, then the path the server would like, with no length prefix.
        let mut request = vec![0xFB];
        request.extend_from_slice(b"/etc/passwd");
        wire.inbox.extend_from_slice(&packet(0, &request));

        let refused = read_ok(&mut wire, CLIENT_CAPABILITIES)
            .expect_err("a `LOCAL INFILE` request is not an answer this client accepts");

        assert_eq!(refused.kind(), io::ErrorKind::PermissionDenied);
        assert!(
            refused.to_string().contains("/etc/passwd"),
            "the refusal does not name the path that was asked for: {refused}"
        );
        let sent = everything_sent(wire.peer());
        assert_eq!(
            sent.len(),
            4,
            "the client wrote {} bytes where only an empty packet was owed",
            sent.len()
        );
        assert_eq!(
            &sent[..3],
            &[0, 0, 0],
            "the packet the client terminated the transfer with is not empty"
        );
    }

    /// ADR 0067 § 9's zone, as the numeric offset the section requires and
    /// never a name.
    ///
    /// A sweep rather than a line: every offset in use is a whole number of
    /// minutes, and the rendering has to hold at the sign boundary and either
    /// side of an hour.
    #[test]
    fn a_declared_zone_renders_as_a_numeric_offset_on_both_sides_of_utc() {
        assert_eq!(offset_literal(0), "+00:00");
        assert_eq!(offset_literal(3600), "+01:00");
        assert_eq!(offset_literal(-3600), "-01:00");
        assert_eq!(offset_literal(19800), "+05:30");
        assert_eq!(offset_literal(-12600), "-03:30");
        assert_eq!(offset_literal(50400), "+14:00");
        // A zone is not a clock: seconds under a minute have no rendering, so
        // they truncate toward zero rather than rounding a zone into another.
        assert_eq!(offset_literal(59), "+00:00");
        assert_eq!(offset_literal(-59), "-00:00");
    }

    /// The plugin gate is a roster and not a denylist: a plugin nobody has
    /// heard of is refused for the same reason a known-bad one is.
    #[test]
    fn an_unknown_authentication_plugin_is_refused_by_name() {
        let refused = super::plugin_or_refuse(b"auth_gssapi_client")
            .expect_err("a plugin off the roster is not answered");
        assert_eq!(refused.kind(), io::ErrorKind::ConnectionRefused);
        assert!(
            refused.to_string().contains("auth_gssapi_client"),
            "the refusal does not name the plugin: {refused}"
        );
    }

    /// The context every plugin runs against says the transport is encrypted,
    /// and that is a fact rather than an assertion: [`request_tls`] refuses
    /// every path to that point that is not.
    #[test]
    fn a_plugin_is_told_the_transport_is_encrypted_and_is_offered_no_server_key() {
        use mysql_common::auth::plugins::Context as _;

        let context = AuthContext {
            password: PASSWORD.as_bytes(),
            nonce: NONCE,
        };
        assert!(context.is_tls_transport());
        assert!(!context.is_ipc_transport());
        assert!(
            context.server_key_pem().is_none(),
            "an RSA key exchange was offered on a transport that is already encrypted"
        );
    }

    /// A `Greeting` is a value and its `Debug` is not a credential — the
    /// nonce is a server's challenge, not a secret, but the type is one a
    /// future field could be added to carelessly.
    #[test]
    fn a_greeting_renders_without_a_password_because_it_never_held_one() {
        let rendered = format!(
            "{:?}",
            Greeting {
                capabilities: CLIENT_CAPABILITIES,
                nonce: NONCE.to_vec(),
                plugin: super::CACHING_SHA2_PASSWORD.as_bytes().to_vec(),
                server_version: (8, 0, 36),
            }
        );
        assert!(!rendered.contains(PASSWORD));
        let target = format!("{:?}", target());
        assert!(
            !target.contains(PASSWORD),
            "a target rendered its password: {target}"
        );
    }

    /// `COM_STMT_PREPARE_OK`: the status byte, the handle, and the two counts.
    fn prepare_ok(statement_id: u32, columns: u16, params: u16) -> Vec<u8> {
        let mut body = vec![0x00];
        body.extend_from_slice(&statement_id.to_le_bytes());
        body.extend_from_slice(&columns.to_le_bytes());
        body.extend_from_slice(&params.to_le_bytes());
        body.push(0x00);
        body.extend_from_slice(&0_u16.to_le_bytes());
        body
    }

    /// One column definition, in the order a server writes the fields.
    ///
    /// **Built by hand, and `Column::serialize` is deliberately not used**:
    /// `mysql_common` 0.38.2 writes `column_length` before `character_set` and
    /// reads them back the other way round, so a definition it serialized comes
    /// back with those two fields swapped. Its *reader* is the one that agrees
    /// with the protocol — charset first — and it is the reader this driver
    /// runs against a real server, so a test that used the writer would be
    /// pinning the bug rather than the wire. § 9's split between `tainted
    /// string` and `tainted bytes` is read off the charset, which is exactly
    /// the field the swap corrupts.
    fn typed_column_def(
        name: &str,
        ty: ColumnType,
        flags: ColumnFlags,
        charset: u16,
        length: u32,
    ) -> Vec<u8> {
        let mut body = Vec::new();
        // Catalog, schema, table, org_table, name, org_name — each a
        // length-encoded string, and only two of them carry anything.
        for field in [b"def".as_slice(), b"", b"", b"", name.as_bytes(), b""] {
            body.push(u8::try_from(field.len()).expect("a field short enough to be one byte"));
            body.extend_from_slice(field);
        }
        body.push(0x0c);
        body.extend_from_slice(&charset.to_le_bytes());
        body.extend_from_slice(&length.to_le_bytes());
        body.push(ty as u8);
        body.extend_from_slice(&flags.bits().to_le_bytes());
        // Decimals, then the two filler bytes.
        body.extend_from_slice(&[0, 0, 0]);
        body
    }

    /// A text column under a name, for the cases that only care that the
    /// definition packets were counted and read past.
    fn column_def(name: &str) -> Vec<u8> {
        typed_column_def(
            name,
            ColumnType::MYSQL_TYPE_VAR_STRING,
            ColumnFlags::empty(),
            255,
            255,
        )
    }

    /// An `OK` packet: the header, two length-encoded numbers, the status word
    /// and the warning count.
    fn ok_packet(affected: u8, last_id: u8) -> Vec<u8> {
        vec![0x00, affected, last_id, 0x02, 0x00, 0x00, 0x00]
    }

    /// The packet that ends a result set on a connection that negotiated
    /// `CLIENT_DEPRECATE_EOF`: an `OK` packet under a `0xFE` header, whose two
    /// length-encoded numbers the protocol says to skip rather than read.
    fn result_set_end(status: u16) -> Vec<u8> {
        let mut body = vec![0xFE, 0x00, 0x00];
        body.extend_from_slice(&status.to_le_bytes());
        body.extend_from_slice(&0_u16.to_le_bytes());
        body
    }

    /// ADR 0067 § 1's two round trips, and its no-emulated-prepares rule
    /// asserted **on the wire** rather than as a claim about the code.
    ///
    /// The parameter is a value that would end the statement and start another
    /// one if it were ever spliced into SQL. What is pinned is that the two
    /// halves never meet: the first packet carries the statement text and not a
    /// byte of the value, the second carries the value and not a byte of the
    /// statement. A driver that interpolated would still answer `Done` here and
    /// would fail on exactly one of those two lines.
    ///
    /// The count of sends is § 1's other half — the cost recorded rather than
    /// hidden. Two, for a statement no cache has seen.
    #[test]
    fn a_statement_is_prepared_and_executed_and_no_parameter_reaches_the_sql() {
        const SQL: &str = "INSERT INTO t (a) VALUES (?)";
        const INJECTION: &[u8] = b"'); DROP TABLE t; --";

        let mut wire = Wire::new(Peer::new(|sent: &[u8]| {
            // The command byte is the payload's first, past the four-byte
            // header the codec wrote.
            match sent.get(4) {
                Some(0x16) => {
                    let mut out = packet(1, &prepare_ok(9, 0, 1));
                    // The one parameter's definition packet. A prepare sends
                    // them whether or not anybody wants them, so a driver that
                    // did not read past them would start its execution
                    // mid-stream.
                    out.extend_from_slice(&packet(2, &column_def("a")));
                    out
                }
                Some(0x17) => packet(1, &ok_packet(1, 7)),
                other => panic!("the driver sent command {other:?}"),
            }
        }));
        let state = Cell::new(State::Idle);

        {
            let mut rows = start_statement(
                &mut wire,
                &state,
                CLIENT_CAPABILITIES,
                SQL,
                &[Some(INJECTION)],
            )
            .expect("a prepare and an execution the server answered");

            assert_eq!(rows.affected(), Some(1));
            assert_eq!(rows.last_id(), Some(7));
            assert!(
                rows.columns().is_empty(),
                "a statement with no result set has none"
            );
            assert!(
                rows.next_row()
                    .expect("a stream that ended before it began")
                    .is_none(),
                "a statement with no result set has no row to read either"
            );
            assert_eq!(
                state.get(),
                State::Idle,
                "a statement with nothing left to read leaves the connection reusable"
            );
        }

        let sent = &wire.peer().sent;
        assert_eq!(
            sent.len(),
            2,
            "ADR 0067 § 1: a statement's first execution costs two round trips, \
             and this is where that cost is visible"
        );
        assert!(
            contains(&sent[0], SQL.as_bytes()) && !contains(&sent[0], INJECTION),
            "the prepare carried the statement and must carry no parameter"
        );
        assert!(
            contains(&sent[1], INJECTION) && !contains(&sent[1], SQL.as_bytes()),
            "the execution carried the parameter and must carry no SQL — a value \
             spliced into the statement is what ADR 0067 § 1 removes"
        );
    }

    /// A result set that a server answers three definitions and one row of,
    /// read to the terminator that gives the connection back.
    ///
    /// **Three claims meet in the one row, and each fails differently.** The
    /// null bitmap is offset by two bits because a *server* wrote it, so a
    /// driver that used the client-side offset reads the wrong column as
    /// absent. A value states no width, so `name` can only be found by having
    /// decoded `id` at the width its column declared. And § 9's `uint` row is
    /// an `UNSIGNED` flag on an ordinary `BIGINT` rather than a type code of
    /// its own, so a driver reading the code alone answers `int` and overflows
    /// on the half of that range PHP hands back as a float.
    ///
    /// The result set's own definitions are the ones the row is read against,
    /// not the prepare's: the server sends both, and the prepare's are read and
    /// dropped, which is why this case names its columns differently in each.
    #[test]
    fn a_binary_row_decodes_its_null_bitmap_and_each_value_at_its_columns_type() {
        let mut wire = Wire::new(Peer::new(|sent: &[u8]| match sent.get(4) {
            Some(0x16) => {
                let mut out = packet(1, &prepare_ok(3, 3, 0));
                out.extend_from_slice(&packet(2, &column_def("dropped")));
                out.extend_from_slice(&packet(3, &column_def("also")));
                out.extend_from_slice(&packet(4, &column_def("dropped-too")));
                out
            }
            Some(0x17) => {
                let mut out = packet(1, &[0x03]);
                out.extend_from_slice(&packet(
                    2,
                    &typed_column_def(
                        "id",
                        ColumnType::MYSQL_TYPE_LONGLONG,
                        ColumnFlags::UNSIGNED_FLAG,
                        super::BINARY_CHARSET,
                        20,
                    ),
                ));
                out.extend_from_slice(&packet(
                    3,
                    &typed_column_def(
                        "name",
                        ColumnType::MYSQL_TYPE_VAR_STRING,
                        ColumnFlags::empty(),
                        255,
                        255,
                    ),
                ));
                out.extend_from_slice(&packet(
                    4,
                    &typed_column_def(
                        "note",
                        ColumnType::MYSQL_TYPE_BLOB,
                        ColumnFlags::empty(),
                        super::BINARY_CHARSET,
                        65535,
                    ),
                ));
                // The row: the `0x00` header, a bitmap whose only set bit is
                // the third column's — bit `2 + 2`, the server-side offset —
                // then eight little-endian bytes and a length-encoded string.
                let mut row = vec![0x00, 0b0001_0000];
                row.extend_from_slice(&42_u64.to_le_bytes());
                row.extend_from_slice(&[0x03, b'a', b'd', b'a']);
                out.extend_from_slice(&packet(5, &row));
                out.extend_from_slice(&packet(6, &result_set_end(0x0002)));
                out
            }
            other => panic!("the driver sent command {other:?}"),
        }));
        let state = Cell::new(State::Idle);

        {
            let mut rows = start_statement(
                &mut wire,
                &state,
                CLIENT_CAPABILITIES,
                "SELECT id, name, note",
                &[],
            )
            .expect("a result set the server described");

            let named: Vec<String> = rows
                .columns()
                .iter()
                .map(|column| column.name_str().into_owned())
                .collect();
            assert_eq!(named, ["id", "name", "note"]);
            assert_eq!(
                state.get(),
                State::Streaming,
                "rows remain unread, which is the state § 4 refuses a second statement in"
            );
            assert_eq!(
                rows.affected(),
                None,
                "a stream that has not ended has no count"
            );

            assert_eq!(rows.column_type(0), Some(NovisType::Uint));
            assert_eq!(rows.column_type(1), Some(NovisType::Text));
            assert_eq!(rows.column_type(2), Some(NovisType::Bytes));
            assert_eq!(
                rows.column_type(3),
                None,
                "the result set has three columns"
            );

            let row = rows
                .next_row()
                .expect("a row the server sent")
                .expect("a row, not the end of the stream");
            assert_eq!(row.len(), 3);
            assert_eq!(
                row.value(0),
                Some(&MyValue::UInt(42)),
                "an `UNSIGNED BIGINT` is § 9's `uint` row, and the flag is what says so"
            );
            assert_eq!(row.value(1), Some(&MyValue::Bytes(b"ada".to_vec())));
            assert_eq!(
                row.value(2),
                Some(&MyValue::NULL),
                "the bitmap's set bit is the third column's, not the first's"
            );

            assert!(
                rows.next_row().expect("the terminator").is_none(),
                "one row and then the end of the stream"
            );
            assert_eq!(
                state.get(),
                State::Idle,
                "the terminator is what gives the connection back"
            );
            assert_eq!(rows.affected(), Some(1), "one row came back");
            assert_eq!(rows.last_id(), Some(0), "a `SELECT` generated no key");
        }

        assert_eq!(
            wire.peer().sent.len(),
            2,
            "reading a result set costs no round trip of its own: the rows were \
             already coming"
        );
    }

    /// The bound § 9's `uint` row exists for, asserted on both sides of it.
    ///
    /// `i64::MAX` and `i64::MAX + 1` are the last `BIGINT UNSIGNED` an `int`
    /// could have carried and the first it could not. Either alone reads
    /// plausibly — the low one is the same number under either type — and it is
    /// the pair that fails a driver which packs an unsigned column into a
    /// signed value, because the second comes back as `-9223372036854775808`.
    ///
    /// That is what `mysql_common`'s own row deserializer does, which is why
    /// [`super::decode_row`] is written out rather than delegated, and this
    /// case is the reason to keep it that way.
    #[test]
    fn a_bigint_unsigned_past_i64s_range_is_a_uint_and_not_a_negative_int() {
        let last = i64::MAX.cast_unsigned();
        let first = last + 1;

        let mut wire = Wire::new(Peer::new(move |sent: &[u8]| match sent.get(4) {
            Some(0x16) => {
                let mut out = packet(1, &prepare_ok(1, 1, 0));
                out.extend_from_slice(&packet(2, &column_def("n")));
                out
            }
            Some(0x17) => {
                let mut out = packet(1, &[0x01]);
                out.extend_from_slice(&packet(
                    2,
                    &typed_column_def(
                        "n",
                        ColumnType::MYSQL_TYPE_LONGLONG,
                        ColumnFlags::UNSIGNED_FLAG,
                        super::BINARY_CHARSET,
                        20,
                    ),
                ));
                for (seq, value) in [last, first].iter().enumerate() {
                    let mut row = vec![0x00, 0x00];
                    row.extend_from_slice(&value.to_le_bytes());
                    out.extend_from_slice(&packet(u8::try_from(seq).expect("two rows") + 3, &row));
                }
                out.extend_from_slice(&packet(5, &result_set_end(0x0002)));
                out
            }
            other => panic!("the driver sent command {other:?}"),
        }));
        let state = Cell::new(State::Idle);

        let mut rows = start_statement(&mut wire, &state, CLIENT_CAPABILITIES, "SELECT n", &[])
            .expect("a result set the server described");

        let mut read = Vec::new();
        while let Some(row) = rows.next_row().expect("a row or the end of the stream") {
            read.push(row.value(0).cloned().expect("a row of one column"));
        }

        assert_eq!(
            read,
            [MyValue::UInt(last), MyValue::UInt(first)],
            "the first of these is past `i64::MAX`, and a driver that packed it into \
             an `int` answers a negative number for a column § 9 makes a `uint`"
        );
    }

    /// ADR 0067 § 4's one-statement-at-a-time rule, on the state alone.
    ///
    /// It is asked here rather than beside a live result set because the borrow
    /// `MySqlRows` holds is what makes the second call unwritable in the
    /// first place, and a case that cannot be written is not a case that proves
    /// anything. What is left to pin is the other door: a connection whose
    /// state says `Streaming` refuses, and refuses **before** writing, so a
    /// busy connection never carries half a second command.
    #[test]
    fn a_second_statement_over_an_unread_result_set_is_refused_before_a_byte() {
        let mut wire = Wire::new(Peer::new(|sent: &[u8]| {
            panic!("the driver wrote {sent:?} to a connection that is already busy")
        }));
        let state = Cell::new(State::Streaming);

        let refused = start_statement(&mut wire, &state, CLIENT_CAPABILITIES, "SELECT 1", &[])
            .expect_err("a second statement over an unread result set — ADR 0067 § 4");

        assert_eq!(refused.kind(), io::ErrorKind::InvalidInput);
        assert!(
            refused.to_string().contains("one at a time"),
            "the refusal is the one sentence `crate::pg` words for both drivers: {refused}"
        );
        assert!(wire.peer().sent.is_empty());
    }

    /// A result set nobody read, dropped — and the connection comes back
    /// anyway.
    ///
    /// This is the ordinary case rather than an error: the rows are already on
    /// their way, so draining to the terminator is deterministic and is what
    /// lets § 13's pool take the connection instead of closing it. A driver
    /// that walked away would leave the next statement reading this one's rows.
    #[test]
    fn an_abandoned_result_set_is_drained_and_the_connection_comes_back() {
        let mut wire = Wire::new(Peer::new(|sent: &[u8]| match sent.get(4) {
            Some(0x16) => {
                let mut out = packet(1, &prepare_ok(7, 1, 0));
                out.extend_from_slice(&packet(2, &column_def("v")));
                out
            }
            Some(0x17) => {
                let mut out = packet(1, &[0x01]);
                out.extend_from_slice(&packet(2, &column_def("v")));
                out.extend_from_slice(&packet(3, &[0x00, 0x00, 0x01, b'a']));
                out.extend_from_slice(&packet(4, &[0x00, 0x00, 0x01, b'b']));
                out.extend_from_slice(&packet(5, &result_set_end(0x0002)));
                out
            }
            other => panic!("the driver sent command {other:?}"),
        }));
        let state = Cell::new(State::Idle);

        drop(
            start_statement(&mut wire, &state, CLIENT_CAPABILITIES, "SELECT v", &[])
                .expect("a result set the server described"),
        );

        assert_eq!(
            state.get(),
            State::Idle,
            "two unread rows and a terminator were drained, so the connection is \
             reusable rather than destroyed"
        );
    }

    /// ADR 0067 § 9's type map, as the whole table rather than a row of it.
    ///
    /// Counted rather than read off a line, because a map answering plausibly
    /// column by column is exactly what a per-case assertion cannot catch. Four
    /// pairs in here are the ones a driver gets wrong by reading the type code
    /// alone: signed against unsigned, `BIT(1)` against `BIT(8)`, `ENUM`
    /// against `SET` — which share a code and differ only by a flag — and text
    /// against binary, which share a code and differ only by the collation.
    ///
    /// `TINYINT(1)` is in here as `int`. § 6 is explicit that MySQL's display
    /// width is not a type, and a driver that read it as `bool` would answer
    /// one application's flag column correctly and another's small integer
    /// wrongly, with nothing on the wire to tell them apart.
    #[test]
    fn section_nines_type_map_is_read_off_a_column_definition_and_never_a_value() {
        let binary = super::BINARY_CHARSET;
        let utf8 = 255_u16;
        let cases: [(ColumnType, ColumnFlags, u16, u32, NovisType); 19] = [
            (
                ColumnType::MYSQL_TYPE_TINY,
                ColumnFlags::empty(),
                utf8,
                1,
                NovisType::Int,
            ),
            (
                ColumnType::MYSQL_TYPE_LONG,
                ColumnFlags::empty(),
                utf8,
                11,
                NovisType::Int,
            ),
            (
                ColumnType::MYSQL_TYPE_LONGLONG,
                ColumnFlags::empty(),
                utf8,
                20,
                NovisType::Int,
            ),
            (
                ColumnType::MYSQL_TYPE_LONGLONG,
                ColumnFlags::UNSIGNED_FLAG,
                utf8,
                20,
                NovisType::Uint,
            ),
            (
                ColumnType::MYSQL_TYPE_YEAR,
                ColumnFlags::UNSIGNED_FLAG,
                utf8,
                4,
                NovisType::Uint,
            ),
            (
                ColumnType::MYSQL_TYPE_NEWDECIMAL,
                ColumnFlags::empty(),
                utf8,
                12,
                NovisType::Decimal,
            ),
            (
                ColumnType::MYSQL_TYPE_FLOAT,
                ColumnFlags::empty(),
                binary,
                12,
                NovisType::Float,
            ),
            (
                ColumnType::MYSQL_TYPE_DOUBLE,
                ColumnFlags::empty(),
                binary,
                22,
                NovisType::Float,
            ),
            (
                ColumnType::MYSQL_TYPE_BIT,
                ColumnFlags::empty(),
                binary,
                1,
                NovisType::Bool,
            ),
            (
                ColumnType::MYSQL_TYPE_BIT,
                ColumnFlags::empty(),
                binary,
                8,
                NovisType::Other,
            ),
            (
                ColumnType::MYSQL_TYPE_DATE,
                ColumnFlags::empty(),
                binary,
                10,
                NovisType::Date,
            ),
            (
                ColumnType::MYSQL_TYPE_TIME,
                ColumnFlags::empty(),
                binary,
                10,
                NovisType::Time,
            ),
            (
                ColumnType::MYSQL_TYPE_DATETIME,
                ColumnFlags::empty(),
                binary,
                19,
                NovisType::DateTime,
            ),
            (
                ColumnType::MYSQL_TYPE_TIMESTAMP,
                ColumnFlags::empty(),
                binary,
                19,
                NovisType::DateTime,
            ),
            (
                ColumnType::MYSQL_TYPE_JSON,
                ColumnFlags::empty(),
                binary,
                4096,
                NovisType::Json,
            ),
            (
                ColumnType::MYSQL_TYPE_STRING,
                ColumnFlags::ENUM_FLAG,
                utf8,
                12,
                NovisType::Text,
            ),
            (
                ColumnType::MYSQL_TYPE_STRING,
                ColumnFlags::SET_FLAG,
                utf8,
                12,
                NovisType::Other,
            ),
            (
                ColumnType::MYSQL_TYPE_BLOB,
                ColumnFlags::empty(),
                utf8,
                65535,
                NovisType::Text,
            ),
            (
                ColumnType::MYSQL_TYPE_BLOB,
                ColumnFlags::empty(),
                binary,
                65535,
                NovisType::Bytes,
            ),
        ];

        let disagreed: Vec<String> = cases
            .iter()
            .filter_map(|(ty, flags, charset, length, novis)| {
                let column = Column::new(*ty)
                    .with_name(b"c")
                    .with_flags(*flags)
                    .with_character_set(*charset)
                    .with_column_length(*length);
                let read = column_type(&column);
                (read != *novis)
                    .then(|| format!("{ty:?}({length}) read as {read:?}, not {novis:?}"))
            })
            .collect();

        assert!(
            disagreed.is_empty(),
            "every row of § 9's table, and the four pairs in it that share a type code \
             differ only by a flag, a width or a collation: {disagreed:?}"
        );
    }

    /// The null bitmap, which is the binary protocol's answer to `NULL` and the
    /// reason a bound `null` is not a word anywhere.
    ///
    /// Both halves in one case, because either alone reads plausibly: the byte
    /// says which parameter is absent, and the payload says the absence was
    /// never spelled out as text. The count guard is here too — it is the same
    /// packet's other way of being wrong, and ADR 0067 § 5's rewriter is what
    /// keeps the two numbers equal.
    #[test]
    fn a_null_parameter_is_a_bitmap_bit_and_never_a_word_in_the_packet() {
        let mut wire = Wire::new(Peer::new(|sent: &[u8]| match sent.get(4) {
            Some(0x16) => {
                let mut out = packet(1, &prepare_ok(4, 0, 2));
                out.extend_from_slice(&packet(2, &column_def("a")));
                out.extend_from_slice(&packet(3, &column_def("b")));
                out
            }
            Some(0x17) => packet(1, &ok_packet(0, 0)),
            other => panic!("the driver sent command {other:?}"),
        }));
        let state = Cell::new(State::Idle);

        start_statement(
            &mut wire,
            &state,
            CLIENT_CAPABILITIES,
            "UPDATE t SET a = ?, b = ?",
            &[None, Some(b"kept")],
        )
        .expect("one absent parameter and one present one");

        // Past the codec's four-byte header, at the offset `mysql_common`
        // builds the bitmap at.
        let execution = &wire.peer().sent[1];
        let bitmap = execution[4 + ComStmtExecuteRequestBuilder::NULL_BITMAP_OFFSET];
        assert_eq!(
            bitmap & 0b11,
            0b01,
            "the first parameter is the absent one and the second is not: {bitmap:#010b}"
        );
        assert!(
            contains(execution, b"kept") && !contains(execution, b"NULL"),
            "a bound `null` is a bit, never a word the server would have to parse"
        );

        // The same statement, bound wrong. It is refused before anything is
        // written, so the server never sees a packet it would have to guess at.
        let before = wire.peer().sent.len();
        let miscounted = execute(
            &mut wire,
            CLIENT_CAPABILITIES,
            Prepared {
                statement_id: 4,
                params: 2,
            },
            &[None],
        )
        .expect_err("one value for a statement that wants two");
        assert_eq!(miscounted.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(wire.peer().sent.len(), before);
    }

    /// Whether `needle` appears anywhere in `haystack`, which is how the two
    /// cases above ask what did and did not reach the wire.
    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
        haystack.windows(needle.len()).any(|at| at == needle)
    }
}
