//! `rule:observability/an-exporter-brings-no-second-scheduler-and-no-second-client`'s
//! encoder: every serving core's registry added up, written in the Prometheus
//! text exposition format.
//!
//! # Why the format is written here
//!
//! That rule's predicate turned on the two candidate crates and found neither
//! takeable, so the wire format is ours against its specification
//! (`docs/decisions/0186.md` § *Investigation* is the reading). What is left
//! over once a transport and a registry are refused is an encoder of a few
//! hundred bytes per series, which is this module: it reads
//! [`crate::metrics::Registry::series`] in order and formats it, exactly as
//! `rule:observability/the-exporters-are-crates` says an exporter does, and
//! nothing above it changes because it exists.
//!
//! # The merge is arithmetic
//!
//! `rule:observability/a-registry-is-per-core-and-nothing-reads-it` is why
//! [`scrape`] is handed every core's registry rather than reading one shared
//! store: a counter two cores contend over would be coordination bought for a
//! number that is approximate by definition. So the cores are added up **here**,
//! at the one moment anybody reads them — counters and gauges summed, a
//! histogram summed bucket by bucket — which is the merge that rule calls
//! arithmetic and never coordination.
//!
//! Two cores can disagree about what a name is, because a name is fixed to a
//! kind per registry and `Core\Metrics` is what writes the ones this crate did
//! not declare. The first core to carry a name decides it and a later value of
//! another kind is **dropped**, because an exposition format cannot hold a
//! family whose members disagree — the same constraint that makes an unmatched
//! request's `route` label empty rather than absent.
//!
//! # What it spends
//!
//! One `String` per scrape in progress, released when the response is written,
//! and one merged map beside it — both O(cores × series) and neither held past
//! the scrape, which is `rule:programs/memory-priority`'s per-process half of
//! what an exporter costs.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::convert::Infallible;
use std::fmt::Write as _;
use std::io;
use std::ops::ControlFlow;
use std::time::Instant;

use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode, header};
use nvs_config::server::Waits;
use nvs_runtime::host::Woken;

use crate::io::ConnectionIo;
use crate::metrics::{Histogram, Kind, Registry, Series, Value};
use crate::serve::{AcceptBackoff, Answer, Draining, Listening};

/// The content type the text exposition format is served as, version and all.
///
/// The version is part of the media type rather than decoration: a scraper
/// reads it to know which grammar it is parsing, and a body sent as bare
/// `text/plain` is one Prometheus accepts by falling back rather than by being
/// told.
const EXPOSITION: &str = "text/plain; version=0.0.4; charset=utf-8";

/// Answers scrapes on `listener` until the drain begins, one connection at a
/// time.
///
/// **One at a time is the design and not a simplification.** A scrape is a
/// request every fifteen seconds from one or two collectors, and it costs a
/// copy of every core's series; serving two of them concurrently buys nothing
/// and would put a second copy of the whole registry set in flight. The
/// control endpoint serializes for a related reason
/// (`rule:config/one-local-control-socket`), and this loop is the same shape
/// with no thread of its own: it runs as a task on whichever core the boot gave
/// the listener to, and hands that core back at every park.
///
/// **Nothing here counts a request.** The series a scrape reports are the
/// application's, and a `[metrics]` endpoint that appeared in
/// `nvs_requests_total` would be measuring its own collector — so this loop
/// reaches [`crate::metrics::every_core`] and never [`crate::metrics::count_request`].
///
/// # Errors
///
/// The listener's own, on [`crate::serve_on_this_core`]'s terms: an `accept`
/// that fails for anything but descriptor exhaustion ends the loop, and
/// exhaustion is waited out so that a shortage the whole process is in does not
/// leave the exporter dark after it clears.
pub fn serve_scrapes_on_this_core<L>(
    listener: &mut L,
    waits: Waits,
    draining: &Draining,
    mut report: impl FnMut(&str),
    mut keep_serving: impl FnMut() -> ControlFlow<()>,
) -> io::Result<()>
where
    L: Listening + ?Sized,
{
    let mut backoff = AcceptBackoff::default();
    loop {
        if draining.is_draining() {
            break;
        }
        let stream = match listener.arrived_or_woken() {
            // The park ended and named nothing — the drain above is what this
            // goes back round to read.
            Ok(None) => continue,
            Ok(Some((stream, _arrival))) => {
                backoff.accepted();
                stream
            }
            Err(err) => {
                let Some((wait, note)) = backoff.after(&err, Instant::now()) else {
                    return Err(err);
                };
                if let Some(note) = note {
                    report(&note);
                }
                if matches!(nvs_host::sleep(wait), Woken::Cancelled) {
                    break;
                }
                continue;
            }
        };
        // A connection's own failure is the connection's, and there is nobody
        // to report a reset collector to: the socket that would carry the
        // report is the one that failed.
        drop(answer_one(ConnectionIo::new(stream, waits)));
        if keep_serving().is_break() {
            break;
        }
    }
    Ok(())
}

