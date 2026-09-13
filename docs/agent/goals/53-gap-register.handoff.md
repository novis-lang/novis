# Handoff

## State

**Goal 53 — one register reads every place a gap is written, and a milestone is an owner — has just started; nothing of it has landed yet.** Goal `plan-truth`'s whole list is this goal's Stage 1 floor.

Settled before the first session: **widen `tools/owners.py`, never write a second tool.** A milestone
owner means M9 or later (the user's rule, 2026-09-13); a past-milestone tag is *reported* here and made
fatal by goal `gap-zero`. `unowned` stays a legal kind until goal `unowned-closures` answers the user's
decision sheet.

## Next group

**Stage 2: one roster over every register** — one file set: `tools/owners.py`, `tools/holes.py`.

- [ ] **`--registers`** — `tools/owners.py:@collect` reads carried-refusals.md, the four ratchets,
      guard-name-debt.md and the playbook's `[until:]` bullets beside the module docs; last line
      `N register(s)`.
- [ ] **The past-milestone owner kind** — `tools/owners.py:@classify`, `:@milestones`; the new count
      `N item(s) owed by a past milestone` and `--past-is-an-error`.
- [ ] **`--deferrals`** — `tools/owners.py:@main`; each M9+ tag checked against `docs/plan/mN.md`.

## Backlog

- Stage 3, the ratchet test — `crates/nvs-stdlib/tests/spec_registry_coverage.rs:433`. Shares no file
  with stage 2.
- Stage 4, the heading sweep across module docs — the list is in the goal prose; re-run the sweep.
- Stage 5, `plan.py --past` and `done` — `tools/plan.py:@sync_schedule`, `:@run_check`.
- Stage 6, routing — `tools/brief.py:471`.
- When this goal's last check goes green the driver takes goal `m4-refusals`.
