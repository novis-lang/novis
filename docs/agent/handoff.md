# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run.** Stage 1 is the
carried floor, stage 2 is done ([0186](../decisions/0186.md) is the only ADR number this goal opens),
and stages 3 to 11 are built. Nothing is blocked.

**The OTLP push is landed, behind the `exporter` feature.** `nvs_server::otlp`
(`crates/nvs-server/src/otlp.rs`) is the sibling of `prometheus`: one bounded process-wide queue
(`otlp::queue`, `QUEUE_CEILING` spans, oldest dropped and counted), a protobuf encoder written here
against OTLP `trace/v1`, and one `POST` per batch driven by `nvs_host::block_on` over a parking
socket. `push_queued_on_this_core` is the drain task, armed on worker 0 beside the scrape listener
(`crates/nvs-cli/src/serve.rs:1169`). A collector that is gone costs a batch, not a response: a
request touches the queue and nothing else.

**`[trace]` is read at the boot that dials it.** `nvs_config::export::Tracing`
(`crates/nvs-config/src/export.rs:118`) is the resolved reading beside `Metering`;
`serve::trace_collector` (`crates/nvs-cli/src/serve.rs:2104`) refuses an `otlp` exporter this build
cannot run or one with no `endpoint`, and `nvs_server::otlp::Endpoint::of` parses and resolves the
URL once — `https` and a scheme that is not HTTP are refused where they are written. `tree.rs`'s
`[trace] endpoint` no longer carries an `[unread:]` trailer.

**Nothing hands a finished request's spans over yet**, which is known gap 1 in `otlp.rs`'s module
doc and the next group: `crate::trace::spans` derives the graph and only a test calls `queue`.

## Next group

**Stage 11: the seam from a finished request to the span queue** — one file set:
`crates/nvs-runtime/src/host.rs`, `crates/nvs-host/src/isolate.rs` and
`crates/nvs-cli/src/serve.rs`.

- [ ] **A finished request's trace events reach the door**
      (`rule:observability/four-kinds-become-a-span`). `Completion`
      (`crates/nvs-runtime/src/host.rs:395`) carries the output, the status and the failure and not
      the events `nvs_runtime::Ctx::trace` (`crates/nvs-runtime/src/ctx/trace.rs:328`) filed, so the
      door cannot derive spans for a request it just answered. Decide it under ADR 0004's ordering
      and write it down: the events on the completion, taken only where the request's
      `TraceContext::sampled` says somebody is recording, is the shape that costs an unsampled
      request nothing. The isolate that answers is built at `crates/nvs-cli/src/serve.rs:955`.
- [ ] **The door queues what it derived** (`rule:observability/the-exporters-are-crates`). With the
      events in hand, `crate::trace::spans` (`crates/nvs-server/src/trace.rs:194`) runs once per
      answered request and its result goes to `nvs_server::otlp::queue`
      (`crates/nvs-server/src/otlp.rs:310`) — the call that closes known gap 1 in that module's doc.
      A scheduled run is the same call with the entry as the root's request
      (`crates/nvs-server/src/schedule.rs:78`).

## Backlog

- `[metrics] endpoint` has no pusher: a tree writing `exporter = "otlp"` under `[metrics]` builds a
  registry nothing ships — `crates/nvs-server/src/metrics.rs:86`, known gap 1 there.
- A span has no window; every one is encoded as an instant — `crates/nvs-server/src/otlp.rs` known
  gap 2, owner M10.
- `Core\Metrics`'s three rows are goal `m8-stdlib-depth`'s — `crates/nvs-server/src/metrics.rs:94`.
- The `unowned` gaps at `crates/nvs-server/src/route.rs:30` and `bounds.rs:62` are goal
  `unowned-closures`'.
