# Handoff

## State

**Goal 6, M7. ADR 0076 § 2's trace now reaches the request**, which closes the stage-6 acceptance
check that had been failing on `an_inbound_traceparent_is_continued_and_a_missing_one_is_generated`.
The door reads the arrived `traceparent` at `crates/nvs-server/src/trace.rs:64`
(`nvs_server::trace::take`), beside `route::take` and `mount::carry`, and writes the answer onto the
carrier; `nvs_runtime::Inbound` holds it and `Ctx::set_inbound` is the one place it becomes the
context's, leaving the eager root `Ctx::new` drew standing where no door asked. `nvs-cli`'s handler
is the one production caller. Two `traceparent` field lines are treated as one malformed one — the
module doc owns why — and every other way a header can be unusable is
`nvs_runtime::trace_context`'s, unchanged.

**The full `verify.py` gate was not reached**, and not because of anything here: it stopped at `cargo
fmt` on `crates/nvs-config/tests/secret.rs`, which a concurrent loop session has in flight along with
five other paths. `nvs-server`, `nvs-runtime` and `nvs-cli` were checked scoped instead — fmt, test
and clippy, all green — and the new playbook bullet owns why formatting somebody else's file is the
wrong repair. **Re-run the whole gate once the tree is quiet.**

**The exporter is still the whole of what is open in stage 6**, and it is bigger than the handoff's
old item [2] read: ADR 0076 § 6 is two `System` config blocks, § 7 a per-core cardinality bound, § 8
two new dependencies, and § 1 nine series. It is decomposed below rather than carried as one item.

**Two of that check's names need triage before a line is written.**
`no_probe_is_added_to_the_measured_path` is a claim about ADR 0018's per-statement path, which
`crates/nvs-server` cannot observe at all; `the_default_metric_series_are_exported` names series
(`nvs_gc_pause_seconds`, `nvs_memory_bytes`) that are the runtime's readings. Both may be filed in
the wrong crate — the playbook's run of `loop-goal.toml` bullets is the shape, and the manifest plus
one `sed -n '1,20p'` of an existing test in the named directory settles it.

## Next group

**ADR 0076's exporter**, over `crates/nvs-config/src/tree.rs`, `crates/nvs-config/src/directive.rs`
and a new `crates/nvs-server/src/metrics.rs`. Take them in this order — the registry cannot be
written before it knows what bounds it.

- [ ] **`[metrics]` and `[trace]` are read at boot, both `System`, and a bad `exporter` refuses it**
      (ADR 0076 § 6) — the block structs go beside `crates/nvs-config/src/tree.rs:713`'s `Schedule`
      and their fields beside `crates/nvs-config/src/tree.rs:94`; the class is
      `crates/nvs-config/src/directive.rs`'s registry. `exporter` is `false | "prometheus" | "otlp"`
      for metrics and `false | "otlp"` for traces, and anything else is the next free
      configuration code, **E0627** — `crates/nvs-config/src/log.rs` is the worked shape for a
      block whose values are a closed set. `sample` is `0.0`–`1.0` and `propagate` a bool; neither
      has a reader yet and both are stated in § 6.
- [ ] **The per-core registry, with § 7's `max_series` bound** (ADR 0076 §§ 5, 7) — new
      `crates/nvs-server/src/metrics.rs`, registered in `crates/nvs-server/src/lib.rs:104` beside
      `pub mod trace;`. A new series past the cap is a **no-op**, an existing one is **never
      evicted** (§ 7 argues why at length), and the memory is charged to the core exactly as ADR
      0059 § 3's cache is. § 8 names `metrics-exporter-prometheus` for the scrape path; picking it
      is pre-authorized under ADR 0051 § 4.
- [ ] **§ 1's nine default series, and the `route` label reaching them** (ADR 0076 §§ 1, 4) —
      `crates/nvs-server/src/route.rs:129`'s `label` is written and has no caller, which is that
      module's own known gap 2; this closes it. The label is the matched route's **declared name**
      and never the path, and § 4 refuses a `tainted` label value. Triage where
      `the_default_metric_series_are_exported` and `no_probe_is_added_to_the_measured_path` can
      honestly live before writing either — see `## State`.

## Backlog

- `Core\Metrics`'s three members — ADR 0076 § 3, Tier 0, and not reachable from `crates/nvs-server`.
- `Core\Log`'s record gains `trace_id`/`span_id` while a trace is active — ADR 0076 § 6's last line.
- `Core\Server::traceId()` reads the context's trace id — ADR 0076 § 2; no member exists yet.
- Head sampling: `[trace] sample` is what will ever set a root's `sampled` — `nvs-runtime/src/trace_context.rs`'s *What is not here yet*.
- `a_fleet_scoped_entry_fires_once_across_the_fleet_under_its_lease` — ADR 0073 *Verification*'s M8 bullet, waiting on the shared store's compare-and-set.
- ADR 0102 § 4's CSRF refusal at the door — `crates/nvs-server/src/route.rs`'s known gap 1.
