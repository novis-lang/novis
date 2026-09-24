//! The wire half of `rule:core-classes/db-one-api`'s `Core\Db`:
//! five drivers, each a borrowed sans-IO codec plus a state machine written
//! here, over [`nvs_host`]'s parking stream.
//!
//! `rule:core-classes/db-drivers-are-an-enum`
//! is this crate's charter and its decisions are what a reader most needs
//! before writing a connection. **A codec is borrowed, a state
//! machine is written** (§ 2): message framing, value encoding and
//! authentication mechanisms are large, fiddly and the same for everyone, and
//! that is exactly what the sans-IO crates are; *sequencing* — what to send
//! next, what a park in the middle of a message means, when a connection is
//! reusable — is where this project's own decisions live, and borrowing it is
//! what would have dragged an async runtime in. There is no `Driver` trait
//! (§ 5): the five are [`Connection`]'s variants and each entry point `match`es
//! once.
//!
//! # Where this sits
//!
//! Below `nvs-stdlib`, never beside it. `Core\Db`'s registry rows, reference
//! cards and helper bodies are in `nvs-stdlib` where every other class's are,
//! and this crate holds no Novis-facing name at all — it does not build the
//! faults it causes, because the message that names the *call* is only sayable
//! where the call's spelling is known. `Cargo.toml` says the same thing to
//! whoever is about to add a dependency; § 1 is the rule.
//!
//! # A connection's busy state — `rule:core-classes/db-connection-busy-state`
//!
//! [ADR 0067 § 4](/docs/decisions/0067.md) requires a second
//! statement on a streaming connection to throw `LogicError`. That state is
//! **not** on the stream: `NvsStream`'s readiness registration is one task's and
//! is deliberately invisible above `Read`/`Write`, a connection is busy whether
//! or not its socket happens to be readable, and SQLite must answer the same
//! `LogicError` with no stream underneath it at all. So it is a [`State`] on
//! each driver's own connection, in a plain [`Cell`](std::cell::Cell) with no
//! atomic and no lock — a task never migrates and a connection is owned by one
//! request at a time, which `nvs-host`'s `!Send` scheduler makes true rather
//! than hoped:
//!
//! - [`State::Idle`] — the wire is at a message boundary; a new command may be
//!   written.
//! - [`State::Executing`] — a buffered statement is in flight, and only its own
//!   call may write.
//! - [`State::Streaming`] — rows remain unread. A second statement here is
//!   § 4's `LogicError`; [`Connection::may_start_statement`] is where that is
//!   decided, and `nvs-stdlib` turns the `false` into the fault naming both
//!   fixes.
//! - [`State::Poisoned`] — the wire is *not* at a known message boundary: a
//!   deadline fired mid-message, a decode failed, or a stream was abandoned in
//!   a shape the driver cannot drain.
//!
//! **A poisoned connection is closed, never reset, and never returned to the
//! pool** — [`Connection::is_poolable`] is that rule and it is the one
//! security-relevant call in this module. [ADR 0067
//! § 13](/docs/decisions/0067.md) makes the reset a boundary because
//! a connection carrying one request's state into another's is a cross-tenant
//! leak; a `RESET ALL` written into the middle of an unfinished message is not
//! a reset, it is a fragment of one request's protocol stream that the *next*
//! request will read as its own. Draining first would mean trusting a length
//! prefix that has already proven untrustworthy.
//!
//! An abandoned result set is the ordinary case of poison and is **not**
//! automatically poison: a driver that can cancel and drain a partial result
//! deterministically — PostgreSQL's `Sync` after closing the portal — returns
//! to [`State::Idle`] and pools. One that cannot marks itself
//! [`State::Poisoned`]. That choice is per driver, in the driver.
//!
//! # `NVS_DB_MATRIX_*`: how this crate's own tests find a server
//!
//! `rule:core-classes/db-one-api`'s *Verification* section asks for a five-driver matrix against real
//! servers. `tools/db-matrix.py` brings those servers up from
//! `tests/db/compose.yaml` and points **this crate's** assertions at one of
//! them at a time; every assertion is here, and that tool is a harness rather
//! than a test. What it hands over is discrete fields in the environment and
//! never a connection string, because `rule:core-classes/db-connection-is-named` makes `Db\Settings` five
//! types rather than one loose shape and Novis has no DSN anywhere in its
//! surface — a harness that invented one would be the first place a DSN
//! *parser* had to exist, and this crate would then be tested through a
//! spelling no program can use:
//!
//! ```text
//! NVS_DB_MATRIX_DRIVER     postgres | mysql | mariadb | mssql | sqlite
//! NVS_DB_MATRIX_HOST       127.0.0.1
//! NVS_DB_MATRIX_PORT       the published port
//! NVS_DB_MATRIX_USER
//! NVS_DB_MATRIX_PASSWORD
//! NVS_DB_MATRIX_DATABASE
//! NVS_DB_MATRIX_CA         the PEM bundle vouching for that server, exported per run
//! NVS_DB_MATRIX_PATH       sqlite only, a scratch file the harness creates and removes
//! ```
//!
//! **The rule is this crate's, not the harness's: a case that finds
//! `NVS_DB_MATRIX_DRIVER` unset returns without asserting anything**, so
//! `bun nv verify` stays green on a machine with no containers.
//! [`matrix::endpoint`] is the one reader of those fields — were every test
//! file to parse the environment its own way, that would be one place per file
//! for the rule to be got wrong — and the ports and credentials themselves
//! have exactly one home in `tests/db/compose.yaml`, which is why nothing here
//! carries a default for any of them.
//!
//! # What is here
//!
//! The shared half, which `rule:core-classes/db-drivers-are-an-enum` keeps as plain functions and data rather
//! than behind the drivers at all: [`Driver`]'s closed roster, [`State`] and
//! [`Connection`]'s `match`-once entry points, and [`matrix`]. Of the five
//! drivers, [`pg`] has its opening and its statement path: the socket,
//! § 3's in-band upgrade, the SASL exchange, and the extended-query state
//! machine that walks [`State`]'s values over one flushed round trip, and
//! `rule:security/db-pool-reset-is-a-boundary`'s reset — six commands pipelined into a second round trip, with
//! `PgConn::reset` taking `self` by value so a reset that failed cannot hand a
//! connection back. [`sql`] is the shared half of the statement path, plain
//! data with no wire in it because every driver makes the same two decisions:
//! `rule:core-classes/db-parameters`'s `?`/`:name` rewriter and `inList` expansion over four
//! dialects, holding the bind order a driver cannot recover by counting, and
//! § 1's [`StatementCache`] keyed by SQL text plus that expansion's arity.
//! [`pg`] spends it: a hit drops the `Parse` from the batch, and an
//! eviction's `Close` rides in the batch that replaced it.
//!
//! [`mysql`] is complete from a `[db.<name>]` block to a decoded row:
//! [`MySqlTarget::resolve`] reads the block, then the greeting, § 3's
//! `CLIENT_SSL` upgrade, the authentication exchange over `mysql_common`'s
//! plugins, the forced `utf8mb4` collation, § 9's declared zone as a
//! session variable, § 13's `COM_RESET_CONNECTION`, and § 1's two
//! round trips over `COM_STMT_PREPARE`/`COM_STMT_EXECUTE` with § 9's whole type
//! map decoded off the binary rows. § 1's cache is on it as well, keyed the
//! same way and holding the id the server hands back rather than a name this
//! side mints, so a statement a connection has run before costs one round trip
//! and not two — and § 13's `COM_RESET_CONNECTION` empties it, which is the
//! asymmetry with PostgreSQL that § 13 calls the protocol's. § 13's
//! pool is above this crate — `nvs_runtime::pool` is the store and `nvs-stdlib`
//! the acquire path — so what is here is the halves only a driver can hold:
//! [`Connection::is_poolable`]'s release gate and [`pg`]'s reset, met over a
//! real server in `tests/pool_reuse.rs`. [`maria`] is that same framing
//! under an authentication roster and a § 8 code table of its own
//! — which is the whole of why `rule:core-classes/db-one-api` makes MariaDB a driver rather than a
//! flag.
//!
//! [`tds`] is where SQL Server begins, and it begins one layer below where the
//! others did: TDS 7.4 has no sans-IO crate to borrow, so its packet framing is
//! written here — the eight-byte header, the split a message longer than the
//! negotiated packet size takes, and the reassembly a token stream needs
//! because a token is cut wherever that size lands rather than at a message
//! boundary. Its handshake is whole on top of that — a `[db.<name>]` block,
//! PRELOGIN, § 3's TLS tunnelled inside PRELOGIN packets, LOGIN7 and the tokens
//! that answer it — so [`TdsConn`] holds a live wire. Its statement path is
//! whole on top of *that*: `sp_prepexec` under § 1's cache, the row path over
//! the same reassembly, and § 13's `sp_reset_connection`. [`sqlite`] is the one
//! driver with no wire at all — `rusqlite` is the protocol, reached through § 3's
//! handoff to `nvs-host`'s blocking pool, which is why its rows come back
//! materialized where every other driver's stream. PostgreSQL comes first
//! throughout because its extended protocol pays nothing extra for a prepare and
//! so exercises the design rather than the driver's own quirks. What a driver
//! still owes is recorded in its own module's `# Known gaps`.
//!
//! **The anchor seam `rule:security/one-tls-client` anticipated exists**, so a handshake against
//! `tests/db/compose.yaml`'s PostgreSQL completes: `NvsTls::over_bundle`
//! verifies against a named PEM bundle alone, `[db.<name>] tls_ca_file` is
//! where a program names one, and [`matrix`]'s `NVS_DB_MATRIX_CA` is where
//! this crate's own cases get theirs. **Every server there serves one**, some
//! of them by `tests/db/compose.yaml`'s own arrangement rather than by their
//! images' — that file's block comments own how — so `tools/db-matrix.py` runs
//! every leg. [`pg`]'s exchange is asserted against a scripted SCRAM server as
//! well as against a container: a unit test that needs neither socket nor
//! certificate is the one that keeps failing usefully when the servers are
//! down.

