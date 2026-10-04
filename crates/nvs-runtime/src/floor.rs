//! `rule:errors/panics-bypass-user-code` and `rule:errors/log-write`'s
//! **tier 4** — the engine floor: what reports a failure when there is no
//! script left to run and no second attempt to make.
//!
//! # One record, whichever tier writes it
//!
//! § 6's claim is that ordinary application code and the floor produce
//! **schema-identical** records, so a log pipeline never has to reconcile two
//! shapes depending on which tier happened to write a line. This module is the
//! floor's half of that: it builds an
//! [`nvs_render::Record`] and renders it with
//! [`nvs_render::json::line`] — the same call `Core\Log::write` makes in
//! `nvs_stdlib::log`. Neither side owns a serialiser of its own, because two
//! writers that agree today is the failure
//! `rule:errors/diagnostic-record`
//! exists to prevent, and the floor is the worst possible place to discover a
//! disagreement.
//!
//! # It writes to the diagnostic channel, never to the program's output
//!
//! [`Ctx::write_diagnostic`] is the destination, and the failure to write is
//! discarded: this runs where a report has already lost its usual guarantees —
//! after a response, or as a process is ending — so a sink that has gone is
//! nothing left to fail about. `Core\Log::write` writes the same rendering to
//! the program's own stream instead, which is the one difference between them
//! and is about *where* rather than *what*.
//!
//! # What is in an uncaught throw's record, and what is deliberately not
//!
//! [`uncaught`] fills § 6's `level`, `message`, a field for the error class and
//! one node per frame the throw carried. It fills none of
//! `ts`, `request_id`, `trace_id` or `span_id`, because it is handed a
//! `Throwable` and not a context, and those keys are the request's. [`report`]
//! is where a context arrives, and it stamps them through
//! [`Ctx::stamp_envelope`] — the *same* method `Core\Log::write` calls, which
//! is what keeps a floor line and an application's the one shape § 6 asks for.
//! A record built with no request in front of it carries none of them, and an
//! absent envelope key is omitted rather than written empty.
//!
//! The stamp happens **after** [`key`] has taken the coalescing window's key,
//! which is why it is a call of its own rather than something
//! [`Ctx::write_log_record`] does: those keys are exactly what
//! distinguishes two occurrences of one failure, and a limiter that saw them
//! would never fire.
//!
//! # It cannot fill the disk it writes to
//!
//! `rule:http-server/the-floor-cannot-fill-the-disk`
//! : the floor writes unconditionally, which is correct, so a request that
//! faults in a loop writes in a loop. [`report`] therefore holds
//! § 10's second bound — repeated identical records inside
//! [`COALESCING_WINDOW`] become one record carrying [`nvs_render::Envelope::count`].
//! It sits here rather than on either caller because § 10 puts both bounds on
//! the *sink*, so that no caller has to be trusted to be rare, and here rather
//! than on [`Ctx::write_diagnostic`] beneath it because this is the layer that
//! still has a [`Record`] to add a count field to rather than bytes to guess
//! at, and because a `Ctx` is per request while a fault loop need not be.
//!
//! **Both writers of a disk-bounded record coalesce here, and each gets the
//! window its own traffic needs.** `rule:errors/a-repeat-is-bounded-at-the-sink-that-suffers`
//! : the floor keeps [`WINDOW`]'s single slot, because a fault loop reports one
//! record over and over; `Core\Log::write` goes through [`admit_log_record`]'s
//! fixed table, because application code interleaves and one slot would
//! coalesce none of it. They share [`key`] and [`COALESCING_WINDOW`], so what
//! counts as the same record and how long one holds is decided once for both.
//!
//! # The tier above is reached from here too
//!
//! Tier 3 runs in `nvs_host`, which is above this crate, so a failure this
//! crate classifies for itself — a throw escaping an abandoned generator's
//! `finally`, which reaches no request root to be reported at
//! ([`Ctx::with_pending_set_aside`]) — cannot call it. [`Ladder`] is the
//! function pointer the process's entry point installs and [`escalate`] is the
//! one read of it, answering `false` in a process that installed none, which is
//! the same sentence "no handler is configured" already gets: the floor still
//! owes the line. It sits beside the floor rather than in a module of its own
//! because [`report_argument`] already does — the crate that owns the meaning
//! of a record keeps both the shape tier 3 is handed and the seam it is reached
//! through, and `nvs_host::ladder`'s own module doc argues that split in full.
//!
//! `[log] format` is not read here. `rule:errors/renderings`'s plaintext rendering of the
//! same record is what `rule:config/a-mode-is-five-defaults`
//! 's `development` default selects, and nothing reads that directive at run
//! time yet; JSON Lines is § 6's default and the honest single answer until the
//! reader lands.

