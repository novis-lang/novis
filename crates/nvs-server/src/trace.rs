//! `rule:observability/an-inbound-traceparent-is-continued`'s trace
//! identity — `rule:observability/a-trace-id-exists-for-every-request`'s id — taken at the door: the `traceparent` the request arrived with,
//! read once, before any application code runs.
//!
//! # Why the door, and why it is one call
//!
//! § 2 gives every request a trace id whatever the sampling decision, and makes
//! that id Novis's *only* request identifier — `Core\Server::traceId()` reads
//! it, every `[log]` record carries it, and there is deliberately no second one
//! and no inbound `X-Request-ID` (`rule:http-server/the-trace-id-is-the-request-identifier`). An id that a subsystem drew
//! for itself on first ask would be a second place for two readers to disagree
//! about which trace a request is in, so [`nvs_runtime::Ctx::new`] draws a root
//! eagerly and the only question left is whether *this* request continues
//! somebody else's trace. That question is a header's, and a header is the
//! door's to read: [`take`] answers it beside [`crate::route::take`] and
//! [`crate::mount::carry`], and the answer rides on the carrier
//! ([`nvs_runtime::Inbound::set_trace_context`]) exactly as the peer and the
//! match do.
//!
//! **Whether the trace is sampled is decided here, and only for a trace this
//! request starts.** Sampling is head-based
//! (`rule:observability/sampling-is-head-based`): `[trace] sample` is the
//! fraction a *new* trace is recorded with, and [`take`] is handed it off the
//! snapshot standing when the request arrived, because that key reloads and a
//! rate read once at a boot would be the one an operator can no longer change.
//! A request that continued somebody else's trace carries the flag that
//! arrived, whatever the local fraction — the root already decided, and half a
//! distributed trace is worse than none. The draw is
//! [`nvs_runtime::TraceContext`]'s, made on the one path that roots a trace, so
//! a continued one costs nothing and a rate of `0.0` costs nothing either.
//!
//! # What a sampled request becomes
//!
//! [`spans`] is the other half of this module: the graph a sampled request
//! hands to an exporter, derived at the end of the request from the events the
//! timeline already filed. **Exactly four things are in it** — the root, a
//! `query`, an outbound call and a `spawn` (`rule:observability/four-kinds-become-a-span`) — and a
//! `call` event never is, because a span per compiled call site is the trace no
//! backend can store that `rule:observability/four-kinds-become-a-span` opens by ruling out. A `gc` event is not one
//! either: a collection pause is a histogram in
//! `rule:observability/default-series`, not a unit of work in a request's
//! causal graph.
//!
//! Derivation, rather than a second set of probes, is the whole shape of it:
//! `rule:testing/debug-probes`'s sites stay as cheap as they are and a request
//! that nothing samples does no tracing work at all.
//!
//! # What a bad header does
//!
//! It starts a new trace, and that is the rule rather than a leniency: a
//! `traceparent` arrived from outside, it is `tainted`, and refusing a request
//! over a malformed tracing header would turn an observability feature into an
//! availability one (§ 2). [`nvs_runtime::TraceContext::continuing`] owns the
//! parse and every way one can be unusable; what this module adds is the one
//! way *two* of them can be, below.
//!
//! **What it spends:** one walk of the arrived header lines per request, and
//! the bytes the carrier holds for the answer. A sampled request additionally
//! holds its derived spans, capped at [`SPAN_CEILING`] of them, for as long as
//! it takes to hand them on. An unsampled one holds none, and nothing outlives
//! the request either way.

use nvs_runtime::{Inbound, TraceContext, TraceEvent, TraceKind};
use rand::RngExt;

/// The field name § 2 names, lower case because `hyper` normalises a
/// `HeaderName` on the way in and the carrier keeps what it was given.
const TRACEPARENT: &str = "traceparent";

