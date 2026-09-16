//! `rule:observability/a-registry-is-per-core-and-nothing-reads-it` and `rule:observability/past-max-series-a-new-series-is-refused`'s per-core
//! registry: the series one core accumulates between scrapes, the shape each
//! name is fixed to on first use, and the bound past which a new one is refused.
//!
//! # Why the accumulator is here and the exporter is not
//!
//! `rule:observability/the-exporter-is-a-feature-and-core-metrics-is-not` puts
//! `Core\Metrics` in every build and the exporter behind a Cargo feature, and
//! that is a split between two crates as much as between two builds:
//! `nvs-stdlib` owns the class, `nvs-server` owns the scrape, and neither may
//! depend on the other. So the registry the two of them share sits under both,
//! in the crate that owns per-request state — a `Core` member reaches it
//! through [`record`] with nothing but a [`crate::Ctx`] in hand, and
//! `nvs_server::prometheus` reads it through [`every_core`]. What stays in
//! `nvs-server` is the exposition format and the listener, which is what its
//! `exporter` feature removes.
//!
//! **A registry is therefore not the server's to create.** [`meter_this_core`]
//! is how a serving core gets the one `[metrics]` configured, and [`record`]
//! builds one on any thread that writes a metric without having been metered —
//! which is every `nvs run`, and which is § 3's "accumulates even with no
//! exporter built" rather than a second kind of registry.
//!
//! # Why a core owns it, and why that is not `rule:concurrency/cross-request-state-is-explicit`'s closed door
//!
//! A registry is mutable state outliving a request, which is the shape
//! `rule:concurrency/cross-request-state-is-explicit` exists to
//! constrain, and § 5 is where it passes that ADR's own test: nothing reads a
//! metric to make a decision — no Novis program can read one at all — the values
//! are approximate aggregates whose merge across cores is arithmetic rather than
//! coordination, § 4 forbids any request-derived value from becoming a label,
//! and the memory is charged to the core and capped. That is the same exception
//! 0059 § 3 already records for `Core\Cache::local`, and it is why a core's
//! registry is its own rather than a store every core writes into: a counter two
//! cores contended over would be coordination bought for a number that is
//! defined as approximate. [`meter_this_core`] is where a core takes one,
//! [`count_request`] is what the door reaches it through, and [`every_core`] is
//! the copy of each of them that a scrape merges.
//!
//! # How a scrape reaches a core it is not running on
//!
//! The reader is never the core that wrote the series. One process binds one
//! `[metrics] listen`, so the core answering a scrape has to come by every
//! other core's counters somehow, and it comes by them through a **handle**:
//! each core's registry lives behind a lock of its own, the process keeps a
//! weak handle onto each, and [`every_core`] copies what is behind them.
//!
//! One lock **per core**, which is the whole of why this is not the shared
//! store § 5 refuses. A core takes only its own — uncontended on every count
//! except during the moment a scrape is copying it — and no two cores ever
//! touch the same word. The alternative considered was a message, the scraping
//! core poking each of the others and waiting for it to publish, and
//! `docs/decisions/0004.md`'s ordering refused it: the two cost the same per
//! request, nothing either of them does being measurable against the map lookup
//! a count already makes, and the message needs a wake, a reply and a timeout
//! for the core that is idle or is deep inside one long request, where a lock
//! held for the length of a copy needs none of them. Simplicity decides where
//! latency cannot tell two designs apart.
//!
//! A handle is **weak**, so the roster is O(cores serving) rather than O(threads
//! this process has ever started): a core that ended has already released its
//! registry, and the next gather drops the empty handle.
//!
//! # Refuse the new, never evict the old
//!
//! [`Registry::max_series`] is § 7's bound, per core. Past it a series that does
//! not exist yet is **refused** — the call is a no-op and the refusal is counted
//! against the metric's name — and one that does is untouched. Evicting instead
//! would make a counter's value appear to reset when it was next recreated,
//! which every backend reads as a process restart and which silently corrupts
//! every `rate()` over it. A missing series is a hole in a dashboard; a
//! recreated one is a wrong number on it, and § 7 chose the hole.
//!
//! **§ 1's declared series are seeded past the bound rather than counted through
//! it.** They are what the ADR promises exists "the moment an exporter is
//! configured", so a `max_series` small enough to refuse them would quietly turn
//! that promise into a different configuration; the bound applies to everything
//! registered afterwards, which is the entire population it exists to hold down.
//!
//! # A name is fixed to one kind, one label set and one bucket table
//!
//! § 3 fixes a name to a kind on first use within a process, and this module
//! fixes its label *names* with it, because the exposition format both exporters
//! write has no way to say that two members of one family disagree about which
//! labels they have. Both mismatches come back as a [`Refused`] naming what the
//! name was fixed to, which is what lets `Core\Metrics` throw for the kind (§ 3
//! calls it a mistake and not a mode) while a cardinality refusal stays the
//! no-op § 7 asks for.
//!
//! A histogram carries its boundaries for the same reason: a table whose first
//! boundary sits above every observation records a count and answers nothing, so
//! [`PAUSE_BUCKETS`] is what `nvs_gc_pause_seconds` gets and
//! [`LATENCY_BUCKETS`] is what a request duration does.
//!
//! # What it spends
//!
//! As `rule:programs/memory-priority`
//! requires: O(cores × series), bounded by `max_series` per core, a counter or
//! gauge costing its key and eight bytes and a histogram its bucket array on top.
//! Nothing is charged to a request, nothing grows with requests served, and a
//! process with no exporter configured builds no registry at all.
//!
//! # Known gaps
//!
//! 1. **Only a scrape reads this.** `nvs_server::prometheus` binds `[metrics]
//!    listen` and answers a collector with [`every_core`]; `[metrics] endpoint`
//!    has no pusher, so a tree naming `exporter = "otlp"` builds a registry
//!    nothing ships. That module is the half a build without the `exporter`
//!    feature loses, and this one is in every build either way.
//!    — owner: unowned-closures

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};
use std::time::Duration;

