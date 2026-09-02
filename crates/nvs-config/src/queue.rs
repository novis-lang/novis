//! What a `[queue]` block still owes once it has deserialized: the `[db.<name>]` its rows live in,
//! resolved against the merged tree, and ADR 0084 §§ 2 and 6's bounds read into numbers. Both at
//! boot.
//!
//! **A queue fails silently by construction, which is why every question here is asked at boot.**
//! A job nothing ever claims is indistinguishable from one whose turn has not come round, so an
//! operator who wrote `connection = "man"` learns it from the work that did not happen — days
//! later, from the dead-letter table that never filled because nothing was ever enqueued. That is
//! [`mod@crate::schedule`]'s reasoning one subsystem over, and it is the same trade: one pass over
//! the merged tree at boot against a failure mode with no signal.
//!
//! **`connection` is resolved and not merely read.** § 2 names a `[db.<name>]` block, so the check
//! is that the tree holds one — the same shape [`mod@crate::db`] gives a `tls_ca_file`, minus the
//! filesystem, because a queue names a block and not a path. It runs over the merged tree for that
//! module's reason: which `[db]` blocks exist is a question only the merge has answered, and a
//! per-file check would refuse a base file an include was about to complete.
//!
//! **The trust question is answered by the block it names, not again here.** A queue reaches its
//! database through the connection an operator wrote in root-owned configuration, so its
//! credentials, its CA bundle and its pool are that block's — already canonicalized and
//! trust-checked by [`db::canonicalize`](crate::db::canonicalize) before this pass runs. A second
//! answer would be a second place a deployment could be pointed at a different server.
//!
//! **The bounds are finite with nothing configured**, per
//! [ADR 0074](../../../docs/adr/0074-http-defaults-safe-and-finite.md), and § 6 has no unbounded
//! spelling at all: a job is retried a fixed number of times and then kept, so `max_attempts =
//! false` is not a bound this module can read. `QueueBounds::DEFAULTS` transcribes § 2's own
//! example rather than choosing numbers.
//!
//! Cost: one pass over one optional block at boot and at reload, and four words held per
//! configuration generation. Nothing here runs on a request path.

use std::collections::BTreeMap;
use std::time::Duration;

use nvs_diagnostics::{Diagnostic, code};

use crate::resolve::{Origin, origin_note};
use crate::tree::{Config, Queue, Setting};
use crate::value::{Quantity, Unit};

/// ADR 0084 § 2's `[queue]`, resolved: the connection named, and every bound a number.
///
/// Held by value and cloned per configuration generation rather than borrowed from the tree,
/// because [ADR 0078](../../../docs/adr/0078-config-reload-and-control-socket.md) § 1's reload
/// replaces the tree whole and a worker holding a borrow into the old one would be reading a
/// generation the deployment has moved off.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueueBounds {
    /// The `[db.<name>]` key § 2 names — proven to exist by [`queue_for`], so nothing downstream
    /// re-asks.
    pub connection: String,
    /// Workers this instance runs. `0` is enqueue-only, which § 2 states as a deployment and not as
    /// a disabled queue.
    pub workers: u32,
    /// Attempts a job gets before § 6 moves it to the dead-letter table. Never `0` — a job that may
    /// never be attempted is dead-lettered by the enqueue that created it.
    pub max_attempts: u32,
    /// How long a claimed job stays invisible to other workers (§ 4). Never zero: a lease that
    /// expires as it is taken is every worker claiming every job at once.
    pub visibility: Duration,
}

impl QueueBounds {
    /// § 2's own example, which is this module's default set. The ADR writes the three numbers out,
    /// so they are transcribed here rather than chosen; `connection` has no default because § 2's
    /// whole point is that an operator names the database.
    const DEFAULTS: (u32, u32, Duration) = (4, 5, Duration::from_secs(5 * 60));
}

/// The `[queue]` block resolves, or there is none — the boot half of [`queue_for`].
///
/// It runs over the merged tree for [`crate::db::validate`]'s reason, plus one of its own: the
/// `[db.<name>]` roster this block is checked against accumulates across the file tree
/// ([ADR 0103](../../../docs/adr/0103-configuration-is-a-tree-of-files.md) § 4), so the name only
/// has an answer once the merge is done.
///
/// # Errors
///
/// Whatever [`queue_for`] refuses.
pub fn validate(config: &Config, origins: &BTreeMap<String, Origin>) -> Result<(), Diagnostic> {
    queue_for(config, origins).map(|_| ())
}

