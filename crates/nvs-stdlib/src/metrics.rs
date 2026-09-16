//! `Core\Metrics` — `rule:observability/metrics-three-members`'s three verbs
//! for three kinds, over the per-core registry `nvs_runtime::metrics` owns.
//!
//! `increment` for a counter, `observe` for a histogram, `gauge` for a
//! point-in-time value. Three members rather than one `record` with a kind
//! argument, because the kind is a property of the **series** and not of the
//! call: a name is fixed to one kind on first use, and a call that disagrees is
//! a mistake rather than a mode.
//!
//! # Where the series live, and why not here
//!
//! Nothing in this module holds a counter. The registry is
//! [`nvs_runtime::metrics`]'s, one per core, and this module is three calls
//! onto [`nvs_runtime::metrics::record`] — which is where the bound, the
//! per-core locking and the merge a scrape performs are all owned.
//!
//! That crate rather than `nvs-server`, where the exporter is, because
//! `rule:observability/the-exporter-is-a-feature-and-core-metrics-is-not` makes
//! these three members Tier 0 in **every** build while the scrape is a Cargo
//! feature. `nvs-stdlib` cannot depend on `nvs-server` and `nvs-server` cannot
//! depend on this crate, so the one registry both write to sits under both. A
//! build with no exporter still accumulates, and a `nvs run` that calls these
//! members builds a registry on its own thread that nothing will ever read —
//! bounded, tiny, and identical in behaviour to the build that does export,
//! which is the whole of what that rule buys.
//!
//! **No member answers a value back.**
//! `rule:observability/a-registry-is-per-core-and-nothing-reads-it` is why: the
//! values are per-core aggregates merged at a scrape, so a program that read
//! one would be reading an approximation of its own core, and a program that
//! would be incorrect if the read answered nothing is using the wrong tier.
//! All three return `void`.
//!
//! # A name is fixed to one kind, and a mismatch names both sites
//!
//! `rule:observability/metrics-three-members` makes the kind a property of the
//! series, so the second call to disagree with the first throws a `LogicError`
//! — and it names **where the name was fixed** as well as where the disagreement
//! is, because the line that decided the kind is the one a reader has to go and
//! look at. That is what puts these three members on
//! [`crate::registry::SOURCE_MEMBERS`]: the call site arrives as a constant of
//! the call, so each of them has `args: [4]` for three declared arguments.
//!
//! A **cardinality** refusal is not a throw. It is the no-op
//! `rule:observability/past-max-series-a-new-series-is-refused` defines, counted
//! against the metric's name where an operator reads it, because losing a label
//! combination is the deployment's problem and not the program's — the program
//! did nothing wrong and has nothing to handle.
//!
//! # What it spends
//!
//! Per call: one borrow of the `labels` array and one `(String, String)` per
//! label written, both freed before the member returns. Per *name* a program
//! writes: the series itself, and one `String` holding the `file:line` that
//! fixed it. Nothing is charged to a request, and nothing grows with requests
//! served — `rule:programs/memory-priority`'s O(in-flight) reading of the same
//! cost `nvs_runtime::metrics` states in full.

use nvs_runtime::metrics::{Refused, Written};
use nvs_runtime::{Fault, ThrownClass, Value};

use crate::registry::{
    Const, CoreClass, CoreMethod, CoreOption, CoreTy, ErrorDoc, MethodDoc, ParamDoc, Qual,
};

/// The class's fully-qualified name.
pub(crate) const NAME: &str = r"Core\Metrics";

/// `rule:observability/metrics-three-members`'s `[a-z][a-z0-9_]*`, and the one
/// home of it.
///
/// It answers about a **written** name alone. `nvs_types::intrinsics` reads a
/// literal argument through here and refuses one outside the grammar where it
/// was written; `nvs_runtime::metrics::Registry` fixes whatever it is handed,
/// because a metric write may not fail the request that made it — the same
/// reading `rule:observability/past-max-series-a-new-series-is-refused` gives a
/// series past the bound, where the write is dropped and the response is
/// untouched. So this is reached while checking and never on the request path.
///
/// # Errors
///
/// The sentence a diagnostic quotes under the literal, naming what the name
/// holds: empty, a first character that is not `a`-`z`, or a later one outside
/// `[a-z0-9_]`.
pub fn validate_name(name: &str) -> Result<(), String> {
    let mut rest = name.chars();
    let Some(first) = rest.next() else {
        return Err("a series name is `[a-z][a-z0-9_]*`, and this one is empty".to_owned());
    };
    if !first.is_ascii_lowercase() {
        return Err(format!(
            "a series name starts with `a`-`z`, and `{name}` starts with `{first}`"
        ));
    }
    if let Some(stray) = rest.find(|c| !c.is_ascii_lowercase() && !c.is_ascii_digit() && *c != '_')
    {
        return Err(format!(
            "a series name is `[a-z][a-z0-9_]*`, and `{name}` carries `{stray}`"
        ));
    }
    Ok(())
}

