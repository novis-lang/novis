//! `nvs queue migrate` — [ADR 0084] § 2's two tables, as the explicit operator command that owns
//! them.
//!
//! § 2 gives the runtime one jobs table and one dead-letter table and has them created from here and
//! nowhere else: DDL is an injection sink and a privileged act
//! ([ADR 0024](../../../docs/adr/0024-taint-tracking-for-injection-sinks.md)), so the runtime never
//! issues it at boot or from a request, and a queue's schema arrives by an operator running a
//! command rather than by a request being served. This module is that command's front half — the
//! configuration tree resolved, the `[db.<name>]` block proven, the driver checked — and the
//! statements are [`nvs_stdlib::queue::MIGRATION`], which lives beside the `insert` and the `select`
//! that read the columns it creates so that the schema has one home rather than two.
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
//! bent — so the socket opens on the main thread and the whole cost of the applying half was an
//! `nvs-db` dependency in this crate's manifest. Borrowing `main.rs`'s `run` task was the
//! alternative and buys nothing: a task exists there so ADR 0072 § 1's children have a parent, and
//! a migration spawns nothing and shares nothing.
//!
//! **The `db.connect` capability is deliberately not asked.** [ADR 0067] § 3 grants an
//! *application* permission to open a named block, and it is keyed on the entry file this command
//! does not have; the principal here is the operator who ran it over configuration only a trusted
//! account may write ([ADR 0103] § 6). A capability bounds the program, so asking it of the
//! operator's own command would be a check that has nothing to check.
//!
//! What it *does* answer offline is everything a mistyped configuration gets wrong, which is the
//! failure mode [`nvs_config::queue`]'s own module doc calls the expensive one: whether a `[queue]`
//! block or a `--connection` names a `[db.<name>]` the merged tree actually holds, and whether that
//! block speaks a driver these statements are written in. A tree that fails those two never reaches
//! a database to fail against.
//!
//! **Exit status is the contract**, and each half answers for the work it did: `--dry-run` succeeds
//! when the tree resolves and the connection is real, the applying half when every statement ran on
//! the server. A statement refused mid-way leaves the ones before it applied and exits non-zero
//! naming the label that failed — safe to re-run, because every statement carries `if not exists`
//! for the reason [`nvs_stdlib::queue::MIGRATION`]'s own doc gives.
//!
//! Cost: `--dry-run` is one pass over the configuration tree and nothing else — no port is touched,
//! so it is safe against a production tree from a machine that cannot reach the server at all.
//! Applying adds one connection, held for the five statements and closed with the process; it opens
//! no pool, because a pool exists to be reused by a second request and this is a command.
//!
//! [ADR 0067]: ../../../docs/adr/0067-core-db.md
//! [ADR 0084]: ../../../docs/adr/0084-durable-background-jobs.md
//! [ADR 0103]: ../../../docs/adr/0103-configuration-is-a-tree-of-files.md

use std::net::{SocketAddr, ToSocketAddrs};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use nvs_diagnostics::{Diagnostic, Diagnostics, SourceMap};

use crate::config::{LocalFiles, named_roots, working_directory};
use crate::render_diagnostics;

/// The driver [`nvs_stdlib::queue::MIGRATION`] is written in, which is the only one with a statement
/// path at all — that constant's own doc owns why a second backend brings a second list.
const DIALECT: &str = "postgres";

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
                    "note: write ADR 0084 § 2's block, as `connection = \"main\"`, or name a \
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

    match block.driver.as_deref() {
        Some(DIALECT) => {}
        Some(other) => {
            eprintln!(
                "error: `[db.{name}]` names the `{other}` driver, and § 2's statements are \
                 written in {DIALECT}'s dialect"
            );
            eprintln!(
                "note: the other four of ADR 0067's backends spell the identity column and the \
                 partial index differently, and each brings its own statements rather than a \
                 dialect switch inside these"
            );
            return ExitCode::FAILURE;
        }
        None => {
            eprintln!("error: `[db.{name}]` names no `driver`, so it is not openable at all");
            return ExitCode::FAILURE;
        }
    }

    if dry_run {
        println!(
            "-- ADR 0084 § 2's schema for `[db.{name}]`, as {} statements in this order.",
            nvs_stdlib::queue::MIGRATION.len()
        );
        for step in nvs_stdlib::queue::MIGRATION {
            println!("-- {}", step.label);
            println!("{};", step.sql);
        }
        return ExitCode::SUCCESS;
    }

    apply(&name, block)
}

/// How long the whole handshake may take.
///
/// Finite because a command that hangs against an unreachable server reports nothing at all, and
/// generous because the server is often a container the operator started moments ago. It is the
/// connection's deadline and not the migration's: the statements themselves are `create table if
/// not exists` against an empty schema, and a server that accepted the handshake runs them in
/// milliseconds or is holding a lock no timeout here should resolve by walking away.
const CONNECT_DEADLINE: Duration = Duration::from_secs(10);

/// Opens `block` and runs [`nvs_stdlib::queue::MIGRATION`] on it, statement by statement.
///
/// One statement at a time and never one string with four `;` in it: [ADR 0067] § 1's driver takes
/// a statement, and a multi-statement text is exactly what that ADR's § 10 refuses on a literal
/// query. Each answer is drained to the end of its stream before the next one starts, because a
/// connection is only usable at a message boundary and an abandoned portal is not one.
fn apply(name: &str, block: &nvs_config::tree::Database) -> ExitCode {
    let target = match nvs_db::PgTarget::resolve(block) {
        Ok(target) => target,
        Err(refused) => {
            eprintln!("error: {}", refused.refusal(name));
            return ExitCode::FAILURE;
        }
    };
    let address = match address_of(target.host, block.port) {
        Some(address) => address,
        None => {
            eprintln!(
                "error: `[db.{name}]` names the host `{}`, which resolves to no address",
                target.host
            );
            return ExitCode::FAILURE;
        }
    };
    let mut conn =
        match nvs_db::PgConn::connect(address, &target, Some(Instant::now() + CONNECT_DEADLINE)) {
            Ok(conn) => conn,
            Err(err) => {
                eprintln!("error: `[db.{name}]` at {address} did not open: {err}");
                return ExitCode::FAILURE;
            }
        };

    println!(
        "-- ADR 0084 § 2's schema, applied to `[db.{name}]` at {address}: {} statements.",
        nvs_stdlib::queue::MIGRATION.len()
    );
    for step in nvs_stdlib::queue::MIGRATION {
        if let Err(err) = run(&mut conn, step.sql) {
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

/// One statement, run to the end of whatever it answered with.
///
/// DDL answers with no rows, so the loop body never executes — it is the *drain* that matters, and
/// writing it as a loop is what makes that true for a statement that does answer rather than
/// something to remember when one is added.
fn run(conn: &mut nvs_db::PgConn, sql: &str) -> std::io::Result<()> {
    let mut rows = conn.query(sql, &[])?;
    while rows.next_row()?.is_some() {}
    Ok(())
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
pub(crate) fn address_of(host: &str, port: Option<u16>) -> Option<SocketAddr> {
    let port = port.unwrap_or(nvs_db::pg::DEFAULT_PORT);
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
