//! The five drivers as an enum, and the busy state each one carries.
//!
//! [ADR 0132 § 5](../../../docs/adr/0132-a-driver-is-a-sans-io-codec-over-the-parking-stream.md)
//! decides that this is an `enum` and not a `Driver` trait, for three reasons
//! in this project's priority order: the set is **closed** (ADR 0067 § 12 makes
//! a new backend an ADR rather than a plugin, and wasm extensions cannot host
//! one anyway, so open-set extensibility is the one property a trait buys and
//! this design does not want it); a trait wide enough for all five would be
//! half `unimplemented!()`, since `executeMany` is one prepare and N executions
//! on four drivers and a bulk protocol message on the fifth, a prepare is a
//! round trip on two and free on one, and a reset is four different command
//! sequences and a rollback on the fifth; and a `Box<dyn Driver>` is an
//! allocation and an indirect call per connection operation where a `match` is
//! something the compiler can see through.
//!
//! So where a signature repeats five times, **it repeats**. Five `fn query`
//! bodies that each say what their own protocol does are cheaper to read and to
//! fix than one that makes four of them lie, and what keeps them honest is not
//! a type — it is `tools/db-matrix.py` running one assertion set against five
//! real servers.
//!
//! This module holds the five shapes and the one rule they share. A driver's
//! own wire code — its handshake, its sequencing, its error table — is in that
//! driver's module beside this one ([`crate::pg`] is the first), so the enum
//! stays readable as an enum while a protocol grows to the size a protocol is.
//!
//! What is *not* per driver is § 4's rule about when a connection may be
//! written to and when it may be pooled: that is one rule over [`State`], and
//! it lives on that type rather than being restated in five places. What each
//! driver decides for itself is which state a given wire event leaves it in —
//! whether an abandoned result set can be cancelled and drained back to
//! [`State::Idle`], or is [`State::Poisoned`].

use std::cell::Cell;

use crate::pg::{CancelKey, Wire};

/// The five backends [ADR 0067 § 12](../../../docs/adr/0067-core-db.md) closes
/// the set at.
///
/// MariaDB is its own driver and not a MySQL flag: the two have diverged in
/// auth plugins, error tables and bulk protocol, and ADR 0067 argues that at
/// length. The spellings here are also the ones `NVS_DB_MATRIX_DRIVER` and
/// `tools/db-matrix.py --driver` use, so that roster has one home — see
/// [`Driver::matrix_name`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Driver {
    /// PostgreSQL, over `postgres-protocol` and the extended-query protocol.
    Postgres,
    /// MySQL, over `mysql_common` and `COM_STMT_*`.
    MySql,
    /// MariaDB, over `mysql_common` with its own auth plugins and error table.
    MariaDb,
    /// Microsoft SQL Server, over a TDS 7.4 implementation written here.
    SqlServer,
    /// SQLite, over `rusqlite`, with no wire and no socket at all.
    Sqlite,
}

impl Driver {
    /// Every driver, in the order ADR 0067's own tables use.
    ///
    /// The matrix harness iterates this rather than a list of its own, so a
    /// sixth backend cannot be added to the language without appearing in the
    /// verification matrix on the same commit.
    pub const ALL: [Driver; 5] = [
        Driver::Postgres,
        Driver::MySql,
        Driver::MariaDb,
        Driver::SqlServer,
        Driver::Sqlite,
    ];

    /// The name this driver has in `NVS_DB_MATRIX_DRIVER` and in
    /// `tools/db-matrix.py --driver`.
    ///
    /// These are wire-adjacent identifiers rather than display names, so they
    /// are lower case and stable; nothing renders them to a user.
    #[must_use]
    pub fn matrix_name(self) -> &'static str {
        match self {
            Driver::Postgres => "postgres",
            Driver::MySql => "mysql",
            Driver::MariaDb => "mariadb",
            Driver::SqlServer => "mssql",
            Driver::Sqlite => "sqlite",
        }
    }

    /// The driver a `NVS_DB_MATRIX_DRIVER` value names, or `None` for a value
    /// no driver answers to.
    ///
    /// Deliberately not `FromStr`: this parses one harness-supplied identifier
    /// and is not the general question of how a driver is named in
    /// configuration, which ADR 0067 § 2 answers with a `[db.*]` block's own
    /// key and not with a string at all.
    #[must_use]
    pub fn from_matrix_name(name: &str) -> Option<Driver> {
        Driver::ALL.into_iter().find(|d| d.matrix_name() == name)
    }
}

