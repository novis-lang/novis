//! The distributed-trace identity of one request —
//! `rule:observability/a-trace-id-exists-for-every-request`'s trace id, the span an outbound
//! call names as its parent, and the sampled flag, read from and rendered as a W3C `traceparent`.
//!
//! **This is not `rule:testing/debug-probes`'s
//! trace, which [`Ctx::trace`](crate::Ctx::trace) holds**, and the two share nothing but the word.
//! That one records an event per compiled call site and is a debugging surface; this one is three
//! identifiers a whole request carries. `rule:observability/a-trace-id-exists-for-every-request` opens by separating them, because conflating
//! them would produce a distributed trace with one span per function call, which no backend can
//! store and no human can read.
//!
//! **An id exists for every request, whatever the sampling decision** (§ 2). Sampling governs
//! whether a trace is *exported*, never whether an id is generated, and that is what lets this be
//! Novis's only request identifier: `Core\Server::traceId()` reads this same id, every `[log]`
//! record and every error rendering carries it, and it is emitted on the response so a proxy can log
//! it with one `log_format` line. There is deliberately no second identifier and no inbound
//! `X-Request-ID` (`rule:http-server/the-trace-id-is-the-request-identifier`).
//! So the id is generated where a request's state lives — in [`Ctx::new`](crate::Ctx::new), eagerly
//! — rather than by whichever subsystem asks for it first, which is what would let two of them
//! disagree.
//!
//! **The span id is this request's own root span, and never the caller's.** A `traceparent` names
//! the span the *sender* was in, so a server that adopted that id as its own would put two spans
//! with one id in a trace — which is why the arrived id is kept apart, as
//! [`parent_span_id`](TraceContext::parent_span_id), and the id this context renders onward is
//! drawn here. `nvs_server::trace::spans` builds the root span that id belongs to
//! (`rule:observability/four-kinds-become-a-span`), so an outbound call's header names a span that
//! exists.
//!
//! **A root's flag is the head draw, and a continued trace's is the header's**
//! (`rule:observability/sampling-is-head-based`). A request that *starts* a trace is recorded with
//! probability `[trace] sample`, drawn once in [`TraceContext::rooted`]; a request that continued
//! somebody else's carries the flag that arrived, whatever the local fraction, because the root
//! already decided and a partially recorded distributed trace is worse than none. That is also why
//! an inbound sampled trace is propagated onward unchanged.
//!
//! **What is not here yet.** Nothing pushes a derived span to a collector
//! (`rule:observability/the-exporters-are-crates`).
//!
//! **What it spends:** 34 bytes per request, and at most 32 bytes drawn from the thread's CSPRNG
//! while the context is built — the two ids, and the head draw a written `[trace] sample` adds to a
//! request that roots a trace. O(in-flight requests), never O(requests served).

use rand::{Rng, RngExt};

/// One request's place in a distributed trace.
///
/// Built by [`TraceContext::continuing`] for a request a door read a header for — which roots a new
/// trace through [`TraceContext::rooted`] when there was nothing usable to continue — and by
/// [`TraceContext::started`] for the eager root every context begins with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraceContext {
    /// The trace this request belongs to. Never all-zero, which the W3C format reserves as "no
    /// trace".
    trace_id: [u8; 16],
    /// This request's own root span, which is what an outbound call names as its parent and what
    /// `nvs_server::trace::spans` gives the root span it derives. Always drawn here, continued
    /// trace or not — the module doc owns why the arrived id is not reused for it.
    span_id: [u8; 8],
    /// The span the caller was in, for a request that arrived with a header this process could
    /// read, and `None` for a root. It is the parent edge of the root span and nothing else reads
    /// it.
    parent_span_id: Option<[u8; 8]>,
    /// Whether this trace is being recorded. Adopted from an inbound header, and otherwise the one
    /// head draw against `[trace] sample` that [`TraceContext::rooted`] makes.
    sampled: bool,
}

/// The one `traceparent` version this parses and the only one it emits.
const VERSION: &str = "00";

/// The flags bit W3C gives to "sampled". No other bit is defined, and an unknown one is ignored
/// rather than refused, exactly as an unparseable header is.
const SAMPLED: u8 = 0x01;

impl TraceContext {
    /// A new trace, rooted at this request and recorded by nothing.
    ///
    /// The eager root [`Ctx::new`](crate::Ctx::new) draws for every context, before any door has
    /// read a header: a request a door decides for gets [`Self::continuing`]'s answer written over
    /// this one, and a context no door serves keeps it. Head sampling is the door's question and is
    /// asked where a header is (`nvs_server::trace::take`), so a context that was never asked
    /// records nothing.
    #[must_use]
    pub fn started() -> Self {
        Self::rooted(0.0)
    }