/// Answers one connected collector, and returns when its connection is closed.
///
/// `hyper` frames both halves, which is the argument `crate::control` and
/// `crate::serve` both make: framing is where smuggling lives, and a parser of
/// ours on a socket an operator can reach is a parser of ours to get right.
/// Driven on this core's own stack through [`nvs_host::block_on`], so the park
/// a slow collector causes hands the core back rather than holding it.
///
/// `None` is the cancellation [`nvs_host::block_on`] answers with rather than
/// parking on a wake that is not coming: this loop's task is being torn down,
/// and the connection goes with it. The answer inside it is whatever `hyper`
/// ended the connection on — a collector that went away, or one that sent
/// something that is not HTTP.
fn answer_one(io: ConnectionIo) -> Option<hyper::Result<()>> {
    let service = service_fn(|request: Request<Incoming>| {
        std::future::ready(Ok::<_, Infallible>(answered(request.method())))
    });
    nvs_host::block_on(http1::Builder::new().serve_connection(io, service))
}

/// What one scrape is answered with.
///
/// **Whatever the path.** `[metrics] listen` is the exporter's own address and
/// nothing else is served on it, so matching a path would be one more thing to
/// configure and to get wrong — and answering every one of them is what makes a
/// collector's default `metrics_path` work without being told anything. A
/// method other than `GET` or `HEAD` is a `405` naming the two, because a
/// scrape endpoint that is not read-only is the one thing a reader must be able
/// to rule out.
fn answered(method: &Method) -> Response<Answer> {
    if !matches!(*method, Method::GET | Method::HEAD) {
        return Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .header(header::ALLOW, "GET, HEAD")
            .body(Answer::empty())
            .unwrap_or_else(|_| Response::new(Answer::empty()));
    }
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, EXPOSITION)
        .body(Answer::new(scrape_every_core()))
        .unwrap_or_else(|_| Response::new(Answer::empty()))
}

/// This whole process, scraped.
///
/// [`scrape`] over [`crate::metrics::every_core`], and the one call a listener
/// on `[metrics] listen` makes. It takes no argument because a scrape is about
/// the process rather than about the connection that asked for it: what it
/// answers with is every core that is serving at the moment it is called,
/// including the one it is called on.
///
/// An empty body where no core built a registry, which is what a tree writing
/// `exporter = false` produces and is a valid exposition of nothing.
#[must_use]
pub fn scrape_every_core() -> String {
    scrape(&crate::metrics::every_core())
}

