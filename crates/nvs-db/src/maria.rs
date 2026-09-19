//! MariaDB: reading a `[db.<name>]` block, opening a connection to a MariaDB
//! server over [`crate::mysql`]'s framing, with MariaDB's own authentication
//! plugins and MariaDB's own error table.
//!
//! **Its own driver, and the reason is behaviour rather than bytes.** MariaDB
//! and MySQL frame packets identically — the same codec, the same sequence-id
//! discipline, the same in-band TLS upgrade — and this module borrows every one
//! of those from [`crate::mysql`] rather than keeping a second copy of a wire
//! format that would then have two places to go wrong. What it does not borrow
//! is the part a program can observe: which plugins a server may authenticate
//! this driver with, what a vendor error code means
//! ([ADR 0067 § 8](/docs/decisions/0067.md): "MariaDB needs its own
//! code table, not MySQL's"), and `RETURNING`, which MySQL does not have.
//! `rule:core-classes/db-one-api` argues at length that treating those as
//! flags on a MySQL connection is a design error; the split here is that
//! argument, and `crate::mysql::Backend` is the one place the shared framing
//! asks which server it is framing for.
//!
//! # The roster is the security decision, and it is not MySQL's
//!
//! This driver authenticates with [`MYSQL_NATIVE_PASSWORD`], MariaDB's default
//! through 11.x, and with [`CLIENT_ED25519`] and [`PARSEC`], which are
//! MariaDB's own and which MySQL has never offered. Every other plugin is
//! refused **by name**, including `caching_sha2_password`: a server naming it
//! is not the server this driver was pointed at, and [`crate::mysql`]'s roster
//! is not reachable from here even where a peer asks for it.
//!
//! `client_ed25519` and `parsec` are answered by `mysql_common`'s own
//! implementations, behind its `client_ed25519` and `client_parsec` features —
//! pure Rust, which is
//! [ADR 0051 § 4](/docs/decisions/0051.md)'s answer
//! for exactly this case. That section names these plugins in advance and
//! allows either a Rust implementation or a documented refusal; this is the
//! implementation. Both send a signature over the server's nonce, so what
//! leaves this process is a proof and not the secret — the same test the
//! plugins in [`crate::mysql`]'s roster pass, and the reason
//! `mysql_clear_password` fails it on either driver.
//!
//! [`plugin_or_refuse`] is the single gate, and it is applied in both places a
//! plugin can be named: the greeting, and the `AuthSwitchRequest` a server may
//! send after the response has gone out. `crate::mysql`'s module doc owns why
//! the second one matters.
//!
//! # Opening is [`crate::mysql`]'s exchanges
//!
//! The greeting in the clear, the `SSLRequest`, the handshake response and its
//! auth loop over TLS, then `SET time_zone` — that sequence is the protocol's
//! and is described once, in [`crate::mysql`]'s module doc. What
//! [`MariaConn::connect`] adds is which roster the handshake response runs
//! with and which table a refusal at any of them is read against.

use std::cell::Cell;
use std::io;
use std::path::Path;
use std::time::Instant;

#[cfg(unix)]
use mysql_common::constants::CapabilityFlags;
use mysql_common::constants::MariadbCapabilities;
use mysql_common::packets::AuthPlugin;
use mysql_common::packets::Column;
use nvs_config::tree::Database;
use nvs_host::net::NvsTcp;
#[cfg(unix)]
use nvs_host::net::NvsUnix;
use nvs_host::tls::NvsTls;

use crate::conn::{BlockError, DbErrorKind, Driver, Endpoint, MariaConn, State, written_value};
#[cfg(unix)]
use crate::mysql::REQUIRED_OVER_A_SOCKET;
use crate::mysql::{Backend, Login, MyStream, REQUIRED_CAPABILITIES, Wire};
use crate::sql::{StatementCache, statement_cache_for, time_zone_for};