use nvs_config::{Config, DEFAULT_MAX_SERIES, Exporter, Metering};

/// The boundaries a duration histogram over a request, a query or a spawn is
/// bucketed into, in seconds — the range a web request lives in, from a cache
/// hit to a wait nobody wants.
pub const LATENCY_BUCKETS: &[f64] = &[
    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];

/// The boundaries a pause histogram is bucketed into, in seconds.
///
/// Three orders of magnitude below [`LATENCY_BUCKETS`] because that is where a
/// GC pause lives: measured against the request table, every pause this runtime
/// produces would land in the first bucket and the histogram would report only
/// what its `_count` already said.
pub const PAUSE_BUCKETS: &[f64] = &[
    0.000_01, 0.000_05, 0.000_1, 0.000_5, 0.001, 0.005, 0.01, 0.05, 0.1,
];

/// What a series counts, and therefore how it is written and how it is read.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    /// A value that only rises, read as a rate.
    Counter,
    /// An observation bucketed into a fixed table, read as a quantile.
    Histogram,
    /// A point-in-time value that rises and falls.
    Gauge,
}

impl Kind {
    /// The kind's name, for a refusal to quote back.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Counter => "counter",
            Self::Histogram => "histogram",
            Self::Gauge => "gauge",
        }
    }

    /// The boundaries a histogram of this kind gets when nothing names them.
    fn buckets(self) -> &'static [f64] {
        match self {
            Self::Histogram => LATENCY_BUCKETS,
            Self::Counter | Self::Gauge => &[],
        }
    }
}

/// One of `rule:observability/default-series`'s declared series: a name, what it counts, and the label
/// names every member of it carries.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Family {
    /// The series name, as § 1 spells it.
    pub name: &'static str,
    /// What it counts.
    pub kind: Kind,
    /// The label names, in § 1's order. Every member carries all of them.
    pub labels: &'static [&'static str],
    /// The boundaries, for a histogram; empty otherwise.
    pub buckets: &'static [f64],
}

/// `rule:observability/default-series`'s declared series, present the moment an exporter is configured.
///
/// Each is read from instrumentation that already exists, which is § 1's own
/// governing claim and the reason no probe site is added anywhere to carry them.
/// A name here is fixed to its kind and its labels before any code runs, so the
/// first mistaken `Core\Metrics::observe("nvs_requests_total", …)` is refused
/// rather than recorded.
pub const DEFAULT: &[Family] = &[
    Family {
        name: "nvs_requests_total",
        kind: Kind::Counter,
        labels: &["method", "status", "route"],
        buckets: &[],
    },
    Family {
        name: "nvs_request_duration_seconds",
        kind: Kind::Histogram,
        labels: &["method", "status", "route"],
        buckets: LATENCY_BUCKETS,
    },
    Family {
        name: "nvs_db_query_duration_seconds",
        kind: Kind::Histogram,
        labels: &["connection", "operation"],
        buckets: LATENCY_BUCKETS,
    },
    Family {
        name: "nvs_gc_pause_seconds",
        kind: Kind::Histogram,
        labels: &[],
        buckets: PAUSE_BUCKETS,
    },
    Family {
        name: "nvs_spawn_duration_seconds",
        kind: Kind::Histogram,
        labels: &["kind"],
        buckets: PAUSE_BUCKETS,
    },
    Family {
        name: "nvs_tasks_in_flight",
        kind: Kind::Gauge,
        labels: &[],
        buckets: &[],
    },
    Family {
        name: "nvs_deferred_trees",
        kind: Kind::Gauge,
        labels: &[],
        buckets: &[],
    },
    Family {
        name: "nvs_memory_bytes",
        kind: Kind::Gauge,
        labels: &["scope"],
        buckets: &[],
    },
    Family {
        name: "nvs_schedule_runs_total",
        kind: Kind::Counter,
        labels: &["name", "outcome"],
        buckets: &[],
    },
];

/// One series: a name and the label values that pick it out of its family.
///
/// Labels are held sorted by name, so two calls writing the same pair in two
/// orders reach one series rather than two — which is a cardinality question and
/// not a tidiness one.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Series {
    /// The family's name.
    pub name: String,
    /// The label values, by label name, sorted.
    pub labels: Vec<(String, String)>,
}

/// A histogram's accumulated observations.
#[derive(Clone, Debug, PartialEq)]
pub struct Histogram {
    /// The boundaries, in the unit of the series, ascending.
    pub bounds: &'static [f64],
    /// One count per boundary, plus a last slot for everything above the
    /// highest — the `+Inf` bucket both exposition formats require. Held per
    /// bucket rather than cumulatively because an exporter can add and this
    /// cannot subtract without rounding twice.
    pub counts: Vec<u64>,
    /// The sum of every observation.
    pub sum: f64,
    /// How many observations there have been.
    pub count: u64,
}

impl Histogram {
    /// An empty histogram over `bounds`.
    fn new(bounds: &'static [f64]) -> Self {
        Self {
            bounds,
            counts: vec![0; bounds.len() + 1],
            sum: 0.0,
            count: 0,
        }
    }

    /// Records one observation.
    ///
    /// A `NaN` compares false against every boundary and so lands in the last
    /// slot; that is the one reading available and it is why the sum of a
    /// histogram that saw one is `NaN` as well, which is the honest answer.
    fn observe(&mut self, value: f64) {
        let slot = self
            .bounds
            .iter()
            .position(|bound| value <= *bound)
            .unwrap_or(self.bounds.len());
        self.counts[slot] = self.counts[slot].saturating_add(1);
        self.sum += value;
        self.count = self.count.saturating_add(1);
    }
}