pub mod catalog;
pub mod conn;
pub mod ddl;
pub mod direct;
pub mod maria;
pub mod matrix;
pub mod mysql;
pub mod pg;
pub mod plan;
pub mod schema;
pub mod span;
pub mod sql;
pub mod sqlite;
pub mod tds;

pub use conn::{
    BlockError, ColumnType, Connection, DbErrorKind, Driver, Endpoint, Isolation, MariaConn,
    MySqlConn, PgConn, ServerError, SqliteConn, State, TdsConn, is_socket_host, socket_endpoint,
};
pub use maria::MariaTarget;
pub use mysql::{MySqlDate, MySqlRow, MySqlRows, MySqlScalar, MySqlTarget, MySqlTime, MyStream};
pub use pg::{CancelKey, PgColumn, PgDate, PgRow, PgRows, PgScalar, PgTarget, PgTime, encode};
// `catalog::Read` is deliberately not re-exported here either, and for the
// neighbouring reason: a bare `Read` at the crate root reads as `std::io`'s
// trait rather than as one of `rule:core-classes/schema-introspection`'s two introspection queries. It is
// `catalog::Read`, where the module name says which question it answers.
// `schema::Column` is deliberately not re-exported here: `PgColumn` and
// `SqliteColumn` beside it are *result* columns, and one bare `Column` at the
// crate root would read as the third of those rather than as a line of a
// `CREATE TABLE`. It is `schema::Column`, where its neighbours say which
// question it answers.
pub use plan::{Change, Grade, KeyKind, Plan, Step, diff};
pub use schema::{
    ColumnDefault, FloatWidth, Ident, IntWidth, Key, MAX_IDENTIFIER, Node, ScalarType, Schema,
    SchemaError, Table, is_bare_identifier,
};
pub use span::{QuerySpan, SQL_LIMIT};
pub use sql::{Binding, Dialect, Params, Prepared, Source, Statement, StatementCache, rewrite};
pub use sqlite::{SqliteColumn, SqliteRows, SqliteTarget, SqliteValue};
pub use tds::TdsTarget;
