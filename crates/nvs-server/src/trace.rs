//! `rule:observability/an-inbound-traceparent-is-continued`'s trace
//! identity — `rule:observability/a-trace-id-exists-for-every-request`'s id — taken at the door: the `traceparent` the request arrived with,
//! read once, before any application code runs.
//!
//! # Why the door, and why it is one call
//!
//! Every request has a trace id whatever the sampling decision, and that id is
//! Novis's *only* request identifier — `Core\Server::traceId()` reads
//! it, every `[log]` record carries it, and there is deliberately no second one
//! and no inbound `X-Request-ID` (`rule:observability/a-trace-id-exists-for-every-request`). An id that a subsystem drew
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
//! Where the graph goes is the `otlp` module's queue, which is the whole of what
//! handing it on costs a request — the crate doc's § *The `exporter` feature*
//! is why this paragraph names that module rather than linking it, and why a
//! build without the feature derives no spans at all.
//!
//! [`record`] is the two of those as one call, and it is the **one** place a
//! finished run reaches an exporter from: the door makes it where a request's
//! isolate is joined ([`crate::serve`], both for a buffered answer and for a
//! streamed one) and the ticker where a fire ends ([`crate::schedule`]). The
//! events it derives from arrive on the completion — a context is the child's
//! and is gone by then, so `nvs_runtime::host::Completion::trace` is the
//! channel — and a run nobody sampled carries none.
//!
//! # What a bad header does
//!
//! It starts a new trace, and that is the rule rather than a leniency: a
//! `traceparent` arrived from outside, it is `tainted`, and refusing a request
//! over a malformed tracing header would turn an observability feature into an
//! availability one (`rule:observability/an-inbound-traceparent-is-continued`).
//! [`nvs_runtime::TraceContext::continuing`] owns the
//! parse and every way one can be unusable; what this module adds is the one
//! way *two* of them can be, below.
//!
//! **What it spends:** one walk of the arrived header lines per request, and
//! the bytes the carrier holds for the answer. A sampled request additionally
//! holds the events it files and the spans derived from them, capped at
//! [`SPAN_CEILING`] of them, for as long as it takes to hand them on. An
//! unsampled one holds none, and nothing outlives the request either way.
//!
//! # What a filing site asks
//!
//! `nvs_runtime::Ctx::records_spans`, and never whether a debugger is attached:
//! a `query`, an `http` and a `spawn` are filed from routines that are already
//! rare, so a recorded request files all three while paying nothing a normal one
//! does not. `nvs_runtime::DebugFlags::TRACE` answers that question too and adds
//! a `call` event per compiled call site, which is why the two are one question
//! and not one bit — `rule:observability/four-kinds-become-a-span` refuses
//! that cost on a served request outright, and [`spans`] drops the kind here as
//! well. What a recorded request holds is bounded where it is filed
//! (`nvs_runtime::SPAN_EVENT_CEILING`), which is what [`SPAN_CEILING`] counts.

use nvs_runtime::{Inbound, TraceContext, TraceEvent, TraceKind};
use rand::RngExt;
use std::time::Duration;

/// The field name W3C Trace Context names, lower case because `hyper` normalises a
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
/// sent. A new trace is what `rule:observability/an-inbound-traceparent-is-continued`
/// already prescribes for a header this process cannot read, and this is one
/// more way for that to be true.
///
/// Something is always written: a request that carried no header at all is a
/// root, which is `rule:observability/a-trace-id-exists-for-every-request` stated where the
/// request is.
///
/// `sample` is `[trace] sample` off the snapshot this request reads
/// (`nvs_config::export::head_sample`), and it is consulted only where this
/// request roots a trace — the module doc owns why that is the whole of head
/// sampling. It is `None` where no `[trace]` exporter is configured, and then
/// nothing is recorded: not by a draw, and not by the flag a client sent. A
/// caller with no configuration in hand passes `None`.
pub fn take(inbound: &mut Inbound, sample: Option<f64>) {
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
        match sample {
            Some(sample) => TraceContext::continuing(carried, sample),
            None => TraceContext::continuing(carried, 0.0).unrecorded(),
        }
    };
    inbound.set_trace_context(decided);
}

