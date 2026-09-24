# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 to 7 have landed, Stage 4's records are valid, and Stage 8's
checks are green. Stage 8's prose still owes `nv goal context --add` and the three changes that wait
for the cutover.** `bun nv check` prints `nv check: 0 findings`, and `bun nv render --check` is current.
The driver's red check is Stage 9's `bun nv audit goals`, which is the cutover and is not started.

`liveGoal` reads the driver's pointer by slug (`tools/nv/lib/state.ts`). The pointer is
`.loop/state.sqlite`'s `pointer` row when that file exists. Until the cutover it is the Python driver's
`.loop/chain.json`, whose number resolves through the goal file named `N-<slug>.md`. The H1 match
against `docs/agent/loop-goal.md` is the fallback for a tree with no pointer. `writePointer` exists
for the new driver, and nothing calls it yet. Cutover step 5 ("`.loop/chain.json` becomes the state
database's pointer") is `writePointer(legacyPointer())`.

Two items of the last group wait for the cutover, because the running Python driver still reads what
they would drop. `manifestCopies` (`tools/nv/cmd/session.ts:387`) gates `docs/agent/loop-goal.toml`
beside the record, and that toml is the driver's acceptance list. `recordPack`
(`tools/nv/cmd/session.ts:1512`) measures `python tools/orient.py`, the pack the driver pipes. Each
switches in the cutover commit.

`bun nv session --wrap` validates and applies a wrap by itself. Every playbook record has the same
`lead`, `body` and `until` as its fragment file. Eight differ in `files` alone, where the record is
right; `nv import --write` still rewrites those eight, and the bullet
`docs/agent/playbook/tooling/bun-nv-import-write-rewrites-a-data-playbook-bullet-back-to.md` says
what to do.

`tools/orient.py`, `chain.py`, `session.py`, `playbook.py` and `tools/verify_keys.py` stay until the
Stage 9 cutover. `main` is frozen. Tag `pre-overhaul` is the rollback. An edit to a bullet goes into
both playbook homes until Stage 9, and a wrap does that by itself.

These habits hold for every session of this goal. A mechanical change goes through a script under
`.agent-tmp/`, written with Write. Never prove a cut with a sweep. A Python tool is deleted only after
its replacement's parity is green. `bun x tsc --noEmit -p .` typechecks the tools, and `bun nv
selftest` runs every tools test. A scratch worktree needs `target/debug/nvs.exe` copied in and a
junction to `node_modules`. It has none of the git-ignored fixtures. Code copied into it is
uncommitted there, so a `git reset --hard` in the worktree silently puts the old tools back. Wrap
with `bun nv session --wrap`, not the pack's `python tools/session.py --wrap`: only the first also
writes `data/goals/tooling-overhaul.handoff.json`.

## Next group

**Stage 8: a session widens its own manifest** — one file set: a new `tools/nv/cmd/goal.ts`,
`tools/nv/schema/goal.ts`, `tools/nv/lib/chain.ts`. loop-goal.md § *Stage 8* specifies it: "A session
adds a module to `[context]` with `nv goal context --add <path>`."

- [ ] **`nv goal context --add <path>`.** It adds the path to the live goal's record `context.modules`
      (`tools/nv/schema/goal.ts:11`), the goal `liveGoal` names (`tools/nv/lib/chain.ts:70`). While
      `docs/agent/loop-goal.toml` exists the Python driver reads its `[context]`, so the command also
      adds the path to that file's `modules` array as a text edit, never a re-serialisation of the whole
      file. It refuses a path that is not on disk, and a path already listed is a no-op.
- [ ] **A test and the process docs.** `tools/nv/test/goal.test.ts` over a scratch root, and the session
      prompt's "edit the `[context]` field" line names the command instead
      (`docs/agent/session-prompt.md:22`).

## Backlog

- `bun nv playbook` answers `--show` and `--retire` only. `--check`, `--match`, `--manifest` and
  `--dupes` are still `tools/playbook.py`'s (loop-goal.md § *Stage 8*).
- At the cutover: `manifestCopies` gates the record alone and `pruneManifests` drops `GOAL_TOML`
  (`tools/nv/cmd/playbook.ts:37`); `recordPack` measures `bun tools/nv/main.ts orient`; the pointer
  moves into `.loop/state.sqlite` (loop-goal.md § *Stage 9*).
- The floor as the view "every check of every walked goal" has no query yet (loop-goal.md § *Stage 8*).