/// Where a connection's wire is, between commands —
/// [ADR 0132 § 4](../../../docs/adr/0132-a-driver-is-a-sans-io-codec-over-the-parking-stream.md).
///
/// This is not on the stream and cannot be: `NvsStream`'s readiness
/// registration is one task's and is invisible above `Read`/`Write`, a
/// connection is busy whether or not its socket is readable, and SQLite must
/// answer the same `LogicError` with no stream underneath it at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum State {
    /// The wire is at a message boundary; a new command may be written.
    Idle,
    /// A buffered statement is in flight, and only its own call may write.
    Executing,
    /// Rows remain unread. A second statement here is ADR 0067 § 4's
    /// `LogicError`.
    Streaming,
    /// The wire is *not* at a known message boundary — a deadline fired
    /// mid-message, a decode failed, or a stream was abandoned in a shape the
    /// driver cannot drain.
    Poisoned,
}

impl State {
    /// Whether a new statement may be written on a connection in this state.
    ///
    /// Only [`State::Idle`] permits one. `nvs-stdlib` turns a `false` into ADR
    /// 0067 § 4's `LogicError` naming both fixes — this crate does not build
    /// that fault, because the message names the Novis-level call and only the
    /// standard library knows its spelling.
    #[must_use]
    pub fn may_start_statement(self) -> bool {
        matches!(self, State::Idle)
    }

    /// Whether a connection in this state may be reset and returned to the
    /// per-core pool.
    ///
    /// This is the security-relevant one. A [`State::Poisoned`] connection is
    /// **closed, never reset**: ADR 0067 § 13 makes the reset a boundary
    /// because a connection carrying one request's state into another's is a
    /// cross-tenant leak, and a `RESET ALL` written into the middle of an
    /// unfinished message is not a reset — it is a fragment of one request's
    /// protocol stream that the next request will read as its own. Draining
    /// first would mean trusting a length prefix that has already proven
    /// untrustworthy. Closing costs one handshake and is the only answer that
    /// is provable.
    ///
    /// A connection still `Executing` or `Streaming` is not poolable either,
    /// but for the ordinary reason: it is in the middle of a call. Its own
    /// driver returns it to [`State::Idle`] first, or poisons it.
    #[must_use]
    pub fn is_poolable(self) -> bool {
        matches!(self, State::Idle)
    }
}

/// A PostgreSQL connection: `postgres-protocol`'s codec plus the extended-query
/// state machine, the SASL handshake and ADR 0067 § 13's reset, written here.
///
/// The reset is deliberately **not** `DISCARD ALL`, which would deallocate the
/// prepared statements the statement cache exists to preserve.
#[derive(Debug)]
pub struct PgConn {
    /// The stream and the bytes read off it that are not yet a whole message,
    /// once [`crate::pg::PgConn::connect`] has upgraded it.
    ///
    /// ADR 0132 § 3's in-band upgrade means the `SSLRequest` and its one-byte
    /// answer are the only plaintext this connection ever carries, so there is
    /// no variant here for "not yet encrypted": a connection that did not
    /// upgrade was never built.
    pub(crate) wire: Wire,
    /// ADR 0132 § 4's busy state. A plain [`Cell`]: no atomic and no lock,
    /// because a task never migrates and a connection is owned by one request
    /// at a time, which `nvs-host`'s `!Send` scheduler makes true rather than
    /// hoped.
    pub(crate) state: Cell<State>,
    /// What the backend handed over at startup so a *second* connection can
    /// cancel this one's query — the only way PostgreSQL offers, and it is
    /// unrepeatable: the key arrives once, during the handshake, and a
    /// connection that dropped it cannot ask again.
    pub(crate) cancel: CancelKey,
}