/// MariaDB's port, which an absent `port` in a `[db.<name>]` block means.
///
/// The same number MySQL's default is, and a separate constant because it is a
/// separate protocol's default that happens to coincide: a deployment that
/// moved one would not have moved the other.
pub const DEFAULT_PORT: u16 = 3306;

/// The plugin MariaDB defaults to through 11.x, and one this driver accepts.
pub const MYSQL_NATIVE_PASSWORD: &str = "mysql_native_password";

/// MariaDB's Ed25519 plugin, as the wire spells it.
///
/// Named `client_ed25519` on the wire and `ed25519` in `CREATE USER`, which is
/// the spelling mismatch a reader hits once: the server-side plugin and the
/// client-side one have different names, and this is the client's.
pub const CLIENT_ED25519: &str = "client_ed25519";

/// MariaDB 11.6's Parsec, one of the plugins this driver answers.
pub const PARSEC: &str = "parsec";

/// What this driver claims in the handshake's second capability word — the one
/// MariaDB carved out of MySQL's trailing filler, because its own bits start at
/// 32 and the first word is 32 bits wide.
///
/// One bit: `MARIADB_CLIENT_STMT_BULK_OPERATIONS`, bit 34, which is the
/// server's permission to send `COM_STMT_BULK_EXECUTE` — one command carrying
/// every parameter set of an `executeMany` instead of one execute per set.
/// Claiming it costs a connection nothing and obliges it to nothing: it widens
/// what this client *may* send and changes no packet the server sends back, so
/// it is claimed at the handshake and spent later or not at all. **It is not
/// spent**: [ADR 0067 § 4](/docs/decisions/0067.md) runs `executeMany`
/// as N executions on every driver, because a bulk command ends at a refusal
/// where the loop carries on and cannot take a set that answers with rows. The
/// bit stays claimed because that is the word the *server* answers in too, and
/// the intersection is what tells this driver which server it reached.
///
/// **The bits not here are absences with reasons**, in the shape
/// `crate::mysql`'s `CLIENT_CAPABILITIES` uses for the first word.
/// `MARIADB_CLIENT_PROGRESS` asks the server to interleave progress packets
/// into a result stream, which is a second packet shape on the row path for a
/// report nothing in `Core\Db` surfaces. `MARIADB_CLIENT_EXTENDED_METADATA` and
/// `MARIADB_CLIENT_CACHE_METADATA` are decisions about column metadata, and
/// [ADR 0067 § 9](/docs/decisions/0067.md)'s type map is read off the
/// metadata MySQL's own framing already carries — a driver that asked for a
/// wider or a suppressed form would be decoding two layouts to answer one
/// table. `MARIADB_CLIENT_BULK_UNIT_RESULTS` changes what a bulk command
/// answers with, and § 4's `executeMany` reports one sum.
pub(crate) const EXTENDED_CAPABILITIES: MariadbCapabilities =
    MariadbCapabilities::MARIADB_CLIENT_STMT_BULK_OPERATIONS;

/// This module's server: MariaDB's name on a [`crate::ServerError`], MariaDB's
/// [`kind_of`], and the extended word only MariaDB has.
pub(crate) const MARIADB: Backend = Backend {
    name: "mariadb",
    kind_of,
    extended: EXTENDED_CAPABILITIES,
};