/// What one series holds.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// A counter's total.
    Counter(u64),
    /// A histogram's buckets, sum and count.
    Histogram(Histogram),
    /// A gauge's current value.
    Gauge(f64),
}

impl Value {
    /// This value's kind.
    #[must_use]
    pub fn kind(&self) -> Kind {
        match self {
            Self::Counter(_) => Kind::Counter,
            Self::Histogram(_) => Kind::Histogram,
            Self::Gauge(_) => Kind::Gauge,
        }
    }
}

/// Why a write reached no series.
///
/// A mismatch is the caller's mistake and a cardinality refusal is the
/// deployment's, which is the distinction a caller acts on: § 3 makes a kind or
/// a label mismatch a throw naming what the name was fixed to, while § 7's
/// cardinality refusal is a no-op with a warning behind it
/// ([`Registry::refusals`]).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Refused {
    /// The name is fixed to another kind (§ 3).
    Kind {
        /// The series name.
        name: String,
        /// What first use fixed it to.
        fixed: Kind,
        /// What this call asked for.
        asked: Kind,
    },
    /// The name is fixed to another set of label names.
    Labels {
        /// The series name.
        name: String,
        /// The label names first use fixed, sorted.
        fixed: Vec<String>,
        /// The label names this call wrote, sorted.
        asked: Vec<String>,
    },
    /// The core already holds `max_series` series and this one is new (§ 7).
    Cardinality {
        /// The series name whose newest labels were lost.
        name: String,
        /// The bound in force.
        max_series: u64,
    },
}

impl Refused {
    /// The metric this refusal is about, which is what § 7's warning names.
    #[must_use]
    pub fn metric(&self) -> &str {
        match self {
            Self::Kind { name, .. }
            | Self::Labels { name, .. }
            | Self::Cardinality { name, .. } => name,
        }
    }
}

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Kind { name, fixed, asked } => write!(
                f,
                "`{name}` is a {} and this call records a {}",
                fixed.name(),
                asked.name()
            ),
            Self::Labels { name, fixed, asked } => write!(
                f,
                "`{name}` carries the labels {} and this call wrote {}",
                fixed.join(", "),
                asked.join(", ")
            ),
            Self::Cardinality { name, max_series } => write!(
                f,
                "`{name}` produced a new series past this core's `max_series` of {max_series}, \
                 and it was not recorded"
            ),
        }
    }
}

/// What a name was fixed to on first use, and where.
#[derive(Clone, Debug, PartialEq)]
struct Shape {
    kind: Kind,
    labels: Vec<String>,
    buckets: &'static [f64],
    /// The `file:line` of the `Core\Metrics` call that fixed it, or `None` for
    /// a name fixed by something with no call site — [`DEFAULT`]'s declared
    /// families, and [`Registry::request`]'s two.
    ///
    /// Held so that the throw a later mismatch produces can name **both**
    /// sites, which is `rule:observability/metrics-three-members`'s kind being
    /// a property of the series: a reader of that message wants the line that
    /// decided the kind at least as much as the line that disagreed with it.
    site: Option<String>,
}

/// One `Core\Metrics` write: which verb wrote it, and what it wrote.
///
/// The three verbs differ in the kind they fix a name to and in what they do to
/// the value behind it, and in nothing else — so one entry point takes this
/// rather than three near-identical ones, and
/// `rule:observability/metrics-three-members`'s "three verbs for three kinds"
/// is one `match` in [`Registry::record`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Written {
    /// `Core\Metrics::increment` — this much onto a counter.
    Increment(u64),
    /// `Core\Metrics::observe` — one observation into a histogram.
    Observe(f64),
    /// `Core\Metrics::gauge` — a gauge set to this.
    Gauge(f64),
}

impl Written {
    /// The kind this verb fixes a name to.
    #[must_use]
    pub fn kind(self) -> Kind {
        match self {
            Self::Increment(_) => Kind::Counter,
            Self::Observe(_) => Kind::Histogram,
            Self::Gauge(_) => Kind::Gauge,
        }
    }
}

/// One core's series, and the bound it holds them under.
///
/// Built by [`Registry::of`] where an exporter is configured, and by
/// [`Registry::new`] where § 3's members accumulate without one.
#[derive(Clone, Debug)]
pub struct Registry {
    exporter: Option<Exporter>,
    max_series: u64,
    shapes: BTreeMap<String, Shape>,
    series: BTreeMap<Series, Value>,
    refusals: BTreeMap<String, u64>,
}

impl Registry {
    /// A registry bounded at `max_series`, carrying § 1's declared series and no
    /// exporter.
    #[must_use]
    pub fn new(max_series: u64) -> Self {
        Self::build(None, max_series)
    }

    /// What `config` asks this core to build, or `None` where `[metrics]`
    /// configures no exporter — in which case no registry exists at all, which
    /// is the cheapest possible reading of `exporter = false`.
    #[must_use]
    pub fn of(config: &Config) -> Option<Self> {
        let metering = Metering::of(config)?;
        Some(Self::build(Some(metering.exporter), metering.max_series))
    }

    /// The protocol this core's series ship to, or `None` for a registry
    /// nothing exports.
    #[must_use]
    pub fn exporter(&self) -> Option<Exporter> {
        self.exporter
    }

    /// § 7's bound, per core.
    #[must_use]
    pub fn max_series(&self) -> u64 {
        self.max_series
    }

