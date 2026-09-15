# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run.** Stage 1 is the
carried floor, stage 2 is done ([0186](../decisions/0186.md) is the only ADR number this goal opens),
and stages 3 to 9 are complete. Stage 10, the metrics export, is the earliest red stage: its first
check is green and its second is the next group. Nothing is blocked.

**A serving core counts every response it writes.** `crates/nvs-server/src/metrics.rs:671`'s
`meter_this_core` gives the core the registry `[metrics]` asks for — a thread-local, because that is
what "per core" is when one core runs one accept loop per listening socket and they share its
counters — and `crates/nvs-server/src/serve.rs`'s service closure reaches it through
`count_request` at all four of its answers, including the forwarded refusal, the ceiling's `503` and
a preflight. `rule:observability/route-label-is-the-declared-name`'s label is taken off the reply's
own carrier before `Isolate::start` moves it (`nvs_host::Isolate::answering_request`), which closes
`route.rs`'s old gap 2.

**The encoder is written and has no gatherer.** `crates/nvs-server/src/prometheus.rs:63`'s `scrape`
takes every core's registry, adds them up and writes the Prometheus text exposition format — by
hand against the specification, which is `rule:observability/an-exporter-brings-no-second-scheduler-and-no-second-client`'s
finding that neither candidate crate is takeable. What it has no caller for is the slice below:
nothing binds `[metrics] listen` and nothing carries another core's `on_this_core()` copy across.

## Next group

**Stage 10: the exporter's listener** — one file set: `crates/nvs-cli/src/serve.rs` for the boot and
the per-core arming, with `crates/nvs-server/src/metrics.rs` and
`crates/nvs-server/src/prometheus.rs` for the two halves it joins.

- [ ] **A scrape reaches every core's registry** — `crates/nvs-server/src/metrics.rs:700`'s
      `on_this_core` is the per-core copy, and `crates/nvs-server/src/prometheus.rs:63`'s `scrape`
      is the consumer that wants all of them. The goal's stage 10 prose decides the shape: a message
      over the cross-core channel, gathered on the scraping core, never a shared store
      (`rule:observability/a-registry-is-per-core-and-nothing-reads-it`). **No such channel was found
      under that name** — `nvs-host` has `Worker` and `wake_at_drain` and nothing else obvious — so
      the first thing this slice owes is to name what it is or to build it.
- [ ] **`serve` answers a scrape at `[metrics] listen`, and a tree whose exporter is `false` binds
      nothing** — the key is `crates/nvs-config/src/tree.rs:1050`, still carrying its `[unread:]`
      marker, and the arming goes beside the per-core listener loop at
      `crates/nvs-cli/src/serve.rs:984`. One more listening socket on this server's own accept loop
      and never a second HTTP server, per the rule above. Tests:
      `serve_answers_a_scrape_at_the_metrics_listen_address`,
      `a_tree_whose_metrics_exporter_is_false_binds_nothing_and_builds_no_registry`, `-p nvs-cli`.
- [ ] **The exporter goes behind a cargo feature, on by default** —
      `rule:observability/the-exporter-is-a-feature-and-core-metrics-is-not`, whose fragment is still
      marked *designed, not yet shipped*. `crates/nvs-server/src/prometheus.rs:1` is the module the
      gate wraps; `crates/nvs-server/src/metrics.rs:1` is the half that is Tier 0 in every build and
      stays ungated. A build without it refuses any `exporter` other than `false` at boot, naming the
      feature.

## Backlog

- No `# HELP` line is written for any family: `Family` (`crates/nvs-server/src/metrics.rs:135`) holds
  no help text, and adding one changes what `rule:observability/default-series`'s roster carries.
- `Core\Metrics`'s three rows are goal `m8-stdlib-depth`'s — `crates/nvs-server/src/metrics.rs`'s
  known gap 2 owns it.
- Stage 11's spans and the OTLP push are still ahead of this stage's `otlp` half.
