# Handoff

## State

**Goal `workspace-index`, stage 6 is two thirds landed.** `crates/nvs-lsp/src/hints.rs` answers the
two shapes `loop-goal.md` § *Stage 6* bounds the reversal to — the inferred type after a `var`
declaration with no annotation, and the parameter name at a call site whose argument is a bare
literal — and the driver's acceptance check for the stage is green.

**Both halves are read out of what the type phase recorded and out of nothing else.** The
declaration's type is `Analysed::local_ty`'s answer over `ExprTypeTable::local_scopes`; the
parameter's name is `nvs_types::ResolvedCall::param_names` joined through that call's `arg_slots`.
A site the table holds nothing for gets no hint, which is the third acceptance test.

**The walk matches `nvs_syntax::ast` rather than `nvs_syntax::walk`**, on `crate::semantic`'s terms
and documented in `hints.rs`'s module doc: both facts it turns on are *absences* — a declaration that
wrote no type, an argument that wrote no name — and `walk` models neither.

**The wire is landed.** `server_capabilities` declares `inlayHintProvider`, `server.rs`'s dispatch
answers `textDocument/inlayHint` beside `completion`, and the answer is narrowed to the asked-for
range there rather than in the walk. `tests/handshake.rs`'s closed capability list names it.

**What is left of the stage is the freezing**, and after it the goal owes only its one ADR.

## Next group

**Stage 6: the freezing** — one file set: `crates/nvs-lsp/src/case.rs`,
`crates/nvs-lsp/src/render.rs`, `crates/nvs-lsp/src/suite.rs`, `crates/nvs-lsp/src/coverage.rs`,
`tests/lsp/hints/` (new).

- [ ] **The request and its rendering.** `crates/nvs-lsp/src/case.rs:57`'s `Request` gains
      `InlayHint` — `ALL`, `name()` as `inlayHint`, and `takes_cursor` false, since the request is
      asked of a document — and `crates/nvs-lsp/src/render.rs:135`'s `Response` gains
      `Hints(Vec<InlayHint>)` with its canonical line. `rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`
      and `rule:ide/the-rendering-has-one-home`. `crates/nvs-lsp/src/suite.rs`'s `answer` needs the
      arm too, or a case naming the request scores as a failure rather than an answer.
- [ ] **The cases.** `tests/lsp/hints/` freezes the rendering: the two shapes, a declaration that
      wrote its type out, a named argument, and a receiver nothing declared. Adding a `Request`
      variant puts a new row in the coverage matrix, so
      `crates/nvs-lsp/tests/coverage.rs:57`'s `every_request_answers_every_construct` fails until the
      cases reach every construct — run `target/debug/nvs.exe lsp-test tests/lsp/ --coverage`, which
      `cargo test` does not.
- [ ] **The goal's one ADR.** `docs/decisions/0171.md` (re-check the number first), covering stage
      2's index shape, stage 4's request admissions against [ADR 0099](../decisions/0099.md) § 3,
      stage 5's namespace and bare-name arms and stage 6's reversal of that record's inlay-hint
      deferral. Its `changes:` block amends `rule:ide/the-request-set-is-closed` to name M10's
      additions beside M4B's nine — the entry at `docs/rules/ide.json:181` and the fragment
      `docs/rules/ide/the-request-set-is-closed.md:1` — then `python tools/rules.py --render`.

## Backlog

- `nvs.check.scope` and `nvs.codeLens.enable` are read by the server and contributed by neither the
  manifest nor the chapter — `rule:ide/contributions-are-frozen-and-only-ever-added`.
- `nvs.inlayHints.enable` is contributed by nothing: the layer is on for every client that asks —
  same rule, and a setting name is frozen the day it is first written.
- A call recorded as anything but `ExprInfo::Call` — a `new`, a callable signature, an erased call —
  carries no parameter hints — `hints.rs` § *Known gaps*.
- A `NotRegistered` item cannot name the milestone it waits on: only `tests/migration-members-outstanding.txt` knows, and it is a test fixture — `php_names.rs` module doc.
- A destination naming an interface (`Core\Db\Queryable`) inserts nothing, since the interface has no registry row — same.
- A cursor in a type annotation or a parameter list reaches no arm of its own — `completion.rs` § *Known gaps*.