/// The `{labels?}` every one of the three members carries.
///
/// The **value** position is a sink with no launderer
/// (`rule:security/metric-label-refuses-tainted`): a user id, a tenant name or
/// an error message in a label is unbounded cardinality, which is the one way a
/// program can make a registry cost what § 7's bound exists to stop. What is
/// admitted instead is an enum case, an `as`-converted scalar or a route name.
///
/// `$name` is a sink for the same reason and a stronger one: a label value
/// multiplies a series, while a name *is* one, and the grammar a literal name is
/// checked against at the call site has no run-time equivalent for text a
/// request wrote.
const LABELS: CoreOption = CoreOption {
    name: "labels",
    ty: CoreTy::Array(&CoreTy::Text(Qual::Sink)),
    default: Const::EmptyArray,
};

/// `increment`'s bag: how much to add, and the labels.
const INCREMENT_OPTIONS: &[CoreOption] = &[
    CoreOption {
        name: "by",
        // One, because a counter's overwhelmingly common call is "one more of
        // these happened" and a required argument for it would be noise at
        // every call site. A `uint`, so a counter cannot be made to go
        // backwards — which every backend reads as a process restart.
        ty: CoreTy::Uint,
        default: Const::Uint(1),
    },
    LABELS,
];

/// `observe`'s and `gauge`'s bag: the labels alone, the value being positional.
const VALUE_OPTIONS: &[CoreOption] = &[LABELS];

/// `rule:observability/metrics-three-members`'s three members, and nothing else
/// — there is no reader.
pub(crate) const CLASS: CoreClass = CoreClass {
    name: NAME,
    methods: &[
        CoreMethod {
            name: "increment",
            names: &["name"],
            params: &[CoreTy::Text(Qual::Sink), CoreTy::Options(INCREMENT_OPTIONS)],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_metrics_increment",
            doc: Some(&INCREMENT_DOC),
        },
        CoreMethod {
            name: "observe",
            names: &["name", "value"],
            params: &[
                CoreTy::Text(Qual::Sink),
                CoreTy::Float,
                CoreTy::Options(VALUE_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_metrics_observe",
            doc: Some(&OBSERVE_DOC),
        },
        CoreMethod {
            name: "gauge",
            names: &["name", "value"],
            params: &[
                CoreTy::Text(Qual::Sink),
                CoreTy::Float,
                CoreTy::Options(VALUE_OPTIONS),
            ],
            defaults: &[],
            return_ty: CoreTy::Void,
            symbol: "nvs_core_metrics_gauge",
            doc: Some(&GAUGE_DOC),
        },
    ],
    instance: &[],
    slots: &[],
    constants: &[],
};

/// What every one of the three cards says about `$name`.
const NAME_DESC: &str = "The series name, `[a-z][a-z0-9_]*`. A literal outside that grammar is a \
                         compile error, and a `tainted` name is refused wherever it was written.";

/// What every one of the three cards says about `labels`.
const LABELS_DESC: &str = "The label values this series is written under, keyed by label name. A \
                           `tainted` or `secret` value is refused: label by an enum case, an \
                           `as`-converted scalar or a route name.";

/// What every one of the three cards says about a kind mismatch.
const MISMATCH_DESC: &str = "The name is already fixed to another kind or to another set of label \
                             names. The message gives both the call that fixed it and this one. \
                             A new series past the core's `max_series` is **not** an error: it is \
                             dropped and counted, and the call answers normally.";

/// `Core\Metrics::increment`'s reference card — `rule:core-api/reference-card`.
const INCREMENT_DOC: MethodDoc = MethodDoc {
    short: "Adds to a counter, fixing `$name` to a counter on its first use — the replacement for \
            every hand-rolled `$GLOBALS` tally and for a `statsd` client's `increment`.",
    params: &[
        ParamDoc {
            name: "name",
            desc: NAME_DESC,
            shape: &[],
        },
        ParamDoc {
            name: "by",
            desc: "How much to add; one by default, and never negative, since a counter that went \
                   backwards would read as a process restart.",
            shape: &[],
        },
        ParamDoc {
            name: "labels",
            desc: LABELS_DESC,
            shape: &[],
        },
    ],
    ret: "Nothing. No program can read a metric back: the values are per-core aggregates a scrape \
          merges, and the only reader is outside every request.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: MISMATCH_DESC,
    }],
};