/// The [ADR 0067 § 8](/docs/decisions/0067.md) kind a MariaDB error
/// code means.
///
/// **Not [`crate::mysql`]'s table, and the divergence is not decorative.** The
/// two servers agree below 1900, where both inherited MySQL 5.5's numbering,
/// and stop agreeing exactly where each added conditions the other does not
/// have. These rows carry the whole difference:
///
/// - **A `CHECK` violation is `4025`** (`ER_CONSTRAINT_FAILED`), MariaDB
///   10.2.1's. MySQL's `3819` is not a MariaDB code at all, so a driver reusing
///   that table reports every failed `CHECK` on MariaDB as
///   [`DbErrorKind::Other`] — the one an application branches on, read as the
///   one it cannot.
/// - **A statement that ran past its `max_statement_time` is `1969`**
///   (`ER_STATEMENT_TIMEOUT`), where MySQL's `max_execution_time` raises `3024`.
/// - **`1927` is `ER_CONNECTION_KILLED`**, which MariaDB raises where MySQL
///   sends `1053`; both are named, since a proxy in front of either may forward
///   the other's.
///
/// The `SQLSTATE` fallback is MySQL's and is unchanged, for MySQL's reason: the
/// classes MariaDB does fill mean what the standard says they mean, and the
/// integer is what names the rest.
pub(crate) fn kind_of(code: u16, sql_state: &str) -> DbErrorKind {
    match code {
        // `ER_DUP_ENTRY` and the siblings that word the same condition
        // for a write, a unique index and a named key.
        1022 | 1062 | 1169 | 1586 => DbErrorKind::UniqueViolation,
        1216 | 1217 | 1451 | 1452 => DbErrorKind::ForeignKeyViolation,
        1048 | 1263 => DbErrorKind::NotNullViolation,
        // `ER_CONSTRAINT_FAILED` — the row above says why it is not MySQL's.
        4025 => DbErrorKind::CheckViolation,
        // `ER_LOCK_DEADLOCK`: InnoDB rolled this transaction back whole, so
        // § 7's retry re-runs the closure from nothing.
        1213 => DbErrorKind::Deadlock,
        // `ER_LOCK_WAIT_TIMEOUT` is a `Timeout` and not a `Deadlock` here for
        // the reason `crate::mysql::kind_of` argues at length: the transaction
        // is still open and still holding its locks, so re-running the closure
        // would run its earlier statements a second time inside it.
        1205 | 1317 | 1969 => DbErrorKind::Timeout,
        1040 | 1053 | 1152 | 1927 => DbErrorKind::ConnectionLost,
        1044 | 1045 | 1130 | 1142 | 1143 | 1227 | 1370 | 1698 => DbErrorKind::Permission,
        1049 | 1051 | 1054 | 1064 | 1146 => DbErrorKind::Syntax,
        _ => match sql_state.get(..2) {
            Some("08") => DbErrorKind::ConnectionLost,
            Some("28") => DbErrorKind::Permission,
            Some("40") => DbErrorKind::SerializationFailure,
            Some("42") => DbErrorKind::Syntax,
            _ => DbErrorKind::Other,
        },
    }
}

/// The plugin a name selects, or the refusal that name earns.
///
/// [`crate::mysql`]'s gate with MariaDB's roster, and the refusal names every
/// plugin it does answer, so an operator reading it can see which server they
/// are actually talking to: a peer that asked for `caching_sha2_password` is
/// refused here even though the other driver answers it, because a plugin
/// MariaDB does not implement being offered by a server claiming to be MariaDB
/// is the shape of a redirected connection.
///
/// # Errors
///
/// `ConnectionRefused`, naming what was asked for and what this driver answers.
pub(crate) fn plugin_or_refuse(name: &[u8]) -> io::Result<AuthPlugin<'static>> {
    match AuthPlugin::from_bytes(name) {
        AuthPlugin::MysqlNativePassword => Ok(AuthPlugin::MysqlNativePassword),
        AuthPlugin::Ed25519 => Ok(AuthPlugin::Ed25519),
        AuthPlugin::Parsec => Ok(AuthPlugin::Parsec),
        _ => Err(io::Error::new(
            io::ErrorKind::ConnectionRefused,
            format!(
                "the server asked to authenticate with `{}`, and this driver answers \
                 `{MYSQL_NATIVE_PASSWORD}`, `{CLIENT_ED25519}` and `{PARSEC}` only: those \
                 are the three MariaDB plugins that send a proof rather than the password",
                String::from_utf8_lossy(name)
            ),
        )),
    }
}

