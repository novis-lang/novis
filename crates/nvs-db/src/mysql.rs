//! MySQL: reading a `[db.<name>]` block, reading the server's greeting,
//! upgrading the socket in band, authenticating with a proof rather than a
//! password, forcing the connection's charset to `utf8mb4`, and running one
//! statement over `COM_STMT_PREPARE` and `COM_STMT_EXECUTE` — the prepare only
//! the first time, which is § 1's cache.
//!
//! **The statement path runs to the end of the result set.**
//! [`start_statement`] answers a [`MySqlRows`], and [`MySqlRows::next_row`]
//! takes one binary row packet at a time until the terminator returns the
//! connection to [`State::Idle`]. [`cached_statement`] is what sits in front of
//! the prepare: a statement this connection has already run costs the single
//! round trip § 1 prices a cached re-execution at, and § 13's
//! `COM_RESET_CONNECTION` empties the cache along with the statements it names
//! — the asymmetry with PostgreSQL that § 13 calls the protocol's rather than a
//! choice.
//!
//! A statement whose rows a *held* cursor walks cannot be a borrow — the program
//! advances it from a later call, with nothing borrowed in between — so its read
//! state is parked on the connection instead ([`MySqlConn::stream`]) and
//! [`State::Streaming`] is what refuses the second statement. [`MySqlCursor`] is
//! that state, both paths carry one, and [`next_row_of`] is the single place a
//! binary row packet is read.
//!
//! **A value is finished in two places, and § 9's table says which.**
//! [`scalar`] reads one column against its definition and answers a
//! [`MySqlScalar`]; [`decode`] mints the [`Value`] for the rows that are
//! values, and the rows that are `Core\Time` instances stay components for
//! `nvs-stdlib` to build — the boundary [`crate::PgScalar`] sits on for the
//! other driver, and [`MySqlDate`] owns why it is a boundary at all.
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
//! [ADR 0132 § 2](/docs/decisions/0132.md)
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
//!    than answered. [ADR 0067 § 3](/docs/decisions/0067.md) makes
//!    `VerifyFull` the default with no spelling for turning it off, and a
//!    handshake that continued in the clear here is exactly PHP's
//!    `sslmode=prefer` under another name. This step belongs to the TCP
//!    transport: a connection over a Unix-domain socket has no network to
//!    upgrade and skips it, for the reason [`MyStream`] states.
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
//! [ADR 0067 § 3](/docs/decisions/0067.md)'s standard. That is the
//! plugin's design and MySQL 8's default; it is not a downgrade a server can
//! ask for, because the fast path and the full path are the same plugin and the
//! same verified peer.
//!
//! # `LOCAL INFILE` is not a capability this client has
//!
//! [`CLIENT_CAPABILITIES`] does not contain `CLIENT_LOCAL_FILES`, so a
//! conforming server may not ask for a file at all — that is
//! [ADR 0067 § 3](/docs/decisions/0067.md)'s first closed hole, and
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
//! [ADR 0067 § 3](/docs/decisions/0067.md)'s third default — text
//! columns arrive as valid UTF-8 by construction, which is what
//! `rule:types/bytes`'s guarantee needs —
//! and it costs nothing, where a `SET NAMES` after the fact would be a round
//! trip and a window in which one was not set.

use std::borrow::Cow;
use std::cell::Cell;
use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use bytes::BytesMut;
use mysql_common::auth::plugins::{
    AuthProc, ChallengeResponsePlugin, Context as AuthContextTrait, Response as AuthResponse,
};
use mysql_common::collations::CollationId;
use mysql_common::constants::{
    CapabilityFlags, ColumnFlags, ColumnType as MyColumnType, MariadbCapabilities, StatusFlags,
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
use nvs_config::tree::Database;
use nvs_host::net::NvsTcp;
#[cfg(unix)]
use nvs_host::net::NvsUnix;
use nvs_host::tls::NvsTls;
use nvs_runtime::{Decimal, NotDecimal, NvsStr, Tag, Value};

use crate::conn::{
    BlockError, ColumnType, DbErrorKind, Driver, Endpoint, Isolation, MySqlConn, ServerError,
    State, written_value,
};
use crate::span::QuerySpan;
use crate::sql::{StatementCache, statement_cache_for, time_zone_for};

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

/// `COM_QUERY` — the only text command this driver composes, and the whole of
/// what it carries is this module's own SQL: [`set_session_time_zone`]'s
/// declared zone, and § 7's transaction commands.
const COM_QUERY: u8 = 0x03;

/// `COM_QUIT`, the goodbye a destroyed connection writes.
const COM_QUIT: u8 = 0x01;

/// `COM_RESET_CONNECTION`, `rule:security/db-pool-reset-is-a-boundary`'s reset for this backend.
const COM_RESET_CONNECTION: u8 = 0x1F;

/// `COM_STMT_PREPARE` — `rule:core-classes/db-one-api`'s first round trip.
///
/// `COM_STMT_EXECUTE` has no constant beside this one because `mysql_common`
/// builds that packet header and all, and a second spelling of a byte it
/// already writes is a place for the two to disagree.
const COM_STMT_PREPARE: u8 = 0x16;

/// `COM_STMT_CLOSE`, which an eviction from § 1's statement cache sends.
///
/// It has a constant where `COM_STMT_EXECUTE` does not because this module
/// writes the packet itself — a command byte and a handle, with no builder in
/// `mysql_common` to disagree with.
///
/// **The one command here the server does not answer.** It deallocates the
/// statement and writes nothing back, so a driver that read for an `OK` after
/// it would take the next command's answer as this one's.
const COM_STMT_CLOSE: u8 = 0x19;

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
/// [ADR 0067 § 3](/docs/decisions/0067.md)'s forced `utf8mb4`.
const COLLATION: u8 = CollationId::UTF8MB4_GENERAL_CI as u8;

/// What this client claims it can do, and the absences are the interesting
/// half.
///
/// **`CLIENT_LOCAL_FILES` is not here**, which is `rule:core-classes/db-capabilities`'s `LOCAL INFILE`
/// closed at the only place it can be closed *before* a server asks — the
/// module doc owns the second half of that rule. **`CLIENT_COMPRESS` and
/// `CLIENT_ZSTD_COMPRESSION_ALGORITHM` are not here** either: the workspace
/// takes `mysql_common` with its compression feature off, so there is no
/// implementation behind the bit, and a compressed stream would in any case be
/// a second framing layer under a TLS session that already has one.
/// **`CLIENT_MULTI_STATEMENTS` is not here**, and that is a security bit: it is
/// what turns one injected `;` into two statements, and `rule:core-classes/db-one-api`'s
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
pub(crate) const REQUIRED_CAPABILITIES: CapabilityFlags = CapabilityFlags::CLIENT_PROTOCOL_41
    .union(CapabilityFlags::CLIENT_SECURE_CONNECTION)
    .union(CapabilityFlags::CLIENT_PLUGIN_AUTH)
    .union(CapabilityFlags::CLIENT_SSL);

/// [`REQUIRED_CAPABILITIES`] over a transport that never leaves the machine.
///
/// TLS is out of it, and only TLS. A Unix-domain socket has no network for a
/// certificate to stand between, and demanding one there would refuse every
/// server built without TLS for a threat that transport does not have —
/// [`MyStream`] owns that argument in full.
#[cfg(unix)]
pub(crate) const REQUIRED_OVER_A_SOCKET: CapabilityFlags =
    REQUIRED_CAPABILITIES.difference(CapabilityFlags::CLIENT_SSL);

/// The endpoint a socket `host` names on this driver, which is the path an
/// operator wrote and nothing derived from it.
///
/// `rule:core-classes/db-unix-socket-path`: a MySQL socket file has no naming
/// convention to derive one from, so `port` is taken and ignored. It is taken
/// at all because [`crate::pg::socket_endpoint`] needs it — the door that
/// admits a configured socket holds one of these per driver as a single
/// function pointer, and that is where the difference between the two
/// spellings is visible.
///
/// # Errors
///
/// `Unsupported` on a build with no `AF_UNIX` transport, which is
/// [`crate::conn::socket_endpoint`]'s refusal rather than a second wording of
/// it.
pub fn socket_endpoint(path: &str, _port: u16) -> io::Result<Endpoint> {
    crate::conn::socket_endpoint(path)
}

/// The socket underneath a MySQL or MariaDB connection: TLS over TCP, or a
/// Unix-domain socket carrying the protocol in the clear.
///
/// [`crate::conn::Endpoint`]'s two transports, arriving here as the two things a
/// [`Wire`] can be framed over. The framing has no opinion about which it is,
/// which is why this is a stream and not a second codec.
///
/// **There is no TLS over the local arm, and that is not a downgrade a server
/// can ask for.** A path reaches this driver only where an operator wrote one
/// into root-owned configuration
/// (`rule:config/a-unix-socket-is-admitted-only-where-an-operator-wrote-it`), it
/// names no host for a certificate to be valid for, and the bytes never leave
/// the machine — the socket's own file permissions are the boundary TLS would
/// otherwise be standing in for. The in-band upgrade is still mandatory on the
/// TCP arm, which is the one with a network in it, and a server chooses neither:
/// the transport is decided before the greeting is read.
///
/// The local arm is `#[cfg(unix)]`, as [`crate::conn::Endpoint`]'s socket arm
/// is — a link to the variant itself would be unresolvable on the build where
/// it does not exist, which is exactly the property being described — so
/// this match is exhaustive on both platforms with no arm that exists only to
/// refuse.
///
/// `clippy::large_enum_variant` is allowed, and it fires only where the local
/// arm exists. The large arm is the TLS session every networked connection
/// uses, so a `Box` would put an allocation per connect and a pointer hop per
/// read on the common path to shrink the rare one, which
/// `rule:core-classes/db-drivers-are-an-enum`'s third argument declines. What
/// it spends is a TLS session's size on each local connection, one per pooled
/// connection.
#[allow(clippy::large_enum_variant)]
pub enum MyStream {
    /// A server that named a host, reached over TCP and upgraded in band.
    Tls(NvsTls<NvsTcp>),
    /// A server that named a path, reached over a Unix-domain socket.
    #[cfg(unix)]
    Local(NvsUnix),
}

impl Read for MyStream {
    /// Whatever has arrived, from whichever socket this is.
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Tls(stream) => stream.read(buf),
            #[cfg(unix)]
            Self::Local(stream) => stream.read(buf),
        }
    }
}

impl Write for MyStream {
    /// As much of `buf` as the socket took.
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Self::Tls(stream) => stream.write(buf),
            #[cfg(unix)]
            Self::Local(stream) => stream.write(buf),
        }
    }

    /// Pushes what is held, which on the TLS arm is a record not yet sealed.
    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Tls(stream) => stream.flush(),
            #[cfg(unix)]
            Self::Local(stream) => stream.flush(),
        }
    }
}

impl std::fmt::Debug for MyStream {
    /// Which transport this is, and nothing about the socket: a `rustls` session
    /// holds key material and a buffered packet is one request's data, which is
    /// [`Wire`]'s own `Debug` and its reason.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tls(_) => f.write_str("MyStream::Tls"),
            #[cfg(unix)]
            Self::Local(_) => f.write_str("MyStream::Local"),
        }
    }
}

impl nvs_host::net::Deadline for MyStream {
    /// Bounds every wait on this connection, whichever socket carries it.
    fn set_deadline(&mut self, at: Option<Instant>) {
        match self {
            Self::Tls(stream) => stream.set_deadline(at),
            #[cfg(unix)]
            Self::Local(stream) => stream.set_deadline(at),
        }
    }

    fn deadline(&self) -> Option<Instant> {
        match self {
            Self::Tls(stream) => stream.deadline(),
            #[cfg(unix)]
            Self::Local(stream) => stream.deadline(),
        }
    }
}

/// Where one MySQL server is, and who to be there.
///
/// [`crate::pg::PgTarget`]'s shape and its split: `host` is the name the
/// certificate must be valid for, and the *address* is not here at all because
/// it was resolved by whoever checked the `db.connect` capability. A driver
/// that re-resolved the name would be connecting somewhere nobody approved.
pub struct MySqlTarget<'a> {
    /// The name the server's certificate is checked against.
    pub host: &'a str,
    /// The user to log in as. `rule:core-classes/db-capabilities` accepts `tainted` here freely: it is
    /// a length-prefixed protocol field, never parsed text.
    pub user: &'a str,
    /// The user's password, used only to derive a challenge response.
    pub password: &'a str,
    /// The schema to select on connect.
    pub database: &'a str,
    /// The PEM bundle whose anchors this server's certificate is verified
    /// against, or the compiled-in Mozilla set where the block names none.
    ///
    /// Not a way to turn verification off — `rule:core-classes/db-capabilities` has no spelling for
    /// that. What it changes is *whose* certificates are believed.
    pub tls_ca_file: Option<&'a Path>,
    /// The zone a `DATETIME` or `TIMESTAMP` off this connection is read in, as
    /// a whole number of seconds east of UTC.
    ///
    /// [ADR 0067 § 9](/docs/decisions/0067.md)'s declared zone. It is
    /// sent to the server as well as held here, so `CURRENT_TIMESTAMP` and a
    /// decoded column agree about which zone they are in —
    /// [`set_session_time_zone`] is where it goes out, and as a numeric offset
    /// rather than a name because named zones need `mysql.time_zone` populated
    /// and it usually is not.
    pub time_zone: i32,
    /// How many prepared statements this connection may keep alive on the
    /// server, [ADR 0067 § 1](/docs/decisions/0067.md)'s
    /// `statement_cache`.
    ///
    /// [`crate::PgTarget::statement_cache`]'s twin: the same reader answers it
    /// and the number means the same thing. What differs is what it buys — a
    /// hit on this driver drops a whole round trip, where PostgreSQL's extended
    /// protocol was paying nothing extra for the parse in the first place.
    pub statement_cache: usize,
}

