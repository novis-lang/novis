# Handoff

## State

**M4's Stage 8, plus the acceptance gate's own open item.** The tree is at **867 conformance plus
189 differential**, all green. Nothing is blocked.

The guard-name debt is **1 unresolved of the 128 named test entries** `loop-goal.toml` holds, and
`nvs-codegen (fatal locals)` is now **0 of 1** — closed, joining `nvs-ir (control flow)`, `nvs-ir
(targets and tags)`, `nvs-syntax (the last unparsed shapes)` and both `nvs-types` blocks. The one
name left is Stage 8's own gate, and it is the next group. The debt file's two counts stay derived
off the tree by the pass its header describes, never carried forward.

This session landed `a_fatal_releases_the_frames_locals` over the emitted code, and the shape is
worth knowing before the next fatal-path guard: nothing in a program runs after a fatal, so the
observation is a `#[global_allocator]`'s **balance** across the run — the shape `tests/arrays.rs`
already uses, here counting bytes rather than requests. `Core\Arr::countBy` over a `float`-keyed
subject is the trigger, being a `Fault::fatal` reachable from source with two locals live, and a
second assertion on bytes *ever* allocated is what keeps the zero balance from being vacuous. The
one allocation the run is meant to leave behind is the fatal's pending message, drained before the
balance is read.

## Next group

**Stage 8's end gate, and it is a classification slice rather than a lowering one.** File set:
`crates/nvs-ir/` (the crate has no `tests/` directory — the guard is a new
`crates/nvs-ir/tests/refusals.rs` or an in-crate `#[cfg(test)]` module), `tools/holes.py`, and
`docs/agent/loop-goal.toml:1004` for the check block that names it.

- [ ] **`every_refusal_is_a_diagnostic_or_decided`** — `docs/agent/guard-name-debt.md:322`, the last
      unresolved name, cause 3. The check's own comment at `docs/agent/loop-goal.toml:999` is the
      specification: read `nvs-ir`'s and `nvs-codegen`'s sources for refusal sites and fail naming
      every one not on an allowlist frozen in the test, which **may never grow**. `tools/holes.py`
      already reads exactly those sites and is where the recognizer should come from rather than a
      second regex.
- [ ] **Decide the 17 standing sites before freezing the allowlist**, since the guard cannot go
      green while a site is neither. `python tools/holes.py` attributes 15 of them to items 1, 4,
      16 and 25 — item 1's nine are all `emit.rs` catch-all arms (`the binary operator {other:?}`
      at `crates/nvs-codegen/src/emit.rs:1348`, `the terminator {other:?}` at `:3074`, `the runtime
      helper {other:?}` at `:3458`), which read as internal-consistency arms unreachable from
      source rather than as holes. The two unattributed ones are
      `crates/nvs-codegen/src/ty.rs:116` and `:121`.
- [ ] **Then the plan's `Open now` loses its acceptance-gate sentence**, the debt file's Stage 8
      block closes, and `docs/agent/guard-name-debt.md` is a file of ticks only.

## Backlog

- Item 1's promotion-table lowering, 9 refusal sites — `python tools/holes.py --item 1`.
- Item 16's named/spread argument lowering, 3 sites — checker half first, per
  `docs/agent/loop-goal.md` § *Standing decisions*.
- Item 25's `object` representation arm, 2 sites — same source.
- The two unattributed `nvs-codegen/src/ty.rs` sites belong to no item and may want one.
- ADR 0053 § 4's abandoned-generator `finally`, pre-authorized in § *Standing decisions*.
