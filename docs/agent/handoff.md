# Handoff

## State

**Goal `tooling-overhaul`: Stage 3 is landed whole. Stage 4 (the importer) reads eleven of the
thirteen legacy homes it owes.** `tools/nv/cmd/import.ts` runs every importer in
`tools/nv/import/index.ts`, stages the records under `.cache/nv-import/`, runs `nv check`'s schema,
foreign-key and invariant checks over them, renders from them, and prints each legacy file it could
not read with the reason. Today it reports 0 unread and 0 findings, and it exits 1 only because five
types have no importer: gap, the two spec tables, proof policy and help backlog. The goals importer
(`tools/nv/import/goals.ts`) builds every goal, side goal, handoff and `chain`; the plan importer
(`tools/nv/import/plan.ts`) builds the status block and the milestones. The mapping rules each one
applies (the installed goal read from `loop-goal.*`, `post-parity`/`dossier` goals with a null
milestone, the generator's note in a dossier handoff dropped) are in their module docs, so
`known.json` owes none of them. `bun nv render --check` passes only because
`tools/nv/renderers/index.ts` lists no renderer yet. `data/` does not exist yet, so the acceptance
check for `data/chain.json` stays red until `--write` lands. `main` is still frozen, and the tag
`pre-overhaul` is the rollback point.

Three habits hold for every session of this goal: a mechanical change goes through a script under
`.agent-tmp/`; never prove a cut with a sweep; a Python tool is deleted only after its
replacement's parity is green.

## Next group

**Stage 4: the importer** — one file set: `tools/nv/import/**`, `tools/nv/schema/**`,
`tools/nv/cmd/import.ts`. loop-goal.md § *Stage 4* is the spec. Each new importer is a module under
`tools/nv/import/` and is listed in `tools/nv/import/index.ts:13`, with a case in
`tools/nv/test/import.test.ts`.

- [ ] **Gaps with slugs**: every `//! # Known gaps` block into `tools/nv/schema/gap.ts:1`, each
      gap's slug from its bold title and its owner from its `— owner:` trailer. List every citation
      of a gap by its position for Stage 9 to rewrite (loop-goal.md § *Stage 4*).
- [ ] **Spec rows, proof policy and help backlog**: the tables `gaps.py`, `check-migration.py`,
      `reference.py` and `sync-core.mjs` parse into `tools/nv/schema/spec.ts:1`, and
      `tools/data/dossier-policy.toml` and `help-backlog.toml` into `tools/nv/schema/proofs.ts:1`.
- [ ] **`known.json` and `--write`** (`tools/nv/cmd/import.ts:20`): the declared differences, one
      per file with its reason, then `--write`, which writes `data/` and rewrites the prose files'
      front matter. `--write` is what turns the `data/chain.json` acceptance check green.

## Backlog

- No stage owns the renderers for the legacy generated files: `docs/rules/<topic>.md`,
  `ground-rules.md`, `decisions.md` and the rest named in loop-goal.md § *The data model*. Until they
  exist, `render --check` and the compare step of `import --check` pass over zero files. They
  belong with `--write`, before Stage 5's parity.
- A playbook bullet's `files` is computed against the tree at import time, as `tools/playbook.py`'s
  `anchors` does, so a path that is deleted later falls out of the list at the next import.
- `docs/plan/m10.md` says **Backlog 2.** under its H1, while the plan's table says goals carry M10.
  The prose is stale, and the renderer that writes the milestone headers is where it gets fixed.
