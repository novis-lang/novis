//! This crate's name for the per-core registry, which lives in
//! [`nvs_runtime::metrics`] — and the declared series a configured exporter
//! promises.
//!
//! **The accumulator moved down and the exporter did not.**
//! `rule:observability/the-exporter-is-a-feature-and-core-metrics-is-not` puts
//! `Core\Metrics` in every build behind a feature-gated exporter, so the
//! registry the class writes into and the scrape reads has to sit under both
//! `nvs-stdlib` and this crate, neither of which may depend on the other. It
//! sits in `nvs-runtime`, where a `Core` member reaches it with nothing but a
//! `Ctx` in hand; that module's own doc owns the shape, the bound and the
//! per-core locking. What stays here is the reader —
//! [`crate::prometheus`]'s exposition format and its listener, which is what
//! this crate's `exporter` feature removes.
//!
//! The re-export is so that a caller in this crate names one module rather than
//! two for one subject: `crate::metrics::count_request` at the door,
//! `crate::metrics::every_core` at the scrape.

pub use nvs_runtime::metrics::*;

#[cfg(test)]
mod tests {
    use nvs_config::tree::Metrics;
    use nvs_config::{Config, Exporter, Setting};

    use super::{DEFAULT, Kind, Registry, Value};

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

    /// `rule:observability/default-series`: the eleven series exist the moment an exporter is configured,
    /// each fixed to the kind and the labels that rule gives it.
    ///
    /// Asserted by **counting** as well as by name: a roster that grew a
    /// twelfth entry or lost one would otherwise pass every row below.
    ///
    /// Here rather than beside the registry because the claim is this crate's:
    /// what that rule promises is that a *configured exporter* has these eleven to
    /// report, and the exporter is what this crate is.
    #[test]
    fn the_default_metric_series_are_exported() {
        let registry = Registry::of(&configured(Setting::Text("prometheus".to_owned()), None))
            .expect("a written exporter builds a registry");
        assert_eq!(registry.exporter(), Some(Exporter::Prometheus));
        assert_eq!(DEFAULT.len(), 11);

        let expected: [(&str, Kind, &[&str]); 11] = [
            (
                "nvs_requests_total",
                Kind::Counter,
                &["method", "route", "status"],
            ),
            (
                "nvs_request_duration_seconds",
                Kind::Histogram,
                &["method", "route", "status"],
            ),
            (
                "nvs_db_query_duration_seconds",
                Kind::Histogram,
                &["connection", "operation"],
            ),
            ("nvs_gc_pause_seconds", Kind::Histogram, &[]),
            ("nvs_spawn_duration_seconds", Kind::Histogram, &["kind"]),
            ("nvs_tasks_in_flight", Kind::Gauge, &[]),
            ("nvs_deferred_trees", Kind::Gauge, &[]),
            ("nvs_memory_bytes", Kind::Gauge, &["scope"]),
            ("nvs_request_memory_peak_bytes", Kind::Histogram, &["route"]),
            (
                "nvs_schedule_runs_total",
                Kind::Counter,
                &["name", "outcome"],
            ),
            ("nvs_log_handler_dropped_total", Kind::Counter, &[]),
        ];
        for (name, kind, labels) in expected {
            assert_eq!(registry.kind(name), Some(kind), "{name}");
            let fixed: Vec<&str> = registry
                .labels(name)
                .unwrap_or_else(|| panic!("{name} is a declared family"))
                .iter()
                .map(String::as_str)
                .collect();
            assert_eq!(fixed, labels, "{name}");
        }

        // The four families with no labels have exactly one member each, and it
        // reads as zero before anything writes one.
        let unlabelled: Vec<&str> = registry
            .series()
            .map(|(series, _)| series.name.as_str())
            .collect();
        assert_eq!(
            unlabelled,
            [
                "nvs_deferred_trees",
                "nvs_gc_pause_seconds",
                "nvs_log_handler_dropped_total",
                "nvs_tasks_in_flight"
            ]
        );
        assert_eq!(
            registry.read("nvs_tasks_in_flight", &[]),
            Some(&Value::Gauge(0.0))
        );
    }
}
