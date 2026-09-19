//! SQL Server: TDS 7.4's packet framing, which is an eight-byte header in front
//! of every message, the split a message longer than the negotiated packet size
//! takes on the way out, and the reassembly it takes on the way back.
//!
//! [ADR 0132 § 5](/docs/decisions/0132.md)
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
//! # What this module holds
//!
//! Framing; the block that says which server to frame *to*, since [`TdsTarget`]
//! reads a `[db.<name>]` block the way every other driver's target does; and
//! the handshake as far as encryption — [`prelogin`] asks for it and
//! [`negotiate_tls`] tunnels `rule:core-classes/db-capabilities`'s TLS handshake inside PRELOGIN
//! packets, after which the socket is an ordinary [`NvsTls`] stream carrying
//! ordinary TDS packets; [`login7_request`], the credential-carrying message
//! that rides that session and is the reason § 3's TLS is not optional here;
//! and [`Tokens`], which reads that message's answer as far as `LOGINACK`,
//! `ENVCHANGE`, `ERROR`, `INFO` and `DONE`.
//!
//! **The handshake runs end to end**: [`TdsConn::connect`] opens the socket
//! and [`login`] sends the credential and reads what came back, so a `TdsConn`
//! holds a live encrypted wire and the framing the server's `ENVCHANGE`
//! settled. **A statement's answer reads back**: [`read_rows`] takes a wire
//! with a request already on it and answers a [`TdsRows`], which streams `ROW`,
//! `NBCROW` and `PLP` over [`Wire::read_packet`] with the remainder held across
//! the packet boundary — the reader the first section says [`Tokens`] cannot be.
//!
//! **A statement goes out**: [`sp_prepexec_request`] builds the RPC, ADR
//! 0067 § 1's cache holds the handle the `RETURNVALUE` came back with,
//! [`start_statement`] is the sequencing both `query` and `execute` are,
//! [`execute_many`] is § 4's batch over that same sequencing once per parameter
//! set, and [`reset_session`] is § 13's `sp_reset_connection`. **§ 7's commands
//! are here**: [`begin`], [`commit`] and [`roll_back`] send T-SQL over
//! [`batch_command`], the one path in this module that is text rather than an
//! RPC, and [`begin`]'s own doc owns the rules that are this dialect's alone —
//! a session-scoped isolation level this driver has to put back, and no
//! read-only transaction to offer at all. Every member `rule:core-classes/db-one-api` declares is
//! therefore reachable on this backend, and [ADR 0067 §
//! 9](/docs/decisions/0067.md)'s table is whole in both directions: a `bytes`
//! is read by [`decode_column`] from `varbinary`, `binary` and `image`, and
//! bound as a `varbinary` parameter of its own — [`encode`] decides the form
//! and [`binary_param`] writes it, so the declaration varies with the values a
//! call binds and [`TdsPlan`] is what tells two plan shapes apart.

use std::borrow::Cow;
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
use nvs_runtime::{Decimal, NvsStr, Tag, Value};

use crate::conn::{
    BlockError, ColumnType, DbErrorKind, Driver, Isolation, ServerError, State, TdsConn,
    is_socket_host, written_value,
};
use crate::span::QuerySpan;
use crate::sql::{Dialect, StatementCache, statement_cache_for, time_zone_for};

mod framing;
mod login;
mod plan;
mod prelogin;
mod rows;
mod rpc;
mod stream;
#[cfg(test)]
mod testing;
mod token;
mod types;
mod value;
mod wire;

pub use framing::*;
pub use login::*;
pub use plan::*;
pub use prelogin::*;
pub use rows::*;
pub use rpc::*;
pub use stream::*;
pub use token::*;
pub use types::*;
pub use value::*;
pub use wire::*;

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