/// Where one MariaDB server is, and who to be there.
///
/// [`crate::MySqlTarget`]'s shape, field for field, and it is a separate type
/// for the reason the driver is a separate driver: a target is what a resolver
/// hands a `connect`, and a `[db.main] driver = "mysql"` block resolving into
/// something this module accepts is the one mistake the type system can catch
/// for free.
pub struct MariaTarget<'a> {
    /// The name the server's certificate is checked against.
    pub host: &'a str,
    /// The user to log in as, accepting `tainted` per `rule:core-classes/db-capabilities`.
    pub user: &'a str,
    /// The user's password, used only to derive a challenge response.
    pub password: &'a str,
    /// The schema to select on connect.
    pub database: &'a str,
    /// The PEM bundle whose anchors this server's certificate is verified
    /// against, or the compiled-in Mozilla set where the block names none.
    ///
    /// Not a way to turn verification off — `rule:core-classes/db-capabilities` has no spelling for
    /// that.
    pub tls_ca_file: Option<&'a Path>,
    /// `rule:core-classes/db-column-types`'s declared zone, as a whole number of seconds east of UTC.
    pub time_zone: i32,
    /// `rule:core-classes/db-one-api`'s `statement_cache`, read off the block by the same reader
    /// both other drivers go through.
    pub statement_cache: usize,
}

impl<'a> MariaTarget<'a> {
    /// One `[db.<name>]` block as this driver's target, or why it is not one.
    ///
    /// [`crate::MySqlTarget::resolve`]'s twin and deliberately its mirror
    /// image: the checks run in the same order, refuse with the same
    /// [`BlockError`] vocabulary, and differ only in which `driver` spelling
    /// they accept. A MySQL block reaching here is refused by
    /// [`BlockError::OtherDriver`] rather than opened, which is the same
    /// sentence that resolver says about a MariaDB one.
    ///
    /// # Errors
    ///
    /// [`BlockError`], in the order the checks run: the `driver`, then a field
    /// belonging to another driver, then the fields the handshake sends, then
    /// § 9's zone.
    pub fn resolve(block: &'a Database) -> Result<MariaTarget<'a>, BlockError<'a>> {
        let written = block.driver.as_deref().ok_or(BlockError::NoDriver)?;
        match Driver::from_config_name(written) {
            Some(Driver::MariaDb) => {}
            Some(driver) => {
                return Err(BlockError::OtherDriver {
                    written,
                    driver,
                    expected: Driver::MariaDb,
                });
            }
            None => return Err(BlockError::UnknownDriver { written }),
        }

        if block.path.is_some() {
            return Err(BlockError::Unusable {
                field: "path",
                expected: Driver::MariaDb,
            });
        }

        let password = match (block.password.as_deref(), block.password_file.is_some()) {
            // `nvs_config::secret` materializes the file's content into
            // `password` and leaves `password_file` set, so a block with the
            // file and no value is one that never went through that pass.
            (None, true) => return Err(BlockError::SecretUnread),
            (Some(""), _) => return Err(BlockError::Blank { field: "password" }),
            (Some(password), _) => password,
            (None, false) => {
                return Err(BlockError::Missing {
                    field: "password",
                    expected: Driver::MariaDb,
                });
            }
        };

        let host = written_value(block.host.as_deref(), "host", Driver::MariaDb)?;
        let user = written_value(block.user.as_deref(), "user", Driver::MariaDb)?;
        // `CLIENT_CONNECT_WITH_DB` is negotiated, so the schema goes out in the
        // handshake response rather than in a `USE` afterwards.
        let database = written_value(block.database.as_deref(), "database", Driver::MariaDb)?;

        let Some(time_zone) = time_zone_for(block) else {
            return Err(BlockError::TimeZone {
                written: block.time_zone.as_deref().unwrap_or_default(),
            });
        };

        Ok(MariaTarget {
            host,
            user,
            password,
            database,
            // Absolute and trust-checked by `nvs_config::db` before the block
            // reached here.
            tls_ca_file: block.tls_ca_file.as_deref().map(Path::new),
            time_zone,
            statement_cache: statement_cache_for(block),
        })
    }
}

