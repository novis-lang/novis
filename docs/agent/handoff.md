# Handoff

## State

**Goal 6, M7, stage 6 — the registry landed and the exporter is what is left.** The stage-6
acceptance failure is closed: `the_default_metric_series_are_exported` runs in
`crates/nvs-server/src/metrics.rs`, which is ADR 0076 §§ 5 and 7's per-core registry — § 1's nine
families fixed to their kind and labels before anything writes one, § 7's `max_series` refusing a
*new* series and never evicting an old one, and the two request series carrying § 1's `route` label.
That module's own doc is the authority on all of it; nothing here restates it.

**`[metrics]` resolves in `nvs-config`.** `nvs_config::Metering` (`crates/nvs-config/src/export.rs`)
is the exporter in force plus § 6's own `max_series = 10000` default, beside the refusal that was
already there — `crate::server`'s `Capacity` is the precedent. `exporter = false`, an unwritten key
and a missing block all resolve to `None`, and `Registry::of` then builds nothing at all.

**Nothing scrapes or pushes yet, and that is one gap with two faces.** ADR 0076 § 8's two exporters
are not in this crate's graph, so no core owns a registry and `crates/nvs-server/src/route.rs`'s
`label` still has no caller — its known gap 2 now says so precisely. The decision the next session
owes first: `opentelemetry-otlp` and `metrics-exporter-prometheus` both bring a tokio *runtime*
(`rt`, `net`, `time`), and this crate compiles tokio as `features = ["sync"]` on purpose
(`crates/nvs-server/src/lib.rs:86`). `nvs_host::block_on` drives one connection future per
coroutine; whether it can drive a `tonic` client that spawns its own tasks is the question, and the
cheap answer if it cannot is that the scrape path needs no client at all — a `hyper` server on
`[metrics] listen`, which this crate already has, formats `Registry::series` and pushes nothing.

**`[context] adrs` needs `0076 §§ 3, 4, 5, 8`.** The pack carries §§ 1, 6, 7; § 5 and § 7 are what
the item cited, § 3 fixes a name to one kind, § 4 is the `route` label's whole rule and § 8 is the
next slice — four `sed` slices to fetch what the item's own ADR already governs.

## Next group

**ADR 0076's exporter**, over `crates/nvs-server/Cargo.toml`, `crates/nvs-server/src/metrics.rs`,
`crates/nvs-server/src/serve.rs` and `crates/nvs-server/src/route.rs`. The first decides what the
second can be.

- [ ] **§ 8's two dependencies, feature-gated — and what an executor-less crate can take of them**
      (ADR 0076 §§ 6, 8) — `crates/nvs-server/Cargo.toml:12` for the graph and
      `crates/nvs-server/src/metrics.rs:52` for the paragraph that records the answer. § 6 makes
      the exporter Native and feature-gated, defaulting on in the server distribution and absent
      from an ADR 0048 single-file build. Decide the scrape path and the push path separately: the
      scrape is a `hyper` service over `Registry::series` and needs no new dependency, the push is
      where tokio's runtime arrives. Pre-authorized under the goal's § *Standing decisions*; record
      it in the module doc, not a new ADR.
- [ ] **A core owns one registry, and the door records each response into it** (ADR 0076 §§ 1, 5) —
      `crates/nvs-server/src/serve.rs:916` is where a core's loop starts,
      `crates/nvs-server/src/metrics.rs:371` is the type it would hold and
      `crates/nvs-server/src/metrics.rs:526` is the one call the response path makes, whose third
      argument is `crates/nvs-server/src/route.rs:133`. § 5's "per core" is why it is a `&mut` on
      the core's own loop and not anything shared; a `RefCell` in a thread-local is the shape, since
      every connection on that core is a coroutine on that one thread. Closing this closes route.rs's
      known gap 2, and the ceiling on the whole group is that `serve.rs` is large — read it at the
      anchor.

## Backlog

- `Core\Metrics`'s three members, Tier 0 and always present — ADR 0076 § 3, rows in
  `crates/nvs-stdlib/src/registry.rs`; § 4's `tainted` refusal on the `labels` value is the half
  that needs `nvs-types`.
- Seven of § 1's nine series have no writer — the query, GC, spawn and memory numbers come from
  ADR 0041 §§ 1-3's event kinds, which is where the wiring is specified.
- `Core\Log`'s JSON-Lines record gains `trace_id` and `span_id` — ADR 0076 § 6, ADR 0020 § 6.
- `[trace] propagate` still has no reader, as § 6 leaves it — `crates/nvs-config/src/tree.rs:757`.
- ADR 0073's fleet-scoped lease is M8's, not this goal's — ADR 0073 *Verification*, and the stage-6
  check's own comment in `docs/agent/loop-goal.toml` carries the triage.
