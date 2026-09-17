//! `nvs queue migrate` — `rule:core-classes/queue-storage-is-a-table`'s two tables, as the explicit operator command that owns
//! them.
//!
//! § 2 gives the runtime one jobs table and one dead-letter table and has them created from here and
//! nowhere else: DDL is an injection sink and a privileged act
//! (`rule:security/tainted-qualifier`), so the runtime never
//! issues it at boot or from a request, and a queue's schema arrives by an operator running a
//! command rather than by a request being served. This module is that command's front half — the
//! configuration tree resolved, the `[db.<name>]` block proven, the driver read — and the schema
//! itself is [`nvs_stdlib::queue::schema`], one value beside the `insert` and the `select` that
//! read the columns it declares. **This command decides nothing about the schema and nothing about
//! its dialect**: the value is emitted by `nvs_db::ddl` for whichever driver the block named, so
//! every backend has one and a backend gaining a *statement* is an edit there and none here.
//!
//! ## Two halves, and `--dry-run` chooses which
//!
//! Without the flag the command opens the `[db.<name>]` it proved and **converges** it onto that
//! value — `rule:core-classes/schema-converges`'s plan, which is the difference between the schema
//! and what the database has. With it nothing is opened: the create-from-nothing statements go to
//! standard output and the exit status is about the configuration alone.
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
//! block names a driver at all. A tree that fails either never reaches a database to fail against.
//!
//! **Exit status is the contract**, and each half answers for the work it did: `--dry-run` succeeds
//! when the tree resolves and the connection is real, the applying half when every step of the plan
//! ran on the server. A step refused mid-way leaves the ones before it applied and exits non-zero
//! naming the change that failed — safe to re-run, because the next plan is computed against the
//! database as it is then rather than against what the last run believed.
//!
//! Cost: `--dry-run` is one pass over the configuration tree and nothing else — no port is touched,
//! so it is safe against a production tree from a machine that cannot reach the server at all.
//! Applying adds one connection, held for the catalog read and the plan and closed with the
//! process; it opens no pool, because a pool exists to be reused by a second request and this is a
//! command.
//!

use std::net::{SocketAddr, ToSocketAddrs};
use std::path::PathBuf;
use std::process::ExitCode;

use nvs_diagnostics::{Diagnostic, Diagnostics, SourceMap};

use crate::config::{LocalFiles, named_roots, working_directory};
use crate::render_diagnostics;

/// The block's `driver` field as the driver it names, or the refusal an operator sees instead.
///
/// **Separate failures rather than one**, because they are separate mistakes: a block writing no
/// `driver` is not openable by anything, and a `driver` no backend answers to is a typo in a value
/// `rule:core-classes/db-connection-is-named` closes. There is no third refusal: § 2's schema is
/// one value and `nvs_db::ddl` emits it in every dialect, so no backend is one this command has to
/// turn away for having no schema written for it.
///
/// `pub(crate)` because the boot in [`crate::serve`] asks it of the same block before it serves a
/// queue at all, and a driver an operator misspelled is the same mistake wherever it is read.
pub(crate) fn dialect_of(name: &str, written: Option<&str>) -> Option<nvs_db::Driver> {
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
    Some(driver)
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
    including_risky: bool,
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

    let Some(driver) = dialect_of(&name, block.driver.as_deref()) else {
        return ExitCode::FAILURE;
    };

    if dry_run {
        // The create-from-nothing reading of the value, which is what a database that does not
        // have the queue would be converged to. Each statement carries its own terminator, so
        // nothing here adds one: what is printed is what a server would be sent.
        let steps = nvs_stdlib::queue::migration(driver);
        println!(
            "-- `rule:core-classes/queue-storage-is-a-table`'s schema for `[db.{name}]` in {}'s dialect, as {} statements in this \
             order.",
            driver.display_name(),
            steps.len()
        );
        for step in steps {
            println!("-- {}", step.label);
            println!("{}", step.sql);
        }
        return ExitCode::SUCCESS;
    }

    apply(config, paths, &name, including_risky)
}

/// The applying half: the `[db.<name>]` opened, and the database converged onto
/// [`nvs_stdlib::queue::schema`].
///
/// **Converged rather than run**, which is `rule:core-classes/schema-converges` and not a
/// refinement of it. `nvs_db::ddl` writes no `if not exists`, so a second `nvs queue migrate` over
/// a schema already built is not a list of no-ops but a plan with nothing in it — and that is the
/// same answer against a half-applied earlier run, against a queue an operator has altered, and
/// against a database this command has never seen. A list of statements could give none of the
/// three.
///
/// **A step that is not `Safe` is refused and named**, and `--including-risky` is the same word
/// `nvs schema apply` takes, said at the same place and for the same reason
/// (`rule:core-classes/schema-apply-capability`). A queue converging onto a database that does not
/// have one is `Safe` throughout, so a refusal here is about a table an older Novis built — and
/// whether to narrow a column of it is a decision about an operator's own data, made with the
/// change the refusal names in front of them.
///
/// The tree is resolved a second time inside [`crate::schema::opened`], which is the price of the
/// two commands sharing one convergence: `nvs schema apply` proves a name against the merged tree
/// the same way, and a second reading of a file this process already read is cheaper than a second
/// implementation of what it means.
///
/// **The trailer is per table and not per step**, because what an operator asked this command is
/// whether the queue's two tables are there — and after a plan that ran, they are, whether it held
/// five steps or none. A run that changed nothing prints the same two lines as the run that built
/// them, which is what makes *converged* the answer rather than *applied five statements*.
fn apply(config: &[PathBuf], paths: &[PathBuf], name: &str, including_risky: bool) -> ExitCode {
    let mut conn = match crate::schema::opened(config, paths, name) {
        Ok(conn) => conn,
        Err(code) => return code,
    };
    let converged = crate::schema::converge(
        &mut conn,
        &nvs_stdlib::queue::schema(),
        name,
        including_risky,
    );
    if converged != ExitCode::SUCCESS {
        return converged;
    }
    for label in ["jobs", "dead_letter"] {
        println!("-- {label}: applied");
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