/// SQL Server's own port, which an absent `port` in a `[db.<name>]` block
/// means.
///
/// Not a field of [`TdsTarget`], for the reason it is not one on
/// [`crate::pg::PgTarget`]: a port is half of an address, and the address is
/// resolved by whoever checked the `db.connect` capability. What this crate
/// owes that caller is the number to fall back to.
///
/// It is the *default instance's* port. A named instance listens wherever the
/// server assigned it and is found by asking the SQL Browser over UDP 1434 —
/// a second protocol, unauthenticated, and one this driver does not speak, so
/// a block naming such an instance writes the `port` an operator read off the
/// server rather than having it discovered.
pub const DEFAULT_PORT: u16 = 1433;

/// One `[db.<name>]` block, read as the facts a LOGIN7 message carries.
///
/// [`crate::MySqlTarget`]'s and [`crate::PgTarget`]'s twin, and the twinning is
/// the point: [ADR 0067 § 2](/docs/decisions/0067.md) makes a block a
/// discriminated union on `driver`, so a server block holds the same fields
/// whichever driver reads it and the refusals are one vocabulary —
/// [`BlockError`], in [`mod@crate::conn`] for that reason.
///
/// **The target borrows the block and copies nothing**, for the reason
/// [`crate::MySqlTarget`] gives at length: owning these strings would put a
/// second copy of the password — a `secret` at the language level (§ 3) — in a
/// struct nothing zeroes.
///
/// The address is not here. `host` is the name the server's certificate is
/// checked against; resolving it to a [`std::net::SocketAddr`] belongs to
/// whoever checked the `db.connect` capability, and a driver that re-resolved
/// the name would be connecting somewhere nobody approved.
pub struct TdsTarget<'a> {
    /// The name the server's certificate is checked against.
    pub host: &'a str,
    /// The login to authenticate as. `rule:core-classes/db-capabilities` accepts `tainted` here
    /// freely: LOGIN7 carries it as a length-prefixed field, never as parsed
    /// text.
    pub user: &'a str,
    /// The password. On this protocol it goes out *as* a password rather than
    /// as a challenge response — obfuscated by LOGIN7's nibble swap, which is
    /// not encryption — which is why `rule:core-classes/db-capabilities`'s TLS is not optional here in
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
    /// Not a way to turn verification off — `rule:core-classes/db-capabilities` has no spelling for
    /// that. What it changes is *whose* certificates are believed.
    pub tls_ca_file: Option<&'a Path>,
    /// The zone a `datetime2` or `datetime` off this connection is read in, as
    /// a whole number of seconds east of UTC.
    ///
    /// [ADR 0067 § 9](/docs/decisions/0067.md)'s declared zone, read
    /// by the same `time_zone_for` the other drivers go through — **and this is
    /// the one driver that cannot send it**. SQL Server has no session time
    /// zone: `AT TIME ZONE` is an expression, and `SET` has no such setting, so
    /// there is nothing to send it to. The declared zone therefore governs
    /// decoding alone, and `GETDATE()` stays the server's own clock in the
    /// server's own zone. § 9's sentence about the server agreeing is a MySQL
    /// and PostgreSQL fact, not a property of the field.
    pub time_zone: i32,
    /// How many prepared statements this connection may keep alive on the
    /// server, [ADR 0067 § 1](/docs/decisions/0067.md)'s
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
    /// then each field LOGIN7 sends, by its own key — `host` twice, once for
    /// being unwritten and once for being a socket path this protocol has no
    /// transport for; then § 9's zone.
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
        // TDS has no `AF_UNIX` transport, so a path is refused here rather than
        // carried to a dial that would have nothing to open it with —
        // `rule:core-classes/db-unix-socket-path`, and the refusal names the
        // target rather than the file.
        if is_socket_host(host) {
            return Err(BlockError::NoSocketTransport {
                written: host,
                expected: Driver::SqlServer,
            });
        }
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
    /// (`rule:core-classes/db-capabilities`) and is not printed in any rendering.
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