/// Every core's series, merged and written in the text exposition format.
///
/// The answer is the whole body of a scrape: `# TYPE` once per family, then its
/// members, in the order [`Registry::series`] holds them — which is sorted by
/// name and then by label, so one core's scrape of an unchanged process
/// produces identical bytes and every member of a family is adjacent to the
/// header that declared it.
///
/// **No `# HELP` and no timestamps.** Both are optional in the format, a scrape
/// that omits the timestamp is the one Prometheus wants — it stamps the sample
/// with the moment of collection — and help text here would be a second copy of
/// `rule:observability/default-series`'s roster with nothing to keep it in step.
///
/// A series whose family was first seen as another kind is left out
/// (§ *The merge is arithmetic*), and so is nothing else: what the cores hold
/// is what a scrape says.
#[must_use]
pub fn scrape(cores: &[Registry]) -> String {
    let merged = merged(cores);
    let mut out = String::new();
    // The family currently open, and the kind its header declared. Carried
    // rather than re-derived because the header is written once and the members
    // under it are many, and because a member of another kind is only
    // recognisable against the one that opened the family.
    let mut family: Option<(&str, Kind)> = None;
    for (series, value) in &merged {
        let name = series.name.as_str();
        match family {
            Some((open, kind)) if open == name => {
                if value.kind() != kind {
                    continue;
                }
            }
            _ => {
                let kind = value.kind();
                let _ = writeln!(out, "# TYPE {name} {}", kind.name());
                family = Some((name, kind));
            }
        }
        match value {
            Value::Counter(total) => line(&mut out, name, &series.labels, None, &total.to_string()),
            Value::Gauge(value) => line(&mut out, name, &series.labels, None, &number(*value)),
            Value::Histogram(histogram) => buckets(&mut out, series, histogram),
        }
    }
    out
}

/// Every core's series, added up.
///
/// `pub(crate)` because both exporters make the same merge: § *The merge is
/// arithmetic* is a property of the registry and not of the format, so the push
/// in [`crate::otlp`] adds the cores up here rather than beside a second copy of
/// [`add`] that could drift from this one.
pub(crate) fn merged(cores: &[Registry]) -> BTreeMap<Series, Value> {
    let mut merged: BTreeMap<Series, Value> = BTreeMap::new();
    for core in cores {
        for (series, value) in core.series() {
            match merged.get_mut(series) {
                Some(running) => add(running, value),
                None => {
                    merged.insert(series.clone(), value.clone());
                }
            }
        }
    }
    merged
}

/// Adds one core's value into the running total for the same series.
///
/// Saturating rather than wrapping: a counter that wrapped would be read as a
/// process restart by every backend, which is § 7's argument against evicting a
/// series stated over the merge instead of over the registry.
///
/// A pair that does not match is dropped, which is the kind disagreement above
/// and, for a histogram, a name two cores gave different boundaries. Neither can
/// be added without inventing a number.
fn add(running: &mut Value, value: &Value) {
    match (running, value) {
        (Value::Counter(running), Value::Counter(value)) => {
            *running = running.saturating_add(*value);
        }
        (Value::Gauge(running), Value::Gauge(value)) => *running += *value,
        (Value::Histogram(running), Value::Histogram(value)) if running.bounds == value.bounds => {
            for (running, value) in running.counts.iter_mut().zip(&value.counts) {
                *running = running.saturating_add(*value);
            }
            running.sum += value.sum;
            running.count = running.count.saturating_add(value.count);
        }
        _ => {}
    }
}

/// One histogram: its buckets, its sum and its count.
///
/// The buckets go out **cumulative**, which is the format's own shape and not
/// the registry's: [`Histogram::counts`] holds one observation count per
/// boundary because a merge can add those and cannot subtract them back, so the
/// running total is taken here, at the only place that needs it. The last slot
/// is `+Inf` and therefore equals `_count`, which is what the format requires of
/// every histogram.
fn buckets(out: &mut String, series: &Series, histogram: &Histogram) {
    let mut running = 0_u64;
    for (slot, count) in histogram.counts.iter().enumerate() {
        running = running.saturating_add(*count);
        let bound = histogram
            .bounds
            .get(slot)
            .map_or_else(|| "+Inf".to_owned(), |bound| number(*bound));
        line(
            out,
            &format!("{}_bucket", series.name),
            &series.labels,
            Some(("le", &bound)),
            &running.to_string(),
        );
    }
    line(
        out,
        &format!("{}_sum", series.name),
        &series.labels,
        None,
        &number(histogram.sum),
    );
    line(
        out,
        &format!("{}_count", series.name),
        &series.labels,
        None,
        &histogram.count.to_string(),
    );
}

