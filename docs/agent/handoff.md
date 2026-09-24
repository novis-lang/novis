# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 to 8 have landed, and Stage 9's `bun nv audit` is on disk.** What is
left is the cutover itself: the switches and the one cutover commit. `bun nv audit eol` is already
green: the index holds no CRLF file. `bun nv audit goals` and `bun nv audit checks` are red, and that is
right until the cutover lands. Each names every offender; their counts today are hundreds of numbered
goal files and tracked copies, and the floor `argv`s that run Python. `bun nv check` printed `nv check: 0
findings` last session, and `bun nv render --check` was current.

`nv audit checks` reads the checks from every goal and side-goal record under `data/goals/`, plus
`docs/agent/loop-goal.toml` while that file exists. The old name of the feature proofs counts in a
check's name or `argv`, in a `//` or `#` directive line of a tracked `.nvs`, `.nvst` or `.rs`, and in any
tracked file under `tools/` by path or by a line of text. `tools/nv/cmd/audit.ts` and its test build the
word from two halves, so neither is its own offender. Prose outside `tools/` is not scanned: decision
records are frozen and mention it.

`liveGoal` reads the driver's pointer by slug (`tools/nv/lib/state.ts`). The pointer is
`.loop/state.sqlite`'s `pointer` row when that file exists. Until the cutover it is the Python driver's
`.loop/chain.json`. Cutover step 5 ("`.loop/chain.json` becomes the state database's pointer") is
`writePointer(legacyPointer())`.

Three changes wait for the cutover, because the running Python driver still reads what they would drop.
`manifestCopies` (`tools/nv/cmd/session.ts:387`) gates `docs/agent/loop-goal.toml` beside the record,
and that toml is the driver's acceptance list. `recordPack` (`tools/nv/cmd/session.ts:1512`) measures
`python tools/orient.py`, the pack the driver pipes. `nv goal context --add` edits
`docs/agent/loop-goal.toml` while it exists, and does nothing to it once the cutover deletes it.

Eight playbook records differ from their fragment in `files` alone, where the record is right; `nv
import --write` still rewrites those eight, and the bullet
`docs/agent/playbook/tooling/bun-nv-import-write-rewrites-a-data-playbook-bullet-back-to.md` says what
to do.

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
writes `data/goals/tooling-overhaul.handoff.json`. The guard hook refuses any shell command whose text
names `docs/agent/loop-goal.toml` beside a `grep`, even in an unrelated part of the command.

## Next group

**Stage 9: the cutover** — one file set: `tools/nv/cmd/session.ts`, `tools/nv/lib/state.ts`,
`tools/nv/cmd/audit.ts`, and the `.agent-tmp/` scripts the cutover runs. loop-goal.md § *Stage 9* is the
specification, and `bun nv audit` is its acceptance.

- [ ] **The cutover switches.** `manifestCopies` (`tools/nv/cmd/session.ts:387`) stops gating
      `docs/agent/loop-goal.toml`, `recordPack` (`tools/nv/cmd/session.ts:1512`) measures `bun nv
      orient`, and step 5 is `writePointer(legacyPointer())` (`tools/nv/lib/state.ts:49`).
- [ ] **The cutover commit**, `docs/agent/loop-goal.md:553` steps 1 to 7, in one slice, with `bun nv loop
      --goal-only` green under the new driver before it commits. `bun nv audit` (`tools/nv/cmd/audit.ts:1`)
      lists every file and check the rewrite scripts must reach.

## Backlog

- The `[context]` selectors a session widens by hand in `docs/agent/loop-goal.toml` (rules, shapes,
  playbook) have no `nv goal context` flag; only `--add` for a module exists, which is all Stage 8 asks.
- `tools/nv/import/` says the old name of the feature proofs by necessity, since it reads the legacy
  homes; the cutover decides whether the importer stays, and `nv audit checks` names it until then.