/// The most spans one request contributes, root included: the root, plus every
/// event a recorded run is allowed to file.
///
/// **The count itself is `nvs_runtime::SPAN_EVENT_CEILING`'s**, because that is
/// where filing stops and where the memory is held — a request keeps its events
/// until it ends, and that constant's doc comment owns why the bound is a fixed
/// count and what an operator multiplies it by. Derived here rather than chosen
/// again so that the two cannot drift into a graph truncated somewhere other
/// than where the run stopped filing.
///
/// A graph built from a real run therefore never reaches this, and the walk that
/// enforces it is the backstop for a `Completion` assembled by hand.
pub const SPAN_CEILING: usize = nvs_runtime::SPAN_EVENT_CEILING + 1;

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
    /// When the span began, as an `nvs_runtime::trace_clock` reading: the
    /// event's start, or for the root the reading the door took before the
    /// run. `nvs_runtime::unix_nanos` puts it on a collector's clock.
    pub start: Duration,
    /// When it ended, on the same clock: the event's end, or for the root the
    /// reading [`record`] takes once the run has ended. So the root's window
    /// contains every other span's.
    pub end: Duration,
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
///
/// `root` is the root span's window, as two `nvs_runtime::trace_clock`
/// readings: one taken before the run and one after it. A parameter for the
/// same reason as `request`, and so that a derivation reads no clock itself.
#[must_use]
pub fn spans(
    request: &str,
    trace: &TraceContext,
    root: (Duration, Duration),
    events: &[TraceEvent],
) -> Vec<Span> {
    if !trace.sampled() {
        return Vec::new();
    }
    let mut spans = vec![Span {
        kind: SpanKind::Root,
        trace_id: trace.trace_id(),
        span_id: trace.span_id(),
        parent: trace.parent_span_id(),
        detail: request.to_owned(),
        start: root.0,
        end: root.1,
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
            // `rule:observability/spawn-is-its-own-event` puts every event on
            // one clock for, and needs the child's events, which this list
            // does not carry.
            parent: Some(trace.span_id()),
            detail: event.callee.clone(),
            start: event.start,
            end: event.end,
        });
    }
    spans
}

/// Derives a finished run's spans and hands them to the process's exporter —
/// the whole of what tracing costs the request that was traced.
///
/// One call at the end of a request, and one for a scheduled run, so that
/// "where the graph goes" has a single home rather than one per door. What it
/// does is [`spans`] and `otlp::queue`'s `hand_over`: a stamp, a lock and a
/// push, with a full queue dropping its oldest trace rather than making a
/// request wait. A collector that is unreachable costs a batch and never a
/// response — the drain is somebody else's task, and it is the only thing that
/// dials.
///
/// **A build without the `exporter` feature derives nothing here**, which is
/// the crate doc's § *The `exporter` feature* spelled as the one line that
/// would otherwise do the work: there is no queue to hand a graph to, so a
/// request pays a load and a branch instead of a walk of its events.
///
/// `request` is what the root span is *of* — [`spans`] owns why it is a
/// parameter, and the two callers give it the method and path the door matched
/// and the `[[schedule]]` entry's name. `started` is the root's start, read
/// before the run; its end is read here, once the run has ended.
pub fn record(request: &str, trace: &TraceContext, started: Duration, events: &[TraceEvent]) {
    #[cfg(feature = "exporter")]
    crate::otlp::queue().hand_over(spans(
        request,
        trace,
        (started, nvs_runtime::trace_clock()),
        events,
    ));
    #[cfg(not(feature = "exporter"))]
    let _ = (request, trace, started, events);
}

/// The field name W3C Trace Context gives the id a server answers with.
const TRACERESPONSE: &str = "traceresponse";

/// Writes `trace` on a response's head as `traceresponse`, in the
/// `traceparent` shape: `00-<trace id>-<this request's span id>-<flags>`.
///
/// `rule:observability/a-trace-id-exists-for-every-request` emits the id on
/// the response so a proxy logs it with one `log_format` line, and it is
/// written sampled or not because the id exists either way. The flag says
/// which, so a backend that finds no trace for a `-00` id knows why.
///
/// It replaces a `traceresponse` the program set for itself: the request has
/// one id, and the door decided it. An inbound `X-Request-ID` reaches neither
/// the trace nor this header.
///
/// `on` is `[trace] response_header` off the snapshot the request ran under
/// ([`response_header_on`]). With it `false`, the header is removed, a
/// program's own included, and none is written
/// (`rule:observability/the-traceresponse-header-can-be-turned-off`); the id
/// still reaches the access line and the spans.
///
/// **What it spends:** the 55-byte value, once per response, and nothing with
/// the header off.
pub fn respond(headers: &mut hyper::HeaderMap, trace: &TraceContext, on: bool) {
    let name = hyper::header::HeaderName::from_static(TRACERESPONSE);
    if !on {
        headers.remove(name);
        return;
    }
    // Always 55 bytes of lower-case hex and dashes, so it is a valid value.
    if let Ok(value) = hyper::header::HeaderValue::from_str(&trace.traceparent()) {
        headers.insert(name, value);
    }
}