    /// A new trace, rooted at this request and recorded with probability `sample`.
    ///
    /// `sample` is `rule:observability/sampling-is-head-based`'s fraction, and this is where it is
    /// spent because this is the constructor for a trace that *starts*. The draw is made once and
    /// every span under the root is exported or not by it — which is what head-based means, and why
    /// there is no second decision anywhere below.
    #[must_use]
    pub fn rooted(sample: f64) -> Self {
        let mut trace_id = [0_u8; 16];
        rand::rng().fill_bytes(&mut trace_id);
        // A CSPRNG draws all-zero with probability 2^-128, and the format reserves that value for
        // "absent". Forcing one bit is cheaper than a redraw loop and is the same distribution
        // everywhere it matters.
        trace_id[0] |= 1;
        Self {
            trace_id,
            span_id: draw_span_id(),
            parent_span_id: None,
            sampled: draws(sample),
        }
    }

    /// The context an inbound request produces: `inbound`'s trace continued when it is a
    /// `traceparent` this understands, and a new trace otherwise.
    ///
    /// **A malformed header starts a new trace rather than throwing** (§ 2). It arrived from
    /// outside, it is `tainted`, and refusing a request over a bad tracing header would turn an
    /// observability feature into an availability one. That is why this takes an `Option` and
    /// returns a `TraceContext` rather than a `Result`: there is no failure to report.
    ///
    /// **`sample` reaches only the trace this starts.** A header this understands brings its own
    /// sampled flag and that flag stands whatever the local fraction is
    /// (`rule:observability/sampling-is-head-based`); the fraction is drawn against for the request
    /// that arrived with no header and for the one whose header could not be read, because both of
    /// those *are* new traces. Carrying the rate down here rather than deciding above is what makes
    /// those two cases impossible to tell apart by accident — the malformed one is a new trace by
    /// § 2, so it is a new trace in the head draw too — and the draw is made only on the path that
    /// roots one.
    #[must_use]
    pub fn continuing(inbound: Option<&str>, sample: f64) -> Self {
        inbound
            .and_then(Self::parse)
            .unwrap_or_else(|| Self::rooted(sample))
    }

    /// The trace this request belongs to.
    #[must_use]
    pub fn trace_id(&self) -> [u8; 16] {
        self.trace_id
    }

    /// The span an outbound call names as its parent — see the field's own docs.
    #[must_use]
    pub fn span_id(&self) -> [u8; 8] {
        self.span_id
    }

    /// The caller's span, for a request that continued somebody else's trace — the parent edge of
    /// the root span, and `None` for a request that started one.
    #[must_use]
    pub fn parent_span_id(&self) -> Option<[u8; 8]> {
        self.parent_span_id
    }

    /// Whether this trace is being recorded.
    #[must_use]
    pub fn sampled(&self) -> bool {
        self.sampled
    }

    /// The trace id in the lower-case hex a `traceparent` writes it in.
    ///
    /// Beside [`Self::trace_id`] rather than left to each caller, because § 2
    /// makes this id Novis's *only* request identifier and a second rendering
    /// of it is how a `[log]` record's `request_id` and the header a proxy
    /// logged come to be two spellings of one number. Thirty-two characters,
    /// always: [`Self::traceparent`] pads through the same helper.
    #[must_use]
    pub fn trace_id_hex(&self) -> String {
        let mut out = String::with_capacity(32);
        push_hex(&mut out, &self.trace_id);
        out
    }

    /// The span id on the same terms, sixteen characters wide.
    #[must_use]
    pub fn span_id_hex(&self) -> String {
        let mut out = String::with_capacity(16);
        push_hex(&mut out, &self.span_id);
        out
    }

    /// This context as the header value an outbound call carries — `00-<trace>-<span>-<flags>`.
    #[must_use]
    pub fn traceparent(&self) -> String {
        let mut out = String::with_capacity(55);
        out.push_str(VERSION);
        out.push('-');
        push_hex(&mut out, &self.trace_id);
        out.push('-');
        push_hex(&mut out, &self.span_id);
        out.push_str(if self.sampled { "-01" } else { "-00" });
        out
    }

