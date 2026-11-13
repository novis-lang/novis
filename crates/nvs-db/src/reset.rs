//! The table of session state a pool reset must return to the configured one
//! (`rule:security/db-pool-reset-is-a-boundary`).
//!
//! The rule states the reset as a property, and this table is how a live test
//! walks that property per driver. Each [`Kind`] is one piece of session state a
//! request can leave behind. [`probe`] answers, for a [`Driver`] and a kind, the
//! statements a request would run to change it and the statement that reads it,
//! or the reason that driver gives a request no way to change it.
//!
//! **No `match` here has a wildcard.** A new driver or a new kind does not
//! compile until every pair has an answer, so a backend added to
//! [`Driver::ALL`] answers this table before it is pooled.
//!
//! `crates/nvs-db/tests/pool_reuse.rs`'s
//! `a_reset_returns_every_kind_of_session_state_to_the_configured_one` reads
//! each kind, runs its change, releases the connection through the pool, draws
//! it again, resets it and reads the first value back. A read is written to
//! return one text cell where the driver allows it, so the test compares
//! strings. A SQLite `PRAGMA` returns an integer, which the test renders.
//! A read may also fail, as PostgreSQL's `lastval()` does in a session that
//! drew no sequence value: the test compares the server's error code then.

use crate::Driver;

/// One piece of session state a request can leave on a connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// The default database or schema an unqualified name resolves in.
    Database,
    /// Who the session talks as: an assumed role or an impersonated user.
    Role,
    /// The character set and collation the session reads and writes text in.
    Charset,
    /// The id the last insert generated, and a sequence's session value.
    GeneratedId,
    /// A user variable, a session setting or a session context value.
    SessionVariables,
}

impl Kind {
    /// Every kind, in the order the rule's property names them.
    pub const ALL: [Kind; 5] = [
        Kind::Database,
        Kind::Role,
        Kind::Charset,
        Kind::GeneratedId,
        Kind::SessionVariables,
    ];
}

/// How a driver answers for one [`Kind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Probe {
    /// A request can change this kind.
    Sql {
        /// The statements that change it, run in order on one connection.
        change: &'static [&'static str],
        /// One statement whose first cell, as text, is the kind's value.
        read: &'static str,
    },
    /// A request can change this kind, and the reset then fails, so the pool
    /// closes the connection. The rule allows this outcome: a connection that
    /// cannot be proven clean is closed.
    Closes {
        /// The statements that change it, run in order on one connection.
        change: &'static [&'static str],
        /// One statement whose first cell, as text, is the kind's value.
        read: &'static str,
        /// Why the reset fails, and why no statement before it is sent.
        why: &'static str,
    },
    /// No statement a request may run changes this kind on this driver, and
    /// the string says why.
    None(&'static str),
}

/// What `driver` answers for `kind`.
#[must_use]
pub fn probe(driver: Driver, kind: Kind) -> Probe {
    match driver {
        Driver::Postgres => postgres(kind),
        Driver::MySql => mysql(kind),
        Driver::MariaDb => mariadb(kind),
        Driver::SqlServer => sql_server(kind),
        Driver::Sqlite => sqlite(kind),
    }
}

/// PostgreSQL's row of the table.
///
/// `pg_monitor` is a role every server since 10 has, and the matrix's account
/// is the image's superuser, which may assume it. `work_mem` is a built-in
/// setting: a custom one survives `RESET ALL` as the empty string its
/// placeholder was born with, so it would report a change no reset can undo.
fn postgres(kind: Kind) -> Probe {
    match kind {
        Kind::Database => Probe::None(
            "a PostgreSQL session cannot change its database: the startup message names it",
        ),
        Kind::Role => Probe::Sql {
            change: &[
                "SET SESSION AUTHORIZATION pg_monitor",
                "SET ROLE pg_monitor",
            ],
            read: "SELECT session_user || ' ' || current_user",
        },
        Kind::Charset => Probe::Sql {
            change: &["SET client_encoding = 'LATIN1'"],
            read: "SELECT current_setting('client_encoding')",
        },
        Kind::GeneratedId => Probe::Sql {
            change: &[
                "CREATE SEQUENCE IF NOT EXISTS novis_reset_sequence",
                "SELECT nextval('novis_reset_sequence')::text",
            ],
            read: "SELECT lastval()::text",
        },
        Kind::SessionVariables => Probe::Sql {
            change: &["SET work_mem = '7MB'"],
            read: "SELECT current_setting('work_mem')",
        },
    }
}