/// Decides `rule:observability/an-inbound-traceparent-is-continued`'s trace for this request, and records it on the
/// carrier.
///
/// The header is read off `inbound` rather than off `hyper`'s request for
/// [`crate::route::take`]'s reason turned one notch: the carrier already holds
/// one entry per arrived field line, in order, so asking it is both cheaper
/// than a second pass over the head and the only reading that can see the
/// duplicate below at all.
///
/// **Two `traceparent` lines are as unusable as one malformed one.** The W3C
/// format has no rule for combining them and neither of them is more the
/// caller's than the other, so continuing either would be a coin flip recorded
/// as a causal fact — and continuing the joined value is not a trace anybody
/// sent. A new trace is what § 2 already prescribes for a header this process
/// cannot read, and this is one more way for that to be true.
///
/// Something is always written: a request that carried no header at all is a
/// root, which is § 2's "an id exists for every request" stated where the
/// request is.
///
/// `sample` is `[trace] sample` off the snapshot this request reads
/// (`nvs_config::export::head_sample`), and it is consulted only where this
/// request roots a trace — the module doc owns why that is the whole of head
/// sampling, and why a caller with no configuration in hand passes `0.0` rather
/// than a rate of its own.
pub fn take(inbound: &mut Inbound, sample: f64) {
    // The whole reading is one block, so that the walk and the borrow it holds
    // on the carrier are both over before the answer is written back — the
    // shape [`crate::route::take`] uses for the same reason, and the one that
    // makes the "once" visible: one call, one answer, one write.
    let decided = {
        let mut lines = inbound
            .headers()
            .filter(|(name, _)| name.eq_ignore_ascii_case(TRACEPARENT))
            .map(|(_, value)| value);
        let carried = match (lines.next(), lines.next()) {
            (Some(only), None) => std::str::from_utf8(only).ok(),
            // No line, or more than one. Both are "nothing this process can
            // continue", and the doc above owns why the second one is.
            _ => None,
        };
        TraceContext::continuing(carried, sample)
    };
    inbound.set_trace_context(decided);
}

/// The most spans one request contributes, root included.
///
/// A request holds its spans until it ends, so an unbounded count would make a
/// long-running sampled request's memory a function of how many statements it
/// ran — and a program that queries in a loop would be the one paying for it.
/// The bound is a fixed count rather than a share of anything so that the worst
/// case is arithmetic an operator can do: it is per *sampled in-flight*
/// request, and `rule:programs/memory-priority`'s reading of it is in the
/// goal's own accounting.
pub const SPAN_CEILING: usize = 512;

/// Which of `rule:observability/four-kinds-become-a-span`'s four a [`Span`] is.
///
/// There is no variant for a `call` or a `gc` event, which is the rule stated
/// in the type rather than checked at the end of it: a kind that cannot be
/// named cannot be pushed by a later reader that forgot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpanKind {
    /// The request or scheduled run the other spans hang off.
    Root,
    /// One statement, from a `query` event
    /// (`rule:observability/a-query-is-a-trace-event`).
    Query,
    /// One outbound `Core\Http\Client` call, from an `http` event — **one span
    /// however many attempts it took**, because the transport files one event
    /// per call and the attempt count is inside it.
    Http,
    /// One isolate spawn, from a `spawn` event
    /// (`rule:observability/spawn-is-its-own-event`).
    Spawn,
}

/// One span of a sampled request's trace, as [`spans`] derives it.
///
/// The ids are the export's whole point and are carried as bytes rather than
/// rendered: a collector wants them as they are, and a hex rendering is the
/// header's spelling rather than the span's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    /// Which of the four this is.
    pub kind: SpanKind,
    /// The trace it belongs to — the same for every span of one request.
    pub trace_id: [u8; 16],
    /// Its own id, unique within the trace.
    pub span_id: [u8; 8],
    /// What it hangs off: the root's parent is the caller's span, and every
    /// other span's is the root. `None` only on a root that started its trace.
    pub parent: Option<[u8; 8]>,
    /// What the span is *of*: the root's request line, and otherwise the
    /// payload the event was filed with — a `query`'s rendering from
    /// `nvs_db::QuerySpan`, an `http`'s from the transport, a `spawn`'s from
    /// the runtime. It is one string because a [`nvs_runtime::TraceEvent`] is
    /// one, and that type's own doc comment owns why a per-kind field set
    /// waits on `rule:testing/debug-probes`'s sink.
    pub detail: String,
}

