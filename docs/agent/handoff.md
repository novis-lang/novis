# Handoff

## State

**Goal 6, M7, stage 6 — the exporter is what is left of it.** The stage-6 acceptance failure is
closed and it was a filing bug, not work: `no_probe_is_added_to_the_measured_path` asserts ADR 0076
§ 1's "no probe site is added to the per-statement/per-call path", which is an effect on *emitted
code*, and `crates/nvs-server/Cargo.toml` names neither `nvs-codegen` nor `nvs-ir`. The check moved
to `-p nvs-codegen` in both copies of the goal file and the test landed beside ADR 0018's own probe
fixtures. `the_default_metric_series_are_exported` stays where it is — the registry is the server's.

**ADR 0076 § 6's two blocks are read at boot.** `crates/nvs-config/src/export.rs` is the roster and
the refusal, reached from `crates/nvs-config/src/resolve.rs:340` with every other merged-tree check.
Both blocks were already deserialized (`crates/nvs-config/src/tree.rs:737`, `:751`) and already
`System` in the registry; what was missing was that nothing refused a value. `E0627` is a written
`exporter` neither block spells — metrics take a scrape or a push, a trace only the push, and the
asymmetry is § 6's own — and `E0628` is a `[trace] sample` outside `0.0`–`1.0`, which the item did
not name and which is in for the reason the module doc gives: nothing downstream can tell `5` from a
deployment that meant it. `propagate` still has no reader, as § 6 leaves it.

**`[context] adrs` gained `0076 §§ 1, 6, 7`** in `docs/agent/loop-goal.toml` and in
`docs/agent/goals/6-server.toml`. The pack printed no part of ADR 0076 while the whole open group is
that ADR, and three calls went on fetching § 1, § 6 and § 8 by hand.

## Next group

**ADR 0076's registry and its series**, over a new `crates/nvs-server/src/metrics.rs`,
`crates/nvs-server/src/lib.rs` and `crates/nvs-config/src/export.rs`. In this order — nothing can be
registered before the thing that bounds it exists.

- [ ] **The per-core registry, with § 7's `max_series` bound** (ADR 0076 §§ 5, 7) — a new
      `crates/nvs-server/src/metrics.rs`, declared in the module list at
      `crates/nvs-server/src/lib.rs:98`. The bound is `crates/nvs-config/src/tree.rs:745`'s
      `max_series`, per core, and § 7 is *refuse the new, never evict the old*: a recreated counter
      reads as a process restart to every backend and corrupts `rate()` silently. The exporter
      selection is already resolved — `crates/nvs-config/src/export.rs:36`'s `Exporter`, with
      `false` meaning no registry is built at all.
- [ ] **§ 1's nine default series, and the `route` label reaching them** (ADR 0076 §§ 1, 4) — this
      is what `the_default_metric_series_are_exported` in the stage-6 `-p nvs-server` check names.
      The label is the *declared route name* and never a path; the door already took the match and
      `crates/nvs-server/src/route.rs:129`'s `label` hands it back. § 4 refuses a `tainted` label
      value, which is a `nvs-types` qualifier the server cannot see — check which half of that is
      this crate's before writing it, the way the moved check above had to be checked.
- [ ] **§ 8's two dependencies, feature-gated** (ADR 0076 § 8) — the selection they hang off is
      `crates/nvs-config/src/export.rs:36`'s `Exporter`, and the feature is declared beside the
      module list at `crates/nvs-server/src/lib.rs:93`: `metrics-exporter-prometheus` for
      the scrape and `opentelemetry`/`opentelemetry-otlp` for the push, defaulting on in the server
      distribution only, so an ADR 0048 single-file executable carries neither. Pre-authorized under
      ADR 0051 § 4; take it last, since the registry above is what they export.

## Backlog

- `an_after_response_tree_outlives_its_connection`, filed `-p nvs-host` — still open; ADR 0072 § 6.
- `a_fleet_scoped_entry_fires_once_across_the_fleet_under_its_lease` — ADR 0073 *Verification*'s M8
  bullet; needs a compare-and-set the shared tier does not have.
- `Core\Log`'s record gaining `trace_id`/`span_id` whenever a trace is active — ADR 0076 § 6's last
  paragraph, and the whole of what makes a log line jump to a trace.
- Raw/unparsed body access for an arbitrary content-type — ADR 0024 *Revisiting*, narrowed by
  `docs/plan/m7.md`; a decided-and-recorded call in `Core\Request`'s module doc.
- `[trace] propagate` has no reader: ADR 0076 § 6 sends `traceparent` on outbound
  `Core\Http\Client` calls, and nothing does.
