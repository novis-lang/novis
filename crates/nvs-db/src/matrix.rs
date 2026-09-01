//! Where this crate's own tests find a real server: the `NVS_DB_MATRIX_*`
//! fields `tools/db-matrix.py` sets, read in one place.
//!
//! ADR 0067's *Verification* section asks for a five-driver matrix against real
//! servers rather than mocks. `tools/db-matrix.py` brings them up from
//! `tests/db/compose.yaml`, reads the ports and credentials back out of that
//! file — so it holds no copy of either — and runs `cargo test -p nvs-db` once
//! per driver with the fields below in the environment. One driver per process,
//! deliberately: a single run with five endpoints would report "3 failed" and
//! leave which server broke to a reader of the output.
//!
//! **The contract is discrete fields and never a DSN.** ADR 0067 § 2 makes
//! `Db\Settings` five types rather than one loose shape, and Novis has no
//! connection string anywhere in its surface; a harness that invented one would
//! be the first place a DSN *parser* had to exist, and this crate would then be
//! tested through a spelling no program can use. Nothing here carries a default
//! for a host, a port or a credential either — `tests/db/compose.yaml` is the
//! one home for those, and a default here would be a second one that is wrong
//! the first time someone edits the compose file.
//!
//! **A case that finds `NVS_DB_MATRIX_DRIVER` unset returns without asserting
//! anything.** That is what keeps `python tools/verify.py` green on a machine
//! with no containers, and it is this crate's rule rather than the harness's:
//! [`endpoint`] answers `None` and the case returns. A field that is *set but
//! unusable* is the opposite case and panics, because the harness sets all of
//! them together — skipping there would report green for a run that never
//! happened, which is the one outcome a verification matrix must not produce.

use std::path::PathBuf;

use crate::conn::Driver;

/// The environment field naming which driver this process is testing.
///
/// Its absence is the whole of the skip decision, so it is named once here and
/// read nowhere else.
pub const DRIVER_VAR: &str = "NVS_DB_MATRIX_DRIVER";

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
}

/// Where a driver's database is.
///
/// Two shapes rather than one struct with unused fields, so a SQLite case
/// cannot read a port that means nothing and a PostgreSQL case cannot read a
/// path that was never set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Location {
    /// A published TCP server — every driver but SQLite.
    Server(Server),
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

    let location = if driver == Driver::Sqlite {
        Location::File(PathBuf::from(field("PATH")))
    } else {
        let port = field("PORT");
        Location::Server(Server {
            host: field("HOST"),
            port: port
                .parse()
                .unwrap_or_else(|e| panic!("NVS_DB_MATRIX_PORT={port} is not a port: {e}")),
            user: field("USER"),
            password: field("PASSWORD"),
            database: field("DATABASE"),
        })
    };

    Some(Endpoint { driver, location })
}

#[cfg(test)]
mod tests {
    use super::{Endpoint, Location, endpoint_from};
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

    /// A network driver reads all five server fields, and the port is a number
    /// rather than the string the environment carries.
    #[test]
    fn a_network_driver_reads_its_five_server_fields() {
        let found = endpoint_from(env(&[
            ("NVS_DB_MATRIX_DRIVER", "postgres"),
            ("NVS_DB_MATRIX_HOST", "127.0.0.1"),
            ("NVS_DB_MATRIX_PORT", "55432"),
            ("NVS_DB_MATRIX_USER", "novis"),
            ("NVS_DB_MATRIX_PASSWORD", "novis"),
            ("NVS_DB_MATRIX_DATABASE", "novis_test"),
        ]))
        .expect("a named driver is an endpoint");

        assert_eq!(found.driver, Driver::Postgres);
        let Location::Server(server) = found.location else {
            panic!("postgres is reached over a socket")
        };
        assert_eq!(server.port, 55432);
        assert_eq!(server.host, "127.0.0.1");
        assert_eq!(server.database, "novis_test");
    }

    /// SQLite is the driver with no wire, so it reads a path and none of the
    /// server fields — asserted by giving it a full set of them and checking
    /// that what comes back carries only the file.
    #[test]
    fn sqlite_reads_a_path_and_no_server_fields() {
        let found = endpoint_from(env(&[
            ("NVS_DB_MATRIX_DRIVER", "sqlite"),
            ("NVS_DB_MATRIX_PATH", "/tmp/novis-matrix.db"),
            ("NVS_DB_MATRIX_HOST", "127.0.0.1"),
            ("NVS_DB_MATRIX_PORT", "not-a-port"),
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
