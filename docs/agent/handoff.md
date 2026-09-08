# Handoff

## State

**Goal 14, stage 9 is closed: the coverage matrix over `tests/lsp/` has no empty cell.**
`nvs lsp-test tests/lsp/` is at `189 passed, 0 failed` against the goal's floor of 160, the
vocabulary is 26 constructs, and `crates/nvs-lsp/tests/coverage.rs` is the gate that reads the
matrix. It asserts `crates/nvs-lsp/src/coverage.rs`'s own split and nothing stronger: the five
cursor rows each owe the whole vocabulary, the six document-wide rows owe it between them, and each
of those six owes a non-empty row of its own. It is not vacuous — `ClassConstAccess` had an empty
document-wide half at the commit before it, and the message names each cell it finds.

**`ClassConstAccess` was closed by reading the checker's table, not the grammar.**
`crates/nvs-lsp/src/semantic.rs` now colours `enumMember` on the member half of an access
`nvs_types::ExprInfo::EnumCase` was recorded against, and leaves a plain `Config::MAX` silent — LSP's
legend has a name for one of the two and none for the other. That module's § *Decision: what only a
resolution knows is read off the checker's table* is the home of why; no `tests/lsp/semantic/` case
needed re-freezing, because none read an enum case before this one.

**Stage 10 is the next red check and its test does not exist yet** — an open item, not a regression.

## Next group

**Stage 10: the latency guard** — one file set: a new `crates/nvs-lsp/tests/latency.rs`,
`crates/nvs-lsp/src/document.rs`, and `benches/abi-probe/tests/perf_guards.rs` read for its shape
only.

- [ ] **A full re-analysis of a ~1,000-line document is measured** —
      `rule:ide/a-full-reanalysis-stays-under-a-bound`. The subject is
      `crates/nvs-lsp/src/document.rs:453`, `analyse_current`, over a document the test generates
      rather than one kept on disk; `crates/nvs-lsp/tests/coverage.rs` is the nearest example of an
      integration test in this crate that drives the whole front end. The guard's shape is
      `benches/abi-probe/tests/perf_guards.rs`, which is 1,363 lines — locate one guard in it and
      read around that, never the file.
- [ ] **The bound is a number written down where it can be read** — the rule asks for a *named*
      bound and nothing names one. The goal's § *Standing decisions* sends it to this crate's own
      doc rather than to an ADR, so it belongs in the guard's module doc beside what it was measured
      on and what the answer is if it ever goes red — item-level caching over
      `crates/nvs-lsp/src/document.rs:374`'s parse, never a smaller number in the test.

## Backlog

- Stage 11, the reference chapter, is a `command` check — `docs/agent/loop-goal.toml:5334`.
- Stage 12, guards — `docs/agent/loop-goal.toml`, after stage 11.
- `Foo::class` colours neither half; the `class` keyword is the TextMate layer's —
  `crates/nvs-lsp/src/semantic.rs`'s module doc.
- The matrix is a ratchet now: a case at a construct nobody reached obliges six more rows —
  `crates/nvs-lsp/src/coverage.rs:46`.
