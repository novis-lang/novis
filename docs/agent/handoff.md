# Handoff

## State

**Goal `decided-closures`, stage 2 — the runtime and the lowering.** Stage 1's floor is the closed goal
`cache-shared-dial`'s checks, and they pass.

The `otlp` push half is **built for both signals**, so `crates/nvs-runtime/src/metrics.rs` gap 1 is
deleted and `[metrics] endpoint` is read rather than tagged `[unread:]`. One
`crates/nvs-server/src/otlp.rs` now carries both: `Signal` picks the path a base URL takes
(`/v1/traces`, `/v1/metrics`) and the key a refusal quotes, `delivered` is the one dialler both use,
and `push_registry_on_this_core` gathers `nvs_runtime::metrics::every_core` every `INTERVAL` and
merges it through `crate::prometheus::merged` — the scrape's own arithmetic, now `pub(crate)`, so
there is no second copy of it. `nvs serve` arms that task on the worker the scrape listener and the
span drain already go to, and `metrics_collector` in `crates/nvs-cli/src/serve.rs` is
`trace_collector`'s twin on the other block.

Both of stage 2's checks are green. What keeps the goal open is its **owner gate**, not a check:
three stage 2 items are still gaps naming this goal. Nothing is blocked.

## Next group

**Stage 2: the three gaps the stage's checks do not name** — one file set:
`crates/nvs-runtime/src/`, `crates/nvs-ir/src/`.

- [ ] **`crates/nvs-runtime/src/routes.rs:83` gap 1 — measure the linear walk before replacing it** —
      the gap is `rule:routing/path-grammar`'s trie against the scan that is there. The goal's own
      sentence makes this measure-first: run `benches/serve-proxied.json` and build the trie only if
      the walk shows, and otherwise strike the gap as a stated bound naming where the figure lives
      (`docs/agent/conventions.md` § *A code comment* forbids the figure itself in the comment).
      `crates/nvs-runtime/src/routes.rs:83` is the gap and the walk it describes is beside it.
- [ ] **`crates/nvs-ir/src/lib.rs:614` gap 18 — a throw escaping an abandoned generator's `finally`
      is reported and replaces nothing** — through the escalation ladder rather than dropped, per
      `rule:errors/propagation` and `rule:errors/throw-is-not-slower` (the raise is what allocates,
      and this path raises once). `crates/nvs-ir/src/lib.rs:614` is the gap.
- [ ] **`crates/nvs-runtime/src/record.rs:48` gap 1 — a `secret` into an array element or a shape
      field is refused at compile time** — the walk that would meet the value is here and the
      refusal is `nvs-types`'s, so this one leaves the file set: expect
      `crates/nvs-types/src/` beside `crates/nvs-runtime/src/record.rs:48`. Take it last, or give
      it its own group.

## Backlog

- Stage 3 opens at `crates/nvs-diagnostics/src/embedded.rs:30` gap 1 — `docs/agent/loop-goal.md`
  § *Stage 3*.
- Nothing builds this crate without the `exporter` feature, so the two `#[cfg(not(...))]` arms in
  `crates/nvs-cli/src/serve.rs` are compiled by no check — `tools/verify.py` owns whether that is
  worth a pass.
- `docs/decisions/0186.md` § *Investigation* stays frozen; it predates both pushers existing.
