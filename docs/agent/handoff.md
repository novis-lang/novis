# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 to 8 have landed, and Stage 4's records are valid.** Stage 8's last
open item, `bun nv goal context --add <path>` (`tools/nv/cmd/goal.ts`), is on disk with its test. What
Stage 8 still owes is three changes that wait for the cutover. `bun nv check` prints `nv check: 0
findings`, and `bun nv render --check` is current. The driver's red check is Stage 9's `bun nv audit
goals`, which is the cutover, and no `nv audit` command exists yet.

`liveGoal` reads the driver's pointer by slug (`tools/nv/lib/state.ts`). `chainGoals` and `liveGoal`
take a `root`, so a test runs them over a scratch tree. The pointer is `.loop/state.sqlite`'s `pointer`
row when that file exists. Until the cutover it is the Python driver's `.loop/chain.json`, whose number
resolves through the goal file named `N-<slug>.md`. Cutover step 5 ("`.loop/chain.json` becomes the
state database's pointer") is `writePointer(legacyPointer())`.

Three changes wait for the cutover, because the running Python driver still reads what they would drop.
`manifestCopies` (`tools/nv/cmd/session.ts:387`) gates `docs/agent/loop-goal.toml` beside the record,
and that toml is the driver's acceptance list. `recordPack` (`tools/nv/cmd/session.ts:1512`) measures
`python tools/orient.py`, the pack the driver pipes. `nv goal context --add` edits
`docs/agent/loop-goal.toml` while it exists, and does nothing to it once the cutover deletes it. Each of
the first two switches in the cutover commit.

`bun nv session --wrap` validates and applies a wrap by itself. Eight playbook records differ from their
fragment in `files` alone, where the record is right; `nv import --write` still rewrites those eight, and
the bullet `docs/agent/playbook/tooling/bun-nv-import-write-rewrites-a-data-playbook-bullet-back-to.md`
says what to do.

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

**Stage 9: the cutover** — one file set: a new `tools/nv/cmd/audit.ts`, `tools/nv/main.ts`,
`tools/nv/cmd/session.ts`, `tools/nv/lib/state.ts`. loop-goal.md § *Stage 9* is the specification, and
Stage 9's three checks in `docs/agent/loop-goal.toml` are its acceptance.

- [ ] **`nv audit goals | checks | eol`.** A new command registered in `tools/nv/main.ts:44`. `goals`
      prints `audit: no goal file carries a number` and `audit: no copy of a goal's files is tracked`
      when both hold; `checks` prints `audit: no check runs python` and `audit: nothing says dossier`;
      `eol` prints `audit: every tracked text file is LF`. Each names every offender and exits nonzero
      otherwise. It is red today, and that is right until the cutover lands.
- [ ] **The cutover switches.** `manifestCopies` (`tools/nv/cmd/session.ts:387`) stops gating
      `docs/agent/loop-goal.toml`, `recordPack` (`tools/nv/cmd/session.ts:1512`) measures `bun nv
      orient`, and step 5 is `writePointer(legacyPointer())` (`tools/nv/lib/state.ts:49`).
- [ ] **The cutover commit**, `docs/agent/loop-goal.md:553` steps 1 to 7, in one slice, with `bun nv loop
      --goal-only` green under the new driver before it commits.

## Backlog

- The `[context]` selectors a session widens by hand in `docs/agent/loop-goal.toml` (rules, shapes,
  playbook) have no `nv goal context` flag; only `--add` for a module exists, which is all Stage 8 asks.
