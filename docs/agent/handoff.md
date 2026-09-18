# Handoff

## State

**Goal `bigint` — `Core\BigInt` — is complete through stage 4.** The class, its 24 members, six
`bigint-*.nvst` cases and `examples/bigint.nvs` are on disk, and the three guard tests the acceptance
list names live in `crates/nvs-stdlib/src/bigint.rs`'s own `mod tests`.

**`Core\Arr::sort` now orders two `Comparable` objects**, and so do `Core\Arr::min`/`max`,
`Core\Math::min`/`max`/`clamp` and a comparator-less `Core\Heap`: `crate::ordering::compare_values`
took a `&mut Ctx` and grew one last row that asks `nvs_runtime::call_compare_to`, which is machinery
`heap.rs` already had and nothing else had reached for. `heap.rs`'s own object branch and its
`sign_of` are gone into that one home. `rule:classes/comparable` is where the decision is recorded.

**The live `docs/agent/loop-goal.*` is still goal `test-doubles`** — the driver holds goal 68 open on
a DONE-claim retry, so goal `bigint`'s checks are not the ones it runs yet, and this session fixed
only `docs/agent/goals/69-bigint.toml`. Its stage-4 `cases` were drafted names; all four claims were
on disk under the names their author chose, and the list is repointed.

**The floor's `abi-probe` failure is load, not a regression** — see the playbook bullet.

## Next group

**Goal `bigint`, stage 5 — the gate** — one file set: `crates/nvs-stdlib/src/ordering.rs`,
`tests/conformance/core/`, `docs/agent/goals/69-bigint.toml`.

- [ ] **One case pinning the natural ordering over objects everywhere it is now read** — `Core\Arr::min`
      and `max` and `Core\Math::min`, `max` and `clamp` over `Core\BigInt`, agreeing with `compareTo`
      by counting, and a class with no `compareTo` still throwing. The row is
      `crates/nvs-stdlib/src/ordering.rs:47` and the rule is `rule:classes/comparable`; the sort half
      is already pinned in
      `tests/conformance/core/bigint-orders-through-compare-to-and-every-spelling-of-it-agrees.nvst:30`.
- [ ] **Add that case to the goal's stage-4 check and run the goal's end gates** —
      `docs/agent/goals/69-bigint.toml:139` is the `cases` list, then
      `python tools/owners.py --closes bigint`, `python tools/playbook.py --closes bigint` and
      `python tools/verify.py --doc`, closing or re-ownering every gap they name.
- [ ] **Claim the goal if both are clean** — every other stage is green;
      `docs/agent/goals/69-bigint.toml:166` is the gate block, and nothing in it is unwritten work.

## Backlog

- The five-driver matrix needs a warm docker daemon; a cold boot fails the floor — `docs/agent/goals/69-bigint.toml:88`.
- `Core\Bytes` still has no natural-order row, deliberately — `crates/nvs-stdlib/src/ordering.rs:40`.
- Goal `gap-zero`'s ratchet half waits only on the chain reaching it — `docs/implementation-plan.md` § *Blocking*.
