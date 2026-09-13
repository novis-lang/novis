//! [ADR 0180 § 15](/docs/decisions/0180.md)'s `http` trace event: the facts one
//! outbound call contributes to a trace, and why a header value is not one of
//! them.
//!
//! § 15 fixes the field set — the method, the scheme, host and port, the path
//! without its query, the status, the attempt and hop counts, the address
//! connected to, and the time spent resolving, connecting, in the handshake, to
//! the first byte and in total — and then says *never a query string, a header
//! value or a body*. `crates/nvs-db/src/span.rs` is the precedent for a kind
//! fixing its fields at all, and this module takes the second half of its shape
//! too: [`HttpSpan`] **is never handed the [`super::transport::Call`]**, so the
//! headers, the body and the URL a call was built from are not in scope for a
//! later field, a later rendering or a later transport to leak. What it is
//! handed instead is the request-target, and [`HttpSpan::at`] keeps only what
//! precedes the `?` — so the query is dropped by the type that renders rather
//! than by every caller remembering to drop it.
//!
//! The rule it serves is `rule:observability/trace-events-carry-a-kind`'s,
//! which states the same exclusion for a `query`'s bound parameters in the same
//! words, because a trace is a `secret` sink.
//!
//! # What it costs, and when it is paid
//!
//! A handful of `Instant` reads and one `String` per call, held for as long as
//! the call — O(in-flight calls), never O(calls made). Against a network round
//! trip that is not measurable, which is why the span is built unconditionally
//! and only its *rendering* waits on `DebugFlags::TRACE`, exactly as
//! `nvs_db::QuerySpan` is built unconditionally against a statement's round
//! trip.
//!
//! **One event per call, whatever its attempt count**, which is the rule's own
//! sentence: the counters accumulate across every attempt and every hop, and
//! the timings are the answering attempt's, so a call that reached its answer
//! on the third try reports `attempts=3` beside the times the third one took.
//! `Core\Http\Client`'s member is the single filer, and a call a test's answer
//! table served never opens one at all — nothing crossed a network, so there is
//! no time to report and no address to name.

use std::fmt;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

/// One outbound call's `http` trace event, from the moment the member was
/// entered to the moment it had a reply's head.
///
/// **`Debug` is derived on purpose**, for `nvs_db::QuerySpan`'s reason: the
/// claim this type exists to keep is that no query string, header value or body
/// octet appears *anywhere* in it, and
/// [`an_http_span_carries_no_query_string_header_value_or_body`] asserts that
/// over the renderings rather than over the fields — so a field added later
/// joins the `Debug` output without anyone having to remember it, and the test
/// covers it on the commit that adds it.
#[derive(Clone, Debug)]
pub(crate) struct HttpSpan {
    /// The method, upper-cased, as the row's own name has it.
    method: String,
    /// Whether the call ended up on `https`, which is the scheme the event
    /// names. A redirect hop may change it, so this is the answering hop's.
    tls: bool,
    /// The bare host, with no port and never any userinfo — `Parts::host`,
    /// which is what a certificate was checked against.
    host: String,
    /// Where the octets went, defaulted by scheme where the URL wrote no port.
    port: u16,
    /// The request-target up to its `?`, and never past it — see the module
    /// docs for why the cut is here and not at the caller.
    path: String,
    /// The reply's status, once there is one. `None` for a call that never got
    /// an answer, which files no event at all.
    status: Option<i64>,
    /// Attempts made in total, counting the first and counting every hop's —
    /// the rule's *one event carrying its attempt count*.
    attempts: u32,
    /// Redirect hops taken, which is zero for every call that did not ask to
    /// follow any.
    hops: u32,
    /// The address the answering attempt's connection actually goes to, which
    /// is not always the approved set's first.
    address: Option<SocketAddr>,
    /// What resolving this call's host cost, measured where the resolution
    /// happens and handed in — see [`HttpSpan::opened`].
    resolve: Duration,
    /// What the answering attempt spent opening its socket, and zero where this
    /// core was already holding one.
    connect: Duration,
    /// What the answering attempt spent in the handshake: zero on a plain
    /// `http` call and zero on a pooled connection, whose session was
    /// negotiated for an earlier call.
    handshake: Duration,
    /// When the member was entered. Every duration below is measured from here.
    opened: Instant,
    /// From [`HttpSpan::opened`] to the first octet of the answering attempt's
    /// reply — curl's `time_starttransfer`, so a retried call reports the wait
    /// the caller actually saw and not the last socket's alone.
    first_byte: Option<Duration>,
    /// Frozen by [`HttpSpan::finished`]; `None` while the call is still
    /// running, which is what makes [`HttpSpan::duration`] read the clock.
    elapsed: Option<Duration>,
}

