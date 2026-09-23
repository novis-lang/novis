# Handoff

## State

**Side goal `restart-free` is met: a running server takes every code change without a restart, and
every config change it can.** ADR 0218 covers source revalidation (Stages 2 to 4) and ADR 0219
configuration apply (Stages 5 and 6). Stages 2 to 7 are complete on disk. The driver's last
acceptance run passed whole.

The goal-end gates are clean. `verify.py --doc` resolves every link. `owners.py --closes
restart-free` names no module-doc gap. `playbook.py --closes restart-free` names no carried-gaps row.

Two landings were held by an uncommitted `modules` block in `docs/agent/loop-goal.toml`. The
driver's own context sweep wrote it: `tools/loop.py` `context_sweep` did not pass `--goal`, so a side
run widened the chain's manifest. That call now passes the side goal's toml, and the stray edit is
reverted. The worktree is clean.

Calls that are mine and not confirmed: the two Stage 7 checks re-scoped in
`docs/agent/goals/side/restart-free.toml`, the mark spelled `# default; restart required`, and the
reference saying a `[[server.mount]]` table resolves its links at boot.

## Next group

**Stage 7: the driver lands the branch on `main`** — one file set:
`docs/agent/goals/side/restart-free.toml`.

- [x] **The goal is met and the worktree is clean**, so the driver can land it —
      `tools/loop.py:6518` passes `--goal`.

## Backlog

- A driver process that loaded `tools/loop.py` before this fix still runs the old sweep until it restarts (`tools/loop.py` `context_sweep`).
