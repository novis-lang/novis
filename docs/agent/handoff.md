# Handoff

## State

**Goal `gap-owners` — a module doc's gap names its owner — has just started; nothing of it has landed yet.**
Goal `xml-tree`'s whole list is this goal's Stage 1 floor.

The finding this goal answers: **50 `# Known gaps` blocks across the crates hold 152 enumerated
items.** `docs/agent/carried-gaps.md` indexes 22 of them and `docs/agent/carried-refusals.md` covers
`nvs-ir`'s 15. The remaining ~110 name no owner, appear in no index, and are invisible to every tool —
the same failure `carried-gaps.md` was created to fix, one level down.

**Most of those 110 are not alarming, and that is the point.** The module docs are careful: an item
typically says where it is closed (`nvs-hir`'s visibility gap is closed in
`nvs_types::expr::members::check_member_visibility`), or names its blocker (`Core\Queue`'s two wait on
a shape spelling), or explains why it is inert (`nvs_safepoint`'s two cleared flags wait on `rule:security/arena-is-an-ownership-root`'s
collector and on `nvs dap`). What is missing is not care — it is a **machine-readable owner**, so the
short list of real scheduling questions can be told from the long list of explained residue without
reading 50 module docs.

## Next group

**Stage 2: the tool and the gate** — one file set: the new `tools/owners.py`, `tools/verify.py`,
`tools/chain.py`.

- [ ] **The owner tag lives with the gap**, one per enumerated item: `— owner: 21`, `— owner: M9`, or
      `— owner: unowned`. It sits in the module because a gap's home is its module, and the index is
      then *derived* — the rule `holes.py` already follows ("the list is derived, never copied").
- [ ] **Three owner kinds and no fourth** — a live goal; a **future milestone** whose plan states
      the scope; or `unowned` with a bullet in `carried-gaps.md` § *Unowned* carrying the reason. The
      milestone kind is what stops scheduled work being counted as an unclosed gap.
- [ ] **`tools/owners.py`**, modelled on `holes.py --unattributed`: prints owned-by-goal,
      deferred-to-milestone, unowned, and — the interesting output — **untagged**. `--unowned`,
      `--check`, `--json`.
- [ ] **The gate fails** on an untagged item, a goal not on the chain, a milestone that does not exist,
      or `unowned` with no reason behind it. **No allowlist**, and adding one is the move it forbids.

## Backlog

- **Stage 3 (the attribution pass)** is the judgement, module by module. Known deferrals so nobody
  re-derives them: `nvs-cli/src/bundle.rs`'s `.nvsx` embedding is M9's; the inlining items in
  `nvs-runtime/src/decimal.rs` and the string fast path in `nvs-runtime/src/lib.rs` are M12's. Known
  non-gaps that move *out* of the block rather than getting a tag: `time.rs`'s "there is not going to
  be one" and `casing.rs`'s "left out deliberately" — a decision is not a gap.
- **One stale entry to settle**: `nvs-stdlib/src/router.rs` gap 3 says `Core\Router::match` is absent
  while the plan's *Open now* says a request is matched. One of the two is wrong.
- **Stage 4 (it stays true)** wires the gate into `verify.py` and the unowned count into the
  orientation pack. The tag survives a goal switch — which is the mechanism that orphaned all of this —
  and that property is why the module doc is the right home.
