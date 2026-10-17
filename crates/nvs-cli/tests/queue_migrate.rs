//! `nvs queue migrate` against a real database: it creates the queue's two tables, and a second run
//! over them changes nothing (`rule:core-classes/queue-storage-is-a-table`,
//! `rule:core-classes/schema-converges`).
//!
//! Through the built binary, because the command is the whole of what an operator runs: the tree
//! resolved, the block opened and the plan applied. The tables are read back with `nvs schema dump`,
//! which reads the database's own catalog.
//!
//! The database is a scratch SQLite file, or the server `bun nv db-matrix` names in
//! `NVS_DB_MATRIX_*`, which runs this suite once per driver. A socket leg is skipped: it reaches
//! the same server its published leg already asked.

use std::path::Path;
use std::process::{Command, Output};

use nvs_db::Driver;
use nvs_db::matrix::{self, Location};

/// A directory of the case's own, locked to this account so its `nvs.toml` passes
/// `rule:config/ownership-is-the-trust-boundary`'s check.
fn scratch(case: &str) -> nvs_repo::Scratch {
    nvs_repo::scratch_private(case)
}

/// Runs `nvs` with `args` from `dir`, with the data folder `dir/.nvsdata`.
fn nvs(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nvs"))
        .arg("--data")
        .arg(dir.join(".nvsdata"))
        .args(args)
        .current_dir(dir)
        .env_remove("NOVIS_NO_INIT")
        .output()
        .expect("the `nvs` binary this test was built beside runs")
}

/// `value` as a TOML basic string.
fn quoted(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

/// The `[db.jobs]` block for this process's database, and whether that database starts empty.
/// `None` for a socket leg.
fn block(dir: &Path) -> Option<(String, bool)> {
    let Some(endpoint) = matrix::endpoint() else {
        return Some(("driver = \"sqlite\"\npath = \"jobs.db\"\n".to_owned(), true));
    };
    match endpoint.location {
        Location::Socket(_) => None,
        Location::File(_) => {
            assert_eq!(endpoint.driver, Driver::Sqlite);
            // The harness's file is shared by every suite of the leg, so this case makes its own.
            let own = dir.join("jobs.db");
            Some((
                format!(
                    "driver = \"sqlite\"\npath = {}\n",
                    quoted(&own.display().to_string())
                ),
                true,
            ))
        }
        Location::Server(server) => Some((
            format!(
                "driver = {}\nhost = {}\nport = {}\nuser = {}\npassword = {}\ndatabase = {}\n\
                 tls_ca_file = {}\n",
                quoted(endpoint.driver.matrix_name()),
                quoted(&server.host),
                server.port,
                quoted(&server.user),
                quoted(&server.password),
                quoted(&server.database),
                quoted(&server.ca.display().to_string()),
            ),
            // A matrix server outlives one run, so an earlier run may have built the tables.
            false,
        )),
    }
}

/// The first run creates `nvs_jobs` and `nvs_dead_jobs`, and the second finds both and runs no
/// step. What the database holds is the same after either run.
// covers: tools:cli/nvs-queue-migrate
#[test]
fn queue_migrate_creates_both_tables_and_a_second_run_changes_nothing() {
    let dir = scratch("queue-migrate");
    let Some((db, starts_empty)) = block(&dir) else {
        return;
    };
    std::fs::write(
        dir.join("nvs.toml"),
        format!("[db.jobs]\n{db}\n[queue]\nconnection = \"jobs\"\n"),
    )
    .unwrap();

    let first = nvs(&dir, &["queue", "migrate", "nvs.toml"]);
    let said = String::from_utf8_lossy(&first.stdout);
    assert_eq!(
        first.status.code(),
        Some(0),
        "{said}{}",
        String::from_utf8_lossy(&first.stderr)
    );
    if starts_empty {
        for table in ["nvs_jobs", "nvs_dead_jobs"] {
            assert!(
                said.contains(&format!("create table {table}: applied")),
                "the first run creates `{table}`:\n{said}"
            );
        }
    }
    let dump = || {
        let dumped = nvs(
            &dir,
            &["schema", "dump", "--connection", "jobs", "nvs.toml"],
        );
        assert_eq!(
            dumped.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&dumped.stderr)
        );
        String::from_utf8_lossy(&dumped.stdout).into_owned()
    };
    let built = dump();
    for table in ["nvs_jobs", "nvs_dead_jobs"] {
        assert!(
            built.contains(&format!("\"name\": \"{table}\"")),
            "the database has `{table}` after the run:\n{built}"
        );
    }

    let second = nvs(&dir, &["queue", "migrate", "nvs.toml"]);
    let said = String::from_utf8_lossy(&second.stdout);
    assert_eq!(
        second.status.code(),
        Some(0),
        "{said}{}",
        String::from_utf8_lossy(&second.stderr)
    );
    assert!(
        said.contains("applied 0 step(s)"),
        "a second run over the built tables runs nothing:\n{said}"
    );
    assert_eq!(
        dump(),
        built,
        "the second run changed what the database has"
    );
}
