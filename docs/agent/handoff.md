# Handoff

## State

**Goal `unowned-closures`, stage 5.** `crates/nvs-server/` carries no goal-owned gap any more — the
ticker's and the door's trace items are both struck — so what is left of the stage is the artifact
loader's aarch64 half and one `docs/agent/carried-gaps.md` row. `python tools/owners.py` reads 82
items, `--deferrals` green; stage 6's `unowned: 0` is the 15 scheduling questions and no stage-5
slice moves it.

**A recorded run files a span's events with no debugger attached.**
`nvs_runtime::Ctx::records_spans` (`crates/nvs-runtime/src/ctx/trace.rs`) is the one question every
filing site asks — this trace is recorded, or `DebugFlags::TRACE` is on — so `Core\Db`'s statement
routines, `Core\Http\Client` and `Ctx::open_spawn` file a sampled request's `query`, `http` and
`spawn`, while the call probes go on reading their own bit and a `call` never becomes a span.
`nvs_runtime::SPAN_EVENT_CEILING` is the one home of what a recorded request may hold, and
`nvs_server::trace::SPAN_CEILING` is the root plus it.

**`crates/nvs-server/src/schedule.rs`'s gap 1 was stale and is struck rather than built.** A fire's
tree has been the implementor's since `nvs-cli`'s `Scheduled` took an `nvs_config::Current`, and
`Fires::isolate` now owes it in writing, where a second implementor would read it.

**`crates/nvs-db/src/span.rs` gap 1 stands and is still `unowned`**: the `debug.trace` grant reaches
no `DebugFlags` bit, and sampling answers neither half of that.

## Next group

**Stage 5: what is left of it** — one file set: `crates/nvs-cli/src/cache.rs` with
`crates/nvs-codegen/src/` for the relocation vocabulary, then one doc cell.

- [ ] **The artifact loader speaks aarch64's relocations** — `crates/nvs-cli/src/cache.rs:166`
      gap 1, whose rule is
      `rule:packaging/an-artifact-is-a-relocatable-object-behind-a-self-describing-header`. An
      `ADR_PREL_PG_HI21`/`…_LO12` pair and a `CALL26` are bit ranges inside a fixed-width
      instruction rather than the flat little-endian field `Layout::apply` writes, so `HOST_ARCH` is
      `Architecture::Unknown` off x86-64 and every aarch64 run recompiles.
- [ ] **Spec § 13's `Core\Test` cell spells `request`'s options bag** —
      `crates/nvs-stdlib/src/test.rs:308` is the roster (`REQUEST_OPTIONS`) and
      `docs/spec/01-core-library.md:1001` the cell that states it in English; it is the last
      `docs/agent/carried-gaps.md` row goal `m7-server-surface` left behind, listed in stage 5's own
      paragraph.

## Backlog

- `crates/nvs-runtime/src/metrics.rs:105` gap 1 — only a scrape reads the registry.
- `crates/nvs-runtime/src/record.rs:38` gaps 1–2 — the `secret` walk, one file set.
- `crates/nvs-stdlib/src/json.rs:216` gaps 1–2 — the derive's per-class field walk and the encoder's
  real depth bound.
- Stage 6 is the register itself (`python tools/owners.py`), and nothing before it closes it.