    /// `header` as a context, or `None` for anything this does not understand.
    ///
    /// Strict on purpose, and about the smallest rule that is still right: version `00` exactly,
    /// four fields, lower-case hex of the stated widths, and neither id all-zero. A later version is
    /// not guessed at — W3C allows a lenient reading of one, but a header naming a format this
    /// process has never seen is better treated as the new trace [`continuing`](Self::continuing)
    /// gives it than as a half-understood continuation of someone else's.
    fn parse(header: &str) -> Option<Self> {
        let mut fields = header.trim().split('-');
        let (version, trace, span, flags) = (
            fields.next()?,
            fields.next()?,
            fields.next()?,
            fields.next()?,
        );
        if fields.next().is_some() || version != VERSION {
            return None;
        }
        let trace_id: [u8; 16] = from_hex(trace)?;
        let span_id: [u8; 8] = from_hex(span)?;
        if trace_id == [0; 16] || span_id == [0; 8] {
            return None;
        }
        let flags: [u8; 1] = from_hex(flags)?;
        Some(Self {
            trace_id,
            // The arrived id is the caller's span and becomes this request's parent edge; the id
            // this request *is* gets drawn, which the module doc argues.
            span_id: draw_span_id(),
            parent_span_id: Some(span_id),
            sampled: flags[0] & SAMPLED != 0,
        })
    }
}

/// A span id, drawn from the thread's CSPRNG with [`TraceContext::started`]'s one bit forced so the
/// all-zero value the format reserves for "absent" is never the answer.
fn draw_span_id() -> [u8; 8] {
    let mut span_id = [0_u8; 8];
    rand::rng().fill_bytes(&mut span_id);
    span_id[0] |= 1;
    span_id
}

/// Head sampling's one draw: `true` with probability `sample`.
///
/// `[trace] sample` is refused where it is written (`nvs_config::export`), so what arrives here is
/// finite and inside `0.0..=1.0`. A value that somehow is not records nothing rather than
/// everything: every ordering against a `NaN` is false, and of the two readings of a number that
/// names no fraction, the one that exports less is the one to fail into.
///
/// `0.0` is answered without asking the generator, because it is the shipped default and so the
/// rate almost every request is drawn against — and a uniform `f64` over `0.0..1.0` is never below
/// it anyway.
fn draws(sample: f64) -> bool {
    sample > 0.0 && rand::rng().random::<f64>() < sample
}

/// `bytes` as lower-case hex, appended.
fn push_hex(out: &mut String, bytes: &[u8]) {
    for byte in bytes {
        out.push(char::from_digit(u32::from(byte >> 4), 16).unwrap_or('0'));
        out.push(char::from_digit(u32::from(byte & 0xf), 16).unwrap_or('0'));
    }
}