/// The spans `request` contributes to its trace — `rule:observability/four-kinds-become-a-span`'s four kinds and
/// nothing else, derived from the events the timeline already filed.
///
/// **A trace nothing is recording produces nothing**, and that is the
/// head-based decision being spent rather than an optimisation: `sampled` is
/// decided once at the root (`rule:observability/an-inbound-traceparent-is-continued`), so a request that is not in a
/// recorded trace does none of this work and holds none of this memory. The
/// events themselves are `rule:testing/debug-probes`'s and were filed whether or not anyone
/// is exporting them.
///
/// `request` is what the root span is *of* — the method and path the door
/// matched, or the schedule entry a run fired for. It is a parameter because
/// this module reads the carrier's headers and not its request line, and a
/// second reading of the line here is a second thing to keep in step with the
/// door.
#[must_use]
pub fn spans(request: &str, trace: &TraceContext, events: &[TraceEvent]) -> Vec<Span> {
    if !trace.sampled() {
        return Vec::new();
    }
    let mut spans = vec![Span {
        kind: SpanKind::Root,
        trace_id: trace.trace_id(),
        span_id: trace.span_id(),
        parent: trace.parent_span_id(),
        detail: request.to_owned(),
    }];
    for event in events {
        // The ceiling counts the root, so a request that only ever spans is
        // still bounded by it; the doc on the constant owns why.
        if spans.len() >= SPAN_CEILING {
            break;
        }
        let kind = match event.kind {
            // The two the rule excludes, and the `match` is exhaustive so a
            // sixth kind arriving is a compile error here rather than a span
            // nobody decided to create.
            TraceKind::Call | TraceKind::Gc => continue,
            TraceKind::Query => SpanKind::Query,
            TraceKind::Http => SpanKind::Http,
            TraceKind::Spawn => SpanKind::Spawn,
        };
        spans.push(Span {
            kind,
            trace_id: trace.trace_id(),
            span_id: draw_span_id(),
            // Flat under the root rather than nested: the events are a flat
            // list in the order they were filed, and only a `spawn`'s child
            // could nest — which is the export-time join
            // `rule:observability/spawn-is-its-own-event` gives the shared
            // timeline epoch for, and not something this list can reconstruct.
            parent: Some(trace.span_id()),
            detail: event.callee.clone(),
        });
    }
    spans
}

/// A span id for a derived span, drawn like the root's: all-zero is the value
/// the W3C format reserves for "absent", so one bit is forced rather than
/// redrawn.
fn draw_span_id() -> [u8; 8] {
    let mut span_id: [u8; 8] = rand::rng().random();
    span_id[0] |= 1;
    span_id
}

#[cfg(test)]
mod tests {
    use super::{SPAN_CEILING, SpanKind, spans, take};
    use nvs_render::Level;
    use nvs_runtime::{Ctx, Inbound, OutputSink, TraceContext, TraceEvent, TraceKind, floor};

    /// A carrier as the door builds one: the request line, then one field line
    /// per entry in arrival order.
    fn arrived(headers: &[(&str, &str)]) -> Inbound {
        let mut inbound = Inbound::new("GET", "/orders/17", "");
        for (name, value) in headers {
            inbound.push_header(name, value.as_bytes());
        }
        inbound
    }

    /// The trace `take` decided under a head rate nothing is recorded at, which
    /// is the shipped default and so what every case that is not about sampling
    /// wants.
    fn decided(headers: &[(&str, &str)]) -> TraceContext {
        decided_at(headers, 0.0)
    }

