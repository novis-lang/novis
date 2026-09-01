//! [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md) §§ 5-6's
//! **tier 4** — the engine floor: what reports a failure when there is no
//! script left to run and no second attempt to make.
//!
//! # One record, two callers
//!
//! § 6's claim is that ordinary application code and the floor produce
//! **schema-identical** records, so a log pipeline never has to reconcile two
//! shapes depending on which tier happened to write a line. This module is the
//! floor's half of that: it builds an
//! [`nvs_render::Record`] and renders it with
//! [`nvs_render::json::line`] — the same call `Core\Log::write` makes in
//! `nvs_stdlib::log`. Neither side owns a serialiser of its own, because two
//! writers that agree today is the failure
//! [ADR 0092](../../../docs/adr/0092-one-diagnostic-record-three-renderings.md)
//! exists to prevent, and the floor is the worst possible place to discover a
//! disagreement.
//!
//! # It writes to the diagnostic channel, never to the program's output
//!
//! [`Ctx::write_diagnostic`] is the destination, and the failure to write is
//! discarded: this runs where a report has already lost its usual guarantees —
//! after a response, or as a process is ending — so a sink that has gone is
//! nothing left to fail about. `Core\Log::write` writes the same rendering to
//! the program's own stream instead, which is the one difference between the
//! two callers and is about *where* rather than *what*.
//!
//! # What is in an uncaught throw's record, and what is deliberately not
//!
//! [`uncaught`] fills § 6's `level`, `message` and two fields — the error class
//! and, when the throw carried frames, the backtrace. It leaves `ts`,
//! `request_id`, `trace_id` and `span_id` unset, exactly as
//! `nvs_stdlib::log`'s `record` does: an envelope field is omitted rather than
//! written empty, and a floor that filled one its ordinary-code twin does not
//! would be the schema divergence § 6 forbids. When a clock and a request id
//! reach one of the two callers they reach both, in the envelope they share.
//!
//! # It cannot fill the disk it writes to
//!
//! [ADR 0106](../../../docs/adr/0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md)
//! § 10: the floor writes unconditionally, which is correct, so a request that
//! faults in a loop writes in a loop. [`report`] therefore holds
//! § 10's second bound — repeated identical records inside
//! [`COALESCING_WINDOW`] become one record carrying [`nvs_render::Envelope::count`].
//! It sits here rather than on either caller because § 10 puts both bounds on
//! the *sink*, so that no caller has to be trusted to be rare, and here rather
//! than on [`Ctx::write_diagnostic`] beneath it for two reasons: this is the
//! layer that still has a [`Record`] to add a count field to rather than bytes
//! to guess at, and a `Ctx` is per request while a fault loop need not be.
//!
//! `[log] format` is not read here. ADR 0092 § 3's plaintext rendering of the
//! same record is what [ADR 0091](../../../docs/adr/0091-run-mode-is-two-values-a-ceiling-and-a-list-of-defaults.md)
//! § 3's `development` default selects, and nothing reads that directive at run
//! time yet; JSON Lines is § 6's default and the honest single answer until the
//! reader lands.

use std::cell::Cell;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

use nvs_render::{Level, Node, Record, Rendered, Scalar};

use crate::Ctx;
use crate::array::NvsArray;
use crate::string::NvsStr;
use crate::throwable::Thrown;
use crate::value::Value;

/// One uncaught `Throwable` as [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md)
/// § 6's record, at [`Level::Error`].
///
/// The frames are carried as one `backtrace` field in the `#0`-first form
/// [`Thrown::trace_as_string`] renders, rather than as a node per frame: § 6
/// calls the field "a stack summary", and one string is what a log pipeline
/// greps. A throw with no frames — a `FATAL`, which has none by design — omits
/// the field rather than carrying an empty one.
#[must_use]
pub fn uncaught(thrown: &Thrown) -> Record {
    let mut record = note(Level::Error, &thrown.message());
    record
        .envelope
        .fields
        .push(("class".to_owned(), text(&thrown.class_name())));
    let trace = thrown.trace_as_string();
    if !trace.is_empty() {
        record
            .envelope
            .fields
            .push(("backtrace".to_owned(), text(&trace)));
    }
    record
}

