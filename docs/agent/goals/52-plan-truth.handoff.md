# Handoff

## State

**Goal 52 — every plan file and module doc says what the tree does — has just started; nothing of it has landed yet.** Goal `websocket-client`'s whole list is this goal's Stage 1 floor.

Settled before the first session: this is the first goal of the gap program the user set on 2026-09-13
(M0–M8 complete, only M9+ deferrals). It changes text, not behaviour. The stale sentences in the goal
prose were found by an audit and are **anchors to re-check**, not a list to apply blind.

## Next group

**Stage 2 and stage 3: the stale-sentence lint, then the plan files** — one file set: `tools/plan.py`,
`docs/plan/`, `docs/implementation-plan.md`.

- [ ] **`plan.py --stale`** — `tools/plan.py:@main`, reusing `tools/plan.py:@live_goal` and
      `:@chain_goals`; its last line is `sentences deferring to a walked goal: N`.
- [ ] **M1–M4S plan files** — `docs/plan/m1.md:3`, `docs/plan/m2.md:40`, `docs/plan/m3.md:21`,
      `docs/plan/m4.md:3`, `docs/plan/m4s.md:3`, each rewritten whole to what the tree does.
- [ ] **M5–M8 plan files and the status block** — `docs/plan/m5.md:22`, `docs/plan/m6.md:5`,
      `docs/plan/m7.md:1`, `docs/plan/m8.md:1`, `docs/implementation-plan.md:19`.

## Backlog

- Stage 4, the module docs — `crates/nvs-cli/src/serve.rs:79`, `crates/nvs-syntax/src/lib.rs:57`, the six
  retired-owner items `python tools/owners.py` lists. Shares no file with stage 3.
- Stage 5, `docs/agent/carried-gaps.md` § *Owned* — `python tools/playbook.py --check` is the worklist.
- When this goal's last check goes green the driver takes goal `gap-register`.
