//! `rule:observability/metrics-and-trace-blocks-are-system`'s two exporters: which sink `[metrics]`
//! and `[trace]` may ship to, the fraction a head sample is written as, and the boot-time refusal
//! of everything else.
//!
//! **The two blocks share a module because they share a grammar and not a roster.** `exporter` is
//! written the same way in both — `false` for no export, otherwise one word naming a protocol — and
//! [`Exporter::of`] is the one place a word becomes one. What differs is which of them a block
//! accepts: § 6 gives metrics a scrape or a push and gives a trace only a push, so [`Block`] carries
//! the asymmetry and neither roster is spelled twice.
//!
//! **Refused where it is written, never where it is used**, which is [`crate::log`]'s rule. Both
//! blocks are `System` ([`crate::directive`]), so the value in force is the one the boot read, and
//! what a wrong one produces is *silence*: a collector nothing ever writes to looks exactly like a
//! deployment with nothing to say, and every dashboard over it is empty rather than wrong. There is
//! no later moment at which that reports itself, which is the same argument § 6 makes for the class.
//!
//! `sample` is refused on the same footing and for a sharper reason. § 6 writes it as a fraction of
//! one, and a number outside that does not name a smaller or larger sample — it names nothing, and
//! what an eventual reader does with `sample = 5` depends entirely on which side of a comparison it
//! lands on. Both readings are plausible and neither is what anyone wrote it for.
//!
//! **The resolved reading lives here too**, as [`Metering`], for the reason [`crate::server`] keeps
//! `Capacity` beside its own refusal: § 6's written defaults are part of what a block *means*, and a
//! reader that applied them itself would be a second place for `max_series = 10000` to be written
//! down. What comes out is what a core is asked to build — the protocol in force and § 7's bound —
//! and `None` where `exporter = false` says to build nothing.
//!
//! Cost: one match over a short string per block in the merged tree, at boot and at reload. Per
//! request there is one field read, [`head_sample`]'s, and nothing else: the exporters a block names
//! are resolved into what a core builds, while the head fraction is a number the door draws against
//! on the request in front of it and so is read where a request is.

use std::collections::BTreeMap;

use nvs_diagnostics::{Diagnostic, code};

use crate::resolve::{Origin, origin_note};
use crate::tree::{Config, Setting};
use crate::value::as_written;

/// One of `rule:observability/metrics-and-trace-blocks-are-system`'s two export protocols, as written.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Exporter {
    /// `prometheus` — the scrape endpoint served at `[metrics] listen`.
    ///
    /// A pull, so what it can answer with is the *current* value of a series. That is why no trace
    /// block accepts it rather than an oversight in § 6's table: a span is a finished record, and
    /// there is no current value of one to answer a scrape with.
    Prometheus,
    /// `otlp` — a push to the collector named by the block's own `endpoint`.
    Otlp,
}

impl Exporter {
    /// What `written` names, or `None` for a word § 6 gives neither block.
    #[must_use]
    pub fn of(written: &str) -> Option<Self> {
        match written {
            "prometheus" => Some(Self::Prometheus),
            "otlp" => Some(Self::Otlp),
            _ => None,
        }
    }
}

/// § 6's own `max_series`, for a `[metrics]` block that writes an exporter and not a bound.
///
/// `pub` because a registry can exist without a `[metrics]` block at all: `Core\Metrics` is Tier 0
/// in every build (`rule:observability/the-exporter-is-a-feature-and-core-metrics-is-not`), so a
/// `nvs run` that writes a metric accumulates into one this file never described, and
/// `rule:observability/past-max-series-a-new-series-is-refused` holds it to the same number rather
/// than to a second one written somewhere else.
pub const DEFAULT_MAX_SERIES: u64 = 10_000;

/// `rule:observability/metrics-and-trace-blocks-are-system`'s `[metrics]` block, resolved into what one core is asked to build: where its
/// series ship to, and § 7's bound on how many of them it may hold.
///
/// Only the two values a registry needs. `listen` and `endpoint` are the exporter's own address and
/// are read where the exporter is built, which is a crate this one does not know about; putting them
/// here would make this the shape of an exporter rather than of a block.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Metering {
    /// The protocol § 6 named, already checked by [`validate`] against [`Block::Metrics`].
    pub exporter: Exporter,
    /// `[metrics] max_series`, or § 6's own `10000` where the key is unwritten. Per core (§ 7).
    pub max_series: u64,
}

