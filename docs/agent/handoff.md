# Handoff

## State

**Goal `fmt` (M10) is closed: both stage-7 checks pass.** The whole corpus is a fixed point and
parses to the same tree after formatting (`crates/nvs-fmt/tests/corpus.rs`), a comment in every
position the grammar allows survives where it started (`crates/nvs-fmt/tests/fixtures.rs`, over the
new `tests/fmt/{input,formatted}/comments.nvs` pair), and every `tooling/fmt-*` rule is `shipped`
with its `guardedBy` filled. Nothing is blocked.

**The floor's eight red example fixtures were a dead container stack, not the tree.** The host had
rebooted and `[docker] memoize_on` skipped the bring-up, so `examples/cache.nvs` reported a `W1008`
warning over a redis nothing was listening on; after `docker compose -f tests/db/compose.yaml up -d`
it exits 0 with all five of its `want` lines. The playbook bullet is the standing note.

**What the formatter still leaves to the author is `crates/nvs-fmt/src/lib.rs`'s `# Known gaps`.**
That list now names a comment-only line: the index answers at no node for one, so it keeps the
indentation its author wrote — except inside a `match`, where the arm it precedes places it.

## Next group

**Stage 7 is closed, so what is left is `nvs-fmt`'s own gap list** — one file set:
`crates/nvs-fmt/src/indent.rs`, `crates/nvs-fmt/src/print.rs` and `crates/nvs-fmt/src/tokens.rs`.

- [ ] **A comment-only line is indented like the code it precedes** —
      `rule:tooling/fmt-base-style-is-per`, whose four-spaces-per-body claim reaches every line and
      today skips this one. `crates/nvs-fmt/src/indent.rs:116`'s `of_line` answers `None` for an
      offset that begins no node, and the arm-list branch above it is the one case that already
      answers; gap 2 of `crates/nvs-fmt/src/lib.rs:72` is the statement of what is missing.
- [ ] **A rewrite landing inside a trivium refuses rather than corrupts** —
      `crates/nvs-fmt/src/print.rs:183` is a `debug_assert`, so the release binary writes the bytes
      anyway and the file it was handed comes back mangled. A formatter may panic; it may not
      silently change a program.
- [ ] **A literal inside an attribute is respelled like any other** — gap 4 of
      `crates/nvs-fmt/src/lib.rs:86`. `crates/nvs-fmt/src/tokens.rs:209` respells a literal the
      index has a node for, and `crates/nvs-syntax/src/walk.rs` builds none for an attribute's
      argument list, so `#[Core\Command(name: "greet")]` keeps its double quotes.

## Backlog

- The `guardedBy` of `ide/tokens-plus-trivia-reproduce-the-file` is still empty though
  `crates/nvs-syntax/tests/lossless.rs` holds it — `docs/rules/ide.json`.
- `tests/fmt/` holds four pairs; a pair per landed rule is what makes one readable in isolation —
  `crates/nvs-fmt/tests/fixtures.rs`'s own doc says how to add one without Rust.
