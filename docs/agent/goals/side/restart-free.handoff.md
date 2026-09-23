# Handoff

## State

**Side goal `restart-free`: a running server takes every code change without a restart, and every
config change it can.** ADR 0218 covers source revalidation (Stages 2 to 4) and ADR 0219
configuration apply (Stages 5 and 6). Stages 2 to 7 are complete on disk.

The driver's last acceptance run stopped on the floor check `dossier: lang:programs`, whose nine
perf figures were stale because `docs/reference/lang/10-programs.md` changed in d1fd418fe, a commit
from `main`, not from this goal. `dossier.py --record-perf` re-measured those nine and the twenty
other stale figures across the floor, and `--perf-report` rewrote `docs/perf/members.md`.
`dossier.py --verify --group lang:programs` is green.

The worktree needs two git-ignored things the main checkout holds: `tests/db/ca.crt`, copied in, and
`editors/vscode/node_modules`, installed with `npm ci`. Both are in the playbook, and both are in
place now.

Calls that are mine and not confirmed: the two Stage 7 checks re-scoped in
`docs/agent/goals/side/restart-free.toml`, the mark spelled `# default; restart required`, and the
reference saying a `[[server.mount]]` table resolves its links at boot.

The uncommitted `modules` block in `docs/agent/loop-goal.toml` is the driver's sweep, not this
goal's, and no session of this side run commits it.

## Next group

**Stage 7: the driver's acceptance run over the whole floor** — one file set:
`docs/agent/goals/side/restart-free.toml`.

- [x] **Every floor check passes** — `docs/agent/goals/side/restart-free.toml:1`; a perf figure made
      stale by a commit from `main` is re-measured with `python tools/dossier.py --record-perf`.

## Backlog

- None owned by this goal. A floor check that goes red on a perf figure a `main` commit made stale
  is closed by `dossier.py --record-perf`, which measures only what has no current figure.