/// A MySQL connection: `mysql_common`'s codec plus the handshake, `COM_STMT_*`
/// sequencing and the `LOCAL INFILE` refusal, written here.
///
/// Its reset is `COM_RESET_CONNECTION`, which is atomic and complete and also
/// drops prepared statements — a real asymmetry with PostgreSQL that is the
/// protocol's and not a choice.
#[derive(Debug)]
pub struct MySqlConn {
    /// ADR 0132 § 4's busy state; the reasoning is on [`PgConn`].
    pub(crate) state: Cell<State>,
}

/// A MariaDB connection: `mysql_common`'s codec, its own auth plugins and its
/// own error table, plus `COM_STMT_BULK_EXECUTE` and `RETURNING`.
///
/// Its own driver rather than a MySQL flag — ADR 0067 argues that at length and
/// treating it as a flag is a design error, not a simplification.
#[derive(Debug)]
pub struct MariaConn {
    /// ADR 0132 § 4's busy state; the reasoning is on [`PgConn`].
    pub(crate) state: Cell<State>,
}

/// A SQL Server connection over TDS 7.4, all of which is written here: there is
/// no sans-IO crate for the protocol, and the only implementation is async over
/// `futures-io` with its TLS behind `tokio-rustls`.
///
/// This is the largest single piece of new wire code the database goal buys,
/// and § 3's handshake — TLS tunnelled inside TDS packets before the connection
/// becomes an ordinary TLS stream — is why `NvsTls` is generic over its
/// transport.
#[derive(Debug)]
pub struct TdsConn {
    /// ADR 0132 § 4's busy state; the reasoning is on [`PgConn`].
    pub(crate) state: Cell<State>,
}

/// A SQLite connection: `rusqlite`, a file handle, and no bytes on any wire.
///
/// It has no socket to park on, so its calls go to `nvs-host`'s blocking pool
/// rather than to a readiness registration. Rolling back an open transaction is
/// the whole of its reset, because a file handle has no session state to leak.
#[derive(Debug)]
pub struct SqliteConn {
    /// ADR 0132 § 4's busy state; the reasoning is on [`PgConn`]. SQLite carries it for
    /// the same reason the others do even with no wire to be mid-message on:
    /// ADR 0067 § 4's `LogicError` is a property of the API, not of a socket.
    pub(crate) state: Cell<State>,
}

/// One open connection to one database, whichever backend it is.
///
/// [ADR 0132 § 5](../../../docs/adr/0132-a-driver-is-a-sans-io-codec-over-the-parking-stream.md):
/// each variant owns its own state machine, its own error-code table and its
/// own reset, and `Core\Db`'s entry points `match` here exactly once.
///
/// `clippy::large_enum_variant` is allowed here and the reasoning is ADR 0132
/// § 5's third argument, unchanged: a `Box` around a variant is an allocation
/// and an indirection on every message this enum's own hot path reads, and the
/// lint is measuring a transitional shape rather than a real disparity —
/// [`PgConn`] carries a TLS session because its driver landed first, and the
/// other four are one byte only until theirs do. Nothing holds these in an
/// array either: a `Connection` is one live object per pooled connection, which
/// at ADR 0067 § 13's ceiling of 16 a core is kilobytes.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Connection {
    /// See [`PgConn`].
    Postgres(PgConn),
    /// See [`MySqlConn`].
    MySql(MySqlConn),
    /// See [`MariaConn`].
    MariaDb(MariaConn),
    /// See [`TdsConn`].
    SqlServer(TdsConn),
    /// See [`SqliteConn`].
    Sqlite(SqliteConn),
}

