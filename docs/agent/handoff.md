# Handoff

## State

**Goal 14, stage 7 is closed.** Semantic tokens answer every kind and every modifier the legend names,
and what Novis refuses is frozen as colouring nothing: `===`, `(int)$x`, PHP 8.5's hole-less `|>` and
`if (...): … endif;` each have a `.lspt` case under `tests/lsp/semantic/`, checked against `nvs check`
so each really is refused rather than quietly accepted.

**A qualifier now reaches a property.** `crates/nvs-lsp/src/semantic.rs:444`'s `qualifiers_recorded`
reads the declared type off the access's own `ExprInfo::Property`/`StaticProperty`/`HookedProperty`/
`ShapeProperty` entry — the entry `crate::hover` renders a type from — so `$this->token` and
`self::$shared` both carry `secret` where the declaration wrote it. A `foreach` binding needed no work:
`rule:types/grammar`.2 makes it write its own type, so it is a binding in `LocalScope` like any other.

**One qualified position still carries nothing: a property *declaration*.** The reason is in that file's
module doc and in the playbook bullet this session added; closing it is a `nvs-types` change, not an LSP
one. `nvs lsp-test tests/lsp/` reports `74 passed, 0 failed` against the goal's floor of 160.

The earliest red acceptance check is stage 8's `nvs/redactions`, which is unwritten — that is the group
below.

## Next group

**Stage 8: `nvs/redactions`, the one request of Novis's own** — one file set: a new
`crates/nvs-lsp/src/redactions.rs`, with `crates/nvs-lsp/src/server.rs`,
`crates/nvs-lsp/src/render.rs` and `crates/nvs-lsp/src/case.rs` edited and
`crates/nvs-lsp/src/semantic.rs` read for how a qualifier is asked.

- [ ] **A `secret` literal and a `secret` interpolation slot are answered** —
      `rule:security/redaction-ranges-come-from-the-server`,
      `rule:security/redaction-covers-bytes-only`. A `TextDocumentIdentifier` in, `{range, kind}` out
      with `kind` the open string `secretLiteral`. The walk is `semantic.rs`'s shape narrowed to
      literals, and `crates/nvs-lsp/src/server.rs:284`'s `semantic_tokens` is the handler to copy —
      empty answer, not `null`, for a document nothing is open for. Register the module at
      `crates/nvs-lsp/src/lib.rs:106`.
- [ ] **A plain and a `tainted` literal are not answered** — same rule. `secret` alone conceals;
      `rule:security/tainted-has-no-default-decoration` is why `tainted` is not in this answer at all.
      The qualifier question is `crates/nvs-lsp/src/semantic.rs:414`'s `qualifiers_at` and
      `crates/nvs-lsp/src/semantic.rs:444`'s `qualifiers_recorded`, already written.
- [ ] **An untypable expression whose binding is `secret` is answered anyway** — the named fail
      direction, `docs/agent/loop-goal.md:110`. A value must not flash on screen while its literal is
      being typed, so a span the checker could not type but whose binding declares `secret` is still
      returned. Freeze the three as `.lspt` cases: the request name goes at
      `crates/nvs-lsp/src/case.rs:101` and its canonical rendering at `crates/nvs-lsp/src/render.rs:152`.

## Backlog

- A property *declaration* carries no qualifier modifier; the fix is a shared `ExprTypeTable` between
  `nvs_types`' signature pass and its check pass — `crates/nvs-types/src/check.rs:262`.
- Stage 9's extension-host run proving the client legend equals the server's —
  `rule:ide/semantic-tokens-carry-the-qualifiers`, goal 15.
- `nvs lsp-test --coverage`'s matrix is not yet asserted empty-cell-free —
  `rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`.
