# Handoff

## State

**Side goal `restart-free`: a running server takes every code change without a restart, and every
config change it can.** ADR 0218 covers source revalidation (Stages 2 to 4) and ADR 0219
configuration apply (Stages 5 and 6). Stages 2 to 7 are complete on disk, and nothing in the goal's
own list is open.

Both floor checks this branch's renames held red are re-pointed by the user: `nvs-config (the
validate default)` in `6bcbecaac`, and `nvs-config (four rows, System throughout, every key Reload)`
in `866ed6c3c`. The four tests the second one names pass on this tree. `owners.py --closes
restart-free`, `playbook.py --closes restart-free` and `verify.py --doc` are all green, so this
session wrote `DONE`.

The floor's program and command checks after those two had not run on this branch before this
session. If the driver's acceptance run stops on one, it is the next item: a static scan of every
`cargo-named` name found no other test this branch renamed.

The worktree needs two git-ignored things the main checkout holds: `tests/db/ca.crt`, copied in, and
`editors/vscode/node_modules`, installed with `npm ci` for the `vscode (headless)` floor check. Both
are in the playbook, and both are in place now.

Calls that are mine and not confirmed: the two Stage 7 checks re-scoped in
`docs/agent/goals/side/restart-free.toml`, the mark spelled `# default; restart required`, and the
reference saying a `[[server.mount]]` table resolves its links at boot.

The uncommitted `modules` block in `docs/agent/loop-goal.toml` is the driver's sweep, not this
goal's, and no session of this side run commits it.

## Next group

**Stage 7: the driver's acceptance run over the whole floor** — one file set:
`docs/agent/goals/side/restart-free.toml`.

- [x] **The `[queue]` floor check names the renamed tests** — re-pointed by the user in `866ed6c3c`;
      `every_queue_key_reloads` at `crates/nvs-config/tests/directives.rs:2040` and
      `a_reload_that_changes_workers_publishes_the_new_count` at
      `crates/nvs-config/tests/snapshot.rs:392` pass. `rule:config/reloadability-is-its-own-field`.

## Backlog

- A floor program or command check red on this branch alone: triage it against the renames in
  ADR 0218 and 0219, and fix the code, never the check.
