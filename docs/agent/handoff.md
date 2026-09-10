# Handoff

## State

**Goal `gap-owners`, stage 4 "wired" is green.** Both of its checks' own `argv` run clean here:
`python tools/verify.py --list` prints nine steps and exits 0, and `python tools/brief.py --where
unowned` routes to the register and names its size. Stage 3 is unchanged — 147 tagged items in 69
blocks across 68 files, nothing untagged, nothing `unowned` without a reason.

**The order the gate walks now has one home.** `tools/verify.py:375`'s `steps_for` is it; `--list`
prints that list without spawning anything, and `-p`, `--fast` and `--doc` narrow the listing exactly
as far as they narrow a run. The hand-written copy of the order in `docs/agent/commands.md` was
already stale — it omitted `lints` and `extension` — and is gone rather than corrected.

**The register's count is measured, never typed.** `tools/brief.py:481`'s `ownership_line` imports
`owners` and classifies the tree: 106 of 147 tagged items, in 55 files, are `unowned` today. Only a
keyword that routes to `docs/agent/carried-gaps.md` § *Unowned* pays the third of a second it costs.

Nothing is blocked, and every stage of `docs/agent/loop-goal.toml` passes locally; the driver's own
acceptance sweep is what ends the run.

## Next group

**Stage 4: it stays true where the loop is not running** — one file set: `.github/workflows/ci.yml`,
`docs/agent/commands.md` and `tools/session.py`. Both items wire the gate this goal built into a
place that outlives the goal, since a check in `loop-goal.toml` stops running the day the goal
retires.

- [ ] **CI's `docs` job runs the ownership gate** — `.github/workflows/ci.yml:359` runs `rules.py
      --check` and seven siblings in a Python-only job, and `owners.py --check
      --untagged-is-an-error --reasons` is not one of them, so the register stays true only while
      this goal's stage-3 checks are live. Add the step beside them and name it in the list at
      `docs/agent/commands.md:263`, which is that job's one home; the goal's § *Standing decisions*
      ("the gate has no allowlist") is what it must enforce.
- [ ] **A wrap cannot commit a gap that names nobody** — `tools/session.py:880`'s gate families run
      `rules.py`, `records.py` and `check-migration.py` in-process, each only for a session that
      touched the tree feeding it, through `tools/session.py:953`'s `tree_gate`. The same shape with
      `crates` as the tree and `owners.py --check --untagged-is-an-error --reasons` as the gate
      refuses a wrap whose own edit left a `# Known gaps` item untagged, which is where that
      decision is cheapest. The scan is 0.3s, so the trigger can be any crate edit at all.

## Backlog

- `owners.py --json` carries each gap's whole lead paragraph, so a caller that wants only sizes reads
  megabytes; a `--counts` shape would serve it (tools/owners.py:457).
- `carried-gaps.md` § *Unowned* is 63 entries against 106 tagged items, because an entry names a file
  and a tag names an item — say so in that section's header (docs/agent/carried-gaps.md).
- Stage "4 suites" is the standing conformance/differential floor and owes nothing new
  (docs/agent/loop-goal.toml:6796).