impl TdsConn {
    /// [ADR 0067 § 3](/docs/decisions/0067.md)'s handshake end to
    /// end: a socket, PRELOGIN, the TLS session tunnelled inside it, LOGIN7 and
    /// the tokens that answer it.
    ///
    /// [`crate::MySqlConn::connect`]'s shape and its rules. `addr` is where to
    /// connect and `target.host` is only the name the certificate is checked
    /// against: resolving one to the other belongs to whoever checked
    /// `db.connect`, and a driver that re-resolved the name would be connecting
    /// somewhere nobody approved. `deadline` covers the whole handshake rather
    /// than each step, because what a caller bounds is how long opening a
    /// connection may take. It is lifted before the connection is handed back,
    /// as [`crate::PgConn::connect`] lifts its own and for that one's reason.
    ///
    /// Unlike every other driver here this one sends nothing after the login.
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
        let ack = login(&mut wire, target)?;
        // Lifted for [`crate::PgConn::connect`]'s reason, which is the same on every
        // driver that files the handshake clock on its own socket.
        wire.set_deadline(None);

        Ok(TdsConn {
            wire,
            state: Cell::new(State::Idle),
            // The numbers LOGINACK sent, spelled once — see the field.
            server_version: ack.server_version(),
            // § 9's zone-less row is decoded a layer up, where the target is
            // gone — see the field.
            time_zone: target.time_zone,
            // § 1's capacity is the `[db.<name>]` block's, already read by
            // `statement_cache_for` and carried here on the target.
            cache: StatementCache::new(target.statement_cache),
            depth: Cell::new(0),
            // LOGIN7 leaves the session at the login's own default, which is
            // `DEFAULT_ISOLATION`, so nothing is owed until a `transaction()`
            // asks for a level.
            isolation_moved: Cell::new(false),
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

    /// [ADR 0067 § 13](/docs/decisions/0067.md)'s reset, before this
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
        // A walk the program abandoned is read to its end before the reset goes
        // out, and then dropped. The reset is a request like any other, and
        // every TDS request has exactly one answer: written over an answer this
        // side has not finished reading, its own would be read as the rest of
        // that one.
        end_stream_of(
            &mut self.wire,
            &self.state,
            Some(&mut self.cache),
            &mut self.reading,
        );
        reset_session(
            &mut self.wire,
            &self.state,
            &mut self.cache,
            &self.isolation_moved,
        )?;
        // `sp_reset_connection` rolls back whatever transaction was open, so
        // § 7's depth is answered by it — and the isolation level is **not**,
        // which is why the flag is cleared by the restore [`reset_session`] pays
        // rather than here. The server's own transaction descriptor is cleared
        // there too, that being where the request carrying it is written. A
        // connection pooled at a depth it no longer has would open the next
        // request's outermost `transaction()` as a `SAVE TRANSACTION` against
        // nothing.
        self.depth.set(0);
        Ok(self)
    }

    /// [ADR 0067 § 1](/docs/decisions/0067.md)'s round trips for one
    /// statement — one either way on this protocol — and the columns its result
    /// set turned out to have.
    ///
    /// The two-line delegation the playbook prescribes, for
    /// [`crate::MySqlConn::query`]'s reason: [`start_statement`] is where the
    /// sequencing lives, because a method on `TdsConn` can only be reached
    /// through a real socket and a real certificate and so cannot be unit-tested
    /// at all.
    ///
    /// **`execute` is this same method**, unlike the separate members § 4
    /// declares: `sp_prepexec` carries a statement that describes no result set
    /// exactly as it carries one that does, and what tells them apart is
    /// [`TdsRows::affected`] rather than a second request. `crate::mysql` makes
    /// the same call for the same reason, and `nvs-stdlib` is where those
    /// members part.
    ///
    /// The result borrows the connection until it ends, which is [`TdsRows`]'
    /// whole point: § 4's one-statement-at-a-time rule is not a check this
    /// method performs but a borrow the caller cannot get around.
    ///
    /// # Errors
    ///
    /// As [`start_statement`].
    pub fn query(&mut self, sql: &str, params: &[Option<&[u8]>]) -> io::Result<TdsRows<'_>> {
        start_statement(&mut self.wire, &self.state, &mut self.cache, sql, params)
    }