/// `[trace] response_header` as `snapshot` has it: `true` unless the file
/// wrote `false`. Read per response off the snapshot the request ran under,
/// so a reload reaches the next response, at the cost [`access_on`] states.
#[must_use]
pub fn response_header_on(snapshot: &std::sync::Arc<nvs_config::Snapshot>) -> bool {
    snapshot.trace_response_header()
}

/// `[log] access` as `snapshot` has it: what the file wrote, or the mode's
/// default (`rule:config/a-mode-is-five-defaults`).
///
/// Read per request off the snapshot the request ran under, so an `[[app]]`
/// block that writes its own value and a reload both reach the next request.
/// What it costs is one read of the limits the snapshot resolved once, and
/// only for a response that is not a `5xx`.
#[must_use]
pub fn access_on(snapshot: &std::sync::Arc<nvs_config::Snapshot>) -> bool {
    snapshot.access_log()
}

/// What `rule:http-server/the-access-log-is-a-mode-default` puts on one access
/// line: the facts the door holds once a response exists.
///
/// `path` is the path alone, never the query string, for the reason a span's
/// name carries none (`rule:observability/trace-events-carry-a-kind`). `bytes`
/// is `None` for a body whose length is not known when the head goes out — a
/// stream. The door passes a `trace` for every response; an answer it wrote
/// before any carrier existed, such as a refusal at the valve, carries a new
/// root drawn for it.
#[derive(Clone, Copy, Debug)]
pub struct Access<'a> {
    /// Whether the response is the health probe's
    /// ([`crate::serve::is_probe`]), which writes no line at all.
    pub probe: bool,
    /// The verb, as the request line spelled it.
    pub method: &'a str,
    /// The request target's path.
    pub path: &'a str,
    /// The status this connection wrote.
    pub status: u16,
    /// From the door first having the request to the response existing.
    pub took: std::time::Duration,
    /// The response body's length, where it is known.
    pub bytes: Option<u64>,
    /// `Core\Request::clientIp()`'s answer.
    pub client_ip: Option<std::net::IpAddr>,
    /// `[log] client_ip` for the snapshot the request ran under. When `false`
    /// the line has no `clientIp` field at all, not a `null` one
    /// (`rule:http-server/a-log-line-can-leave-out-the-client-address`).
    pub client_ip_logged: bool,
    /// `rule:observability/route-label-is-the-path-pattern`'s label, where a
    /// route matched.
    pub route: Option<&'a str>,
    /// The trace the door decided for this request.
    pub trace: Option<TraceContext>,
}

