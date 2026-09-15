# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run.** Stage 1 is the
carried floor, stage 2 is done ([0186](../decisions/0186.md) is the only ADR number this goal opens),
and stages 3 to 10 are complete but for one bullet: stage 10's `otlp` push, which is the same
transport stage 11's last two checks need and is therefore grouped with them. Stage 11, the traces, is
the earliest red stage. Nothing is blocked.

**The exporter is a cargo feature, and one manifest switches it.** `nvs-server`'s `exporter` — on by
default — gates `crates/nvs-server/src/prometheus.rs` and nothing else; `crates/nvs-server/src/metrics.rs`
is in every build, which is `rule:observability/the-exporter-is-a-feature-and-core-metrics-is-not`'s
second half and the crate doc's § *The `exporter` feature*. The workspace entry for `nvs-server`
carries `default-features = false` so that `nvs-cli`'s own `exporter` is the single switch, and
`cargo tree -p nvs-cli --no-default-features -f "{p} feats={f}"` is what shows the crate then reached
with no features. A featureless build refuses a configured `[metrics] exporter` at boot through
`crates/nvs-cli/src/serve.rs`'s `exporter_not_built`, which takes the compile-time fact as a parameter
so the sentence an operator reads is asserted in the build the suite runs in.

**No gate builds the featureless shape** — the playbook bullet under *Running things* is the command
that does, and until `verify.py` carries one it is a session's own job after touching a `#[cfg]` arm.

## Next group

**Stage 11: spans at the door** — one file set: `crates/nvs-server/src/trace.rs`,
`crates/nvs-server/src/serve.rs`, and `crates/nvs-config/src/tree.rs`'s `[trace]` block. The driver's
earliest failing check is this stage's, and its four test names are the four things to write.

- [ ] **Exactly four event kinds become a span, and a `call` never does** — the root, a `query`, an
      outbound HTTP call and a `spawn`, with `gc` a metric instead
      (`rule:observability/four-kinds-become-a-span`, `rule:observability/a-call-never-becomes-a-span`).
      `crates/nvs-server/src/trace.rs:64`'s `take` is the door that already decides trace identity and
      the module doc's "nothing here decides whether the trace is exported" is the sentence this
      changes; the check wants `exactly_four_event_kinds_become_a_span_and_a_call_never_does` under
      `-p nvs-server`.
- [ ] **Sampling is decided at the root, and a sampled inbound trace is always continued** —
      `crates/nvs-config/src/tree.rs:1070`'s `sample` is head-based and read once, at the door beside
      `crates/nvs-server/src/trace.rs:64`, never per event
      (`rule:observability/an-inbound-traceparent-is-continued`). The check wants
      `trace_sample_decides_at_the_root_and_a_sampled_inbound_trace_is_always_continued`.
- [ ] **The OTLP transport, behind the same `exporter` feature** — stage 10's third bullet and stage
      11's last two checks are one piece of work: `crates/nvs-config/src/tree.rs:1054`
      (`[metrics] endpoint`) and `:1067` (`[trace] endpoint`) are the two `[unread:]` keys it closes,
      `crates/nvs-server/src/prometheus.rs:213`'s `scrape` over
      `crates/nvs-server/src/metrics.rs:771`'s `every_core` is what the metrics half pushes, and
      `rule:observability/an-exporter-brings-no-second-scheduler-and-no-second-client` is the shape:
      `nvs-host`'s stream and its one TLS client, a crate for an encoder and never for a transport.

## Backlog

- `[trace] exporter = "otlp"` has no featureless boot refusal yet; it joins
  `crates/nvs-cli/src/serve.rs`'s `exporter_not_built` when the push lands.
- `nvs_config::Exporter` has no `name()`, so `exporter_not_built` spells the two protocols itself —
  `crates/nvs-config/src/export.rs:53`'s `impl` is where one would go if a second caller wants it.
- `[metrics] endpoint` and `[trace] endpoint` are the only `[unread:]` keys left in either block.