    /// Runs one statement and leaves its answer open, **borrowing nothing**:
    /// the read state is parked on this connection and the rows come off it one
    /// [`Self::stream_next_row`] at a time.
    ///
    /// The wire half of `rule:core-classes/db-streaming`'s `stream`, costing the round trips
    /// [`Self::query`] costs and stopping where it stops — [`TdsCursor`] owns
    /// why the state has to be here rather than inside a borrow. The answer is
    /// what the result set described, empty for a statement that returned none,
    /// and [`Self::stream_columns`] hands the same slice back to the later calls
    /// that decode against it.
    ///
    /// A second statement is refused while this one is open, which is
    /// `rule:core-classes/db-streaming`'s `LogicError` read off [`State::Streaming`] rather than
    /// off a lifetime. [`Self::end_stream`] is what a program that walks away
    /// from the rows owes; a walk read to its end needs no call at all.
    ///
    /// # Errors
    ///
    /// As [`Self::query`].
    pub fn stream(&mut self, sql: &str, params: &[Option<&[u8]>]) -> io::Result<&[TdsColumn]> {
        let reading = open_result(&mut self.wire, &self.state, &mut self.cache, sql, params)?;
        Ok(self.reading.insert(reading).columns())
    }

    /// What the parked walk's result set described, or `None` for a connection
    /// that has not streamed since its last reset.
    ///
    /// It outlives the rows on purpose: a value is measured by the column it
    /// belongs to, and § 9's decode of it happens after the step that produced
    /// it.
    #[must_use]
    pub fn stream_columns(&self) -> Option<&[TdsColumn]> {
        Some(self.reading.as_ref()?.columns())
    }

    /// The next row of the parked walk, or `None` once it has ended — and
    /// `None` too for a connection with no walk parked on it at all.
    ///
    /// Ending it returns the connection to [`State::Idle`], exactly as
    /// [`TdsRows::next_row`] does. The state itself stays parked, holding what
    /// the statement finished with, until the next [`Self::stream`] replaces it
    /// or [`Self::end_stream`] drops it.
    ///
    /// # Errors
    ///
    /// As [`next_row_of`].
    pub fn stream_next_row(&mut self) -> io::Result<Option<TdsRow>> {
        let Some(reading) = self.reading.as_mut() else {
            return Ok(None);
        };
        next_row_of(&mut self.wire, &self.state, Some(&mut self.cache), reading)
    }

    /// [ADR 0067 § 11](/docs/decisions/0067.md)'s trace event for the
    /// parked walk, or `None` where there is none.
    #[must_use]
    pub fn stream_span(&self) -> Option<&QuerySpan> {
        Some(self.reading.as_ref()?.span())
    }

    /// Names the `[db.<name>]` block the parked walk is running on, and does
    /// nothing where there is no walk — [`TdsRows::name_connection`] owns why
    /// the driver cannot work the name out for itself.
    pub fn name_stream_connection(&mut self, connection: &str) {
        if let Some(reading) = self.reading.as_mut() {
            reading.name_connection(connection);
        }
    }

    /// Abandons the parked walk: reads what is left of the answer and forgets
    /// it.
    ///
    /// [`end_stream_of`] owns why the drain is not optional on this protocol.
    pub fn end_stream(&mut self) {
        end_stream_of(
            &mut self.wire,
            &self.state,
            Some(&mut self.cache),
            &mut self.reading,
        );
    }