/// [ADR 0020](../../../docs/adr/0020-error-escalation-ladder.md) § 3's one
/// explicit argument to the tier-3 handler, built from the record tier 4 would
/// otherwise have reported.
///
/// **A keyed array, not a `Core\Fatal\ErrorReport` instance**, for the reason
/// § 1 gives for `LimitReport` being one: the report is built where the failure
/// is, in a crate that holds no `Core` class descriptor to instantiate an object
/// from, and a keyed array takes a later key without changing the signature of a
/// handler already written. The keys are the record's *own* — `level`, `message`
/// and one per envelope field — so a handler reads the same names a JSON Lines
/// line carries and there is no second vocabulary between the two renderings of
/// one failure.
///
/// A field whose node is not a string scalar is **skipped** rather than
/// rendered. Nothing the floor produces has one today, and rendering a tree here
/// would be a second serialiser beside [`nvs_render`]'s, which is the divergence
/// this module exists to prevent; when a producer grows a structured field, the
/// walk that turns a [`Node`] into a `Value` is what this reaches for, not a
/// stringification of its own.
///
/// The caller takes over the returned value's one reference — it is the
/// argument an isolate is handed, and `nvs_host::Isolate::new` consumes it.
#[must_use]
pub fn report_argument(record: &Record) -> Value {
    let mut report = NvsArray::new();
    report.set(NvsStr::new(b"level"), string(record.envelope.level.name()));
    if let Some(message) = &record.envelope.message {
        report.set(NvsStr::new(b"message"), string(message.as_str()));
    }
    for (name, node) in &record.envelope.fields {
        if let Node::Scalar(Scalar::Str { text, .. }) = node {
            report.set(NvsStr::new(name.as_bytes()), string(text.as_str()));
        }
    }
    Value::array(report)
}

/// One `&str` as an owned Novis `string` value.
fn string(text: &str) -> Value {
    Value::str(NvsStr::new(text.as_bytes()))
}

/// A record carrying nothing but a level and a message — what the floor has to
/// say when there is no `Throwable` behind the failure.
#[must_use]
pub fn note(level: Level, message: &str) -> Record {
    let mut record = Record::at(level);
    record.envelope.message = Some(Rendered::new(message));
    record
}

/// One string as a field's node, with ADR 0092 § 5's substitution applied.
///
/// Public because a caller's own context — which queue the failure came out of,
/// which limit was reached — is a field it adds to a record this module built,
/// and building a [`Node`] by hand is the one part of that a caller should not
/// have to restate.
#[must_use]
pub fn text(value: &str) -> Node {
    Node::Scalar(Scalar::Str {
        text: Rendered::new(value),
        bytes: value.len(),
    })
}

/// Renders `record` as ADR 0092 § 3's JSON Lines line and writes it where
/// `[log] target` says — `ctx`'s diagnostic channel where it says nothing —
/// unless [ADR 0106](../../../docs/adr/0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md)
/// § 10's window has already written it.
///
/// [`Ctx::write_log_record`] is the routing and the only reader of that
/// directive; this is one of its two callers and `Core\Log::write` is the
/// other, which is ADR 0020 § 6's sameness on the destination as well as on
/// the record. It reads `[log] level` in the same place, so a floor record
/// quieter than the configured minimum is dropped there and not here — the
/// coalescing above still counts it, because what that window bounds is how
/// often *this* module builds a record at all.
///
/// Infallible by construction: [`nvs_render::json::line`] answers a `String`
/// rather than a `Result` for this caller's sake, and a sink that has already
/// gone is ignored for the reason this module's docs give.
pub fn report(ctx: &mut Ctx, record: &Record) {
    let Some(count) = admit(key(record), Instant::now()) else {
        return;
    };
    let line = if count == 1 {
        nvs_render::json::line(record)
    } else {
        let mut carried = record.clone();
        carried.envelope.count = Some(count);
        nvs_render::json::line(&carried)
    };
    let _ = ctx.write_log_record(
        record.envelope.level,
        crate::LogChannel::Diagnostic,
        line.as_bytes(),
    );
}

/// How long one record holds the window open — ADR 0106 § 10's rate limit,
/// which is what keeps a request faulting in a loop from writing in a loop.
///
/// One second, and **not a directive**: a bound that exists so the disk cannot
/// be filled is not one an operator should be able to raise to infinity, and
/// `Core\RateLimit`'s own § 4 already settles that no configuration at all is
/// the design for a limiter. The number is chosen so that a fault loop costs a
/// line per second — enough to see that it is still happening, few enough that
/// a day of it is 86,400 lines rather than a partition.
pub const COALESCING_WINDOW: Duration = Duration::from_secs(1);