/// Writes `line` as one diagnostic record where `[log] target` says, when
/// `on` — `[log] access` for the snapshot the request ran under — or when the
/// status is a `5xx` other than `501`, whatever `on` says.
///
/// **The same record a `Core\Log::write` produces**, rendered by the same
/// [`Ctx::write_log_record`](nvs_runtime::Ctx::write_log_record) in whichever
/// `[log] format` is configured, so there is no access-log format of its own.
/// An access line is `Info`; a `5xx` is `Error`, because it is a diagnostic
/// that has to survive the production default level. A `501` is an access
/// line like a `4xx`: the door answers it to a verb the client made up, so a
/// client could otherwise force an error line per request.
///
/// The envelope's ids are filled here rather than by
/// `nvs_runtime::Ctx::stamp_envelope`: `ctx` is the connection's context, and
/// the request's carrier — which that stamp reads — went down with the
/// request. The ids are the same ones, read off the trace the door decided.
///
/// The health probe writes no line at all, whatever its status.
pub fn access(ctx: &mut nvs_runtime::Ctx, on: bool, line: &Access<'_>) {
    use nvs_render::{Level, Node, Scalar};
    use nvs_runtime::floor;

    let failed = line.status >= 500 && line.status != 501;
    if line.probe || (!on && !failed) {
        return;
    }
    let level = if failed { Level::Error } else { Level::Info };
    if !ctx.log_writes(level) {
        return;
    }
    let mut record = floor::note(
        level,
        &format!("{} {} {}", line.method, line.path, line.status),
    );
    let envelope = &mut record.envelope;
    envelope.ts = Some(jiff::Timestamp::now().to_string());
    if let Some(trace) = &line.trace {
        envelope.request_id = Some(trace.request_id_hex());
        if trace.sampled() || trace.continued() {
            envelope.trace_id = Some(trace.trace_id_hex());
            envelope.span_id = Some(trace.span_id_hex());
        }
    }
    let fields = &mut envelope.fields;
    fields.push(("method".into(), floor::text(line.method)));
    fields.push(("path".into(), floor::text(line.path)));
    fields.push((
        "status".into(),
        Node::Scalar(Scalar::Uint(u64::from(line.status))),
    ));
    fields.push((
        "durationMs".into(),
        Node::Scalar(Scalar::Float(line.took.as_secs_f64() * 1000.0)),
    ));
    if let Some(bytes) = line.bytes {
        fields.push(("bytes".into(), Node::Scalar(Scalar::Uint(bytes))));
    }
    if line.client_ip_logged {
        let client = line.client_ip.map_or(Node::Scalar(Scalar::Null), |ip| {
            floor::text(&ip.to_string())
        });
        fields.push(("clientIp".into(), client));
    }
    if let Some(route) = line.route {
        fields.push(("route".into(), floor::text(route)));
    }
    // A sink that has gone is ignored, for `floor::report`'s reason.
    let _ = ctx.write_log_record(&record, nvs_runtime::LogChannel::Diagnostic);
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
    #[cfg(feature = "exporter")]
    use super::record;
    use super::{SPAN_CEILING, SpanKind, spans, take};
    use nvs_render::Level;
    use nvs_runtime::{Ctx, Inbound, OutputSink, TraceContext, TraceEvent, TraceKind, floor};
    use std::time::Duration;

    /// A carrier as the door builds one: the request line, then one field line
    /// per entry in arrival order.
    fn arrived(headers: &[(&str, &str)]) -> Inbound {
        let mut inbound = Inbound::new("GET", "/orders/17", "");
        for (name, value) in headers {
            inbound.push_header(name, value.as_bytes());
        }
        inbound
    }

    /// The trace `take` decided with an exporter configured, under a head rate
    /// nothing is recorded at, which is the shipped default and so what every
    /// case that is not about sampling wants.
    fn decided(headers: &[(&str, &str)]) -> TraceContext {
        decided_at(headers, Some(0.0))
    }

    /// The trace `take` decided for `headers` under `sample`, which is never
    /// absent — the module doc owns why something is always written.
    fn decided_at(headers: &[(&str, &str)], sample: Option<f64>) -> TraceContext {
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

        // The malformed case, at the door: a header this process cannot read
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
            decided_at(&[("host", "localhost")], Some(1.0)).sampled(),
            "a root under a rate of one was not recorded"
        );
        assert!(
            !decided_at(&[("host", "localhost")], Some(0.0)).sampled(),
            "a root under a rate of zero was recorded"
        );
        assert!(
            decided_at(&[("traceparent", INBOUND)], Some(0.0)).sampled(),
            "an arrived sampled trace was dropped by a rate that is not its to answer"
        );
        assert!(
            !decided_at(&[("traceparent", UNSAMPLED)], Some(1.0)).sampled(),
            "a continued trace was re-decided at this hop"
        );

        // A header this process cannot read is a new trace
        // (`rule:observability/an-inbound-traceparent-is-continued`), so it is the
        // head draw's to decide and not the header's — asserted over both of the
        // door's own unusable shapes, each carrying a bit that says *not*
        // recorded, so continuing one would answer the opposite.
        for unreadable in [
            vec![("traceparent", "00-not-a-trace-00")],
            vec![("traceparent", UNSAMPLED), ("traceparent", UNSAMPLED)],
        ] {
            assert!(
                decided_at(&unreadable, Some(1.0)).sampled(),
                "an unreadable header started a trace the head rate never saw: {unreadable:?}"
            );
        }

        // Between the ends it is a draw and not a threshold, which no single
        // request can show: over a sweep at one half both answers appear, while
        // a door that had collapsed the rate to a constant would give one of
        // them every time. A fair draw fails this with probability 2^-999.
        let recorded = (0..1_000)
            .filter(|_| decided_at(&[("host", "localhost")], Some(0.5)).sampled())
            .count();
        assert!(
            (1..1_000).contains(&recorded),
            "a rate of one half recorded {recorded} of 1000 roots"
        );
    }

    /// `rule:observability/sampling-is-head-based`: with no `[trace] exporter`
    /// nothing is recorded, and a client's sampled flag does not change that.
    ///
    /// A recorded request files events and derives spans, and with nothing to
    /// ship them the flag would only make the request hold memory. The trace
    /// id is still continued, because a log line needs it.
    #[test]
    fn with_no_exporter_a_sampled_header_records_nothing() {
        let continued = decided_at(&[("traceparent", INBOUND)], None);
        assert!(
            !continued.sampled(),
            "a client's flag recorded a request nothing ships"
        );
        assert_eq!(
            continued.trace_id_hex(),
            "4bf92f3577b34da6a3ce929d0e0e4736",
            "the trace id was dropped with the flag"
        );
        assert!(
            spans("GET /", &continued, ROOT, &[]).is_empty(),
            "an unrecorded request derived spans"
        );
        assert!(!decided_at(&[("host", "localhost")], None).sampled());
    }

    /// `rule:observability/a-trace-id-exists-for-every-request`: the request id
    /// is always one this process drew. A client that sends the same
    /// `traceparent` twice gets two request ids, and a root's request id is its
    /// trace id.
    #[test]
    fn a_continued_trace_gets_a_request_id_the_client_did_not_choose() {
        let first = decided(&[("traceparent", INBOUND)]);
        let second = decided(&[("traceparent", INBOUND)]);
        assert_eq!(first.trace_id(), second.trace_id());
        assert_ne!(first.request_id(), first.trace_id());
        assert_ne!(
            first.request_id(),
            second.request_id(),
            "two requests share the request id a client chose"
        );
        let root = decided(&[("host", "localhost")]);
        assert_eq!(root.request_id(), root.trace_id());
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
        take(&mut inbound, Some(0.0));
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
    /// Asked on both sides of the sampling bound (`rule:observability/sampling-is-head-based`), because they fail
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
            ("request_id", continued.request_id_hex()),
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

        // The other side: a request that carried no header is a root, and its
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
            start: Duration::ZERO,
            end: Duration::ZERO,
        }
    }

    /// A root window for the cases that assert what is derived and not when.
    const ROOT: (Duration, Duration) = (Duration::ZERO, Duration::ZERO);

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
        let derived = spans("GET /orders/17", &sampled, ROOT, &events);

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
            spans("GET /orders/17", &sampled, ROOT, &calls).len(),
            1,
            "a call site became a span"
        );

        // A trace nobody is recording produces none of this, and the ceiling
        // holds for one that is — both are memory bounds rather than tidiness,
        // and `SPAN_CEILING`'s own doc owns why the cap is a fixed count.
        let unsampled = decided(&[("host", "localhost")]);
        assert!(
            spans("GET /orders/17", &unsampled, ROOT, &events).is_empty(),
            "an unrecorded trace built spans nobody will ever read"
        );
        let many: Vec<TraceEvent> = (0..SPAN_CEILING * 2)
            .map(|row| event(TraceKind::Query, &format!("select {row}")))
            .collect();
        assert_eq!(
            spans("GET /orders/17", &sampled, ROOT, &many).len(),
            SPAN_CEILING,
            "a request in a loop held an unbounded number of spans"
        );
    }

    /// [`record`]'s whole claim: what a finished run derived reaches the queue
    /// the drain empties, and a run nobody is recording puts nothing in it.
    ///
    /// The process-wide queue is what is asserted against rather than a
    /// [`crate::otlp::Pending`] of the test's own, because that is the seam the
    /// case is about — [`record`] is the only writer to it in this binary, so
    /// the graphs taken out of it are this case's two and nothing else's. The
    /// span count is the second claim: a `call` event among the events is
    /// filed and does not become a span
    /// (`rule:observability/four-kinds-become-a-span`).
    #[cfg(feature = "exporter")]
    #[test]
    fn a_recorded_runs_spans_reach_the_queue_and_an_unsampled_runs_do_not() {
        let events = [
            event(TraceKind::Query, "select 1 -- db=main rows=1"),
            event(TraceKind::Call, "Orders::total"),
        ];
        record(
            "GET /orders/17",
            &decided_at(&[], Some(1.0)),
            Duration::ZERO,
            &events,
        );
        record("GET /orders/18", &decided(&[]), Duration::ZERO, &events);

        let queued = crate::otlp::queue().take(usize::MAX);
        let roots: Vec<&str> = queued
            .iter()
            .filter_map(|recorded| recorded.spans.first())
            .map(|root| root.detail.as_str())
            .collect();
        assert!(
            !roots.contains(&"GET /orders/18"),
            "a run nobody is recording queued a graph anyway: {roots:?}"
        );
        let recorded = queued
            .iter()
            .find(|recorded| {
                recorded
                    .spans
                    .first()
                    .is_some_and(|root| root.detail == "GET /orders/17")
            })
            .expect("a sampled run's graph never reached the queue the drain empties");
        assert_eq!(
            recorded.spans.len(),
            2,
            "the root and the `query` are the spans; a `call` became one, or the `query` did not"
        );
    }

    /// The snapshot of `written`, both halves filled, as the door holds one.
    fn tree_of(written: &str) -> std::sync::Arc<nvs_config::Snapshot> {
        std::sync::Arc::new(nvs_config::Snapshot {
            config: toml::from_str(written).expect("the tree deserializes"),
            table: written.parse().expect("the tree is TOML"),
            ..Default::default()
        })
    }

    /// One access line for `status`, as the door writes it, read back off a
    /// buffered diagnostic channel. Empty where nothing was written.
    fn logged(on: bool, probe: bool, status: u16, trace: Option<TraceContext>) -> String {
        logged_naming(on, probe, status, trace, true)
    }

    /// [`logged`], under a `[log] client_ip` of `client_ip_logged`.
    fn logged_naming(
        on: bool,
        probe: bool,
        status: u16,
        trace: Option<TraceContext>,
        client_ip_logged: bool,
    ) -> String {
        let mut ctx = Ctx::buffered();
        ctx.set_diagnostic_sink(OutputSink::Buffer(Vec::new()));
        super::access(
            &mut ctx,
            on,
            &super::Access {
                probe,
                method: "GET",
                path: "/orders/17",
                status,
                took: std::time::Duration::from_millis(12),
                bytes: Some(42),
                client_ip: Some(std::net::IpAddr::from([192, 0, 2, 7])),
                client_ip_logged,
                route: Some("orders.show"),
                trace,
            },
        );
        String::from_utf8(
            ctx.take_buffered_diagnostic()
                .expect("a buffered channel hands its bytes back"),
        )
        .expect("a JSON Lines line is text")
    }

    /// `rule:http-server/the-access-log-is-a-mode-default`: one line per
    /// request, through the diagnostic record, with every field the rule names
    /// and the trace id the door decided. `[log] access` is the mode's default
    /// where the file writes none: on in development and off in production.
    #[test]
    fn an_access_line_carries_method_path_status_duration_bytes_and_trace_id() {
        let trace = decided(&[("traceparent", INBOUND)]);
        let line = logged(true, false, 200, Some(trace));
        assert_eq!(line.lines().count(), 1, "one request is one line: {line}");
        for field in [
            "\"method\":\"GET\"",
            "\"path\":\"/orders/17\"",
            "\"status\":200",
            "\"durationMs\":12",
            "\"bytes\":42",
            "\"clientIp\":\"192.0.2.7\"",
            "\"route\":\"orders.show\"",
            "\"trace_id\":\"4bf92f3577b34da6a3ce929d0e0e4736\"",
        ] {
            assert!(line.contains(field), "the line has no {field}: {line}");
        }
        assert!(
            line.contains(&format!("\"request_id\":\"{}\"", trace.request_id_hex())),
            "the line's request id is not the one the door drew: {line}"
        );
        assert!(
            line.contains("\"level\":\"info\"") || line.contains("\"level\":\"Info\""),
            "an access line is `Info`: {line}"
        );

        assert!(
            super::access_on(&tree_of("[mode]\ndefault = \"development\"\n")),
            "development writes access lines by default"
        );
        assert!(
            !super::access_on(&tree_of("")),
            "production writes none by default"
        );
        assert!(
            super::access_on(&tree_of("[log]\naccess = true\n")),
            "a written `access = true` is read"
        );
    }

    /// `rule:http-server/a-log-line-can-leave-out-the-client-address`: with
    /// `[log] client_ip = false` the `5xx` line, which is written whatever
    /// `access` says, has no `clientIp` field at all, and neither has an access
    /// line. With the key unwritten both name the address.
    // covers: directive:log.client_ip
    #[test]
    fn client_ip_false_leaves_the_address_out_of_the_5xx_and_the_access_line() {
        assert!(
            !super::access_on(&tree_of("[log]\nclient_ip = false\n")),
            "the key turned the access log on"
        );
        for (on, status) in [(false, 500), (true, 200)] {
            let named = logged_naming(on, false, status, None, true);
            assert!(
                named.contains("\"clientIp\":\"192.0.2.7\""),
                "with the key unwritten a {status} line names the address: {named}"
            );
            let left_out = logged_naming(on, false, status, None, false);
            assert_eq!(
                left_out.lines().count(),
                1,
                "the {status} line is still written"
            );
            assert!(
                !left_out.contains("clientIp") && !left_out.contains("192.0.2.7"),
                "a {status} line under `client_ip = false` names the address: {left_out}"
            );
        }

        assert!(
            tree_of("").client_ip_logged(),
            "an unwritten `client_ip` logs the address"
        );
        assert!(
            tree_of("[log]\nclient_ip = true\n").client_ip_logged(),
            "a written `client_ip = true` logs the address"
        );
        assert!(
            !tree_of("[log]\nclient_ip = false\n").client_ip_logged(),
            "a written `client_ip = false` is not read"
        );
    }

    /// The health probe writes no line, while it is accepting and while it
    /// drains with a `503`, and with `access` on. The response
    /// [`crate::Reply::health`] builds is the one asked, so a probe that lost
    /// its mark fails here.
    #[test]
    fn the_health_probe_writes_no_access_line() {
        let draining = crate::serve::Draining::detached();
        let crate::Reply::Done(response) = crate::Reply::health(&draining) else {
            panic!("the probe is answered by the door")
        };
        assert!(crate::serve::is_probe(&response), "the probe is not marked");
        let status = response.status().as_u16();
        assert_eq!(logged(true, true, status, None), "", "the probe was logged");
        assert_eq!(
            logged(true, true, 503, None),
            "",
            "a draining probe's `503` was logged"
        );

        // The control: the same response without the mark is a line.
        assert_ne!(logged(true, false, status, None), "");
        let crate::Reply::Done(other) = crate::Reply::healthy() else {
            panic!("a status reply is the door's")
        };
        assert!(!crate::serve::is_probe(&other), "an ordinary 200 is marked");
    }

    /// The `traceresponse` value [`super::respond`] writes for `trace` on a
    /// response that already has `written` on it, with the header on.
    fn responded(written: &[(&str, &str)], trace: &TraceContext) -> Vec<String> {
        responded_with(written, trace, true)
    }

    /// [`responded`], with `[trace] response_header` as `on` says.
    fn responded_with(written: &[(&str, &str)], trace: &TraceContext, on: bool) -> Vec<String> {
        let mut headers = hyper::HeaderMap::new();
        for (name, value) in written {
            headers.append(
                hyper::header::HeaderName::from_bytes(name.as_bytes()).expect("a token"),
                hyper::header::HeaderValue::from_str(value).expect("printable"),
            );
        }
        super::respond(&mut headers, trace, on);
        headers
            .get_all("traceresponse")
            .iter()
            .map(|value| value.to_str().expect("ASCII").to_owned())
            .collect()
    }

    /// `rule:observability/a-trace-id-exists-for-every-request`: the response
    /// names the request's trace id in one `traceresponse` header, whether the
    /// trace is sampled or not. The flag on the value says which.
    ///
    /// The unsampled root is the row that carries the rule: a door that wrote
    /// the header only for recorded requests passes the sampled row. A value
    /// the program set for itself is replaced, so the response names one id.
    #[test]
    fn every_response_carries_its_trace_id_sampled_or_not() {
        let sampled = decided(&[("traceparent", INBOUND)]);
        assert!(sampled.sampled(), "the fixture's inbound trace is sampled");
        assert_eq!(
            responded(&[], &sampled),
            [format!(
                "00-4bf92f3577b34da6a3ce929d0e0e4736-{}-01",
                sampled.span_id_hex()
            )],
            "a sampled request's response does not name its trace"
        );

        let unsampled = decided(&[]);
        assert!(!unsampled.sampled(), "a root at rate 0 is not sampled");
        assert_eq!(
            responded(&[], &unsampled),
            [format!(
                "00-{}-{}-00",
                unsampled.trace_id_hex(),
                unsampled.span_id_hex()
            )],
            "an unsampled request's response does not name its trace"
        );

        let replaced = responded(
            &[(
                "traceresponse",
                "00-0123456789abcdef0123456789abcdef-0123456789abcdef-01",
            )],
            &unsampled,
        );
        assert_eq!(
            replaced,
            [unsampled.traceparent()],
            "the response named a second id beside the door's"
        );
    }

    /// `rule:observability/the-traceresponse-header-can-be-turned-off`: the
    /// key is on unless the file writes `false`, and off removes the header
    /// and nothing else. A program's own `traceresponse` is removed too, and
    /// the access line still names the request's trace id.
    #[test]
    fn traceresponse_is_on_by_default_and_false_removes_it_and_nothing_else() {
        assert!(
            super::response_header_on(&tree_of("")),
            "a tree that writes nothing sends no `traceresponse`"
        );
        assert!(
            super::response_header_on(&tree_of("[trace]\nresponse_header = true\n")),
            "`response_header = true` sends no `traceresponse`"
        );
        assert!(
            !super::response_header_on(&tree_of("[trace]\nresponse_header = false\n")),
            "`response_header = false` still sends the header"
        );

        let trace = decided(&[]);
        assert_eq!(
            responded_with(&[], &trace, false),
            Vec::<String>::new(),
            "the header is off and the door wrote one"
        );
        assert_eq!(
            responded_with(
                &[(
                    "traceresponse",
                    "00-0123456789abcdef0123456789abcdef-0123456789abcdef-01",
                )],
                &trace,
                false,
            ),
            Vec::<String>::new(),
            "the header is off and the program's own value went out as the request's id"
        );

        let line = logged(true, false, 200, Some(trace));
        assert!(
            line.contains(&trace.trace_id_hex()),
            "the header is off and the access line lost the trace id: {line}"
        );
    }

    /// `rule:observability/a-trace-id-exists-for-every-request`: an inbound
    /// `X-Request-ID` is ignored. It is not the trace id, it is not on the
    /// response and it is not on the access line.
    #[test]
    fn an_inbound_x_request_id_appears_nowhere() {
        const SENT: &str = "0af7651916cd43dd8448eb211c80319c";
        let trace = decided(&[("x-request-id", SENT)]);
        assert_ne!(trace.trace_id_hex(), SENT, "the header became the trace id");

        let mut headers = hyper::HeaderMap::new();
        super::respond(&mut headers, &trace, true);
        assert_eq!(headers.len(), 1, "the response carries more than its trace");
        for (name, value) in &headers {
            assert_ne!(name.as_str(), "x-request-id", "the header was echoed");
            let value = value.to_str().expect("ASCII");
            assert!(
                !value.contains(SENT),
                "the response names the header: {value}"
            );
        }

        let line = logged(true, false, 200, Some(trace));
        assert!(
            line.contains(&trace.trace_id_hex()),
            "the control: the access line names the trace: {line}"
        );
        assert!(
            !line.to_lowercase().contains("x-request-id") && !line.contains(SENT),
            "the access line names the header: {line}"
        );
    }

    /// A `5xx` is a diagnostic, so it is written with `access` off, at
    /// `Error`. Every other status writes nothing with `access` off.
    #[test]
    fn a_5xx_is_logged_with_access_off() {
        let trace = decided(&[]);
        let line = logged(false, false, 502, Some(trace));
        assert!(line.contains("\"status\":502"), "the 5xx was lost: {line}");
        assert!(
            line.contains(&format!("\"request_id\":\"{}\"", trace.trace_id_hex())),
            "the 5xx names no request: {line}"
        );
        assert!(
            line.to_lowercase().contains("\"level\":\"error\""),
            "a 5xx is `Error`: {line}"
        );
        for status in [200, 304, 404, 499, 501] {
            assert_eq!(
                logged(false, false, status, Some(trace)),
                "",
                "`access` is off and a {status} was logged"
            );
        }
    }

    /// `rule:http-server/the-access-log-is-a-mode-default`: a `501` is an
    /// access line at `Info`, because the door answers it to a verb the client
    /// made up. A client cannot force an error line per request.
    #[test]
    fn a_501_is_an_access_line_at_info() {
        let line = logged(true, false, 501, Some(decided(&[])));
        assert!(line.contains("\"status\":501"), "the 501 was lost: {line}");
        assert!(
            line.to_lowercase().contains("\"level\":\"info\""),
            "a 501 is `Info`: {line}"
        );
    }
}
