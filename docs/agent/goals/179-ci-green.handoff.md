# Handoff

## State

**Goal 179 — CI is green on `main` — has just started; nothing of it has landed yet.** The last
generated goal's whole list is this goal's Stage 1 floor.

This was goal `gap-zero`'s stage 4, moved to the end of the chain by the user on 2026-09-18 because
GitHub starts no job while the account's payments fail. The loop never pushes, so `origin/main` is
as far behind as the run is long, and the first answer here is expected to be a `BLOCKED` asking the
user to push `main` — or naming the billing block, if it is still in force.

## Next group

**Stage 2: the run exists, and it is this commit's** — one file set: `tools/ci-green.py`, `tools/ci-changes.py`.

- [ ] **Ask the check** — `tools/ci-green.py:@main` says which of the three it is: no green run, a
      green run for older code (a `BLOCKED` asking for the push, naming the commit), or green.
- [ ] **A run that ends `failure` in seconds with zero steps is the billing block** — `gh run view`
      on its URL shows the annotation; that is a `BLOCKED` naming it, never a fix in the tree.

## Backlog

- Stage 3: a leg that really ran and went red is ordinary work — `gh run view --log-failed`, fix,
  ask for the push again. File set: whatever the log names, plus `tools/ci-changes.py`.
- When this goal's last check goes green the chain is walked.
