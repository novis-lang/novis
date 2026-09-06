//! `nvs queue migrate` — `rule:core-classes/queue-storage-is-a-table`'s two tables, as the explicit operator command that owns
//! them.
//!
//! § 2 gives the runtime one jobs table and one dead-letter table and has them created from here and
//! nowhere else: DDL is an injection sink and a privileged act
//! (`rule:security/tainted-qualifier`), so the runtime never
//! issues it at boot or from a request, and a queue's schema arrives by an operator running a
//! command rather than by a request being served. This module is that command's front half — the
//! configuration tree resolved, the `[db.<name>]` block proven, the driver read — and the
//! statements are whichever list [`nvs_stdlib::queue::migration`] answers with for that driver,
//! beside the `insert` and the `select` that read the columns they create so that the schema has
//! one home rather than two. **Which dialect a driver gets is that function's answer and not this
//! command's**, so a backend gaining one is an edit there and none here.
//!
//! ## Two halves, and `--dry-run` chooses which
//!
//! Without the flag the command opens the `[db.<name>]` it proved and runs those statements in
//! order, which is the whole of what *migrate* means. With it nothing is opened: the statements go
//! to standard output and the exit status is about the configuration alone.
//!
//! **Applying needs no request and no task, which is why this is a call and not an architecture.**
//! `Core\Db` reaches a connection through `nvs_runtime::Ctx::memoized_connection` because a
//! *request* owns what it opens and must give it back; nothing about [`nvs_db::PgConn`] itself
//! wants one. `nvs_host`'s parking stream blocks the calling thread when there is no core to hand
//! back — `nvs_host::net`'s § *Off a core, it blocks* owns why that is the rule kept rather than
//! bent — so the socket opens on the main thread and the whole cost of the applying half is an
//! `nvs-db` dependency in this crate's manifest. Borrowing `main.rs`'s `run` task is the
//! alternative and buys nothing: a task exists there so `rule:concurrency/a-child-belongs-to-the-calling-task`'s children have a parent, and
//! a migration spawns nothing and shares nothing.
//!
//! **The `db.connect` capability is deliberately not asked.** `rule:core-classes/db-capabilities` grants an
//! *application* permission to open a named block, and it is keyed on the entry file this command
//! does not have; the principal here is the operator who ran it over configuration only a trusted
//! account may write (`rule:config/ownership-is-the-trust-boundary`). A capability bounds the program, so asking it of the
//! operator's own command would be a check that has nothing to check.
//!
//! What it *does* answer offline is everything a mistyped configuration gets wrong, which is the
//! failure mode [`nvs_config::queue`]'s own module doc calls the expensive one: whether a `[queue]`
//! block or a `--connection` names a `[db.<name>]` the merged tree actually holds, and whether that
//! block speaks a driver § 2's schema has a dialect for. A tree that fails either never reaches
//! a database to fail against.
//!
//! **Exit status is the contract**, and each half answers for the work it did: `--dry-run` succeeds
//! when the tree resolves and the connection is real, the applying half when every statement ran on
//! the server. A statement refused mid-way leaves the ones before it applied and exits non-zero
//! naming the label that failed — safe to re-run, because every statement of either list carries
//! `if not exists` for the reason [`nvs_stdlib::queue::MIGRATION_POSTGRES`]'s own doc gives.
//!
//! Cost: `--dry-run` is one pass over the configuration tree and nothing else — no port is touched,
//! so it is safe against a production tree from a machine that cannot reach the server at all.
//! Applying adds one connection, held for that dialect's statements and closed with the process; it opens
//! no pool, because a pool exists to be reused by a second request and this is a command.
//!

use std::net::{SocketAddr, ToSocketAddrs};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use nvs_diagnostics::{Diagnostic, Diagnostics, SourceMap};
use nvs_stdlib::queue::Migration;

use crate::config::{LocalFiles, named_roots, working_directory};
use crate::render_diagnostics;