impl std::fmt::Debug for MariaTarget<'_> {
    /// Everything but the password, which is a `secret` at the language level
    /// (`rule:core-classes/db-capabilities`) and is not printed in any rendering.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MariaTarget")
            .field("host", &self.host)
            .field("user", &self.user)
            .field("database", &self.database)
            .field("time_zone", &self.time_zone)
            .field("statement_cache", &self.statement_cache)
            .finish_non_exhaustive()
    }
}

impl MariaConn {
    /// Opens a verified, authenticated connection to one MariaDB server.
    ///
    /// [`crate::MySqlConn::connect`]'s sequence — that method's doc and
    /// [`crate::mysql`]'s module doc own every step of it, including which
    /// transport an [`Endpoint`] names and why the greeting over a socket asks
    /// for less — with two things substituted: the wire is framed
    /// [`on`](Wire::on) [`MARIADB`], so a refusal anywhere in the handshake
    /// carries this module's [`kind_of`] and says `mariadb`; and the auth loop
    /// runs with [`plugin_or_refuse`]'s roster, which is where MariaDB's
    /// `client_ed25519` becomes answerable and MySQL's `caching_sha2_password`
    /// becomes a refusal.
    ///
    /// A socket endpoint is **opened as written**, as MySQL's is:
    /// `rule:core-classes/db-unix-socket-path` puts both drivers on the socket
    /// *file*, which has no naming convention for anything to derive a name
    /// from, so nothing here rewrites the path.
    ///
    /// A TCP endpoint is where to connect and `target.host` is the name the
    /// certificate must be valid for; they are separate because the address was
    /// resolved by whoever checked the `db.connect` capability, and a driver
    /// that resolved the name again would be connecting somewhere nobody
    /// approved.
    ///
    /// # Errors
    ///
    /// `ConnectionRefused` when the server will not upgrade to TLS, offers none
    /// of the capabilities the handshake needs, or names an authentication
    /// plugin this driver does not answer; an `Other` carrying a
    /// [`crate::ServerError`] for the server's own refusal; `InvalidData` for a
    /// packet the protocol does not allow at that point — or for a
    /// `tls_ca_file` that holds no certificate — `TimedOut` when the deadline
    /// passes, and whatever the socket or the TLS handshake itself reported.
    pub fn connect(
        endpoint: impl Into<Endpoint>,
        target: &MariaTarget<'_>,
        deadline: Option<Instant>,
    ) -> io::Result<MariaConn> {
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

                let mut plain = Wire::on(&MARIADB, tcp);
                let greeting = crate::mysql::read_greeting(&mut plain, REQUIRED_CAPABILITIES)?;
                crate::mysql::request_tls(&mut plain, &greeting)?;

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

                let mut plain = Wire::on(&MARIADB, local);
                let mut greeting = crate::mysql::read_greeting(&mut plain, REQUIRED_OVER_A_SOCKET)?;
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
        let capabilities = crate::mysql::authenticate(&mut wire, &login, &greeting)?;
        crate::mysql::set_session_time_zone(&mut wire, capabilities, target.time_zone)?;
        // Lifted for [`crate::PgConn::connect`]'s reason, which is the same on every
        // driver that files the handshake clock on its own socket.
        wire.set_deadline(None);

        Ok(MariaConn {
            wire,
            state: Cell::new(State::Idle),
            capabilities,
            // What the greeting already carried — see the field.
            server_version: greeting.banner,
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

    /// `rule:core-classes/db-one-api`'s round trips for one statement, and the columns its result
    /// set turned out to have.
    ///
    /// The two-line delegation the playbook prescribes, into the *same*
    /// sequencing [`crate::MySqlConn::query`] runs: `COM_STMT_PREPARE` and
    /// `COM_STMT_EXECUTE` are one protocol's commands, and § 1's cache in front
    /// of them is one rule. Where the two drivers will part is MariaDB's own
    /// `RETURNING`, which MySQL does not have and which is not here yet —
    /// `COM_STMT_BULK_EXECUTE` is not a second such place, because
    /// [§ 4](/docs/decisions/0067.md) runs `executeMany` as N
    /// executions on every driver.
    ///
    /// # Errors
    ///
    /// As [`crate::mysql::start_statement`].
    pub fn query(
        &mut self,
        sql: &str,
        params: &[Option<&[u8]>],
    ) -> io::Result<crate::MySqlRows<'_, MyStream>> {
        crate::mysql::start_statement(
            &mut self.wire,
            &self.state,
            self.capabilities,
            &mut self.cache,
            sql,
            params,
        )
    }

    /// `rule:core-classes/db-streaming`'s `stream`: one statement left open, its read state
    /// parked here rather than lent out inside a borrow.
    ///
    /// The two-line delegation into the *same* row loop
    /// [`crate::MySqlConn::stream`] runs, which is that member's whole reasoning:
    /// one protocol is framed one way, and a result set is read one way with it.
    ///
    /// # Errors
    ///
    /// As [`crate::mysql::open_result`].
    pub fn stream(&mut self, sql: &str, params: &[Option<&[u8]>]) -> io::Result<&[Column]> {
        let reading = crate::mysql::open_result(
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
    /// that has not streamed since its last reset —
    /// [`crate::MySqlConn::stream_columns`].
    #[must_use]
    pub fn stream_columns(&self) -> Option<&[Column]> {
        Some(self.reading.as_ref()?.columns())
    }

    /// The next row of the parked walk, or `None` once it has ended —
    /// [`crate::MySqlConn::stream_next_row`].
    ///
    /// # Errors
    ///
    /// As [`crate::mysql::next_row_of`].
    pub fn stream_next_row(&mut self) -> io::Result<Option<crate::MySqlRow>> {
        let Some(reading) = self.reading.as_mut() else {
            return Ok(None);
        };
        crate::mysql::next_row_of(&mut self.wire, &self.state, reading)
    }

    /// § 11's trace event for the parked walk —
    /// [`crate::MySqlConn::stream_span`].
    #[must_use]
    pub fn stream_span(&self) -> Option<&crate::QuerySpan> {
        Some(self.reading.as_ref()?.span())
    }

    /// Names the `[db.<name>]` block the parked walk is running on —
    /// [`crate::MySqlConn::name_stream_connection`].
    pub fn name_stream_connection(&mut self, connection: &str) {
        if let Some(reading) = self.reading.as_mut() {
            reading.name_connection(connection);
        }
    }

    /// Abandons the parked walk, draining what is left of it —
    /// [`crate::MySqlConn::end_stream`].
    pub fn end_stream(&mut self) {
        crate::mysql::end_stream_of(&mut self.wire, &self.state, &mut self.reading);
    }

    /// [ADR 0067 § 4](/docs/decisions/0067.md)'s `executeMany`: one
    /// prepare, N executions, and the affected counts summed.
    ///
    /// # Errors
    ///
    /// As [`crate::mysql::execute_many`].
    pub fn execute_many(&mut self, sql: &str, sets: &[&[Option<&[u8]>]]) -> io::Result<u64> {
        crate::mysql::execute_many(
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
    /// # Errors
    ///
    /// As [`crate::mysql::begin`].
    pub fn begin(
        &mut self,
        isolation: Option<crate::Isolation>,
        read_only: bool,
    ) -> io::Result<crate::QuerySpan> {
        crate::mysql::begin(
            &mut self.wire,
            &self.state,
            self.capabilities,
            &self.depth,
            isolation,
            read_only,
        )
    }

    /// How many transaction levels are open — [`crate::MySqlConn::depth`] owns
    /// why § 7 needs this to be public.
    #[must_use]
    pub fn depth(&self) -> u32 {
        self.depth.get()
    }

    /// § 7's `COMMIT`, or the `RELEASE SAVEPOINT` closing a nested one.
    ///
    /// # Errors
    ///
    /// As [`crate::mysql::commit`].
    pub fn commit(&mut self) -> io::Result<crate::QuerySpan> {
        crate::mysql::commit(&mut self.wire, &self.state, self.capabilities, &self.depth)
    }

    /// § 7's `ROLLBACK`, or the `ROLLBACK TO SAVEPOINT` undoing a nested one.
    ///
    /// # Errors
    ///
    /// As [`crate::mysql::roll_back`].
    pub fn roll_back(&mut self) -> io::Result<crate::QuerySpan> {
        crate::mysql::roll_back(&mut self.wire, &self.state, self.capabilities, &self.depth)
    }

    /// [ADR 0067 § 13](/docs/decisions/0067.md)'s reset, before this
    /// connection may be handed to another request.
    ///
    /// `COM_RESET_CONNECTION` on this server too, and it takes `self` by value
    /// for [`crate::MySqlConn::reset`]'s reason: a reset that failed must not be
    /// able to hand a connection back.
    ///
    /// # Errors
    ///
    /// As [`crate::mysql::reset_session`]. The connection is consumed either
    /// way.
    pub fn reset(mut self) -> io::Result<MariaConn> {
        // A walk the program abandoned is read to its end before the reset goes
        // out — [`crate::MySqlConn::reset`] owns why a command written over rows
        // that are still arriving is one nothing can frame the answer to.
        crate::mysql::end_stream_of(&mut self.wire, &self.state, &mut self.reading);
        crate::mysql::reset_session(
            &mut self.wire,
            self.capabilities,
            self.time_zone,
            &mut self.cache,
        )?;
        // `COM_RESET_CONNECTION` rolls back whatever transaction was open, so
        // every level this count named is gone with it.
        self.depth.set(0);
        Ok(self)
    }
}

impl Drop for MariaConn {
    /// Says goodbye rather than vanishing — `crate::mysql`'s `say_goodbye` has
    /// the reasoning, and `COM_QUIT` is the same byte on this server.
    fn drop(&mut self) {
        crate::mysql::say_goodbye(&mut self.wire);
    }
}

#[cfg(test)]
mod tests {
    use mysql_common::auth::plugins::{
        AuthProc, ChallengeResponsePlugin, Context, Error as PluginError,
    };
    use mysql_common::packets::AuthPlugin;

    use super::{
        CLIENT_ED25519, DbErrorKind, MYSQL_NATIVE_PASSWORD, PARSEC, kind_of, plugin_or_refuse,
    };

    /// A 32-byte challenge, which is `client_ed25519`'s fixed nonce width — a
    /// shorter one is refused by the plugin itself and would prove nothing
    /// about whether it is implemented.
    const NONCE: &[u8; 32] = b"Ll2yA*qU(0>Sd?bW#tE!7kZ}9vN^1cX{";
    const PASSWORD: &str = "correct-horse-battery";

    /// What a plugin is allowed to know, as the exchange gives it —
    /// `crate::mysql`'s `AuthContext` for a caller outside that module.
    #[derive(Clone, Copy)]
    struct TestContext;

    impl Context for TestContext {
        fn pass(&self) -> &[u8] {
            PASSWORD.as_bytes()
        }

        fn is_ipc_transport(&self) -> bool {
            false
        }

        fn is_tls_transport(&self) -> bool {
            true
        }

        fn scramble(&self) -> &[u8] {
            NONCE
        }

        fn server_key_pem(&self) -> Option<&[u8]> {
            None
        }
    }

    /// `rule:packaging/a-c-dependency-answers-two-questions`'s standing answer for MariaDB's own plugins, asserted as
    /// the *pair* of outcomes it allows: a plugin this driver names is either
    /// implemented in Rust or refused by name, and never accepted-then-broken.
    ///
    /// The state in between is the one worth a test, because it is the one that
    /// looks fine: `mysql_common` knows `client_ed25519` and `parsec` whether
    /// or not its features for them are on, so an accepting gate over a
    /// feature that is off reaches a real handshake and fails there with "`…`
    /// feature must be enabled for this plugin to work" — a Cargo diagnostic
    /// delivered to an operator whose MariaDB is merely configured the way
    /// MariaDB documents. Asserting that no accepted plugin answers
    /// [`PluginError::FeatureRequired`] is what holds
    /// [`Cargo.toml`](/Cargo.toml)'s feature names in place.
    #[test]
    fn the_mariadb_auth_plugins_are_implemented_in_rust_or_refused_by_name() {
        for named in [MYSQL_NATIVE_PASSWORD, CLIENT_ED25519, PARSEC] {
            let plugin = plugin_or_refuse(named.as_bytes())
                .unwrap_or_else(|e| panic!("`{named}` is one of MariaDB's own: {e}"));
            let mut exchange = AuthProc::init(&plugin).expect("the plugin initializes");
            if let Err(error) = exchange.run(TestContext, NONCE) {
                assert!(
                    !matches!(error, PluginError::FeatureRequired(_)),
                    "`{named}` is accepted by the gate but not compiled in: {error}"
                );
            }
        }

        // `client_ed25519`'s answer is a signature over the nonce, so the
        // implementation is asserted by what it produced rather than only by
        // the absence of a complaint.
        let mut ed25519 = AuthProc::init(&AuthPlugin::Ed25519).expect("the plugin initializes");
        let signed = ed25519
            .run(TestContext, NONCE)
            .expect("`client_ed25519` signs the nonce");
        assert_eq!(
            signed.data().map(<[u8]>::len),
            Some(64),
            "an Ed25519 signature is 64 bytes, and nothing else is a proof"
        );
        assert!(
            signed.data() != Some(PASSWORD.as_bytes()),
            "what leaves this process is a proof and never the password"
        );

        // The roster is closed, and `caching_sha2_password` is in this list on
        // purpose: MySQL's default is not MariaDB's plugin, and a server
        // claiming to be MariaDB while asking for it is answered by a refusal
        // rather than by the other driver's roster.
        for refused in [
            "caching_sha2_password",
            "sha256_password",
            "mysql_clear_password",
            "mysql_old_password",
            "auth_gssapi_client",
        ] {
            let error = plugin_or_refuse(refused.as_bytes())
                .expect_err("a plugin outside the roster is refused");
            assert_eq!(error.kind(), std::io::ErrorKind::ConnectionRefused);
            assert!(
                error.to_string().contains(refused),
                "the refusal names what was asked for: {error}"
            );
        }
    }

    /// § 8's table is MariaDB's, asserted where it diverges from MySQL's rather
    /// than where the two agree.
    ///
    /// The codes below are the whole divergence, and each is a condition
    /// an application branches on: read against `crate::mysql`'s table they all
    /// answer [`DbErrorKind::Other`], which is the kind meaning "read
    /// `driverCode` yourself". A `CHECK` violation reported as
    /// un-normalisable is the failure this pins.
    ///
    /// The other half of the same rule is `crate::conn`'s
    /// `every_driver_normalises_its_codes_to_one_error_kind`, which asks each
    /// driver for the *same condition* and asserts the answers agree —
    /// this case is where they are allowed to be spelled differently, that one
    /// is where they may not mean differently.
    #[test]
    fn mariadb_uses_its_own_code_table_and_not_mysqls() {
        assert_eq!(kind_of(4025, "23000"), DbErrorKind::CheckViolation);
        assert_eq!(kind_of(1969, "70100"), DbErrorKind::Timeout);
        assert_eq!(kind_of(1927, "70100"), DbErrorKind::ConnectionLost);

        // MySQL's own, which MariaDB does not raise: unnamed here, and an
        // application reads `driverCode` for them.
        assert_eq!(kind_of(3819, "HY000"), DbErrorKind::Other);
        assert_eq!(kind_of(3024, "HY000"), DbErrorKind::Other);

        // Everything below 1900 is shared, and the fallback is the standard's.
        assert_eq!(kind_of(1062, "23000"), DbErrorKind::UniqueViolation);
        assert_eq!(kind_of(1213, "40001"), DbErrorKind::Deadlock);
        assert_eq!(kind_of(9999, "40003"), DbErrorKind::SerializationFailure);
    }
}