    /// [ADR 0067 § 4](/docs/decisions/0067.md)'s `executeMany`: one
    /// prepare, N executions, and the affected counts summed.
    ///
    /// The two-line delegation [`TdsConn::query`] gives its reason for.
    ///
    /// # Errors
    ///
    /// As [`execute_many`].
    pub fn execute_many(&mut self, sql: &str, sets: &[&[Option<&[u8]>]]) -> io::Result<u64> {
        execute_many(&mut self.wire, &self.state, &mut self.cache, sql, sets)
    }

    /// [ADR 0067 § 7](/docs/decisions/0067.md)'s `BEGIN TRANSACTION`,
    /// or the `SAVE TRANSACTION` a nested `transaction()` is.
    ///
    /// The driver half of § 7 and nothing more — the closure, the rollback-only
    /// flag and the retry rule are `nvs-stdlib`'s, exactly as on
    /// [`crate::MySqlConn::begin`]. [`begin`] owns which command a nesting depth
    /// gets, why `read_only` is refused on this backend, and how a
    /// session-scoped isolation level is put back.
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
            &self.depth,
            &self.isolation_moved,
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

    /// § 7's `COMMIT TRANSACTION`, or the nested commit that sends nothing — a
    /// normal return out of the closure either way.
    ///
    /// # Errors
    ///
    /// As [`commit`].
    pub fn commit(&mut self) -> io::Result<QuerySpan> {
        commit(
            &mut self.wire,
            &self.state,
            &self.depth,
            &self.isolation_moved,
        )
    }

    /// § 7's `ROLLBACK TRANSACTION`, or the one naming the savepoint that undoes
    /// a nested level — a throw out of the closure, or `rollBack`'s own signal.
    ///
    /// # Errors
    ///
    /// As [`roll_back`].
    pub fn roll_back(&mut self) -> io::Result<QuerySpan> {
        roll_back(
            &mut self.wire,
            &self.state,
            &self.depth,
            &self.isolation_moved,
        )
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
    use crate::tds::testing::*;

    /// A complete block resolves to what LOGIN7 sends, and the fields with
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

    /// `rule:core-classes/db-connection-is-named`'s discriminant, from this side of it.
    ///
    /// Every driver's resolver owes this case, and the reason it is not
    /// redundant with `crate::mysql`'s is the direction: what is asserted is
    /// that *this* resolver refuses every other driver's block, so a block an
    /// operator wrote for one backend cannot open a connection that speaks
    /// another.
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

    /// The fields LOGIN7 sends are refused one by one, each naming its own
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

    /// `rule:core-classes/db-unix-socket-path`: MSSQL reports a socket path as
    /// a target it does not speak, and never as a file it could not open.
    ///
    /// Asserted at the *resolver* rather than at a dial, which is the whole
    /// point of the shape: TDS has no `AF_UNIX` transport on any platform, so
    /// there is no build where this path reaches a connect, and the refusal
    /// arrives when an operator's block is read. The message names `host`,
    /// because `host` is the line they would edit, and it must not read as an
    /// unopenable file — a `[db.<name>]` on this driver is wrong, not missing.
    #[test]
    fn a_tds_target_refuses_a_socket_path() {
        for written in ["/var/run/mssql.sock", "/tmp/sqlserver/"] {
            let mut block = block();
            block.host = Some(written.to_owned());
            let refused = TdsTarget::resolve(&block).unwrap_err();
            assert_eq!(
                refused,
                BlockError::NoSocketTransport {
                    written,
                    expected: Driver::SqlServer,
                }
            );

            let text = refused.refusal("main");
            assert!(
                text.contains("`host`") && text.contains("transport that speaks one"),
                "the refusal names the field and the transport that is missing: {text}"
            );
            assert!(
                !text.contains("open") && !text.contains("file"),
                "a path is refused as a target, so nothing here reads as an \
                 unopenable file: {text}"
            );
        }

        let mut network = block();
        network.host = Some("mssql.test".to_owned());
        assert!(
            TdsTarget::resolve(&network).is_ok(),
            "only a path is refused, and a host:port is untouched"
        );
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
}
