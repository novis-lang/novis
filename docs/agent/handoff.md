# Handoff

## State

**Goal `decided-closures`, stage 2 — the runtime and the lowering.** Stage 1's floor is the closed goal
`cache-shared-dial`'s checks, and they pass.

Two of stage 2's items are off the register. `crates/nvs-runtime/src/decimal.rs` gap 1 is **built and
deleted**: a division whose 128-bit scale fold overflows retries at 192 bits, exact everywhere, and the
fold that fits keeps its `u128` arithmetic — `Decimal::long_divide_at_192` is the second path and `U192`
is the width, both in `decimal.rs`. `crates/nvs-runtime/src/record.rs` gap 2 is **struck as a bound**:
the walk reads a node kind off a `Tag` and an enum has none, so through `mixed` a case is its integer,
and that is now the module's own prose rather than a numbered item.

What keeps stage 2 red is the collector — `nvs_safepoint` still clears `COLLECT` and acts on nothing —
and the `otlp` push in `nvs-server`. Nothing is blocked.

## Next group

**Stage 2: the collector, near the memory ceiling** — one file set:
`crates/nvs-runtime/src/ctx/safepoint.rs`, `crates/nvs-runtime/src/ctx/mod.rs`,
`crates/nvs-runtime/src/object.rs`, `crates/nvs-runtime/src/array.rs`, `crates/nvs-runtime/src/lib.rs`.

- [ ] **`COLLECT` runs a collection instead of being cleared** — `crates/nvs-runtime/src/ctx/safepoint.rs:536`,
      where the flag is dropped beside `DEBUG_BREAK` and `MEMORY_LIMIT`, and
      `crates/nvs-runtime/src/ctx/mod.rs:233` where it is declared. The decision is *a collector that
      runs only near the memory ceiling*, so the normal request path pays nothing; what it spends per
      request goes in the module doc per `rule:programs/memory-priority`. The test the stage names is
      `a_cycle_is_reclaimed_near_the_memory_ceiling_while_the_request_runs`.
- [ ] **The walk reuses dismantling rather than a second traversal** —
      `crates/nvs-runtime/src/object.rs:3237` and `crates/nvs-runtime/src/array.rs:1299` are the two
      `dismantle` bodies that already know how to reach a value's children, per
      `rule:classes/no-destructors`.
- [ ] **Both halves of the gap are deleted once it runs** — `crates/nvs-runtime/src/lib.rs:199` gap 5
      and `crates/nvs-ir/src/lib.rs:614` item 14 say the same thing from either side of the crate edge.

## Backlog

- Stage 2's other check, its own file set: `an_otlp_endpoint_receives_the_registry_as_a_push` in
  `nvs-server` — `docs/agent/loop-goal.toml:11558`.
- `crates/nvs-runtime/src/record.rs:38` gap 1: refuse at compile time storing a `secret` into an array
  element or a shape field — a `nvs-types` slice, not a runtime one.
- Nothing in the workspace builds `nvs_render::Node::EnumCase`; the kind is consumer-only today, and no
  rule asks a producer for it — `docs/rules/errors/diagnostic-record.md`.
