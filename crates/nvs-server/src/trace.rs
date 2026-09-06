//! `rule:observability/a-trace-id-exists-for-every-request`'s trace
//! identity, taken at the door: the `traceparent` the request arrived with,
//! read once, before any application code runs.
//!
//! # Why the door, and why it is one call
//!
//! § 2 gives every request a trace id whatever the sampling decision, and makes
//! that id Novis's *only* request identifier — `Core\Server::traceId()` reads
//! it, every `[log]` record carries it, and there is deliberately no second one
//! and no inbound `X-Request-ID` (ADR 0097 § 9). An id that a subsystem drew
//! for itself on first ask would be a second place for two readers to disagree
//! about which trace a request is in, so [`nvs_runtime::Ctx::new`] draws a root
//! eagerly and the only question left is whether *this* request continues
//! somebody else's trace. That question is a header's, and a header is the
//! door's to read: [`take`] answers it beside [`crate::route::take`] and
//! [`crate::mount::carry`], and the answer rides on the carrier
//! ([`nvs_runtime::Inbound::set_trace_context`]) exactly as the peer and the
//! match do.
//!
//! **Nothing here decides whether the trace is exported.** § 2's sampling is
//! head-based at the root and `[trace] sample` is unbuilt, so what this module
//! produces for a request that arrived without a usable header is always an
//! unsampled root — and an inbound *sampled* trace is always continued, which
//! is the half of § 2 that is fully landed. Which spans a sampled trace then
//! produces is § 1's exporter and not this crate's.
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
//! the 26 bytes the carrier holds for the answer. Nothing outlives the request.

use nvs_runtime::{Inbound, TraceContext};

/// The field name § 2 names, lower case because `hyper` normalises a
/// `HeaderName` on the way in and the carrier keeps what it was given.
const TRACEPARENT: &str = "traceparent";

/// Decides `rule:observability/a-trace-id-exists-for-every-request`'s trace for this request, and records it on the
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
pub fn take(inbound: &mut Inbound) {
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
        TraceContext::continuing(carried)
    };
    inbound.set_trace_context(decided);
}

#[cfg(test)]
mod tests {
    use super::take;
    use nvs_render::Level;
    use nvs_runtime::{Ctx, Inbound, OutputSink, TraceContext, floor};

    /// A carrier as the door builds one: the request line, then one field line
    /// per entry in arrival order.
    fn arrived(headers: &[(&str, &str)]) -> Inbound {
        let mut inbound = Inbound::new("GET", "/orders/17", "");
        for (name, value) in headers {
            inbound.push_header(name, value.as_bytes());
        }
        inbound
    }

    /// The trace `take` decided, which is never absent — the module doc owns
    /// why something is always written.
    fn decided(headers: &[(&str, &str)]) -> TraceContext {
        let mut inbound = arrived(headers);
        take(&mut inbound);
        inbound
            .trace_context()
            .expect("the door wrote no trace for a request it walked")
    }

    /// One valid inbound header, and the trace it names.
    const INBOUND: &str = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";

    /// `rule:observability/a-trace-id-exists-for-every-request`: an inbound `traceparent` is continued — trace id, parent
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
            continued.traceparent(),
            INBOUND,
            "the arrived header was not adopted whole"
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
            // No header: a root, which nothing samples until `[trace] sample`
            // lands.
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
        take(&mut inbound);
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

    /// `rule:observability/a-trace-id-exists-for-every-request` and `rule:observability/metrics-and-trace-blocks-are-system`, across the seam neither crate owns alone: the trace
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
}