use std::cell::{Cell, RefCell};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use nvs_render::{Level, Node, Record, Rendered, Scalar};

use crate::Ctx;
use crate::array::NvsArray;
use crate::string::NvsStr;
use crate::throwable::Thrown;
use crate::value::Value;

/// One uncaught `Throwable` as `rule:errors/log-write`'s record, at [`Level::Error`].
///
/// The frames are the record's **own nodes**, one [`Node::Frame`] each, which
/// is `rule:errors/record-producers`'s row for this producer and what
/// [`Record::nodes`] already names as a `Throwable`'s half of it. A pipeline
/// reads `nodes[0].function` instead of grepping inside a stack summary, and
/// the plaintext rendering still prints the `#0`-first line a person greps —
/// two readings of one shape, which is what one string could not give. A throw
/// with no frames — a `FATAL`, which has none by design — carries none rather
/// than an empty node.
///
/// Beside the error class, the throw's **own declared properties** are one
/// `properties` field when its class adds any past `Throwable`'s four
/// ([`Thrown::properties`] says which, and why the root's own are not among
/// them), so a `ParseError`'s `issues` and a user class's own state reach the
/// report. They are walked by [`crate::record`] — the same walk
/// `Core\Debug::dump` uses — so `rule:errors/record-transformations`'s
/// redaction reaches a `secret` property anywhere under one, and a secret is a
/// [`Node::Redacted`] rather than bytes in a log.
///
/// **What it spends:** one node per frame and one per value reached under those
/// properties, bounded by [`nvs_render::Caps`], on a path that has already
/// failed.
#[must_use]
pub fn uncaught(thrown: &Thrown) -> Record {
    let mut record = note(Level::Error, &thrown.message());
    record
        .envelope
        .fields
        .push(("class".to_owned(), text(&thrown.class_name())));
    let properties = thrown.properties();
    if !properties.is_empty() {
        record.envelope.fields.push((
            "properties".to_owned(),
            Node::Map(
                properties
                    .into_iter()
                    .map(|(name, node)| (Rendered::new(&name), node))
                    .collect(),
            ),
        ));
    }
    record.nodes = thrown
        .frames()
        .iter()
        .enumerate()
        .map(|(depth, frame)| frame.node(depth))
        .collect();
    record
}

/// `rule:errors/handler-script`'s one
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
/// rendered, and the record's frames are the one structured thing carried
/// across — as a `backtrace` list of `{function, file, line}` arrays, the shape
/// [`nvs_render::json`] writes them in, so a handler reads the trace the same
/// way whichever it was handed. The throw's `properties` field is not carried:
/// a [`Node::Redacted`] or an [`nvs_render::Elision`] has no `Value` spelling,
/// and a handler's array is the last place a redaction should be undone to make
/// one.
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
    if let Some(frames) = frames_argument(&record.nodes) {
        report.set(NvsStr::new(b"backtrace"), frames);
    }
    Value::array(report)
}