/// One exposition line: the name, the labels it carries, and the value.
///
/// `extra` is the one label the format adds that the registry does not hold — a
/// bucket's `le` — and it is written last, after the sorted ones, which is
/// where the specification's own examples put it.
fn line(
    out: &mut String,
    name: &str,
    labels: &[(String, String)],
    extra: Option<(&str, &str)>,
    value: &str,
) {
    out.push_str(name);
    if !labels.is_empty() || extra.is_some() {
        out.push('{');
        for (at, (label, value)) in labels.iter().enumerate() {
            if at > 0 {
                out.push(',');
            }
            let _ = write!(out, "{label}=\"{}\"", escaped(value));
        }
        if let Some((label, value)) = extra {
            if !labels.is_empty() {
                out.push(',');
            }
            let _ = write!(out, "{label}=\"{}\"", escaped(value));
        }
        out.push('}');
    }
    let _ = writeln!(out, " {value}");
}

/// A label value as the format admits it.
///
/// A backslash, a double quote and a newline are the three characters it
/// escapes and nothing else is touched, so a value that needs none of them is
/// borrowed rather than copied — which is every value this server writes for
/// itself, since `rule:security/metric-label-refuses-tainted` keeps a request's
/// own bytes out of a label in the first place. A `Core\Metrics` caller's label
/// is the case this exists for.
fn escaped(value: &str) -> Cow<'_, str> {
    if value
        .bytes()
        .any(|byte| matches!(byte, b'\\' | b'"' | b'\n'))
    {
        let mut escaped = String::with_capacity(value.len() + 8);
        for character in value.chars() {
            match character {
                '\\' => escaped.push_str("\\\\"),
                '"' => escaped.push_str("\\\""),
                '\n' => escaped.push_str("\\n"),
                character => escaped.push(character),
            }
        }
        Cow::Owned(escaped)
    } else {
        Cow::Borrowed(value)
    }
}