/// The read of the charset kind on MySQL and MariaDB.
///
/// The result is converted to `utf8mb4` so the read returns text even while
/// the changed session's `character_set_connection` is `latin1`.
const MYSQL_CHARSET: &str = "SELECT CONVERT(CONCAT_WS(' ', @@character_set_client, \
                             @@character_set_connection, @@character_set_results, \
                             @@collation_connection) USING utf8mb4)";

/// The change of the charset kind on MySQL and MariaDB. `character_set_results`
/// stays as it is, so the reads that follow still return `utf8mb4` text.
const MYSQL_CHARSET_CHANGE: &[&str] = &["SET character_set_connection = latin1"];

/// The generated id on MySQL and MariaDB: `LAST_INSERT_ID(n)` sets the value
/// the next `LAST_INSERT_ID()` returns, with no table.
const MYSQL_ID_CHANGE: &[&str] = &["SELECT LAST_INSERT_ID(7)"];
const MYSQL_ID: &str = "SELECT CAST(LAST_INSERT_ID() AS CHAR)";

/// A user variable and a session system variable. `CONCAT_WS` skips a `NULL`,
/// so the unchanged session reads the setting alone.
const MYSQL_VARIABLES_CHANGE: &[&str] = &[
    "SET @novis_reset = 'marker'",
    "SET SESSION group_concat_max_len = 4321",
];
const MYSQL_VARIABLES: &str = "SELECT CONCAT_WS(' ', CAST(@novis_reset AS CHAR), \
                               CAST(@@SESSION.group_concat_max_len AS CHAR))";

/// MySQL's row of the table.
///
/// `novis_reset_role` is granted to the matrix's account by
/// `tests/db/mysql-init.sql`.
fn mysql(kind: Kind) -> Probe {
    match kind {
        Kind::Database => Probe::None(
            "every statement a request runs is prepared, and MySQL refuses to prepare `USE` \
             with error 1295. The reset still selects the configured database",
        ),
        Kind::Role => Probe::Sql {
            change: &["SET ROLE novis_reset_role"],
            read: "SELECT CAST(CURRENT_ROLE() AS CHAR)",
        },
        Kind::Charset => Probe::Sql {
            change: MYSQL_CHARSET_CHANGE,
            read: MYSQL_CHARSET,
        },
        Kind::GeneratedId => Probe::Sql {
            change: MYSQL_ID_CHANGE,
            read: MYSQL_ID,
        },
        Kind::SessionVariables => Probe::Sql {
            change: MYSQL_VARIABLES_CHANGE,
            read: MYSQL_VARIABLES,
        },
    }
}

/// MariaDB's row of the table. It shares MySQL's statements, and is its own
/// function so a difference between the two servers has a place to go.
fn mariadb(kind: Kind) -> Probe {
    match kind {
        Kind::Database => Probe::None(
            "MariaDB runs a prepared `USE information_schema` and `DATABASE()` still names \
             the configured database, as the matrix shows. The reset still selects it",
        ),
        Kind::Role => Probe::Sql {
            change: &["SET ROLE novis_reset_role"],
            read: "SELECT CAST(CURRENT_ROLE() AS CHAR)",
        },
        Kind::Charset => Probe::Sql {
            change: MYSQL_CHARSET_CHANGE,
            read: MYSQL_CHARSET,
        },
        Kind::GeneratedId => Probe::Sql {
            change: MYSQL_ID_CHANGE,
            read: MYSQL_ID,
        },
        Kind::SessionVariables => Probe::Sql {
            change: MYSQL_VARIABLES_CHANGE,
            read: MYSQL_VARIABLES,
        },
    }
}

