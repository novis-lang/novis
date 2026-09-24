# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 to 7 have landed, and Stage 8 is in progress.** `bun nv chain`,
`bun nv loop --list`, `bun nv loop --goal`, `bun nv bg`, `bun nv guard`, `bun nv splice` and now
`bun nv verify` are written. `chain.py` stays until the Stage 9 cutover, because its `--show`,
`--retire`, `--set` and `--retitle` have no successor yet. `main` is frozen. Tag `pre-overhaul` is the
rollback.

**`bun nv verify` runs `verify.py`'s steps in the same order, with the same flags.** It ran green over
the whole tree (15 of 15), and a second run answered 14 steps from its cache. Its module doc is
`tools/nv/cmd/verify.ts`. Each step's key is in `tools/nv/keys/steps.ts`, over `builtFrom`. Each test
binary's key is its `checks.ts` unit. The wide-binary check `impact.py --check` ran inside `test`, and
it is ported to `tools/nv/keys/escape.ts`. `--start` runs through `nv bg` and keeps the job id in
`.agent-tmp/verify-background.json`. `tools/verify.py` still exists, and nothing calls the new command
yet.

These are the differences from `verify.py` that slice 2's parity group must list, each with its
reason:
- The green cache is shape 3, so the two programs never read each other's step entries.
- `verify-test-green.json` holds the TS binary keys. `loop.py`'s `crate_tests` then misses verify's
  record until the driver is ported, and runs the binary itself, which is the safe direction.
- `impact-reads.json` entries have no `compiled` field. Python's `impact.Reach` then treats them as
  unrecorded, and an `nvs_repo` reader goes wide there, which is also safe.
- The case trees key at `card`, as `checks.ts` keys suites. `nv` also keys `data/`.

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

**Stage 8: `nv verify` replaces `verify.py`** — one file set: `tools/nv/parity/groups.json`,
`tools/nv/parity/known.json`, `tools/loop.py`, `tools/session.py`, `tools/orient.py`. The goal's
Stage 8 bullet "`nv verify` takes over `verify.py`'s steps unchanged, over the Stage 6 keys" specifies
it.

- [ ] **Add a `verify` parity group** after `reference` at `tools/nv/parity/groups.json:279`. The run
      itself cannot be compared, because it prints timings and each program keeps its own cache. So
      the cases are the deterministic ones: `--list`, with `--fast`, `-p nvs-ir` and `--doc`, and the
      flag conflicts `--start --wait`, `--doc --fast` and `--list --start`. `--list` already matched
      byte for byte. List the State's differences in `tools/nv/parity/known.json` or in the commit.
- [ ] **Point every caller at `bun nv verify`, and delete `tools/verify.py`.** The callers are the
      driver's rustdoc gate at `tools/loop.py:1045` (`VerifyWatch`) and its `--doc` call, then
      `tools/session.py` and `tools/orient.py`. The 344 citations are a script's job, as the `splice`
      rewrite was. `tools/verify_keys.py` stays while `loop.py` and `impact.py` import it.

## Backlog

- `nv import` needs an `impact_probes` importer. Or rewrite `data/goals/*.json` with `summary`, which
  closes `nv check`'s 80 findings (`tools/nv/cmd/import.ts`).
- Stage 9 cutover: delete `dossier.py`'s `--emit-goals`, `--check-goals` and fan-out flags, and
  `generated_by` in `tools/loop.py`.
- `chain.py`'s `--show`, `--retire`, `--set` and `--retitle` have no `nv chain` successor yet.
- `docs/agent/playbook/` has no renderer from `data/playbook/`.
