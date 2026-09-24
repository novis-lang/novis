# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 to 7 have landed, and Stage 8 is in progress.** `bun nv chain`,
`bun nv loop --list`, `bun nv loop --goal`, `bun nv bg`, `bun nv guard`, `bun nv splice` and
`bun nv verify` are written. `chain.py` stays until the Stage 9 cutover, because its `--show`,
`--retire`, `--set` and `--retitle` have no successor yet. `main` is frozen. Tag `pre-overhaul` is the
rollback.

**Sessions and the driver both run `bun nv verify` now.** The loop prompt's step 3, the pack's
"WHEN YOU ARE DONE" block and the rustdoc-gate line, and `tools/session.py`'s refusal text all name
`bun nv verify`. The driver's rustdoc gate and per-binary reuse (`bun nv verify --keys`) already did.
`VerifyWatch` reads `.agent-tmp/verify-progress.json`, which both programs write in one shape.
`tools/verify.py` is still on disk and still cited across the tree (AGENTS.md rule 4, commands.md,
about 230 files). Two floor checks run `python tools/verify.py --list`, so deleting it means
rewriting those two `argv`s to `bun nv verify --list` with their `want` lines unchanged, in both toml
copies. `bun nv loop --list | grep verify` names them.

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

**Stage 8: `nv verify` replaces `verify.py`** — one file set: `tools/loop.py`, `tools/loop-stats.py`,
and every prose file citing `tools/verify.py`. The goal's Stage 8 bullet "`nv verify` takes over
`verify.py`'s steps unchanged, over the Stage 6 keys" specifies it.

- [ ] **Move the driver's last two `verify.py` calls to `bun nv verify`.** Anchors: the optimization
      pass's baseline at `tools/loop.py:7647` and the side-goal landing at `tools/loop.py:8391`. Also
      `tools/loop-stats.py:49`'s `VERIFY_MARKERS` and `tools/loop-stats.py:128`, which must count a
      `nv verify` call as verification.
- [ ] **Rewrite the remaining citations with a script under `.agent-tmp/`, and delete
      `tools/verify.py`.** Anchors: `docs/agent/commands.md:217`, AGENTS.md's rule 4 and its § *Session workflow* step 3, then the
      rest of `grep -rl tools/verify.py`. First confirm `bun nv verify --list` prints every `want`
      line of the two floor checks named in State, and rewrite their `argv`s in
      `docs/agent/loop-goal.toml` and `docs/agent/goals/tooling-overhaul.toml`. `bun nv parity
      verify` runs `verify.py`, so retire that parity group in the same slice.
      `tools/verify_keys.py` stays while `tools/loop.py:70` and `tools/impact.py` import it.

## Backlog

- `nv import` needs an `impact_probes` importer. Or rewrite `data/goals/*.json` with `summary`, which
  closes `nv check`'s 80 findings (`tools/nv/cmd/import.ts`).
- Stage 9 cutover: delete `dossier.py`'s `--emit-goals`, `--check-goals` and fan-out flags, and
  `generated_by` in `tools/loop.py`.
- `chain.py`'s `--show`, `--retire`, `--set` and `--retitle` have no `nv chain` successor yet.
- `docs/agent/playbook/` has no renderer from `data/playbook/`.
- `lints.py` has no `nv` successor. `verify`'s parity group declares its step command on the `nv` side
  until it has one (`tools/nv/parity/known.json`).
