# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run.** Stage 1 is the
carried floor, stage 2 is done ([0186](../decisions/0186.md) is the only ADR number this goal opens),
and stages 3 to 10 are complete but for stage 10's `otlp` push, which is the same transport stage 11
needs and is the next group. Stage 11 is now one item short of done. Nothing is blocked.

**Head sampling is landed, and the decision is the root's.** `nvs_server::trace::take`
(`crates/nvs-server/src/trace.rs:95`) carries `[trace] sample` off the snapshot standing when the
request arrived; `nvs_runtime::TraceContext::rooted` (`crates/nvs-runtime/src/trace_context.rs:96`)
makes the one draw, and `continuing` hands the rate to it rather than deciding above, so a header
this process cannot read is a new trace in the draw exactly as it is in the rule. A continued trace
keeps the flag that arrived either way. `nvs_config::export::head_sample`
(`crates/nvs-config/src/export.rs:120`) is the one home for the unwritten default, which is `0.0`,
and `rule:observability/sampling-is-head-based` is `shipped` with those three files as its guards.

**A sampled request's spans are derived, not probed.** `nvs_server::trace::spans`
(`crates/nvs-server/src/trace.rs:189`) turns the events `rule:testing/debug-probes` already filed into
`rule:observability/four-kinds-become-a-span`'s four, bounded by `SPAN_CEILING`. **Nothing calls it
yet**: the door now decides who is recorded, and no transport takes what they produced.

## Next group

**Stage 11: the OTLP push, behind the `exporter` feature** — one file set: a new
`crates/nvs-server/src/otlp.rs`, `crates/nvs-server/src/lib.rs`, `crates/nvs-server/src/trace.rs` and
`crates/nvs-cli/src/serve.rs`.

- [ ] **A sampled request's spans are pushed to `[trace] endpoint` as one trace**
      (`rule:observability/the-exporters-are-crates`). The spans `crates/nvs-server/src/trace.rs:189`
      derives go to one bounded process-wide queue and a task drains it; the encoder is written here
      against the OTLP specification exactly as the exposition format was, with
      `crates/nvs-server/src/prometheus.rs` as the feature-gated sibling to sit beside and
      `crates/nvs-server/src/lib.rs:138` as the `#[cfg(feature = "exporter")]` gate to copy (the
      re-export list is `crates/nvs-server/src/lib.rs:168`). `[trace] exporter` and `endpoint` are
      `crates/nvs-config/src/tree.rs:1064-1068`, whose `[unread:]` trailer this closes, and
      `nvs_config::export::Metering` (`crates/nvs-config/src/export.rs:91`) is the resolved-reading
      shape to follow. `crates/nvs-cli/src/serve.rs:479` is where the sibling listener is armed at
      the boot. The check wants `a_sampled_request_is_pushed_to_the_otlp_endpoint_as_one_trace`
      under `-p nvs-server`.
- [ ] **An unreachable collector drops and counts spans, and never delays a response**
      (`rule:observability/an-exporter-brings-no-second-scheduler-and-no-second-client`). A full
      queue drops and counts rather than blocking the request that filled it, and a collector that
      will not answer is the same case: `crates/nvs-server/src/trace.rs:116`'s `SPAN_CEILING` bounds
      one request, and the queue's own bound is what bounds the process. The check wants
      `an_unreachable_collector_drops_and_counts_spans_and_never_delays_a_response` under
      `-p nvs-server`.

## Backlog

- Stage 12: a runaway under `nvs serve`, and the hot-reload case M7's *Verify* names
  ([docs/agent/loop-goal.md](loop-goal.md) § *Stage 12*).
- Stage 13: flip the rest of this goal's rules to `shipped` with `guardedBy` filled, then `python
  tools/rules.py --render` (§ *Stage 13*).
- `crates/nvs-config/src/default.toml:743`'s "NOT IMPLEMENTED" note under `[trace]` goes when the
  push lands.
- A scheduled run's root is never head-sampled: a fire gets `Ctx::new`'s `TraceContext::started()`
  and no door reads a header for it (`crates/nvs-server/src/schedule.rs:1060`, owner
  `rule:observability/sampling-is-head-based`).