    /// How many series this core holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.series.len()
    }

    /// Whether this core holds no series at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.series.is_empty()
    }

    /// Every series this core holds, by name and then by labels — the order an
    /// exporter writes them in, and the reason a scrape of an unchanged core
    /// produces identical bytes.
    pub fn series(&self) -> impl Iterator<Item = (&Series, &Value)> {
        self.series.iter()
    }

    /// One series, by name and labels, or `None` if nothing has written it.
    #[must_use]
    pub fn read(&self, name: &str, labels: &[(&str, &str)]) -> Option<&Value> {
        self.series.get(&key(name, labels))
    }

    /// What a name is fixed to, or `None` for one nothing has used.
    #[must_use]
    pub fn kind(&self, name: &str) -> Option<Kind> {
        self.shapes.get(name).map(|shape| shape.kind)
    }

    /// Where a name was fixed to its kind, as `file:line`, or `None` for a name
    /// nothing has used and for one no call site fixed — see [`Shape::site`].
    #[must_use]
    pub fn fixed_at(&self, name: &str) -> Option<&str> {
        self.shapes.get(name)?.site.as_deref()
    }

    /// The label names a name is fixed to, sorted, or `None` for one nothing has
    /// used.
    #[must_use]
    pub fn labels(&self, name: &str) -> Option<&[String]> {
        self.shapes.get(name).map(|shape| shape.labels.as_slice())
    }

    /// How many series each metric has lost to § 7's bound since this registry
    /// was built.
    ///
    /// The count rather than the fact, because the warning § 7 asks for is one
    /// per window and an operator reading it wants to know whether they lost one
    /// label combination or ten thousand.
    #[must_use]
    pub fn refusals(&self) -> &BTreeMap<String, u64> {
        &self.refusals
    }

    /// Adds `by` to a counter.
    ///
    /// # Errors
    ///
    /// [`Refused`] where the name is fixed to another kind or another label set,
    /// or where the series is new and the core is at `max_series`.
    pub fn increment(
        &mut self,
        name: &str,
        by: u64,
        labels: &[(&str, &str)],
    ) -> Result<(), Refused> {
        self.record(None, name, Written::Increment(by), labels)
    }

    /// Records one observation into a histogram.
    ///
    /// # Errors
    ///
    /// As [`Registry::increment`].
    pub fn observe(
        &mut self,
        name: &str,
        value: f64,
        labels: &[(&str, &str)],
    ) -> Result<(), Refused> {
        self.record(None, name, Written::Observe(value), labels)
    }

    /// Sets a gauge to `value`.
    ///
    /// # Errors
    ///
    /// As [`Registry::increment`].
    pub fn gauge(
        &mut self,
        name: &str,
        value: f64,
        labels: &[(&str, &str)],
    ) -> Result<(), Refused> {
        self.record(None, name, Written::Gauge(value), labels)
    }

    /// One write, from the verb that wrote it and the call site that is about to
    /// fix the name if nothing has yet.
    ///
    /// The three members above are this with no site: nothing inside the engine
    /// has one, and `rule:errors/a-record-names-where-it-was-produced`'s carrier
    /// is the only thing that does.
    ///
    /// # Errors
    ///
    /// As [`Registry::increment`].
    pub fn record(
        &mut self,
        site: Option<&str>,
        name: &str,
        written: Written,
        labels: &[(&str, &str)],
    ) -> Result<(), Refused> {
        let series = self.admit(name, written.kind(), labels, site)?;
        match (written, self.series.get_mut(&series)) {
            (Written::Increment(by), Some(Value::Counter(total))) => {
                *total = total.saturating_add(by);
            }
            (Written::Observe(value), Some(Value::Histogram(histogram))) => {
                histogram.observe(value);
            }
            (Written::Gauge(value), Some(Value::Gauge(held))) => *held = value,
            // Unreachable: `admit` answered `Ok`, so the series exists and its
            // value is the one `empty` built for this verb's own kind.
            _ => {}
        }
        Ok(())
    }

    /// Puts `metering`'s exporter and bound onto a registry that already exists.
    ///
    /// A core can hold one before it is metered: [`record`] builds this thread's
    /// registry on the first `Core\Metrics` call, and on a `nvs run` that is the
    /// only registry there will ever be. Adopting rather than rebuilding is
    /// § *Refuse the new, never evict the old*'s argument one step earlier — a
    /// rebuilt registry would reset a counter that had already been written, and
    /// every backend reads a reset as a process restart.
    fn adopt(&mut self, metering: Metering) {
        self.exporter = Some(metering.exporter);
        self.max_series = metering.max_series;
    }

    /// `rule:observability/default-series`'s two request series, recorded together from what the door
    /// already knows: the verb, the status that went back, and the **route's
    /// declared name** (`nvs_server::route::label`).
    ///
    /// `route` is the label § 1 says would otherwise be unbounded, and this is
    /// the one place its value is chosen. It is a name out of `rule:routing/routes-are-compiled-not-registered`'s
    /// compile-time table and never the request's path, so the label's set is
    /// closed by the compile that produced the table. **A request that matched
    /// nothing carries the label empty rather than not at all**: an exposition
    /// format cannot hold a family whose members disagree about which labels
    /// they have, and a program with no route table then has one series per
    /// method and status, which is § 1's "no `route` label" with a spelling the
    /// wire allows.
    ///
    /// Nothing is returned. Both names are seeded by [`DEFAULT`], so neither
    /// mismatch in [`Refused`] is reachable here, and § 7's cardinality refusal
    /// is the no-op it is defined to be — counted in [`Registry::refusals`] and
    /// never allowed to affect the response that produced it.
    pub fn request(&mut self, method: &str, status: u16, route: Option<&str>, took: Duration) {
        let status = digits(status);
        let status = std::str::from_utf8(&status).unwrap_or("000");
        let labels = [
            ("method", method),
            ("status", status),
            ("route", route.unwrap_or_default()),
        ];
        let _ = self.increment("nvs_requests_total", 1, &labels);
        let _ = self.observe("nvs_request_duration_seconds", took.as_secs_f64(), &labels);
    }

    /// [`Registry::new`] and [`Registry::of`]'s shared body: § 1's roster, then
    /// a zero-valued member of every family that has no labels to distinguish
    /// one member from another.
    ///
    /// A labelled family gets no member until something writes one, because
    /// which members it has is the question the labels answer; an unlabelled one
    /// has exactly one member and reporting it as zero from the first scrape is
    /// what makes a `rate()` over it correct across a restart.
    fn build(exporter: Option<Exporter>, max_series: u64) -> Self {
        let mut registry = Self {
            exporter,
            max_series,
            shapes: BTreeMap::new(),
            series: BTreeMap::new(),
            refusals: BTreeMap::new(),
        };
        for family in DEFAULT {
            let mut labels: Vec<String> = family.labels.iter().map(|&l| l.to_owned()).collect();
            labels.sort();
            registry.shapes.insert(
                family.name.to_owned(),
                Shape {
                    kind: family.kind,
                    labels,
                    buckets: family.buckets,
                    site: None,
                },
            );
            if family.labels.is_empty() {
                registry
                    .series
                    .insert(key(family.name, &[]), empty(family.kind, family.buckets));
            }
        }
        registry
    }

    /// Fixes or checks the name's shape, applies § 7's bound, and makes sure the
    /// series exists — so that the writers above are one lookup each and
    /// share every rule.
    fn admit(
        &mut self,
        name: &str,
        kind: Kind,
        labels: &[(&str, &str)],
        site: Option<&str>,
    ) -> Result<Series, Refused> {
        let series = key(name, labels);
        let written: Vec<String> = series
            .labels
            .iter()
            .map(|(label, _)| label.clone())
            .collect();

        let buckets = match self.shapes.get(name) {
            Some(shape) if shape.kind != kind => {
                return Err(Refused::Kind {
                    name: name.to_owned(),
                    fixed: shape.kind,
                    asked: kind,
                });
            }
            Some(shape) if shape.labels != written => {
                return Err(Refused::Labels {
                    name: name.to_owned(),
                    fixed: shape.labels.clone(),
                    asked: written,
                });
            }
            Some(shape) => shape.buckets,
            None => kind.buckets(),
        };

        // The bound is asked before the shape is written, so a name refused on
        // its very first call leaves nothing behind that a later call would then
        // be measured against.
        let held = u64::try_from(self.len()).unwrap_or(u64::MAX);
        if !self.series.contains_key(&series) && held >= self.max_series {
            *self.refusals.entry(name.to_owned()).or_default() += 1;
            return Err(Refused::Cardinality {
                name: name.to_owned(),
                max_series: self.max_series,
            });
        }

        self.shapes.entry(name.to_owned()).or_insert(Shape {
            kind,
            labels: written,
            buckets,
            site: site.map(str::to_owned),
        });
        self.series
            .entry(series.clone())
            .or_insert_with(|| empty(kind, buckets));
        Ok(series)
    }
}