impl Metering {
    /// What the merged tree asks a core to meter, or `None` where nothing does.
    ///
    /// `None` covers three ways of saying the same thing — no `[metrics]` block, no `exporter` key,
    /// and `exporter = false` — because § 6 writes the default as `false` and a block that only
    /// sets `max_series` has bounded a registry it never asked for. A word naming no protocol is
    /// `None` as well and cannot be reached: [`validate`] refused it at boot, and a caller holding a
    /// [`Config`] holds one that got past that.
    #[must_use]
    pub fn of(config: &Config) -> Option<Self> {
        let metrics = config.metrics.as_ref()?;
        let Setting::Text(written) = metrics.exporter.as_ref()? else {
            return None;
        };
        Some(Self {
            exporter: Exporter::of(written)?,
            max_series: metrics.max_series.unwrap_or(DEFAULT_MAX_SERIES),
        })
    }
}

/// `rule:observability/metrics-and-trace-blocks-are-system`'s `[trace]` block, resolved into what a
/// process is asked to push: the protocol § 6 named, and the collector it ships to.
///
/// Beside [`Metering`] and read the same way, with one difference the asymmetry in § 6 forces: a
/// trace has only a push, so the address is not the exporter's own to bind but the thing it dials,
/// and a block naming a protocol with nowhere to send it has asked for silence. `endpoint` is
/// therefore carried here — borrowed rather than copied, so this stays a reading of the block and
/// not a second place the URL is stored — and what a *reachable* endpoint is remains the pusher's
/// question (`nvs_server::otlp::Endpoint`), which is where a URL is parsed and an address resolved.
///
/// The fraction is not here. [`head_sample`] is read per request off the snapshot standing at the
/// door, and folding it into a value resolved at a boot is exactly the reload that key is written
/// to have.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Tracing<'a> {
    /// The protocol § 6 named, already checked by [`validate`] against [`Block::Trace`].
    pub exporter: Exporter,
    /// `[trace] endpoint`, as written, or `None` where the block names no collector.
    pub endpoint: Option<&'a str>,
}

impl<'a> Tracing<'a> {
    /// What the merged tree asks this process to push, or `None` where nothing does.
    ///
    /// `None` covers the same three ways of writing `exporter = false` [`Metering::of`] covers, and
    /// for its reason. A word naming no protocol cannot be reached: [`validate`] refused it at boot.
    #[must_use]
    pub fn of(config: &'a Config) -> Option<Self> {
        let trace = config.trace.as_ref()?;
        let Setting::Text(written) = trace.exporter.as_ref()? else {
            return None;
        };
        Some(Self {
            exporter: Exporter::of(written)?,
            endpoint: trace.endpoint.as_deref(),
        })
    }
}

/// § 6's own `sample`, for a tree that writes no fraction: a trace this process starts is recorded
/// by nothing until an operator says how much of it to record.
const DEFAULT_SAMPLE: f64 = 0.0;

/// `rule:observability/sampling-is-head-based`'s fraction, resolved: the probability that a request
/// which *starts* a trace is recorded.
///
/// Beside [`Metering`] and for its reason — § 6's written default is part of what `[trace]` means,
/// and a door that applied `0.0` itself would be a second place for it to be written down.
///
/// Read per request rather than once at a boot, because `sample` is one of the keys
/// `rule:config/reloadability-is-its-own-field` has reloading despite being `System`: a request
/// draws against what the published snapshot says, and an edit reaches the next request. What comes
/// back is finite and inside `0.0..=1.0` — [`validate`] refused anything else where it was written,
/// and a caller holding a [`Config`] holds one that got past that.
#[must_use]
pub fn head_sample(config: &Config) -> f64 {
    config
        .trace
        .as_ref()
        .and_then(|trace| trace.sample)
        .unwrap_or(DEFAULT_SAMPLE)
}

/// Which block an `exporter` was written in: its name, and the protocols § 6 gives it.
///
/// A block rather than a bare roster because every half of the refusal below differs between the
/// two — the key an origin is recorded under, the sentence quoting § 6, and the one near miss worth
/// naming — and threading three arguments through would let them drift apart.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Block {
    /// `[metrics]`, which may be scraped or pushed.
    Metrics,
    /// `[trace]`, which may only be pushed.
    Trace,
}

impl Block {
    /// The block's name, as `nvs.toml` spells it and as the merge keys it.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Metrics => "metrics",
            Self::Trace => "trace",
        }
    }

    /// The protocols § 6 gives this block, in the order that section lists them.
    #[must_use]
    pub fn accepts(self) -> &'static [Exporter] {
        match self {
            Self::Metrics => &[Exporter::Prometheus, Exporter::Otlp],
            Self::Trace => &[Exporter::Otlp],
        }
    }

    /// § 6's own spellings for this block, for a refusal to quote back.
    fn roster(self) -> &'static str {
        match self {
            Self::Metrics => "`false`, `\"prometheus\"` and `\"otlp\"`",
            Self::Trace => "`false` and `\"otlp\"`",
        }
    }
}

