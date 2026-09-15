# Handoff

## State

**Goal `m7-server-surface` — everything M7 promised a deployment is there to run.** Stage 1 is the
carried floor, stage 2 is done ([0186](../decisions/0186.md) is the only ADR number this goal opens),
and stages 3 to 9 are complete. Stage 10, the metrics export, is the earliest red stage and both of
its `[[check]]` blocks are now green; what is left of it is the cargo feature and the OTLP push.
Nothing is blocked.

**A scrape reaches every core through a handle, not a message.** The goal's stage 10 prose names "the
existing cross-core channel" and there is none — `nvs_host::channel` is one core's tasks and is
`!Send`. Each core's registry now lives behind a lock of its own with a weak handle in one
process-wide roster, and `crates/nvs-server/src/metrics.rs:770`'s `every_core` copies what is behind
them. The alternative (poke each core, wait for it to publish) and why ADR 0004's ordering refused it
are in that module's doc § *How a scrape reaches a core it is not running on*; the per-request cost is
one uncontended lock where it was a `RefCell` borrow.

**The endpoint is bound and answers.** `crates/nvs-cli/src/serve.rs:1950`'s `scrape_socket` resolves
and binds `[metrics] listen` at boot — `None` for every way of not asking, and a **refusal** where
`exporter = "prometheus"` names no address — and one worker gets the socket.
`crates/nvs-server/src/prometheus.rs:90`'s `serve_scrapes_on_this_core` is the loop: a task on that
core, one collector at a time, `hyper` framing both halves, and it counts no request of its own.
`[metrics] endpoint` is still the only `[unread:]` key in that block.

## Next group

**Stage 10: the exporter behind a feature** — one file set: `crates/nvs-server/src/prometheus.rs` and
`crates/nvs-server/src/lib.rs` for the gate, `crates/nvs-cli/src/serve.rs` for the boot refusal, and
the two `Cargo.toml`s that declare it.

- [ ] **The exporter goes behind a cargo feature, on by default** — gate `crates/nvs-server/src/lib.rs:123`'s
      `pub mod prometheus;` and its re-export beside it, so a CLI build carries no encoder
      (`rule:observability/the-exporter-is-a-feature-and-core-metrics-is-not`). `crates/nvs-server/src/metrics.rs:121`
      stays in every build unconditionally — that rule's second half is the whole reason the split is
      here and not one module up.
- [ ] **A build without the feature refuses any `exporter` other than `false` at boot, naming the
      feature** — `crates/nvs-cli/src/serve.rs:1950`'s `scrape_socket` is where the `prometheus` arm
      already refuses, and the `otlp` arm returns `Ok(None)` today; both become the refusal under
      `#[cfg(not(feature = ...))]`. Its neighbouring case
      `a_prometheus_exporter_with_no_listen_address_is_refused_at_boot` is the shape to copy.
- [ ] **The OTLP push** — stage 10's third bullet, and the last `[unread:]` key in the block is
      `crates/nvs-config/src/tree.rs:1051`'s `endpoint`. It pushes
      `crates/nvs-server/src/prometheus.rs:192`'s merge on an interval through stage 2's transport, so
      read `rule:observability/an-exporter-brings-no-second-scheduler-and-no-second-client` before
      reaching for a client.

## Backlog

- The `unowned` gaps at `crates/nvs-server/src/route.rs:30` and `crates/nvs-server/src/bounds.rs:62` — goal `unowned-closures`.
- `docs/plan/m7.md`'s carrier list and `crates/nvs-cli/src/serve.rs`'s "no configuration" gap — goal `plan-truth`.
- `Core\Metrics`'s three rows, which would give `Registry` a second writer — goal `m8-stdlib-depth`.
- No end-to-end `nvs serve` runaway test for the CPU and memory ceilings — `crates/nvs-runtime/src/budget.rs`.
