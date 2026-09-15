# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run.** Stage 1 is the
carried floor, stage 2 is done ([0186](../decisions/0186.md) is the only ADR number this goal opens),
and stages 3 to 10 are complete but for stage 10's `otlp` push, which is the same transport stage 11
needs and is grouped with it. Stage 11 is now half landed. Nothing is blocked.

**A sampled request's spans are derived, not probed.** `nvs_server::trace::spans`
(`crates/nvs-server/src/trace.rs:176`) turns the events `rule:testing/debug-probes` already filed into
`rule:observability/four-kinds-become-a-span`'s four — the root, a `query`, an outbound call and a
`spawn` — and `SpanKind` has no variant for the two the rule excludes, so a `call` or a `gc` cannot
become one by a later reader's oversight. `SPAN_CEILING` is what one request holds at most. **Nothing
calls it yet**: the door decides no sampling and no transport pushes it.

**A request's span id is now its own, and the caller's arrives beside it.**
`nvs_runtime::TraceContext` used to adopt the arrived `traceparent`'s span id as this request's, which
would have given a continued trace two spans with one id the moment a root span existed; `continuing`
now draws this request's span and keeps the arrived one as `parent_span_id()`. The outbound header is
unchanged in shape and now names a span that exists. That module's doc owns why.

**The pack's stage 11 manifest is short two things**, both of which this session paid for by hand:
`[context] modules` prints no `crates/nvs-runtime/src/trace_context.rs` even though the stage names
the file, and `[context.stage.11] rules` names neither `rule:observability/sampling-is-head-based`
nor `rule:observability/a-call-never-becomes-a-span`, which are the two the remaining items are
specified by.

## Next group

**Stage 11: the rate at the door, then the push** — one file set: `crates/nvs-server/src/trace.rs`,
`crates/nvs-cli/src/serve.rs` and `crates/nvs-cli/src/runner.rs`.

- [ ] **Sampling is decided at the root, and a sampled inbound trace is always continued**
      (`rule:observability/sampling-is-head-based`). Only a request that *started* a trace draws
      against `[trace] sample`; one that continued a sampled header is recorded regardless, and one
      that continued an unsampled header stays unsampled — the root already decided.
      `crates/nvs-server/src/trace.rs:83`'s `take` is what widens to carry the rate, and its two
      callers are `crates/nvs-cli/src/serve.rs:851` and `crates/nvs-cli/src/runner.rs:1479`. The rate
      must come off the **live** snapshot per request, not off boot: `[trace] sample` is
      `crates/nvs-config/src/tree.rs:1070`, the handler closure that would capture the holder is
      `crates/nvs-cli/src/serve.rs:758`, the `current.load()` idiom is
      `crates/nvs-cli/src/serve.rs:1325`, and `crates/nvs-stdlib/src/http.rs:4074` is the existing
      per-request read of a `[trace]` key. The check wants
      `trace_sample_decides_at_the_root_and_a_sampled_inbound_trace_is_always_continued` under
      `-p nvs-server`.
- [ ] **The OTLP transport, behind the same `exporter` feature** — stage 10's third bullet and stage
      11's last two checks are one piece of work. A sampled request's derived spans
      (`crates/nvs-server/src/trace.rs:104`'s ceiling bounds them) go to one bounded process-wide
      queue and a task pushes them to `[trace] endpoint`
      (`crates/nvs-config/src/tree.rs:1068`, whose `[unread:]` trailer this closes);
      a full queue drops and counts, and an unreachable collector never delays a response
      (`rule:observability/the-exporters-are-crates`). `crates/nvs-server/src/prometheus.rs` is the
      feature-gated sibling to sit beside.

## Backlog

- Stage 12: a runaway under `nvs serve`, and the hot-reload case M7's *Verify* names
  ([docs/agent/loop-goal.md](loop-goal.md) § *Stage 12*).
- Stage 13: flip this goal's rules to `shipped` with `guardedBy` filled, then `python tools/rules.py
  --render` (§ *Stage 13*).
- `crates/nvs-config/src/default.toml:743`'s "NOT IMPLEMENTED" note under `[trace]` goes when the
  push lands.
