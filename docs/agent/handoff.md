# Handoff

## State

**Goal `decided-closures`, stage 2 — the runtime and the lowering.** Stage 1's floor is the closed goal
`cache-shared-dial`'s checks, and they pass.

The in-flight cycle collector is **built**, so three items are off the register:
`crates/nvs-runtime/src/lib.rs` gaps 5 and 7 and `crates/nvs-ir/src/lib.rs` gap 14 are deleted.
`crates/nvs-runtime/src/object.rs`'s `reclaim` is the one walk both moments share — `sweep` at
teardown, which detaches survivors, and `collect` in flight, which does not — and
`Ctx::collect_if_asked` is the only door in. `crate::budget`'s threshold raises `COLLECT` beside
`MEMORY_LIMIT` at the allocation that crossed the ceiling, so a request under its ceiling pays
nothing and a request over it collects before the counter is read, at both poll sites
(`nvs_safepoint` and `run_helper`). The run files `rule:observability/gc-pause-is-its-own-event`'s
event, which moved that rule from `designed` to `shipped`.

What keeps stage 2 red is the `otlp` push in `nvs-server`: a tree naming `exporter = "otlp"` builds a
registry nothing ships. Nothing is blocked.

## Next group

**Stage 2: the registry leaves the process as a push** — one file set:
`crates/nvs-server/src/otlp.rs`, `crates/nvs-server/src/prometheus.rs`,
`crates/nvs-runtime/src/metrics.rs`.

- [ ] **`[metrics] endpoint` gets a pusher, over the queue the trace half already drains** —
      `crates/nvs-server/src/otlp.rs:437` (`push_queued_on_this_core`, the drain task a request never
      touches), `crates/nvs-server/src/otlp.rs:308` (`[trace] endpoint`'s resolution, which the
      metrics endpoint needs its twin of) and `crates/nvs-server/src/otlp.rs:122` (the `/v1/traces`
      path constant, beside which `/v1/metrics` goes). The registry is read exactly as the scrape
      reads it, per `rule:observability/a-registry-is-per-core-and-nothing-reads-it`:
      `crates/nvs-runtime/src/metrics.rs:936` (`every_core`) summed by the same arithmetic
      `crates/nvs-server/src/prometheus.rs:213` (`scrape`) does, which is the rule's "arithmetic and
      never coordination". No second scheduler and no second client
      (`rule:observability/an-exporter-brings-no-second-scheduler-and-no-second-client`) — the encoder
      is ours against OTLP's `metrics/v1` messages, as `docs/decisions/0186.md` § *Investigation*
      settled for the trace half. The test the stage names is
      `an_otlp_endpoint_receives_the_registry_as_a_push`.
- [ ] **`crates/nvs-runtime/src/metrics.rs:105` gap 1 is deleted once it ships** — the gap is "only a
      scrape reads this", and it names no second half; what the push spends per core goes in that
      module's own doc per `rule:programs/memory-priority`.

## Backlog

- `crates/nvs-runtime/src/routes.rs:83` gap 1 — the route walk is a linear scan, not
  `rule:routing/path-grammar`'s trie (goal `decided-closures`, stage 2).
- `crates/nvs-runtime/src/record.rs:48` gap 1 — a `secret` value reaches the record walk with no
  property to be declared on (goal `decided-closures`, stage 2).
- `crates/nvs-ir/src/lib.rs:614` gap 18 — an abandoned generator's `finally` runs; a throw escaping
  one is dropped (goal `decided-closures`).
- Stage 3 onward is the goal file's own list; `python tools/owners.py --closes decided-closures` is
  what the driver gates the goal's end on.
