# Handoff

## State

**Goal `tooling-overhaul`: Stages 3 and 4 are landed whole, and every Stage 4 check passes.**
`bun nv import` reads all thirteen legacy homes into 3174 records of 18 types. `--check` writes
nothing and exits 0. `--write` wrote `data/`, and a second run changes nothing. `bun nv check`
reports 0 findings over `data/` and 3767 prose files, and `render --check` passes only because
`tools/nv/renderers/index.ts` lists no renderer yet. `tools/nv/import/known.json` declares the five
files where the import and the Python tools disagree, one reason each.
**`data/` is a snapshot, not yet the authority.** The legacy homes still are, and every wrap edits
some of them (the handoff at least). Re-run `bun nv import --write` before a stage reads `data/` as
the truth, and at the cutover. `main` is still frozen, and the tag `pre-overhaul` is the rollback
point.

Three habits hold for every session of this goal: a mechanical change goes through a script under
`.agent-tmp/`; never prove a cut with a sweep; a Python tool is deleted only after its
replacement's parity is green.

## Next group

**Stage 5: the read-only tools, proven by parity** — one file set: `tools/nv/cmd/**`,
`tools/nv/parity/**`, `tools/nv/main.ts`. loop-goal.md § *Stage 5* is the spec. A subcommand is
registered in `tools/nv/main.ts:15`'s `COMMANDS`.

- [ ] **`bun nv parity <group>` and `tools/nv/parity/known.json`** (`tools/nv/main.ts:15`): runs
      the Python tool and its replacement on one tree and compares their output, ignoring only what
      `known.json` declares. loop-goal.md § *Stage 5*, last bullet. Build it first, since every
      command after it is proven with it.
- [ ] **`bun nv brief --where`** (`tools/brief.py:515`): routes a keyword to the rulebook chapter
      or home that owns it, reading `data/rules` and `data/topics`. It is the stage's acceptance
      check (`--where testing` prints `docs/rules/testing.md`). Prove it with `nv parity brief`.
- [ ] **`bun nv migration` and `bun nv reference`** (`tools/check-migration.py:167`,
      `tools/reference.py:199`): read `data/spec/php-migration.json` in place of the table. The
      `Core\Jwt` row difference is declared in `tools/nv/import/known.json`, and parity has to
      declare it again in its own list.

## Backlog

- `tools/check-migration.py` reads the `**Complete:**` line out of `docs/spec/02-php-migration.md`'s
  prose. The standing decisions say no tool reads a field from prose, so `nv migration` needs a record
  field for it (loop-goal.md § *Stage 5*).
- `website/scripts/sync-core.mjs` parses `docs/spec/01-core-library.md` with its own parser and
  `spec-overrides.mjs`. Moving it onto `data/spec/core-members.json` is cutover work (loop-goal.md
  § *Stage 9*).
- No test covers `--write`'s refusal or its removal of a stale record file, because `build()` in
  `tools/nv/cmd/import.ts` reads `ROOT`. A root parameter would let `tools/nv/test/import.test.ts`
  cover both.
- The pack for Stage 5 has no `[context.stage.5]` overlay in `docs/agent/loop-goal.toml`, so it
  carries the base manifest. Add one naming `tools/brief.py` and `tools/nv/main.ts` if the base
  pack reads wide.