/// Every serving core's registry, as the weak handle that core filed when it
/// took one.
///
/// The one process-wide thing in this module, and it holds no series: what is
/// behind a handle belongs to one core and is written by that core alone. The
/// module doc § *How a scrape reaches a core it is not running on* owns why the
/// handles are weak and why there is one lock per core rather than one here.
static CORES: Mutex<Vec<Weak<Mutex<Registry>>>> = Mutex::new(Vec::new());

thread_local! {
    /// The registry of the core this thread is.
    ///
    /// A thread-local because that is what "per core" *is* on a runtime which
    /// never migrates a request: the core that accepted a connection is the
    /// core that answers every request on it, so a counter reached from this
    /// cell is reached without going looking for it and without contending with
    /// any other core. One process runs [`meter_this_core`] once per listening
    /// socket per core — `[server] listen` names any number of them — and the
    /// second of those finds what the first built rather than splitting the
    /// core's counters between two sockets.
    ///
    /// This is the **strong** handle, so dropping this cell at thread exit is
    /// what takes a core out of [`CORES`].
    ///
    /// `None` on every thread that is not serving, and on every core of a
    /// process whose `[metrics]` names no exporter.
    static CORE: RefCell<Option<Arc<Mutex<Registry>>>> = const { RefCell::new(None) };
}

/// A lock taken past a poisoning.
///
/// What a panic can leave behind here is a counter one increment out, which is
/// a value and not a broken invariant, so a poisoning is recovered from rather
/// than propagated: refusing every later count because one request panicked
/// would turn a bug in an application into a process with no metrics at all.
fn held<T>(lock: &Mutex<T>) -> MutexGuard<'_, T> {
    lock.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Gives this core the registry `config` asks for, unless it already has one.
///
/// [`Registry::of`] is what decides, so a tree naming no exporter leaves this
/// core with nothing and every count below it a no-op — which is § 3's reading
/// of `exporter = false` and the cheapest one there is.
///
/// **Idempotent**, and that is the contract rather than a convenience: the
/// counters belong to the core and not to the accept loop that happened to
/// build them, so a second loop on the same core adds to the same series. It
/// is also what makes a re-entered loop safe, since a registry that was
/// rebuilt would reset every counter under it and every backend reads that as
/// a process restart (§ 7's own argument against eviction). A core that
/// already holds one **adopts** the exporter into it ([`Registry::adopt`])
/// rather than keeping a registry nothing ships, which is the same sentence
/// read against the registry [`record`] builds.
///
/// A registry is filed in the roster as it is built, which is the whole of how
/// [`every_core`] reaches a core that is not the one scraping.
pub fn meter_this_core(config: &Config) {
    let Some(metering) = Metering::of(config) else {
        return;
    };
    CORE.with_borrow_mut(|core| {
        if let Some(registry) = core.as_ref() {
            held(registry).adopt(metering);
            return;
        }
        let registry = Arc::new(Mutex::new(Registry::build(
            Some(metering.exporter),
            metering.max_series,
        )));
        held(&CORES).push(Arc::downgrade(&registry));
        *core = Some(registry);
    });
}

