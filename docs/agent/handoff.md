# Handoff

## State

**Goal 39 — stage 2 is closed: a record names where it was produced, and a `Throwable`'s `location`
is that same datum.** A `throw` hands the raise the carrier a record producer already takes —
`Terminator::Throw`'s new `source`, which is `InstKind::SourceConst` unchanged — and
`nvs_runtime::nvs_raise` fills `LOCATION_SLOT` from it, so the four IR instructions the lowering used
to emit per throw are gone. The zero word is a raise that is no site of its own (an unmatched `catch`
hands the very same reference onward) and leaves the first throw's answer standing.

`rule:errors/a-record-names-where-it-was-produced` is **shipped**.
`rule:errors/a-repeat-is-bounded-at-the-sink-that-suffers` is still `designed`, so stage 4's flip is
that rule alone once stage 3 lands. [0165](../decisions/0165.md) § *Standing decisions* are not a
session's to re-open.

The floor's `examples/logging.nvs` check was red on landed work, not on a gap: the envelope it prints
carries `source` now, and its `want` names the lines the example's own writes sit on — in
`docs/agent/loop-goal.toml` and the archived `docs/agent/goals/39-record-origin.toml` alike.

## Next group

**Stage 3: the log target's window** — one file set: `crates/nvs-runtime/src/floor.rs`,
`crates/nvs-stdlib/src/log.rs`.

- [ ] **The log target gets a small fixed table where the floor keeps one slot** —
      `crates/nvs-runtime/src/floor.rs:212` is `COALESCING_WINDOW` and
      `crates/nvs-runtime/src/floor.rs:317` is `key`, the identity the rule fixes: `ts`,
      `request_id`, `trace_id`, `span_id` and any `count` cleared before hashing, everything else
      counting, `source` included. The floor keeps its single slot unchanged.
      `rule:errors/a-repeat-is-bounded-at-the-sink-that-suffers`.
- [ ] **`Core\Log::write` writes through that table** — `crates/nvs-stdlib/src/log.rs:203` is the
      `nvs_helper!` block the member is defined in, and a record that differs is written immediately
      rather than held behind the window. Nothing touches the debug stream, which groups at read time
      ([0165](../decisions/0165.md) § 3). `rule:errors/a-repeat-is-bounded-at-the-sink-that-suffers`.
- [ ] **The three `-p nvs-runtime` tests the stage-3 check names** —
      `crates/nvs-runtime/src/floor.rs:290` is the seam a test uses instead of sleeping through the
      window: `two_interleaved_repeats_each_coalesce_rather_than_evicting_one_another`,
      `the_next_record_after_a_window_closes_carries_how_many_it_stands_for`,
      `the_floor_keeps_its_single_slot_and_its_own_window`.
      `rule:errors/a-repeat-is-bounded-at-the-sink-that-suffers`.
- [ ] **Ship the second rule** — `docs/rules/errors.json:240` to `shipped` with what stage 3 landed in
      its `guardedBy`, then `python tools/rules.py --render`. That closes stage 4 and the goal.

## Backlog

- `nvs_raise_new`'s `ArithmeticError` still reads an empty `location`: the `%`-by-zero site
  (`crates/nvs-codegen/src/emit.rs:1939`) has no carrier to hand it — `crates/nvs-runtime/src/throwable.rs`'s module doc owns the gap.
- The goal's `[context] modules` names neither `crates/nvs-ir/src/ir.rs`, `crates/nvs-ir/src/print.rs`,
  `crates/nvs-ir/src/lower/{call,convert}.rs`, `crates/nvs-codegen/src/lib.rs` nor
  `crates/nvs-types/src/error_lib.rs`, all of which stage 2 had to edit.
- A record's `count` is still filled only by the engine floor — stage 3's whole subject
  ([0165](../decisions/0165.md) § 2).
