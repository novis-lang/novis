//! Where this crate's own tests find a real server: the `NVS_DB_MATRIX_*`
//! fields `tools/db-matrix.py` sets, read in one place.
//!
//! `rule:core-classes/db-one-api`'s *Verification* section asks for a five-driver matrix against real
//! servers rather than mocks. `tools/db-matrix.py` brings them up from
//! `tests/db/compose.yaml`, reads the ports and credentials back out of that
//! file — so it holds no copy of either — and runs `cargo test -p nvs-db` once
//! per driver with the fields below in the environment. One driver per process,
//! deliberately: a single run with five endpoints would report "3 failed" and
//! leave which server broke to a reader of the output.
//!
//! **The contract is discrete fields and never a DSN.** `rule:core-classes/db-connection-is-named` makes
//! `Db\Settings` five types rather than one loose shape, and Novis has no
//! connection string anywhere in its surface; a harness that invented one would
//! be the first place a DSN *parser* had to exist, and this crate would then be
//! tested through a spelling no program can use. Nothing here carries a default
//! for a host, a port or a credential either — `tests/db/compose.yaml` is the
//! one home for those, and a default here would be a second one that is wrong
//! the first time someone edits the compose file.
//!
//! **A network endpoint carries its own trust anchor.** `NVS_DB_MATRIX_CA`
//! names a PEM bundle on this machine, and it belongs to the required group
//! with the host and the port rather than beside it: every server that file
//! publishes is TLS-only, no public root vouches for any of them, and
//! `nvs_host::tls` has no spelling for connecting without verifying. An
//! endpoint with no anchor is therefore not an endpoint a case can reach at
//! all, so reading one here is what makes a misconfigured matrix fail at the
//! handshake rather than at an assertion. The anchor is a container's file
//! rather than the tree's — `tests/db/ca.crt` is a copy of one and is not in
//! git — so the harness exports it per run, and a driver whose server it
//! cannot anchor is reported `n/a` and never run.
//!
//! **A case that finds `NVS_DB_MATRIX_DRIVER` unset returns without asserting
//! anything.** That is what keeps `python tools/verify.py` green on a machine
//! with no containers, and it is this crate's rule rather than the harness's:
//! [`endpoint`] answers `None` and the case returns. A field that is *set but
//! unusable* is the opposite case and panics, because the harness sets all of
//! them together — skipping there would report green for a run that never
//! happened, which is the one outcome a verification matrix must not produce.
//!
//! # Known gaps
//!
//! 1. **The socket leg is asked for and never published.** [`Location`] has
//!    its third arm and `tests/handshake.rs` dials whichever one it is handed,
//!    so the case list is already the one that would run over the transport
//!    `rule:core-classes/db-unix-socket-path` gives MySQL, MariaDB and
//!    PostgreSQL — but `tools/db-matrix.py` sets no [`SOCKET_VAR`], so nothing
//!    hands it one, and that transport is asserted against listeners this crate
//!    binds itself and against no real server. The property a second transport
//!    has to have is that the driver agrees across both, which is what running
//!    the TCP legs' own case list over `AF_UNIX` would say. It needs a
//!    container's socket directory bind-mounted onto the host by
//!    `tools/db-matrix.py`.
//!    Decided: The whole TCP case list again over the socket — Proves the driver behaves identically on
//!    both transports; roughly doubles those three legs' run time.
//!    — owner: m8-db-queue

use std::path::PathBuf;

use crate::conn::Driver;

/// The environment field naming which driver this process is testing.
///
/// Its absence is the whole of the skip decision, so it is named once here and
/// read nowhere else.
pub const DRIVER_VAR: &str = "NVS_DB_MATRIX_DRIVER";

/// The environment field carrying a Unix-domain socket.
///
/// Its absence is the whole of the choice between the two transports — the
/// harness sets it for a leg reached over `AF_UNIX` and leaves it unset for the
/// published port — so it is named once here, as [`DRIVER_VAR`] is.
pub const SOCKET_VAR: &str = "NVS_DB_MATRIX_SOCKET";

/// A server the harness published, for the four drivers that speak over TCP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Server {
    /// `NVS_DB_MATRIX_HOST`.
    pub host: String,
    /// `NVS_DB_MATRIX_PORT`, the port the container published.
    pub port: u16,
    /// `NVS_DB_MATRIX_USER`.
    pub user: String,
    /// `NVS_DB_MATRIX_PASSWORD`.
    pub password: String,
    /// `NVS_DB_MATRIX_DATABASE`.
    pub database: String,
    /// `NVS_DB_MATRIX_CA`: the PEM bundle holding the certificate that vouches
    /// for this server, which is what `NvsTls::over_bundle` is handed. Required
    /// rather than optional, for the reason this module's doc gives.
    pub ca: PathBuf,
}