impl HttpSpan {
    /// Opens a span for a call that is going out now.
    ///
    /// **The resolve time is a parameter and not something measured here**,
    /// because the resolution is not in this crate's transport at all: the
    /// launderer resolved and pinned the host before the call was framed
    /// (`rule:http-server/allow-url-pins-the-address`), and a call handed an
    /// already-pinned `Core\Http\Target` resolved nothing of its own and passes
    /// a zero. Attributing an earlier call's lookup to this one would be the
    /// copy that disagrees.
    ///
    /// **The `Call` is deliberately not an argument.** The headers and the body
    /// are on it, and the whole of § 15's "never a header value or a body" is
    /// that they are not passed here — see the module docs.
    pub(crate) fn opened(method: &str, resolve: Duration) -> HttpSpan {
        HttpSpan {
            method: method.to_owned(),
            tls: false,
            host: String::new(),
            port: 0,
            path: String::new(),
            status: None,
            attempts: 0,
            hops: 0,
            address: None,
            resolve,
            connect: Duration::ZERO,
            handshake: Duration::ZERO,
            opened: Instant::now(),
            first_byte: None,
            elapsed: None,
        }
    }

    /// Names the origin this attempt is against, and the target it asks for.
    ///
    /// Written on every attempt and every hop rather than once, so the event
    /// describes where the call *ended up*: a redirect chain's event names the
    /// origin that answered it, which is the one a reader of the trace is
    /// asking about.
    ///
    /// `target` arrives as the request line writes it, query and all, and what
    /// is kept is the part before the first `?`. That cut is the module docs'
    /// second half, and it is here so that no caller can hand a query string in
    /// by forgetting to.
    pub(crate) fn at(&mut self, tls: bool, host: &str, port: u16, target: &str) {
        self.tls = tls;
        self.host = host.to_owned();
        self.port = port;
        let path = target.split('?').next().unwrap_or("/");
        self.path = path.to_owned();
    }

    /// Counts one attempt, which is the first one too, and drops whatever the
    /// previous attempt measured to its first octet: the event reports the wait
    /// the answering attempt cost, and a failed one's first byte is not that.
    pub(crate) fn attempted(&mut self) {
        self.attempts += 1;
        self.first_byte = None;
    }

    /// Counts one redirect hop taken.
    pub(crate) fn hopped(&mut self) {
        self.hops += 1;
    }

    /// Files a connection this core was already holding: the address it goes
    /// to, and no time spent on either opening it or negotiating its session.
    ///
    /// The two zeroes are the honest answer rather than a missing measurement —
    /// this call spent nothing on either, which is what pooling is for
    /// (`rule:http-server/an-outbound-connection-is-pooled-per-core-and-stays-pinned`).
    ///
    /// `None` is the same kind of honesty about the address: under
    /// `[http.client.proxy] resolve = "proxy"` nothing in this process ever
    /// learned one, so the event names the host it asked for and no address at
    /// all rather than the proxy's, which is not where the request went.
    pub(crate) fn drawn(&mut self, address: Option<SocketAddr>) {
        self.address = address;
        self.connect = Duration::ZERO;
        self.handshake = Duration::ZERO;
    }

    /// Files a socket this attempt opened, and what the walk across the
    /// approved set cost to get it. `None` where there was no approved set to
    /// walk, for the reason [`HttpSpan::drawn`] gives.
    pub(crate) fn connected(&mut self, address: Option<SocketAddr>, took: Duration) {
        self.address = address;
        self.connect = took;
        self.handshake = Duration::ZERO;
    }

    /// Files what the handshake on that socket cost.
    pub(crate) fn handshook(&mut self, took: Duration) {
        self.handshake = took;
    }