/// § 6's two blocks, asked of the merged tree: both `exporter` values, and `[trace] sample`.
///
/// # Errors
///
/// One [`Diagnostic`] for the first value that names nothing — `E0627` for an exporter, `E0628` for
/// a sample — naming the value, the spellings that would have worked, and the file it was written
/// in.
pub fn validate(config: &Config, origins: &BTreeMap<String, Origin>) -> Result<(), Diagnostic> {
    if let Some(metrics) = config.metrics.as_ref()
        && let Some(written) = metrics.exporter.as_ref()
    {
        exporter(written, Block::Metrics, origins)?;
    }
    if let Some(trace) = config.trace.as_ref() {
        if let Some(written) = trace.exporter.as_ref() {
            exporter(written, Block::Trace, origins)?;
        }
        if let Some(fraction) = trace.sample {
            sampled(fraction, origins)?;
        }
    }
    Ok(())
}

/// [`validate`]'s refusal for one written `exporter`, under the block it was written in.
///
/// `false` is checked before the roster and not as a member of it: § 6 writes the disabled state as
/// a boolean rather than as an `"off"` protocol, which [`Setting`]'s own doc comment gives the
/// reason for, and a roster with a variant meaning "none" would be a second spelling of it.
fn exporter(
    written: &Setting,
    block: Block,
    origins: &BTreeMap<String, Origin>,
) -> Result<(), Diagnostic> {
    if matches!(written, Setting::Bool(false)) {
        return Ok(());
    }
    if let Setting::Text(word) = written
        && Exporter::of(word).is_some_and(|protocol| block.accepts().contains(&protocol))
    {
        return Ok(());
    }

    // The one near miss with a reason behind it, rather than a typo: `prometheus` is a real
    // exporter this configuration has, written in the block that cannot use it.
    let help = if block == Block::Trace && written == &Setting::Text("prometheus".to_owned()) {
        "a scrape answers with a series' current value and a span is a finished record, so there \
         is nothing for a scrape of one to return — push spans with `exporter = \"otlp\"`, and \
         scrape the metrics from `[metrics]`"
            .to_owned()
    } else {
        format!(
            "write `exporter = false` for no export, or one of {}",
            block.roster()
        )
    };

    Err(Diagnostic::error(
        code::E_UNSPELLED_EXPORTER,
        format!(
            "`[{}] exporter = {}` names no exporter this block has",
            block.name(),
            quoted(written)
        ),
    )
    .with_note(format!(
        "`rule:observability/metrics-and-trace-blocks-are-system` gives `[{}]` {}, and nothing else{}",
        block.name(),
        block.roster(),
        origin_note(origins.get(&format!("{}.exporter", block.name())))
    ))
    .with_help(help))
}

/// [`validate`]'s refusal for a `[trace] sample` that is not a fraction of one.
///
/// Non-finite is refused here rather than left to the comparison: TOML spells `nan` and `inf`, and
/// every ordering against a `NaN` is false, so a head sample written as one would read as "record
/// nothing" through the same expression that reads `0.0` that way. An operator who wrote it meant
/// the opposite at least as often.
fn sampled(fraction: f64, origins: &BTreeMap<String, Origin>) -> Result<(), Diagnostic> {
    if fraction.is_finite() && (0.0..=1.0).contains(&fraction) {
        return Ok(());
    }
    Err(Diagnostic::error(
        code::E_SAMPLE_NOT_A_FRACTION,
        format!("`[trace] sample = {fraction}` is not a fraction of one"),
    )
    .with_note(format!(
        "`rule:observability/metrics-and-trace-blocks-are-system`'s head sample runs from `0.0` to `1.0` inclusive{}",
        origin_note(origins.get("trace.sample"))
    ))
    .with_help(
        "write `sample = 0.01` for one trace in a hundred, `sample = 1.0` for every one, or \
         `sample = 0.0` for none — an inbound sampled trace is continued whatever this says"
            .to_owned(),
    ))
}