/// The bounds the tree's `[queue]` asks for, over `QueueBounds::DEFAULTS`, and `Ok(None)` for a
/// tree that writes no `[queue]` at all.
///
/// Every unwritten key keeps its default independently, since ADR 0103 § 3's override record is per
/// key and a partly-written `[queue]` is four decisions rather than one. `origins` names the file a
/// refusal points at, and an empty map simply leaves the note off.
///
/// # Errors
///
/// `E0617` for the four things a `[queue]` can say that leave nothing runnable: no `connection`, a
/// `connection` naming a `[db.<name>]` the tree does not hold, a `max_attempts` of `0`, and a
/// `visibility` of `0`. A `visibility` that is not a duration at all is `E0601` from
/// [`mod@crate::value`], in that module's words rather than this one's.
pub fn queue_for(
    config: &Config,
    origins: &BTreeMap<String, Origin>,
) -> Result<Option<QueueBounds>, Diagnostic> {
    let Some(queue) = config.queue.as_ref() else {
        return Ok(None);
    };
    let (workers, max_attempts, visibility) = QueueBounds::DEFAULTS;

    let connection = connection_of(config, queue, origins)?;
    let workers = count(queue.workers, workers);

    let max_attempts = count(queue.max_attempts, max_attempts);
    if max_attempts == 0 {
        return Err(refuse(
            "queue.max_attempts",
            "0",
            "a job that may never be attempted is dead-lettered by the enqueue that created it, \
             and § 6 has no spelling for unbounded retries either",
            "write the attempts a job gets before it is dead-lettered, as `5`",
            origins,
        ));
    }

    let visibility = match (
        duration("queue.visibility", queue.visibility.as_ref(), origins)?,
        queue.visibility.as_ref(),
    ) {
        // Zero is only reachable when the block wrote it, so the refusal always has the operator's
        // own spelling to quote back — which is why this reads the parsed value and the written one
        // together.
        (Some(zero), Some(written)) if zero.is_zero() => {
            return Err(refuse(
                "queue.visibility",
                &crate::value::as_written(written),
                "a lease that expires as it is taken makes every job visible to every worker at \
                 once, which is § 4's claim doing nothing",
                "write how long a worker holds a job before another may retry it, as `5m`",
                origins,
            ));
        }
        (Some(visibility), _) => visibility,
        (None, _) => visibility,
    };

    Ok(Some(QueueBounds {
        connection,
        workers,
        max_attempts,
        visibility,
    }))
}

/// § 2's `connection`, proven to name a block the merged tree holds.
///
/// The refusal lists the names that *do* exist, because the mistake this catches is almost always a
/// typo or a block an include was expected to bring in, and both are answered by the roster.
fn connection_of(
    config: &Config,
    queue: &Queue,
    origins: &BTreeMap<String, Origin>,
) -> Result<String, Diagnostic> {
    let Some(name) = queue.connection.as_deref().filter(|name| !name.is_empty()) else {
        return Err(refuse(
            "queue.connection",
            "unset",
            "§ 2 stores jobs in a database an operator names, and there is no default one — the \
             recommended configuration is the application's own, because that is what makes an \
             enqueue commit with the write that caused it",
            "name a `[db.<name>]` block, as `connection = \"main\"`",
            origins,
        ));
    };
    if config.db.contains_key(name) {
        return Ok(name.to_owned());
    }
    let known = if config.db.is_empty() {
        "this tree writes no `[db]` block at all".to_owned()
    } else {
        format!(
            "the blocks this tree writes are {}",
            config
                .db
                .keys()
                .map(|known| format!("`{known}`"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    Err(refuse(
        "queue.connection",
        name,
        &format!(
            "no `[db.{name}]` block is in force, so nothing could be enqueued or claimed; {known}"
        ),
        "name one of them, or add the `[db]` block the queue is meant to store its rows in",
        origins,
    ))
}

/// One written count, narrowed to the width the runtime holds it at.
///
/// A value above [`u32::MAX`] saturates rather than being refused: both bounds are already checked
/// for the value that makes them meaningless, and neither has a ceiling ADR 0084 states, so a
/// number no deployment can reach is not a second refusal worth an operator's time.
fn count(written: Option<u64>, default: u32) -> u32 {
    written.map_or(default, |count| u32::try_from(count).unwrap_or(u32::MAX))
}

/// One written duration bound, and `Ok(None)` for one the block left out.
///
/// [`mod@crate::value`] is the parser, so `"5m"`, `"300s"` and a bare `300` all read the same and a
/// suffix it does not know is refused in its own words rather than in this module's.
fn duration(
    key: &str,
    written: Option<&Setting>,
    origins: &BTreeMap<String, Origin>,
) -> Result<Option<Duration>, Diagnostic> {
    let Some(setting) = written else {
        return Ok(None);
    };
    let quantity = Quantity::parse(key, Unit::Duration, setting)
        .map_err(|invalid| invalid.diagnostic(origins.get(key)))?;
    match quantity {
        Quantity::Nanos(nanos) => Ok(Some(Duration::from_nanos(nanos))),
        // `Unit::Duration` yields nothing else, and the reachable one is `false` — everywhere else
        // in this tree "no ceiling", which is a meaning a lease does not have: a job held forever
        // by a worker that died is the failure § 4's visibility timeout exists to end.
        _ => Err(refuse(
            key,
            &crate::value::as_written(setting),
            "a visibility timeout is finite — `false` removes a ceiling, and a lease that never \
             expires is a job no other worker may ever retry",
            "write how long a worker holds a job before another may retry it, as `5m`",
            origins,
        )),
    }
}

/// A `[queue]` that deserialized and still leaves nothing runnable, under `E0617`.
///
/// Built here rather than through [`crate::value::Invalid`] for [`crate::db`]'s reason: that type
/// says which *unit* a value failed to be, and every value refused here is already the right unit
/// or is not a unit at all.
fn refuse(
    key: &str,
    what: &str,
    why: &str,
    help: &str,
    origins: &BTreeMap<String, Origin>,
) -> Diagnostic {
    Diagnostic::error(
        code::E_BAD_QUEUE,
        format!("`{key}` is `{what}`, which leaves the queue unable to run a job"),
    )
    .with_note(format!("{why}{}", origin_note(origins.get(key))))
    .with_help(help.to_string())
}
