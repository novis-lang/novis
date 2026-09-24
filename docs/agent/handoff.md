# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 to 8 have landed, and Stage 9's `bun nv audit` and its switches are
on disk.** What is left is the cutover commit itself. `bun nv audit eol` is green: the index holds no
CRLF file. `bun nv audit goals` and `bun nv audit checks` are red, and that is right until the cutover
lands. Each names every offender: hundreds of numbered goal files and tracked copies, the floor `argv`s
that run Python, and the tools and tests that still say the old name of the feature proofs.

**The switches are one predicate, `cutOver()`** (`tools/nv/lib/state.ts:81`): true once
`.loop/state.sqlite` holds a pointer. Nothing writes that pointer except `adoptLegacyPointer()`
(`tools/nv/lib/state.ts:90`), which is cutover step 5. Once it holds, `manifestCopies`
(`tools/nv/cmd/session.ts:389`) gates the goal's record alone and no longer the installed toml, and
`recordPack` (`tools/nv/cmd/session.ts:1515`) measures `bun nv orient` in place of `python
tools/orient.py`. Before it, both behave exactly as they did. `nv goal context --add` edits
`docs/agent/loop-goal.toml` only while that file exists, so it needs no switch.

**Step 5 must run before step 2's renames**: `legacyPointer` resolves the Python driver's number
through the numbered goal file names, and step 2 deletes those names.

`nv audit checks` reads the checks from every goal and side-goal record under `data/goals/`, plus
`docs/agent/loop-goal.toml` while that file exists. The old name of the feature proofs counts in a
check's name or `argv`, in a `//` or `#` directive line of a tracked `.nvs`, `.nvst` or `.rs`, and in any
tracked file under `tools/` by path or by a line of text. Prose outside `tools/` is not scanned.

Eight playbook records differ from their fragment in `files` alone, where the record is right; `nv
import --write` still rewrites those eight, and the bullet
`docs/agent/playbook/tooling/bun-nv-import-write-rewrites-a-data-playbook-bullet-back-to.md` says what
to do.

`tools/orient.py`, `chain.py`, `session.py`, `playbook.py` and `tools/verify_keys.py` stay until the
cutover. `main` is frozen. Tag `pre-overhaul` is the rollback. An edit to a bullet goes into both
playbook homes until Stage 9, and a wrap does that by itself.

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

**Stage 9: the cutover** — one file set: `.agent-tmp/cutover.ts` (not committed), `tools/loop.py`,
`tools/nv/lib/state.ts`, `tools/nv/cmd/audit.ts`. loop-goal.md § *Stage 9* is the specification, and
`bun nv audit` is its acceptance.

- [ ] **The cutover script.** `.agent-tmp/cutover.ts` runs `docs/agent/loop-goal.md:553` steps 1 to 7
      in a scratch worktree first, calling `adoptLegacyPointer()` (`tools/nv/lib/state.ts:90`) before
      any rename, and `bun nv audit` over the result names what it still leaves behind. It writes the
      `tools/loop.py` shim of step 4 as well.
- [ ] **The cutover commit**, `docs/agent/loop-goal.md:553` steps 1 to 7 in one slice, after `bun nv
      loop --goal-only` is green under the new driver in that worktree; `bun nv audit` goes green.

## Backlog

- The eight playbook records `nv import --write` rewrites — the bullet under
  `docs/agent/playbook/tooling/` named in § State.
- Stage 10's triage waits for the cutover — loop-goal.md § *Stage 10*.
