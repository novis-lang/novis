# Handoff

## State

**Goal `resource-ceilings` — stage 5 is landed on both halves.** The string half was the session
before this one; the array half is now in: `nvs_array_set`, `nvs_array_set_index`, `nvs_array_append`
and `nvs_array_spread` all ask the ceiling in front of the allocation. All three of stage 5's named
array cases are green (`cargo test -p nvs-runtime --test refusal`: 9 passed), and so is the stage 1
floor that had been failing — `crates/nvs-cli/src/serve.rs`'s known gap now names `— owner: M6`,
which is the milestone whose acceptance list already carries the config snapshot a served request
reads.

**`NvsArray::affords_write` (`crates/nvs-runtime/src/array.rs:875`) is the pre-check**, and it prices
one write rather than the whole array: `Table::growth_cost` is what the entry storage's amortized
doubling asks for and `0` while it has room, `Table::separation_cost` is what a copy-on-write copy
asks for, and a shared handle is charged the copy *and* one growth of it because `Table::separate`
reserves the live entries exactly. The ask is made whatever it costs, zero included — a balance
already past the ceiling is a request that is over.

**The refusal has two shapes, one per signature.** A ctx-less write answers its array unchanged and
releases the key and value it was handed (`nvs_array_set`'s key is the reference a leak would hide
in). A write that already carries a status answers `crate::FATAL` through
`crate::abi::report_refusal` (`crates/nvs-runtime/src/abi.rs:512`), which is `run_helper`'s own two
lines — the tier-1 handler, then the breach as a fault no `catch` sees — reached from a helper with
no `Fault` to hand back. `nvs_array_spread` asks per entry, so a large subject is bounded entry by
entry and stops partway with what it had already copied.

Stages 6 and 7 are untouched, and the goal's own record is still unopened — `0174` was the next free
number at this commit; re-derive it.

## Next group

**Stage 6: the detached-accounting bracket** — one file set, `crates/nvs-runtime/src/budget.rs` with
`crates/nvs-runtime/tests/` for the cases, then `crates/nvs-stdlib/src/cache.rs` for the first store
that takes it. `rule:programs/memory-priority`'s *bounded, attributable* is what it owes, and the
goal's § *Standing decisions* settles the scope: bracket the known stores, do not solve provenance,
and record in the goal's record that the general property waits for M6's arena.

- [ ] **The bracket itself, in `crates/nvs-runtime/src/budget.rs:201`** — a guard type shaped like
      `Reporting` beside it (`crates/nvs-runtime/src/budget.rs:498` is the `add` it has to divert):
      allocations and frees inside it move the process counters and not the running request's
      measured usage, and `Drop` restores, so the obligation arrives with the shape rather than
      being one to remember. Test names the check wants:
      `a_detached_bracket_moves_the_process_counter_and_not_the_request_reading`,
      `a_bracket_restores_on_unwind_as_well_as_on_return` and
      `a_request_that_frees_inherited_memory_gains_no_ceiling`.
- [ ] **`Core\Cache::local`'s store takes it** — `crates/nvs-stdlib/src/cache.rs:477` is the
      `thread_local` map and `crates/nvs-stdlib/src/cache.rs:582` the second one beside it; the
      entries must still count against `[cache.local] max_size`
      (`crates/nvs-stdlib/src/cache.rs:501`), which is a different ceiling from the request's. Test
      names: `a_cache_write_is_charged_to_the_process_and_not_to_the_request`,
      `an_eviction_from_a_later_request_lowers_no_ceiling` and
      `a_cache_entry_still_counts_against_the_local_tier_max_size`.

## Backlog

- Stage 7's record and rules — the refusal and its degenerate return, the accounting boundary, the
  expansion rule; the goal's § *Standing decisions* is the brief, `docs/agent/conventions.md` the shape.
- Any other process-lifetime store a session finds unbracketed: bracket it and say so, per the goal's
  standing decision on stage 6.
- `NvsArray::set`/`set_index` (the Rust API the `Core` producers use) stay infallible on purpose —
  `run_helper` asks ahead of a member's body, and `try_reserve` is the fallible seam a producer takes.