/// The record's frames as one keyed array per frame, in the trace's order, or
/// `None` for a record carrying none — a `FATAL`, and every producer that is
/// not a throw.
fn frames_argument(nodes: &[Node]) -> Option<Value> {
    if !nodes.iter().any(|node| matches!(node, Node::Frame { .. })) {
        return None;
    }
    let mut frames = NvsArray::new();
    for node in nodes {
        let Node::Frame {
            function,
            file,
            line,
            ..
        } = node
        else {
            continue;
        };
        let mut frame = NvsArray::new();
        frame.set(NvsStr::new(b"function"), string(function.as_str()));
        if let Some(file) = file {
            frame.set(NvsStr::new(b"file"), string(file));
        }
        if let Some(line) = line {
            frame.set(NvsStr::new(b"line"), Value::uint(u64::from(*line)));
        }
        frames.append(Value::array(frame));
    }
    Some(Value::array(frames))
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

/// One string as a field's node, with `rule:errors/record-transformations`'s substitution applied.
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

/// Renders `record` as `rule:errors/renderings`'s JSON Lines line and writes it where
/// `[log] target` says — `ctx`'s diagnostic channel where it says nothing —
/// unless `rule:http-server/the-floor-cannot-fill-the-disk`
/// 's window has already written it.
///
/// [`Ctx::write_log_record`] is the routing and the only reader of that
/// directive; this is one of its callers and `Core\Log::write` is another,
/// which is `rule:errors/log-write`'s sameness on the destination as well as on
/// the record. It reads `[log] level` in the same place, so a floor record
/// quieter than the configured minimum is dropped there and not here — the
/// coalescing above still counts it, because what that window bounds is how
/// often *this* module builds a record at all.
///
/// It does not render, either: `[log] format` picks between `rule:errors/renderings`'s
/// renderings at that same call, so this module hands over the *record* and a
/// floor line and an application's are the same shape in whichever one the
/// deployment configured.
///
/// Infallible by construction: nothing here can fail, and a sink that has
/// already gone is ignored for the reason this module's docs give.
pub fn report(ctx: &mut Ctx, record: &Record) {
    let Some(count) = admit(key(record), Instant::now()) else {
        return;
    };
    // Owned from here on, because everything left to do writes to the
    // envelope: the multiplicity, and § 6's request keys. The clone
    // is the one [`key`] already takes per call, on the path a request has
    // already failed on rather than on the one it is served by.
    let mut carried = record.clone();
    if count > 1 {
        carried.envelope.count = Some(count);
    }
    ctx.stamp_envelope(&mut carried.envelope);
    let _ = ctx.write_log_record(&carried, crate::LogChannel::Diagnostic);
}

/// How a record reaches `rule:errors/escalation-ladder`'s **tier 3** from a
/// crate beneath the one that runs it.
///
/// `nvs_host::ladder::escalate` is the implementation and this is its shape, so
/// a caller that can name that function calls it directly and one that cannot
/// reads [`escalate`] instead — the two are the same call, and a single meaning
/// for the answer: `true` is "the handler reported, write nothing more", and
/// every other outcome is `false`, which is tier 4's cue.
pub type Ladder = fn(&mut Ctx, &Record) -> bool;

/// The installed tier 3, or `None` in a process whose entry point installed
/// none.
///
/// Process-wide rather than per thread, which is what a tier of the ladder is:
/// `[log] handler` is one deployment's configuration and an isolate spawned for
/// it runs on whichever core the failure happened on. The pointer is copied out
/// before it is called, so a handler that fails — or that reaches a failure of
/// its own — never finds this lock held.
static LADDER: Mutex<Option<Ladder>> = Mutex::new(None);

/// Installs `ladder` as this process's tier 3, replacing whatever was there.
///
/// Called once by the entry point, beside [`install_panic_hook`] and before any
/// program runs. A process that calls it never has [`escalate`] answer `false`
/// for want of a hook, and one that does not still has the floor.
pub fn install_ladder(ladder: Ladder) {
    *LADDER.lock().unwrap_or_else(PoisonError::into_inner) = Some(ladder);
}

/// Swaps the installed tier 3 for `ladder`, answering what was there.
///
/// The restore [`install_ladder`] deliberately is not: a boot-time seam has
/// nothing to put back, and a test that installs one has to leave the process
/// as it found it.
#[cfg(test)]
pub(crate) fn swap_ladder(ladder: Option<Ladder>) -> Option<Ladder> {
    std::mem::replace(
        &mut *LADDER.lock().unwrap_or_else(PoisonError::into_inner),
        ladder,
    )
}

/// Offers `record` to tier 3, answering whether it reported.
///
/// `false` means the floor still owes the line, and [`report`] is what the
/// caller writes next — the shape `nvs_host::ladder`'s own module doc fixes,
/// held to here for the one reason it exists: no tier is retried, and a record
/// nothing escalated is a record tier 4 writes rather than one that is dropped.
#[must_use]
pub fn escalate(ctx: &mut Ctx, record: &Record) -> bool {
    let installed = *LADDER.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(ladder) = installed else {
        return false;
    };
    ladder(ctx, record)
}

/// Installs the process's panic hook: a panic raised beneath a **served
/// request** becomes one of this module's records, with that request's id on
/// it, and every other panic is left to the hook that was already there.
///
/// `rule:errors/helper-abi` asks for the message to reach the request log.
/// [`crate::run_helper`] already recovers it into the [`Ctx`], but that copy is
/// the *fault the request fails with* — it goes where the escalation ladder
/// sends a fault, not where an operator greps by request id. This is the other
/// half, and it is **presentation only**: a hook runs before the unwind starts
/// and then returns, so [`crate::run_helper`] and [`crate::run_task`] contain
/// the panic exactly as they do without one. Nothing here recovers a panic,
/// and `rule:http-server/containment-does-not-end-at-the-helper` is untouched.
///
/// # It reaches the context the way a release does
///
/// A hook is handed a `PanicHookInfo` and nothing else, so there is no argument
/// to carry a context in and `ctx::current`'s thread-local is the only door.
/// It is sound here for that module's own reason and one more: the frames
/// holding `&mut Ctx` above are not running while the hook is — it is running
/// *instead* of them, on a stack that is about to unwind past them.
///
/// # A CLI script keeps stderr
///
/// The test is [`Ctx::inbound`], which is exactly what [`Ctx::stamp_envelope`]
/// gates on: a context with no request has no request id to stamp, and a
/// `nvs run` whose panic went to `[log] target` would take the message off the
/// stream its user is reading. So a panic this hook does not claim is passed to
/// the hook that was installed before it — the default one, which prints to
/// stderr — and `nvs run` behaves as it did.
///
/// **What it spends:** one boxed Rust closure for the life of the process, and
/// nothing per request. The record is built on a path that has already failed.
pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let message = crate::abi::panic_message(info.payload());
        let location = info.location().map(ToString::to_string);
        if !report_panic(&message, location.as_deref()) {
            previous(info);
        }
    }));
}

