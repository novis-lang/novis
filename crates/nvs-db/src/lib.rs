//! The wire half of [ADR 0067](../../../docs/adr/0067-core-db.md)'s `Core\Db`:
//! five drivers, each a borrowed sans-IO codec plus a state machine written
//! here, over [`nvs_host`]'s parking stream.
//!
//! [ADR 0132](../../../docs/adr/0132-a-driver-is-a-sans-io-codec-over-the-parking-stream.md)
//! is this crate's charter and its four decisions are the four things a reader
//! most needs before writing a connection. **A codec is borrowed, a state
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
//! # A connection's busy state — ADR 0132 § 4
//!
//! [ADR 0067 § 4](../../../docs/adr/0067-core-db.md) requires a second
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
//! § 13](../../../docs/adr/0067-core-db.md) makes the reset a boundary because
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
//! ADR 0067's *Verification* section asks for a five-driver matrix against real
//! servers. `tools/db-matrix.py` brings those servers up from
//! `tests/db/compose.yaml` and points **this crate's** assertions at one of
//! them at a time; every assertion is here, and that tool is a harness rather
//! than a test. What it hands over is discrete fields in the environment and
//! never a connection string, because ADR 0067 § 2 makes `Db\Settings` five
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
//! NVS_DB_MATRIX_PATH       sqlite only, a scratch file the harness creates and removes
//! ```
//!
//! **The rule is this crate's, not the harness's: a case that finds
//! `NVS_DB_MATRIX_DRIVER` unset returns without asserting anything**, so
//! `python tools/verify.py` stays green on a machine with no containers.
//! [`matrix::endpoint`] is the one reader of those fields — five test files
//! each parsing the environment their own way is five places for that rule to
//! be got wrong, and the ports and credentials themselves have exactly one home
//! in `tests/db/compose.yaml`, which is why nothing here carries a default for
//! any of them.
//!
//! # What is here, and what is not yet
//!
//! The shared half, which ADR 0132 § 5 keeps as plain functions and data rather
//! than behind the drivers at all: [`Driver`]'s closed roster, [`State`] and
//! [`Connection`]'s `match`-once entry points, and [`matrix`]. Of the five wire
//! implementations, [`pg`] has its opening and its statement path: the socket,
//! § 3's in-band upgrade, the SASL exchange, and the extended-query state
//! machine that walks [`State`]'s four values over one flushed round trip, and
//! ADR 0067 § 13's reset — six commands pipelined into a second round trip, with
//! `PgConn::reset` taking `self` by value so a reset that failed cannot hand a
//! connection back. [`sql`] is the shared half of the statement path, plain
//! data with no wire in it because every driver makes the same two decisions:
//! ADR 0067 § 5's `?`/`:name` rewriter and `inList` expansion over four
//! dialects, holding the bind order a driver cannot recover by counting, and
//! § 1's [`StatementCache`] keyed by SQL text plus that expansion's arity.
//! [`pg`] is the first driver to spend it: a hit drops the `Parse` from the
//! batch, and an eviction's `Close` rides in the batch that replaced it. The
//! pool and its acquire path, and the Novis side of ADR 0067 § 9's type map,
//! are still to come — PostgreSQL first throughout, because its extended
//! protocol pays nothing extra for a prepare and so exercises the design rather
//! than the driver's own quirks.
//!
//! **No driver can yet complete a handshake against `tests/db/compose.yaml`.**
//! Those servers serve a self-signed certificate and `nvs_host::tls` verifies
//! against the compiled-in anchors with no seam for a private one, so the
//! matrix reaches the upgrade and stops there. That is why [`pg`]'s exchange is
//! asserted against a SCRAM server in its own tests rather than only against a
//! container: the anchor seam is ADR 0132 § 3's "future `nvs.toml` anchor
//! bundle", and until it exists a matrix-only test would be a green report for
//! a run that never happened.

pub mod conn;
pub mod matrix;
pub mod pg;
pub mod sql;

pub use conn::{
    Connection, DbErrorKind, Driver, Isolation, MariaConn, MySqlConn, PgConn, ServerError,
    SqliteConn, State, TdsConn,
};
pub use pg::{CancelKey, PgColumn, PgDate, PgRow, PgRows, PgScalar, PgTarget, PgTime};
pub use sql::{Binding, Dialect, Params, Prepared, Source, Statement, StatementCache, rewrite};
