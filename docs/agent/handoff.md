# Handoff

## State

**Goal `gap-zero` — every stage that carries a check is green, and nothing holds it.** `python
tools/owners.py --check` reads 24 gaps, every one owned by a milestone still ahead, and `--deferrals`
passes. `python tools/plan.py --past` reads 11 of 11 past milestones complete, and `--check` finds
every `Carried by` cell already agreeing with the chain, so `--sync` has nothing left to write; the
plan's *Done* field says M0 through M8.

**Stage 4 carries no check, by the user's decision of 2026-09-18.** GitHub starts no CI job while the
account's payments fail, so the CI check moved whole to goal `ci-green`, which is pinned last on the
chain behind every generated goal — `docs/agent/loop-goal.md` § *Stage 4* says so. A session here
neither asks `gh` nor reports a `BLOCKED` about CI.

**The three gates a goal meets only at its end were green when last asked**: `python tools/verify.py
--doc`, `python tools/owners.py --closes gap-zero` and `python tools/playbook.py --closes gap-zero`.

## Next group

**Stage 3 and stage 5: one red check, then close the goal** — one file set: `docs/agent/playbook.md`,
`tools/playbook.py`, `docs/agent/loop-goal.toml`.

- [ ] **Turn *the playbook's own checks survive the file that is gone* green** — `python
      tools/playbook.py --check` lists bullet *Tooling > check-links.py reads* under § *PATHS A
      BULLET NAMES THAT ARE NOT IN THE TREE*: it cites `docs/agent/carried-gaps.md`, which stage 3
      deleted. Read the bullet; it is rewritten, pruned, or held in `tools/playbook.py`'s
      `DELIBERATE_STALE` if the missing path is the trap.
- [ ] **Confirm the list is green and write `DONE`** — `docs/agent/loop-goal.toml:12058` onward is
      what is left of this goal's own checks; the three end gates above are the rest. Nothing is
      owed to the tree, so a session that finds them green writes `DONE` and nothing else.

## Backlog

- The goal prose's stage 3 list still names `owners.py`'s `unowned_paths` and
  `crates/nvs-stdlib/src/lib.rs:138`; both were already gone — `docs/agent/loop-goal.md` § *Stage 3*.
- `tools/playbook.py`'s `owned_rows`, `chain_retired` and `--closes` read a path that no longer
  exists, deliberately, so an index rebuilt there is gated — that module's doc says so.
- The plan's *Status* field still opens on M4's loop goal and the parity program; it is true and
  narrower than what *Done* now says — `docs/implementation-plan.md:12`.
- When this goal's last check goes green the driver takes goal `dossier`.