/// Counts one finished request on this core's registry.
///
/// [`Registry::request`] where this core has one, and nothing at all where it
/// has not: the door calls this for every response it writes, so the absent
/// case is the ordinary one and is a borrow and a branch.
pub fn count_request(method: &str, status: u16, route: Option<&str>, took: Duration) {
    CORE.with_borrow(|core| {
        if let Some(registry) = core.as_ref() {
            held(registry).request(method, status, route, took);
        }
    });
}

/// This thread's registry, built at [`DEFAULT_MAX_SERIES`] if it has none.
///
/// The half of [`meter_this_core`] that is not about an exporter.
/// `rule:observability/the-exporter-is-a-feature-and-core-metrics-is-not` makes
/// `Core\Metrics` Tier 0 in every build, so a `nvs run` — which has no accept
/// loop and therefore no metered core — still accumulates, and it accumulates
/// into a registry with no protocol in force. A thread that never writes a
/// metric builds nothing at all, which is that section's "a program that never
/// calls it holds zero series".
///
/// Filed in [`CORES`] like any other, so a process that is both serving and
/// scraping merges what a `Core\Metrics` call wrote alongside what the door
/// counted.
fn this_core() -> Arc<Mutex<Registry>> {
    CORE.with_borrow_mut(|core| {
        if let Some(registry) = core.as_ref() {
            return Arc::clone(registry);
        }
        let registry = Arc::new(Mutex::new(Registry::new(DEFAULT_MAX_SERIES)));
        held(&CORES).push(Arc::downgrade(&registry));
        *core = Some(Arc::clone(&registry));
        registry
    })
}

/// One `Core\Metrics` write onto this thread's registry, building one if this
/// thread has none.
///
/// `site` is the writing call's own `file:line`, which the member reads off the
/// constant its call was compiled with — see `nvs_stdlib::registry`'s
/// `SOURCE_MEMBERS`. It is held only where it fixes a name
/// ([`Shape::site`]), so the cost is one `String` per *name* a program writes
/// and nothing per call.
///
/// # Errors
///
/// As [`Registry::increment`]. A kind or label mismatch is the caller's mistake
/// and becomes a throw; a cardinality refusal is the deployment's and is the
/// no-op `rule:observability/past-max-series-a-new-series-is-refused` defines —
/// [`Refused`]'s own docs own that split.
pub fn record(
    site: Option<&str>,
    name: &str,
    written: Written,
    labels: &[(&str, &str)],
) -> Result<(), Refused> {
    let registry = this_core();
    held(&registry).record(site, name, written, labels)
}

/// Where this thread's registry fixed `name` to its kind, as `file:line`, or
/// `None` for a name it has not fixed and for one no call site fixed.
///
/// Read on the throwing path alone, which is why it is a second call under a
/// second lock rather than a field of [`Refused`]: what a mismatch costs is a
/// throw, and a shape once fixed never changes, so the answer cannot go stale
/// between the two.
#[must_use]
pub fn fixed_at(name: &str) -> Option<String> {
    CORE.with_borrow(|core| {
        core.as_ref()
            .and_then(|registry| held(registry).fixed_at(name).map(str::to_owned))
    })
}

/// A copy of this core's series, for a caller that is going to send them
/// somewhere else.
///
/// A copy rather than a borrow because the reader is never this core: a scrape
/// merges what every core answers with, and what crosses is values rather than
/// a handle onto a registry that is still being written. `None` where this core
/// built none.
#[must_use]
pub fn on_this_core() -> Option<Registry> {
    CORE.with_borrow(|core| core.as_ref().map(|registry| held(registry).clone()))
}

/// A copy of every serving core's series, in no particular order — the one
/// gather a scrape or a push is made of.
///
/// Each core's handle is upgraded under the roster's lock and copied outside
/// it, so a core that is counting never waits behind the roster and the roster
/// never waits behind a copy. A handle whose core has ended is dropped on the
/// way past, which is the only thing that ever shortens the roster.
///
/// Empty in a process whose `[metrics]` names no exporter, because no core
/// there built a registry to file.
#[must_use]
pub fn every_core() -> Vec<Registry> {
    let serving: Vec<Arc<Mutex<Registry>>> = {
        let mut roster = held(&CORES);
        roster.retain(|core| core.strong_count() > 0);
        roster.iter().filter_map(Weak::upgrade).collect()
    };
    serving
        .iter()
        .map(|registry| held(registry).clone())
        .collect()
}

/// A series key: the name, and the labels sorted by name.
fn key(name: &str, labels: &[(&str, &str)]) -> Series {
    let mut labels: Vec<(String, String)> = labels
        .iter()
        .map(|(label, value)| ((*label).to_owned(), (*value).to_owned()))
        .collect();
    labels.sort();
    Series {
        name: name.to_owned(),
        labels,
    }
}

/// A newly registered series' value: zero, whatever the kind.
fn empty(kind: Kind, buckets: &'static [f64]) -> Value {
    match kind {
        Kind::Counter => Value::Counter(0),
        Kind::Histogram => Value::Histogram(Histogram::new(buckets)),
        Kind::Gauge => Value::Gauge(0.0),
    }
}

