# Handoff

## State

**Goal `gap-owners` — a module doc's gap names its owner. Stage 2 is landed and green; stages 3 and
4 are open.** `tools/owners.py` walks every `# Known gaps` block in `crates/*/src/**` and derives
the roster: **163 items in 58 blocks across 58 files**, of which 3 are tagged (one per kind) and 160
name nobody. That count is the tool's to state, not a doc's.

The tag is `— owner: <goal slug | milestone | unowned>` on a line of its own at the item's end. **A
slug, never a chain number** — a number is a position and moves the moment anything is inserted
ahead of it; the goal file said `owner: 21` and both copies are corrected, as is
`carried-gaps.md`'s contract.

**The gate arrives in three pieces on purpose**, because it is built before the pass it gates:
`--check` alone refuses a tag that resolves to nothing, `--untagged-is-an-error` adds the items that
name nobody, `--reasons` adds the `unowned` ones with no `carried-gaps.md` bullet. The full form is
what stage 4 wires into `verify.py` — wiring it now would turn every unrelated session red, which is
what buys a gate its first allowlist. The goal's own `2 the tool` checks (`loop-goal.toml:6730`)
pass today.

## Next group

**Stage 3: the attribution pass, module by module** — one file set: `crates/nvs-runtime/src/lib.rs`,
`crates/nvs-ir/src/lib.rs`, `crates/nvs-stdlib/src/router.rs`, `docs/agent/carried-gaps.md`.

- [ ] **Settle what a `carried-refusals.md` entry counts as**, before tagging
      `crates/nvs-ir/src/lib.rs:185`'s block — the largest single one, and already carried as entry
      901 there. Entries in that file are numbered 900+ and are none of the three kinds the goal's
      § *Standing decisions* allows, so either they are `unowned` with the reason living in
      `carried-refusals.md` rather than in `carried-gaps.md` § *Unowned* (and `owners.py`'s
      `unowned_paths` learns to read both files), or the block takes the owner of the goal that
      closes the refusals. Write the reading into the goal's § *Standing decisions*; it is the one
      question the rest of the pass repeats.
- [ ] **`crates/nvs-runtime/src/lib.rs:188`'s seven items**, five of which the docs already decide:
      item 2 → `M12` and item 3 is a "Not a gap" paragraph that moves out of the block (both in
      `docs/agent/loop-goal.md` § *Stage 3*), item 4 → `unowned-sweep` and item 7 → `unowned` (both
      in `carried-gaps.md`). Item 5 names two owners in one sentence — `COLLECT` waits on the
      collector, `DEBUG_BREAK` on `nvs dap` — so it splits into two items with a tag each. Items 1
      and 6 are the judgement.
- [ ] **`crates/nvs-stdlib/src/router.rs:40`'s gap 3 is closed or tagged, not both** — it says
      `Core\Router::match` is absent while the plan's *Open now* says a request is matched. Goal md
      § *Stage 3* item 4 is the instruction; finding out which of the two is wrong is the work.

## Backlog

- Stage 4: the full gate into `tools/verify.py`, and `brief.py --where unowned`; the goal's
  `4 wired` checks are at `docs/agent/loop-goal.toml:6774`.
- `[context] modules` names six paths that are not crate modules (`tools/*.py`,
  `docs/agent/carried-gaps.md`), and `orient.py` warns once per session for each; the field only
  matches a `//!` module doc.
- `carried-gaps.md` § *Owned* names seven retired goals; `owners.py` buckets a retired owner as a
  finding rather than a failure, the same way `tools/playbook.py --check` already does.
- The goal md's opening count (50 blocks, 152 items) counts a different thing from the tool, which
  also counts `- ` bullets and one-gap prose blocks. Left alone; `owners.py` is the home now.