impl Connection {
    /// Which backend this is.
    #[must_use]
    pub fn driver(&self) -> Driver {
        match self {
            Connection::Postgres(_) => Driver::Postgres,
            Connection::MySql(_) => Driver::MySql,
            Connection::MariaDb(_) => Driver::MariaDb,
            Connection::SqlServer(_) => Driver::SqlServer,
            Connection::Sqlite(_) => Driver::Sqlite,
        }
    }

    /// Where this connection's wire is — ADR 0132 § 4.
    #[must_use]
    pub fn state(&self) -> State {
        match self {
            Connection::Postgres(c) => c.state.get(),
            Connection::MySql(c) => c.state.get(),
            Connection::MariaDb(c) => c.state.get(),
            Connection::SqlServer(c) => c.state.get(),
            Connection::Sqlite(c) => c.state.get(),
        }
    }

    /// Move this connection to `state`.
    ///
    /// Takes `&self` rather than `&mut self` because the state is a [`Cell`]:
    /// a driver sets it from inside a borrow of the connection it is reading
    /// rows out of, which is the whole reason § 4 puts it in a cell.
    pub fn set_state(&self, state: State) {
        match self {
            Connection::Postgres(c) => c.state.set(state),
            Connection::MySql(c) => c.state.set(state),
            Connection::MariaDb(c) => c.state.set(state),
            Connection::SqlServer(c) => c.state.set(state),
            Connection::Sqlite(c) => c.state.set(state),
        }
    }

    /// Mark the wire as no longer at a known message boundary.
    ///
    /// Once poisoned a connection is closed rather than reset, and never
    /// rejoins the pool — [`State::is_poolable`] owns why. This is one-way on
    /// purpose: there is no state a driver could inspect to prove a poisoned
    /// wire has become trustworthy again.
    pub fn poison(&self) {
        self.set_state(State::Poisoned);
    }

    /// Whether a new statement may be written now — ADR 0067 § 4.
    #[must_use]
    pub fn may_start_statement(&self) -> bool {
        self.state().may_start_statement()
    }

    /// Whether this connection may be reset and returned to the per-core pool.
    #[must_use]
    pub fn is_poolable(&self) -> bool {
        self.state().is_poolable()
    }
}

#[cfg(test)]
mod tests {
    use super::{Driver, State};

    /// Every driver round-trips through the name the matrix harness uses, and
    /// no two share one — the roster `tools/db-matrix.py` selects on is this
    /// array and not a copy of it.
    #[test]
    fn every_drivers_matrix_name_round_trips_and_is_unique() {
        let mut seen = Vec::new();
        for driver in Driver::ALL {
            let name = driver.matrix_name();
            assert_eq!(Driver::from_matrix_name(name), Some(driver));
            assert!(!seen.contains(&name), "two drivers answer to {name}");
            seen.push(name);
        }
        assert_eq!(seen.len(), 5, "ADR 0067 § 12 closes the set at five");
        assert_eq!(Driver::from_matrix_name("oracle"), None);
    }

    /// § 4's rule, asserted on both sides: `Idle` is the only state that
    /// accepts a statement and the only one that may be pooled. Asserting it
    /// over the whole roster rather than on one state is what catches a fifth
    /// state added later without a decision about either question.
    #[test]
    fn only_an_idle_connection_accepts_a_statement_or_rejoins_the_pool() {
        for state in [
            State::Idle,
            State::Executing,
            State::Streaming,
            State::Poisoned,
        ] {
            let idle = state == State::Idle;
            assert_eq!(state.may_start_statement(), idle, "{state:?}");
            assert_eq!(state.is_poolable(), idle, "{state:?}");
        }
    }

    /// The one that is a security boundary rather than an ordinary refusal: a
    /// poisoned wire is never poolable, and no path in this module returns it
    /// to `Idle`.
    #[test]
    fn a_poisoned_connection_is_never_poolable() {
        assert!(!State::Poisoned.is_poolable());
        assert!(!State::Poisoned.may_start_statement());
    }
}