impl<'a> MySqlTarget<'a> {
    /// One `[db.<name>]` block as this driver's target, or why it is not one.
    ///
    /// [`crate::PgTarget::resolve`]'s twin, and the twinning is the point:
    /// [ADR 0067 § 2](/docs/decisions/0067.md) makes a block a
    /// discriminated union on `driver`, so the *fields* a MySQL connection
    /// needs are the ones a PostgreSQL one needs and the refusals are one
    /// vocabulary — [`BlockError`], which lives in [`mod@crate::conn`] for that
    /// reason. What differs is which `driver` spelling this resolver accepts
    /// and which backend its refusals name.
    ///
    /// **The target borrows the block and copies nothing.** A resolved target
    /// outlives the [`MySqlConn::connect`] call that opens with it because the
    /// configuration snapshot it borrows does; owning the four strings would
    /// mean a second copy of the password — a `secret` at the language level
    /// (§ 3) — in a struct nothing zeroes.
    ///
    /// The address is not here, for the reason [`MySqlTarget`] gives: `host` is
    /// the name the certificate is checked against, and resolving it to a
    /// [`std::net::SocketAddr`] belongs to whoever checked the `db.connect`
    /// capability.
    ///
    /// § 1's `statement_cache` goes through [`statement_cache_for`], the reader
    /// PostgreSQL's block goes through, so a block sized for either driver is
    /// sized by one rule and neither connect path reaches into a config tree.
    ///
    /// # Errors
    ///
    /// [`BlockError`], in the order the checks run: the `driver` first, since a
    /// PostgreSQL block resolved as MySQL would send this handshake to a server
    /// that cannot answer it; then a field belonging to another driver; then the
    /// fields the handshake sends, each by its own key; then § 9's zone.
    pub fn resolve(block: &'a Database) -> Result<MySqlTarget<'a>, BlockError<'a>> {
        let written = block.driver.as_deref().ok_or(BlockError::NoDriver)?;
        match Driver::from_config_name(written) {
            Some(Driver::MySql) => {}
            // MariaDB is refused here rather than accepted as a dialect of
            // this one: it is its own driver, which `rule:core-classes/db-one-api` argues at length,
            // and a MariaDB block opened by this resolver would be the design
            // error that argument is about.
            Some(driver) => {
                return Err(BlockError::OtherDriver {
                    written,
                    driver,
                    expected: Driver::MySql,
                });
            }
            None => return Err(BlockError::UnknownDriver { written }),
        }

        if block.path.is_some() {
            return Err(BlockError::Unusable {
                field: "path",
                expected: Driver::MySql,
            });
        }

        let password = match (block.password.as_deref(), block.password_file.is_some()) {
            // `nvs_config::secret` materializes the file's content into
            // `password` and leaves `password_file` set, so a block with the
            // file and no value is one that never went through that pass.
            (None, true) => return Err(BlockError::SecretUnread),
            // A password may legitimately be spaces, so this one field is
            // empty only when it is *empty* — the rule `written_value`'s doc
            // owns.
            (Some(""), _) => return Err(BlockError::Blank { field: "password" }),
            (Some(password), _) => password,
            (None, false) => {
                return Err(BlockError::Missing {
                    field: "password",
                    expected: Driver::MySql,
                });
            }
        };

        let host = written_value(block.host.as_deref(), "host", Driver::MySql)?;
        let user = written_value(block.user.as_deref(), "user", Driver::MySql)?;
        // `CLIENT_CONNECT_WITH_DB` is negotiated, so the schema goes out in the
        // handshake response rather than in a `USE` afterwards — which is why
        // it is as required here as the user is.
        let database = written_value(block.database.as_deref(), "database", Driver::MySql)?;

        let Some(time_zone) = time_zone_for(block) else {
            return Err(BlockError::TimeZone {
                // `time_zone_for` answers `Some(0)` for an absent field, so
                // reaching here means the block wrote one.
                written: block.time_zone.as_deref().unwrap_or_default(),
            });
        };

        Ok(MySqlTarget {
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

impl std::fmt::Debug for MySqlTarget<'_> {
    /// Everything but the password, which is a `secret` at the language level
    /// (`rule:core-classes/db-capabilities`) and is not printed in any rendering.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MySqlTarget")
            .field("host", &self.host)
            .field("user", &self.user)
            .field("database", &self.database)
            .field("time_zone", &self.time_zone)
            .field("statement_cache", &self.statement_cache)
            .finish_non_exhaustive()
    }
}

/// The facts about a server that the framing below cannot answer for itself:
/// what to call it, which table its error codes are read against, and what it
/// claims in the second capability word.
///
/// **This is not MariaDB-as-a-flag**, which `rule:core-classes/db-one-api` rejects and this crate's
/// [`crate::maria`] exists instead of. A flag would be one connection type
/// whose *surface* — its plugin roster, its statement syntax, its error kinds —
/// forked on a bit. The two connections are separate types with separate
/// rosters; what they share is the packet framing, and framing has one place
/// where it must name the server it is framing for. Carrying that on the
/// [`Wire`] rather than on every reader's signature is what keeps
/// [`read_answer`] and the routines around it from growing a parameter each in
/// order to say `"mysql"` or `"mariadb"` at the one point either of them ever
/// says it.
pub(crate) struct Backend {
    /// What a [`ServerError`] off this connection calls the server it came
    /// from — the word an operator reads in the rendered sentence.
    pub(crate) name: &'static str,
    /// [ADR 0067 § 8](/docs/decisions/0067.md)'s table for this
    /// server's vendor codes. MariaDB's is not MySQL's, which is § 8's own
    /// sentence and the reason this field is a function rather than a bool.
    pub(crate) kind_of: fn(u16, &str) -> DbErrorKind,
    /// The extended capability word this driver claims, which is the *only*
    /// place either handshake packet's last four reserved bytes get a value —
    /// see [`agreed_extended`] for what those bytes are and why a 32-bit
    /// [`CapabilityFlags`] cannot hold them.
    ///
    /// MySQL's is empty and has to be: the bits are MariaDB's, and a MySQL
    /// server reads that field as filler it expects to be zero.
    pub(crate) extended: MariadbCapabilities,
}

/// This module's server: MySQL's name and [`kind_of`], MySQL's table.
pub(crate) const MYSQL: Backend = Backend {
    name: "mysql",
    kind_of,
    extended: MariadbCapabilities::empty(),
};

/// A MySQL connection's stream, the bytes read off it that are not yet a whole
/// packet, and the sequence-id state that says which packet is next.
///
/// Generic in the stream so the exchanges below can be asserted against a
/// server that answers rather than against a socket — the playbook's rule for
/// this crate, and the reason every routine here takes a `&mut Wire<S>` instead
/// of being an inherent method on [`MySqlConn`]. A real connection's wire is
/// the [`MyStream`] the default names.
///
/// [`crate::maria`] frames its packets with this too, through [`Wire::on`]: one
/// protocol is framed one way, and a second copy of the sequence-id discipline
/// would be a second place for it to go wrong.
pub(crate) struct Wire<S: Read + Write = MyStream> {
    stream: S,
    inbox: BytesMut,
    /// Which server is on the other end — see [`Backend`].
    pub(crate) backend: &'static Backend,
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
        Wire::on(&MYSQL, stream)
    }

    /// The same wire, framing for a named server.
    ///
    /// [`crate::maria`]'s constructor: the framing is one protocol's and the
    /// [`Backend`] is the half of it that differs.
    pub(crate) fn on(backend: &'static Backend, stream: S) -> Wire<S> {
        let mut codec = PacketCodec::default();
        codec.max_allowed_packet = MAX_PACKET as usize;
        Wire {
            stream,
            inbox: BytesMut::new(),
            backend,
            codec,
        }
    }

    /// Bounds every wait on this wire by `at`, or lifts the bound.
    ///
    /// [`crate::pg`]'s `set_deadline` exactly, for `rule:core-classes/db-statement-members`'s statement
    /// deadline and with the same two properties: the bound is the socket's, so
    /// it covers a whole `COM_STMT_PREPARE`/`COM_STMT_EXECUTE` conversation
    /// rather than one syscall, and it is on the method rather than the type so
    /// a scripted stream that never waits needs no clock.
    pub(crate) fn set_deadline(&mut self, at: Option<std::time::Instant>)
    where
        S: nvs_host::net::Deadline,
    {
        self.stream.set_deadline(at);
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
    pub(crate) fn upgrade<T: Read + Write>(
        self,
        wrap: impl FnOnce(S) -> io::Result<T>,
    ) -> io::Result<Wire<T>> {
        Ok(Wire {
            stream: wrap(self.stream)?,
            inbox: self.inbox,
            backend: self.backend,
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
    /// One function because a half-written packet is the shape `rule:core-classes/db-connection-busy-state`
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
/// has not been checked. Only the fields the steps after it cannot proceed
/// without are kept.
#[derive(Debug)]
pub(crate) struct Greeting {
    /// The capability bits the server claims, already checked against what the
    /// transport asks of them — [`REQUIRED_CAPABILITIES`] where there is a
    /// network and `REQUIRED_OVER_A_SOCKET` where there is not. A socket arm
    /// takes `CLIENT_SSL` back out of this afterwards, because nothing is
    /// upgraded there and the handshake response offers back what it reads
    /// here.
    pub(crate) capabilities: CapabilityFlags,
    /// The extended capability bits the server claims, which are MariaDB's own
    /// and which a MySQL server leaves as the zero filler they sit in —
    /// [`agreed_extended`] is the one reader.
    extended: MariadbCapabilities,
    /// The challenge every plugin's response is derived from.
    nonce: Vec<u8>,
    /// The plugin the server wants, as it spelled it.
    plugin: Vec<u8>,
    /// The version, which decides `HandshakeResponse`'s collation width. Absent
    /// from a server whose banner does not parse, in which case the modern
    /// framing is assumed — every server this driver will speak to is past
    /// 5.5.3 by decades.
    server_version: (u16, u16, u16),
    /// That same field as the server wrote it, which is what
    /// `Core\Db\Connection::serverVersion` answers on MySQL and on MariaDB —
    /// [ADR 0187 § 2](/docs/decisions/0187.md). The numbers above are a
    /// decision this handshake makes; this is the string an operator reads,
    /// suffix and all, and neither is rebuilt from the other.
    ///
    /// Lossy where those bytes are not UTF-8: it is a label from a peer nothing
    /// has authenticated yet, and refusing a connection over the spelling of a
    /// banner is a refusal no operator could act on.
    pub(crate) banner: String,
}

/// Reads the server's greeting off a plaintext socket, requiring `required` of
/// it.
///
/// The set is the caller's because it is the *transport's*:
/// [`REQUIRED_CAPABILITIES`] over TCP, where the in-band upgrade is what stands
/// between this handshake and a network, and [`REQUIRED_OVER_A_SOCKET`] over a
/// Unix-domain socket, where there is no network for it to stand in.
///
/// # Errors
///
/// `ConnectionRefused` for a server that offers none of `required`,
/// `InvalidData` for a packet that is not a greeting, and the server's own
/// refusal where it declined the connection before greeting at all.
pub(crate) fn read_greeting<S: Read + Write>(
    wire: &mut Wire<S>,
    required: CapabilityFlags,
) -> io::Result<Greeting> {
    let packet = wire.read_packet()?;
    if packet.first() == Some(&0xFF) {
        return Err(server_refusal(
            wire.backend,
            &packet,
            CapabilityFlags::empty(),
        ));
    }

    let handshake = HandshakePacket::deserialize((), &mut ParseBuf(&packet))?;
    let capabilities = handshake.capabilities();
    let missing = required.difference(capabilities);
    if !missing.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::ConnectionRefused,
            format!(
                "the server offers no {missing:?}, and this driver needs \
                 {required:?}: `rule:core-classes/db-capabilities` has no spelling for a \
                 connection without plugin authentication, or without TLS where the \
                 transport is a network"
            ),
        ));
    }

    Ok(Greeting {
        capabilities,
        extended: handshake.mariadb_ext_capabilities(),
        nonce: handshake.nonce(),
        plugin: handshake
            .auth_plugin_name_ref()
            .unwrap_or_default()
            .to_vec(),
        server_version: handshake.server_version_parsed().unwrap_or((5, 5, 3)),
        banner: String::from_utf8_lossy(handshake.server_version_ref()).into_owned(),
    })
}

/// The extended capabilities this driver and this server both have, which is
/// what goes in the last four of either handshake packet's reserved bytes.
///
/// **A second capability word exists because the first one filled up.**
/// MariaDB's own bits start at 32 and `CapabilityFlags` is `u32`, so
/// [`CLIENT_CAPABILITIES`] cannot name one however it is written:
/// `MARIADB_CLIENT_STMT_BULK_OPERATIONS` is bit 34. MariaDB's answer was to
/// redefine the handshake's trailing filler — 23 bytes in MySQL's layout, 19
/// bytes plus a little-endian `u32` in MariaDB's — and `mysql_common` models
/// exactly that, so neither packet is composed by hand here: `SslRequest` and
/// `HandshakeResponse` each take the word and serialize it where the filler's
/// last four bytes were. The reply's own builder carrying it is the reason this
/// slice is a `.with_…` call rather than a patched buffer.
///
/// **The intersection is the whole gate, and it needs no test for which server
/// this is.** A MySQL server sends zeros in those bytes because to it they are
/// filler, so its greeting offers nothing and nothing is claimed back at it;
/// [`MYSQL`]'s own [`Backend::extended`] is empty for the same reason from the
/// other side, so the two independently agree on zero. What a driver claims is
/// therefore a fact about the driver, and what it gets is a fact about the pair.
///
/// It is sent **twice** — once in the `SSLRequest` and once in the handshake
/// response — because those packets share a header and the server parses the
/// word out of whichever it is reading. libmariadb writes it in both, and a
/// client that claimed a capability in the cleartext half and dropped it in the
/// encrypted one would be describing itself two ways to one server.
pub(crate) fn agreed_extended(backend: &Backend, greeting: &Greeting) -> MariadbCapabilities {
    backend.extended.intersection(greeting.extended)
}

/// Writes the `SSLRequest` that turns the rest of this socket into TLS records.
///
/// It is a capability word and nothing else — no user, no database, no
/// credential — because everything identifying is owed to the encrypted half.
///
/// # Errors
///
/// Whatever the stream reported.
pub(crate) fn request_tls<S: Read + Write>(
    wire: &mut Wire<S>,
    greeting: &Greeting,
) -> io::Result<()> {
    let mut payload = Vec::new();
    SslRequest::new(
        CLIENT_CAPABILITIES.intersection(greeting.capabilities),
        MAX_PACKET,
        COLLATION,
    )
    .with_mariadb_capabilities(agreed_extended(wire.backend, greeting))
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

/// Who to be on a connection, and which plugins the driver opening it will
/// prove that with.
///
/// [`authenticate`] takes this rather than a [`MySqlTarget`] because the
/// exchange is the protocol's and the *roster* is the driver's: MariaDB names
/// plugins MySQL has never heard of and refuses one MySQL defaults to, and
/// [`crate::maria`] passes its own gate here rather than owning a second copy
/// of the loop that runs it. Which plugins each driver answers is the security
/// question this crate takes most seriously, so it is one field with one
/// meaning rather than something inferred from the connection's type.
pub(crate) struct Login<'a> {
    /// The user to log in as.
    pub(crate) user: &'a str,
    /// The password, used only to derive a challenge response.
    pub(crate) password: &'a str,
    /// The schema named in the handshake response.
    pub(crate) database: &'a str,
    /// The plugin a server-named plugin selects, or the refusal that name
    /// earns — [`plugin_or_refuse`] for this driver, and MariaDB's own for the
    /// other.
    pub(crate) roster: fn(&[u8]) -> io::Result<AuthPlugin<'static>>,
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
/// on this connection is decoded against. The *extended* half is not returned
/// with it and does not need to be: [`agreed_extended`] is a function of the
/// backend and the greeting, both of which a caller that wants it still holds.
///
/// # Errors
///
/// `ConnectionRefused` for a plugin this driver does not answer, an `Other`
/// carrying a [`ServerError`] for the server's own refusal — a wrong password, a
/// schema that does not exist — with § 8's kind, its `SQLSTATE` and its message,
/// `InvalidData` for a packet the protocol does not allow at that point, and
/// whatever the stream reported.
pub(crate) fn authenticate<S: Read + Write>(
    wire: &mut Wire<S>,
    login: &Login<'_>,
    greeting: &Greeting,
) -> io::Result<CapabilityFlags> {
    let capabilities = CLIENT_CAPABILITIES.intersection(greeting.capabilities);
    let context = AuthContext {
        password: login.password.as_bytes(),
        nonce: &greeting.nonce,
    };

    let plugin = (login.roster)(&greeting.plugin)?;
    let mut exchange = AuthProc::init(&plugin)
        .map_err(|e| io::Error::new(io::ErrorKind::ConnectionRefused, e.to_string()))?;
    let first = exchange
        .run(context, &greeting.nonce)
        .map_err(auth_failed)?;

    let mut payload = Vec::new();
    HandshakeResponse::new(
        first.data().map(<[u8]>::to_vec),
        greeting.server_version,
        Some(login.user.as_bytes()),
        Some(login.database.as_bytes()),
        Some(plugin),
        capabilities,
        None,
        MAX_PACKET,
    )
    .with_mariadb_ext_capabilities(agreed_extended(wire.backend, greeting))
    .serialize(&mut payload);
    wire.send(&payload)?;

    loop {
        let packet = wire.read_packet()?;
        match packet.first() {
            // `OK`, and the only way out of this loop that is a connection.
            Some(0x00) => return Ok(capabilities),
            Some(0xFF) => return Err(server_refusal(wire.backend, &packet, capabilities)),
            // `AuthSwitchRequest`: the server names a different plugin, and the
            // gate applies again — this is the packet the module doc is about.
            Some(0xFE) => {
                let switch = AuthSwitchRequest::deserialize((), &mut ParseBuf(&packet))?;
                let named = (login.roster)(switch.auth_plugin().as_bytes())?;
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

/// The server's `ERR` packet, worded as this crate's callers read it and
/// carrying [ADR 0067 § 8](/docs/decisions/0067.md)'s normalised kind.
///
/// An `Other` holding a [`ServerError`], which is [`crate::pg`]'s `server_error`
/// and its reasons: a caller that prints the sentence has the server's own
/// words, and one that branches — § 7's retry rule among them — asks
/// [`ServerError::of`] rather than matching on the text.
///
/// **Deliberately not a `PermissionDenied`.** That kind says only "the server
/// worded this", to [`poison_on_write`], the one caller that has to know, and
/// it says it about a duplicate key as loudly as about a rejected credential —
/// a classification only § 8's table can make, and one `io::ErrorKind` has no
/// room to hold. It would also put every MySQL refusal outside the arm
/// `nvs_stdlib::db` throws `Db\DbError` from, which reads `Other`.
///
/// Both raw values ride along, because they are what an operator greps for and
/// § 8 keeps them available for the conditions normalising does not reach. MySQL
/// sends neither a severity nor a constraint name, so the first is the constant
/// the packet means and the second is `None`.
pub(crate) fn server_refusal(
    backend: &Backend,
    packet: &[u8],
    capabilities: CapabilityFlags,
) -> io::Error {
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
    let sql_state = error
        .sql_state_ref()
        .map(|s| String::from_utf8_lossy(&s.as_bytes()).into_owned())
        .unwrap_or_else(|| "HY000".to_owned());
    let code = error.error_code();
    io::Error::other(ServerError {
        kind: (backend.kind_of)(code, &sql_state),
        sql_state,
        severity: String::from("ERROR"),
        message: error.message_str().into_owned(),
        constraint: None,
        // Widened where it is held, not here: MySQL's own field is the `u16`
        // this packet carries and `kind_of` above is keyed on it.
        driver_code: Some(u32::from(code)),
        backend: backend.name,
    })
}

/// The § 8 kind a MySQL error code means.
///
/// **Keyed on the vendor integer where [`crate::pg`]'s table is keyed on the
/// `SQLSTATE`**, because the two servers specify opposite halves. MySQL's
/// `SQLSTATE` is a compatibility field it fills with `HY000` for every condition
/// it has no ODBC class for, so that one string covers a deadlock, a lock
/// timeout and a shutdown alike, while the integer names each of them exactly.
/// The `SQLSTATE` is still read, as the fallback for a code this table does not
/// name: the classes MySQL does fill mean what PostgreSQL's mean.
///
/// Everything unnamed is [`DbErrorKind::Other`] rather than a guess, as on the
/// other driver — § 8 normalises the conditions applications branch on, and a
/// code outside that set is one they read `driverCode` for. MariaDB's codes
/// diverge above 1900 and it is its own driver with its own table, so none of
/// them is here.
pub(crate) fn kind_of(code: u16, sql_state: &str) -> DbErrorKind {
    match code {
        // `ER_DUP_ENTRY` and the siblings that word the same condition for a
        // write, a unique index and a named key.
        1022 | 1062 | 1169 | 1586 => DbErrorKind::UniqueViolation,
        // The parent row is missing, or the child row still points at this one.
        1216 | 1217 | 1451 | 1452 => DbErrorKind::ForeignKeyViolation,
        1048 | 1263 => DbErrorKind::NotNullViolation,
        // `ER_CHECK_CONSTRAINT_VIOLATED`, which is MySQL 8.0.16 and later; a
        // server that does not enforce `CHECK` cannot raise it.
        3819 => DbErrorKind::CheckViolation,
        // `ER_LOCK_DEADLOCK`: InnoDB rolled this transaction back whole to break
        // the cycle, so the callable § 7 re-runs starts from nothing.
        1213 => DbErrorKind::Deadlock,
        // **`ER_LOCK_WAIT_TIMEOUT` is a `Timeout` and not a `Deadlock`**, which
        // is the row of this table worth arguing: it is the other code an
        // application reads while deciding to retry, and it is not one
        // [`DbErrorKind::is_retryable`] may name. A retry is sound only where
        // the server aborted the *transaction*, and InnoDB rolls back only the
        // statement on a lock wait timeout unless `innodb_rollback_on_timeout`
        // is set — the transaction is still open and still holding its locks, so
        // re-running the callable would run its earlier statements a second time
        // inside it. `ER_QUERY_INTERRUPTED` and `ER_QUERY_TIMEOUT` are a
        // `KILL QUERY` and `max_execution_time`, which is `57014` on the other
        // driver and the same thing from the statement's point of view.
        1205 | 1317 | 3024 => DbErrorKind::Timeout,
        // The server going away or refusing to take work: the connection
        // ceiling, a shutdown in progress, an aborted connection, a `KILL`.
        1040 | 1053 | 1152 | 1927 => DbErrorKind::ConnectionLost,
        // Access denied — to the server, to a schema, a table, a column, a
        // routine, or to the privilege the statement itself needs.
        1044 | 1045 | 1130 | 1142 | 1143 | 1227 | 1370 | 1698 => DbErrorKind::Permission,
        // A parse error and the "no such thing" codes, which § 8 makes one
        // kind: an undefined table and a malformed statement are the same bug to
        // a caller.
        1049 | 1051 | 1054 | 1064 | 1146 => DbErrorKind::Syntax,
        _ => match sql_state.get(..2) {
            // Class 08 — connection exception, every member of it.
            Some("08") => DbErrorKind::ConnectionLost,
            // Class 28 — invalid authorization specification.
            Some("28") => DbErrorKind::Permission,
            // Class 40 — the transaction the server itself rolled back. MySQL
            // spells `ER_LOCK_DEADLOCK` `40001` and that code is named above, so
            // an unnamed member is a condition of the same shape: the work is
            // undone and re-running it is sound.
            Some("40") => DbErrorKind::SerializationFailure,
            // Class 42 — syntax error or access rule violation, § 8's `Syntax`
            // being the half an application branches on.
            Some("42") => DbErrorKind::Syntax,
            _ => DbErrorKind::Other,
        },
    }
}

/// Reads the answer to a command that returns no rows — the handshake's
/// `SET time_zone`, and § 13's `COM_RESET_CONNECTION`.
///
/// [`read_answer`] is the reader and owns every packet shape, including
/// [ADR 0067 § 3](/docs/decisions/0067.md)'s `LOCAL INFILE` refusal.
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
        /// Rows the statement changed, as the server counted them — `rule:core-classes/db-statement-members`'s `affected`.
        affected: u64,
        /// The `AUTO_INCREMENT` value the statement generated, and `None` where
        /// it generated none. The status packet spells that absence `0` and
        /// `mysql_common` maps it, so the absence is an absence in the first
        /// place that holds it: the server reports no key as `0` and reports a
        /// generated key from `1` upward, and § 4's `lastId` is `?uint`.
        last_id: Option<u64>,
    },
    /// A result set of this many columns. That many column definition packets
    /// follow it, and [`read_columns`] is what takes them.
    Columns(u16),
}

/// Reads one command's first answer packet, refusing a `LOCAL INFILE` request.
///
/// The shapes a server may put here, and how they are told apart:
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
/// An `Other` carrying a [`ServerError`] for the server's own error,
/// `PermissionDenied` for a `LOCAL INFILE` request, `InvalidData` for anything
/// else.
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
                last_id: ok.last_insert_id(),
            })
        }
        Some(0xFF) => Err(server_refusal(wire.backend, &packet, capabilities)),
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
                     `rule:core-classes/db-capabilities` turns `LOCAL INFILE` off with no option to enable it: \
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
/// [ADR 0067 § 9](/docs/decisions/0067.md)'s split between `tainted
/// string` and `tainted bytes` is read off the charset or it is not read at
/// all. The number is protocol constant `binary`, fixed since 4.1.
const BINARY_CHARSET: u16 = 63;

/// [ADR 0067 § 9](/docs/decisions/0067.md)'s type map, as the Novis
/// type one MySQL column definition declares — what `Core\Db\Column::type`
/// answers for this column.
///
/// **It describes the column and never a value**, which is the whole reason it
/// reads no row: a column whose every row is `NULL` still has a type.
/// [`crate::PgColumn::column_type`] is the same question on the other driver,
/// and [`ColumnType`]'s own doc owns why describing a column and decoding one
/// of its values are two functions rather than one.
///
/// Some of § 9's rows are decided by something other than the type byte, and
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

/// Sends `rule:core-classes/db-column-types`'s declared zone as a session variable.
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
pub(crate) fn set_session_time_zone<S: Read + Write>(
    wire: &mut Wire<S>,
    capabilities: CapabilityFlags,
    seconds_east: i32,
) -> io::Result<()> {
    // `COM_QUERY`, and the only text statement this driver ever composes: its
    // one interpolated value is a number this process computed, so `rule:core-classes/db-one-api`'s no-emulated-prepares rule has nothing to bite on.
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
    /// Opens a connection: the transport [`Endpoint`] names, the greeting, § 3's
    /// in-band upgrade where there is a network to upgrade, the authentication
    /// exchange, and the declared zone.
    ///
    /// A socket endpoint is **opened as written**.
    /// `rule:core-classes/db-unix-socket-path` leaves MySQL's socket in the
    /// operator's hands because MySQL's socket has no naming convention to
    /// derive one from, which is the whole of the difference from
    /// [`crate::pg`]'s directory: nothing here rewrites the path.
    ///
    /// `deadline` bounds the whole of that and not one leg of it — the connect,
    /// the handshake and every authentication round trip share one clock, filed
    /// on the socket where `nvs-host` keeps it. `None` is unbounded and is for
    /// a caller that has its own bound. **The clock is lifted before the
    /// connection is handed back**, as [`crate::PgConn::connect`] lifts its own
    /// and for the reason that one gives.
    ///
    /// The connection comes back [`State::Idle`]: at a packet boundary, with a
    /// statement allowed.
    ///
    /// # Errors
    ///
    /// `ConnectionRefused` when the server will not upgrade to TLS, offers none
    /// of the capabilities its transport requires, or names an authentication
    /// plugin this driver does not answer; an `Other` carrying a
    /// [`ServerError`] for the server's own refusal;
    /// `InvalidData` for a packet the protocol does not allow at that point —
    /// or for a `tls_ca_file` that holds no certificate — `TimedOut` when the
    /// deadline passes, and whatever the socket or the TLS handshake itself
    /// reported.
    pub fn connect(
        endpoint: impl Into<Endpoint>,
        target: &MySqlTarget<'_>,
        deadline: Option<Instant>,
    ) -> io::Result<MySqlConn> {
        let (mut wire, greeting) = match endpoint.into() {
            Endpoint::Tcp(address) => {
                let mut tcp = match deadline {
                    Some(at) => NvsTcp::connect_timeout(
                        address,
                        at.saturating_duration_since(Instant::now()),
                    )?,
                    None => NvsTcp::connect(address)?,
                };
                tcp.set_deadline(deadline);

                let mut plain = Wire::new(tcp);
                let greeting = read_greeting(&mut plain, REQUIRED_CAPABILITIES)?;
                request_tls(&mut plain, &greeting)?;

                // Whatever is still buffered arrived in the clear and is carried
                // across with the codec: the greeting is the only thing a server
                // may say before the upgrade, and `read_greeting` took it.
                let wire = plain.upgrade(|tcp| match target.tls_ca_file {
                    Some(bundle) => {
                        NvsTls::over_bundle(tcp, target.host, bundle).map(MyStream::Tls)
                    }
                    None => NvsTls::over(tcp, target.host).map(MyStream::Tls),
                })?;
                (wire, greeting)
            }
            #[cfg(unix)]
            Endpoint::Socket(path) => {
                let mut local = match deadline {
                    Some(at) => NvsUnix::connect_timeout(
                        path,
                        at.saturating_duration_since(Instant::now()),
                    )?,
                    None => NvsUnix::connect(path)?,
                };
                local.set_deadline(deadline);

                let mut plain = Wire::new(local);
                let mut greeting = read_greeting(&mut plain, REQUIRED_OVER_A_SOCKET)?;
                // Nothing is upgraded on this arm, so the handshake response
                // must not claim `CLIENT_SSL`: `authenticate` offers back
                // whatever the greeting said the server has, and a client that
                // claims TLS and then does not send it is a wire the server
                // stops reading.
                greeting.capabilities.remove(CapabilityFlags::CLIENT_SSL);
                let wire = plain.upgrade(|local| Ok(MyStream::Local(local)))?;
                (wire, greeting)
            }
        };

        let login = Login {
            user: target.user,
            password: target.password,
            database: target.database,
            roster: plugin_or_refuse,
        };
        let capabilities = authenticate(&mut wire, &login, &greeting)?;
        set_session_time_zone(&mut wire, capabilities, target.time_zone)?;
        // Lifted for [`crate::PgConn::connect`]'s reason, which is the same on every
        // driver that files the handshake clock on its own socket.
        wire.set_deadline(None);

        Ok(MySqlConn {
            wire,
            state: Cell::new(State::Idle),
            capabilities,
            // What the greeting already carried — see the field.
            server_version: greeting.banner,
            // `rule:core-classes/db-one-api`'s size, already read off the `[db.<name>]` block by
            // `statement_cache_for` and carried here on the target — this path
            // takes a number and has no opinion about where an unwritten
            // field's default comes from.
            cache: StatementCache::new(target.statement_cache),
            // § 9's zone-less row is decoded a layer up, where the target is
            // gone — see the field.
            time_zone: target.time_zone,
            // § 7's nesting, which a fresh connection is outside of.
            depth: Cell::new(0),
            // Nothing is parked until `stream` parks it — see the field.
            reading: None,
        })
    }

    /// Bounds every wait on this connection by `at`, or lifts the bound.
    ///
    /// [`crate::PgConn::set_deadline`]'s twin, and the same clock
    /// [`crate::Connection::set_deadline`] files through the enum.
    pub fn set_deadline(&mut self, at: Option<Instant>) {
        self.wire.set_deadline(at);
    }

    /// The zone a zone-less `DATETIME` or `TIMESTAMP` off this connection is
    /// read in, as seconds east of UTC — the same number the handshake sent the
    /// server.
    #[must_use]
    pub fn time_zone(&self) -> i32 {
        self.time_zone
    }

    /// `rule:core-classes/db-one-api`'s round trips for one statement — two the first time this
    /// connection runs it, one every time after — and the columns its result
    /// set turned out to have.
    ///
    /// The two-line delegation the playbook prescribes: [`start_statement`] is
    /// where the sequencing lives, because a method on `MySqlConn` can only be
    /// reached through a real socket and a real certificate and so cannot be
    /// unit-tested at all.
    ///
    /// The result borrows the connection until it ends, which is
    /// [`MySqlRows`]' whole point: `rule:core-classes/db-statement-members`'s one-statement-at-a-time rule
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
    ) -> io::Result<MySqlRows<'_, MyStream>> {
        start_statement(
            &mut self.wire,
            &self.state,
            self.capabilities,
            &mut self.cache,
            sql,
            params,
        )
    }

    /// Runs one statement and leaves its result set open, **borrowing nothing**:
    /// the read state is parked on this connection and the rows come off it one
    /// [`Self::stream_next_row`] at a time.
    ///
    /// The wire half of `rule:core-classes/db-streaming`'s `stream`, costing the round trips
    /// [`Self::query`] costs and stopping where it stops — [`MySqlCursor`] owns
    /// why the state has to be here rather than inside a borrow. The answer is
    /// what the result set described, empty for a statement that returned none,
    /// and [`Self::stream_columns`] hands the same slice back to the later calls
    /// that decode against it.
    ///
    /// A second statement is refused while this one is open, which is
    /// `rule:core-classes/db-streaming`'s `LogicError` read off [`State::Streaming`] rather than
    /// off a lifetime. [`Self::end_stream`] is the abandonment [`MySqlRows`] gets
    /// from `Drop`; a walk read to its end needs no call at all.
    ///
    /// # Errors
    ///
    /// As [`Self::query`].
    pub fn stream(&mut self, sql: &str, params: &[Option<&[u8]>]) -> io::Result<&[Column]> {
        let reading = open_result(
            &mut self.wire,
            &self.state,
            self.capabilities,
            &mut self.cache,
            sql,
            params,
        )?;
        Ok(self.reading.insert(reading).columns())
    }

    /// What the parked walk's result set described, or `None` for a connection
    /// that has not streamed since its last reset.
    ///
    /// It outlives the rows on purpose: a binary row is decoded against the
    /// definitions it belongs to, and that decode happens after the step that
    /// produced it.
    #[must_use]
    pub fn stream_columns(&self) -> Option<&[Column]> {
        Some(self.reading.as_ref()?.columns())
    }

    /// The next row of the parked walk, or `None` once it has ended — and `None`
    /// too for a connection with no walk parked on it at all.
    ///
    /// Ending it returns the connection to [`State::Idle`], exactly as
    /// [`MySqlRows::next_row`] does. The state itself stays parked, holding what
    /// the statement finished with, until the next [`Self::stream`] replaces it
    /// or [`Self::end_stream`] drops it.
    ///
    /// # Errors
    ///
    /// As [`MySqlRows::next_row`].
    pub fn stream_next_row(&mut self) -> io::Result<Option<MySqlRow>> {
        let Some(reading) = self.reading.as_mut() else {
            return Ok(None);
        };
        next_row_of(&mut self.wire, &self.state, reading)
    }

    /// `rule:observability/a-query-is-a-trace-event`'s trace event for the parked walk, or `None`
    /// where there is none — [`MySqlRows::span`] for what a caller reading one
    /// mid-walk gets.
    #[must_use]
    pub fn stream_span(&self) -> Option<&QuerySpan> {
        Some(self.reading.as_ref()?.span())
    }

    /// Names the `[db.<name>]` block the parked walk is running on, and does
    /// nothing where there is no walk — [`MySqlRows::name_connection`] owns why
    /// the driver cannot work the name out for itself.
    pub fn name_stream_connection(&mut self, connection: &str) {
        if let Some(reading) = self.reading.as_mut() {
            reading.name_connection(connection);
        }
    }

    /// Abandons the parked walk: reads what is left of the result set and
    /// forgets it.
    ///
    /// This is [`MySqlRows`]' `Drop` written as a call, and for the same reason —
    /// the rows are on their way whether or not anybody reads them, so draining
    /// to the terminator is what keeps the connection poolable instead of
    /// poisoned. A read that fails on the way poisons it, through the same helper
    /// every other read here uses.
    pub fn end_stream(&mut self) {
        end_stream_of(&mut self.wire, &self.state, &mut self.reading);
    }

    /// [ADR 0067 § 4](/docs/decisions/0067.md)'s `executeMany`: one
    /// prepare, N executions, and the affected counts summed.
    ///
    /// The two-line delegation [`MySqlConn::query`] gives its reason for.
    ///
    /// # Errors
    ///
    /// As [`execute_many`].
    pub fn execute_many(&mut self, sql: &str, sets: &[&[Option<&[u8]>]]) -> io::Result<u64> {
        execute_many(
            &mut self.wire,
            &self.state,
            self.capabilities,
            &mut self.cache,
            sql,
            sets,
        )
    }

    /// [ADR 0067 § 7](/docs/decisions/0067.md)'s `START TRANSACTION`,
    /// or the `SAVEPOINT` a nested `transaction()` is.
    ///
    /// The driver half of § 7 and nothing more — the callable, the
    /// rollback-only flag and the retry rule are `nvs-stdlib`'s, exactly as on
    /// [`crate::PgConn::begin`]. [`begin`] owns which command a nesting depth
    /// gets and how the two options are rendered, including the second round
    /// trip an isolation level costs on this protocol and not on PostgreSQL's.
    ///
    /// # Errors
    ///
    /// As [`begin`].
    pub fn begin(
        &mut self,
        isolation: Option<Isolation>,
        read_only: bool,
    ) -> io::Result<QuerySpan> {
        begin(
            &mut self.wire,
            &self.state,
            self.capabilities,
            &self.depth,
            isolation,
            read_only,
        )
    }

    /// How many transaction levels are open on this connection — 0 outside one,
    /// 1 inside an outermost `transaction()`, deeper inside a nested one.
    ///
    /// [`crate::PgConn::depth`] owns why this is public at all: § 7 retries a
    /// serialization failure only for an outermost transaction, and the caller
    /// cannot tell the two apart on its own.
    #[must_use]
    pub fn depth(&self) -> u32 {
        self.depth.get()
    }

    /// § 7's `COMMIT`, or the `RELEASE SAVEPOINT` closing a nested one — a
    /// normal return out of the callable either way.
    ///
    /// # Errors
    ///
    /// As [`commit`].
    pub fn commit(&mut self) -> io::Result<QuerySpan> {
        commit(&mut self.wire, &self.state, self.capabilities, &self.depth)
    }

    /// § 7's `ROLLBACK`, or the `ROLLBACK TO SAVEPOINT` undoing a nested one —
    /// a throw out of the callable, or `rollBack`'s own signal.
    ///
    /// # Errors
    ///
    /// As [`roll_back`].
    pub fn roll_back(&mut self) -> io::Result<QuerySpan> {
        roll_back(&mut self.wire, &self.state, self.capabilities, &self.depth)
    }

    /// [ADR 0067 § 13](/docs/decisions/0067.md)'s reset, before this
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
        // A walk the program abandoned is read to its end before the reset goes
        // out, and then dropped. `COM_RESET_CONNECTION` is a command like any
        // other: written over rows that are still arriving it is a second run of
        // packets numbered from the same 0 as the first, and nothing framing
        // them tells the reset's `OK` from the row that was already coming.
        end_stream_of(&mut self.wire, &self.state, &mut self.reading);
        reset_session(
            &mut self.wire,
            self.capabilities,
            self.time_zone,
            &mut self.cache,
        )?;
        // `COM_RESET_CONNECTION` rolls back whatever transaction was open, so
        // every level this count named is gone with it — and a connection
        // pooled at a depth it no longer has would open the next request's
        // outermost `transaction()` as a `SAVEPOINT` against nothing.
        self.depth.set(0);
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
/// variables, so `rule:core-classes/db-column-types`'s declared zone has to be sent again or the next
/// request decodes every zone-less column in UTC while believing otherwise.
/// The prepared-statement cache goes with it too — § 13 names that as the real
/// asymmetry with PostgreSQL, and it is the protocol's rather than a choice.
///
/// Free and generic in the stream for this crate's usual reason: a
/// `Wire<MyStream>` needs a socket and a certificate that no unit test has.
///
/// # Errors
///
/// As [`read_ok`], for either of the two commands.
pub(crate) fn reset_session<S: Read + Write>(
    wire: &mut Wire<S>,
    capabilities: CapabilityFlags,
    seconds_east: i32,
    cache: &mut StatementCache<Prepared>,
) -> io::Result<()> {
    wire.codec.reset_seq_id();
    wire.send(&[COM_RESET_CONNECTION])?;
    read_ok(wire, capabilities)?;
    // Emptied here rather than by the caller so the invalidation cannot be
    // forgotten by whoever pools the connection: the statements are gone from
    // the server the moment that `OK` arrives, and a cache still naming them
    // would bind the next request against handles this session no longer has. A
    // reset that failed destroys the connection (§ 13), so what the cache holds
    // on that path is nobody's business.
    cache.clear();
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

/// `rule:core-classes/db-one-api`'s first round trip: `COM_STMT_PREPARE`.
///
/// **This is the round trip § 1 says is recorded rather than hidden.** A
/// statement's first execution on a connection costs two — this one and
/// [`execute`] — where PostgreSQL's extended protocol pays nothing extra. The
/// honest answer to that asymmetry is [`cached_statement`], which is why this
/// function runs once per statement per connection rather than once per
/// execution; an emulated prepare that spliced the value into the SQL to save
/// the packet is the thing § 1 removes and was never the other option.
///
/// The prepare's own column and parameter definition packets are read and
/// dropped. They have to be read — they are packets in the stream and the next
/// command cannot start until the wire is at a boundary — and nothing wants
/// them: what a caller decodes rows against is the *execution's* definitions,
/// which arrive again and are the ones that are true.
///
/// Free and generic in the stream for this crate's usual reason, which
/// `crate::pg`'s `start_statement` states: a `Wire<MyStream>` needs a socket
/// and a certificate that no unit test has.
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
        return Err(server_refusal(wire.backend, &packet, capabilities));
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

/// `rule:core-classes/db-one-api`'s second round trip: `COM_STMT_EXECUTE`, over the binary
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
                "the statement takes {} parameter(s) and {} were bound — `rule:core-classes/db-parameters`'s \
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
        // does not write it: a parameter that large is a value `rule:core-classes/db-statement-members`'s
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

/// § 1's cache in front of [`prepare`]: the statement this connection is
/// already holding for `sql`, or a fresh one recorded under it.
///
/// **This is where the first of § 1's two round trips is skipped.** The key is
/// the SQL text plus the parameter count — [`crate::StatementCache`]'s own key,
/// so `IN` over three ids and over four stay two statements — and the handle is
/// what MySQL *hands back* rather than a name this side minted. That is the
/// whole reason this driver drives the cache through `lookup`/`make_room`/
/// `commit` instead of answering a batch the way `crate::pg`'s `start_statement`
/// does: the id arrives with the prepare, so there is nothing to write down
/// until it has.
///
/// **An eviction is closed before the prepare that needed the room**, so the
/// server never holds more statements than `statement_cache` allows, not even
/// for the length of one round trip. The commit is after the prepare landed: a
/// prepare that failed leaves a cache naming only statements the server has.
///
/// # Errors
///
/// As [`prepare`], plus whatever the eviction's write reported.
fn cached_statement<S: Read + Write>(
    wire: &mut Wire<S>,
    capabilities: CapabilityFlags,
    cache: &mut StatementCache<Prepared>,
    sql: &str,
    arity: usize,
) -> io::Result<Prepared> {
    if let Some(stmt) = cache.lookup(sql, arity) {
        return Ok(stmt);
    }
    if let Some(evicted) = cache.make_room() {
        close_statement(wire, evicted)?;
    }
    let stmt = prepare(wire, capabilities, sql)?;
    cache.commit(sql, arity, stmt);
    Ok(stmt)
}

/// `COM_STMT_CLOSE` for a statement the cache evicted.
///
/// Nothing is read afterwards, because nothing is sent: a read here would block
/// until the *next* command's reply arrived and then take it as this one's. A
/// write that fails is reported and the caller poisons the connection, which is
/// every half-written packet's treatment — there is no boundary to be found
/// after one.
///
/// # Errors
///
/// Whatever the write reported.
fn close_statement<S: Read + Write>(wire: &mut Wire<S>, stmt: Prepared) -> io::Result<()> {
    let mut payload = vec![COM_STMT_CLOSE];
    payload.extend_from_slice(&stmt.statement_id.to_le_bytes());
    // Every command starts a new packet sequence, and a stale counter is a wire
    // the codec cannot find a boundary in.
    wire.codec.reset_seq_id();
    wire.send(&payload)
}

/// One statement, end to end: the cache or a prepare, execute, and stop at the
/// first row — answering the read state [`MySqlCursor`] is, which the borrowed
/// path and the parked one carry the same copy of.
///
/// The shape is [`crate::pg`]'s `start_statement` and for its reasons — free
/// and generic in the stream so a unit test can script a server for it, and
/// taking the busy state by reference so `rule:core-classes/db-statement-members`'s one-statement-at-a-time
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
pub(crate) fn open_result<S: Read + Write>(
    wire: &mut Wire<S>,
    state: &Cell<State>,
    capabilities: CapabilityFlags,
    cache: &mut StatementCache<Prepared>,
    sql: &str,
    params: &[Option<&[u8]>],
) -> io::Result<MySqlCursor> {
    if !state.get().may_start_statement() {
        return Err(crate::pg::second_statement(state));
    }

    // `rule:observability/a-query-is-a-trace-event`'s span, opened before the prepare rather than around the
    // execute alone: § 1 says a statement's first run on this connection costs
    // two round trips, and what the caller waited is both of them. It is handed
    // `sql` and never `params`, which is the whole of § 11's "never parameters"
    // — `crate::span`'s module doc owns why that is a signature and not a rule.
    let mut span = QuerySpan::opened(Driver::MySql, sql);

    state.set(State::Executing);
    let stmt = match cached_statement(wire, capabilities, cache, sql, params.len()) {
        Ok(stmt) => stmt,
        Err(e) => return Err(poison_on_write(state, e)),
    };
    let answer = match execute(wire, capabilities, stmt, params) {
        Ok(answer) => answer,
        Err(e) => return Err(poison_on_write(state, e)),
    };

    match answer {
        Answer::Done { affected, last_id } => {
            // A statement with no result set has already ended, so its span
            // ends here with the count the status packet carried — there is no
            // stream left to reach [`MySqlRows::next_row`]'s terminator.
            span.finished(Some(affected));
            state.set(State::Idle);
            Ok(MySqlCursor {
                capabilities,
                columns: Arc::from(Vec::new()),
                rows: 0,
                affected,
                last_id,
                ended: true,
                span,
            })
        }
        Answer::Columns(count) => match read_columns(wire, count) {
            Ok(columns) => {
                state.set(State::Streaming);
                Ok(MySqlCursor {
                    capabilities,
                    columns: Arc::from(columns),
                    rows: 0,
                    affected: 0,
                    last_id: None,
                    ended: false,
                    span,
                })
            }
            Err(e) => Err(poison_on_write(state, e)),
        },
    }
}

/// [`open_result`] with the read state lent out beside a borrow of the
/// connection: the shape every buffered member of `rule:core-classes/db-statement-members` wants, and
/// the one `Core\Db\Connection::stream` is the single caller that cannot use.
///
/// # Errors
///
/// As [`open_result`].
pub(crate) fn start_statement<'a, S: Read + Write>(
    wire: &'a mut Wire<S>,
    state: &'a Cell<State>,
    capabilities: CapabilityFlags,
    cache: &mut StatementCache<Prepared>,
    sql: &str,
    params: &[Option<&[u8]>],
) -> io::Result<MySqlRows<'a, S>> {
    let reading = open_result(wire, state, capabilities, cache, sql, params)?;
    Ok(MySqlRows {
        wire,
        state,
        reading,
    })
}

/// `rule:core-classes/db-statement-members`'s `executeMany`: one prepare, N `COM_STMT_EXECUTE`s, and the
/// affected counts summed.
///
/// **N round trips where [`crate::pg`]'s batch costs one, and the protocol is
/// what decides that.** PostgreSQL's extended query puts every `Bind`/`Execute`
/// pair in one flush because the replies are self-describing messages on a
/// stream with no per-command numbering. A MySQL command restarts the packet
/// sequence id ([`Wire::codec`]'s `reset_seq_id`), so two commands in flight are
/// two packet runs numbered from the same 0 and nothing framing them tells the
/// second reply from the first. What § 4 buys here is therefore § 1's prepare —
/// paid once for the whole batch, which is what the cache gives a loop of
/// [`start_statement`] anyway — and one member's worth of round trips is the
/// honest price rather than a hidden one: `rule:core-classes/db-one-api` records the cost of this
/// protocol rather than hiding it, and this is the same account.
///
/// **The batch is not a transaction, and a refusal does not end it**, which is
/// [`crate::pg`]'s `execute_many` semantics reached a different way: there each
/// execution carries its own `Sync` and the server resumes at the next one, here
/// each execution is its own command and the one after a refusal still goes out.
/// So the two drivers agree on what a caller observes — the writes before a
/// failure stand, the ones after are still attempted, and the **first** error is
/// what the batch reports. A caller who wants all-or-nothing writes
/// `transaction(fn ($tx) => $tx->executeMany(…))`, which is § 4's answer on
/// either driver.
///
/// **This is the loop on MariaDB too, and `COM_STMT_BULK_EXECUTE` is the thing
/// it is deliberately not.** That command carries every set at once and would
/// turn this member's N round trips into one, which is the only reason to want
/// it — and § 4 refuses the trade because the ways it diverges are mostly in
/// what a caller observes rather than in what the wire costs. A bulk command
/// **ends** at a refusal where the paragraph above has the batch carry on, and
/// no driver can hide the difference: PostgreSQL's flush is already gone by the
/// time it reads the error, so the loop cannot be made to stop, and MariaDB's
/// command cannot be made to continue. And a set that answers with rows — a
/// `CALL` — is not something the command takes, while nothing here can route
/// one to this loop in advance, a prepare reporting `0` columns for any
/// statement whose result set depends on the data ([`Prepared`] says so). ADR
/// 0067 § 4 owns that decision and names what would reopen it.
///
/// A wire failure is the one thing that does end it: a poisoned connection is
/// one nothing can find a packet boundary in, so the remaining sets are not
/// written and that error is the answer even where a server refusal came first.
///
/// A set that answered with a result set — MySQL has no `RETURNING`, but a
/// `CALL` does it — is drained and contributes the rows it produced, which is
/// the number [`MySqlRows::affected`] reports for one and the number
/// PostgreSQL's `SELECT n` tag contributes to the other driver's sum.
///
/// An empty `sets` is § 4's no-op answering `0`, with the busy check still ahead
/// of it for [`crate::pg`]'s reason: § 4's refusal is a property of the
/// connection and not of the payload.
///
/// # Errors
///
/// `InvalidInput` for a statement written to a connection that is not idle, and
/// for a `sets` whose members do not all bind the same number of parameters —
/// one prepare has one parameter count, and it is what § 1's cache is keyed on
/// beside the SQL. Otherwise the first error any execution drew, or the wire
/// failure that stopped the batch.
pub(crate) fn execute_many<S: Read + Write>(
    wire: &mut Wire<S>,
    state: &Cell<State>,
    capabilities: CapabilityFlags,
    cache: &mut StatementCache<Prepared>,
    sql: &str,
    sets: &[&[Option<&[u8]>]],
) -> io::Result<u64> {
    if !state.get().may_start_statement() {
        return Err(crate::pg::second_statement(state));
    }

    let Some(first) = sets.first() else {
        return Ok(0);
    };
    let arity = first.len();
    if let Some(odd) = sets.iter().find(|set| set.len() != arity) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "one executeMany bound {arity} parameters in its first set and {} in another, and \
                 `rule:core-classes/db-statement-members`'s one prepare has one parameter count",
                odd.len()
            ),
        ));
    }

    let mut affected = 0_u64;
    let mut refused: Option<io::Error> = None;
    for set in sets {
        match execute_one(wire, state, capabilities, cache, sql, set) {
            Ok(count) => affected += count,
            Err(e) => {
                if state.get() == State::Poisoned {
                    return Err(e);
                }
                refused.get_or_insert(e);
            }
        }
    }

    match refused {
        Some(e) => Err(e),
        None => Ok(affected),
    }
}