/// A server the harness published on a Unix-domain socket, for the three
/// drivers `rule:core-classes/db-unix-socket-path` gives that transport to.
///
/// Its own shape rather than a [`Server`] with an empty anchor, by the
/// required-group argument this module's doc makes: nothing vouches for a
/// socket and no handshake over one asks, so there is no `NVS_DB_MATRIX_CA`
/// here at all — where an empty field would be one a case could still read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Socket {
    /// [`SOCKET_VAR`]: the string a deployment writes in its own
    /// `[db.<name>] host`, which `rule:core-classes/db-unix-socket-path` makes
    /// the directory for PostgreSQL and the socket file for MySQL and MariaDB.
    /// Carried as written and a `String` rather than a `PathBuf`, because that
    /// is what it is — the host a driver is handed, opened by that driver and
    /// never by this module.
    pub path: String,
    /// `NVS_DB_MATRIX_PORT`, which PostgreSQL derives `.s.PGSQL.<port>` from
    /// and the other two ignore — exactly as the `socket_endpoint` signature
    /// those drivers share does with it.
    pub port: u16,
    /// `NVS_DB_MATRIX_USER`.
    pub user: String,
    /// `NVS_DB_MATRIX_PASSWORD`.
    pub password: String,
    /// `NVS_DB_MATRIX_DATABASE`.
    pub database: String,
}

/// Where a driver's database is.
///
/// Three shapes rather than one struct with unused fields, so a SQLite case
/// cannot read a port that means nothing, a PostgreSQL case cannot read a path
/// that was never set, and a socket case cannot read an anchor no handshake
/// over that transport will ask for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Location {
    /// A published TCP server — every driver but SQLite.
    Server(Server),
    /// A Unix-domain socket — PostgreSQL, MySQL and MariaDB.
    Socket(Socket),
    /// `NVS_DB_MATRIX_PATH`: a scratch file the harness creates before the run
    /// and removes after it. SQLite only, because it has no wire.
    File(PathBuf),
}

/// One driver, and where the harness put its database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    /// Which backend this process is testing.
    pub driver: Driver,
    /// Where to reach it.
    pub location: Location,
}

/// The endpoint this process was pointed at, or `None` when nothing pointed it
/// anywhere.
///
/// `None` means `NVS_DB_MATRIX_DRIVER` is unset — no harness, no containers,
/// an ordinary `cargo test` — and the caller returns without asserting.
///
/// # Panics
///
/// When `NVS_DB_MATRIX_DRIVER` names no driver, or when it is set and one of
/// the fields that driver needs is missing or unparseable. The harness sets the
/// whole group together, so either of those is a bug in the harness or in the
/// compose file, and reporting it as a skip would report green for a run that
/// never happened.
#[must_use]
pub fn endpoint() -> Option<Endpoint> {
    endpoint_from(|name| std::env::var(name).ok())
}

