# Handoff

## State

**Side goal `restart-free` is met: a running server takes every code change without a restart, and
every config change it can.** ADR 0218 covers source revalidation (Stages 2 to 4) and ADR 0219
configuration apply (Stages 5 and 6). Stages 2 to 7 are complete on disk. The driver's last
acceptance run passed whole.

The goal-end gates are clean. `verify.py --doc` resolves every link. `owners.py --closes
restart-free` names no module-doc gap. `playbook.py --closes restart-free` names no carried-gaps row.

Calls that are mine and not confirmed: the two Stage 7 checks re-scoped in
`docs/agent/goals/side/restart-free.toml`, the mark spelled `# default; restart required`, and the
reference saying a `[[server.mount]]` table resolves its links at boot.

The uncommitted `modules` block in `docs/agent/loop-goal.toml` is the driver's sweep, not this
goal's, and no session of this side run commits it.

## Next group

**Stage 7: the driver lands the branch on `main`** — one file set:
`docs/agent/goals/side/restart-free.toml`.

- [x] **The goal is green, and the goal-end gates are clean** — the driver rebases, verifies and
      fast-forwards `side/restart-free` onto `main`, and retires the goal, anchored at
      `docs/agent/goals/side/restart-free.toml:1`.

## Backlog

- Confirm the three unconfirmed calls in ## State with the user — owner: `docs/agent/goals/side/restart-free.md`.
