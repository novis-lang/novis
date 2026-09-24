# Handoff

## State

**Goal `tooling-overhaul`: Stage 3 (the foundation) is landed whole, and Stage 4 (the importer) has
its record types.** `tools/nv/schema/` declares one type per entity in loop-goal.md § *The data
model*, all 18 listed in `RECORDS`: goal, handoff, side goal and side handoff, chain, topic, rule,
decision, plan status, milestone, gap, playbook section and bullet, reference chapter, the two spec
tables, proof policy and help backlog. The impact probe waits for Stage 6, which defines it. Each
type's doc comment says what it holds; the invariants a foreign key cannot state are `checks` SQL,
pinned by `tools/nv/test/records.test.ts`. `RecordType.suffix` lets a handoff live beside its goal as
`<slug>.handoff.json`. `nv check`'s summary now opens `nv check: N findings`, which is the line the
Stage 4 check wants. `data/` does not exist yet, so the check passes over zero records until the
importer writes them. `main` is still frozen, and the tag `pre-overhaul` is the rollback point.

Three habits hold for every session of this goal: a mechanical change goes through a script under
`.agent-tmp/`; never prove a cut with a sweep; a Python tool is deleted only after its
replacement's parity is green.

## Next group

**Stage 4: the importer** — one file set: `tools/nv/import/**`, `tools/nv/schema/**`. loop-goal.md
§ *Stage 4* is the spec, and the types under `tools/nv/schema/` are what each record holds.

- [ ] **`bun nv import --check`**: reads every legacy home into records in memory, prints each file
      it could not read with the reason, then renders and compares (`docs/agent/loop-goal.md:289`).
      The mappings the types already fixed: TOML keys become camelCase (`min_bytes` is `minBytes`);
      `[context.stage.N]` folds into `stages[].context`; a stage's title comes from its checks'
      `stage = "N title"`; a milestone id is lower case (`m4s`), so the goal tags `dossier` and
      `post-parity` need milestone records of their own or a declared difference
      (`tools/nv/schema/goal.ts:66`); a decision's title is its H1 after `ADR NNNN — `; a reference
      chapter's `id` is the field `slug`, and `keywords` is split on `, `
      (`tools/nv/schema/reference.ts:12`).
- [ ] **Declared differences and gap slugs**: `tools/nv/import/known.json`, a slug per gap from its
      bold title, and the list of positional gap citations for Stage 9 (`docs/agent/loop-goal.md:293`).
- [ ] **`data/chain.json` and `--write`**: goal numbers become the slug list, the dossier goals join
      as ordinary goals in order, and `--write` writes `data/` (`docs/agent/loop-goal.md:298`).

## Backlog

- `data/schema/` publishes each type's `jsonSchema()`, and nothing writes it yet. `nv check` would
  call those files orphans, so `recordFiles` must skip `data/schema/` when it lands
  (`tools/nv/lib/store.ts`).
- `.gitattributes` `linguist-generated` for rendered files lands with the first renderer
  (`tools/nv/lib/render.ts` holds `MARKER`).
- The installed TypeScript is 7.0.2, the native `tsc`. `tsconfig.json` uses `module: Preserve` and
  `moduleResolution: bundler`, which it accepts.
