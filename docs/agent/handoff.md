# Handoff

## State

**Goal `resource-ceilings` — stage 5's string half is landed, and all three stage 0 cases are green**
(`target/debug/nvs.exe test tests/conformance/error/`: 46 passed, 0 failed). The array half is
untouched: `nvs_array_set`, `nvs_array_set_index`, `nvs_array_append` and `nvs_array_spread` still
allocate and are stopped only at the next poll.

**`NvsStr::alloc_or_refusal` (`crates/nvs-runtime/src/string.rs:501`) is what replaced
`alloc_uninit`.** It sorts `try_alloc_uninit`'s `None` into two: one the running request was refused,
which the caller answers with a degenerate value, and one no ceiling explains, which still aborts
through `handle_alloc_error` because that is `NvsStr::new`'s known gap and no program can drive it.
That split is the decision this stage turned on — a blanket "never abort" would answer a wrong value
with nothing having marked the request over.

**The degenerate returns.** `EMPTY_IMMORTAL` (`crates/nvs-runtime/src/string.rs:263`) is a static
`StrHeader` at `IMMORTAL_REFCOUNT`, so releasing it frees nothing however many times it happens;
`NvsStr::build` answers it without running its writer, and `nvs_str_concat_n` answers it directly.
`nvs_str_append` answers `target` unchanged, which is the one value balancing its one-reference-in,
one-out protocol. `StrWriter::grow` answers `false` and `push` then writes no more, so a refused
growth truncates rather than overflowing the payload.

**The pre-check is `crate::budget::affords`, asked in the two places a string allocation is made** —
`try_alloc_uninit` for a fresh one, against the whole layout, and `StrWriter::grow` for one in hand,
against the difference between the two layouts.

**`crate::budget::Reporting` is the reserve's third half, and it is new.** A widened ceiling and a
taken verdict were not enough: the pre-check refuses against the *balance*, which a request reaching
tier 1 is already past, so the report came out as an array of empty strings. The guard suspends the
pre-check for the length of the escalation — `Ctx::run_limit_handler` and `nvs_host::ladder`'s tier 3
— and what bounds the slice instead is the counting ceiling plus the zero-retry rule. Stage 5's array
half will need the same guard for any report path it touches.

**`run_helper` now reports a breach over a member's own fault** (`crates/nvs-runtime/src/abi.rs:470`).
A refused `NvsStr::try_build` reaches `nvs-stdlib` as "no room" and is worded there as a throw; left
alone, a resource limit would have become a `catch` a program carries on from, against
`rule:errors/escalation-ladder`. Asked on the `Ok(Err(..))` arm only, because that is the one exit
with no result `Value` to release.

The goal's own record is still unopened. `0174` was the next free number at this commit; re-derive it.

## Next group

**Stage 5: the array half** — one file set, `crates/nvs-runtime/src/array.rs` with
`crates/nvs-runtime/tests/refusal.rs` for the cases. `rule:errors/on-limit` is what the refusal owes,
and the goal's § *Standing decisions* settles the shape: a refusal is a complete no-op **including not
separating a shared array**, and `nvs_array_new`'s per-thread singleton needs nothing.

- [ ] **`nvs_array_set` and `nvs_array_set_index` refuse by answering their array unchanged**
      — `crates/nvs-runtime/src/array.rs:1515` and `crates/nvs-runtime/src/array.rs:1552` are the two
      ctx-less writes. The pre-check goes in front of both the growth and the *separation*: a refusal
      that separated but did not write, or wrote into the shared original, is the one way this reaches
      `nvs_array_value_at`'s `.expect` on a live cursor. Test names the check wants:
      `an_array_set_past_the_ceiling_neither_writes_nor_separates` and
      `a_refused_write_leaves_a_live_foreach_cursor_on_its_own_snapshot`.
- [ ] **`nvs_array_append` and `nvs_array_spread` refuse through the status they already carry**
      — `crates/nvs-runtime/src/array.rs:1600` and `crates/nvs-runtime/src/array.rs:1666` are
      `(ctx, ..) -> i32` today, so neither needs a degenerate value and neither call site changes.
      Test name: `an_array_append_past_the_ceiling_refuses_through_the_status_it_already_has`.
- [ ] **The three cases, beside the four this session wrote** —
      `crates/nvs-runtime/tests/refusal.rs:194`. The cursor one is what says the degenerate return is
      sound rather than merely cheap, so write it against a `foreach` walking a snapshot, not against
      the counters.

## Backlog

- Stage 6, the accounting boundary and the bracketed cross-request stores — `crates/nvs-stdlib/src/cache.rs`.
- Stage 7's record and the rule fragments its `changes:` block names, `Reporting` among them — `docs/agent/loop-goal.md` § *Standing decisions*.
- `NvsStr::try_build`'s `None` can now carry a recorded breach; `crates/nvs-stdlib/src/str.rs:1764` still words it as a throw and is overridden in `run_helper` rather than at the source.
- `[context]` gap: the pack prints the goal's § *Standing decisions* but not its § *Stage N* item list, which is where the stage's numbered items live — the field selecting goal-prose sections should name the current stage's section too.
