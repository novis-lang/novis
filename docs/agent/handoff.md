# Handoff

## State

**Goal `tooling-overhaul`: Stage 3 is landed whole. Stage 4 (the importer) has its record types and
the first half of `bun nv import --check`.** `tools/nv/cmd/import.ts` runs every importer in
`tools/nv/import/index.ts`. It stages their records under `.cache/nv-import/` and runs `nv check`'s own
schema, foreign-key and invariant checks over them. Then it renders from them, and it prints each
legacy file it could not read with the reason. It exits 1 while a record type has no importer, and it
prints those types on a `not imported yet:` line. Six types are imported, and the command reports
0 unread and 0 findings for them: topic and rule (`docs/rules/*.json`), decision (YAML block, H1, bold
fields, `docs/decisions.toml`), reference chapter, playbook section and bullet. A decision whose
**Depends on:** says more than its links keeps the text in the new `dependsOnText` field. The
importer also checks that each record's `changes:` block matches what the rules' `because` lists
derive. `bun nv render --check` now prints `render: every generated file is current`, but only because
`tools/nv/renderers/index.ts` lists no renderer yet. `data/` does not exist yet. `main` is still
frozen, and the tag `pre-overhaul` is the rollback point.

Three habits hold for every session of this goal: a mechanical change goes through a script under
`.agent-tmp/`; never prove a cut with a sweep; a Python tool is deleted only after its
replacement's parity is green.

## Next group

**Stage 4: the importer** — one file set: `tools/nv/import/**`, `tools/nv/schema/**`,
`tools/nv/cmd/import.ts`. loop-goal.md § *Stage 4* is the spec. Each new importer is a module under
`tools/nv/import/` and is listed in `tools/nv/import/index.ts:10`, with a case in
`tools/nv/test/import.test.ts`.

- [ ] **Goals, handoffs and side goals**: import `docs/agent/goals/N-<slug>.{md,toml,handoff.md}`,
      `docs/agent/goals/side/<slug>/`, `docs/agent/loop-goal.*` and `docs/agent/handoff.md` into
      the types at `tools/nv/schema/goal.ts:65` and `tools/nv/schema/handoff.ts:1`. TOML keys become
      camelCase, `[context.stage.N]` folds into `stages[].context`, and a stage's title comes from its
      checks' `stage = "N title"`. The goal tags `dossier` and `post-parity` need milestone records
      of their own or a declared difference. Read TOML with `smol-toml`.
- [ ] **`data/chain.json` and the dossier goals** (`docs/agent/loop-goal.md:298`): goal numbers
      become the slug list in `tools/nv/schema/chain.ts:1`, and the 102 goals under
      `docs/agent/goals/dossier/` join the others in the same order.
- [ ] **Milestones and the plan status**: `docs/implementation-plan.md`'s status block and
      milestone table and `docs/plan/<id>.md` into `tools/nv/schema/milestone.ts:1` and
      `tools/nv/schema/plan-status.ts:1`.
- [ ] **Gaps with slugs, spec rows, proof policy and help backlog**, then `known.json` and
      `--write` (`tools/nv/cmd/import.ts:20`): `tools/nv/schema/gap.ts:1`, `tools/nv/schema/spec.ts:1`,
      `tools/nv/schema/proofs.ts:1`. `--write` writes `data/` and rewrites the prose files' front matter.

## Backlog

- No stage owns the renderers for the legacy generated files: `docs/rules/<topic>.md`,
  `ground-rules.md`, `decisions.md` and the rest named in loop-goal.md § *The data model*. Until they
  exist, `render --check` and the compare step of `import --check` pass over zero files. They
  belong with `--write`, before Stage 5's parity.
- A playbook bullet's `files` is computed against the tree at import time, as `tools/playbook.py`'s
  `anchors` does, so a path that is deleted later falls out of the list at the next import.
