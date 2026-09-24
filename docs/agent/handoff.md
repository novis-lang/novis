# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 to 7 have landed, and Stage 8 is in progress.** `bun nv chain`,
`bun nv loop --list`, `bun nv loop --goal`, `bun nv bg`, `bun nv guard`, `bun nv splice` and
`bun nv verify` are written. `chain.py` stays until the Stage 9 cutover, because its `--show`,
`--retire`, `--set` and `--retitle` have no successor yet. `main` is frozen. Tag `pre-overhaul` is the
rollback.

**Nothing in the driver runs `tools/verify.py` any more.** Sessions, the rustdoc gate, the per-binary
reuse, the optimization pass's baseline (`verify_state`) and the side-goal landing
(`landing_verify`) all run `bun nv verify`, and `loop-stats.py` counts a `nv verify` call as
verification. What still names `verify.py` is prose (about 540 hits over about 340 files, 201 of
them under `docs/agent/`, 79 under `data/playbook/`), the parity group
`tools/nv/parity/groups.json:300`, and two floor checks that run `python tools/verify.py --list`
(`bun nv loop --list | grep verify` names them). No tool imports `verify.py` as a module.

The playbook still has two homes: `docs/agent/playbook/` (read by `playbook.py` and `orient.py`) and
`data/playbook/` (what `nv import` made). An edit to a bullet goes into both until Stage 9.
`nv check`'s 80 findings are all "a stage says what it does", because `data/goals/*.json` was imported
before `summary` existed and `nv import --write` refuses while `impact_probes` has no importer. So the
acceptance check `every record is valid and every reference resolves` stays red until that lands (see
Backlog).

These habits hold for every session of this goal. A mechanical change goes through a script under
`.agent-tmp/`, written with Write. Never prove a cut with a sweep. A Python tool is deleted only after
its replacement's parity is green. `bun x tsc --noEmit -p .` typechecks the tools, and
`bun nv selftest` runs every tools test.

## Next group

**Stage 8: `nv verify` replaces `verify.py`** — one file set: every file citing `tools/verify.py`,
the two toml copies of the goal, and `tools/nv/parity/groups.json`. The goal's Stage 8 bullet
"`nv verify` takes over `verify.py`'s steps unchanged, over the Stage 6 keys" specifies it.

- [ ] **Rewrite the remaining citations with a script under `.agent-tmp/`, and delete
      `tools/verify.py`.** Anchors: `docs/agent/commands.md:217`, AGENTS.md's rule 4 and its
      § *Session workflow* step 3, the parity group at `tools/nv/parity/groups.json:300`, then the
      rest of `git grep -l verify.py`. The script skips `docs/decisions/` and `data/decisions/`,
      which are frozen history, and `tools/loop-stats.py`, whose `VERIFY_MARKERS` keeps
      `verify.py` to read old logs. Map `python tools/verify.py` and `python3 tools/verify.py` to
      `bun nv verify`, and a bare `verify.py` in prose to `nv verify`; read a sample of each
      directory's diff before committing. First confirm `bun nv verify --list` prints every `want`
      line of the two floor checks, and rewrite their `argv`s in `docs/agent/loop-goal.toml` and
      `docs/agent/goals/tooling-overhaul.toml`. Retire the `verify` parity group in the same slice.
      `tools/verify_keys.py` stays while `tools/loop.py:70` and `tools/impact.py` import it.
      CI does not run `verify.py`: its two hits (`.github/workflows/ci.yml:569`,
      `.github/workflows/release.yml:32`) are comments, and the script rewrites them too.

## Backlog

- `nv import` needs an `impact_probes` importer. Or rewrite `data/goals/*.json` with `summary`, which
  closes `nv check`'s 80 findings (`tools/nv/cmd/import.ts`).
- Stage 9 cutover: delete `dossier.py`'s `--emit-goals`, `--check-goals` and fan-out flags, and
  `generated_by` in `tools/loop.py`.
- `chain.py`'s `--show`, `--retire`, `--set` and `--retitle` have no `nv chain` successor yet.
- `docs/agent/playbook/` has no renderer from `data/playbook/`.
- `lints.py` has no `nv` successor. `verify`'s parity group declares its step command on the `nv` side
  until it has one (`tools/nv/parity/known.json`).
