# Handoff

## State

**Goal `gap-zero` — no gap is owed by anyone but a future milestone, and the index that held them is
gone. Stages 2 and 3 are landed and green; stages 4 and 5 are what is left.** `unowned` is refused by
name in both halves of the register — `tools/owners.py`'s gate and `owner_problem` in
`crates/nvs-stdlib/tests/spec_registry_coverage.rs` — and `docs/agent/carried-gaps.md` is deleted,
with every reader re-pointed in the same slice. `python tools/owners.py --registers` now reads
`5 register(s)` and ends on the file's absence, which it asserts rather than merely not walking.

Stage 2's second half needed no work: `verify.py` already ran `owners.py --check` as its fifth step
and the gate already held whole, with `--untagged-is-an-error`, `--reasons` and `--past-is-an-error`
accepted and doing nothing.

Settled before the first session and unchanged: **this goal builds nothing.** If `python
tools/owners.py --check` or `python tools/plan.py --past` names an open item, a closure goal left it
behind — that is a `BLOCKED` naming the item, never a new owner or a deferral invented to pass. Both
read green today: 24 gaps, every one a milestone still ahead, and 11 of 11 past milestones complete.

## Next group

**Stage 5 and stage 4: the past milestones written down, then CI** — one file set:
`docs/implementation-plan.md`, `tools/plan.py`, `docs/agent/loop-goal.toml`.

- [ ] **`python tools/plan.py --sync` writes `done` into every past milestone's `Carried by` cell** —
      `tools/plan.py:506`'s `sync_schedule` derives that cell, and `--past` already reports all
      eleven complete, so this is the write rather than a decision.
- [ ] **The status block's *Done* field is rewritten whole to say M0–M8 are complete** —
      `docs/implementation-plan.md:19`, through a `## plan: Done` section of the wrap, not an edit.
- [ ] **Stage 4, CI green on `main`** — `docs/agent/loop-goal.toml:12053` is the check; `gh run
      list --branch main --workflow ci.yml` is what answers it. The goal expects this to be the one
      hold: if GitHub still refuses to start jobs, it is a `BLOCKED` naming the billing block and
      never a check rewritten to pass.

## Backlog

- The goal prose's stage 3 list still names `owners.py`'s `unowned_paths` and
  `crates/nvs-stdlib/src/lib.rs:138`; both were already gone — `docs/agent/loop-goal.md` § *Stage 3*.
- `tools/playbook.py`'s `owned_rows`, `chain_retired` and `--closes` read a path that no longer
  exists, deliberately, so an index rebuilt there is gated — that module's doc says so.
- When this goal's last check goes green the driver takes goal `dossier`.