/// One of [`execute_many`]'s sets, drained, and what it changed.
///
/// Split out so the batch's own accounting is a `match` on one result rather
/// than a stream held across the next iteration: a [`MySqlRows`] borrows the
/// wire, and the count has to be read before it is dropped.
///
/// # Errors
///
/// As [`start_statement`] and [`MySqlRows::next_row`].
fn execute_one<S: Read + Write>(
    wire: &mut Wire<S>,
    state: &Cell<State>,
    capabilities: CapabilityFlags,
    cache: &mut StatementCache<Prepared>,
    sql: &str,
    set: &[Option<&[u8]>],
) -> io::Result<u64> {
    let mut rows = start_statement(wire, state, capabilities, cache, sql, set)?;
    while rows.next_row()?.is_some() {}
    Ok(rows.affected().unwrap_or(0))
}

/// [ADR 0067 § 7](/docs/decisions/0067.md)'s `START TRANSACTION`, or
/// the `SAVEPOINT` a nested `transaction()` opens.
///
/// **The depth decides which**, as in [`crate::pg`]'s `begin`, and § 7's
/// composition is the reason: a library that wraps its own writes cannot ask
/// whether it is already inside a caller's transaction, so which command a
/// level gets is the connection's answer rather than the caller's.
///
/// **An isolation level costs a second round trip here and none on
/// PostgreSQL.** MySQL has no `START TRANSACTION ISOLATION LEVEL`: the
/// characteristic is set by a `SET TRANSACTION` that applies to the *next*
/// transaction started on this session, so a level asked for is a command of
/// its own ahead of the one that opens the transaction. Both are inside one
/// § 11 span, because the caller waited for both and timing only the second
/// would price a serializable transaction as a plain one.
///
/// **A `START TRANSACTION` the server refuses after it accepted the
/// `SET TRANSACTION` poisons the connection**, which is this module's one use
/// of that state for a wire that is still at a boundary. The armed
/// characteristic belongs to the *session* and no command disarms it, so a
/// connection returned to § 13's pool would run some later request's
/// unqualified `transaction()` at a level nobody there asked for — and where
/// the armed level is `READ UNCOMMITTED`, that request gets weaker isolation
/// than it asked for, and nothing says so.
/// [`State::is_poolable`] is the only lever that says "close this rather than
/// reuse it", and one handshake on a path a healthy server never takes is
/// cheap for it.
///
/// `read_only` renders `READ ONLY` or nothing, never `READ WRITE`, for the
/// reason [`crate::pg`]'s `begin_command` gives in full: the option's absence
/// means the server's own default, and widening that from inside a program is
/// the wrong direction for a security rule.
///
/// # Errors
///
/// `InvalidInput` for a nested call carrying either option, otherwise as
/// [`simple_command`]. The depth moves only after a command the server
/// accepted, so a refused begin leaves the connection at the level it had.
pub(crate) fn begin<S: Read + Write>(
    wire: &mut Wire<S>,
    state: &Cell<State>,
    capabilities: CapabilityFlags,
    depth: &Cell<u32>,
    isolation: Option<Isolation>,
    read_only: bool,
) -> io::Result<QuerySpan> {
    let open = depth.get();
    if open > 0 {
        if isolation.is_some() || read_only {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "a transaction nested {open} deep asked for its own isolation level or \
                     read-only mode, and MySQL settles both for the whole transaction: ask for \
                     them on the outermost `transaction()`, or give this one a `{{shared: false}}` \
                     connection of its own"
                ),
            ));
        }
        let command = format!("SAVEPOINT {}", crate::pg::savepoint_name(open));
        let span = simple_command(wire, state, capabilities, &command)?;
        depth.set(open + 1);
        return Ok(span);
    }

    let start = if read_only {
        "START TRANSACTION READ ONLY"
    } else {
        "START TRANSACTION"
    };
    let Some(level) = isolation else {
        let span = simple_command(wire, state, capabilities, start)?;
        depth.set(1);
        return Ok(span);
    };

    let set = isolation_command(level);
    let mut span = QuerySpan::opened(Driver::MySql, &format!("{set}; {start}"));
    text_command(wire, state, capabilities, set)?;
    if let Err(refused) = text_command(wire, state, capabilities, start) {
        state.set(State::Poisoned);
        return Err(refused);
    }
    span.finished(None);
    depth.set(1);
    Ok(span)
}

/// The `SET TRANSACTION` one of § 7's isolation levels renders to.
///
/// **Only [`Isolation::Snapshot`] collapses**, onto `REPEATABLE READ`: InnoDB
/// reads that level from one snapshot established at the transaction's first
/// read, which is the guarantee `Snapshot` names — so asking for either gets
/// the same thing under the name this server uses, and neither is the missing
/// level § 7 says to throw over. Every other level is MySQL's own spelling.
///
/// A `&'static str` per level rather than [`crate::pg`]'s composed `String`,
/// because nothing composes here: the command that opens a MySQL transaction
/// takes no isolation option, so a level is a whole command.
fn isolation_command(level: Isolation) -> &'static str {
    match level {
        Isolation::ReadUncommitted => "SET TRANSACTION ISOLATION LEVEL READ UNCOMMITTED",
        Isolation::ReadCommitted => "SET TRANSACTION ISOLATION LEVEL READ COMMITTED",
        Isolation::RepeatableRead | Isolation::Snapshot => {
            "SET TRANSACTION ISOLATION LEVEL REPEATABLE READ"
        }
        Isolation::Serializable => "SET TRANSACTION ISOLATION LEVEL SERIALIZABLE",
    }
}

/// § 7's `COMMIT`, or the `RELEASE SAVEPOINT` that closes a nested level.
///
/// # Errors
///
/// `InvalidInput` for a connection in no transaction, otherwise as
/// [`simple_command`]. **The count follows the connection where the server took
/// it**: an outermost `COMMIT` the server refused has already rolled the
/// transaction back, so that level really is gone and the depth goes to 0 — a
/// caller that opens the next transaction on the connection, which § 7's
/// `{retries: n}` is, must get a `START TRANSACTION` and not a `SAVEPOINT`
/// against nothing. Which refusals those are is asked of the error itself —
/// [`ServerError::of`], the same question [`poison_on_write`] asks — and not
/// read back off the [`State`] that answer left. The state is a *summary* of it,
/// two calls apart and writable by anything else holding the connection, so
/// reading it here would make a claim about the transaction out of a fact about
/// the wire. § 4's busy check never reached the server and a wire failure was
/// never worded by one, so neither carries a [`ServerError`] and neither moves
/// the depth. A refused `RELEASE SAVEPOINT` says nothing of the kind and moves
/// nothing either, as in [`roll_back`].
pub(crate) fn commit<S: Read + Write>(
    wire: &mut Wire<S>,
    state: &Cell<State>,
    capabilities: CapabilityFlags,
    depth: &Cell<u32>,
) -> io::Result<QuerySpan> {
    let open = crate::pg::open_transaction(depth, "commit")?;
    let command: Cow<'_, str> = if open == 1 {
        Cow::Borrowed("COMMIT")
    } else {
        Cow::Owned(format!(
            "RELEASE SAVEPOINT {}",
            crate::pg::savepoint_name(open - 1)
        ))
    };

    let span = match simple_command(wire, state, capabilities, &command) {
        Ok(span) => span,
        Err(refused) => {
            if open == 1 && ServerError::of(&refused).is_some() {
                depth.set(0);
            }
            return Err(refused);
        }
    };
    depth.set(open - 1);
    Ok(span)
}