/// [`endpoint`] over an arbitrary lookup, so the contract can be asserted
/// without writing to this process's environment — which is shared by every
/// test running beside it.
fn endpoint_from(lookup: impl Fn(&str) -> Option<String>) -> Option<Endpoint> {
    let name = lookup(DRIVER_VAR)?;
    let driver = Driver::from_matrix_name(&name)
        .unwrap_or_else(|| panic!("{DRIVER_VAR}={name} names no driver"));

    let field = |suffix: &str| {
        let var = format!("NVS_DB_MATRIX_{suffix}");
        lookup(&var).unwrap_or_else(|| {
            panic!(
                "{var} is unset, but {DRIVER_VAR}={name} asked for {} here",
                driver.matrix_name()
            )
        })
    };

    // One reading for both wire transports, called at the head of an arm so
    // that whichever one is being read reports a missing or unparseable port
    // the same way, and so that a SQLite leg reads no port at all.
    let port_of = || {
        let port = field("PORT");
        port.parse::<u16>()
            .unwrap_or_else(|e| panic!("NVS_DB_MATRIX_PORT={port} is not a port: {e}"))
    };

    let location = if driver == Driver::Sqlite {
        Location::File(PathBuf::from(field("PATH")))
    } else if let Some(path) = lookup(SOCKET_VAR) {
        // Set for exactly the leg reached over `AF_UNIX`, so its presence is
        // the whole of the choice: a host published beside it is left unread.
        // The rest of the group is what a published server reads, minus the
        // anchor — a socket leg authenticates and names a database as one over
        // TCP does.
        assert!(
            driver != Driver::SqlServer,
            "{SOCKET_VAR} is set for {}, which has no socket transport to reach: \
             TDS speaks over TCP alone",
            driver.matrix_name()
        );
        Location::Socket(Socket {
            port: port_of(),
            path,
            user: field("USER"),
            password: field("PASSWORD"),
            database: field("DATABASE"),
        })
    } else {
        let port = port_of();
        Location::Server(Server {
            host: field("HOST"),
            port,
            user: field("USER"),
            password: field("PASSWORD"),
            database: field("DATABASE"),
            ca: PathBuf::from(field("CA")),
        })
    };

    Some(Endpoint { driver, location })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{Endpoint, Location, Socket, endpoint_from};
    use crate::conn::Driver;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let owned: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        move |name| {
            owned
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.clone())
        }
    }

    /// The skip rule, which is the one every case in this crate depends on:
    /// with no driver named, there is no endpoint and the case asserts nothing.
    #[test]
    fn no_driver_named_is_no_endpoint_rather_than_a_failure() {
        assert_eq!(endpoint_from(env(&[])), None);
        // Not a partial set either: the other fields alone do not point anywhere.
        assert_eq!(
            endpoint_from(env(&[("NVS_DB_MATRIX_HOST", "127.0.0.1")])),
            None
        );
    }

    /// A network driver reads all six server fields: the port is a number
    /// rather than the string the environment carries, and the anchor is a
    /// path.
    #[test]
    fn a_network_driver_reads_its_six_server_fields() {
        let found = endpoint_from(env(&[
            ("NVS_DB_MATRIX_DRIVER", "postgres"),
            ("NVS_DB_MATRIX_HOST", "127.0.0.1"),
            ("NVS_DB_MATRIX_PORT", "55432"),
            ("NVS_DB_MATRIX_USER", "novis"),
            ("NVS_DB_MATRIX_PASSWORD", "novis"),
            ("NVS_DB_MATRIX_DATABASE", "novis_test"),
            ("NVS_DB_MATRIX_CA", "/tmp/novis-matrix/ca.crt"),
        ]))
        .expect("a named driver is an endpoint");

        assert_eq!(found.driver, Driver::Postgres);
        let Location::Server(server) = found.location else {
            panic!("postgres is reached over a socket")
        };
        assert_eq!(server.port, 55432);
        assert_eq!(server.host, "127.0.0.1");
        assert_eq!(server.database, "novis_test");
        assert_eq!(server.ca, PathBuf::from("/tmp/novis-matrix/ca.crt"));
    }

    /// The anchor is in the required group and not beside it: a server nothing
    /// vouches for cannot be connected to at all, so a set of fields without
    /// one is the harness being wrong rather than a case with less to assert.
    #[test]
    #[should_panic(expected = "NVS_DB_MATRIX_CA is unset")]
    fn a_network_driver_without_a_trust_anchor_stops_the_run() {
        let _ = endpoint_from(env(&[
            ("NVS_DB_MATRIX_DRIVER", "postgres"),
            ("NVS_DB_MATRIX_HOST", "127.0.0.1"),
            ("NVS_DB_MATRIX_PORT", "55432"),
            ("NVS_DB_MATRIX_USER", "novis"),
            ("NVS_DB_MATRIX_PASSWORD", "novis"),
            ("NVS_DB_MATRIX_DATABASE", "novis_test"),
        ]));
    }

    /// A socket field selects the socket leg, and the anchor is not in that
    /// leg's required group: there is no `NVS_DB_MATRIX_CA` in this
    /// environment at all, which is what stops a published server dead, and a
    /// host beside the socket is left unread rather than preferred to it.
    #[test]
    fn a_socket_field_selects_the_socket_leg_and_needs_no_anchor() {
        let found = endpoint_from(env(&[
            ("NVS_DB_MATRIX_DRIVER", "postgres"),
            ("NVS_DB_MATRIX_SOCKET", "/var/run/postgresql"),
            ("NVS_DB_MATRIX_HOST", "127.0.0.1"),
            ("NVS_DB_MATRIX_PORT", "55432"),
            ("NVS_DB_MATRIX_USER", "novis"),
            ("NVS_DB_MATRIX_PASSWORD", "novis"),
            ("NVS_DB_MATRIX_DATABASE", "novis_test"),
        ]))
        .expect("a named driver is an endpoint");

        assert_eq!(
            found,
            Endpoint {
                driver: Driver::Postgres,
                location: Location::Socket(Socket {
                    path: "/var/run/postgresql".to_string(),
                    port: 55432,
                    user: "novis".to_string(),
                    password: "novis".to_string(),
                    database: "novis_test".to_string(),
                }),
            }
        );
    }

    /// The path is carried as written for each of the three drivers that have
    /// a socket leg, because `rule:core-classes/db-unix-socket-path` keeps
    /// each derivation — a directory for PostgreSQL, the file itself for the
    /// other two — in that driver, and a harness that spelled one of them here
    /// would be the second home for it.
    #[test]
    fn a_socket_path_is_carried_as_written_for_every_driver_that_has_one() {
        for (name, driver, written) in [
            ("postgres", Driver::Postgres, "/var/run/postgresql"),
            ("mysql", Driver::MySql, "/var/run/mysqld/mysqld.sock"),
            ("mariadb", Driver::MariaDb, "/run/mysqld/mysqld.sock"),
        ] {
            let found = endpoint_from(env(&[
                ("NVS_DB_MATRIX_DRIVER", name),
                ("NVS_DB_MATRIX_SOCKET", written),
                ("NVS_DB_MATRIX_PORT", "5432"),
                ("NVS_DB_MATRIX_USER", "novis"),
                ("NVS_DB_MATRIX_PASSWORD", "novis"),
                ("NVS_DB_MATRIX_DATABASE", "novis_test"),
            ]))
            .expect("a named driver is an endpoint");

            assert_eq!(found.driver, driver);
            let Location::Socket(socket) = found.location else {
                panic!("{name} was pointed at a socket")
            };
            assert_eq!(socket.path, written);
            assert_eq!(socket.port, 5432);
        }
    }

    /// SQL Server is the driver that refuses the transport rather than lacking
    /// a spelling for it, so a socket leg scheduled for it is the harness
    /// being wrong and stops the run — a leg that dialled one anyway would
    /// report the driver's own refusal as a failed handshake.
    #[test]
    #[should_panic(expected = "which has no socket transport")]
    fn a_socket_leg_for_sql_server_stops_the_run() {
        let _ = endpoint_from(env(&[
            ("NVS_DB_MATRIX_DRIVER", "mssql"),
            ("NVS_DB_MATRIX_SOCKET", "/var/run/mssql.sock"),
            ("NVS_DB_MATRIX_PORT", "51433"),
        ]));
    }

    /// SQLite is the driver with no wire, so it reads a path and none of the
    /// server fields — not the anchor either, since a file handle has nothing
    /// to verify, and not a socket, since its own path *is* the database.
    /// Asserted by giving it a full set of them and checking that what comes
    /// back carries only the file.
    #[test]
    fn sqlite_reads_a_path_and_no_server_fields() {
        let found = endpoint_from(env(&[
            ("NVS_DB_MATRIX_DRIVER", "sqlite"),
            ("NVS_DB_MATRIX_PATH", "/tmp/novis-matrix.db"),
            ("NVS_DB_MATRIX_HOST", "127.0.0.1"),
            ("NVS_DB_MATRIX_PORT", "not-a-port"),
            ("NVS_DB_MATRIX_SOCKET", "/var/run/mysqld/mysqld.sock"),
            ("NVS_DB_MATRIX_CA", "/tmp/novis-matrix/ca.crt"),
        ]))
        .expect("a named driver is an endpoint");

        assert_eq!(
            found,
            Endpoint {
                driver: Driver::Sqlite,
                location: Location::File("/tmp/novis-matrix.db".into()),
            }
        );
    }

    /// A driver that is named but not one of the five is the harness being
    /// wrong, and it stops the run rather than skipping it.
    #[test]
    #[should_panic(expected = "names no driver")]
    fn an_unknown_driver_stops_the_run() {
        let _ = endpoint_from(env(&[("NVS_DB_MATRIX_DRIVER", "oracle")]));
    }

    /// The same, for a field the harness sets as a group and did not: a skip
    /// here would report green for a run that never happened.
    #[test]
    #[should_panic(expected = "NVS_DB_MATRIX_PORT is unset")]
    fn a_missing_field_stops_the_run_rather_than_skipping_it() {
        let _ = endpoint_from(env(&[("NVS_DB_MATRIX_DRIVER", "mysql")]));
    }
}
