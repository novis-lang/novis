# Handoff

## State

**Goal `workspace-index`, stage 3 is complete: `documentHighlight` is `.lspt` vocabulary.**
`Request::DocumentHighlight` and `Response::DocumentHighlight(Vec<Place>)` are in the closed sets,
rendered exactly as `references` is — one sorted `file:L:C` per line — so a highlight row and a
reference row of the matrix read against each other. `nvs_lsp::server`'s `uses_in` is the narrowing
written once for the wire half and for `highlights_of_case`, and `tests/lsp/highlight/` holds 26
cases, one per construct the matrix knows. The corpus is 249 passing, every cell of the
`documentHighlight` row filled.

The declaration is not a highlight: `occurrences_in` is uses only, and the six cases that answer
something freeze that difference against their `references` twins. Nothing is blocked. The goal's one
ADR is still unopened, and it still owes `rule:ide/the-request-set-is-closed` the amendment naming
M10's additions beside M4B's nine — `documentHighlight` is now one of them.

## Next group

**Stage 4: signature help and the two type navigations, which answer from what hover already reads**
— `crates/nvs-lsp/src/hover.rs`'s `core`/`signature` pair is the whole of the new reading, so the
three requests are dispatch arms and a renderer rather than analysis. One file set:
`crates/nvs-lsp/src/hover.rs`, `crates/nvs-lsp/src/server.rs`,
`crates/nvs-lsp/src/capabilities.rs`, `crates/nvs-lsp/tests/`.

**None of the three becomes a `Request` variant.** Stage 4's check names only `-p nvs-lsp` tests and
no `nvs-suite` leg, and a variant added to `Request::ALL` at `crates/nvs-lsp/src/case.rs:82` owes a
full row of the coverage matrix — 26 more cases per request — which
`every_request_answers_every_construct` turns red the moment the variant lands without them.

- [ ] **Signature help.** `textDocument/signatureHelp` dispatched beside `HoverRequest` at
      `crates/nvs-lsp/src/server.rs:254`, declared at `crates/nvs-lsp/src/capabilities.rs:163` with
      its trigger characters, reading the enclosing call off the index and rendering its row through
      `crates/nvs-lsp/src/hover.rs:172`'s `signature` so a `Core` row and a declared parameter list
      come out the same shape. `signature_help_names_the_active_parameter_of_the_enclosing_call` and
      `signature_help_renders_a_core_row_and_a_declared_list_the_same_way`.
      `rule:ide/every-feature-is-staged-behind-its-dependency`.
- [ ] **The two type navigations.** `textDocument/typeDefinition` and `textDocument/implementation`,
      dispatched beside `GotoDefinition` at `crates/nvs-lsp/src/server.rs:250` and resolved through
      the same `require`/`autoload` graph `definition` already walks — the first to the declared
      type's own declaration, the second to every class the index records as implementing it.
      `type_definition_and_implementation_resolve_through_the_graph`.
      `rule:ide/five-features-are-one-reference-index`.
- [ ] **The one deliberately not added.** `textDocument/declaration` is not answered, because in a
      language with no separate declaration site it would answer identically to `definition`;
      `declaration_is_not_answered_because_it_would_answer_identically` holds both halves the way
      `crates/nvs-lsp/tests/index.rs:275`'s `call_hierarchy_is_not_answered` does — the capability is
      undeclared and the crate names the request nowhere.
      `rule:ide/the-request-set-is-closed`.

## Backlog

- The goal's one ADR is unopened; its `changes:` amends `rule:ide/the-request-set-is-closed`
  (`docs/agent/loop-goal.md` § *Standing decisions*).
- Gaps 1–4 stand in `crates/nvs-lsp/src/index.rs`'s `# Known gaps`.
- `docs/spec/02-php-migration.md` is still the stage-6 completion layer's unfilled input.
