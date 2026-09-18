# Handoff

## State

**Goal `gap-zero` — stages 2, 3 and 5 are green, and stage 4 is the one hold.** `python
tools/owners.py --check` reads 24 gaps, every one owned by a milestone still ahead, and `--deferrals`
passes. `python tools/plan.py --past` reads 11 of 11 past milestones complete, and `--check` finds
every `Carried by` cell already agreeing with the chain, so `--sync` has nothing left to write; the
plan's *Done* field now says M0 through M8 rather than stopping at M4.

**Stage 4 is a `BLOCKED` on the user, exactly as `docs/agent/loop-goal.md:75` anticipated.** Every
run of `ci.yml` on `main` ends `failure` two seconds in with zero steps and no logs, and the
check-run annotation is GitHub's own sentence: *"The job was not started because recent account
payments have failed or your spending limit needs to be increased."* Nothing in the tree changes
that, and the goal's stage 4 prose forbids rewriting the check to pass.

**The three gates a goal meets only at its end are green as of this session**, so the session that
sees a green run writes `DONE` and nothing else: `python tools/verify.py --doc`, `python
tools/owners.py --closes gap-zero` and `python tools/playbook.py --closes gap-zero`.

## Next group

**Stage 4: CI green on `main`** — one file set: `docs/agent/loop-goal.toml`, `.github/workflows/ci.yml`.

- [ ] **Re-ask stage 4 once the block is lifted** — `docs/agent/loop-goal.toml:12058` is the whole
      check: `gh run list --branch main --workflow ci.yml --limit 1 --json conclusion` must read
      `"conclusion":"success"`. A run started after the block lifts is what turns it green, and no
      file in the tree is part of it.
- [ ] **Read the first run that actually executes, fix what it names, then close the goal** —
      `docs/agent/loop-goal.md:75` is stage 4's prose and the last thing the goal owes, the gates
      above being green already. No job has run since the block, so `.github/workflows/ci.yml`'s
      legs are unproven against every commit landed since; a red leg there is ordinary work and not
      a second `BLOCKED`.

## Backlog

- The goal prose's stage 3 list still names `owners.py`'s `unowned_paths` and
  `crates/nvs-stdlib/src/lib.rs:138`; both were already gone — `docs/agent/loop-goal.md` § *Stage 3*.
- `tools/playbook.py`'s `owned_rows`, `chain_retired` and `--closes` read a path that no longer
  exists, deliberately, so an index rebuilt there is gated — that module's doc says so.
- The plan's *Status* field still opens on M4's loop goal and the parity program; it is true and
  narrower than what *Done* now says — `docs/implementation-plan.md:12`.
- A handoff item about CI cannot be anchored: `.github` is not one of the roots
  `orient.ANCHOR_RE` accepts — `tools/orient.py:159`.
- When this goal's last check goes green the driver takes goal `dossier`.