/// § 7's `ROLLBACK`, or the `ROLLBACK TO SAVEPOINT` that undoes a nested level.
///
/// **One command where [`crate::pg`]'s nested rollback is two, and MySQL's own
/// savepoint rule is what decides that.** PostgreSQL releases the savepoint it
/// returned to, because a second `SAVEPOINT` of the same name leaves the first
/// one behind and a loop that opens and abandons a nested transaction per
/// iteration would leave the server holding one per iteration. MySQL deletes
/// the savepoint a same-named `SAVEPOINT` finds, so re-opening a level reuses
/// `nvs_1` instead of adding to it and the server holds at most one name per
/// open level. The `RELEASE` would therefore buy nothing and cost a round trip
/// of its own: it cannot ride along with the rollback, because this driver
/// never asks for `CLIENT_MULTI_STATEMENTS` ([`REQUIRED_CAPABILITIES`]) and a
/// connection that accepted two statements in one command is one an injection
/// reaches further into.
///
/// # Errors
///
/// `InvalidInput` for a connection in no transaction, otherwise as
/// [`simple_command`]. A refused rollback leaves the depth where it was: the
/// level is still open as far as the server is concerned, and the level above
/// it rolls back over this one anyway.
pub(crate) fn roll_back<S: Read + Write>(
    wire: &mut Wire<S>,
    state: &Cell<State>,
    capabilities: CapabilityFlags,
    depth: &Cell<u32>,
) -> io::Result<QuerySpan> {
    let open = crate::pg::open_transaction(depth, "roll back")?;
    let command: Cow<'_, str> = if open == 1 {
        Cow::Borrowed("ROLLBACK")
    } else {
        Cow::Owned(format!(
            "ROLLBACK TO SAVEPOINT {}",
            crate::pg::savepoint_name(open - 1)
        ))
    };

    let span = simple_command(wire, state, capabilities, &command)?;
    depth.set(open - 1);
    Ok(span)
}

/// One of § 7's commands, sent as text, as [ADR 0067 § 11](/docs/decisions/0067.md)'s
/// span.
///
/// **`COM_QUERY` rather than § 1's prepared statements**, which is where the
/// two halves of § 1 stop pulling together: a prepare would cost the extra
/// round trip § 1 charges for a first execution and then hold a slot in a cache
/// sized for the request's real statements, to run a command of a few words
/// that binds nothing. It is the second text this driver composes, after
/// [`set_session_time_zone`], and § 1's no-emulated-prepares rule has nothing to
/// bite on either time: no caller's SQL and no bound value reaches this path,
/// and the only thing interpolated is a savepoint name this module minted.
///
/// The span carries no count. § 7's commands change no rows themselves, and
/// `affected` says "this statement reported a count" rather than "it reported
/// zero".
///
/// # Errors
///
/// As [`text_command`].
fn simple_command<S: Read + Write>(
    wire: &mut Wire<S>,
    state: &Cell<State>,
    capabilities: CapabilityFlags,
    sql: &str,
) -> io::Result<QuerySpan> {
    let mut span = QuerySpan::opened(Driver::MySql, sql);
    text_command(wire, state, capabilities, sql)?;
    span.finished(None);
    Ok(span)
}

/// One `COM_QUERY`, and the status packet a command with no result set owes.
///
/// Free and generic in the stream for this crate's usual reason, and split from
/// [`simple_command`] for a second one: [`begin`]'s isolation level is two
/// commands that the caller waited for as one, so the span belongs to the pair
/// rather than to either half of it.
///
/// # Errors
///
/// `InvalidInput` for a connection that is not [`State::Idle`], in
/// [`crate::pg`]'s `second_statement` wording because § 4 is the rule being
/// broken and both of its fixes are what that caller needs to hear; otherwise
/// as [`read_ok`]. [`poison_on_write`] decides what a failure leaves behind —
/// the server's own refusal leaves the connection idle and poolable, and
/// anything else leaves a wire no packet boundary can be found in.
fn text_command<S: Read + Write>(
    wire: &mut Wire<S>,
    state: &Cell<State>,
    capabilities: CapabilityFlags,
    sql: &str,
) -> io::Result<()> {
    if !state.get().may_start_statement() {
        return Err(crate::pg::second_statement(state));
    }

    let mut payload = Vec::with_capacity(1 + sql.len());
    payload.push(COM_QUERY);
    payload.extend_from_slice(sql.as_bytes());

    state.set(State::Executing);
    wire.codec.reset_seq_id();
    if let Err(e) = wire.send(&payload) {
        return Err(poison_on_write(state, e));
    }
    match read_ok(wire, capabilities) {
        Ok(()) => {
            state.set(State::Idle);
            Ok(())
        }
        Err(e) => Err(poison_on_write(state, e)),
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
/// [ADR 0132 § 2](/docs/decisions/0132.md)'s
/// borrowed codec answering in its own vocabulary. [`scalar`] is what turns one
/// into the Novis value [ADR 0067 § 9](/docs/decisions/0067.md)'s
/// table names, against the column it belongs to — the boundary
/// [`crate::PgColumn::decode`] sits on for the other driver.
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

/// A calendar date, in the fields the server sent and no further.
///
/// [`crate::PgDate`]'s opposite number and the same promise: a
/// `Core\Time\Date` is an instance of a class `nvs-stdlib` declares, this crate
/// cannot allocate one, and so [ADR 0067 § 9](/docs/decisions/0067.md)'s
/// structured rows are finished one layer up.
///
/// Nothing here parsed a rendering — MySQL's binary protocol sends the
/// components as integers — so the fields are the server's own and are not
/// checked at all. MySQL's zero date arrives as `0000-00-00`, month and day
/// both zero, and is refused by the type that has a calendar in it rather than
/// by this one, which is where every other impossible date is refused too.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct MySqlDate {
    /// The year, `0` for the zero date and `1` to `9999` otherwise.
    pub year: i32,
    /// The month, 1 to 12, or `0` in the zero date.
    pub month: u8,
    /// The day of the month, 1 to 31, or `0` in the zero date.
    pub day: u8,
}

impl std::fmt::Debug for MySqlDate {
    /// The type and none of the fields, for the reason [`MySqlRow`]'s own
    /// rendering gives: a decoded column is one request's data.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MySqlDate").finish_non_exhaustive()
    }
}

/// A time of day, to the nanosecond, in the fields the server sent.
///
/// The reason this is components rather than an instance is [`MySqlDate`]'s.
///
/// **A `TIME` that is not a time of day never reaches this struct.** MySQL's
/// `TIME` is a signed interval of `-838:59:59` to `838:59:59` — it carries a
/// sign and a day count — while § 9 reads the column as a `Core\Time\TimeOfDay`,
/// which has neither. [`time_of_day`] refuses the values that fall outside a
/// day, so what this holds is always a clock reading.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct MySqlTime {
    /// The hour, 0 to 23.
    pub hour: u8,
    /// The minute, 0 to 59.
    pub minute: u8,
    /// The second, 0 to 59. MySQL has no leap second: the column stops at 59.
    pub second: u8,
    /// The nanosecond within the second. MySQL stores microseconds, so the
    /// last three digits are always zero; the field counts nanoseconds because
    /// that is what every `Core\Time` type holds.
    pub nanosecond: u32,
}

impl std::fmt::Debug for MySqlTime {
    /// As [`MySqlDate`]'s: the type, and none of the data.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MySqlTime").finish_non_exhaustive()
    }
}

/// One column's value, decoded but not yet allocated as a [`Value`].
///
/// [`crate::PgScalar`]'s opposite number, one row of § 9's table per variant,
/// and it exists here for the reasons it exists there: everything that can go
/// wrong with a column happens before anything is allocated, so § 9's whole
/// table is assertable in a `-p nvs-db` test that leaks nothing, and the rows
/// whose Novis type is a class instance have somewhere to be returned as
/// components — see [`MySqlDate`].
///
/// **Fewer rows than [`crate::PgScalar`] carries.** MySQL has no
/// `TIMESTAMPTZ`, so nothing here carries its own offset — § 9's zone-less row
/// is [`Self::DateTime`] and the zone is the one `set_session_time_zone`
/// declared — and MySQL has no `UUID` column type either: § 9 sends its
/// `BINARY(16)` to the `bytes` row, and the backend that does have the type is
/// MariaDB, which is its own driver. There is no array variant for the matching
/// PostgreSQL's `array<T>` is a type and MySQL's `SET` is a text column with a
/// flag, which [`column_type`] describes as [`ColumnType::Other`].
///
/// A `Text` and a `Bytes` borrow the row they were decoded out of, where
/// [`crate::PgScalar`] owns its octets: this protocol carries them as
/// themselves rather than as a text rendering that had to be unhexed, so there
/// is nothing for the decoder to own and no `into_owned` to make.
pub enum MySqlScalar<'a> {
    /// SQL `NULL`: the row of § 9's table that makes every column `?T`.
    Null,
    /// `BIT(1)`. MySQL has no `BOOLEAN` — `TINYINT(1)` is § 6's `int` — so this
    /// is the one column type that reaches it.
    Bool(bool),
    /// A signed `TINYINT`/`SMALLINT`/`MEDIUMINT`/`INT`/`BIGINT`, and `YEAR`.
    Int(i64),
    /// The same set carrying `UNSIGNED`, which is § 9's `uint` row and the
    /// range `BIGINT UNSIGNED` needs.
    UInt(u64),
    /// `FLOAT`/`DOUBLE`.
    Float(f64),
    /// `DECIMAL`, which the binary protocol sends as its digits.
    Decimal(Decimal),
    /// A `tainted string`'s text, already proven well-formed UTF-8. Borrowed
    /// out of the row.
    Text(&'a str),
    /// A `tainted bytes`'s octets, borrowed out of the row.
    Bytes(&'a [u8]),
    /// `DATE`, which is a `Core\Time\Date`.
    Date(MySqlDate),
    /// `TIME`, which is a `Core\Time\TimeOfDay`.
    Time(MySqlTime),
    /// `DATETIME` and `TIMESTAMP`: § 9's zone-less row, a `Core\Time\DateTime`
    /// in the zone [`MySqlTarget::time_zone`] declared and the server was told.
    DateTime {
        /// The civil date, as the server sent it.
        date: MySqlDate,
        /// The civil time, as the server sent it.
        time: MySqlTime,
    },
}

impl std::fmt::Debug for MySqlScalar<'_> {
    /// Which row of § 9's table this landed on, and never the value: a column
    /// in flight is one request's data, which is the rule [`MySqlRow`]'s own
    /// rendering holds.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            MySqlScalar::Null => "null",
            MySqlScalar::Bool(_) => "bool",
            MySqlScalar::Int(_) => "int",
            MySqlScalar::UInt(_) => "uint",
            MySqlScalar::Float(_) => "float",
            MySqlScalar::Decimal(_) => "decimal",
            MySqlScalar::Text(_) => "string",
            MySqlScalar::Bytes(_) => "bytes",
            MySqlScalar::Date(_) => "date",
            MySqlScalar::Time(_) => "time",
            MySqlScalar::DateTime { .. } => "datetime",
        })
    }
}

impl MySqlScalar<'_> {
    /// The Novis value, taking on the one reference a `string` or a `bytes`
    /// costs and nothing at all for the rest.
    ///
    /// `None` for § 9's structured rows, whose Novis type is a class instance
    /// this crate cannot allocate — [`MySqlDate`] owns why. A caller that wants
    /// the whole table matches those first and reaches this for everything
    /// left, which is what [`crate::PgScalar::into_value`]'s caller already
    /// does for the other driver.
    #[must_use]
    pub fn into_value(self) -> Option<Value> {
        Some(match self {
            MySqlScalar::Null => Value::null(),
            MySqlScalar::Bool(value) => Value::bool(value),
            MySqlScalar::Int(value) => Value::int(value),
            MySqlScalar::UInt(value) => Value::uint(value),
            MySqlScalar::Float(value) => Value::float(value),
            MySqlScalar::Decimal(value) => Value::decimal(value),
            MySqlScalar::Text(text) => Value::str(NvsStr::new(text.as_bytes())),
            MySqlScalar::Bytes(octets) => Value::bytes(NvsStr::new(octets)),
            MySqlScalar::Date(_) | MySqlScalar::Time(_) | MySqlScalar::DateTime { .. } => {
                return None;
            }
        })
    }
}

/// One bound parameter as the octets `COM_STMT_EXECUTE` carries it in:
/// [`crate::encode`]'s opposite number, and deliberately not the same
/// rendering.
///
/// **Every parameter is sent as a length-encoded string and the server casts it
/// to the column's type** ([`execute`] says why that is not an escaping
/// decision), so this is a text rendering exactly as PostgreSQL's is — and it
/// is a *different* text rendering, because the two servers read different
/// literals. The rows of [ADR 0067 § 9](/docs/decisions/0067.md)
/// that differ are below, each one a value the other driver's rendering would
/// store wrongly rather than fail on:
///
/// - A `bool` is `1`/`0`. PostgreSQL's `t`/`f` are not boolean input here at
///   all — MySQL casts `'t'` to the number `0` — so the wrong rendering is a
///   quietly wrong row and not an error.
/// - A `bytes` is its own octets. `bytea` has a hex input form and `BLOB` has
///   none, because a string parameter already *is* the octets, so `\x61` would
///   be stored as those four characters.
/// - A non-finite `float` is refused. MySQL has no `Infinity` or `NaN` literal
///   and its `DOUBLE` holds neither value; the cast of the word is `0`, so a
///   refusal naming the tag is the only answer that is not a wrong number in a
///   column. PostgreSQL accepts all three, and that divergence is the servers'
///   rather than this crate's.
///
/// A `Core\Db\InList` never reaches here for [`crate::encode`]'s reason: § 5's
/// marker has expanded into one bound value per element by the time a statement
/// has its bind list.
///
/// # Errors
///
/// `InvalidInput` for a value with no form to send — an array, an object, a
/// callable, and the three non-finite floats — where the whole answer is the
/// tag and never the value, for the reason [`malformed`] gives.
pub fn encode(value: Value) -> io::Result<Option<Vec<u8>>> {
    let rendered = match value.tag() {
        Some(Tag::Null) => return Ok(None),
        Some(Tag::Bool) => String::from(if value.as_bool() == Some(true) {
            "1"
        } else {
            "0"
        }),
        Some(Tag::Int) => value.as_int().unwrap_or_default().to_string(),
        Some(Tag::Uint) => value.as_uint().unwrap_or_default().to_string(),
        Some(Tag::Float) => {
            let float = value.as_float().unwrap_or_default();
            if !float.is_finite() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "a `float` that is not finite has no MySQL literal and no `DOUBLE` value — \
                     the three of them are what this driver cannot bind, and a program that \
                     stores one writes its own text column",
                ));
            }
            // Rust's shortest round-tripping form, which `DOUBLE`'s own string
            // cast reads back to the same bits.
            float.to_string()
        }
        // Exact on both sides, as PostgreSQL's is: `rule:types/decimal`'s `decimal` renders
        // as digits and a point, which is `DECIMAL`'s own input form, so
        // nothing rounds here the way binding it as a `DOUBLE` would.
        Some(Tag::Decimal) => value
            .as_decimal()
            .map(|exact| exact.to_string())
            .unwrap_or_default(),
        // A `string` is UTF-8 by `rule:types/bytes` and the session is `utf8mb4` by the
        // connect path, so the octets go out as they are — and so do a
        // `bytes`'s, which is the row that differs from the other driver.
        Some(Tag::Str) => {
            return Ok(Some(value.as_str_bytes().unwrap_or_default().to_vec()));
        }
        Some(Tag::Bytes) => {
            return Ok(Some(value.as_bytes().unwrap_or_default().to_vec()));
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "a value of tag {} has no form this driver can bind",
                    value.tag_byte()
                ),
            ));
        }
    };
    Ok(Some(rendered.into_bytes()))
}

/// One column's value as the Novis value § 9's table names, or `None` for the
/// rows that are class instances.
///
/// [`crate::PgColumn::decode`] on the other driver, and free rather than a
/// method for the reason [`column_type`] is: `Column` is `mysql_common`'s type
/// and an inherent `impl` belongs to the crate that declares it.
///
/// # Errors
///
/// As [`scalar`].
pub fn decode(column: &Column, value: &MyValue) -> io::Result<Option<Value>> {
    Ok(scalar(column, value)?.into_value())
}

/// [`decode`]'s whole decision, before anything is allocated, and every row of
/// § 9's table rather than the scalar half.
///
/// `nvs-stdlib` calls this one: it is the only crate that can turn a
/// [`MySqlScalar::Date`] and its two siblings into the `Core\Time` instances
/// the table names.
///
/// **The column decides the row and the value only fills it in**, which is why
/// this takes both and why [`column_type`] is the match's subject: a `BIT(1)`
/// and a `BIT(8)` arrive as the same [`MyValue::Bytes`] and § 9 sends them to
/// different rows, exactly as `ENUM` and `SET` do. The one thing read off the
/// value instead is `NULL`, which every column type may be.
///
/// # Errors
///
/// `InvalidData` for a value its column's own type cannot be read out of: a
/// `DECIMAL` past what `rule:types/decimal`'s
/// `decimal` holds, a text column whose octets are not UTF-8, a `BIT(1)` that
/// is neither bit, a `TIME` outside a day ([`time_of_day`]), and a value whose
/// shape is not the one its column declared, which is a server that did not
/// send the row those definitions describe. Every such message names the
/// column and its type and **never the value**, for the reason
/// [`crate::PgColumn::decode`] gives.
pub fn scalar<'a>(column: &Column, value: &'a MyValue) -> io::Result<MySqlScalar<'a>> {
    Ok(match (column_type(column), value) {
        // Asked first and off the value, because § 9's `?T` is every row's
        // and a column that was null says nothing about its type.
        (_, MyValue::NULL) => MySqlScalar::Null,
        (ColumnType::Int, MyValue::Int(number)) => MySqlScalar::Int(*number),
        // A `uint` column's value is put back under its declared type in
        // `decode_row`, which is where that rule lives and why this arm sees a
        // `UInt` at all.
        (ColumnType::Uint, MyValue::UInt(number)) => MySqlScalar::UInt(*number),
        (ColumnType::Float, MyValue::Float(number)) => MySqlScalar::Float(widened(*number)),
        (ColumnType::Float, MyValue::Double(number)) => MySqlScalar::Float(*number),
        (ColumnType::Decimal, MyValue::Bytes(digits)) => {
            MySqlScalar::Decimal(decimal(column, text(column, digits)?)?)
        }
        // § 9's `BIT(1)` row: one octet, and the width is what said this
        // column was a `bool` at all.
        (ColumnType::Bool, MyValue::Bytes(bits)) => MySqlScalar::Bool(match bits.as_slice() {
            [0] => false,
            [1] => true,
            _ => return Err(malformed(column, "a one-bit string")),
        }),
        // § 9 keeps JSON at `tainted string`: it is never auto-decoded, and
        // `Core\Json::decode` is one honest call.
        (ColumnType::Text | ColumnType::Json, MyValue::Bytes(body)) => {
            MySqlScalar::Text(text(column, body)?)
        }
        (ColumnType::Bytes, MyValue::Bytes(body)) => MySqlScalar::Bytes(body),
        (ColumnType::Date, MyValue::Date(year, month, day, ..)) => {
            MySqlScalar::Date(civil_date(*year, *month, *day))
        }
        (ColumnType::DateTime, MyValue::Date(year, month, day, hour, minute, second, micros)) => {
            MySqlScalar::DateTime {
                date: civil_date(*year, *month, *day),
                time: civil_time(*hour, *minute, *second, *micros),
            }
        }
        (ColumnType::Time, MyValue::Time(negative, days, hour, minute, second, micros)) => {
            MySqlScalar::Time(time_of_day(
                column, *negative, *days, *hour, *minute, *second, *micros,
            )?)
        }
        // § 9's last row, which on this protocol is two rows and not one.
        // `BIT(n>1)` and `GEOMETRY` are octets that are not text in any
        // encoding, while a `SET` is its members joined by commas; the charset
        // is what separates them, exactly as it separates a `BLOB` from a
        // `TEXT` one arm up. § 9 words that row as `tainted string` because it
        // was written against PostgreSQL, whose server renders every one of
        // them as text before it reaches a driver.
        (ColumnType::Other, MyValue::Bytes(body)) => {
            if column.character_set() == BINARY_CHARSET {
                MySqlScalar::Bytes(body)
            } else {
                MySqlScalar::Text(text(column, body)?)
            }
        }
        // Every pairing left, including the rows no MySQL column describes as:
        // a value whose shape is not its column's is a server that did not
        // send the row these definitions describe.
        _ => return Err(malformed(column, "the type its definition declared")),
    })
}

/// A `FLOAT`'s four bytes as the `float` § 9 makes them, by way of the
/// shortest decimal that reads back as the same `f32`.
///
/// `f64::from` would be free and would answer the `f32`'s *exact* value:
/// `0.100000001490116119384765625` for a column holding `0.1`. That is a
/// different double from the one [`crate::PgColumn::scalar`] answers for the
/// same row — PostgreSQL renders `float4` as `0.1` and that driver parses the
/// rendering — and § 9's table is one table for every backend, so the same
/// column on two of them reading back as two different numbers is the thing
/// worth a `to_string` per value. `DOUBLE`, which is every wider column, takes
/// neither path.
fn widened(value: f32) -> f64 {
    value
        .to_string()
        .parse()
        .unwrap_or_else(|_| f64::from(value))
}

/// The date components the binary protocol sent, widened to the fields every
/// `Core\Time` type counts in.
fn civil_date(year: u16, month: u8, day: u8) -> MySqlDate {
    MySqlDate {
        year: i32::from(year),
        month,
        day,
    }
}

/// The time components the binary protocol sent. MySQL stores microseconds, so
/// the last three digits of the nanosecond are always zero; the field counts
/// nanoseconds because that is what every `Core\Time` type holds.
fn civil_time(hour: u8, minute: u8, second: u8, micros: u32) -> MySqlTime {
    MySqlTime {
        hour,
        minute,
        second,
        nanosecond: micros.saturating_mul(1_000),
    }
}

/// A `TIME` column's value as the clock reading § 9 makes it.
///
/// # Errors
///
/// `InvalidData` for the values MySQL's `TIME` has and a `Core\Time\TimeOfDay`
/// does not: a negative one, and one carrying whole days — the column's range
/// is `-838:59:59` to `838:59:59` because it doubles as an interval type, and
/// most of that range is not a time of day at all. The refusal is
/// here rather than one layer up because it is the column type's own shape and
/// not the calendar's: `nvs-stdlib` is handed hours, minutes and seconds, and
/// a day count has nowhere to go in them.
fn time_of_day(
    column: &Column,
    negative: bool,
    days: u32,
    hour: u8,
    minute: u8,
    second: u8,
    micros: u32,
) -> io::Result<MySqlTime> {
    if negative || days != 0 || hour > 23 {
        return Err(malformed(column, "a time of day"));
    }
    Ok(civil_time(hour, minute, second, micros))
}

/// A value's octets as text, **checked**.
///
/// The connection's charset is `utf8mb4` and the server was told so, but a
/// column's own charset is per column and a `latin1` one still arrives; the
/// check is what keeps § 9's `tainted string` a Novis `string`, which
/// `rule:types/bytes` makes UTF-8 by
/// definition.
fn text<'a>(column: &Column, body: &'a [u8]) -> io::Result<&'a str> {
    std::str::from_utf8(body).map_err(|_| malformed(column, "well-formed UTF-8"))
}

