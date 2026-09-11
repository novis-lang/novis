# Handoff

## State

**Goal `workspace-index`, stage 3: an occurrence is recorded at the name.** `nvs_syntax::walk::Node`
now carries a `name: Option<Span>` — where the production wrote its own name — filled beside the
grammar for every declaration, member access, `new`, variable and constant fetch, and `None` for a
production that wrote no name at all (`new $class()`, `$u->{$name}`). `crate::index`'s `named` reads
it, so `$u->greet()` is a use at `greet` rather than at `$u`, and the six `tests/lsp/references/`
expectations carrying a column are re-frozen at the name. The corpus is 223 passing.

Two productions resolve to a name written on their *class* side — an enum case read, which the
module doc's gap 1 records against its enum, and an `instanceof` — and one on its *callee* side, so
`named` answers the child node that holds it. That match is on the walk's own production vocabulary,
never on the source text.

Gap 5 is gone from `crates/nvs-lsp/src/index.rs`'s `# Known gaps`; gaps 1–4 stand. Nothing is
blocked. The goal's one ADR is still unopened, and it still owes
`rule:ide/the-request-set-is-closed` the amendment naming M10's additions beside M4B's nine.

## Next group

**Stage 3: `documentHighlight` becomes `.lspt` vocabulary, and all three slices land in one session
or none of them do** — `crates/nvs-lsp/tests/coverage.rs:68` walks `Request::ALL` against every
construct the corpus reached, so a variant added without its full row of cases turns
`every_request_answers_every_construct` red. One file set: `crates/nvs-lsp/src/case.rs`,
`crates/nvs-lsp/src/render.rs`, `crates/nvs-lsp/src/server.rs`, `tests/lsp/highlight/`.

- [ ] **The variant and its rendering.** `Request::DocumentHighlight` beside `Request::References` in
      the enum at `crates/nvs-lsp/src/case.rs:82` and in the `ALL` list under it, and
      `Response::DocumentHighlight(Vec<Place>)` beside `Response::References` at
      `crates/nvs-lsp/src/render.rs:163` — one `file:L:C` per line, sorted, exactly as the references
      rendering already does. `rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`.
- [ ] **The arm.** A `pub(crate) fn highlights_of_case` beside `references_of_case` at
      `crates/nvs-lsp/src/server.rs:821`: the same index, the same `uses_of`, narrowed to the case's
      own entry file — which is what makes it the read `rule:ide/five-features-are-one-reference-index`
      calls "the same query narrowed to the open file" rather than a second walk.
- [ ] **Twenty-six cases under `tests/lsp/highlight/`**, one per construct in the matrix, on the
      shape of `tests/lsp/references/` — every span they freeze is a box an editor draws, which is
      why the narrowing above had to land first. The row that has to come out full is the one
      `crates/nvs-lsp/tests/coverage.rs:68` builds. `rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`.

## Backlog

- Gap 4 — a cursor on a declaration's own name resolves to no symbol — is now cheap:
  `walk::Node::name` carries a declaration's own name span, so `crates/nvs-lsp/src/index.rs`'s
  `symbol_at` could answer there. It re-freezes five `-answers-nothing` cases and changes what
  `definition` means at a declaration, so it is a decision for the goal's ADR, not a chore.
- Gaps 1–3 in `crates/nvs-lsp/src/index.rs`'s `# Known gaps`: an enum case recorded against its
  enum, a class constant read that is no occurrence, an `extends`/`implements` clause that is none
  either.
- The goal's one ADR is unopened — `docs/agent/loop-goal.md` § *Standing decisions* says what it
  covers.
- Stage 4's own check is red for want of its artefacts: signature help and the two type navigations,
  `docs/agent/loop-goal.toml:7767`.