/// Writes one panic as [`report`]'s record, answering whether it was claimed —
/// `false` when no served request is running on this thread, which is
/// [`install_panic_hook`]'s signal to fall through to the previous hook.
///
/// Separate from the hook because a `PanicHookInfo` cannot be constructed, so
/// this is the half a test can put a question to.
///
/// [`Level::Critical`] and not [`Level::Error`]: an internal panic is
/// `rule:errors/escalation-ladder`'s tier 4, which is the level that exists for
/// it, while an uncaught `Throwable` — a program's own failure — is the tier
/// [`uncaught`] reports at. The panic's own `file:line:column` is a `location`
/// field for [`text`]'s reason, and the message stays the message so that a
/// hook line and the fault the request fails with read the same.
fn report_panic(message: &str, location: Option<&str>) -> bool {
    crate::ctx::with_current(|ctx| {
        if ctx.inbound().is_none() {
            return false;
        }
        let mut record = note(Level::Critical, message);
        if let Some(location) = location {
            record
                .envelope
                .fields
                .push(("location".to_owned(), text(location)));
        }
        report(ctx, &record);
        true
    })
    .unwrap_or(false)
}

/// How long one record holds the window open — `rule:http-server/the-floor-cannot-fill-the-disk`'s rate limit,
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
    /// This core's one open window, which is all the floor needs: what it
    /// bounds is a fault *loop*, and a loop reports the same record over and
    /// over, so a second slot here would hold a record no repeat ever arrives
    /// for. The sink whose traffic interleaves is the log target, and
    /// [`LOG_WINDOWS`] is the table it gets for it.
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

/// How many distinct records the log target holds a window open for at once.
///
/// The table is **scanned, not indexed**, so a record's slot never depends on
/// its hash and two records can never collide into one — a slot holds one key,
/// and a write is only ever suppressed by an identical write. Sixteen is chosen
/// so that a handler reporting a handful of distinct failures in a loop
/// coalesces every one of them, while the whole table stays a scan of a couple
/// of cache lines and a constant per core.
pub const LOG_WINDOW_SLOTS: usize = 16;

thread_local! {
    /// The log target's open windows —
    /// `rule:errors/a-repeat-is-bounded-at-the-sink-that-suffers`'s small fixed
    /// table, where the floor keeps [`WINDOW`]'s single slot.
    ///
    /// Per core and holding no record content, for [`Window`]'s own reason: a
    /// window outlives the request that opened it, so it remembers only that
    /// something identical was written and how often. The footprint is
    /// [`LOG_WINDOW_SLOTS`] windows whatever the program goes on to write.
    static LOG_WINDOWS: RefCell<[Option<Window>; LOG_WINDOW_SLOTS]> =
        const { RefCell::new([None; LOG_WINDOW_SLOTS]) };
}

/// Decides whether this `Core\Log::write` writes `record` at all, stamping the
/// multiplicity onto it when the line stands for more than itself.
///
/// [`report`]'s bound, over the table instead of the single slot: the first
/// write of a record opens a window and goes out at once, repeats inside it are
/// swallowed, and the first one after it closes carries how many it stands for.
/// What differs is only how many records can be in that state at the same time,
/// which is the whole difference between a fault loop and application code.
///
/// The count is written here rather than by the caller because it is
/// `rule:errors/diagnostic-record`'s one sink-written key, and this module is
/// the sink for both of its writers.
pub fn admit_log_record(record: &mut Record) -> bool {
    let Some(count) = admit_to_table(key(record), Instant::now()) else {
        return false;
    };
    if count > 1 {
        record.envelope.count = Some(count);
    }
    true
}