/// The block's `driver` field as the driver it names, or the refusal an operator sees instead.
///
/// **Separate failures rather than one**, because they are separate mistakes: a `driver` no backend
/// answers to is a typo in a value `rule:core-classes/db-connection-is-named` closes, and a backend
/// Novis knows but § 2's schema has no dialect for is a deployment that is early rather than wrong.
/// Which drivers have a list is [`nvs_stdlib::queue::migration`]'s answer and not this command's —
/// the schema lives beside the statements that read its columns, and so does the roster of dialects
/// it is written in.
fn dialect_of(name: &str, written: Option<&str>) -> Option<(nvs_db::Driver, &'static [Migration])> {
    let Some(written) = written else {
        eprintln!("error: `[db.{name}]` names no `driver`, so it is not openable at all");
        return None;
    };
    let Some(driver) = nvs_db::Driver::from_config_name(written) else {
        eprintln!(
            "error: `[db.{name}]` names the `{written}` driver, which Novis has no backend for"
        );
        eprintln!(
            "note: `rule:core-classes/db-one-api`'s five are {}",
            nvs_db::Driver::ALL
                .iter()
                .map(|one| format!("`{}`", one.matrix_name()))
                .collect::<Vec<_>>()
                .join(", ")
        );
        return None;
    };
    let Some(list) = nvs_stdlib::queue::migration(driver) else {
        eprintln!(
            "error: `[db.{name}]` names the {} driver, and § 2's schema has no dialect for it yet",
            driver.display_name()
        );
        eprintln!(
            "note: that driver runs no statement at all yet — `Core\\Db`'s own known gaps are the \
             list, and the schema follows the driver rather than leading it"
        );
        return None;
    };
    Some((driver, list))
}

/// `nvs queue migrate [--connection <name>] [--dry-run] [<file>...]`.
///
/// The roots are read exactly as [`crate::config::check`] reads them, for the reason that function
/// gives: an audit and a migration are the same question about the same files, so naming a root
/// positionally disables the search for `./nvs.toml` here too.
///
/// `--connection` overrides `[queue] connection` and does not replace it: a tree writing no
/// `[queue]` block at all is a deployment that has not decided where its jobs live, and the flag is
/// how an operator builds the schema for a block *before* the block that names it is written. Either
/// way the name is proven against the merged tree before anything is printed, because printing DDL
/// for a database nobody configured is worse than refusing to.
pub(crate) fn migrate(
    config: &[PathBuf],
    paths: &[PathBuf],
    connection: Option<&str>,
    dry_run: bool,
) -> ExitCode {
    let files = LocalFiles;
    let mut sources = SourceMap::new();
    let named = named_roots(config, paths);
    let resolved = working_directory().and_then(|cwd| {
        let roots = nvs_config::resolve::roots(&named, &cwd, &files);
        nvs_config::resolve::resolve(&roots, &mut sources, &files)
    });
    let resolved = match resolved {
        Ok(resolved) => resolved,
        Err(diagnostic) => return refuse(diagnostic, &mut sources),
    };

    let name = match connection {
        Some(named) => named.to_owned(),
        // No flag, so the block decides — and `queue_for` is the same resolution the runtime does at
        // boot, refusals included, rather than a second reading of the same keys.
        None => match nvs_config::queue::queue_for(&resolved.config, &resolved.origins) {
            Ok(Some(bounds)) => bounds.connection,
            Ok(None) => {
                eprintln!(
                    "error: this tree writes no `[queue]` block, so nothing names the database the \
                     jobs table belongs in"
                );
                eprintln!(
                    "note: write `rule:core-classes/queue-storage-is-a-table`'s block, as `connection = \"main\"`, or name a \
                     `[db.<name>]` here with `--connection <name>`"
                );
                return ExitCode::FAILURE;
            }
            Err(diagnostic) => return refuse(diagnostic, &mut sources),
        },
    };

    let Some(block) = resolved.config.db.get(&name) else {
        // The roster is the whole of the help this refusal can give, and it is nearly always
        // enough: the mistake is a typo or an include that was expected to bring the block in.
        let known = if resolved.config.db.is_empty() {
            "this tree writes no `[db]` block at all".to_owned()
        } else {
            let names: Vec<String> = resolved
                .config
                .db
                .keys()
                .map(|known| format!("`{known}`"))
                .collect();
            format!("the blocks this tree writes are {}", names.join(", "))
        };
        eprintln!("error: no `[db.{name}]` block is in force, so there is nothing to migrate");
        eprintln!("note: {known}");
        return ExitCode::FAILURE;
    };

    let Some((driver, list)) = dialect_of(&name, block.driver.as_deref()) else {
        return ExitCode::FAILURE;
    };

    if dry_run {
        println!(
            "-- `rule:core-classes/queue-storage-is-a-table`'s schema for `[db.{name}]` in {}'s dialect, as {} statements in this \
             order.",
            driver.display_name(),
            list.len()
        );
        for step in list {
            println!("-- {}", step.label);
            println!("{};", step.sql);
        }
        return ExitCode::SUCCESS;
    }

    apply(&name, block, driver, list)
}