/// `Core\Metrics::observe`'s reference card — `rule:core-api/reference-card`.
const OBSERVE_DOC: MethodDoc = MethodDoc {
    short: "Records one observation into a histogram, fixing `$name` to a histogram on its first \
            use — a duration, a payload size, a queue depth at the moment it was measured.",
    params: &[
        ParamDoc {
            name: "name",
            desc: NAME_DESC,
            shape: &[],
        },
        ParamDoc {
            name: "value",
            desc: "The observation, in the series' own unit — seconds for a `_seconds` name, bytes \
                   for a `_bytes` one.",
            shape: &[],
        },
        ParamDoc {
            name: "labels",
            desc: LABELS_DESC,
            shape: &[],
        },
    ],
    ret: "Nothing, as `increment`.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: MISMATCH_DESC,
    }],
};

/// `Core\Metrics::gauge`'s reference card — `rule:core-api/reference-card`.
const GAUGE_DOC: MethodDoc = MethodDoc {
    short: "Sets a gauge to `$value`, fixing `$name` to a gauge on its first use — a level that \
            goes up and down, such as the number of items waiting or the bytes a pool holds.",
    params: &[
        ParamDoc {
            name: "name",
            desc: NAME_DESC,
            shape: &[],
        },
        ParamDoc {
            name: "value",
            desc: "What the gauge now reads. The last write before a scrape is the one reported.",
            shape: &[],
        },
        ParamDoc {
            name: "labels",
            desc: LABELS_DESC,
            shape: &[],
        },
    ],
    ret: "Nothing, as `increment`.",
    errors: &[ErrorDoc {
        error: "LogicError",
        desc: MISMATCH_DESC,
    }],
};

/// The address of one of *this* module's symbols, or `None` for a symbol that
/// belongs to another domain. See [`crate::symbols`].
pub(crate) fn address(symbol: &str) -> Option<*const u8> {
    Some(match symbol {
        "nvs_core_metrics_increment" => (nvs_core_metrics_increment as *const ()).cast(),
        "nvs_core_metrics_observe" => (nvs_core_metrics_observe as *const ()).cast(),
        "nvs_core_metrics_gauge" => (nvs_core_metrics_gauge as *const ()).cast(),
        _ => return None,
    })
}