/// SQL Server's row of the table.
///
/// Every statement a request runs here is an `sp_prepexec` call, so a change
/// that lasts only until the end of its own scope is no change at all.
fn sql_server(kind: Kind) -> Probe {
    match kind {
        Kind::Database => Probe::Sql {
            change: &["USE tempdb"],
            read: "SELECT DB_NAME()",
        },
        Kind::Role => Probe::Closes {
            change: &[
                "IF USER_ID('novis_reset_user') IS NULL CREATE USER novis_reset_user WITHOUT LOGIN",
                "EXECUTE AS USER = 'novis_reset_user'",
            ],
            read: "SELECT USER_NAME()",
            why: "`sp_reset_connection` on an impersonated session fails with error 596 and \
                  ends the session. A `REVERT` sent first would cost a round trip on every \
                  release, and undoes one level only, never `WITH NO REVERT`",
        },
        Kind::Charset => Probe::None(
            "SQL Server sends text as UTF-16 and each column names its own collation, \
             so a session has no charset to change",
        ),
        Kind::GeneratedId => Probe::Sql {
            change: &[
                "IF OBJECT_ID('novis_reset_ids') IS NULL \
                 CREATE TABLE novis_reset_ids (id int IDENTITY PRIMARY KEY, n int)",
                "INSERT INTO novis_reset_ids (n) VALUES (1)",
            ],
            read: "SELECT CAST(@@IDENTITY AS varchar(20))",
        },
        Kind::SessionVariables => Probe::Sql {
            change: &[
                "EXEC sp_set_session_context N'novis_reset', N'marker'",
                "SET DATEFIRST 3",
            ],
            read: "SELECT CONCAT(CAST(SESSION_CONTEXT(N'novis_reset') AS nvarchar(20)), \
                   ' ', @@DATEFIRST)",
        },
    }
}

/// SQLite's row of the table. `recursive_triggers` is one of the session rows
/// of [`crate::sqlite`]'s pragma table, so a request may write it.
fn sqlite(kind: Kind) -> Probe {
    match kind {
        Kind::Database => Probe::None(
            "`ATTACH` and `DETACH` are refused when a statement is prepared, \
             so `main` is always the file the block names",
        ),
        Kind::Role => Probe::None("SQLite has no users and no roles"),
        Kind::Charset => Probe::None(
            "the encoding belongs to the file and is fixed when it is created, \
             and `PRAGMA encoding` is allowed with no argument only",
        ),
        Kind::GeneratedId => Probe::Sql {
            change: &[
                "CREATE TABLE IF NOT EXISTS novis_reset_ids (id INTEGER PRIMARY KEY)",
                "INSERT INTO novis_reset_ids DEFAULT VALUES",
            ],
            read: "SELECT CAST(last_insert_rowid() AS TEXT)",
        },
        Kind::SessionVariables => Probe::Sql {
            change: &["PRAGMA recursive_triggers = ON"],
            read: "PRAGMA recursive_triggers",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whether a change and a read name at least one statement each, and none
    /// of them is blank.
    fn statements(change: &[&str], read: &str) -> bool {
        !change.is_empty()
            && change.iter().all(|sql| !sql.trim().is_empty())
            && !read.trim().is_empty()
    }

    /// Every pair of a driver and a kind has statements to run or a reason it
    /// has none, and a pair with neither fails by name.
    #[test]
    fn every_driver_answers_for_every_kind_of_session_state() {
        let mut silent = Vec::new();
        for driver in Driver::ALL {
            for kind in Kind::ALL {
                let answered = match probe(driver, kind) {
                    Probe::Sql { change, read } => statements(change, read),
                    Probe::Closes { change, read, why } => {
                        statements(change, read) && !why.trim().is_empty()
                    }
                    Probe::None(reason) => !reason.trim().is_empty(),
                };
                if !answered {
                    silent.push(format!("{} {kind:?}", driver.matrix_name()));
                }
            }
        }
        assert!(
            silent.is_empty(),
            "these pairs name no SQL and no reason: {silent:?}",
        );
    }
}
