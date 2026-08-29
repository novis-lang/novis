# Handoff

## State

**Stage 6's item 20 is closed end to end, and the `nvs-ir` acceptance check that named two missing tests is closed with it.** `spawn script` and `await` lower to two `InstKind::CoreCall`s; `crates/nvs-ir/src/lower/tests.rs:3819` and `:3921` are the two names the check asks for, written as structural assertions over the emitted IR rather than snapshots.

**The isolate boundary now asks ADR 0023 § 2's third question of the answer.** `nvs_runtime::graph::copy_graph_into` takes the receiving side's class table; `Live::admit` is the rule's one home and states it as descriptor *identity*, not name resolution, with the cost of that spelled out there. `Ctx::class_table` is how a parent's table is taken at the spawn and used at the join. **Known gap, in `isolate.rs`'s module doc:** the argument going *in* is not asked the same question, because the child's table does not exist until its program's prologue installs it — closing it is a change to the `nvs_runtime::script` seam.

**Six of Stage 6's eight `cargo-named` names are green**; the two below are what is left.

**Orientation gaps, unchanged and still real:** `[context] adrs` prints ADR 0023 § 2, ADR 0072 §§ 4-5 and ADR 0106 § 6 — ADR 0006's `## Decision` still has to be sliced by hand, and ADR 0106 § 2 (which item 2 of the last group was written against) is not printed either. `[context] modules` has no pattern for `nvs-runtime/src/graph.rs`, which this session edited.

## Next group

**The last two Stage 6 `cargo-named` names, both about cancellation.** They share `crates/nvs-host/src/isolate.rs` and `crates/nvs-host/src/scheduler.rs`; the join's own cancel loop is `isolate.rs:243` and `cancel_task` is `scheduler.rs:1102`.

- [ ] **`a_cancelled_parent_leaves_no_orphan_and_no_leaked_arena`** — ADR 0072 § 5. Cancel the task parked in `Started::join` (`isolate.rs:191`, the loop at `:236`) and assert the child's task is gone and its `Ctx` dropped: `Ended` (`isolate.rs:272`) is what fires on the unwind, and `a_contained_panic_in_a_child_leaves_the_parent_running` is the sibling whose `run` helper already drives a real `Scheduler`.
- [ ] **`a_child_is_cancelled_at_its_next_safepoint`** — ADR 0072 § 5 from the other side: a child that is *running* dies at its next safepoint rather than being unwound from the parent's frame. `scheduler.rs`'s module doc § *The task tree, and what cancelling one costs* is the mechanism, and `Ctx::cancelled` is what the child's own body reads.

## Backlog

- The argument-in direction of the unresolvable-class rule — `isolate.rs`'s module doc owns the gap and names the seam change it needs.
- `nvs_runtime::graph::walk` returns `Err` from its refusal arms without releasing the value it was handed, while its depth check releases; whether the root's reference is the caller's on the error path is the IR's convention (the landing block releases it), but the nested-child arms look inconsistent — worth one valgrind pass over `Core\Serialize::encode` of a refused nested value.
- ADR 0006's `$result->valueOrThrow()` is `Core\Script::valueOrThrow($result)` and the member does not exist yet — item 22, `docs/plan/m5.md`.
- `Core\Secret::reveal()` is still absent, so ADR 0033's escape hatch is open at both ends — item 18.
- The 1000-case corpus count is M4's residue, met as the suite grows — `docs/implementation-plan.md`.