    /// The trace `take` decided for `headers` under `sample`, which is never
    /// absent — the module doc owns why something is always written.
    fn decided_at(headers: &[(&str, &str)], sample: f64) -> TraceContext {
        let mut inbound = arrived(headers);
        take(&mut inbound, sample);
        inbound
            .trace_context()
            .expect("the door wrote no trace for a request it walked")
    }

    /// One valid inbound header, and the trace it names.
    const INBOUND: &str = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";

    /// `rule:observability/an-inbound-traceparent-is-continued`: an inbound `traceparent` is continued — trace id, parent
    /// span id and sampled flag all adopted — and a request that arrived
    /// without one is a new trace rather than no trace.
    ///
    /// The two halves are one test because the rule is one sentence and the
    /// failure that matters is a door that answers alike for both: a door that
    /// dropped the header would still produce a perfectly good root, and a door
    /// that wrote nothing at all would still leave the eager root
    /// `nvs_runtime::Ctx::new` drew. Only the pair of them says the header was
    /// read.
    #[test]
    fn an_inbound_traceparent_is_continued_and_a_missing_one_is_generated() {
        let continued = decided(&[("host", "localhost"), ("traceparent", INBOUND)]);
        assert_eq!(
            (
                continued.trace_id_hex(),
                continued.parent_span_id(),
                continued.sampled()
            ),
            (
                "4bf92f3577b34da6a3ce929d0e0e4736".to_owned(),
                Some([0x00, 0xf0, 0x67, 0xaa, 0x0b, 0xa9, 0x02, 0xb7]),
                true
            ),
            "the arrived header was not adopted: trace, parent and flag are all it carries"
        );

        let generated = decided(&[("host", "localhost")]);
        assert_ne!(generated.trace_id(), [0; 16]);
        assert_ne!(
            generated.trace_id(),
            continued.trace_id(),
            "a request that carried no header joined somebody else's trace"
        );
        // Two requests with no header are two traces, which is what makes the
        // generated id an identifier rather than a constant.
        assert_ne!(generated, decided(&[("host", "localhost")]));

        // § 2's malformed case, at the door: a header this process cannot read
        // is a new trace and not a refusal. `nvs_runtime::trace_context` owns
        // every way one can be unreadable; what is asserted here is that the
        // door reaches that rule rather than passing the bytes on.
        let broken = decided(&[("traceparent", "00-not-a-trace-01")]);
        assert_ne!(broken.trace_id(), [0; 16]);
        assert_ne!(broken.trace_id(), continued.trace_id());

        // And the door's own unusable case: two field lines are as unreadable
        // as one bad one, because neither is more the caller's than the other.
        let doubled = decided(&[("traceparent", INBOUND), ("traceparent", INBOUND)]);
        assert_ne!(
            doubled.trace_id(),
            continued.trace_id(),
            "one of two traceparent lines was continued anyway"
        );
    }

    /// `rule:observability/a-trace-id-exists-for-every-request`: a trace id exists for every request whatever the sampling
    /// decision — sampling governs whether a trace is *exported*, never whether
    /// an id is generated, which is what lets this id be Novis's only request
    /// identifier.
    ///
    /// Asserted by **counting** over the whole sweep rather than read off one
    /// line: a door that generated an id only for the requests it was going to
    /// record would answer plausibly on the sampled row and leave every log
    /// record of an unsampled request with nothing to correlate on.
    #[test]
    fn a_trace_id_exists_whether_or_not_the_request_is_sampled() {
        let arrivals: [&[(&str, &str)]; 4] = [
            // Sampled, and continued.
            &[("traceparent", INBOUND)],
            // The same trace, not sampled.
            &[(
                "traceparent",
                "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-00",
            )],
            // No header: a root, which the default rate records nothing of.
            &[("host", "localhost")],
            // A header nothing can read: also a root.
            &[("traceparent", "01-4bf92f3577b34da6a3ce929d0e0e4736-x-01")],
        ];
        let traces = arrivals.map(decided);

        let with_an_id = traces
            .iter()
            .filter(|trace| trace.trace_id() != [0; 16] && trace.span_id() != [0; 8])
            .count();
        assert_eq!(
            with_an_id,
            traces.len(),
            "a request was left with no trace id: {traces:?}"
        );

        // The sweep is only worth counting if the sampling decisions in it
        // actually differ — one sampled arrival and three unsampled ones.
        let sampled = traces.iter().filter(|trace| trace.sampled()).count();
        assert_eq!(sampled, 1, "the sweep asked one question four times");
    }