/// `text` as exactly `N` bytes of lower-case hex, or `None`.
fn from_hex<const N: usize>(text: &str) -> Option<[u8; N]> {
    let text = text.as_bytes();
    if text.len() != N * 2 || text.iter().any(|byte| byte.is_ascii_uppercase()) {
        return None;
    }
    let mut out = [0_u8; N];
    for (byte, pair) in out.iter_mut().zip(text.chunks_exact(2)) {
        let high = char::from(pair[0]).to_digit(16)?;
        let low = char::from(pair[1]).to_digit(16)?;
        *byte = u8::try_from(high * 16 + low).ok()?;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::TraceContext;

    /// `rule:observability/a-trace-id-exists-for-every-request`: an id exists for every request whatever the sampling decision, so a root that
    /// nothing will export still renders a header another service can continue.
    #[test]
    fn a_root_trace_has_an_id_and_renders_a_header_that_parses_back() {
        let root = TraceContext::started();
        assert_ne!(root.trace_id(), [0; 16]);
        assert_ne!(root.span_id(), [0; 8]);
        assert_eq!(root.parent_span_id(), None, "a root descends from nothing");
        assert!(!root.sampled(), "`started` is a root nothing drew for");

        let rendered = root.traceparent();
        assert_eq!(rendered.len(), 55, "{rendered}");
        let downstream = TraceContext::continuing(Some(&rendered), 0.0);
        assert_eq!(
            (downstream.trace_id(), downstream.parent_span_id()),
            (root.trace_id(), Some(root.span_id())),
            "a rendering this process cannot read back is not a header anyone else can"
        );
        assert_ne!(
            downstream.span_id(),
            root.span_id(),
            "the next hop called itself the span it was answering"
        );
    }

    /// Two requests are two traces: the id is drawn, not derived from anything shared.
    #[test]
    fn two_started_traces_do_not_share_an_id() {
        assert_ne!(TraceContext::started(), TraceContext::started());
    }

    /// § 2: an inbound header's trace id and parent span id are adopted, and its sampled flag is
    /// honoured — an inbound sampled trace is always continued.
    ///
    /// The arrived span id is adopted **as the parent** and not as this request's own, which is
    /// the one way the three fields differ in where they end up; the module doc owns why. So the
    /// header this request renders onward keeps the trace and the flag and carries a span id of
    /// its own.
    #[test]
    fn an_inbound_header_is_continued_as_this_requests_parent() {
        let inbound = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
        let continued = TraceContext::continuing(Some(inbound), 0.0);
        assert!(continued.sampled());
        assert_eq!(continued.trace_id_hex(), "4bf92f3577b34da6a3ce929d0e0e4736");
        assert_eq!(
            continued.parent_span_id(),
            Some([0x00, 0xf0, 0x67, 0xaa, 0x0b, 0xa9, 0x02, 0xb7]),
            "the caller's span is not this request's parent"
        );
        assert_ne!(continued.span_id_hex(), "00f067aa0ba902b7");
        assert_eq!(
            continued.traceparent(),
            format!(
                "00-4bf92f3577b34da6a3ce929d0e0e4736-{}-01",
                continued.span_id_hex()
            )
        );

        let unsampled = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-00";
        assert!(!TraceContext::continuing(Some(unsampled), 0.0).sampled());
    }

    /// § 2: a malformed header starts a new trace rather than throwing. Asserted over every way one
    /// can be wrong at once, by *counting* — a rule that accepted one of these would still look
    /// right on the line above.
    #[test]
    fn every_unusable_header_starts_a_new_trace() {
        let good = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
        let unusable = [
            None,
            Some(""),
            Some("00"),
            // A field short, a field long, and a field too many.
            Some("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7"),
            Some("00-4bf92f3577b34da6a3ce929d0e0e473-00f067aa0ba902b7-01"),
            Some("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b77-01"),
            Some("00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01-x"),
            // Not this version, not hex, not lower case.
            Some("01-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"),
            Some("00-4bf92f3577b34da6a3ce929d0e0e473g-00f067aa0ba902b7-01"),
            Some("00-4BF92F3577B34DA6A3CE929D0E0E4736-00f067aa0ba902b7-01"),
            // The two ids the format reserves for "absent".
            Some("00-00000000000000000000000000000000-00f067aa0ba902b7-01"),
            Some("00-4bf92f3577b34da6a3ce929d0e0e4736-0000000000000000-01"),
        ];
        // The trace id is what "adopted" means here: a header that was read at all puts this
        // request in the sender's trace, whatever span id the continuation then draws for itself.
        let joined = TraceContext::continuing(Some(good), 0.0).trace_id();
        let adopted = unusable
            .iter()
            .filter(|header| TraceContext::continuing(**header, 0.0).trace_id() == joined)
            .count();
        assert_eq!(adopted, 0, "one of {unusable:?} was adopted");
    }

    /// `rule:observability/sampling-is-head-based`: the fraction is drawn against once, by the
    /// request that roots the trace, and a request that continued one is recorded by the flag that
    /// arrived rather than by the local rate.
    ///
    /// The two ends are asserted because they are the values an operator writes when they mean off
    /// and everything, and they are the only two a single draw can assert exactly; the fraction
    /// between them is arithmetic this has no way to observe in one call.
    #[test]
    fn a_root_draws_against_the_rate_and_a_continued_trace_keeps_the_arrived_flag() {
        assert!(
            TraceContext::rooted(1.0).sampled(),
            "`1.0` recorded nothing"
        );
        assert!(
            !TraceContext::rooted(0.0).sampled(),
            "`0.0` recorded a trace"
        );

        let sampled = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
        let unsampled = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-00";
        assert!(
            !TraceContext::continuing(Some(unsampled), 1.0).sampled(),
            "a continued trace was re-decided by the local rate"
        );
        assert!(
            TraceContext::continuing(Some(sampled), 0.0).sampled(),
            "an inbound sampled trace was dropped by a rate that is not its to answer"
        );

        // A header nothing can read is a new trace in § 2 and so a new trace in the draw too —
        // the case a rate carried by the caller rather than by this type would be the one to miss.
        assert!(
            TraceContext::continuing(Some("00-not-a-trace-01"), 1.0).sampled(),
            "an unreadable header started a trace the head rate never saw"
        );
    }
}