/// The value as the operator wrote it, quoted as TOML quotes it, for a message to hand back.
fn quoted(value: &Setting) -> String {
    match value {
        Setting::Text(text) => format!("\"{text}\""),
        other => as_written(other),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use nvs_diagnostics::code;

    use super::{Block, Exporter, validate};
    use crate::tree::{Config, Metrics, Setting, Trace};

    /// A tree whose `[metrics]` block writes `exporter`, and nothing else.
    fn metrics(exporter: Setting) -> Config {
        Config {
            metrics: Some(Metrics {
                exporter: Some(exporter),
                ..Metrics::default()
            }),
            ..Config::default()
        }
    }

    /// A tree whose `[trace]` block writes the two values this module checks.
    fn trace(exporter: Option<Setting>, sample: Option<f64>) -> Config {
        Config {
            trace: Some(Trace {
                exporter,
                sample,
                ..Trace::default()
            }),
            ..Config::default()
        }
    }

    /// § 6's rosters, asserted as an asymmetry rather than as two lists: every protocol resolves as
    /// a word, and `prometheus` is the one that resolves while still being refused where it was
    /// written. A block that grew its neighbour's roster passes both halves read alone.
    #[test]
    fn a_trace_takes_only_the_push_and_metrics_take_both() {
        for protocol in Block::Metrics.accepts() {
            assert!(Block::Trace.accepts().contains(protocol) == (*protocol == Exporter::Otlp));
        }

        validate(
            &metrics(Setting::Text("prometheus".to_owned())),
            &BTreeMap::new(),
        )
        .expect("§ 6's scrape");
        validate(&metrics(Setting::Text("otlp".to_owned())), &BTreeMap::new())
            .expect("§ 6's push, for metrics");
        validate(
            &trace(Some(Setting::Text("otlp".to_owned())), None),
            &BTreeMap::new(),
        )
        .expect("§ 6's push, for spans");

        // The word is a real exporter and `Exporter::of` says so; what refuses it is the block.
        assert_eq!(Exporter::of("prometheus"), Some(Exporter::Prometheus));
        let refused = validate(
            &trace(Some(Setting::Text("prometheus".to_owned())), None),
            &BTreeMap::new(),
        )
        .expect_err("a scrape of a span");
        assert_eq!(refused.code, Some(code::E_UNSPELLED_EXPORTER));
        assert!(
            refused
                .notes
                .iter()
                .any(|note| note.contains("finished record")),
            "the near miss earns § 6's reason rather than the roster: {:?}",
            refused.notes
        );
    }

    /// `false` is the disabled state and `true` is not a second spelling of anything: § 6 gives the
    /// key one boolean, so the other one names no exporter and is refused with the rest.
    #[test]
    fn only_false_disables_an_export_and_true_names_nothing() {
        validate(&metrics(Setting::Bool(false)), &BTreeMap::new()).expect("no metrics export");
        validate(&trace(Some(Setting::Bool(false)), None), &BTreeMap::new())
            .expect("no trace export");

        for wrong in [
            Setting::Bool(true),
            Setting::Integer(1),
            Setting::Text("statsd".to_owned()),
            Setting::List(vec!["otlp".to_owned()]),
        ] {
            let refused = validate(&metrics(wrong.clone()), &BTreeMap::new())
                .expect_err("no exporter is spelled this way");
            assert_eq!(refused.code, Some(code::E_UNSPELLED_EXPORTER), "{wrong:?}");
        }
    }

    /// § 6's head sample, both bounds named together: `0.0` and `1.0` are inside it, the first
    /// value past either end is not, and neither non-finite spelling TOML has is a sample at all.
    #[test]
    fn the_head_sample_is_a_fraction_of_one_at_both_ends() {
        for inside in [0.0, 0.01, 1.0] {
            validate(&trace(None, Some(inside)), &BTreeMap::new())
                .unwrap_or_else(|_| panic!("{inside} is a fraction of one"));
        }
        for outside in [-0.000_001, 1.000_001, 100.0, f64::NAN, f64::INFINITY] {
            let refused = validate(&trace(None, Some(outside)), &BTreeMap::new())
                .expect_err("not a fraction of one");
            assert_eq!(
                refused.code,
                Some(code::E_SAMPLE_NOT_A_FRACTION),
                "{outside}"
            );
        }
    }

    /// A tree that configures no export still boots, which is what makes the default in § 6's table
    /// the *absence* of an exporter rather than a value anything has to write.
    #[test]
    fn a_tree_that_configures_no_export_still_boots() {
        validate(&Config::default(), &BTreeMap::new()).expect("neither block");
        validate(
            &Config {
                metrics: Some(Metrics::default()),
                trace: Some(Trace::default()),
                ..Config::default()
            },
            &BTreeMap::new(),
        )
        .expect("both blocks, written empty");
    }
}