/// [`admit`]'s decision over [`LOG_WINDOWS`]: `None` to write nothing, or
/// `Some(count)` for what the line about to be written stands for.
///
/// A slot already holding this key answers for the record whether or not its
/// window is still open, so a closed window's count is carried by the next
/// occurrence rather than dropped. A record the table holds no slot for takes
/// the emptiest one — a free slot before any open window, and the window
/// nearest to closing before any younger one, which is an expired slot wherever
/// there is one.
///
/// An evicted window costs the count of what has *already* been reported and
/// never a failure: the record it belonged to is written again at once, which
/// is [`admit`]'s trailing-count cost arriving by a second route.
fn admit_to_table(key: u64, now: Instant) -> Option<u64> {
    LOG_WINDOWS.with(|windows| {
        let mut windows = windows.borrow_mut();
        if let Some(open) = windows.iter_mut().flatten().find(|open| open.key == key) {
            if now < open.deadline {
                open.suppressed += 1;
                return None;
            }
            let carried = open.suppressed + 1;
            open.deadline = now + COALESCING_WINDOW;
            open.suppressed = 0;
            return Some(carried);
        }
        // `None` orders before `Some` and an earlier deadline before a later
        // one, so this one comparison is the whole replacement policy.
        if let Some(slot) = windows
            .iter_mut()
            .min_by_key(|slot| slot.map(|open| open.deadline))
        {
            *slot = Some(Window {
                key,
                deadline: now + COALESCING_WINDOW,
                suppressed: 0,
            });
        }
        Some(1)
    })
}

/// Closes every window the log target holds open, so that the next write of any
/// of those records opens a new one and carries its count.
///
/// [`expire_coalescing_window`]'s seam for the second table, on the same terms:
/// a bound whose whole subject is elapsed time is otherwise only assertable by
/// a test that takes a second to run and is flaky when the machine is loaded.
pub fn expire_log_windows() {
    let now = Instant::now();
    LOG_WINDOWS.with(|windows| {
        for open in windows.borrow_mut().iter_mut().flatten() {
            open.deadline = now;
        }
    });
}

/// What makes two records *the same record*, for both of this module's
/// windows — everything the renderings write except the keys that differ per
/// occurrence by design.
///
/// Hashed off [`nvs_render::json::render`]'s own output rather than off a walk
/// of the tree written here, for this module's standing reason: a second walk
/// over the record model is a second serialiser, and one that drifts from the
/// first would silently coalesce two records that render differently — the
/// exact failure `rule:errors/diagnostic-record` exists to prevent, arriving as a *missing* line.
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

#[cfg(test)]
mod tests {
    use nvs_render::Source;