    /// `rule:observability/sampling-is-head-based`: `[trace] sample` is the
    /// probability that a request which *started* a trace is recorded, and an
    /// inbound trace that is already sampled is continued whatever the local
    /// fraction is.
    ///
    /// The case that carries the rule is the *unsampled* header under a rate of
    /// `1.0`. A door that drew for every request rather than for every root
    /// would record it, and would be shipping the tail of a trace whose root
    /// chose not to be recorded — which is the partial trace the rule is written
    /// to prevent, and the one failure the two ends of the rate cannot see.
    #[test]
    fn trace_sample_decides_at_the_root_and_a_sampled_inbound_trace_is_always_continued() {
        /// The fixture's trace, arriving with its sampled bit clear.
        const UNSAMPLED: &str = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-00";

        assert!(
            decided_at(&[("host", "localhost")], 1.0).sampled(),
            "a root under a rate of one was not recorded"
        );
        assert!(
            !decided_at(&[("host", "localhost")], 0.0).sampled(),
            "a root under a rate of zero was recorded"
        );
        assert!(
            decided_at(&[("traceparent", INBOUND)], 0.0).sampled(),
            "an arrived sampled trace was dropped by a rate that is not its to answer"
        );
        assert!(
            !decided_at(&[("traceparent", UNSAMPLED)], 1.0).sampled(),
            "a continued trace was re-decided at this hop"
        );

        // A header this process cannot read is a new trace (§ 2), so it is the
        // head draw's to decide and not the header's — asserted over both of the
        // door's own unusable shapes, each carrying a bit that says *not*
        // recorded, so continuing one would answer the opposite.
        for unreadable in [
            vec![("traceparent", "00-not-a-trace-00")],
            vec![("traceparent", UNSAMPLED), ("traceparent", UNSAMPLED)],
        ] {
            assert!(
                decided_at(&unreadable, 1.0).sampled(),
                "an unreadable header started a trace the head rate never saw: {unreadable:?}"
            );
        }

        // Between the ends it is a draw and not a threshold, which no single
        // request can show: over a sweep at one half both answers appear, while
        // a door that had collapsed the rate to a constant would give one of
        // them every time. A fair draw fails this with probability 2^-999.
        let recorded = (0..1_000)
            .filter(|_| decided_at(&[("host", "localhost")], 0.5).sampled())
            .count();
        assert!(
            (1..1_000).contains(&recorded),
            "a rate of one half recorded {recorded} of 1000 roots"
        );
    }

    /// What the door decided for `headers`, and the one diagnostic line a
    /// record written on a context serving that request produced.
    ///
    /// The record goes out through [`nvs_runtime::floor::report`] — one of
    /// `rule:errors/log-write`'s two writers, and the one this crate can reach — so the
    /// line read back here is rendered by the same serialiser, stamped by the
    /// same [`nvs_runtime::Ctx::stamp_envelope`] and floored by the same
    /// directive as a record `Core\Log::write` produces. Nothing about the ids
    /// is this fixture's: the door writes them and the stamp reads them.
    fn reported(headers: &[(&str, &str)], message: &str) -> (TraceContext, String) {
        let mut inbound = arrived(headers);
        take(&mut inbound, 0.0);
        let carried = inbound
            .trace_context()
            .expect("the door wrote no trace for a request it walked");
        let mut ctx = Ctx::buffered();
        // The floor writes to the diagnostic channel with no `[log] target`
        // configured, and a fresh context points that at stderr.
        ctx.set_diagnostic_sink(OutputSink::Buffer(Vec::new()));
        ctx.set_inbound(inbound);
        floor::report(&mut ctx, &floor::note(Level::Error, message));
        let line = String::from_utf8(
            ctx.take_buffered_diagnostic()
                .expect("a buffered channel hands its bytes back"),
        )
        .expect("a JSON Lines line is text");
        (carried, line)
    }