nvs_runtime::nvs_helper! {
    /// `Core\Metrics::increment(string $name, {by?: uint, labels?: array<string, string>}): void`
    /// — `rule:observability/metrics-three-members`.
    ///
    /// `args: [4]` for three declared arguments: this member is on
    /// [`crate::registry::SOURCE_MEMBERS`], so argument 0 is the call site.
    fn nvs_core_metrics_increment(_ctx, args: [4]) {
        let by = args[2].as_uint().unwrap_or(1);
        write(args, "increment", Written::Increment(by))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Metrics::observe(string $name, float $value, {labels?: array<string, string>}): void`
    /// — `rule:observability/metrics-three-members`.
    fn nvs_core_metrics_observe(_ctx, args: [4]) {
        let value = args[2].as_float().unwrap_or_default();
        write(args, "observe", Written::Observe(value))
    }
}

nvs_runtime::nvs_helper! {
    /// `Core\Metrics::gauge(string $name, float $value, {labels?: array<string, string>}): void`
    /// — `rule:observability/metrics-three-members`.
    fn nvs_core_metrics_gauge(_ctx, args: [4]) {
        let value = args[2].as_float().unwrap_or_default();
        write(args, "gauge", Written::Gauge(value))
    }
}

/// The three members' shared body: read the call site and the labels, write, and
/// turn the one refusal that is the caller's mistake into a throw.
///
/// One body because the three differ in [`Written`] alone — which is
/// `rule:observability/metrics-three-members`'s own claim about them, kept true
/// here rather than restated in three places that could drift.
fn write(args: &[Value], member: &str, written: Written) -> Result<Value, Fault> {
    #[expect(
        unsafe_code,
        reason = "the carrier came out of a `SourceConst` the compiled unit baked into its own data section, which outlives every request served from it"
    )]
    let site = unsafe { nvs_runtime::source::of_operand(args[0]) }
        .map(|source| nvs_runtime::source::location(&source));
    let name = name_of(&args[1], member)?;
    let labels = labels_of(args[3]);
    let written_labels: Vec<(&str, &str)> = labels
        .iter()
        .map(|(label, value)| (label.as_str(), value.as_str()))
        .collect();
    match nvs_runtime::metrics::record(site.as_deref(), name, written, &written_labels) {
        // `rule:observability/past-max-series-a-new-series-is-refused`: a series
        // lost to the bound is the deployment's problem, counted where an
        // operator reads it, and the call answers as though it had been
        // recorded. A program has nothing to handle here and no way to act on
        // it.
        Ok(()) | Err(Refused::Cardinality { .. }) => Ok(Value::null()),
        Err(refusal) => Err(mismatch(member, name, &refusal, site.as_deref())),
    }
}

/// A kind or label mismatch as the `LogicError` § 3 asks for, naming the call
/// that fixed the name as well as the one that disagreed with it.
///
/// `LogicError` rather than a `RuntimeError`: which kind a name is is decided
/// by the program and knowable from reading it, so a mismatch is a bug in the
/// program rather than a condition the deployment produced.
fn mismatch(member: &str, name: &str, refusal: &Refused, here: Option<&str>) -> Fault {
    let fixed = nvs_runtime::metrics::fixed_at(name).map_or_else(
        || "by a series the runtime itself declares".to_owned(),
        |site| format!("at {site}"),
    );
    let here = here.map_or_else(
        || "in a callable reference, which carries no call site".to_owned(),
        |site| format!("at {site}"),
    );
    Fault::thrown_as(
        ThrownClass::Logic,
        format!(
            "a metric name is fixed to one kind on first use, and this call disagrees with it: \
             {refusal} — fixed {fixed}, written {here} by `Core\\Metrics::{member}`"
        ),
    )
}

/// The `$name` argument as text.
///
/// A `Fault::fatal` rather than a throw, and unreachable from source: the row's
/// parameter is a `CoreTy::Text`, so `E0401` refuses anything else at the call.
fn name_of<'a>(value: &'a Value, member: &str) -> Result<&'a str, Fault> {
    value.as_text().ok_or_else(|| {
        // Unreachable from source: the row's parameter is a `CoreTy::Text`, so
        // `E0401` refuses anything that is not a `string` at the call.
        Fault::fatal(format!(
            "a metric name must be a `string`, and this one arrived as tag {} at \
             `Core\\Metrics::{member}`",
            value.tag_byte()
        ))
    })
}

/// The `labels` bag as the pairs the registry keys a series by.
///
/// Owned pairs rather than borrows into the array, because the registry takes
/// `&str` and the values behind an `NvsStr` are not `&str` without the handle
/// staying alive — one allocation per label written, freed before the member
/// returns.
///
/// A value that is not text is skipped rather than refused: the row's element
/// type is a `CoreTy::Text`, so `E0401` has already refused anything else at the
/// call, and a bag arriving any other way is the engine's bug and not a
/// program's to catch.
fn labels_of(value: Value) -> Vec<(String, String)> {
    let Some(pointer) = value.array_ptr() else {
        return Vec::new();
    };
    let array = crate::arr::borrowed(pointer);
    let mut written = Vec::with_capacity(array.count());
    for key in array.keys() {
        let Some(held) = array.get(&key) else {
            continue;
        };
        let Some(text) = held.as_text() else {
            continue;
        };
        written.push((String::from_utf8_lossy(&key).into_owned(), text.to_owned()));
    }
    written
}

#[cfg(test)]
mod tests {
    use nvs_runtime::metrics::{self, Refused, Written};
    use nvs_runtime::{Fault, ThrownClass};

    use super::{CLASS, NAME, mismatch};
    use crate::registry::{CoreTy, Qual};

    /// `rule:observability/metrics-three-members`: three verbs, three kinds,
    /// and no fourth member that reads one back
    /// (`rule:observability/a-registry-is-per-core-and-nothing-reads-it`).
    #[test]
    fn core_metrics_is_a_registered_class() {
        assert_eq!(NAME, r"Core\Metrics");
        assert!(
            crate::registry::CLASSES
                .iter()
                .any(|class| class.name == NAME),
            "`Core\\Metrics` is registered"
        );
        let members: Vec<&str> = CLASS.methods.iter().map(|method| method.name).collect();
        assert_eq!(members, ["increment", "observe", "gauge"]);
        assert!(CLASS.instance.is_empty());
        for method in CLASS.methods {
            assert!(
                matches!(method.return_ty, CoreTy::Void),
                "{} answers nothing back",
                method.name
            );
            assert!(
                matches!(method.params[0], CoreTy::Text(Qual::Sink)),
                "{}'s name is a sink",
                method.name
            );
        }
    }

    /// `rule:observability/metrics-three-members`: the kind belongs to the
    /// series, so a second verb disagreeing with the first is a `LogicError`
    /// naming **both** the call that fixed the name and the call that
    /// disagreed, and the disagreeing call records nothing.
    #[test]
    fn a_metrics_name_used_as_a_gauge_then_incremented_throws_naming_both_sites() {
        metrics::record(
            Some("queue.nvs:12"),
            "queue_depth",
            Written::Gauge(7.0),
            &[],
        )
        .expect("a gauge write fixes the name to a gauge");
        let refusal = metrics::record(
            Some("queue.nvs:31"),
            "queue_depth",
            Written::Increment(1),
            &[],
        )
        .expect_err("a counter write disagrees with the gauge that fixed the name");
        assert!(
            matches!(refusal, Refused::Kind { .. }),
            "a verb disagreeing about the kind is a kind refusal, not {refusal:?}"
        );

        match mismatch("increment", "queue_depth", &refusal, Some("queue.nvs:31")) {
            Fault::Thrown(class, message) => {
                assert_eq!(class, ThrownClass::Logic);
                assert!(message.contains(&refusal.to_string()), "{message}");
                assert!(message.contains("fixed at queue.nvs:12"), "{message}");
                assert!(message.contains("written at queue.nvs:31"), "{message}");
                assert!(message.contains(r"`Core\Metrics::increment`"), "{message}");
            }
            other => panic!("a kind mismatch is a thrown `LogicError`, and this is {other:?}"),
        }

        let registry = metrics::on_this_core().expect("the first write built this thread's one");
        assert_eq!(
            registry.read("queue_depth", &[]),
            Some(&metrics::Value::Gauge(7.0)),
            "the refused call left the gauge as the call that fixed it wrote it"
        );
    }

    /// `rule:observability/the-exporter-is-a-feature-and-core-metrics-is-not`:
    /// this crate cannot depend on `nvs-server`, so this test binary *is* a
    /// build with no exporter — and all three verbs still accumulate into the
    /// per-core registry, which is the whole behavioural difference that rule
    /// allows between builds.
    #[test]
    fn core_metrics_accumulates_with_the_exporter_feature_off() {
        let mail = [("queue", "mail")];
        metrics::record(
            Some("jobs.nvs:8"),
            "jobs_run_total",
            Written::Increment(2),
            &mail,
        )
        .expect("a counter write is recorded");
        metrics::record(
            Some("jobs.nvs:8"),
            "jobs_run_total",
            Written::Increment(1),
            &mail,
        )
        .expect("a second write accumulates onto the first");
        metrics::record(
            Some("jobs.nvs:9"),
            "job_seconds",
            Written::Observe(0.25),
            &[],
        )
        .expect("a histogram write is recorded");
        metrics::record(
            Some("jobs.nvs:10"),
            "workers_idle",
            Written::Gauge(4.0),
            &[],
        )
        .expect("a gauge write is recorded");

        let registry = metrics::on_this_core().expect("a write builds this thread's registry");
        assert!(
            registry.exporter().is_none(),
            "nothing here exports, and the series accumulate regardless"
        );
        assert_eq!(
            registry.read("jobs_run_total", &mail),
            Some(&metrics::Value::Counter(3))
        );
        assert_eq!(
            registry.read("workers_idle", &[]),
            Some(&metrics::Value::Gauge(4.0))
        );
        match registry.read("job_seconds", &[]) {
            Some(metrics::Value::Histogram(histogram)) => {
                assert_eq!(histogram.count, 1);
                assert!((histogram.sum - 0.25).abs() < 1e-9, "{}", histogram.sum);
            }
            other => panic!("`observe` fixes the name to a histogram, and this read {other:?}"),
        }
    }
}