/// § 9's `DECIMAL` row: the server's own rendering as a [`Decimal`], and the
/// two reasons it may not be one.
///
/// `crate::pg::PgColumn::decimal` is the same split on the other wire driver
/// and owns why there are two — a value past `rule:types/decimal`'s range is a
/// schema an operator narrows, and text that is not a literal is a driver
/// reading a body it was not sent. What is written twice is the column's own
/// metadata, which is each driver's and not shared: a MySQL column names its
/// declared type where a PostgreSQL one names an OID.
///
/// # Errors
///
/// `InvalidData` either way, and neither message carries the value.
fn decimal(column: &Column, text: &str) -> io::Result<Decimal> {
    Decimal::read(text).map_err(|why| match why {
        NotDecimal::PastRange => io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "column {:?} of type {:?} holds more precision than a `decimal` does — 96 \
                 mantissa bits and a scale of 28 — so the column's own type is what narrows",
                column.name_str(),
                column.column_type()
            ),
        ),
        NotDecimal::Unreadable => malformed(column, "a decimal"),
    })
}

/// The refusal every decode above answers with: the column, its declared type,
/// and **not one byte of the value**.
///
/// A message carrying the value would put one request's data into a log line
/// and into whatever the operator's error reporting forwards it to; the column
/// and its type are what a person fixing the schema needs, and they are the
/// server's own metadata rather than the row's.
fn malformed(column: &Column, wanted: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!(
            "column {:?} of type {:?} did not decode as {wanted}",
            column.name_str(),
            column.column_type()
        ),
    )
}

/// A statement's result, and the rows still to come out of it.
///
/// [`crate::PgRows`]' shape, and it is the same borrow for the same reason: the
/// handle holds the wire and the busy state, so the connection is unusable for
/// anything else until the stream ends — which is `rule:core-classes/db-statement-members`'s
/// one-statement-at-a-time rule enforced by the type system rather than by a
/// check every caller has to remember.
///
/// A statement with no result set answers one of these too, already ended: its
/// [`Self::next_row`] is `None` on the first call and [`Self::affected`] is the
/// number the server's status packet carried.
///
/// Everything about the result set that is not the borrow is [`MySqlCursor`],
/// which owns why that is a split rather than its fields inlined here.
pub struct MySqlRows<'a, S: Read + Write = MyStream> {
    wire: &'a mut Wire<S>,
    state: &'a Cell<State>,
    reading: MySqlCursor,
}

/// A statement's read state: what the result set described, how far it has got,
/// what ended it, and the event it is being timed by — everything a row needs
/// that is not the wire.
///
/// [`crate::PgCursor`]'s twin, and a type of its own for that one's reason:
/// `rule:core-classes/db-statement-members`'s rows are reached two ways and only one of them can
/// hold a borrow. The **buffered** members drain their rows inside the call that
/// started the statement, so [`MySqlRows`] keeps this beside a borrow of the
/// connection and the borrow checker is what refuses a second statement.
/// **`Core\Db\Connection::stream`** hands a walk back to the program and is
/// advanced by a *later* call, with nothing of the connection borrowed in
/// between; a borrow cannot span that, so its copy of this state is parked on
/// the connection ([`MySqlConn::stream`]) and [`State::Streaming`] is what
/// refuses the second statement there.
///
/// Both drive [`next_row_of`], which is the one place in this driver a binary
/// row packet is read, so the two paths cannot disagree about what ends a result
/// set or about what the status packet said on the way past.
#[derive(Debug)]
pub(crate) struct MySqlCursor {
    /// What the two ends agreed this connection can do, carried here so a row
    /// read through a fresh borrow of the wire needs nothing else off the
    /// connection: the terminator is deserialized against these bits.
    capabilities: CapabilityFlags,
    /// Shared rather than borrowed because `mysql_common`'s binary row
    /// deserializer takes exactly this: an `Arc<[Column]>` per row, which is a
    /// refcount bump and not a copy of the definitions.
    columns: Arc<[Column]>,
    rows: u64,
    affected: u64,
    last_id: Option<u64>,
    ended: bool,
    /// `rule:observability/a-query-is-a-trace-event`'s trace event for this statement, opened when it went out
    /// and ended by whatever ends the stream — [`crate::PgRows`]' field, for
    /// [`crate::span`]'s reasons, and the reason § 11 reads across drivers at
    /// all.
    span: QuerySpan,
}

impl MySqlCursor {
    /// The result set's column definitions, empty for a statement that returned
    /// none — [`MySqlRows::columns`] and [`MySqlConn::stream_columns`] are both
    /// this.
    pub(crate) fn columns(&self) -> &[Column] {
        &self.columns
    }

    /// `rule:observability/a-query-is-a-trace-event`'s trace event for the statement this walks.
    pub(crate) fn span(&self) -> &QuerySpan {
        &self.span
    }

    /// Names the `[db.<name>]` block the statement ran on — [`QuerySpan::name`]
    /// owns why the driver cannot work it out for itself.
    pub(crate) fn name_connection(&mut self, connection: &str) {
        self.span.name(connection);
    }
}

impl<S: Read + Write> std::fmt::Debug for MySqlRows<'_, S> {
    /// The shape of the result and where the wire is, and nothing that arrived.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MySqlRows")
            .field("columns", &self.reading.columns.len())
            .field("state", &self.state.get())
            .finish_non_exhaustive()
    }
}

impl<S: Read + Write> MySqlRows<'_, S> {
    /// The result set's column definitions, empty for a statement that returns
    /// none.
    #[must_use]
    pub fn columns(&self) -> &[Column] {
        &self.reading.columns
    }

    /// Column `index`'s Novis type, per [`column_type`], or `None` where the
    /// result set has no such column.
    #[must_use]
    pub fn column_type(&self, index: usize) -> Option<ColumnType> {
        self.reading.columns.get(index).map(column_type)
    }

    /// [ADR 0067 § 11](/docs/decisions/0067.md)'s trace event for this
    /// statement.
    ///
    /// Borrowed rather than taken, for [`crate::PgRows::span`]'s reason: a
    /// caller reading it mid-stream is asking a running statement how far it has
    /// got, and [`QuerySpan::duration`] answers from the clock until the stream
    /// ends and freezes it.
    #[must_use]
    pub fn span(&self) -> &QuerySpan {
        &self.reading.span
    }

    /// Names the `[db.<name>]` block this statement ran on, for the layer that
    /// resolved it — [`QuerySpan::name`] owns why the driver cannot.
    pub fn name_connection(&mut self, connection: &str) {
        self.reading.span.name(connection);
    }

    /// [ADR 0067 § 4](/docs/decisions/0067.md)'s affected-row count,
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
        self.reading.ended.then_some(self.reading.affected)
    }

    /// [ADR 0067 § 4](/docs/decisions/0067.md)'s `lastId` — the
    /// `AUTO_INCREMENT` value this statement generated, once the stream has
    /// ended, and `None` where it generated none.
    ///
    /// MySQL puts it in the status packet, so unlike PostgreSQL it belongs to
    /// the write that produced it with no `RETURNING` clause to ask for. A
    /// statement that returned a result set generated no key and answers `None`
    /// too, which is [`Self::affected`]'s shape read the other way round: that
    /// member's `None` is the stream that has not ended, and this one's is a key
    /// the statement never produced.
    #[must_use]
    pub fn last_id(&self) -> Option<u64> {
        self.reading.ended.then_some(self.reading.last_id).flatten()
    }

    /// The next row, or `None` once the stream has ended.
    ///
    /// [`next_row_of`] is the whole of it, and the parked walk
    /// [`MySqlConn::stream_next_row`] advances is the same call over the same
    /// state. Deliberately not `Iterator::next`, for
    /// [`crate::PgRows::next_row`]'s reason: every call can fail, and an
    /// `Option` would have to swallow it.
    ///
    /// # Errors
    ///
    /// As [`next_row_of`].
    pub fn next_row(&mut self) -> io::Result<Option<MySqlRow>> {
        next_row_of(self.wire, self.state, &mut self.reading)
    }
}

/// One row off a live result set, or `None` once it has ended — the whole of
/// [`MySqlRows::next_row`], and of [`MySqlConn::stream_next_row`] with it.
///
/// Free, and generic in the stream, for the two reasons [`crate::pg`]'s
/// `next_row_of` gives: the parked path has no [`MySqlRows`] to call a method
/// on, its state being on the connection and its wire reached through a fresh
/// borrow per row; and a [`MySqlConn`]'s own wire is at the default type
/// parameter, so anything reachable only through an inherent method on it would
/// need a socket and a certificate to reach at all.
///
/// **The first byte says which thing arrived, with no ambiguity to weigh**,
/// unlike the shapes [`read_answer`] tells apart: a binary row always opens
/// `0x00`, the terminator is the `0xFE` status packet `CLIENT_DEPRECATE_EOF`
/// promises instead of an EOF packet, and `0xFF` is the server's own error. A
/// length is not read here because none is needed — the collision `read_answer`
/// reads one for is between a status packet and a column count, and neither of
/// those can be at this point in the stream.
///
/// Ending the stream is what returns the connection to [`State::Idle`].
///
/// # Errors
///
/// The server's own error, which still ends the stream cleanly and leaves the
/// connection idle; `InvalidData` for a packet the protocol does not allow here,
/// or for a second result set, which poison it; and whatever the stream
/// reported.
pub(crate) fn next_row_of<S: Read + Write>(
    wire: &mut Wire<S>,
    state: &Cell<State>,
    reading: &mut MySqlCursor,
) -> io::Result<Option<MySqlRow>> {
    // The state is the only bookkeeping: anything that ended this stream —
    // a terminator, a server error, a poisoning — has already left it.
    if state.get() != State::Streaming {
        return Ok(None);
    }

    let packet = match wire.read_packet() {
        Ok(packet) => packet,
        Err(e) => return Err(poison_on_write(state, e)),
    };
    match packet.first() {
        Some(0x00) => {
            let row =
                decode_row(&reading.columns, &packet).map_err(|e| poison_on_write(state, e))?;
            reading.rows += 1;
            reading.span.row();
            Ok(Some(row))
        }
        Some(0xFE) => {
            let terminator = OkPacketDeserializer::<ResultSetTerminator>::deserialize(
                reading.capabilities,
                &mut ParseBuf(&packet),
            )
            .map_err(|e| poison_on_write(state, e))?
            .into_inner();
            reading.ended = true;
            reading.affected = reading.rows;
            // [`MySqlRows::affected`]'s two numbers under one name, and the
            // span takes the same one: for a result set that is the rows
            // that came back, which is what PostgreSQL's `SELECT 2` tag
            // puts in the other driver's span.
            reading.span.finished(Some(reading.affected));
            if terminator
                .status_flags()
                .contains(StatusFlags::SERVER_MORE_RESULTS_EXISTS)
            {
                // A stored procedure's second result set. There is no
                // reader here to drain it with and no surface in `rule:core-classes/db-statement-members` to hand it to, and walking away from packets that are
                // still coming is what leaves the wire pointing into the
                // middle of one — [`read_ok`] refuses the same thing for
                // the same reason.
                return Err(poison_on_write(
                    state,
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "the server has a second result set for this statement, and ADR \
                         0067 § 4's one-statement-at-a-time surface has nowhere to put it",
                    ),
                ));
            }
            state.set(State::Idle);
            Ok(None)
        }
        Some(0xFF) => {
            // A refused statement is still a statement that ran, and the
            // error packet arrived whole: the wire is at a boundary, so
            // `poison_on_write` leaves the connection idle rather than
            // poisoned.
            reading.ended = true;
            reading.affected = reading.rows;
            // No affected count on the span, as [`crate::PgRows::next_row`]
            // does it: § 11 gives a span no success field to lose, so a
            // refused statement reports the rows that did arrive and the
            // error is the caller's own return value.
            reading.span.finished(None);
            Err(poison_on_write(
                state,
                server_refusal(wire.backend, &packet, reading.capabilities),
            ))
        }
        _ => Err(poison_on_write(
            state,
            io::Error::new(
                io::ErrorKind::InvalidData,
                "the server sent something that is neither a binary row, the end of a \
                 result set, nor an error",
            ),
        )),
    }
}

/// Reads what is left of a result set nobody wants, to the terminator that ends
/// it.
///
/// Abandonment is the ordinary case rather than an error: the rows are coming
/// whether or not anybody reads them, so draining is what returns the connection
/// to the pool instead of closing it — [`crate::PgRows`]' `Drop` and
/// `rule:core-classes/db-connection-busy-state`'s rule for both. A read that fails on the way poisons
/// the connection through the same helper every other read here uses, and the
/// loop ends because that leaves [`State::Streaming`].
fn drain_result<S: Read + Write>(
    wire: &mut Wire<S>,
    state: &Cell<State>,
    reading: &mut MySqlCursor,
) {
    while state.get() == State::Streaming {
        if next_row_of(wire, state, reading).is_err() {
            break;
        }
    }
}

/// Abandons a parked walk and forgets what it read — [`MySqlConn::end_stream`],
/// and [`crate::MariaConn::end_stream`] with it.
///
/// The walk is drained first, by [`drain_result`], so the connection this was
/// read off is at a message boundary and poolable when this returns. Dropping
/// the state is the other half: a result set's shape and its counts are one
/// request's, and `rule:security/db-pool-reset-is-a-boundary` is why the next one must not be able to
/// read them.
pub(crate) fn end_stream_of<S: Read + Write>(
    wire: &mut Wire<S>,
    state: &Cell<State>,
    reading: &mut Option<MySqlCursor>,
) {
    if let Some(walk) = reading.as_mut() {
        drain_result(wire, state, walk);
    }
    *reading = None;
}

impl<S: Read + Write> Drop for MySqlRows<'_, S> {
    /// Abandonment, and it is the ordinary case rather than an error —
    /// [`drain_result`] is the whole of it and owns the reasoning.
    fn drop(&mut self) {
        drain_result(self.wire, self.state, &mut self.reading);
    }
}

