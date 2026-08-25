# Handoff

## State

**Stage 0 is open, and it outranks everything below it.** [loop-goal.md](loop-goal.md) § *Stage 0*
holds items 1 to 17; **1 to 14, 16 and 17 are done**, so what is open is **item 15 alone**, four
named tests at `stage = "0 catch-up"` in [loop-goal.toml](loop-goal.toml). `loop.py` short-circuits
at stage 0, so **Stage 3 is shut until it clears**.

**Item 14 landed.** ADR 0020 § 1's call-stack limit is a third hot word in `Ctx`
(`STACK_LIMIT_OFFSET` is 16) with a `stack_floor` beside it; `mwl-codegen` emits one load, one
`get_stack_pointer` compare and a predicted-not-taken branch at the **first** safepoint of a
non-leaf function, which is the function-entry one, and `mwl_stack_check` decides which of the two
tiers a crossing is. The soft tier throws `RecursionError`, new in spec § 10's tree under
`RuntimeError`; the hard tier is the `FATAL` that section already said no `catch` sees.
`crates/mwl-runtime/src/ctx.rs`'s module doc § *The call-stack limit* owns the design and the one
known gap — the 8 MiB ceiling is **asserted** from the stack pointer at `Ctx::new`, not discovered,
because reading a thread's real bounds needs a platform call this crate has no dependency for. It is
permissive rather than wrong on a shallower stack, and `Ctx::arm_stack_limit` is the seam until M6
gives a request its own stack.

Verify is green (1554 tests, 74 suites, clippy and fmt clean). Conformance **435**, differential
**90** — unchanged, this slice added no `.mwlt`. `mwl test tests/` reports 519 passed / 6 failed;
those six fail identically on `HEAD~1` and are the pre-existing PHP-on-Windows oracle set, not a
regression. Expect a one-time step in `benches/abi-probe`'s `frame_depth`.

`examples/collect.mwl` still exits 1 at `Core\Out::capture`, genuinely blocked behind ADR 0088's
sink carriers; it is M4S work.

**`orient.py` did not print ADR 0020 § 1** even though the item's own text names it as the owner —
one `peek.py` recovered it, but `[context] adrs` in `loop-goal.toml` should carry
`0020-error-escalation-ladder.md` § 1 for as long as item 14 is the current item.

## Next group — item 15, three slices over one file

They share **`crates/mwl-runtime/src/array.rs`** and its own `#[cfg(test)]` module; nothing else is
touched until the last one. [loop-goal.md](loop-goal.md) item 15 is the specification, and that
file's module doc owns the decision, the PHP comparison and what the ABI addition costs if it waits.
[ADR 0007 § 5](../adr/0007-explicit-type-system.md) is **unchanged** — this is representation, not
semantics.

- [ ] **The packed representation itself** — a list-shaped array holds no index map and no key
      strings. Test `a_list_shaped_array_holds_no_index_map`, `-p mwl-runtime`. Anchors:
      `crates/mwl-runtime/src/array.rs:327` (`ArrayHeader`), `:362` (`MwlArray`).
- [ ] **The integer subscript path** — reaching an element by integer index allocates no key. Test
      `an_integer_subscript_allocates_no_key`. Anchors: `crates/mwl-runtime/src/array.rs:795`
      (`mwl_array_get`), `:846` (`mwl_array_set`), `:880` (`mwl_array_append`).
- [ ] **The degrade path and the equivalence** — the first key that breaks the invariant falls back
      to today's hash form with no observable difference. Tests
      `a_non_sequential_key_degrades_the_packed_array`,
      `both_representations_answer_every_primitive_alike`. Anchors:
      `crates/mwl-runtime/src/array.rs:966` (`mwl_array_next_slot`), and the
      `mwl_array_key_at`/`mwl_array_value_at` pair the `foreach` cursor reads through.
- [ ] Last, and only after the three above are green: the **first** `docs/perf/history.ndjson`
      entry, with a `php_ratio`, per [ADR 0026](../adr/0026-performance-measurement-methodology.md).
      That file not existing is why nothing caught this.

## Backlog

- `Core\Fatal::onLimit` and the `[limits] fatal_reserve_*` directives — ADR 0020 § 1, M4S/M7.
- The stack ceiling is asserted, not discovered — `mwl_runtime::ctx`'s module doc; closes at M6.
- `Core\Out::capture`, and with it `examples/collect.mwl` — ADR 0088's sink carriers.
- `Core\Json::decodeAs<T>` — `mwl_stdlib::json` gap 2; the written call-site type argument it waited
  on exists now.
- `do`/`while` is the one M4 control-flow statement that does not lower — `mwl-ir`'s own gap list.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
