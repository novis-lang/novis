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
//! `[log] format` is not read here. ADR 0092 § 3's plaintext rendering of the
//! same record is what [ADR 0091](../../../docs/adr/0091-run-mode-is-two-values-a-ceiling-and-a-list-of-defaults.md)
//! § 3's `development` default selects, and nothing reads that directive at run
//! time yet; JSON Lines is § 6's default and the honest single answer until the
//! reader lands.

use nvs_render::{Level, Node, Record, Rendered, Scalar};

use crate::Ctx;
use crate::throwable::Thrown;

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

/// Renders `record` as ADR 0092 § 3's JSON Lines line and writes it to `ctx`'s
/// diagnostic channel.
///
/// Infallible by construction: [`nvs_render::json::line`] answers a `String`
/// rather than a `Result` for this caller's sake, and a sink that has already
/// gone is ignored for the reason this module's docs give.
pub fn report(ctx: &mut Ctx, record: &Record) {
    let _ = ctx.write_diagnostic(nvs_render::json::line(record).as_bytes());
}