/// Files a failure against the connection, poisoning it unless the failure is
/// one the server worded.
///
/// The split is the difference between "this statement did not work" and "this
/// wire is no longer a sequence of packets". A [`ServerError`] is the server's
/// own `ERR` packet, which arrived whole and left the connection at a boundary —
/// the next statement on it is fine. Anything else reached here with the stream
/// in a position nothing has proven, and § 4's answer to a boundary that cannot
/// be proven is poison.
///
/// Asking [`ServerError::of`] rather than the `io::ErrorKind` also moves the
/// `LOCAL INFILE` refusal to the poisoning side, where it belongs: [`read_answer`]
/// answers that one itself, having written the empty transfer packet and *not*
/// read the status the server sends back, so the next packet on that wire is one
/// nobody has accounted for.
fn poison_on_write(state: &Cell<State>, error: io::Error) -> io::Error {
    if ServerError::of(&error).is_some() {
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
pub(crate) fn say_goodbye<S: Read + Write>(wire: &mut Wire<S>) {
    wire.codec.reset_seq_id();
    let _ = wire.send(&[COM_QUIT]);
}

#[cfg(test)]
mod tests {
    use sha2::{Digest, Sha256};

    use super::{
        AuthContext, Backend, CLIENT_CAPABILITIES, COLLATION, COM_QUERY, DbErrorKind, Greeting,
        Login, MYSQL, MySqlTarget, MyValue, NvsStr, Prepared, REQUIRED_CAPABILITIES, ServerError,
        State, Value, Wire, authenticate, begin, column_type, commit, encode, execute,
        execute_many, kind_of, next_row_of, offset_literal, open_result, plugin_or_refuse,
        read_greeting, read_ok, request_tls, roll_back, scalar, server_refusal, start_statement,
    };
    use crate::conn::ColumnType as NovisType;
    use crate::conn::{BlockError, Driver, Isolation};
    use crate::maria::{
        EXTENDED_CAPABILITIES, MARIADB, MYSQL_NATIVE_PASSWORD,
        plugin_or_refuse as mariadb_plugin_or_refuse,
    };
    use mysql_common::constants::{
        CapabilityFlags, ColumnFlags, ColumnType, MariadbCapabilities, StatusFlags,
    };
    use mysql_common::io::ParseBuf;
    use mysql_common::packets::{Column, ComStmtExecuteRequestBuilder, HandshakePacket};
    use mysql_common::proto::{MyDeserialize, MySerialize};
    use nvs_config::tree::Database;
    use std::cell::Cell;
    use std::io;

    /// The nonce every case challenges with, and the password every case
    /// refuses to find on the wire.
    const NONCE: &[u8; 20] = b"NR3HP:qIYa_9=db?Sd{`";
    const PASSWORD: &str = "correct-horse-battery";

    /// A cache that never caches, so a case about the wire asserts both round
    /// trips rather than one.
    fn no_cache() -> crate::sql::StatementCache<Prepared> {
        sized_cache(0)
    }

    /// A cache of `capacity` statements, for the cases that are about § 1's
    /// cache itself.
    fn sized_cache(capacity: usize) -> crate::sql::StatementCache<Prepared> {
        crate::sql::StatementCache::new(capacity)
    }

    /// A server that answers the client rather than a script.
    ///
    /// [`crate::pg`]'s `Peer` and for its reason: an authentication exchange's
    /// every packet depends on the one before it, so a canned transcript could
    /// only assert that we send *something*. A flush is the message boundary —
    /// every write in this module is `write_all` then `flush` — so the Rust closure
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
    /// every modern server offers, and offering no extended capabilities —
    /// which is what a MySQL server's trailing filler decodes to.
    fn greeting_bytes(plugin: &str, capabilities: CapabilityFlags) -> Vec<u8> {
        greeting_offering(plugin, capabilities, MariadbCapabilities::empty())
    }

    /// [`greeting_bytes`] with the second capability word said out loud: a
    /// MariaDB server writes it where MySQL's filler is, and every case about
    /// that word needs to script both a server that offers a bit and one that
    /// does not.
    fn greeting_offering(
        plugin: &str,
        capabilities: CapabilityFlags,
        extended: MariadbCapabilities,
    ) -> Vec<u8> {
        greeting_versioned("8.0.36", plugin, capabilities, extended)
    }

    /// [`greeting_offering`] from a server whose banner is `version`: the field
    /// a distribution writes its own suffix into, and the one
    /// [ADR 0187 § 2](/docs/decisions/0187.md) keeps verbatim.
    fn greeting_versioned(
        version: &str,
        plugin: &str,
        capabilities: CapabilityFlags,
        extended: MariadbCapabilities,
    ) -> Vec<u8> {
        // A real server's second scramble field carries a trailing NUL, and
        // the length byte in front of it counts that byte — a 12-byte tail
        // here would make the *plugin name* start one byte late, which is
        // exactly the shape a hand-rolled greeting gets wrong.
        let mut tail = NONCE[8..].to_vec();
        tail.push(0);

        let mut payload = Vec::new();
        HandshakePacket::new(
            10,
            version.as_bytes(),
            42,
            NONCE[..8].try_into().expect("eight bytes of nonce"),
            Some(tail),
            capabilities,
            COLLATION,
            StatusFlags::empty(),
            Some(plugin.as_bytes()),
        )
        .with_mariadb_ext_capabilities(extended)
        .serialize(&mut payload);
        packet(0, &payload)
    }

    /// Everything a stock MySQL 8 offers that this driver cares about.
    fn server_capabilities() -> CapabilityFlags {
        CLIENT_CAPABILITIES.union(CapabilityFlags::CLIENT_LOCAL_FILES)
    }

    /// The target every case authenticates as.
    /// [`target`]'s login half, with this driver's own roster.
    ///
    /// Separate from the target because [`authenticate`] takes the credential
    /// and the plugin gate rather than a whole target — the split
    /// [`crate::maria`] authenticates through, with its roster in this field.
    fn login() -> Login<'static> {
        Login {
            user: "novis",
            password: PASSWORD,
            database: "shop",
            roster: plugin_or_refuse,
        }
    }

    fn target() -> MySqlTarget<'static> {
        MySqlTarget {
            host: "db.example.internal",
            user: "novis",
            password: PASSWORD,
            database: "shop",
            tls_ca_file: None,
            time_zone: 0,
            statement_cache: crate::sql::DEFAULT_STATEMENT_CACHE,
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

    /// `rule:core-classes/db-capabilities`'s third default and § 1's proof-not-password rule, in the
    /// one exchange that decides both.
    ///
    /// Properties of the same handshake response, because they are readings of
    /// one packet and splitting them would script the same server once each:
    /// the collation is `utf8mb4`, the scramble is the plugin's
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

        let greeting =
            read_greeting(&mut wire, REQUIRED_CAPABILITIES).expect("the greeting decodes");
        request_tls(&mut wire, &greeting).expect("the upgrade request goes out");
        let agreed = authenticate(&mut wire, &login(), &greeting).expect("the server says OK");

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

    /// The last four of a handshake packet's reserved bytes, as the flags they
    /// spell.
    ///
    /// Past the four-byte header, both packets that carry the word have the
    /// same prefix: a capability word, a max-packet word, the collation byte,
    /// then 19 bytes of filler where MySQL has 23 — so the word starts at 32
    /// in either, which is why one reader answers both.
    fn extended_word(sent: &[u8]) -> MariadbCapabilities {
        MariadbCapabilities::from_bits_retain(u32::from_le_bytes(
            sent[32..36]
                .try_into()
                .expect("an extended capability word"),
        ))
    }

    /// What `backend` claimed in each of the two packets that carry a second
    /// capability word — the `SSLRequest` first, the handshake response second
    /// — at a server offering `offered`.
    fn extended_words(
        backend: &'static Backend,
        login: &Login<'_>,
        plugin: &str,
        offered: MariadbCapabilities,
    ) -> (MariadbCapabilities, MariadbCapabilities) {
        let mut step = 0;
        let mut wire = Wire::on(
            backend,
            Peer::new(move |_sent: &[u8]| {
                step += 1;
                match step {
                    1 => Vec::new(),
                    _ => packet(u8::try_from(step).expect("a small step count") + 1, &[0x00]),
                }
            }),
        );
        wire.inbox
            .extend_from_slice(&greeting_offering(plugin, server_capabilities(), offered));

        let greeting =
            read_greeting(&mut wire, REQUIRED_CAPABILITIES).expect("the greeting decodes");
        request_tls(&mut wire, &greeting).expect("the upgrade request goes out");
        authenticate(&mut wire, login, &greeting).expect("the server says OK");

        let sent = &wire.peer().sent;
        (extended_word(&sent[0]), extended_word(&sent[1]))
    }

    /// The second capability word, asserted as the outcomes the intersection
    /// allows rather than as one packet's bytes.
    ///
    /// `MARIADB_CLIENT_STMT_BULK_OPERATIONS` is bit 34 and `CapabilityFlags` is
    /// 32 bits wide, so this word is not a wider version of the first one — it
    /// is a second field, in bytes MySQL treats as filler, and a driver can get
    /// it wrong in two directions that look identical from one side. So every
    /// leg is here: MariaDB at a server that offers the bit claims it,
    /// MariaDB at a server that does not claims nothing, and **MySQL at a
    /// server that offers it still claims nothing** — the leg that fails if the
    /// word is ever derived from the greeting rather than from the driver.
    ///
    /// Each leg reads both packets, because the `SSLRequest` and the handshake
    /// response carry the same word at the same offset and are written by two
    /// different call sites: a client that claimed a capability in the
    /// cleartext half and dropped it in the encrypted one describes itself two
    /// ways to one server, and that is the failure neither packet shows alone.
    #[test]
    fn the_mariadb_handshake_claims_bulk_operations_and_the_mysql_one_claims_nothing() {
        let mariadb = Login {
            user: "novis",
            password: PASSWORD,
            database: "shop",
            roster: mariadb_plugin_or_refuse,
        };

        let offered = EXTENDED_CAPABILITIES;
        assert_eq!(
            extended_words(&MARIADB, &mariadb, MYSQL_NATIVE_PASSWORD, offered),
            (offered, offered),
            "this driver did not claim `MARIADB_CLIENT_STMT_BULK_OPERATIONS` at a MariaDB \
             offering it, in one or both of the packets that carry the word"
        );

        let none = MariadbCapabilities::empty();
        assert_eq!(
            extended_words(&MARIADB, &mariadb, MYSQL_NATIVE_PASSWORD, none),
            (none, none),
            "this driver claimed an extended capability the server never offered"
        );
        assert_eq!(
            extended_words(&MYSQL, &login(), super::CACHING_SHA2_PASSWORD, offered),
            (none, none),
            "the MySQL driver claimed a MariaDB capability, in bytes a MySQL server reads \
             as filler it expects to be zero"
        );
    }

    /// `rule:core-classes/db-capabilities`'s TLS default, asserted where it has to hold: **before**
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

        let refused = read_greeting(&mut wire, REQUIRED_CAPABILITIES)
            .expect_err("a plaintext server is not a connection");
        assert_eq!(refused.kind(), io::ErrorKind::ConnectionRefused);
        assert!(
            wire.peer().sent.is_empty(),
            "the client wrote something to a server it had already refused"
        );
    }

    /// The banner a greeting carried is kept exactly as the server wrote it,
    /// which is what `Core\Db\Connection::serverVersion` answers on this driver
    /// and on MariaDB — [ADR 0187 § 2](/docs/decisions/0187.md).
    ///
    /// Both servers are asked, because the banner is how a MariaDB server says
    /// it is one: a driver answering the `(major, minor, patch)` beside it
    /// would drop that word along with the distribution's suffix. And nothing
    /// is written for either — the greeting is the round trip, and the version
    /// was in it.
    #[test]
    fn mysql_keeps_the_greetings_version_string() {
        for banner in [
            "8.0.36-0ubuntu0.22.04.1",
            "10.11.8-MariaDB-1:10.11.8+maria~ubu2204",
        ] {
            let mut wire = Wire::new(Peer::new(|_sent: &[u8]| {
                panic!("the client answered a server that had only greeted it")
            }));
            wire.inbox.extend_from_slice(&greeting_versioned(
                banner,
                super::CACHING_SHA2_PASSWORD,
                server_capabilities(),
                MariadbCapabilities::empty(),
            ));

            let greeting =
                read_greeting(&mut wire, REQUIRED_CAPABILITIES).expect("a complete greeting");
            assert_eq!(greeting.banner, banner);
            assert!(
                wire.peer().sent.is_empty(),
                "the driver spent a round trip on a version the greeting already carried"
            );
        }
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

        let greeting =
            read_greeting(&mut wire, REQUIRED_CAPABILITIES).expect("the greeting decodes");
        request_tls(&mut wire, &greeting).expect("the upgrade request goes out");
        let refused = authenticate(&mut wire, &login(), &greeting)
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

    /// `rule:core-classes/db-capabilities`'s first closed hole, asserted as a property of the
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

    /// `rule:core-classes/db-column-types`'s zone, as the numeric offset the section requires and
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
                extended: EXTENDED_CAPABILITIES,
                nonce: NONCE.to_vec(),
                plugin: super::CACHING_SHA2_PASSWORD.as_bytes().to_vec(),
                server_version: (8, 0, 36),
                banner: String::from("8.0.36"),
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

    /// `rule:core-classes/db-statement-members`'s `lastId` over a write the server
    /// generated no key for: the status packet says `0`, and that is an absence
    /// rather than a key whose value is zero.
    ///
    /// An `UPDATE` generates no `AUTO_INCREMENT` value and every driver has to
    /// say so in its own way. A driver that carried the protocol's zero through
    /// would hand a caller a number where the statement produced none, and the
    /// member would mean two things on two drivers.
    // covers: Core\Db\Write::lastId
    #[test]
    fn a_write_that_generated_no_key_reports_none_rather_than_zero() {
        const SQL: &str = "UPDATE t SET a = ? WHERE id = 1";
        const VALUE: &[u8] = b"x";

        let mut wire = Wire::new(Peer::new(|sent: &[u8]| match sent.get(4) {
            Some(0x16) => {
                let mut out = packet(1, &prepare_ok(9, 0, 1));
                out.extend_from_slice(&packet(2, &column_def("a")));
                out
            }
            Some(0x17) => packet(1, &ok_packet(1, 0)),
            other => panic!("the driver sent command {other:?}"),
        }));
        let state = Cell::new(State::Idle);

        let rows = start_statement(
            &mut wire,
            &state,
            CLIENT_CAPABILITIES,
            &mut no_cache(),
            SQL,
            &[Some(VALUE)],
        )
        .expect("a prepare and an execution the server answered");

        assert_eq!(rows.affected(), Some(1), "the status packet's own count");
        assert_eq!(
            rows.last_id(),
            None,
            "the status packet's `0` is no generated key at all"
        );
    }

    /// `rule:core-classes/db-one-api`'s two round trips, and its no-emulated-prepares rule
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
                &mut no_cache(),
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
            "`rule:core-classes/db-one-api`: a statement's first execution costs two round trips, \
             and this is where that cost is visible"
        );
        assert!(
            contains(&sent[0], SQL.as_bytes()) && !contains(&sent[0], INJECTION),
            "the prepare carried the statement and must carry no parameter"
        );
        assert!(
            contains(&sent[1], INJECTION) && !contains(&sent[1], SQL.as_bytes()),
            "the execution carried the parameter and must carry no SQL — a value \
             spliced into the statement is what `rule:core-classes/db-one-api` removes"
        );
    }

    /// `rule:core-classes/db-one-api`'s cache, priced in round trips: the second execution of a
    /// statement this connection has already run sends `COM_STMT_EXECUTE` and
    /// nothing else.
    ///
    /// **The count of commands is the assertion**, because a driver that
    /// re-prepared every time answers every row of every case above correctly
    /// and still costs twice what § 1 prices a cached re-execution at. The two
    /// executions also have to name the *same* handle: a cache that kept the
    /// SQL but not the id would send the server a statement it never issued.
    #[test]
    fn a_second_execution_of_a_cached_statement_skips_the_prepare() {
        const SQL: &str = "SELECT a FROM t WHERE b = ?";

        let mut wire = Wire::new(Peer::new(|sent: &[u8]| match sent.get(4) {
            Some(0x16) => {
                let mut out = packet(1, &prepare_ok(9, 0, 1));
                out.extend_from_slice(&packet(2, &column_def("b")));
                out
            }
            Some(0x17) => packet(1, &ok_packet(0, 0)),
            other => panic!("the driver sent command {other:?}"),
        }));
        let state = Cell::new(State::Idle);
        let mut cache = sized_cache(2);

        for _ in 0..2 {
            drop(
                start_statement(
                    &mut wire,
                    &state,
                    CLIENT_CAPABILITIES,
                    &mut cache,
                    SQL,
                    &[Some(b"x")],
                )
                .expect("a statement the server answered"),
            );
        }

        assert_eq!(
            commands(&wire.peer().sent),
            [Some(0x16), Some(0x17), Some(0x17)],
            "`rule:core-classes/db-one-api`: a statement's first execution costs two round trips \
             and a cached re-execution costs one"
        );
        assert_eq!(
            wire.peer().sent[1][5..9],
            wire.peer().sent[2][5..9],
            "both executions name the handle the one prepare answered with"
        );
        assert_eq!(cache.len(), 1);
    }

    /// An eviction deallocates the statement it replaced rather than leaving
    /// the server holding it for the life of the connection.
    ///
    /// `COM_STMT_CLOSE` is the half a cache is easy to write without: the entry
    /// leaves the `Vec` either way and only the server can tell the difference,
    /// which it does by holding one more prepared statement per eviction until
    /// the connection goes away. The handle in the packet is the *evicted* one,
    /// so a driver closing the statement it just prepared passes this case's
    /// count and fails its bytes.
    #[test]
    fn an_eviction_closes_the_statement_it_replaced() {
        let mut next_id = 1_u32;
        let mut wire = Wire::new(Peer::new(move |sent: &[u8]| match sent.get(4) {
            Some(0x16) => {
                let id = next_id;
                next_id += 1;
                packet(1, &prepare_ok(id, 0, 0))
            }
            Some(0x17) => packet(1, &ok_packet(0, 0)),
            // The one command with no answer at all: a server that replied
            // here would have its reply read as the next statement's.
            Some(0x19) => Vec::new(),
            other => panic!("the driver sent command {other:?}"),
        }));
        let state = Cell::new(State::Idle);
        let mut cache = sized_cache(1);

        for sql in ["SELECT a", "SELECT b"] {
            drop(
                start_statement(&mut wire, &state, CLIENT_CAPABILITIES, &mut cache, sql, &[])
                    .expect("a statement the server answered"),
            );
        }

        assert_eq!(
            commands(&wire.peer().sent),
            [Some(0x16), Some(0x17), Some(0x19), Some(0x16), Some(0x17)],
            "the second statement evicted the first, and the close goes out \
             before the prepare that needed the room"
        );
        let closed = &wire.peer().sent[2];
        assert_eq!(
            u32::from_le_bytes(closed[5..9].try_into().expect("a four-byte handle")),
            1,
            "the handle closed is the evicted statement's and not the new one's"
        );
        assert_eq!(cache.len(), 1);
    }

    /// `rule:security/db-pool-reset-is-a-boundary`'s asymmetry: `COM_RESET_CONNECTION` drops the server's
    /// prepared statements, so the reset empties the cache with them.
    ///
    /// This is the one place the two drivers deliberately disagree. § 13 has
    /// PostgreSQL reset with `RESET ALL` rather than `DISCARD ALL` *because*
    /// its cache must survive; MySQL has no reset primitive that keeps one, and
    /// a cache that survived here would bind the next request against statement
    /// ids this session no longer has — a wrong answer rather than a slow one.
    #[test]
    fn a_reset_invalidates_the_statement_cache() {
        const SQL: &str = "SELECT a";

        let mut wire = Wire::new(Peer::new(|sent: &[u8]| match sent.get(4) {
            Some(0x16) => packet(1, &prepare_ok(4, 0, 0)),
            // The execution, the reset, and the `SET time_zone` the reset owes
            // because it cleared the session variable § 9 declared.
            Some(0x17 | 0x1F | 0x03) => packet(1, &ok_packet(0, 0)),
            other => panic!("the driver sent command {other:?}"),
        }));
        let state = Cell::new(State::Idle);
        let mut cache = sized_cache(2);

        drop(
            start_statement(&mut wire, &state, CLIENT_CAPABILITIES, &mut cache, SQL, &[])
                .expect("a statement the server answered"),
        );
        assert_eq!(cache.len(), 1);

        super::reset_session(&mut wire, CLIENT_CAPABILITIES, 0, &mut cache)
            .expect("a reset the server acknowledged");
        assert!(
            cache.is_empty(),
            "§ 13: the reset drops every prepared statement, so a cache that \
             kept one is naming a handle the server does not have"
        );

        drop(
            start_statement(&mut wire, &state, CLIENT_CAPABILITIES, &mut cache, SQL, &[])
                .expect("a statement the server answered"),
        );
        assert_eq!(
            commands(&wire.peer().sent),
            [
                Some(0x16),
                Some(0x17),
                Some(0x1F),
                Some(0x03),
                Some(0x16),
                Some(0x17)
            ],
            "the same statement after a reset costs the prepare again — § 13's \
             asymmetry with PostgreSQL, which is the protocol's"
        );
    }

    /// The command byte of every message the driver flushed, which is how the
    /// cases above assert a round trip count rather than a payload.
    fn commands(sent: &[Vec<u8>]) -> Vec<Option<u8>> {
        sent.iter().map(|message| message.get(4).copied()).collect()
    }

    /// The text of every `COM_QUERY` the driver flushed.
    ///
    /// § 7's cases assert this rather than [`commands`]' bytes, because every
    /// one of these commands is this module's own SQL and *which* one a level
    /// got is the whole question.
    fn text_commands(sent: &[Vec<u8>]) -> Vec<String> {
        sent.iter()
            .filter(|message| message.get(4) == Some(&COM_QUERY))
            .map(|message| String::from_utf8_lossy(&message[5..]).into_owned())
            .collect()
    }

    /// A server that answers every text command with a bare `OK`.
    fn accepting() -> impl FnMut(&[u8]) -> Vec<u8> {
        |sent: &[u8]| match sent.get(4) {
            Some(&COM_QUERY) => packet(1, &ok_packet(0, 0)),
            other => panic!("the driver sent command {other:?}"),
        }
    }

    /// `rule:core-classes/db-transactions` on this protocol: the depth picks the command, and a nested
    /// rollback is one round trip because MySQL's savepoints are not
    /// PostgreSQL's.
    ///
    /// **Each claim here fails differently.** A driver that spelled the
    /// outermost level `BEGIN` would have no way to render `readOnly`. One that
    /// named its savepoints from a counter would leave the server a name per
    /// nested transaction a loop opened. And one that copied `crate::pg`'s
    /// `ROLLBACK TO SAVEPOINT …; RELEASE SAVEPOINT …` would be sending two
    /// statements in a command this driver never negotiated
    /// `CLIENT_MULTI_STATEMENTS` for — a server error at best, and the shape an
    /// injection wants at worst.
    #[test]
    fn the_depth_picks_section_sevens_command_and_a_nested_rollback_releases_nothing() {
        let mut wire = Wire::new(Peer::new(accepting()));
        let state = Cell::new(State::Idle);
        let depth = Cell::new(0);

        begin(&mut wire, &state, CLIENT_CAPABILITIES, &depth, None, false)
            .expect("an outermost transaction the server opened");
        assert_eq!(depth.get(), 1);
        begin(&mut wire, &state, CLIENT_CAPABILITIES, &depth, None, false)
            .expect("a nested transaction the server opened");
        assert_eq!(depth.get(), 2);
        begin(&mut wire, &state, CLIENT_CAPABILITIES, &depth, None, false)
            .expect("a second nesting the server opened");
        roll_back(&mut wire, &state, CLIENT_CAPABILITIES, &depth)
            .expect("a nested rollback the server accepted");
        commit(&mut wire, &state, CLIENT_CAPABILITIES, &depth)
            .expect("a nested commit the server accepted");
        commit(&mut wire, &state, CLIENT_CAPABILITIES, &depth)
            .expect("an outermost commit the server accepted");

        assert_eq!(
            text_commands(&wire.peer().sent),
            [
                "START TRANSACTION",
                "SAVEPOINT nvs_1",
                "SAVEPOINT nvs_2",
                "ROLLBACK TO SAVEPOINT nvs_2",
                "RELEASE SAVEPOINT nvs_1",
                "COMMIT",
            ],
            "§ 7's nesting is a savepoint named by the depth it opened at, and \
             MySQL deletes a same-named one rather than stacking it — so the \
             `RELEASE` PostgreSQL owes after a rollback buys nothing here"
        );
        assert_eq!(depth.get(), 0, "every level this case opened was closed");
        assert_eq!(state.get(), State::Idle, "the connection is poolable");
    }

    /// § 7's isolation level is a command of its own on MySQL, and `Snapshot`
    /// is the one level that collapses onto another.
    ///
    /// The `SET TRANSACTION` has to come *first* and has to be its own round
    /// trip: MySQL takes no isolation option on the statement that opens a
    /// transaction, and the characteristic it sets applies to the next
    /// transaction started on the session. A driver that spelled
    /// `START TRANSACTION ISOLATION LEVEL …` would be refused by every server
    /// it ever met.
    #[test]
    fn an_isolation_level_is_its_own_command_and_snapshot_is_repeatable_read() {
        let mut wire = Wire::new(Peer::new(accepting()));
        let state = Cell::new(State::Idle);
        let depth = Cell::new(0);

        let span = begin(
            &mut wire,
            &state,
            CLIENT_CAPABILITIES,
            &depth,
            Some(Isolation::Serializable),
            true,
        )
        .expect("a serializable read-only transaction the server opened");
        assert_eq!(
            span.sql(),
            "SET TRANSACTION ISOLATION LEVEL SERIALIZABLE; START TRANSACTION READ ONLY",
            "§ 11's one span covers both round trips the caller waited for"
        );

        commit(&mut wire, &state, CLIENT_CAPABILITIES, &depth).expect("a commit");
        begin(
            &mut wire,
            &state,
            CLIENT_CAPABILITIES,
            &depth,
            Some(Isolation::Snapshot),
            false,
        )
        .expect("a snapshot transaction the server opened");

        assert_eq!(
            text_commands(&wire.peer().sent),
            [
                "SET TRANSACTION ISOLATION LEVEL SERIALIZABLE",
                "START TRANSACTION READ ONLY",
                "COMMIT",
                "SET TRANSACTION ISOLATION LEVEL REPEATABLE READ",
                "START TRANSACTION",
            ],
            "the level is armed by its own command, `readOnly` rides on the \
             start and never spells `READ WRITE`, and § 7's `Snapshot` is what \
             InnoDB calls `REPEATABLE READ`"
        );
    }

    /// A nested `transaction()` carrying either option is refused before a
    /// byte, because a savepoint cannot answer for it.
    ///
    /// MySQL settles both characteristics for the whole transaction, so
    /// running the callable at the outer level while its author wrote
    /// `Isolation::Serializable` would give it weaker semantics than the
    /// program asked for, and nothing would say so.
    #[test]
    fn a_nested_transaction_asking_for_an_option_is_refused_before_a_byte() {
        let mut wire = Wire::new(Peer::new(accepting()));
        let state = Cell::new(State::Idle);
        let depth = Cell::new(1);

        let refused = begin(
            &mut wire,
            &state,
            CLIENT_CAPABILITIES,
            &depth,
            Some(Isolation::Serializable),
            false,
        )
        .expect_err("a nested transaction may not ask for its own level");

        assert_eq!(refused.kind(), io::ErrorKind::InvalidInput);
        assert!(
            wire.peer().sent.is_empty(),
            "nothing was written for a call this driver refused itself"
        );
        assert_eq!(depth.get(), 1, "a refused begin opens no level");
    }

    /// An outermost `COMMIT` the *server* refused leaves no transaction open,
    /// and a `START TRANSACTION` refused after its level was armed leaves no
    /// connection.
    ///
    /// Two counts that only look alike. The server rolls the transaction back
    /// before it refuses a commit, so the level is gone and § 7's `{retries: n}`
    /// must get a `START TRANSACTION` on the next attempt rather than a
    /// savepoint against nothing. The armed `SET TRANSACTION`, by contrast,
    /// survives on the *session* with no command able to clear it — so that
    /// connection is closed rather than pooled, which is the only thing
    /// `State::Poisoned` can say.
    #[test]
    fn a_refused_commit_closes_the_level_and_a_refused_start_closes_the_connection() {
        let mut wire = Wire::new(Peer::new(|sent: &[u8]| match sent.get(4) {
            Some(&COM_QUERY) if sent.ends_with(b"COMMIT") => {
                packet(1, &error_packet(1213, "40001", "Deadlock found"))
            }
            Some(&COM_QUERY) => packet(1, &ok_packet(0, 0)),
            other => panic!("the driver sent command {other:?}"),
        }));
        let state = Cell::new(State::Idle);
        let depth = Cell::new(1);

        commit(&mut wire, &state, CLIENT_CAPABILITIES, &depth)
            .expect_err("a commit the server refused");
        assert_eq!(
            depth.get(),
            0,
            "the server rolled the transaction back before refusing, so the \
             level is gone and the next attempt owes a `START TRANSACTION`"
        );
        assert_eq!(
            state.get(),
            State::Idle,
            "a refusal the server worded arrived whole, so the connection is \
             still at a packet boundary"
        );

        let mut wire = Wire::new(Peer::new(|sent: &[u8]| match sent.get(4) {
            Some(&COM_QUERY) if sent.ends_with(b"START TRANSACTION") => {
                packet(1, &error_packet(1568, "25001", "cannot start"))
            }
            Some(&COM_QUERY) => packet(1, &ok_packet(0, 0)),
            other => panic!("the driver sent command {other:?}"),
        }));
        let state = Cell::new(State::Idle);
        let depth = Cell::new(0);

        begin(
            &mut wire,
            &state,
            CLIENT_CAPABILITIES,
            &depth,
            Some(Isolation::ReadUncommitted),
            false,
        )
        .expect_err("a start the server refused");
        assert_eq!(depth.get(), 0, "no level was opened");
        assert!(
            !state.get().is_poolable(),
            "the session is holding an isolation level nothing can clear, so \
             this connection is closed rather than handed to another request"
        );
    }

    /// The other half of that first bound: a `COMMIT` that failed with no
    /// refusal from the server leaves the level open.
    ///
    /// Nothing came back to say the transaction was rolled back, so its fate is
    /// unknown and the depth stays where it was — while the connection, whose
    /// next packet nobody can name, is poisoned rather than pooled. The pair is
    /// worth more than either half: [`commit`] tells the two apart by asking
    /// [`ServerError::of`], and a version that read "the server refused it" off
    /// anything coarser would pass the case above and drop this level on the
    /// floor.
    #[test]
    fn a_commit_that_failed_on_the_wire_leaves_the_level_open() {
        let mut wire = Wire::new(Peer::new(|_sent: &[u8]| Vec::new()));
        let state = Cell::new(State::Idle);
        let depth = Cell::new(1);

        let refused = commit(&mut wire, &state, CLIENT_CAPABILITIES, &depth)
            .expect_err("a commit the server never answered");

        assert!(
            ServerError::of(&refused).is_none(),
            "a wire failure is nobody's refusal: {refused}"
        );
        assert_eq!(
            depth.get(),
            1,
            "the level is still open until something says otherwise"
        );
        assert!(
            !state.get().is_poolable(),
            "and the connection is closed rather than handed to another request"
        );
    }

    /// A result set that a server answers three definitions and one row of,
    /// read to the terminator that gives the connection back.
    ///
    /// **The claims that meet in the one row each fail differently.** The
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
                &mut no_cache(),
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
            assert_eq!(rows.last_id(), None, "a `SELECT` generated no key");
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

        let mut rows = start_statement(
            &mut wire,
            &state,
            CLIENT_CAPABILITIES,
            &mut no_cache(),
            "SELECT n",
            &[],
        )
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

    /// A `[db.<name>]` block with every field a MySQL connection reads, as an
    /// operator writes it and as `nvs_config` hands it over.
    fn block() -> Database {
        Database {
            driver: Some("mysql".to_owned()),
            host: Some("mysql.test".to_owned()),
            user: Some("novis".to_owned()),
            password: Some("hunter2".to_owned()),
            database: Some("novis_test".to_owned()),
            time_zone: Some("+02:00".to_owned()),
            ..Database::default()
        }
    }

    /// A complete block resolves to exactly what the handshake sends, and the
    /// two fields with readers of their own are asserted through them: § 9's
    /// zone arrives as the seconds `set_session_time_zone` renders rather than
    /// as the text an operator wrote, and an unwritten `statement_cache` is
    /// § 1's default where a written `0` is the cache off.
    #[test]
    fn a_complete_block_resolves_to_the_target_the_handshake_sends() {
        let mut block = block();
        let target = MySqlTarget::resolve(&block).expect("a complete block resolves");
        assert_eq!(target.host, "mysql.test");
        assert_eq!(target.user, "novis");
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
            MySqlTarget::resolve(&block)
                .expect("a block that turns the cache off resolves")
                .statement_cache,
            0,
            "a written `0` is § 1's cache turned off, which is the answer an \
             `unwrap_or_default` reader loses"
        );
        block.statement_cache = None;

        block.time_zone = None;
        assert_eq!(
            MySqlTarget::resolve(&block)
                .expect("an unwritten zone is UTC")
                .time_zone,
            0
        );
    }

    /// `rule:core-classes/db-connection-is-named`'s discriminant, from this side of it — and **MariaDB is
    /// the case worth the test**.
    ///
    /// The two share a wire protocol and a codec crate, so opening a MariaDB
    /// block with this resolver would work often enough to look right; ADR
    /// 0067 refuses that at length, because the two have diverged in auth
    /// plugins, error tables and bulk protocol and a connection that silently
    /// crossed over would answer the wrong table for every error it reported.
    /// A resolver that matched on the shared protocol rather than on the
    /// written `driver` passes every other case here.
    #[test]
    fn a_block_that_is_not_mysqls_is_refused_and_mariadb_is_a_driver_and_not_a_flag() {
        let mut block = block();
        block.driver = None;
        assert_eq!(
            MySqlTarget::resolve(&block).unwrap_err(),
            BlockError::NoDriver
        );

        for (written, driver) in [("mariadb", Driver::MariaDb), ("postgres", Driver::Postgres)] {
            block.driver = Some(written.to_owned());
            assert_eq!(
                MySqlTarget::resolve(&block).unwrap_err(),
                BlockError::OtherDriver {
                    written,
                    driver,
                    expected: Driver::MySql
                }
            );
        }

        block.driver = Some("mysqli".to_owned());
        assert_eq!(
            MySqlTarget::resolve(&block).unwrap_err(),
            BlockError::UnknownDriver { written: "mysqli" }
        );

        // A file is written by a human, so the capitals are a spelling and not
        // another backend.
        block.driver = Some("MySQL".to_owned());
        assert!(MySqlTarget::resolve(&block).is_ok());

        // SQLite's field, and the one a silent resolver loses: ignoring it
        // would open a *server* connection for a block that named a file.
        block.path = Some("app.db".to_owned());
        assert_eq!(
            MySqlTarget::resolve(&block).unwrap_err(),
            BlockError::Unusable {
                field: "path",
                expected: Driver::MySql
            }
        );
    }

    /// The two resolvers refuse one broken block the same way, and each names
    /// its own backend when it does.
    ///
    /// § 2 makes a `[db.<name>]` block one file format with a discriminant, so
    /// a fault an operator can write is a fault under every driver — and the
    /// thing a per-driver refusal quietly grows is a second vocabulary for the
    /// same file. Asserted as agreement rather than as two rosters: each
    /// resolver is handed its own driver's block with the same field broken,
    /// and the *variant* must match while the sentence names PostgreSQL on one
    /// side and MySQL on the other.
    #[test]
    fn the_two_resolvers_refuse_one_block_the_same_way_and_name_their_own_backend() {
        type Break = (&'static str, fn(&mut Database));
        let faults: [Break; 5] = [
            ("host absent", |block| block.host = None),
            ("user blank", |block| block.user = Some("  ".to_owned())),
            ("password absent", |block| block.password = None),
            ("secret unread", |block| {
                block.password = None;
                block.password_file = Some("/run/secrets/db".to_owned());
            }),
            ("zone is a name", |block| {
                block.time_zone = Some("Europe/Vienna".to_owned());
            }),
        ];

        let mut swept = 0;
        for (label, apply) in faults {
            let mut mine = block();
            apply(&mut mine);
            let mut theirs = block();
            theirs.driver = Some("postgres".to_owned());
            apply(&mut theirs);

            let refused = MySqlTarget::resolve(&mine).unwrap_err();
            let other = crate::PgTarget::resolve(&theirs).unwrap_err();
            assert_eq!(
                std::mem::discriminant(&refused),
                std::mem::discriminant(&other),
                "{label} is one fault of the file format and not one driver's"
            );
            swept += 1;
        }
        assert_eq!(swept, faults.len());

        // The one thing that does differ, on both sides of it: the sentence an
        // operator reads names the backend whose resolver read the block.
        let mut absent = block();
        absent.host = None;
        let mine = MySqlTarget::resolve(&absent).unwrap_err().refusal("main");
        assert_eq!(
            mine,
            "[db.main]: the block names no `host`, which a MySQL connection cannot be opened \
             without"
        );

        absent.driver = Some("postgres".to_owned());
        let theirs = crate::PgTarget::resolve(&absent)
            .unwrap_err()
            .refusal("main");
        assert!(theirs.contains("a PostgreSQL connection"), "{theirs}");
    }

    /// `rule:core-classes/db-unix-socket-path`: MySQL takes the socket **file**,
    /// so the path an operator wrote in `[db.<name>] host` is the path this
    /// driver opens, with nothing derived from it — which is the whole of the
    /// difference from `crate::pg`'s directory.
    ///
    /// Asserted against **where the dial landed** rather than against a
    /// completed handshake: a unit test has no MySQL server, and what is under
    /// test is the path, so a listener that accepts once and hangs up answers
    /// that question and nothing else. A driver that derived a name would find
    /// nothing bound at it and the accept would never happen.
    #[test]
    fn a_mysql_socket_path_is_opened_as_written() {
        #[cfg(unix)]
        {
            let (_dir, path) = nvs_repo::socket("mysqld.sock");
            let listener =
                std::os::unix::net::UnixListener::bind(&path).expect("the OS refused the path");
            let server = std::thread::spawn(move || listener.accept().is_ok());

            let opened = crate::conn::MySqlConn::connect(
                crate::conn::Endpoint::Socket(path.clone()),
                &target(),
                None,
            );

            assert!(
                server.join().expect("the fake server runs to completion"),
                "the driver did not dial the path an operator wrote"
            );
            assert!(
                opened.is_err(),
                "a server that hung up before greeting is not a connection"
            );
        }

        #[cfg(not(unix))]
        {
            let refused = crate::conn::socket_endpoint("/run/mysqld/mysqld.sock")
                .expect_err("a build with no `AF_UNIX` transport has no socket to open");
            assert_eq!(refused.kind(), io::ErrorKind::Unsupported);
        }
    }

    /// [`MyStream`]: the transport is settled before the greeting is read, so
    /// what a *server* said is answered the same way over either of them.
    ///
    /// The refusal a server sends **instead of** a greeting is the one exchange
    /// a plaintext script can drive over both arms: [`read_greeting`] answers an
    /// `0xFF` packet before it compares capabilities and before the in-band
    /// upgrade, so the TCP arm reaches it with no certificate anywhere. The two
    /// arms do not ask for the same capabilities — TLS is in one set and not the
    /// other — and this is the case that says the difference stops there.
    ///
    /// Asserted as agreement, over every driver framed on [`Wire`], and
    /// **counted** rather than read off one line, so a driver whose socket arm
    /// grew an error path of its own fails here while its own case still
    /// passes. [`crate::pg`] is outside the sweep by design: over TCP its
    /// refusal arrives inside TLS, which is `PgStream`'s whole argument, and
    /// TDS has no socket arm to disagree over.
    #[test]
    fn a_driver_answers_the_same_over_either_transport() {
        #[cfg(unix)]
        {
            use std::io::Write as _;

            /// MariaDB's target beside [`target`]'s, so the sweep asks each
            /// driver its own question with its own type.
            fn maria_target() -> crate::MariaTarget<'static> {
                crate::MariaTarget {
                    host: "db.example.internal",
                    user: "novis",
                    password: PASSWORD,
                    database: "shop",
                    tls_ca_file: None,
                    time_zone: 0,
                    statement_cache: crate::sql::DEFAULT_STATEMENT_CACHE,
                }
            }

            // The layout a refusal takes where a greeting was due: the
            // capabilities are not negotiated yet, so there is no `SQLSTATE` in
            // it and `read_greeting` reads it under an empty set.
            let mut body = vec![0xFF];
            body.extend_from_slice(&1045u16.to_le_bytes());
            body.extend_from_slice(b"Access denied for user 'novis'");
            let refusal = packet(0, &body);

            type Dial = fn(crate::conn::Endpoint) -> io::Error;
            let drivers: [(&str, Dial); 2] = [
                ("mysql", |endpoint| {
                    crate::MySqlConn::connect(endpoint, &target(), None)
                        .expect_err("a server that refused before greeting is not a connection")
                }),
                ("mariadb", |endpoint| {
                    crate::MariaConn::connect(endpoint, &maria_target(), None)
                        .expect_err("a server that refused before greeting is not a connection")
                }),
            ];

            let mut agreed = 0;
            for (driver, dial) in drivers {
                let listening = std::net::TcpListener::bind("127.0.0.1:0")
                    .expect("the OS refused a loopback port");
                let address = listening
                    .local_addr()
                    .expect("a bound listener has an address");
                let script = refusal.clone();
                let server = std::thread::spawn(move || {
                    let (mut accepted, _) = listening.accept().expect("the driver dials");
                    let _ = accepted.write_all(&script);
                });
                let over_tcp = dial(crate::conn::Endpoint::Tcp(address));
                server.join().expect("the fake server runs to completion");

                let (_dir, path) = nvs_repo::socket(&format!("{driver}.sock"));
                let listening =
                    std::os::unix::net::UnixListener::bind(&path).expect("the OS refused the path");
                let script = refusal.clone();
                let server = std::thread::spawn(move || {
                    let (mut accepted, _) = listening.accept().expect("the driver dials");
                    let _ = accepted.write_all(&script);
                });
                let over_socket = dial(crate::conn::Endpoint::Socket(path.clone()));
                server.join().expect("the fake server runs to completion");

                assert_eq!(
                    over_tcp.kind(),
                    over_socket.kind(),
                    "{driver} read one refusal as two kinds"
                );
                assert_eq!(
                    over_tcp.to_string(),
                    over_socket.to_string(),
                    "{driver} worded one refusal two ways"
                );
                let said_over_tcp = ServerError::of(&over_tcp).expect("the server worded this one");
                let said_over_socket =
                    ServerError::of(&over_socket).expect("the server worded this one");
                assert_eq!(
                    (
                        said_over_tcp.kind,
                        said_over_tcp.driver_code,
                        said_over_tcp.backend
                    ),
                    (
                        said_over_socket.kind,
                        said_over_socket.driver_code,
                        said_over_socket.backend
                    ),
                    "{driver} normalised one refusal two ways"
                );
                agreed += 1;
            }

            assert_eq!(
                agreed, 2,
                "both drivers framed on `Wire` answer over either transport"
            );
        }

        #[cfg(not(unix))]
        {
            // One transport, and the drivers agree about that too: a build with
            // no `AF_UNIX` transport refuses a socket `host` rather than
            // reading it as loopback, and both spellings of `socket_endpoint`
            // reach `crate::conn`'s one refusal to do it.
            for refused in [
                super::socket_endpoint("/run/mysqld/mysqld.sock", super::DEFAULT_PORT),
                crate::pg::socket_endpoint("/run/postgresql", 5432),
            ] {
                assert_eq!(
                    refused
                        .expect_err("a build with no `AF_UNIX` transport has no socket to open")
                        .kind(),
                    io::ErrorKind::Unsupported
                );
            }
        }
    }

    /// A `Column` as the driver reads one, out of the definition packet
    /// [`typed_column_def`] writes and through the same `deserialize`
    /// [`super::read_columns`] uses.
    fn column_of(
        name: &str,
        ty: ColumnType,
        flags: ColumnFlags,
        charset: u16,
        length: u32,
    ) -> Column {
        let body = typed_column_def(name, ty, flags, charset, length);
        Column::deserialize((), &mut ParseBuf(&body)).expect("a definition just written here")
    }

    /// `rule:core-classes/db-column-types`'s type map, both halves at once: every column this driver
    /// can describe, decoded, and the row the value landed on asserted
    /// **against the row the description named**.
    ///
    /// The pairing is the point. `column_type` and `scalar` are two functions
    /// reading two different things — a definition packet and a value — and a
    /// driver where they disagree answers a `Core\Db\Column::type` that no
    /// column of that name ever yields. Asserted as one table rather than a
    /// line each so that a row added to § 9 with no arm here fails the count
    /// as well as the comparison.
    ///
    /// The type codes that are two rows apiece are all here: `BIGINT` and
    /// `BIGINT UNSIGNED`, `ENUM` and `SET`, `BIT(1)` and `BIT(8)`. So is the
    /// one place this driver reads § 9's last row as `bytes` rather than as
    /// `tainted string` — a binary-charset column with no Novis type, whose
    /// octets are not text in any encoding — which `super::scalar`'s own
    /// comment owns.
    #[test]
    fn every_column_of_9s_type_map_decodes_to_the_row_its_description_names() {
        let binary = super::BINARY_CHARSET;
        let utf8 = u16::from(COLLATION);
        let midday = MyValue::Date(2026, 9, 2, 12, 30, 15, 250_000);
        let table: Vec<(&str, Column, MyValue)> = vec![
            (
                "int",
                column_of(
                    "i",
                    ColumnType::MYSQL_TYPE_LONG,
                    ColumnFlags::empty(),
                    binary,
                    11,
                ),
                MyValue::Int(-7),
            ),
            (
                "year",
                column_of(
                    "y",
                    ColumnType::MYSQL_TYPE_YEAR,
                    ColumnFlags::empty(),
                    binary,
                    4,
                ),
                MyValue::Int(2026),
            ),
            (
                "bigint unsigned",
                column_of(
                    "u",
                    ColumnType::MYSQL_TYPE_LONGLONG,
                    ColumnFlags::UNSIGNED_FLAG,
                    binary,
                    20,
                ),
                MyValue::UInt(u64::MAX),
            ),
            (
                "float",
                column_of(
                    "f",
                    ColumnType::MYSQL_TYPE_FLOAT,
                    ColumnFlags::empty(),
                    binary,
                    12,
                ),
                MyValue::Float(0.5),
            ),
            (
                "double",
                column_of(
                    "d",
                    ColumnType::MYSQL_TYPE_DOUBLE,
                    ColumnFlags::empty(),
                    binary,
                    22,
                ),
                MyValue::Double(0.5),
            ),
            (
                "decimal",
                column_of(
                    "m",
                    ColumnType::MYSQL_TYPE_NEWDECIMAL,
                    ColumnFlags::empty(),
                    binary,
                    12,
                ),
                MyValue::Bytes(b"12.34".to_vec()),
            ),
            (
                "bit(1)",
                column_of(
                    "b",
                    ColumnType::MYSQL_TYPE_BIT,
                    ColumnFlags::empty(),
                    binary,
                    1,
                ),
                MyValue::Bytes(vec![0x01]),
            ),
            (
                "varchar",
                column_of(
                    "s",
                    ColumnType::MYSQL_TYPE_VAR_STRING,
                    ColumnFlags::empty(),
                    utf8,
                    255,
                ),
                MyValue::Bytes(b"ada".to_vec()),
            ),
            (
                "enum",
                column_of(
                    "e",
                    ColumnType::MYSQL_TYPE_STRING,
                    ColumnFlags::ENUM_FLAG,
                    utf8,
                    16,
                ),
                MyValue::Bytes(b"green".to_vec()),
            ),
            (
                "set",
                column_of(
                    "t",
                    ColumnType::MYSQL_TYPE_STRING,
                    ColumnFlags::SET_FLAG,
                    utf8,
                    16,
                ),
                MyValue::Bytes(b"red,green".to_vec()),
            ),
            (
                "blob",
                column_of(
                    "o",
                    ColumnType::MYSQL_TYPE_BLOB,
                    ColumnFlags::empty(),
                    binary,
                    65535,
                ),
                MyValue::Bytes(vec![0xFF, 0x00]),
            ),
            (
                "bit(8)",
                column_of(
                    "w",
                    ColumnType::MYSQL_TYPE_BIT,
                    ColumnFlags::empty(),
                    binary,
                    8,
                ),
                MyValue::Bytes(vec![0xFF]),
            ),
            (
                "json",
                column_of(
                    "j",
                    ColumnType::MYSQL_TYPE_JSON,
                    ColumnFlags::empty(),
                    binary,
                    4096,
                ),
                MyValue::Bytes(br#"{"a":1}"#.to_vec()),
            ),
            (
                "date",
                column_of(
                    "da",
                    ColumnType::MYSQL_TYPE_DATE,
                    ColumnFlags::empty(),
                    binary,
                    10,
                ),
                MyValue::Date(2026, 9, 2, 0, 0, 0, 0),
            ),
            (
                "datetime",
                column_of(
                    "dt",
                    ColumnType::MYSQL_TYPE_DATETIME,
                    ColumnFlags::empty(),
                    binary,
                    19,
                ),
                midday.clone(),
            ),
            (
                "timestamp",
                column_of(
                    "ts",
                    ColumnType::MYSQL_TYPE_TIMESTAMP,
                    ColumnFlags::empty(),
                    binary,
                    19,
                ),
                midday,
            ),
            (
                "time",
                column_of(
                    "ti",
                    ColumnType::MYSQL_TYPE_TIME,
                    ColumnFlags::empty(),
                    binary,
                    10,
                ),
                MyValue::Time(false, 0, 23, 59, 59, 999_999),
            ),
            (
                "null",
                column_of(
                    "n",
                    ColumnType::MYSQL_TYPE_LONG,
                    ColumnFlags::empty(),
                    binary,
                    11,
                ),
                MyValue::NULL,
            ),
        ];

        let read: Vec<(&str, NovisType, String)> = table
            .iter()
            .map(|(label, column, value)| {
                let scalar = scalar(column, value).expect("a value its column describes");
                (*label, column_type(column), format!("{scalar:?}"))
            })
            .collect();

        assert_eq!(
            read,
            [
                ("int", NovisType::Int, "int".to_owned()),
                ("year", NovisType::Int, "int".to_owned()),
                ("bigint unsigned", NovisType::Uint, "uint".to_owned()),
                ("float", NovisType::Float, "float".to_owned()),
                ("double", NovisType::Float, "float".to_owned()),
                ("decimal", NovisType::Decimal, "decimal".to_owned()),
                ("bit(1)", NovisType::Bool, "bool".to_owned()),
                ("varchar", NovisType::Text, "string".to_owned()),
                ("enum", NovisType::Text, "string".to_owned()),
                ("set", NovisType::Other, "string".to_owned()),
                ("blob", NovisType::Bytes, "bytes".to_owned()),
                ("bit(8)", NovisType::Other, "bytes".to_owned()),
                ("json", NovisType::Json, "string".to_owned()),
                ("date", NovisType::Date, "date".to_owned()),
                ("datetime", NovisType::DateTime, "datetime".to_owned()),
                ("timestamp", NovisType::DateTime, "datetime".to_owned()),
                ("time", NovisType::Time, "time".to_owned()),
                // Every column type may be null, and the description is still
                // the column's: § 9's `?T` is a value's fact and not a
                // column's.
                ("null", NovisType::Int, "null".to_owned()),
            ]
        );

        // The rows § 9 gives MySQL no column type for, asserted as an
        // absence over the sweep rather than trusted from reading the arms:
        // `TIMESTAMPTZ` is PostgreSQL's and `UUID` is MariaDB's, and MariaDB
        // is its own driver.
        assert!(
            read.iter()
                .all(|(_, described, _)| *described != NovisType::Instant
                    && *described != NovisType::Uuid),
            "no MySQL column describes as an instant or a uuid"
        );
    }

    /// § 9's structured rows answer no [`Value`], and every other row does.
    ///
    /// They are class instances `nvs-stdlib` builds — this crate cannot
    /// allocate one — so a `None` here is what routes a column to that crate
    /// rather than a failure. Asked of the scalar rows that mint nothing:
    /// a `string` or a `bytes` would allocate an `NvsStr` with a reference
    /// this crate has no way to give back, `Value::release` being `unsafe` and
    /// forbidden here.
    ///
    /// [`Value`]: nvs_runtime::Value
    #[test]
    fn the_rows_that_are_class_instances_are_the_rows_with_no_value() {
        let binary = super::BINARY_CHARSET;
        let structured = [
            (
                column_of(
                    "da",
                    ColumnType::MYSQL_TYPE_DATE,
                    ColumnFlags::empty(),
                    binary,
                    10,
                ),
                MyValue::Date(2026, 9, 2, 0, 0, 0, 0),
            ),
            (
                column_of(
                    "ti",
                    ColumnType::MYSQL_TYPE_TIME,
                    ColumnFlags::empty(),
                    binary,
                    10,
                ),
                MyValue::Time(false, 0, 1, 2, 3, 0),
            ),
            (
                column_of(
                    "dt",
                    ColumnType::MYSQL_TYPE_DATETIME,
                    ColumnFlags::empty(),
                    binary,
                    19,
                ),
                MyValue::Date(2026, 9, 2, 12, 30, 15, 0),
            ),
        ];
        for (column, value) in &structured {
            let scalar = scalar(column, value).expect("a value its column describes");
            assert!(
                scalar.into_value().is_none(),
                "a class instance is `nvs-stdlib`'s to build"
            );
        }

        let int = column_of(
            "i",
            ColumnType::MYSQL_TYPE_LONG,
            ColumnFlags::empty(),
            binary,
            11,
        );
        for value in [MyValue::Int(7), MyValue::NULL] {
            assert!(
                scalar(&int, &value)
                    .expect("a value its column describes")
                    .into_value()
                    .is_some(),
                "a scalar row is a value this crate mints itself"
            );
        }
    }

    /// A bound parameter renders as *this* server reads it, and the rows where
    /// that differs from PostgreSQL are asserted against that driver's own
    /// answer rather than on their own.
    ///
    /// Each of them is a value `crate::encode` renders plausibly and this
    /// server would store wrongly without failing: `t` casts to `0`, `\x61` is
    /// four characters in a `BLOB`, and `Infinity` is `0` in a `DOUBLE`. So the
    /// assertion is that the two encoders **disagree** here — an agreement is
    /// the bug, and one written by copying the other passes every test that
    /// only reads this one's output.
    #[test]
    fn a_bound_parameter_renders_as_mysql_reads_it_and_never_as_postgresqls_text() {
        let sent = [
            (Value::bool(true), b"1".to_vec()),
            (Value::bool(false), b"0".to_vec()),
            (Value::int(-7), b"-7".to_vec()),
            (Value::uint(u64::MAX), u64::MAX.to_string().into_bytes()),
            (Value::float(1.5), b"1.5".to_vec()),
            (
                Value::bytes(NvsStr::new(&[0x00, 0x61, 0xFF])),
                vec![0x00, 0x61, 0xFF],
            ),
            (
                Value::str(NvsStr::new("é".as_bytes())),
                "é".as_bytes().to_vec(),
            ),
        ];
        for (value, octets) in sent {
            assert_eq!(
                encode(value).expect("a value § 9 binds"),
                Some(octets),
                "bound as MySQL reads it"
            );
        }

        for divergent in [Value::bool(true), Value::bytes(NvsStr::new(&[0x61]))] {
            assert_ne!(
                encode(divergent).expect("a value § 9 binds"),
                crate::encode(divergent).expect("the other driver binds it too"),
                "the two drivers render this row differently, and this is what says so"
            );
        }

        // A bitmap bit and not a rendering, on both drivers.
        assert_eq!(encode(Value::null()).expect("SQL NULL"), None);

        // The three values PostgreSQL binds and this driver refuses.
        for outside in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
            let refused = encode(Value::float(outside)).expect_err("no `DOUBLE` holds it");
            assert_eq!(refused.kind(), io::ErrorKind::InvalidInput);
            assert!(crate::encode(Value::float(outside)).is_ok());
        }
    }

    /// § 9's `TIME` row is a `Core\Time\TimeOfDay`, and MySQL's `TIME` is not:
    /// the bound is asserted on both sides of itself.
    ///
    /// The column doubles as an interval type — `-838:59:59` to `838:59:59` —
    /// so `23:59:59` is the last value that is a time of day and `24:00:00`,
    /// which arrives as one whole day and a zero hour, is the first that is
    /// not. Either alone reads plausibly: a driver that dropped the day count
    /// answers midnight for the second and passes any case that only asks
    /// about the first.
    ///
    /// The refusal's message is asserted too, and what it must *not* carry is
    /// the value: an error that reaches a log line takes the column and its
    /// type with it and leaves one request's data behind.
    #[test]
    fn a_time_outside_a_day_is_refused_and_the_message_carries_no_value() {
        let column = column_of(
            "shift",
            ColumnType::MYSQL_TYPE_TIME,
            ColumnFlags::empty(),
            super::BINARY_CHARSET,
            10,
        );

        let last = scalar(&column, &MyValue::Time(false, 0, 23, 59, 59, 999_999))
            .expect("the last value that is a time of day");
        assert_eq!(format!("{last:?}"), "time");

        for outside in [
            MyValue::Time(false, 1, 0, 0, 0, 0),
            MyValue::Time(true, 0, 1, 0, 0, 0),
        ] {
            let refused = scalar(&column, &outside).expect_err("a value with no time of day in it");
            assert_eq!(refused.kind(), io::ErrorKind::InvalidData);
            let message = refused.to_string();
            assert!(
                message.contains("shift") && message.contains("a time of day"),
                "the message names the column and what it wanted: {message}"
            );
        }
    }

    /// `rule:core-classes/db-statement-members`'s one-statement-at-a-time rule, on the state alone.
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

        let refused = start_statement(
            &mut wire,
            &state,
            CLIENT_CAPABILITIES,
            &mut no_cache(),
            "SELECT 1",
            &[],
        )
        .expect_err("a second statement over an unread result set — `rule:core-classes/db-statement-members`");

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
            start_statement(
                &mut wire,
                &state,
                CLIENT_CAPABILITIES,
                &mut no_cache(),
                "SELECT v",
                &[],
            )
            .expect("a result set the server described"),
        );

        assert_eq!(
            state.get(),
            State::Idle,
            "two unread rows and a terminator were drained, so the connection is \
             reusable rather than destroyed"
        );
    }

    /// A server holding one single-column result set of two rows, answering the
    /// prepare and the execute the same way however often either is asked for.
    ///
    /// Shared by the parked-walk cases below, one of which runs two statements
    /// over it: what those assert is the sequencing, and a script that answered
    /// the second statement differently would be asserting itself.
    fn two_rows() -> impl FnMut(&[u8]) -> Vec<u8> {
        |sent: &[u8]| match sent.get(4) {
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
        }
    }

    /// The two rows [`two_rows`] holds, as a walk over it hands them back.
    fn walked() -> Vec<Option<MyValue>> {
        vec![
            Some(MyValue::Bytes(b"a".to_vec())),
            Some(MyValue::Bytes(b"b".to_vec())),
        ]
    }

    /// The parked form of a walk, which is the whole point of splitting the read
    /// state off the borrow: the cursor is a local of its own, and every row is
    /// read through a **fresh** borrow of the wire and the state.
    ///
    /// The compiler is half the assertion, as it is in `crate::pg`'s
    /// `a_parked_cursor_reads_every_row_through_a_fresh_borrow`. A [`MySqlRows`]
    /// cannot express this shape at all — its borrow would have to span the calls
    /// between the rows — and that is exactly what a `Core\Db\Connection` holding
    /// a walk across `advance()` calls needs, because the connection goes back to
    /// the request in between. The other half is the run: one row per step, in
    /// order, the connection busy for as long as rows remain, and the terminator
    /// leaving it idle and poolable.
    #[test]
    fn mysql_stream_parks_its_read_and_answers_one_row_per_step() {
        let mut wire = Wire::new(Peer::new(two_rows()));
        let state = Cell::new(State::Idle);

        let mut reading = open_result(
            &mut wire,
            &state,
            CLIENT_CAPABILITIES,
            &mut no_cache(),
            "SELECT v",
            &[],
        )
        .expect("a result set the server described");

        assert_eq!(state.get(), State::Streaming);
        assert_eq!(reading.columns().len(), 1);

        let mut seen = Vec::new();
        while let Some(row) =
            next_row_of(&mut wire, &state, &mut reading).expect("the walk advanced")
        {
            // Read a step at a time and not drained into a buffer: the rows
            // still to come are still on the wire, which is what
            // `rule:core-classes/db-streaming`'s constant memory means and why the connection is
            // busy between the steps.
            assert_eq!(state.get(), State::Streaming);
            seen.push(row.value(0).cloned());
        }

        assert_eq!(seen, walked());
        assert_eq!(
            state.get(),
            State::Idle,
            "a drained walk left the connection unpoolable"
        );
        assert!(reading.ended);
        assert_eq!(reading.affected, 2);
    }

    /// MariaDB walks through the *same* parked read, which is this module's half
    /// of `rule:core-classes/db-streaming`'s "both members answer on all five drivers".
    ///
    /// The wire is MariaDB's — [`MARIADB`] is the table a refusal off it would be
    /// worded from — and every byte of the row loop is [`next_row_of`]. A second
    /// copy of that loop for the second server is what this case exists to keep
    /// from being written: the two drivers differ in their rosters and their
    /// error tables, and `rule:core-classes/db-one-api` is explicit that the framing is not one
    /// of the places they are allowed to.
    #[test]
    fn mariadb_stream_answers_through_the_mysql_parked_read() {
        let mut wire = Wire::on(&MARIADB, Peer::new(two_rows()));
        let state = Cell::new(State::Idle);

        let mut reading = open_result(
            &mut wire,
            &state,
            CLIENT_CAPABILITIES,
            &mut no_cache(),
            "SELECT v",
            &[],
        )
        .expect("a result set the server described");

        assert_eq!(wire.backend.name, "mariadb");
        assert_eq!(state.get(), State::Streaming);

        let mut seen = Vec::new();
        while let Some(row) =
            next_row_of(&mut wire, &state, &mut reading).expect("the walk advanced")
        {
            seen.push(row.value(0).cloned());
        }

        assert_eq!(seen, walked());
        assert_eq!(state.get(), State::Idle);
    }

    /// `rule:core-classes/db-streaming`'s second statement: refused for as long as the walk is
    /// open, and taken the moment it is not.
    ///
    /// The refusal is the state's rather than a lifetime's, and that is the whole
    /// difference the parked read makes — [`MySqlRows`] made this case unwritable,
    /// and a program holding a `Core\Db\Stream` across two calls can write it.
    /// Both halves are asserted: a connection that refused forever would pass the
    /// first one and would strand every pooled connection that ever streamed.
    #[test]
    fn a_statement_on_a_streaming_mysql_connection_is_refused_until_the_walk_ends() {
        let mut wire = Wire::new(Peer::new(two_rows()));
        let state = Cell::new(State::Idle);
        let mut cache = no_cache();

        let mut reading = open_result(
            &mut wire,
            &state,
            CLIENT_CAPABILITIES,
            &mut cache,
            "SELECT v",
            &[],
        )
        .expect("a result set the server described");
        let sent = wire.peer().sent.len();

        let refused = start_statement(
            &mut wire,
            &state,
            CLIENT_CAPABILITIES,
            &mut cache,
            "SELECT v",
            &[],
        )
        .expect_err("a second statement over an open walk — `rule:core-classes/db-streaming`");
        assert_eq!(refused.kind(), io::ErrorKind::InvalidInput);
        assert!(
            refused.to_string().contains("one at a time"),
            "the refusal is the one sentence `crate::pg` words for every driver: {refused}"
        );
        assert_eq!(
            wire.peer().sent.len(),
            sent,
            "a refused statement still put a command on a busy wire"
        );

        while next_row_of(&mut wire, &state, &mut reading)
            .expect("the walk advanced")
            .is_some()
        {}
        assert_eq!(state.get(), State::Idle);

        let taken = start_statement(
            &mut wire,
            &state,
            CLIENT_CAPABILITIES,
            &mut cache,
            "SELECT v",
            &[],
        )
        .expect("the walk ended, so the connection takes a statement again");
        assert_eq!(taken.columns().len(), 1);
    }

    /// § 11's span over this driver, and the property that makes it exportable
    /// at all: the bound value goes out on the wire and appears in no field of
    /// the span, in neither of its renderings, and in nothing a field added
    /// later could carry, since the `Debug` it is asserted over is derived.
    ///
    /// `crate::pg`'s `a_query_span_contains_no_parameter_value_anywhere` asks
    /// the same question of the other driver, and the two are deliberately
    /// separate cases: what survives in the text is this driver's **own**
    /// placeholder — a `?` where PostgreSQL keeps `$1` — so one case over both
    /// would have to stop asserting the thing § 5's rewrite decides.
    ///
    /// The positive half is asserted first and is not decoration: "contains no
    /// parameter value" is trivially true of a span that carries nothing, so
    /// the driver, the row count, the affected count and the SQL are named
    /// before the absence is.
    #[test]
    fn a_mysql_query_span_contains_no_parameter_value_anywhere() {
        const BOUND: &str = "correct-horse-battery-staple";
        const SQL: &str = "SELECT greeting FROM greetings WHERE token = ?";

        let mut wire = Wire::new(Peer::new(|sent: &[u8]| match sent.get(4) {
            Some(0x16) => {
                let mut out = packet(1, &prepare_ok(9, 1, 1));
                // One parameter definition, then one column definition: a
                // prepare describes both halves and the driver reads past all
                // of them or starts its execution mid-stream.
                out.extend_from_slice(&packet(2, &column_def("token")));
                out.extend_from_slice(&packet(3, &column_def("greeting")));
                out
            }
            Some(0x17) => {
                let mut out = packet(1, &[0x01]);
                out.extend_from_slice(&packet(2, &column_def("greeting")));
                out.extend_from_slice(&packet(
                    3,
                    &[0x00, 0x00, 0x05, b'h', b'e', b'l', b'l', b'o'],
                ));
                out.extend_from_slice(&packet(
                    4,
                    &[0x00, 0x00, 0x05, b'w', b'o', b'r', b'l', b'd'],
                ));
                out.extend_from_slice(&packet(5, &result_set_end(0x0002)));
                out
            }
            other => panic!("the driver sent command {other:?}"),
        }));
        let state = Cell::new(State::Idle);

        let mut rows = start_statement(
            &mut wire,
            &state,
            CLIENT_CAPABILITIES,
            &mut no_cache(),
            SQL,
            &[Some(BOUND.as_bytes())],
        )
        .expect("a result set the server described");
        while rows.next_row().expect("the stream drained").is_some() {}
        // Cloned and the stream dropped, so the wire is readable below: the
        // value having reached the server is what makes this about where it
        // stopped rather than about a parameter nobody sent.
        let span = rows.span().clone();
        drop(rows);

        assert!(
            wire.peer()
                .sent
                .iter()
                .any(|flushed| contains(flushed, BOUND.as_bytes())),
            "the bound value never reached the wire"
        );

        assert_eq!(span.driver(), Driver::MySql);
        assert_eq!(span.rows(), 2);
        assert_eq!(
            span.affected(),
            Some(2),
            "a result set's affected count is the rows that came back, which is \
             what PostgreSQL's `SELECT 2` tag says on the other driver"
        );
        assert_eq!(span.sql(), SQL);
        assert!(!span.is_truncated());
        assert!(span.connection().is_none(), "the driver names no block");
        assert!(span.sql().contains('?'), "{}", span.sql());

        let shown = span.to_string();
        assert!(!shown.contains(BOUND), "{shown}");
        assert!(shown.contains("driver=mysql"), "{shown}");
        assert!(shown.contains("rows=2"), "{shown}");

        let traced = format!("{span:?}");
        assert!(!traced.contains(BOUND), "{traced}");

        // The one field the driver does not fill, filled: a named connection
        // adds the block's name and still no value.
        let mut named = span.clone();
        named.name("main");
        let shown = named.to_string();
        assert!(shown.contains("connection=main"), "{shown}");
        assert!(!shown.contains(BOUND), "{shown}");
        assert!(!format!("{named:?}").contains(BOUND));
    }

    /// A statement with no result set ends its span at the status packet.
    ///
    /// The arm that would otherwise be missed: there is no terminator to reach
    /// and no row to count, so a driver that only ended a span in `next_row`
    /// would file a write's event with the duration still running and no
    /// affected count at all — and § 11's whole point on a write is that count.
    #[test]
    fn a_statement_with_no_result_set_ends_its_span_at_the_status_packet() {
        let mut wire = Wire::new(Peer::new(|sent: &[u8]| match sent.get(4) {
            Some(0x16) => packet(1, &prepare_ok(3, 0, 0)),
            Some(0x17) => packet(1, &ok_packet(4, 0)),
            other => panic!("the driver sent command {other:?}"),
        }));
        let state = Cell::new(State::Idle);

        let rows = start_statement(
            &mut wire,
            &state,
            CLIENT_CAPABILITIES,
            &mut no_cache(),
            "DELETE FROM t",
            &[],
        )
        .expect("a statement the server answered with a status packet");

        let span = rows.span();
        assert_eq!(span.rows(), 0);
        assert_eq!(
            span.affected(),
            Some(4),
            "the status packet's own count, and its presence is what says the \
             span ended rather than that it is still running"
        );
        let took = span.duration();
        assert_eq!(
            took,
            span.duration(),
            "a frozen duration reads the same twice; a running one reads the clock"
        );
    }

    /// An `ERR` packet as `CLIENT_PROTOCOL_41` words one: the header, the error
    /// code, the `#`-prefixed `SQLSTATE` and the message.
    fn error_packet(code: u16, sql_state: &str, message: &str) -> Vec<u8> {
        let mut body = vec![0xFF];
        body.extend_from_slice(&code.to_le_bytes());
        body.push(b'#');
        body.extend_from_slice(sql_state.as_bytes());
        body.extend_from_slice(message.as_bytes());
        body
    }

    /// `rule:core-classes/db-error` over MySQL's `ERR` packet: the normalised kind rides inside
    /// the error, and both raw values ride beside it.
    ///
    /// The `io::ErrorKind` is asserted too, and it is the half a reader is most
    /// likely to think is incidental: `nvs_stdlib::db`'s `statement_failure`
    /// throws `Db\DbError` from its `Other` arm alone, so a refusal answered
    /// under any other kind reaches a program as an `IoError` with no `kind`,
    /// no `sqlState` and no `driverCode` on it whatever this function filled in.
    #[test]
    fn a_server_refusal_carries_the_kind_and_both_raw_codes() {
        let refused = server_refusal(
            &MYSQL,
            &error_packet(1213, "40001", "Deadlock found when trying to get lock"),
            CLIENT_CAPABILITIES,
        );

        assert_eq!(refused.kind(), io::ErrorKind::Other);
        let server = ServerError::of(&refused).expect("a refusal the server worded");
        assert_eq!(server.kind, DbErrorKind::Deadlock);
        assert_eq!(server.driver_code, Some(1213));
        assert_eq!(server.sql_state, "40001");
        assert_eq!(server.backend, "mysql");
        assert!(
            refused.to_string().contains("Deadlock found"),
            "the server's own sentence is still what a caller prints: {refused}"
        );
    }

    /// The code table, asserted as a table — and the two codes § 7's
    /// `{retries: n}` turns on named against each other.
    ///
    /// `1213` and `1205` are the pair worth pinning together: they are what an
    /// application reads when it is deciding whether to retry, they are a digit
    /// apart in a long table, and a driver that mapped the lock wait
    /// timeout to `Deadlock` would re-run a callable inside a transaction the
    /// server never rolled back. Asserting `is_retryable` on both sides is the
    /// claim; asserting the kind alone would let that swap read green against
    /// either row on its own.
    ///
    /// The rows at the end are the fallback: a code this table does not name is
    /// classified by its `SQLSTATE` class where MySQL fills one, and is
    /// `Other` — never a guess — where it does not.
    #[test]
    fn the_code_table_names_the_kind_and_the_retryable_pair_disagree() {
        for (code, sql_state, expected) in [
            (1062_u16, "23000", DbErrorKind::UniqueViolation),
            (1452, "23000", DbErrorKind::ForeignKeyViolation),
            (1048, "23000", DbErrorKind::NotNullViolation),
            (3819, "HY000", DbErrorKind::CheckViolation),
            (1213, "40001", DbErrorKind::Deadlock),
            (1205, "HY000", DbErrorKind::Timeout),
            (1053, "08S01", DbErrorKind::ConnectionLost),
            (1045, "28000", DbErrorKind::Permission),
            (1064, "42000", DbErrorKind::Syntax),
            // Unnamed codes: the class decides where MySQL filled one, and
            // `HY000` — the class it fills for everything else — decides
            // nothing.
            (1049, "42000", DbErrorKind::Syntax),
            (9999, "42S02", DbErrorKind::Syntax),
            (9999, "HY000", DbErrorKind::Other),
        ] {
            assert_eq!(
                kind_of(code, sql_state),
                expected,
                "MySQL {code} (SQLSTATE {sql_state}) is not the § 8 kind it means"
            );
        }

        assert!(
            kind_of(1213, "40001").is_retryable(),
            "§ 7 re-runs the callable over a deadlock: the server rolled the \
             transaction back and there is nothing left of it to run twice"
        );
        assert!(
            !kind_of(1205, "HY000").is_retryable(),
            "and not over a lock wait timeout, which rolls back the statement \
             and leaves the transaction open and holding its locks"
        );
    }

    /// `rule:core-classes/db-statement-members`'s batch priced in round trips, which is the only thing about
    /// it a caller cannot see from a loop of `execute`: one prepare for the
    /// whole batch and one execution per set.
    ///
    /// **The count of commands is the assertion**, as § 1's cache case does it.
    /// A batch that re-prepared per set answers the same sum and costs twice
    /// what § 1 prices it at, and this driver's N executions are the protocol's
    /// price rather than an oversight — `execute_many`'s own doc owns why one
    /// flush is not available here.
    ///
    /// The empty batch is asserted first and on the same wire: § 4's no-op
    /// answers `0` and must not prepare a statement for executions that will
    /// not happen.
    #[test]
    fn a_batch_prepares_once_and_costs_one_execution_per_set() {
        const SQL: &str = "INSERT INTO t (a) VALUES (?)";

        let mut wire = Wire::new(Peer::new(|sent: &[u8]| match sent.get(4) {
            Some(0x16) => {
                let mut out = packet(1, &prepare_ok(11, 0, 1));
                out.extend_from_slice(&packet(2, &column_def("a")));
                out
            }
            Some(0x17) => packet(1, &ok_packet(2, 0)),
            other => panic!("the driver sent command {other:?}"),
        }));
        let state = Cell::new(State::Idle);
        let mut cache = sized_cache(2);

        assert_eq!(
            execute_many(&mut wire, &state, CLIENT_CAPABILITIES, &mut cache, SQL, &[])
                .expect("§ 4's no-op"),
            0
        );
        assert!(
            wire.peer().sent.is_empty(),
            "an empty batch prepares nothing, because nothing is going to run"
        );

        let bound: [[Option<&[u8]>; 1]; 3] = [[Some(b"x")], [Some(b"y")], [Some(b"z")]];
        let sets: [&[Option<&[u8]>]; 3] = [&bound[0], &bound[1], &bound[2]];

        let written = execute_many(
            &mut wire,
            &state,
            CLIENT_CAPABILITIES,
            &mut cache,
            SQL,
            &sets,
        )
        .expect("three executions the server answered");

        assert_eq!(
            written, 6,
            "§ 4's answer is the sum of what each execution changed"
        );
        assert_eq!(
            commands(&wire.peer().sent),
            [Some(0x16), Some(0x17), Some(0x17), Some(0x17)],
            "`rule:core-classes/db-statement-members`: one prepare for the batch, and one execution per set"
        );
        assert_eq!(
            state.get(),
            State::Idle,
            "a batch that ended leaves the connection reusable"
        );
    }

    /// § 4's batch is not a transaction, asserted where it is visible: a set the
    /// server refuses does not end the batch, and the **first** refusal is what
    /// the batch reports.
    ///
    /// Two sets are refused so that "first" is a claim with something to be
    /// wrong about — a driver reporting the last one answers a case with one
    /// refusal identically. The command count is the other half: the execution
    /// after a refused one still goes out, which is the observable
    /// `crate::pg`'s own batch reaches by letting the server resume at the next
    /// `Sync`.
    #[test]
    fn a_refused_set_does_not_end_the_batch_and_the_first_refusal_is_reported() {
        const SQL: &str = "INSERT INTO t (a) VALUES (?)";

        let mut wire = Wire::new(Peer::new(|sent: &[u8]| match sent.get(4) {
            Some(0x16) => {
                let mut out = packet(1, &prepare_ok(13, 0, 1));
                out.extend_from_slice(&packet(2, &column_def("a")));
                out
            }
            Some(0x17) if contains(sent, b"boom") => {
                packet(1, &error_packet(1062, "23000", "duplicate boom"))
            }
            Some(0x17) if contains(sent, b"bang") => {
                packet(1, &error_packet(1213, "40001", "deadlock bang"))
            }
            Some(0x17) => packet(1, &ok_packet(1, 0)),
            other => panic!("the driver sent command {other:?}"),
        }));
        let state = Cell::new(State::Idle);
        let mut cache = sized_cache(2);

        let bound: [[Option<&[u8]>; 1]; 4] =
            [[Some(b"x")], [Some(b"boom")], [Some(b"bang")], [Some(b"z")]];
        let sets: [&[Option<&[u8]>]; 4] = [&bound[0], &bound[1], &bound[2], &bound[3]];

        let refused = execute_many(
            &mut wire,
            &state,
            CLIENT_CAPABILITIES,
            &mut cache,
            SQL,
            &sets,
        )
        .expect_err("two of the four sets were refused");

        assert_eq!(
            ServerError::of(&refused).map(|server| server.kind),
            Some(DbErrorKind::UniqueViolation),
            "the batch answers the server's own refusal, § 8's kind and all: {refused}"
        );
        assert!(
            refused.to_string().contains("duplicate boom"),
            "the first refusal is the batch's answer: {refused}"
        );
        assert!(
            !refused.to_string().contains("deadlock bang"),
            "and the second is not: {refused}"
        );
        assert_eq!(
            commands(&wire.peer().sent),
            [Some(0x16), Some(0x17), Some(0x17), Some(0x17), Some(0x17)],
            "every set was attempted: § 4's batch is not a transaction, so a \
             refusal does not cancel the writes after it"
        );
        assert_eq!(
            state.get(),
            State::Idle,
            "a refusal the server worded arrived whole, so the connection is \
             still at a packet boundary"
        );
    }

    /// `rule:core-classes/db-statement-members` on MariaDB, which is the driver that could do this in one
    /// command and does not: the batch is N executions and never
    /// `COM_STMT_BULK_EXECUTE`.
    ///
    /// **The permission is asserted first, and that is what makes this a
    /// decision rather than a gap.** A test that only counted commands would
    /// pass just as well on a driver that never negotiated the capability at
    /// all, and would then be pinning an absence — so the first assertion is
    /// that this driver *may* send `0xFA`, and everything after it is that it
    /// does not.
    ///
    /// The batch refuses a set for the same reason
    /// [`a_refused_set_does_not_end_the_batch_and_the_first_refusal_is_reported`]
    /// does, because that is the observable § 4 spends the round trips on: a
    /// bulk command ends at a refusal, so a batch that carries past one is the
    /// evidence the loop is what ran. Counting `0xFA` alone would still pass on
    /// a driver that sent the bulk command and stopped.
    #[test]
    fn execute_many_on_mariadb_is_n_executions_and_not_the_bulk_command() {
        const SQL: &str = "INSERT INTO t (a) VALUES (?)";
        /// `COM_STMT_BULK_EXECUTE`, the command this test is about not seeing.
        const COM_STMT_BULK_EXECUTE: u8 = 0xFA;

        assert!(
            EXTENDED_CAPABILITIES
                .contains(MariadbCapabilities::MARIADB_CLIENT_STMT_BULK_OPERATIONS),
            "this driver stopped claiming the capability, so what follows would \
             assert an absence rather than `rule:core-classes/db-statement-members`'s decision"
        );

        let mut wire = Wire::new(Peer::new(|sent: &[u8]| match sent.get(4) {
            Some(&COM_STMT_BULK_EXECUTE) => {
                panic!(
                    "MariaDB's batch sent COM_STMT_BULK_EXECUTE, which `rule:core-classes/db-statement-members` refuses"
                )
            }
            Some(0x16) => {
                let mut out = packet(1, &prepare_ok(17, 0, 1));
                out.extend_from_slice(&packet(2, &column_def("a")));
                out
            }
            Some(0x17) if contains(sent, b"boom") => {
                packet(1, &error_packet(1062, "23000", "duplicate boom"))
            }
            Some(0x17) => packet(1, &ok_packet(1, 0)),
            other => panic!("the driver sent command {other:?}"),
        }));
        let state = Cell::new(State::Idle);
        let mut cache = sized_cache(2);

        let bound: [[Option<&[u8]>; 1]; 3] = [[Some(b"x")], [Some(b"boom")], [Some(b"z")]];
        let sets: [&[Option<&[u8]>]; 3] = [&bound[0], &bound[1], &bound[2]];

        let refused = execute_many(
            &mut wire,
            &state,
            CLIENT_CAPABILITIES,
            &mut cache,
            SQL,
            &sets,
        )
        .expect_err("the middle set was refused");
        assert!(
            refused.to_string().contains("duplicate boom"),
            "the batch answers the refusal it drew: {refused}"
        );

        let sent = commands(&wire.peer().sent);
        assert!(
            !sent.contains(&Some(COM_STMT_BULK_EXECUTE)),
            "§ 4 runs executeMany as N executions on every driver: {sent:?}"
        );
        assert_eq!(
            sent,
            [Some(0x16), Some(0x17), Some(0x17), Some(0x17)],
            "one prepare and one execution per set, the third of them written \
             after the second was refused — which is the half of § 4's \
             semantics a bulk command cannot reproduce"
        );
    }

    /// `rule:core-classes/db-column-types`'s type map, as the whole table rather than a row of it.
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
    /// packet's other way of being wrong, and `rule:core-classes/db-parameters`'s rewriter is what
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
            &mut no_cache(),
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

    /// Whether `needle` appears anywhere in `haystack`, which is how the cases
    /// above ask what did and did not reach the wire.
    fn contains(haystack: &[u8], needle: &[u8]) -> bool {
        haystack.windows(needle.len()).any(|at| at == needle)
    }
}
