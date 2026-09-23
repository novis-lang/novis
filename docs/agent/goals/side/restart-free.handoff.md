# Handoff

## State

**Side goal `restart-free`: a running server takes every code change without a restart, and every
config change it can.** ADR 0218 covers source revalidation (Stages 2 to 4) and ADR 0219
configuration apply (Stages 5 and 6). Stages 2 to 7 are complete on disk.

The driver's last acceptance run stopped on the floor check `dossier: the chain already names every
emitted goal`. The cause was the three `Boot` rows this branch added (`directive:server.listen`,
`directive:server.socket_mode`, `directive:server.workers`). Each was a new feature with no example,
no attack and no credited test, so the emitter wanted to append a goal for them. They now have an
example, an attack, an `about.md` and a `covers:` marker. `dossier.py --emit-goals --dry-run` prints
`nothing appended`, and `--check-goals` is green. `verify.py`, `verify.py --doc`, `owners.py
--closes restart-free` and `playbook.py --closes restart-free` are green, so this session wrote
`DONE`.

The same slice rewrote the published text that still said `[server]`, `[queue] connection`,
`[queue] workers` and `[control] socket` need a restart. Those are the `about.md` files and
examples under `docs/examples/config/{server,queue-workers,queue-connection,control-socket}/`, plus
one step comment in `tests/hostile/config/server/`.

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

- [x] **The three `Boot` rows owe no proof** — their test is
      `crates/nvs-config/tests/snapshot.rs:481`. If the driver's acceptance run stops on a later
      floor check, that check is the next item: the checks after `dossier: the chain already names
      every emitted goal` have not run on this branch yet.

## Backlog
- `nvs agent find socket_mode` finds nothing: no configuration key has a help entry yet. The Help
  proof for directives is backfilled by goal `core-class-cards` (ADR 0216).
- `python tools/dossier.py --group config:directives` raises `KeyError: 'help'` in `print_group`
  (`tools/dossier.py:1990`). `--id` and `--verify --only` work.