    /// Marks the first octet of a reply arriving, if this is the first one this
    /// attempt saw.
    ///
    /// Reset per attempt by [`HttpSpan::attempted`], so a retried call reports
    /// the answering attempt's first byte rather than a failed one's.
    pub(crate) fn first_octet(&mut self) {
        if self.first_byte.is_none() {
            self.first_byte = Some(self.opened.elapsed());
        }
    }

    /// Files the status the reply's head carried.
    pub(crate) fn answered(&mut self, status: i64) {
        self.status = Some(status);
    }

    /// Ends the span: freezes the total.
    ///
    /// Idempotent in the direction that matters — a second ending leaves the
    /// first total in place, for `nvs_db::QuerySpan::finished`'s reason.
    pub(crate) fn finished(&mut self) {
        if self.elapsed.is_none() {
            self.elapsed = Some(self.opened.elapsed());
        }
    }

    /// How long the call took, or how long it has been running.
    pub(crate) fn duration(&self) -> Duration {
        self.elapsed.unwrap_or_else(|| self.opened.elapsed())
    }
}

impl fmt::Display for HttpSpan {
    /// The trace line, in § 15's own order: what was asked, of whom, what came
    /// back, how many tries it took and where the time went.
    ///
    /// Every field is written even where it is zero, because a fixed field set
    /// a reader can parse positionally is the point of fixing it — the one
    /// exception is the address, which is absent only for an event no filer
    /// files.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let scheme = if self.tls { "https" } else { "http" };
        write!(
            f,
            "http method={} scheme={scheme} host={} port={} path={}",
            self.method, self.host, self.port, self.path
        )?;
        if let Some(status) = self.status {
            write!(f, " status={status}")?;
        }
        write!(f, " attempts={} hops={}", self.attempts, self.hops)?;
        if let Some(address) = self.address {
            write!(f, " address={address}")?;
        }
        write!(
            f,
            " resolve={:?} connect={:?} tls={:?}",
            self.resolve, self.connect, self.handshake
        )?;
        if let Some(first) = self.first_byte {
            write!(f, " first_byte={first:?}")?;
        }
        write!(f, " took={:?}", self.duration())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::HttpSpan;

    /// § 15's exclusion, over the two renderings a consumer can reach: neither
    /// the trace line nor the `Debug` one holds the query string, and neither
    /// has anywhere a header value or a body octet could have been put, since
    /// nothing hands one in.
    #[test]
    fn an_http_span_carries_no_query_string_header_value_or_body() {
        let mut span = HttpSpan::opened("GET", Duration::from_millis(3));
        span.at(
            true,
            "api.example.com",
            443,
            "/v1/things?token=sekrit&page=2",
        );
        span.attempted();
        span.answered(200);
        span.finished();

        for rendered in [span.to_string(), format!("{span:?}")] {
            assert!(!rendered.contains("sekrit"), "{rendered}");
            assert!(!rendered.contains("token"), "{rendered}");
            assert!(!rendered.contains('?'), "{rendered}");
        }
        assert!(span.to_string().contains("path=/v1/things"));
    }

    /// The counters accumulate across attempts while the timings do not: a call
    /// that answered on its third try is one event saying so, which is the
    /// rule's *one event carrying its attempt count*.
    #[test]
    fn a_retried_span_counts_every_attempt_and_times_the_answering_one() {
        let mut span = HttpSpan::opened("POST", Duration::ZERO);
        span.at(false, "example.test", 80, "/send");
        for _ in 0..3 {
            span.attempted();
        }
        span.answered(200);
        span.finished();

        let line = span.to_string();
        assert!(line.contains("attempts=3"), "{line}");
        assert!(line.contains("hops=0"), "{line}");
        assert!(
            line.contains("scheme=http host=example.test port=80"),
            "{line}"
        );
    }

    /// The total freezes when the call ends, not when the span is read, and a
    /// second ending does not move it.
    #[test]
    fn a_finished_span_keeps_the_total_its_first_ending_measured() {
        let mut span = HttpSpan::opened("GET", Duration::ZERO);
        span.finished();
        let ended = span.duration();
        span.finished();
        assert_eq!(span.duration(), ended);
    }
}
