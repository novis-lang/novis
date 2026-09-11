# Handoff

## State

**Goal `workspace-index`, stage 3: both of the stage's checks are green.** `nvs lsp-test tests/lsp/
--coverage` now prints the matrix and the verdict **under** it — it used to print the matrix instead,
so the `nvs-suite` check reading the last line could never have found one — and the corpus is 195
passing.

**`codeLens` is the first of the four added requests to become `.lspt` vocabulary**, with three cases
under `tests/lsp/lens/`. A lens renders as `L:C title`, which is its anchor's start and its count:
`2:7 2 references`, `3:17 no references`. It is asked of the whole document, so it owed one row and
not a column's worth of cases — see the new playbook bullet for what the other three cost.

**A case is answered against an index of its own**, built by `crate::server`'s `lenses_of_case` at
workspace scope over the directory the runner materialised the case into. It is built there and not
in `crate::suite` because `crates/nvs-lsp/tests/index.rs` counts who may build one; that test now
says every builder is `server.rs` rather than that there is exactly one call.

`crates/nvs-lsp/src/index.rs`'s `# Known gaps` grew two entries this session, both found by a lens
reading zero where a reader would not: a class constant's read is no occurrence (`definition::Target`
has no constant variant) and neither is a name in an `extends` or `implements` clause (a use is read
off `Analysed::exprs` and a clause is not an expression).

Nothing is blocked. The goal's one ADR is still unopened, and it still owes
`rule:ide/the-request-set-is-closed` the amendment naming M10's additions beside M4B's nine.

## Next group

**Stage 3: `textDocument/references` becomes `.lspt` vocabulary** — one file set:
`crates/nvs-lsp/src/case.rs`, `crates/nvs-lsp/src/render.rs`, `crates/nvs-lsp/src/coverage.rs`,
`crates/nvs-lsp/src/server.rs`, `crates/nvs-lsp/src/suite.rs`, `tests/lsp/references/`.

- [ ] **The variant and its rendering.** `Request::References` in the enum at
      `crates/nvs-lsp/src/case.rs:57`, named in `ALL`, in `name()` and in `takes_cursor()`; a
      `Response::References(Vec<Place>)` at `crates/nvs-lsp/src/render.rs:135` rendering one
      `file:L:C` per line, sorted, the way `fn lens` at `crates/nvs-lsp/src/render.rs:474` was added
      beside `fn place`. `rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case` is the format.
- [ ] **The arm.** A `pub(crate) fn references_of_case` beside
      `crates/nvs-lsp/src/server.rs:778`, sharing `case_settings` at
      `crates/nvs-lsp/src/server.rs:762`, and an arm at `crates/nvs-lsp/src/suite.rs:274` calling it
      the way `fn code_lens` at `crates/nvs-lsp/src/suite.rs:559` does. `Materialised::spelling` is
      what turns a `Location` back into the path a case wrote, as the `definition` arm shows. Decide
      and say in the comment which `include_declaration` the runner asks with — a case writes a
      document and a question, never a client's context.
- [ ] **Twenty-six cases under `tests/lsp/references/`**, one per construct in the matrix
      `nvs lsp-test tests/lsp/ --coverage` prints, or `every_request_answers_every_construct` at
      `crates/nvs-lsp/tests/coverage.rs:57` fails naming each empty cell. `tests/lsp/actions/` is
      the same 26 cursors already written down.

## Backlog

- `documentHighlight` and `typeHierarchy`, 26 cases each, on the group above's shape — `docs/agent/loop-goal.md` stage 3.
- `typeHierarchy` is three LSP methods under one request name: the `--REQUEST--` line needs a `direction=` argument under `rule:ide/a-request-line-is-closed`, or three variants and 78 cases.
- A lens carries only the reference count; `rule:ide/five-features-are-one-reference-index` also names a type's implementors and a method's overrides.
- `nvs.codeLens.enable` off answers `None` rather than an empty list, and no case can write a setting — that branch is a `-p nvs-lsp` test's.
- The goal's one ADR (next free is 0171) — `docs/agent/loop-goal.md` § *Standing decisions*.
- Nothing in `editors/vscode` declares `nvs.check.scope` or `nvs.codeLens.enable` yet — `rule:ide/contributions-are-frozen-and-only-ever-added`.