    /// `rule:observability/a-trace-id-exists-for-every-request` and `rule:observability/a-log-record-carries-trace-ids-when-a-trace-is-active`, across the seam neither crate owns alone: the trace
    /// id a record carries is the one **this door** decided, and not a second
    /// one drawn where the record was written.
    ///
    /// `crates/nvs-stdlib/src/log.rs` pins the record's *shape* — which keys a
    /// served request contributes and which a CLI run omits — against a trace
    /// context the fixture sets by hand. That leaves exactly one thing
    /// unasserted, and it is the thing an operator jumping from a log line to a
    /// trace depends on: that the hand a request's ids actually come from is
    /// [`take`]'s. A door that read the header and a stamp that minted its own
    /// id would each pass their own crate's tests and produce a line no backend
    /// could join to anything.
    ///
    /// Asked on both sides of § 2's sampling bound, because they fail
    /// differently: a continued trace pins the ids against the *arrived*
    /// header, and a root — which has an id and no spans being recorded — pins
    /// that `request_id` still names the door's id rather than falling back to
    /// nothing.
    #[test]
    fn a_requests_log_record_and_its_span_carry_the_same_trace_id() {
        let (continued, line) = reported(
            &[("host", "localhost"), ("traceparent", INBOUND)],
            "the store said no",
        );
        assert!(
            continued.sampled(),
            "the arrived header is sampled, which is what puts spans on the record"
        );
        for (key, want) in [
            ("request_id", continued.trace_id_hex()),
            ("trace_id", continued.trace_id_hex()),
            ("span_id", continued.span_id_hex()),
        ] {
            assert!(
                line.contains(&format!("\"{key}\":\"{want}\"")),
                "`{key}` is not the id the door carried: {line}"
            );
        }
        // And that id is the caller's, so the assertions above are about a
        // continuation rather than about two readings of one fresh root.
        assert!(
            line.contains("\"trace_id\":\"4bf92f3577b34da6a3ce929d0e0e4736\""),
            "the record names a trace the request did not arrive in: {line}"
        );

        // § 2's other side: a request that carried no header is a root, and its
        // record still reads the door's id — the half that makes the trace id
        // Novis's only request identifier rather than a tracing detail.
        let (root, line) = reported(&[("host", "localhost")], "the queue was empty");
        assert!(
            line.contains(&format!("\"request_id\":\"{}\"", root.trace_id_hex())),
            "a root's record named no request: {line}"
        );
        assert_ne!(
            root.trace_id_hex(),
            continued.trace_id_hex(),
            "the root joined the continued request's trace"
        );
    }

    /// One trace event of `kind`, carrying `payload` as the routine that files
    /// it rendered — see [`nvs_runtime::TraceEvent`] for why one field holds
    /// every kind's facts.
    fn event(kind: TraceKind, payload: &str) -> TraceEvent {
        TraceEvent {
            kind,
            callee: payload.to_owned(),
            status: None,
        }
    }