/// How long the whole handshake may take.
///
/// Finite because a command that hangs against an unreachable server reports nothing at all, and
/// generous because the server is often a container the operator started moments ago. It is the
/// connection's deadline and not the migration's: the statements themselves are `create table if
/// not exists` against an empty schema, and a server that accepted the handshake runs them in
/// milliseconds or is holding a lock no timeout here should resolve by walking away.
const CONNECT_DEADLINE: Duration = Duration::from_secs(10);

/// One driver's half of [`apply`]: resolve the block as that driver's target, open it, and hand the
/// statements to [`run_all`].
///
/// **A macro because the arms differ in names and not in shape.**
/// `rule:core-classes/db-drivers-are-an-enum`
/// makes the drivers an enum with one `match` per entry point rather than a `Driver` trait, so
/// there is no type parameter to write this as a generic function over — and writing it out per
/// driver would be one body with `Pg`, `MySql` and `Maria` in it plus a copy of every refusal
/// sentence to keep in step. The refusals themselves are already one vocabulary: `BlockError` is
/// shared by every resolver for the reason its own module doc gives.
macro_rules! open_and_apply {
    ($target:ty, $conn:ty, $port:path, $name:expr, $block:expr, $list:expr) => {{
        let target = match <$target>::resolve($block) {
            Ok(target) => target,
            Err(refused) => {
                eprintln!("error: {}", refused.refusal($name));
                return ExitCode::FAILURE;
            }
        };
        let Some(address) = address_of(target.host, $block.port, $port) else {
            eprintln!(
                "error: `[db.{}]` names the host `{}`, which resolves to no address",
                $name, target.host
            );
            return ExitCode::FAILURE;
        };
        let mut conn =
            match <$conn>::connect(address, &target, Some(Instant::now() + CONNECT_DEADLINE)) {
                Ok(conn) => conn,
                Err(err) => {
                    eprintln!("error: `[db.{}]` at {address} did not open: {err}", $name);
                    return ExitCode::FAILURE;
                }
            };
        run_all($name, address, $list, |sql| {
            // DDL answers with no rows, so the loop body never executes — it is the *drain* that
            // matters, and writing it as a loop is what makes that true for a statement that does
            // answer rather than something to remember when one is added.
            let mut rows = conn.query(sql, &[])?;
            while rows.next_row()?.is_some() {}
            Ok(())
        })
    }};
}