/// A float as the format spells it.
///
/// Rust's own rendering is the format's for every finite value; the three that
/// are not finite have spellings of their own — `NaN`, `+Inf` and `-Inf` — and a
/// scrape writing Rust's `inf` for one of them is a body Prometheus rejects
/// whole, not a sample it skips.
fn number(value: f64) -> String {
    if value.is_nan() {
        "NaN".to_owned()
    } else if value == f64::INFINITY {
        "+Inf".to_owned()
    } else if value == f64::NEG_INFINITY {
        "-Inf".to_owned()
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Barrier};
    use std::thread;
    use std::time::Duration;

    use nvs_config::tree::Metrics;
    use nvs_config::{Config, Setting};

    use crate::metrics::Registry;

    /// `rule:observability/a-registry-is-per-core-and-nothing-reads-it`'s merge
    /// and the format it is written in, as one answer: two cores that each
    /// served the same route add up to one family, and a core that saw
    /// something the other did not still appears.
    ///
    /// **The histogram is where a merge goes wrong quietly.** Its buckets are
    /// per boundary in the registry and cumulative on the wire, and its `+Inf`
    /// slot must equal `_count` — a merge that added the wrong halves, or a
    /// writer that forgot to accumulate, produces a body every backend accepts
    /// and reads as a different distribution. So both are asserted by value.
    #[test]
    fn a_scrape_merges_every_cores_series_into_the_text_exposition_format() {
        let mut one = Registry::new(64);
        one.request("GET", 200, Some("products.show"), Duration::from_millis(30));
        one.request("GET", 200, Some("products.show"), Duration::from_millis(30));
        one.gauge("nvs_tasks_in_flight", 3.0, &[])
            .expect("a declared gauge");
        one.increment("app_orders_total", 1, &[("shop", "a \"quoted\" name")])
            .expect("a name nothing had used");

        let mut two = Registry::new(64);
        two.request("GET", 200, Some("products.show"), Duration::from_millis(30));
        two.request("POST", 500, None, Duration::from_millis(2));
        two.gauge("nvs_tasks_in_flight", 4.0, &[])
            .expect("a declared gauge");

        let scraped = super::scrape(&[one, two]);
        let lines: Vec<&str> = scraped.lines().collect();

        // One header per family, whichever core carried the members.
        assert_eq!(
            lines
                .iter()
                .filter(|line| line.starts_with("# TYPE nvs_requests_total "))
                .count(),
            1
        );
        assert!(lines.contains(&"# TYPE nvs_requests_total counter"));
        assert!(lines.contains(&"# TYPE nvs_request_duration_seconds histogram"));
        assert!(lines.contains(&"# TYPE nvs_tasks_in_flight gauge"));

        // Three requests over two cores, under one route's declared name, and
        // the one neither shared.
        assert!(
            lines.contains(
                &r#"nvs_requests_total{method="GET",route="products.show",status="200"} 3"#
            ),
            "the cores' counters were not added: {scraped}"
        );
        assert!(lines.contains(&r#"nvs_requests_total{method="POST",route="",status="500"} 1"#));
        // Gauges are summed for the same reason counters are: each is that
        // core's own reading of a per-core quantity.
        assert!(lines.contains(&"nvs_tasks_in_flight 7"));

        // 30ms is past the third boundary and inside the fourth, so nothing is
        // under `le="0.025"` and everything is from `le="0.05"` up.
        let bucket = |bound: &str, count: &str| {
            format!(
                r#"nvs_request_duration_seconds_bucket{{method="GET",route="products.show",status="200",le="{bound}"}} {count}"#
            )
        };
        assert!(lines.contains(&bucket("0.025", "0").as_str()), "{scraped}");
        assert!(lines.contains(&bucket("0.05", "3").as_str()), "{scraped}");
        assert!(lines.contains(&bucket("+Inf", "3").as_str()), "{scraped}");
        assert!(lines.contains(
            &r#"nvs_request_duration_seconds_count{method="GET",route="products.show",status="200"} 3"#
        ));
        assert!(lines.contains(
            &r#"nvs_request_duration_seconds_sum{method="GET",route="products.show",status="200"} 0.09"#
        ));

        // A series no core ever wrote is still exported, at zero, which is what
        // makes a `rate()` over it correct from the first scrape.
        assert!(lines.contains(&"nvs_deferred_trees 0"));

        // A label value the format has to escape, which is the only thing a
        // `Core\Metrics` caller can put in one.
        assert!(
            lines.contains(&r#"app_orders_total{shop="a \"quoted\" name"} 1"#),
            "a quoted label value was not escaped: {scraped}"
        );

        // Every line is either a header or a sample whose last field is a
        // number, and the body ends with a newline: a scrape is parsed whole or
        // refused whole. Split from the right, because a label value may hold a
        // space and the value never does.
        assert!(scraped.ends_with('\n'));
        for line in &lines {
            if line.starts_with("# TYPE ") {
                continue;
            }
            let (_, sample) = line.rsplit_once(' ').expect("a sample carries a value");
            assert!(
                sample.parse::<f64>().is_ok() || matches!(sample, "NaN" | "+Inf" | "-Inf"),
                "a line was neither a header nor a sample: {line}"
            );
        }
    }

    /// [`super::scrape_every_core`] reaches a core that is not the one
    /// scraping, which is the one thing the case above takes as given by
    /// handing [`super::scrape`] the registries itself.
    ///
    /// Asserted on the other core's own route label rather than on the body as
    /// a whole, because every other test in this binary that serves a request
    /// is a core in the same roster and its series are in this scrape too.
    #[test]
    fn a_process_scrape_reaches_a_core_that_is_not_the_one_scraping() {
        let gate = Arc::new(Barrier::new(2));
        let serving = {
            let gate = Arc::clone(&gate);
            thread::spawn(move || {
                crate::metrics::meter_this_core(&Config {
                    metrics: Some(Metrics {
                        exporter: Some(Setting::Text("prometheus".to_owned())),
                        ..Metrics::default()
                    }),
                    ..Config::default()
                });
                crate::metrics::count_request(
                    "GET",
                    200,
                    Some("scrape.elsewhere"),
                    Duration::from_millis(30),
                );
                gate.wait();
                gate.wait();
            })
        };

        gate.wait();
        let scraped = super::scrape_every_core();
        gate.wait();
        serving.join().expect("the metering thread panicked");

        assert!(
            scraped.lines().any(|line| line
                == r#"nvs_requests_total{method="GET",route="scrape.elsewhere",status="200"} 1"#),
            "the scrape did not reach the other core: {scraped}"
        );
        assert!(scraped.lines().any(|line| line
            == r#"nvs_request_duration_seconds_count{method="GET",route="scrape.elsewhere",status="200"} 1"#));
    }
}