    /// `rule:observability/four-kinds-become-a-span`: the root, a `query`, an outbound call and a `spawn` become
    /// spans, and a `call` never does — nor does a `gc`, which is a histogram
    /// in `rule:observability/default-series` instead.
    ///
    /// The events are built here rather than run out of a served request
    /// because the rule is a statement about the **kind tag**, and one of the
    /// five kinds has no emitter anywhere in the tree yet: nothing files a
    /// `gc` event ([`nvs_runtime::TraceKind::Gc`]'s own doc says so), so a
    /// fixture that ran a real request could never present the case this most
    /// needs pinned — that a collection pause does not enter a request's
    /// causal graph.
    ///
    /// Both directions are asserted over one derivation, because the failure
    /// that matters is a `match` arm rather than a count: a derivation that
    /// dropped the `spawn` and one that also spanned every call site both
    /// produce a plausible-looking list, and only the exact set says which
    /// arms were taken.
    #[test]
    fn exactly_four_event_kinds_become_a_span_and_a_call_never_does() {
        let sampled = decided(&[("traceparent", INBOUND)]);
        assert!(
            sampled.sampled(),
            "the fixture's header is the sampled one, which is what puts spans on this request"
        );

        let events = [
            event(TraceKind::Call, "Orders::total"),
            event(TraceKind::Gc, "gc pause=300us reclaimed=2MiB"),
            event(TraceKind::Query, "select 1 driver=sqlite rows=1"),
            event(
                TraceKind::Http,
                "GET https://api.example.test/v1/pay status=200 attempt=3 hops=1",
            ),
            event(TraceKind::Spawn, "spawn worker started=1ms joined=4ms"),
        ];
        let derived = spans("GET /orders/17", &sampled, &events);

        let kinds: Vec<SpanKind> = derived.iter().map(|span| span.kind).collect();
        assert_eq!(
            kinds,
            [
                SpanKind::Root,
                SpanKind::Query,
                SpanKind::Http,
                SpanKind::Spawn
            ],
            "the four kinds are not the four spans: {derived:?}"
        );
        for excluded in ["Orders::total", "gc pause"] {
            assert!(
                derived.iter().all(|span| !span.detail.contains(excluded)),
                "`{excluded}` reached a span: {derived:?}"
            );
        }

        // A retried outbound call is one span carrying its attempt count, not
        // one span per attempt — the transport files one event per call and
        // the count rides inside it, so the derivation must not read it.
        let outbound: Vec<_> = derived
            .iter()
            .filter(|span| span.kind == SpanKind::Http)
            .collect();
        assert_eq!(outbound.len(), 1, "an attempt became a span of its own");
        assert!(outbound[0].detail.contains("attempt=3"));

        // The graph: every span is in the request's trace, hangs off the root,
        // and has an id of its own. The root's own parent is the caller's
        // span, which is the edge that joins this trace to the one it
        // continued.
        let root = &derived[0];
        assert_eq!(root.span_id, sampled.span_id());
        assert_eq!(root.parent, sampled.parent_span_id());
        for span in &derived[1..] {
            assert_eq!(span.trace_id, sampled.trace_id());
            assert_eq!(span.parent, Some(root.span_id), "a span left the root");
        }
        let mut ids: Vec<[u8; 8]> = derived.iter().map(|span| span.span_id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(
            ids.len(),
            derived.len(),
            "two spans of one trace share an id"
        );

        // Counted rather than eyeballed: a hundred call sites are still one
        // span, which is the per-call trace the rule opens by ruling out.
        let calls: Vec<TraceEvent> = (0..100)
            .map(|line| event(TraceKind::Call, &format!("Orders::line{line}")))
            .collect();
        assert_eq!(
            spans("GET /orders/17", &sampled, &calls).len(),
            1,
            "a call site became a span"
        );

        // A trace nobody is recording produces none of this, and the ceiling
        // holds for one that is — both are memory bounds rather than tidiness,
        // and `SPAN_CEILING`'s own doc owns why the cap is a fixed count.
        let unsampled = decided(&[("host", "localhost")]);
        assert!(
            spans("GET /orders/17", &unsampled, &events).is_empty(),
            "an unrecorded trace built spans nobody will ever read"
        );
        let many: Vec<TraceEvent> = (0..SPAN_CEILING * 2)
            .map(|row| event(TraceKind::Query, &format!("select {row}")))
            .collect();
        assert_eq!(
            spans("GET /orders/17", &sampled, &many).len(),
            SPAN_CEILING,
            "a request in a loop held an unbounded number of spans"
        );
    }
}