/// A status as its three digits, on the stack.
///
/// A label value is only borrowed for the length of the call that writes it, so
/// formatting one through `to_string` would allocate and free once per response
/// on the request path for three bytes that are a division away. A status
/// outside HTTP's own range is `000`, which is a value no backend will mistake
/// for a status and which keeps this total.
fn digits(status: u16) -> [u8; 3] {
    let status = if (100..1000).contains(&status) {
        status
    } else {
        0
    };
    let digit = |place: u16| u8::try_from(u16::from(b'0') + (status / place) % 10).unwrap_or(b'0');
    [digit(100), digit(10), digit(1)]
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Barrier};
    use std::thread;
    use std::time::Duration;

    use nvs_config::tree::Metrics;
    use nvs_config::{Config, Exporter, Setting};

    use super::{
        Kind, LATENCY_BUCKETS, PAUSE_BUCKETS, Refused, Registry, Value, count_request, every_core,
        meter_this_core,
    };

    /// A merged tree whose `[metrics]` block writes an exporter and a bound.
    fn configured(exporter: Setting, max_series: Option<u64>) -> Config {
        Config {
            metrics: Some(Metrics {
                exporter: Some(exporter),
                max_series,
                ..Metrics::default()
            }),
            ..Config::default()
        }
    }

    /// § 6: `exporter = false`, an unwritten key and a missing block each build
    /// no registry at all, and a written bound is what § 7 holds a core to.
    #[test]
    fn no_exporter_builds_no_registry_and_a_written_bound_is_the_one_in_force() {
        assert!(Registry::of(&Config::default()).is_none());
        assert!(Registry::of(&configured(Setting::Bool(false), Some(64))).is_none());
        assert!(
            Registry::of(&Config {
                metrics: Some(Metrics::default()),
                ..Config::default()
            })
            .is_none()
        );

        let written = Registry::of(&configured(Setting::Text("otlp".to_owned()), Some(64)))
            .expect("an otlp exporter builds a registry");
        assert_eq!(written.exporter(), Some(Exporter::Otlp));
        assert_eq!(written.max_series(), 64);

        // § 6's own default, for a block that names an exporter and no bound.
        let defaulted = Registry::of(&configured(Setting::Text("otlp".to_owned()), None))
            .expect("an otlp exporter builds a registry");
        assert_eq!(defaulted.max_series(), 10_000);
    }

    /// § 7 on both sides: the last series the bound accepts is recorded, the
    /// first one past it is refused and counted, and **every existing series
    /// keeps answering** — which is the half that separates this from eviction.
    #[test]
    fn a_new_series_past_max_series_is_refused_and_the_old_ones_survive() {
        // Three seeded members, so a bound of four leaves room for exactly one.
        let mut registry = Registry::new(4);
        assert_eq!(registry.len(), 3);

        registry
            .increment(
                "nvs_requests_total",
                2,
                &[("method", "GET"), ("status", "200"), ("route", "home")],
            )
            .expect("the first request series fits under the bound");
        assert_eq!(registry.len(), 4);

        let refused = registry
            .increment(
                "nvs_requests_total",
                1,
                &[("method", "GET"), ("status", "500"), ("route", "home")],
            )
            .expect_err("a second combination is past the bound");
        assert_eq!(
            refused,
            Refused::Cardinality {
                name: "nvs_requests_total".to_owned(),
                max_series: 4,
            }
        );
        assert_eq!(refused.metric(), "nvs_requests_total");
        assert_eq!(registry.refusals().get("nvs_requests_total"), Some(&1));

        // Refused, not evicted: the accepted series is untouched and still
        // accepts writes, and the refused one was never created.
        assert_eq!(registry.len(), 4);
        registry
            .increment(
                "nvs_requests_total",
                3,
                &[("method", "GET"), ("status", "200"), ("route", "home")],
            )
            .expect("an existing series is never refused");
        assert_eq!(
            registry.read(
                "nvs_requests_total",
                &[("method", "GET"), ("status", "200"), ("route", "home")]
            ),
            Some(&Value::Counter(5))
        );
        assert!(
            registry
                .read(
                    "nvs_requests_total",
                    &[("method", "GET"), ("status", "500"), ("route", "home")]
                )
                .is_none()
        );
    }

    /// § 3: a name is fixed to one kind on first use, and this module fixes its
    /// label names with it. Neither mismatch records anything.
    #[test]
    fn a_name_is_fixed_to_one_kind_and_one_label_set() {
        let mut registry = Registry::new(64);

        let kind = registry
            .observe(
                "nvs_requests_total",
                1.0,
                &[("method", "GET"), ("status", "200"), ("route", "")],
            )
            .expect_err("a counter cannot be observed into");
        assert_eq!(
            kind,
            Refused::Kind {
                name: "nvs_requests_total".to_owned(),
                fixed: Kind::Counter,
                asked: Kind::Histogram,
            }
        );

        let labels = registry
            .increment("nvs_requests_total", 1, &[("method", "GET")])
            .expect_err("a member of the family carries all of its labels");
        assert!(matches!(labels, Refused::Labels { .. }));
        assert!(labels.to_string().contains("nvs_requests_total"));

        // An undeclared name is fixed by its own first use, and the same two
        // rules then hold over it.
        registry
            .gauge("queue_depth", 7.0, &[("queue", "mail")])
            .expect("a name nothing has used is fixed by this call");
        assert_eq!(registry.kind("queue_depth"), Some(Kind::Gauge));
        assert!(
            registry
                .increment("queue_depth", 1, &[("queue", "mail")])
                .is_err()
        );
        assert_eq!(
            registry.read("queue_depth", &[("queue", "mail")]),
            Some(&Value::Gauge(7.0))
        );
    }

    /// A label written in two orders is one series and not two, which is a
    /// cardinality question rather than a tidiness one.
    #[test]
    fn label_order_does_not_make_a_second_series() {
        let mut registry = Registry::new(64);
        registry
            .gauge("nvs_memory_bytes", 128.0, &[("scope", "request")])
            .expect("the seeded family accepts its own label");
        registry
            .observe("q", 0.2, &[("a", "1"), ("b", "2")])
            .expect("a new histogram");
        registry
            .observe("q", 0.4, &[("b", "2"), ("a", "1")])
            .expect("the same series, written the other way round");

        let Some(Value::Histogram(histogram)) = registry.read("q", &[("a", "1"), ("b", "2")])
        else {
            panic!("one histogram, under either spelling");
        };
        assert_eq!(histogram.count, 2);
        assert!((histogram.sum - 0.6).abs() < f64::EPSILON * 8.0);
        assert_eq!(histogram.bounds, LATENCY_BUCKETS);
    }

    /// `rule:observability/default-series`'s two request series are recorded together, carrying
    /// `rule:observability/route-label-is-the-declared-name`'s declared name — and a request that matched nothing carries the label
    /// empty rather than not at all.
    #[test]
    fn a_request_records_both_series_under_the_routes_declared_name() {
        let mut registry = Registry::new(64);
        registry.request("GET", 200, Some("products.show"), Duration::from_millis(30));
        registry.request("GET", 200, Some("products.show"), Duration::from_millis(30));
        registry.request("POST", 404, None, Duration::from_millis(2));

        let matched = [
            ("method", "GET"),
            ("status", "200"),
            ("route", "products.show"),
        ];
        assert_eq!(
            registry.read("nvs_requests_total", &matched),
            Some(&Value::Counter(2))
        );
        let Some(Value::Histogram(histogram)) =
            registry.read("nvs_request_duration_seconds", &matched)
        else {
            panic!("the duration series exists beside the counter");
        };
        assert_eq!(histogram.count, 2);
        // 30ms is past 0.025 and inside 0.05, the fourth boundary.
        assert_eq!(histogram.counts[3], 2);

        let unmatched = [("method", "POST"), ("status", "404"), ("route", "")];
        assert_eq!(
            registry.read("nvs_requests_total", &unmatched),
            Some(&Value::Counter(1))
        );
        assert_eq!(registry.len(), 3 + 4);
    }

    /// A pause histogram is bucketed where a pause lives: measured against the
    /// request table every one of these would land in one bucket and the series
    /// would report only what its count already said.
    #[test]
    fn a_pause_is_bucketed_below_where_a_request_is() {
        let mut registry = Registry::new(64);
        for pause in [0.000_02_f64, 0.000_2, 0.002] {
            registry
                .observe("nvs_gc_pause_seconds", pause, &[])
                .expect("the seeded family carries no labels");
        }
        let Some(Value::Histogram(histogram)) = registry.read("nvs_gc_pause_seconds", &[]) else {
            panic!("the seeded histogram");
        };
        assert_eq!(histogram.bounds, PAUSE_BUCKETS);
        assert_eq!(histogram.count, 3);
        assert_eq!(
            histogram.counts.iter().filter(|count| **count > 0).count(),
            3
        );
        assert_eq!(histogram.counts.len(), PAUSE_BUCKETS.len() + 1);
        assert_eq!(
            histogram.counts[PAUSE_BUCKETS.len()],
            0,
            "nothing reaches the +Inf slot"
        );
        assert!(PAUSE_BUCKETS[0] < LATENCY_BUCKETS[0]);
    }

    /// A gather reaches a core it is not running on, and stops reaching one
    /// that ended.
    ///
    /// The two halves are one test because they are one roster: what proves the
    /// gather works is two cores' distinct series arriving on a third thread,
    /// and what proves the roster does not grow with threads started is the same
    /// two being gone once those threads are joined.
    ///
    /// **Asserted by label and never by count**, because every other test in
    /// this binary that serves a request is a core in the same roster, and a
    /// length is therefore whatever the harness happened to be running
    /// alongside. The barrier is what holds both cores alive across the gather:
    /// a thread that had already exited would be indistinguishable from one the
    /// roster never reached.
    #[test]
    fn a_gather_reaches_every_serving_core_and_drops_one_that_ended() {
        const ROUTES: [&str; 2] = ["gather.one", "gather.two"];
        fn counted(route: &str) -> [(&str, &str); 3] {
            [("method", "GET"), ("status", "200"), ("route", route)]
        }

        let gate = Arc::new(Barrier::new(ROUTES.len() + 1));
        let cores: Vec<_> = ROUTES
            .into_iter()
            .map(|route| {
                let gate = Arc::clone(&gate);
                thread::spawn(move || {
                    meter_this_core(&configured(Setting::Text("prometheus".to_owned()), None));
                    count_request("GET", 200, Some(route), Duration::from_millis(5));
                    gate.wait();
                    gate.wait();
                })
            })
            .collect();

        gate.wait();
        let serving = every_core();
        for route in ROUTES {
            let labels = counted(route);
            assert_eq!(
                serving
                    .iter()
                    .filter_map(|registry| registry.read("nvs_requests_total", &labels))
                    .collect::<Vec<_>>(),
                vec![&Value::Counter(1)],
                "a core's series did not reach a gather running on another"
            );
        }

        gate.wait();
        for core in cores {
            core.join().expect("a metering thread panicked");
        }
        let ended = every_core();
        for route in ROUTES {
            let labels = counted(route);
            assert!(
                ended
                    .iter()
                    .all(|registry| registry.read("nvs_requests_total", &labels).is_none()),
                "a core that ended was still gathered from"
            );
        }
    }
}
