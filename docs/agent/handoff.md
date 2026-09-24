# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 to 7 have landed, and Stage 8 is in progress.** `bun nv chain`,
`bun nv loop --list`, `bun nv loop --goal`, `bun nv bg`, `bun nv guard`, `bun nv splice` and
`bun nv verify` are written. `chain.py` stays until the Stage 9 cutover, because its `--show`,
`--retire`, `--set` and `--retitle` have no successor yet. `main` is frozen. Tag `pre-overhaul` is the
rollback.

**The driver now reads `bun nv verify`, and sessions still run `verify.py`.** `tools/loop.py`'s rustdoc
gate runs `bun nv verify --doc`, and its per-binary reuse (`verify_green`, `binary_inputs`) compares
against `bun nv verify --keys`, which prints each test binary's key as `nv verify`'s `test` step files
it. `VerifyWatch` reads `.agent-tmp/verify-progress.json`, which both programs write in one shape.
Until slice 2 lands, a session's `verify.py` run writes Python keys into `verify-test-green.json`, so
the sweep behind it finds no match and reruns those binaries. That costs time and is safe, and it is
why slice 2 is next. `bun nv parity verify` matched 12 of 12 before `--keys` was added, and `--keys`
has no `verify.py` twin.

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

**Stage 8: `nv verify` replaces `verify.py`** — one file set: `tools/session.py`, `tools/orient.py`,
`docs/agent/session-prompt.md`. The goal's Stage 8 bullet "`nv verify` takes over `verify.py`'s steps
unchanged, over the Stage 6 keys" specifies it.

- [ ] **Point `tools/session.py` and `tools/orient.py` at `bun nv verify`.** Anchors:
      `tools/session.py:463` and `tools/session.py:470` (the wrap's verify-ran check), and the pack's
      "WHEN YOU ARE DONE" block in `tools/orient.py`. The loop prompt's step 3 names
      `python tools/verify.py --start` too, so change it in the same slice, or sessions go on writing
      Python keys the driver no longer reads.
- [ ] **Rewrite the remaining citations with a script under `.agent-tmp/`, and delete
      `tools/verify.py`.** `tools/verify_keys.py` stays while `tools/loop.py:70` and `tools/impact.py`
      import it. The driver's other `verify.py` calls, the optimization pass's baseline at
      `tools/loop.py:7647` and the side-goal landing at `tools/loop.py:8391`, move in this slice.

## Backlog

- `nv import` needs an `impact_probes` importer. Or rewrite `data/goals/*.json` with `summary`, which
  closes `nv check`'s 80 findings (`tools/nv/cmd/import.ts`).
- Stage 9 cutover: delete `dossier.py`'s `--emit-goals`, `--check-goals` and fan-out flags, and
  `generated_by` in `tools/loop.py`.
- `chain.py`'s `--show`, `--retire`, `--set` and `--retitle` have no `nv chain` successor yet.
- `docs/agent/playbook/` has no renderer from `data/playbook/`.
- `lints.py` has no `nv` successor. `verify`'s parity group declares its step command on the `nv` side
  until it has one (`tools/nv/parity/known.json`).
