# Handoff

## State

**Goal `tooling-overhaul`: Stage 3 (the foundation) is landed whole, and Stage 4 (the importer) is
next.** `bun nv` exists: root `package.json` (Bun pinned at `1.3.5` in `engines`), `bun.lock`,
strict `tsconfig.json`, and `tools/nv/` with `lib/{schema,store,index,prose,render,git,paths,proc}`,
the commands `check`, `query`, `render` and `selftest`, and 28 `bun test` cases under `tools/nv/test/`.
`bun nv selftest` prints both lines the driver's Stage 3 check wants. `verify.py` has an `nv` step
beside `build`, keyed on `tools/nv/` and the three package files (`verify_keys.py`). `docs/setup.md`
now lists Bun as required. The registries `tools/nv/schema/index.ts` (`RECORDS`) and
`tools/nv/renderers/index.ts` (`RENDERERS`) are empty on purpose. A record type lands with the
importer that first writes its records. `main` is still frozen, and the tag `pre-overhaul` is the
rollback point.

Three habits hold for every session of this goal: a mechanical change goes through a script under
`.agent-tmp/`; never prove a cut with a sweep; a Python tool is deleted only after its
replacement's parity is green.

## Next group

**Stage 4: the importer** — one file set: `tools/nv/import/**`, `tools/nv/schema/**`. loop-goal.md
§ *Stage 4* is the spec, and § *The data model*'s entity table is what each record holds.

- [ ] **Record types**: one `defineRecord` file per entity in the table, listed in
      `tools/nv/schema/index.ts:10`, with ids, `s.ref` foreign keys and child lists as the table gives
      them (`docs/agent/loop-goal.md:174`).
- [ ] **`bun nv import --check`**: reads every legacy home into records in memory, prints each file
      it could not read with the reason, then renders and compares (`docs/agent/loop-goal.md:289`).
- [ ] **Declared differences and gap slugs**: `tools/nv/import/known.json`, a slug per gap from its
      bold title, and the list of positional gap citations for Stage 9 (`docs/agent/loop-goal.md:293`).
- [ ] **`data/chain.json` and `--write`**: goal numbers become the slug list, the dossier goals join
      as ordinary goals in order, and `--write` writes `data/` (`docs/agent/loop-goal.md:298`).

## Backlog

- `data/schema/` publishes each type's `jsonSchema()`. Nothing writes it yet. It lands with the first
  record type (loop-goal.md § *The data model*).
- `.gitattributes` `linguist-generated` for rendered files lands with the first renderer
  (`tools/nv/lib/render.ts` holds `MARKER`).
- The installed TypeScript is 7.0.2, the native `tsc`. `tsconfig.json` uses `module: Preserve` and
  `moduleResolution: bundler`, which it accepts.
