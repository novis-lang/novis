//! `rule:core-classes/queue-storage-is-a-table`'s `[queue]`, as the boot reads it: the `[db.<name>]` it names, the bounds that are
//! finite with nothing configured (`rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`), and every way a block can be written that leaves
//! nothing able to run a job.
//!
//! The refusals are asserted by **counting**, not read off one line: a reader that grew a hole in
//! one of its checks still answers plausibly for the rest. The two `0`s are named together for the
//! opposite reason — `workers = 0` is § 2's enqueue-only deployment and `max_attempts = 0` is a
//! refusal, so a reader that treats "zero" as one question passes either half alone.

use std::collections::BTreeMap;
use std::time::Duration;

use nvs_config::Config;
use nvs_config::queue::{QueueBounds, queue_for};
use nvs_diagnostics::{Diagnostic, SourceMap, code};

/// The connection every `[queue]` below names, since § 2's `connection` has no default.
const MAIN: &str = "[db.main]\ndriver = \"pgsql\"\n";

/// The tree `text` writes, panicking with the refusal's message when it does not parse.
fn tree(text: &str) -> Config {
    let mut sources = SourceMap::new();
    let (_, parsed) = nvs_config::file::parse::<Config>(&mut sources, "nvs.toml", text);
    parsed.unwrap_or_else(|err| panic!("{text}\n-- refused: {}", err.message))
}

/// The bounds `text`'s tree resolves to, panicking when it is refused or writes no `[queue]`.
fn bounds(text: &str) -> QueueBounds {
    queue_for(&tree(text), &BTreeMap::new())
        .unwrap_or_else(|err| panic!("{text}\n-- refused: {}", err.message))
        .unwrap_or_else(|| panic!("{text}\n-- resolved to no queue at all"))
}

/// The refusal `text`'s tree produces, panicking when it is accepted instead.
fn refusal(text: &str) -> Diagnostic {
    match queue_for(&tree(text), &BTreeMap::new()) {
        Err(refused) => refused,
        Ok(_) => panic!("{text}\n-- was accepted, and should not have been"),
    }
}

/// `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`, which § 2's example is written against: a block that configures nothing but the
/// connection still has finite bounds, and they are the ADR's own numbers.
#[test]
fn nothing_configured_is_a_finite_queue() {
    let queue = bounds(&format!("{MAIN}\n[queue]\nconnection = \"main\"\n"));

    assert_eq!(queue.connection, "main");
    assert_eq!(queue.workers, 4);
    assert_eq!(queue.max_attempts, 5);
    assert_eq!(queue.visibility, Duration::from_secs(5 * 60));
}

/// § 2's block as the ADR writes it, every key read back. The durations are asserted in two
/// spellings because `crate::value` is the parser and `5m` must not be a shape this module knows.
// covers: directive:queue.visibility
#[test]
fn every_written_bound_is_the_one_the_queue_holds() {
    let written = bounds(&format!(
        "{MAIN}\n[queue]\nconnection = \"main\"\nworkers = 8\nmax_attempts = 3\n\
         visibility = \"90s\"\n"
    ));

    assert_eq!(written.connection, "main");
    assert_eq!(written.workers, 8);
    assert_eq!(written.max_attempts, 3);
    assert_eq!(written.visibility, Duration::from_secs(90));
    assert_eq!(
        written.visibility,
        bounds(&format!(
            "{MAIN}\n[queue]\nconnection = \"main\"\nvisibility = 90\n"
        ))
        .visibility,
        "and a bare count of seconds is the same duration"
    );
}

/// The block's own absence is how a deployment has no queue, so a tree that writes none resolves to
/// nothing rather than to a queue over a database it never named.
#[test]
fn a_tree_with_no_queue_block_has_no_queue() {
    assert_eq!(
        queue_for(&tree(MAIN), &BTreeMap::new()).expect("a tree with no `[queue]` cannot refuse"),
        None
    );
}

/// Every way a `[queue]` can deserialize and still leave nothing able to run a job, counted: each is
/// refused, and each is `E0617` rather than the value band's `E0601`, because what is wrong with
/// each is the block and not the spelling of one number.
#[test]
fn every_queue_that_cannot_run_a_job_is_refused() {
    let cannot_run = [
        // § 2 has no default database, so a block naming none names nothing.
        "[queue]\nworkers = 4\n",
        // A name the merged tree holds no block for.
        "[queue]\nconnection = \"man\"\n",
        // The one spelling that could be read as a block name rather than as the key having been
        // left out, and a `[db.]` block is not a thing a file can write.
        "[queue]\nconnection = \"\"\n",
        // § 6's attempts are finite, and zero of them is dead-lettered on arrival.
        "[queue]\nconnection = \"main\"\nmax_attempts = 0\n",
        // § 4's lease, expiring as it is taken.
        "[queue]\nconnection = \"main\"\nvisibility = \"0s\"\n",
        // `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`'s `false` removes a ceiling, which is not a meaning a lease has.
        "[queue]\nconnection = \"main\"\nvisibility = false\n",
    ];

    let refused: Vec<Diagnostic> = cannot_run
        .iter()
        .map(|queue| refusal(&format!("{MAIN}\n{queue}")))
        .collect();

    assert_eq!(refused.len(), cannot_run.len());
    for (queue, diagnostic) in cannot_run.iter().zip(&refused) {
        assert_eq!(
            diagnostic.code,
            Some(code::E_BAD_QUEUE),
            "{queue}\n-- refused as `{:?}`",
            diagnostic.code
        );
    }
}

/// The two zeroes, named together: § 2 states `workers = 0` as an instance that enqueues and lets
/// another work the rows, and § 6 has no zero at all. A reader that treats them as one question
/// still reads right on either line alone.
// covers: directive:queue.workers, directive:queue.max_attempts
#[test]
fn zero_workers_is_a_deployment_and_zero_attempts_is_a_refusal() {
    let enqueue_only = bounds(&format!(
        "{MAIN}\n[queue]\nconnection = \"main\"\nworkers = 0\n"
    ));

    assert_eq!(enqueue_only.workers, 0);
    assert_eq!(
        enqueue_only.max_attempts, 5,
        "and an instance that runs nothing still bounds what it enqueues"
    );
    assert_eq!(
        refusal(&format!(
            "{MAIN}\n[queue]\nconnection = \"main\"\nmax_attempts = 0\n"
        ))
        .code,
        Some(code::E_BAD_QUEUE)
    );
}

/// The mistake this module exists to catch is a typo, and a typo is answered by the roster. The
/// refusal therefore names the blocks that do exist, and says so differently for a tree that writes
/// none — where the operator's next move is to add one rather than to correct a letter.
// covers: directive:queue.connection
#[test]
fn an_unknown_connection_is_refused_with_the_blocks_that_exist() {
    let typo = refusal(&format!(
        "{MAIN}[db.reports]\ndriver = \"pgsql\"\n\n[queue]\nconnection = \"man\"\n"
    ));
    let note = typo.notes.join(" ");

    assert!(note.contains("`main`"), "-- said: {note}");
    assert!(note.contains("`reports`"), "-- said: {note}");
    assert!(
        refusal("[queue]\nconnection = \"main\"\n")
            .notes
            .join(" ")
            .contains("no `[db]` block at all"),
        "and a tree with no connection at all says so"
    );
}
