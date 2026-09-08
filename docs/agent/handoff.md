# Handoff

## State

**Goal 15 stage 6 is done.** `nvs/redactions` answers two kinds on one list —
`secretLiteral`, bytes to conceal, and `taintedDeclaration`, the name of a local whose type carries
`tainted` — and the client conceals the first, marks the second only where `nvs.taint.mark` asks,
and conceals any spelling it has not been taught. The one-list decision and the walk are
`crates/nvs-lsp/src/redactions.rs`'s module doc; the client's partition is
`editors/vscode/src/concealment.ts`'s `MARKERS`. Headless is 158 passing over four suites, the
`tests/lsp/redactions/` tree is 6 cases, `npm run lint` is clean.

**ADR 0101 § 4 asks for a codicon and the API does not offer one.** An inline decoration attachment
takes `contentText` or an image path and never both, and only the text takes a `ThemeColor`, so the
glyph is a themed BMP character. That trade is recorded in `editors/vscode/README.md` §
*Decided here*, which is where the goal's *Standing decisions* put a decision too small for a
record.

**`nvs.taint.mark = sink` marks what `declaration` marks.** No kind answers a sink's argument
positions, because `rule:security/sink-predicate`'s classification is not on the member rows yet and
ADR 0101 § *Open questions* leaves whether that value is built at all open. The gap is stated in
`crates/nvs-lsp/src/redactions.rs` § *What it does not reach*, beside the parameter and property
declarations the marker also cannot see.

## Next group

**Stage 9: the reference chapter's last heading** — one file set:
`docs/reference/tools/40-editor.md`, `docs/examples/`, `tests/hostile/` and the generated
`docs/novis.md`. The dossier roster is derived from `#` headings, so the feature the acceptance
check names does not exist until the heading is written; then it owes what `POLICY["tool"]` owes.

- [ ] **The chapter gains `# The VS Code extension`** — a third `#` heading beside
      `docs/reference/tools/40-editor.md:8`'s `# nvs lsp` and `docs/reference/tools/40-editor.md:119`'s
      `# nvs lsp-test`, written for a reader who has never seen this repository: what the extension
      is, what it does not do, and the settings and commands it contributes.
      `rule:ide/vscode-is-the-reference-client`. Then `python tools/reference.py --no-examples`,
      because `docs/novis.md` is generated and its check is the stage's second one.
- [ ] **The three proofs that heading then owes** — one test, one example, one hostile program and
      no perf figure, which is `tools/dossier.py:250`'s `POLICY["tool"]` row and
      `rule:testing/four-proofs`. `python tools/dossier.py --id
      'tools:editor/the-vs-code-extension'` prints where each goes once the heading exists, and
      `tools:editor`'s other two features are the shape to copy.
- [ ] **The gate, once** — `python tools/dossier.py --only tools:editor/the-vs-code-extension
      --gate` at `docs/agent/loop-goal.toml:5364`, which today refuses with *not on the roster*
      rather than with a missing proof.

## Backlog

- The `sink` value of `nvs.taint.mark` needs a third kind and `rule:security/sink-predicate`'s
  classification first — ADR 0101 § *Open questions* owns whether it is built.
- A parameter's and a property's declaration carry no marker; the gap is
  `crates/nvs-lsp/src/redactions.rs` § *What it does not reach* and it is a `nvs-types` change.
- Stage 7 is untouched: Tasks with a `problemMatcher`, and the AST panel's redaction obligation
  (`docs/agent/loop-goal.md` § *Stage 7*).
- Stage 8's extension-host suite is CI's alone and never run here (goal § *Standing decisions*).