/// Opens `block` with the driver it names and runs that dialect's list on it, statement by
/// statement.
///
/// One statement at a time and never one string with several `;` in it: `rule:core-classes/db-one-api`'s driver
/// takes a statement, and a multi-statement text is exactly what that ADR's § 10 refuses on a
/// literal query. Each answer is drained to the end of its stream before the next one starts,
/// because a connection is only usable at a message boundary and an abandoned portal is not one.
fn apply(
    name: &str,
    block: &nvs_config::tree::Database,
    driver: nvs_db::Driver,
    list: &'static [Migration],
) -> ExitCode {
    match driver {
        nvs_db::Driver::Postgres => open_and_apply!(
            nvs_db::PgTarget<'_>,
            nvs_db::PgConn,
            nvs_db::pg::DEFAULT_PORT,
            name,
            block,
            list
        ),
        nvs_db::Driver::MySql => open_and_apply!(
            nvs_db::MySqlTarget<'_>,
            nvs_db::MySqlConn,
            nvs_db::mysql::DEFAULT_PORT,
            name,
            block,
            list
        ),
        nvs_db::Driver::MariaDb => open_and_apply!(
            nvs_db::MariaTarget<'_>,
            nvs_db::MariaConn,
            nvs_db::maria::DEFAULT_PORT,
            name,
            block,
            list
        ),
        // Unreachable: [`dialect_of`] refuses a driver with no list before anything is opened, and
        // those are exactly the drivers with no connection either. Spelled rather than left to a
        // `_` so that a driver *gaining* a list arrives here as a build failure rather than as a
        // refusal that has stopped being true.
        nvs_db::Driver::SqlServer | nvs_db::Driver::Sqlite => {
            eprintln!(
                "error: `[db.{name}]` names the {} driver, which reached the applying half with no \
                 schema to apply — this is a bug",
                driver.display_name()
            );
            ExitCode::FAILURE
        }
    }
}

/// The statements of one list, in order, each run by whatever `run` the caller's driver supplied.
///
/// The prose is here rather than in [`open_and_apply`] because none of it is the driver's: what an
/// operator is told when a statement is refused is the same sentence whichever backend refused it,
/// and the `if not exists` promise it makes is [`nvs_stdlib::queue::MIGRATION_POSTGRES`]'s and its
/// MySQL sibling's alike.
fn run_all(
    name: &str,
    address: SocketAddr,
    list: &'static [Migration],
    mut run: impl FnMut(&str) -> std::io::Result<()>,
) -> ExitCode {
    println!(
        "-- `rule:core-classes/queue-storage-is-a-table`'s schema, applied to `[db.{name}]` at {address}: {} statements.",
        list.len()
    );
    for step in list {
        if let Err(err) = run(step.sql) {
            eprintln!("error: `{}` was refused by the server: {err}", step.label);
            eprintln!(
                "note: the statements before it are applied and every one of them carries `if not \
                 exists`, so re-running this command once the refusal is fixed completes the schema \
                 rather than colliding with what is already there"
            );
            return ExitCode::FAILURE;
        }
        println!("-- {}: applied", step.label);
    }
    ExitCode::SUCCESS
}

/// `host` and `port` as the address the socket is opened to.
///
/// A literal is parsed rather than resolved — including the `[::1]` form a config file may write —
/// so a deployment naming an address never depends on a resolver being reachable. This is
/// `nvs_stdlib::db`'s `address_of` without the capability half, for the reason this module's doc
/// gives; the two are small enough that a shared helper would only move the duplication.
///
/// Shared with [`crate::worker`] within this crate, which resolves the same block for the same
/// reason and is the second reader rather than a second copy.
///
/// **`fallback` is the driver's default port and never this function's**, for the reason
/// `nvs_stdlib::db`'s twin takes one too: 5432 and 3306 are facts about different backends, and a
/// default written here would be PostgreSQL's answer given to a MySQL block that wrote no `port`.
pub(crate) fn address_of(host: &str, port: Option<u16>, fallback: u16) -> Option<SocketAddr> {
    let port = port.unwrap_or(fallback);
    let bare = host
        .strip_prefix('[')
        .and_then(|held| held.strip_suffix(']'))
        .unwrap_or(host);
    if let Ok(literal) = bare.parse::<std::net::IpAddr>() {
        return Some(SocketAddr::new(literal, port));
    }
    (host, port).to_socket_addrs().ok()?.next()
}

/// A configuration refusal, rendered with the line of the file it came from.
///
/// The map is the caller's for [`crate::config::boot_snapshot`]'s reason: a `nvs.toml` diagnostic
/// carries a span into a file nothing else holds.
fn refuse(diagnostic: Diagnostic, sources: &mut SourceMap) -> ExitCode {
    let mut diags = Diagnostics::new();
    diags.report(diagnostic);
    render_diagnostics(&mut diags, sources);
    ExitCode::FAILURE
}