/// One record's open window: what it is, when it closes, and how many identical
/// records it has swallowed since it opened.
///
/// `Copy`, and deliberately **holding no record content** — a key, a deadline
/// and a count. This state outlives the request that opened it, because the
/// floor is per core and a `Ctx` is per request (which is why the window cannot
/// live on the context: a counter on the request could not coalesce across
/// requests, and a fault loop is as easily one request per fault as one). State
/// that outlives a request must not carry what that request said, so the window
/// remembers only that *something* identical was reported and how often, never
/// the bytes — which also makes its footprint a constant rather than one
/// abandoned record per core.
#[derive(Clone, Copy)]
struct Window {
    /// [`key`]'s hash of the record this window belongs to.
    key: u64,
    /// When it stops swallowing — `opened + COALESCING_WINDOW`, stored as the
    /// deadline so that [`expire_coalescing_window`] can close it by moving it
    /// to now rather than by subtracting from an [`Instant`] that may be too
    /// young to subtract from.
    deadline: Instant,
    /// How many identical records have been swallowed since the window opened.
    suppressed: u64,
}

thread_local! {
    /// This core's one open window. One is enough: § 10 bounds a *loop*, and a
    /// loop reports the same record over and over, so a table keyed by record
    /// would spend memory to bound a case that does not arise and would need an
    /// eviction rule of its own.
    static WINDOW: Cell<Option<Window>> = const { Cell::new(None) };
}

/// Decides what `now`'s report of the record hashing to `key` writes: `None` to
/// write nothing, or `Some(count)` for the number of identical records the line
/// about to be written stands for, itself included.
///
/// The first report opens a window and is written at once — latency on a
/// failure path is the whole reason a diagnostic exists, so nothing is ever
/// held back waiting for a window to close. Repeats inside the window are
/// swallowed. The first repeat *after* it opens a new window and carries the
/// swallowed ones' count, which is how every line stays a line the reader can
/// act on rather than a summary of one.
///
/// **A burst that stops is under-reported by its tail**, and that is the
/// deliberate cost of [`Window`] holding no content: with the record's bytes
/// gone there is nothing left to flush a trailing count onto, and the next
/// report is a different record whose line must not claim someone else's
/// multiplicity. What the reader loses is the *count* of a failure they have
/// already been told about, never the failure.
fn admit(key: u64, now: Instant) -> Option<u64> {
    WINDOW.with(|window| match window.get() {
        Some(open) if open.key == key && now < open.deadline => {
            window.set(Some(Window {
                suppressed: open.suppressed + 1,
                ..open
            }));
            None
        }
        open => {
            window.set(Some(Window {
                key,
                deadline: now + COALESCING_WINDOW,
                suppressed: 0,
            }));
            Some(match open {
                Some(open) if open.key == key => open.suppressed + 1,
                _ => 1,
            })
        }
    })
}

/// Closes this core's window now, so that the next [`report`] of the same
/// record opens a new one and carries the count.
///
/// The seam a test uses instead of sleeping through [`COALESCING_WINDOW`], on
/// the same terms as [`Ctx::set_diagnostic_sink`]: a bound whose whole subject
/// is elapsed time is otherwise only assertable by a test that takes a second
/// to run and is flaky when the machine is loaded.
pub fn expire_coalescing_window() {
    WINDOW.with(|window| {
        if let Some(open) = window.get() {
            window.set(Some(Window {
                deadline: Instant::now(),
                ..open
            }));
        }
    });
}

/// What makes two reports of one failure *the same record* — everything the
/// renderings write except the keys that differ per occurrence by design.
///
/// Hashed off [`nvs_render::json::render`]'s own output rather than off a walk
/// of the tree written here, for this module's standing reason: a second walk
/// over the record model is a second serialiser, and one that drifts from the
/// first would silently coalesce two records that render differently — the
/// exact failure ADR 0092 exists to prevent, arriving as a *missing* line.
///
/// `ts`, `request_id`, `trace_id`, `span_id` and any `count` already on the
/// record are cleared first: they are what distinguishes two occurrences of one
/// failure, and a limiter that let them distinguish would never fire.
fn key(record: &Record) -> u64 {
    let mut stable = record.clone();
    stable.envelope.ts = None;
    stable.envelope.request_id = None;
    stable.envelope.trace_id = None;
    stable.envelope.span_id = None;
    stable.envelope.count = None;
    let mut hasher = DefaultHasher::new();
    nvs_render::json::render(&stable).hash(&mut hasher);
    hasher.finish()
}