    use super::{
        LOG_WINDOW_SLOTS, LOG_WINDOWS, Level, Node, Rendered, Value, Window, admit,
        admit_log_record, expire_log_windows, install_panic_hook, key, note, report_panic, text,
        uncaught,
    };
    use crate::ctx::CurrentCtx;
    use crate::object::{ClassTable, NvsObj};
    use crate::string::NvsStr;
    use crate::throwable::{SLOT_COUNT, Thrown, ThrownClass};
    use crate::{Ctx, Inbound, OutputSink, TaskRoot, run_task};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex, PoisonError};
    use std::time::Instant;

    /// Serialises the tests that replace the process's panic hook, so that
    /// neither restores what the other installed.
    static HOOK: Mutex<()> = Mutex::new(());

    /// The slots every `Throwable` has, in slot order — `crate::throwable`'s
    /// own header says why the order is load-bearing and where it is declared.
    const THROWABLE_SLOTS: [&str; SLOT_COUNT] = ["message", "previous", "backtrace", "location"];

    /// `rule:errors/record-producers`'s row for the throw producer: the frames
    /// are the record's own nodes, one per frame, and the two readers of a
    /// trace each get the spelling they read — `{function, file, line}` in the
    /// JSON rendering a pipeline indexes, and the `#0`-first line a person
    /// greps in the plaintext one.
    #[test]
    fn an_uncaught_throwables_frames_are_a_sequence_of_object_nodes() {
        let mut table = ClassTable::new();
        let id = table.define("RuntimeError", &THROWABLE_SLOTS, &[]);
        let desc = table.desc(id);
        #[expect(unsafe_code, reason = "the table outlives the exception built from it")]
        let thrown = unsafe { Thrown::new(desc, "boom") };
        thrown.push_frame("Boom::go() at app/Boom.nvs:3");
        thrown.push_frame("<script>() at app/main.nvs:11");

        let record = uncaught(&thrown);
        assert_eq!(
            record.nodes,
            vec![
                Node::Frame {
                    depth: 0,
                    function: Rendered::new("Boom::go"),
                    file: Some("app/Boom.nvs".to_owned()),
                    line: Some(3),
                },
                Node::Frame {
                    depth: 1,
                    function: Rendered::new("<script>"),
                    file: Some("app/main.nvs".to_owned()),
                    line: Some(11),
                },
            ],
            "one node per frame, innermost first, and no stack summary beside them"
        );
        let json = nvs_render::json::render(&record);
        assert!(
            json.contains(
                r#""nodes":[{"function":"Boom::go","file":"app/Boom.nvs","line":3},{"function":"<script>","file":"app/main.nvs","line":11}]"#
            ),
            "{json}"
        );
        let plain = nvs_render::plain::render(&record);
        assert!(plain.contains("#0 Boom::go() at app/Boom.nvs:3"), "{plain}");
        assert!(
            plain.contains("#1 <script>() at app/main.nvs:11"),
            "{plain}"
        );
    }

    /// ADR 0092 § *Verification*'s M4 bullet: a `secret`-typed property on an
    /// object in a stack frame is redacted in the uncaught record too.
    ///
    /// Novis's frames carry a label and nothing else —
    /// `rule:errors/propagation` builds the trace as the throw unwinds, so
    /// there is no captured argument or receiver in one to leak — which leaves
    /// exactly one object standing in the frames a record carries: the throw
    /// itself, and whatever its own class declared. Those properties go through
    /// [`crate::record`]'s walk, so the *declared* qualifier decides and the
    /// value is never read at all. Asserted through a **nested** object, which
    /// is the shape the leak would take if the walk stopped at the throw's own
    /// slots.
    #[test]
    fn a_secret_property_on_an_object_in_a_frame_is_redacted_in_the_uncaught_record() {
        let mut table = ClassTable::new();
        let settings = table.define("Settings", &["host", "token"], &[]);
        table.set_secret_fields(settings, vec![false, true]);
        let mut slots: Vec<String> = THROWABLE_SLOTS.iter().map(|s| (*s).to_owned()).collect();
        slots.push("settings".to_owned());
        let error = table.define("ConfigError", &slots, &[]);
        let (settings, error) = (table.desc(settings), table.desc(error));

        #[expect(
            unsafe_code,
            reason = "the table outlives every value built from it, and the \
                      reference the object is built with is moved into the \
                      exception's own slot"
        )]
        let held = unsafe { NvsObj::new(settings) };
        held.set_field(0, Value::str(NvsStr::new(b"db.internal")));
        held.set_field(1, Value::str(NvsStr::new(b"hunter2-the-token")));
        #[expect(unsafe_code, reason = "as the object built above")]
        let thrown = unsafe {
            Thrown::new_as(
                error,
                ThrownClass::Runtime,
                "could not configure",
                &[(SLOT_COUNT, Value::from_obj_ptr(held.into_raw()))],
            )
        };
        thrown.push_frame("Config::load() at app/Config.nvs:9");

        let record = uncaught(&thrown);
        let Some((_, Node::Map(properties))) = record
            .envelope
            .fields
            .iter()
            .find(|(name, _)| name == "properties")
        else {
            panic!("a class declaring a property past the root's four carries it");
        };
        let [
            (
                name,
                Node::Object {
                    class, properties, ..
                },
            ),
        ] = properties.as_slice()
        else {
            panic!("the one property the class declared, as the object it holds");
        };
        assert_eq!(name.as_str(), "settings");
        assert_eq!(class, "Settings");
        assert_eq!(
            properties[1],
            ("token".to_owned(), Node::Redacted),
            "the declared qualifier decides, one object below the throw"
        );

        let json = nvs_render::json::render(&record);
        assert!(
            !json.contains("hunter2-the-token"),
            "a secret never reaches a log line: {json}"
        );
        assert!(json.contains(r#""token":{"$redacted":true}"#), "{json}");
        assert!(
            json.contains("db.internal"),
            "and the property beside it still reports: {json}"
        );
        assert!(
            !nvs_render::plain::render(&record).contains("hunter2-the-token"),
            "nor the plaintext rendering of the same record"
        );
    }

    /// One `Core\Log::write`'s record, as the member and line its call site
    /// would have named — `rule:errors/a-record-names-where-it-was-produced`'s
    /// `source`, which the identity below counts.
    fn written(message: &str, line: u32) -> nvs_render::Record {
        let mut record = note(Level::Info, message);
        record.envelope.source = Some(Source {
            file: "app.nvs".to_owned(),
            line,
            member: Some("App::run".to_owned()),
        });
        record
    }

    /// What one write of `record` puts on the wire: `None` for a write the
    /// table swallowed, `Some(count)` for the line it wrote and how many
    /// occurrences that line stands for.
    fn write(record: &nvs_render::Record) -> Option<u64> {
        let mut carried = record.clone();
        admit_log_record(&mut carried).then(|| carried.envelope.count.unwrap_or(1))
    }

    /// `rule:errors/helper-abi`'s request-log half: a panic raised under a
    /// served request becomes one of this module's records, carrying the id
    /// [`Ctx::stamp_envelope`] would have put on any other record of that same
    /// request — which is what makes the line greppable beside them.
    #[test]
    fn a_panic_on_a_served_request_is_written_to_the_request_log_with_its_request_id() {
        let mut ctx = Ctx::buffered();
        ctx.set_diagnostic_sink(OutputSink::Buffer(Vec::new()));
        ctx.set_inbound(Inbound::new("GET", "/served", ""));
        let request_id = ctx.trace_context().trace_id_hex();

        let installed = CurrentCtx::install(&mut ctx);
        assert!(
            report_panic("a helper broke its own invariant", Some("floor.rs:1:1")),
            "a panic under a served request is the case the hook claims"
        );
        drop(installed);

        let written = String::from_utf8(
            ctx.take_buffered_diagnostic()
                .expect("the diagnostic channel was given a buffer"),
        )
        .expect("a record renders as UTF-8");
        assert!(
            written.contains("a helper broke its own invariant"),
            "the panic's own message is the record's message: {written}"
        );
        assert!(
            written.contains(&request_id),
            "the record carries the id of the request it was raised under: {written}"
        );
        assert!(
            written.contains("floor.rs:1:1"),
            "and where the panic was raised: {written}"
        );
    }

    /// The other side of that split, and why the hook delegates rather than
    /// printing for itself: a `nvs run` has a context but no request, so the
    /// hook installed before this one — the default, which writes to stderr —
    /// is the one that answers, and the request log stays empty.
    #[test]
    fn a_cli_scripts_panic_still_reaches_stderr_through_the_default_hook() {
        let _serialised = HOOK.lock().unwrap_or_else(PoisonError::into_inner);

        let mut ctx = Ctx::buffered();
        ctx.set_diagnostic_sink(OutputSink::Buffer(Vec::new()));
        let installed = CurrentCtx::install(&mut ctx);
        assert!(
            !report_panic("a CLI script's helper panicked", Some("floor.rs:2:1")),
            "a context with no request beneath it is not the hook's case"
        );

        let delegated = Arc::new(AtomicBool::new(false));
        let saw = Arc::clone(&delegated);
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |_| saw.store(true, Ordering::SeqCst)));
        install_panic_hook();
        let outcome: Result<(), _> = run_task(TaskRoot::Worker, || {
            panic!("a CLI script's helper panicked")
        });
        std::panic::set_hook(previous);
        drop(installed);

        assert!(outcome.is_err(), "the panic was contained at the task root");
        assert!(
            delegated.load(Ordering::SeqCst),
            "the hook that was there before this one is what answered"
        );
        assert!(
            ctx.take_buffered_diagnostic()
                .is_some_and(|written| written.is_empty()),
            "and nothing went to the request log, which is what stderr means here"
        );
    }

    /// The standing decision this hook is written under: it changes
    /// presentation and never containment. One panic, both halves asked of it —
    /// the record reached the request log, and [`run_task`] still answers the
    /// fault it answered before rather than a value.
    #[test]
    fn the_hook_changes_presentation_and_never_lets_a_panic_be_recovered() {
        let _serialised = HOOK.lock().unwrap_or_else(PoisonError::into_inner);

        let mut ctx = Ctx::buffered();
        ctx.set_diagnostic_sink(OutputSink::Buffer(Vec::new()));
        ctx.set_inbound(Inbound::new("GET", "/panics", ""));
        let installed = CurrentCtx::install(&mut ctx);

        let previous = std::panic::take_hook();
        install_panic_hook();
        let outcome: Result<u32, _> = run_task(TaskRoot::Request, || {
            panic!("presentation is all this hook changes")
        });
        std::panic::set_hook(previous);
        drop(installed);

        let fault = outcome.expect_err("a hook may not turn a panic into a value");
        assert_eq!(
            fault.message(),
            "presentation is all this hook changes",
            "the panic is contained where it was, carrying what it said"
        );
        let written = String::from_utf8(ctx.take_buffered_diagnostic().unwrap_or_default())
            .expect("a record renders as UTF-8");
        assert!(
            written.contains("presentation is all this hook changes"),
            "and that same panic reached the request log: {written}"
        );
    }

    /// The table's reason for existing —
    /// `rule:errors/a-repeat-is-bounded-at-the-sink-that-suffers`'s "a single
    /// slot coalesces none of that": application code interleaves its records,
    /// and each run of repeats has to be bounded on its own.
    #[test]
    fn two_interleaved_repeats_each_coalesce_rather_than_evicting_one_another() {
        let store = written("the store said no", 12);
        let queue = written("the queue is full", 30);

        assert_eq!(write(&store), Some(1), "each is written the first time");
        assert_eq!(write(&queue), Some(1), "each is written the first time");

        for round in 0..4 {
            assert_eq!(
                write(&store),
                None,
                "round {round}: the repeat is inside its own window"
            );
            assert_eq!(
                write(&queue),
                None,
                "round {round}: and the other record did not close it"
            );
        }
    }

    /// `rule:http-server/the-floor-cannot-fill-the-disk`'s "the next occurrence
    /// after the window carries how many it stands for", asked of the log
    /// target's table rather than of the floor's slot.
    #[test]
    fn the_next_record_after_a_window_closes_carries_how_many_it_stands_for() {
        let record = written("the store said no", 12);

        assert_eq!(write(&record), Some(1), "the first line stands for itself");
        for _ in 0..4 {
            assert_eq!(write(&record), None);
        }

        expire_log_windows();
        assert_eq!(
            write(&record),
            Some(5),
            "the four swallowed occurrences and the one being written"
        );
        assert_eq!(
            write(&record),
            None,
            "and that write opened a window of its own"
        );
    }

    /// The two sinks are two windows: the floor keeps the single slot its own
    /// traffic needs, and neither table can suppress a write the other made.
    #[test]
    fn the_floor_keeps_its_single_slot_and_its_own_window() {
        let store = written("the store said no", 12);
        let queue = written("the queue is full", 30);

        // The floor's slot, asked the traffic the table exists for: every write
        // goes out, because each record displaces the other's window before a
        // repeat of it can be swallowed.
        for round in 0..3 {
            assert_eq!(
                admit(key(&store), Instant::now()),
                Some(1),
                "round {round}: one slot cannot hold two records"
            );
            assert_eq!(admit(key(&queue), Instant::now()), Some(1));
        }

        // The same records through the log target's table, which the floor's
        // reports have not touched.
        assert_eq!(write(&store), Some(1));
        assert_eq!(write(&queue), Some(1));
        assert_eq!(write(&store), None);
        assert_eq!(write(&queue), None);

        assert_eq!(
            admit(key(&queue), Instant::now()),
            None,
            "and the floor's window is still where the floor left it"
        );
    }

    /// The identity is the floor's: the keys that distinguish two *occurrences*
    /// of one record are cleared before hashing, because a limiter that let
    /// them distinguish would never fire.
    #[test]
    fn a_record_differing_only_in_request_id_still_coalesces() {
        let mut first = written("the store said no", 12);
        first.envelope.request_id = Some("req-1".to_owned());
        first.envelope.ts = Some("2026-01-01T00:00:00Z".to_owned());

        let mut second = first.clone();
        second.envelope.request_id = Some("req-2".to_owned());
        second.envelope.ts = Some("2026-01-01T00:00:01Z".to_owned());
        second.envelope.trace_id = Some("trace-2".to_owned());
        second.envelope.span_id = Some("span-2".to_owned());
        second.envelope.count = Some(9);

        assert_eq!(write(&first), Some(1));
        assert_eq!(
            write(&second),
            None,
            "one failure reported by two requests is one failure"
        );
    }

    /// "Everything else counts, `source` included — two identical messages from
    /// two lines are two facts", and a record that differs is never held back.
    #[test]
    fn a_record_differing_in_its_source_line_is_written_immediately() {
        let twelve = written("the store said no", 12);
        let thirty = written("the store said no", 30);

        assert_eq!(write(&twelve), Some(1));
        assert_eq!(
            write(&thirty),
            Some(1),
            "the same message from another line is another record"
        );
        assert_eq!(
            write(&twelve),
            None,
            "and each still bounds its own repeats"
        );

        let mut with_field = twelve.clone();
        with_field
            .envelope
            .fields
            .push(("user".to_owned(), text("7")));
        assert_eq!(
            write(&with_field),
            Some(1),
            "a field the others do not carry is a record they are not"
        );
    }

    /// "Memory stays a constant, just a larger one": the table holds
    /// [`LOG_WINDOW_SLOTS`] windows and no record content, so what a program
    /// writes cannot grow it.
    #[test]
    fn the_windows_footprint_does_not_grow_with_the_number_of_records_written() {
        for line in 1..=10_000 {
            assert_eq!(
                write(&written("the store said no", line)),
                Some(1),
                "line {line} is a record of its own, so it is never held back"
            );
        }

        let open = LOG_WINDOWS.with(|windows| windows.borrow().iter().flatten().count());
        assert_eq!(
            open, LOG_WINDOW_SLOTS,
            "ten thousand records leave the table full, not ten thousand windows deep"
        );
        assert_eq!(
            size_of::<[Option<Window>; LOG_WINDOW_SLOTS]>(),
            LOG_WINDOW_SLOTS * size_of::<Option<Window>>(),
            "the table is its slots inline — a key, a deadline and a count each, no record bytes"
        );
    }
}
