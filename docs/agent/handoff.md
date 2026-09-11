# Handoff

## State

**Goal `workspace-index`, stage 3: all five readers have landed and the stage's first check is green.**
`all_five_readers_query_the_one_index` passes beside the construction-site test, and it holds both
directions — every query the five name is called from outside `index.rs`, and every read-only method the
index offers is one of theirs.

The two new readers: **CodeLens** answers `textDocument/codeLens` from `declarations_in` and
`occurrences`, running no front end at all, and `nvs.codeLens.enable` turned off is `None` rather than an
empty list. **Type hierarchy** answers all three of its requests off a new `Declaration::supertypes` —
the direct `extends`/`implements` edge, read off `nvs_hir`'s resolved graph rather than off the clause's
own text — with `supertypes`/`subtypes` as the index's forward and reverse queries.

One thing to know before touching `capabilities.rs`: `lsp_types` 0.97 cannot express
`typeHierarchyProvider`, so `declared_capabilities` adds it to the serialized object and
`initialize_result` now returns `(PositionEncodingKind, Value)`. The playbook bullet is the whole of it.

Stage 3 still owes its second check — `nvs lsp-test tests/lsp/ --coverage` — because none of the four
added requests has an `.lspt` case, and `render::Response` has no variant for any of them. Nothing is
blocked; the goal's one ADR is still unopened.

**Nothing in the extension declares or sends either setting**, so no developer can reach `nvs.check.scope`
or `nvs.codeLens.enable` yet: `editors/vscode/package.json` contributes neither and `extension.ts` passes
no `initializationOptions`.

## Next group

**Stage 3: an `.lspt` case per added request** — one file set: `crates/nvs-lsp/src/case.rs`,
`crates/nvs-lsp/src/render.rs`, `crates/nvs-lsp/src/suite.rs`, `crates/nvs-lsp/tests/coverage.rs`,
`tests/lsp/`.

- [ ] **The four requests become `.lspt` vocabulary.** A variant each in the `Request` enum at
      `crates/nvs-lsp/src/case.rs:55` — what a `--REQUEST--` line may name — and a matching variant plus
      its rendering in `Response` at `crates/nvs-lsp/src/render.rs:135`, which is the one home for the
      spelling an `--EXPECT--` compares against. A hierarchy item renders as a name and a kind; a lens as
      a line and its title. `rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case` is the format.
- [ ] **The suite answers them.** An arm each in `crates/nvs-lsp/src/suite.rs:258`, which is `answer`'s
      seam for a case and shares what does the work rather than the dispatch — a case holds no
      `RequestId`. The four readers need an index, and `suite::store` at `crates/nvs-lsp/src/suite.rs:377`
      is where a case's documents come from, so building one there is this slice's real question.
- [ ] **A case per request under `tests/lsp/`**, each at a cursor and frozen, plus the coverage row
      `every_request_answers_every_construct` at `crates/nvs-lsp/tests/coverage.rs:57` gains for each.
      `references`, `documentHighlight`, `codeLens` and `typeHierarchy` each need one, and
      `nvs lsp-test tests/lsp/ --coverage` is stage 3's second check.

## Backlog

- The extension contributes neither setting — `editors/vscode/package.json`, and
  `crates/nvs-lsp/tests/extension_reference.rs` wants the chapter updated in the same commit.
- The goal's one ADR is unopened; next free number is 0171 (re-derive before claiming it).
- A lens shows the reference count only. Implementors and overrides are the same rule's other two
  numbers and are now reachable from `SymbolIndex::subtypes` — `rule:ide/five-features-are-one-reference-index`.
- `index.rs`'s known gap 1 stands: an enum case's uses are recorded against its enum, which is why a lens
  skips an `EnumCase` declaration.
