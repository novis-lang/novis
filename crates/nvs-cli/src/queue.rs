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
//! ## `--dry-run` is the offline half, and today it is the only half
//!
//! Applying the statements needs a connection opened **outside a request**, and this binary has no
//! path to one. `Core\Db` reaches a connection through `nvs_runtime::Ctx::memoized_connection`,
//! which is a field of the request context; `nvs-db` opens over `nvs_host`'s parking stream, which
//! exists only inside a task running on a core. Either route is a slice of real work rather than a
//! call this module forgot to make, and neither is a decision worth guessing at from here — so the
//! command prints exactly what it would run, exits non-zero saying it did not run it, and an
//! operator applies the printed statements with their server's own client meanwhile.
//!
//! What it *does* answer offline is everything a mistyped configuration gets wrong, which is the
//! failure mode [`nvs_config::queue`]'s own module doc calls the expensive one: whether a `[queue]`
//! block or a `--connection` names a `[db.<name>]` the merged tree actually holds, and whether that
//! block speaks a driver these statements are written in. A tree that fails those two never reaches
//! a database to fail against.
//!
//! **Exit status is the contract.** `--dry-run` succeeds when the tree resolves and the connection
//! is real; without it the command always fails, because a migration that did not happen must never
//! look like one that did.
//!
//! Cost: one pass over the configuration tree and nothing else — no connection is opened, no port
//! is touched, and the command is safe to run against a production tree from a machine that cannot
//! reach the server at all.
//!
//! [ADR 0084]: ../../../docs/adr/0084-durable-background-jobs.md

use std::path::PathBuf;
use std::process::ExitCode;

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

    println!(
        "-- ADR 0084 § 2's schema for `[db.{name}]`, as {} statements in this order.",
        nvs_stdlib::queue::MIGRATION.len()
    );
    for step in nvs_stdlib::queue::MIGRATION {
        println!("-- {}", step.label);
        println!("{};", step.sql);
    }

    if dry_run {
        return ExitCode::SUCCESS;
    }
    eprintln!(
        "error: `nvs queue migrate` cannot apply these yet — this binary opens a database \
         connection only from inside a request, and a migration is not one"
    );
    eprintln!(
        "note: the statements above are on standard output; apply them with the server's own \
         client, or re-run with `--dry-run` to print them without this refusal"
    );
    ExitCode::FAILURE
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
