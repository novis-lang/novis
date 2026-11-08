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

/// Runs each of `statements` on the SQLite file at `path`, and returns how many rows the last one
/// gave.
fn on_sqlite_file(path: &Path, statements: &[&str]) -> usize {
    let database = nvs_config::tree::Database {
        driver: Some(String::from("sqlite")),
        path: Some(path.display().to_string()),
        ..nvs_config::tree::Database::default()
    };
    let target = nvs_db::SqliteTarget::resolve(&database).expect("the block resolves to a path");
    let conn = nvs_db::sqlite::open(&target).expect("the scratch database opens");
    let mut given = 0;
    for sql in statements {
        let mut answered = conn
            .query(sql, Vec::new())
            .unwrap_or_else(|refused| panic!("`{sql}` runs: {refused}"));
        given = 0;
        while answered.next_row().is_some() {
            given += 1;
        }
    }
    given
}

/// A queue built before the `token` column existed gets it from `nvs queue migrate`, in both
/// tables, and the rows already there keep a null token. The case builds the tables, drops the
/// column, and migrates again. It runs on a SQLite file of its own, so a shared server is never
/// changed.
// covers: tools:cli/nvs-queue-migrate
#[test]
fn migrate_adds_the_token_column_to_an_existing_jobs_table() {
    let dir = scratch("queue-migrate-token");
    let Some((_, true)) = block(&dir) else {
        return;
    };
    let file = dir.join("jobs.db");
    std::fs::write(
        dir.join("nvs.toml"),
        format!(
            "[db.jobs]\ndriver = \"sqlite\"\npath = {}\n\n[queue]\nconnection = \"jobs\"\n",
            quoted(&file.display().to_string())
        ),
    )
    .unwrap();
    let migrate = || {
        let run = nvs(&dir, &["queue", "migrate", "nvs.toml"]);
        let said = String::from_utf8_lossy(&run.stdout).into_owned();
        assert_eq!(
            run.status.code(),
            Some(0),
            "{said}{}",
            String::from_utf8_lossy(&run.stderr)
        );
        said
    };
    migrate();
    let _ = on_sqlite_file(
        &file,
        &[
            "alter table nvs_jobs drop column token",
            "alter table nvs_dead_jobs drop column token",
            "insert into nvs_jobs (queue, script, state, attempts, max_attempts, backoff_ms, \
             run_at, created_at) values ('mail', '/app/jobs/send.nvs', 0, 0, 3, 1000, 1, 1)",
        ],
    );

    let said = migrate();
    for table in ["nvs_jobs", "nvs_dead_jobs"] {
        assert!(
            said.contains(&format!("add column {table}.token: applied")),
            "the run adds `{table}.token`:\n{said}"
        );
    }
    assert_eq!(
        on_sqlite_file(
            &file,
            &["select id from nvs_jobs where token is null and queue = 'mail'"],
        ),
        1,
        "the row written before the column keeps a null token"
    );
    let again = migrate();
    assert!(
        again.contains("applied 0 step(s)"),
        "a run after the column arrived runs nothing:\n{again}"
    );
}
