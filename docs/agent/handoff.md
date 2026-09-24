# Handoff

## State

**Goal `tooling-overhaul`: Stage 3 is landed whole. Stage 4 (the importer) now reads twelve of the
thirteen legacy homes it owes.** `tools/nv/cmd/import.ts` runs every importer in
`tools/nv/import/index.ts`, stages the records under `.cache/nv-import/`, runs `nv check`'s schema,
foreign-key and invariant checks over them, renders from them, and prints each legacy file it could
not read with the reason. The gaps importer (`tools/nv/import/gaps.ts`) turns the 45 `# Known gaps`
items into gap records with slugs and milestone owners. Today `--check` reports 0 findings and 3
unread: three gap blocks open on shared prose that no gap holds. It exits 1 on those and on the four
types with no importer: the two spec tables, proof policy and help backlog.
`bun nv import --citations` lists every citation of a gap by its position (50 today, 14 naming a
gap that exists) for Stage 9's rewrite. `bun nv render --check` passes only because
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

- [ ] **Spec rows, proof policy and help backlog**: the tables `gaps.py`, `check-migration.py`,
      `reference.py` and `sync-core.mjs` parse into `tools/nv/schema/spec.ts:1`, and
      `tools/data/dossier-policy.toml` and `help-backlog.toml` into `tools/nv/schema/proofs.ts:1`.
- [ ] **`known.json` and `--write`** (`tools/nv/cmd/import.ts:51`): the declared differences, one
      per file with its reason, then `--write`, which writes `data/` and rewrites the prose files'
      front matter. `--write` is what turns the `data/chain.json` acceptance check green. The gap
      differences to declare are the three block preambles `--check` names
      (`crates/nvs-cli/src/openapi.rs:29`, `crates/nvs-ir/src/lib.rs:582`,
      `crates/nvs-runtime/src/lib.rs:200`), and the bold-run block at
      `crates/nvs-lsp/src/completion.rs:259`. That block states four gaps as one paragraph, so its
      title is its first sentence and its slug is `nvs-lsp/two-of-them-are-the-class`.

## Backlog

- No stage owns the renderers for the legacy generated files: `docs/rules/<topic>.md`,
  `ground-rules.md`, `decisions.md` and the rest named in loop-goal.md § *The data model*. Until they
  exist, `render --check` and the compare step of `import --check` pass over zero files. They
  belong with `--write`, before Stage 5's parity.
- A playbook bullet's `files` is computed against the tree at import time, as `tools/playbook.py`'s
  `anchors` does, so a path that is deleted later falls out of the list at the next import.
- `docs/plan/m10.md` says **Backlog 2.** under its H1, while the plan's table says goals carry M10.
  The prose is stale, and the renderer that writes the milestone headers is where it gets fixed.
- Stage 9's citation rewrite cannot trust a position blindly. `nv import --citations` resolves a
  position against today's numbering, so an old citation can name a different gap from the one it
  meant, as `docs/agent/goals/7-carried-gaps.md:128` does. Most name no gap at all. Each resolved
  line needs a reader's check before the script rewrites it (`tools/nv/import/gaps.ts`).
