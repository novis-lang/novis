# Handoff

## State

**Goal 14 answers three of the ten requests, all three projections of one analysis.**
`documentSymbol` (`crates/nvs-lsp/src/symbols.rs`), `foldingRange`
(`crates/nvs-lsp/src/folding.rs`) and `documentLink` (`crates/nvs-lsp/src/links.rs`) each walk what
`crate::analyse` already holds; none takes a cursor and none asks the type phase anything, which is
`rule:ide/the-request-set-is-closed`'s "a projection rather than a feature" applied three times.
Each module's doc owns what it decided — why a fold ends one line short of its last, why an import
run is the only range covering several statements, why an `autoload` root gets no link.

**`crates/nvs-lsp/src/server.rs:153`'s `answer` has three arms and one refusal each way**:
`MethodNotFound` outside the list, and `unreadable` for a payload inside it that will not
deserialize — one message rather than one per arm. `crates/nvs-lsp/src/suite.rs:181` is the same
seam for a `.lspt` case and has the same three. `crates/nvs-lsp/src/position.rs`'s `range_at` is now
the one span-to-range conversion, and `symbols` reads it rather than its own copy.

**`nvs lsp-test tests/lsp/` reports `20 passed, 0 failed`**, against the goal's floor of 160.

**No cursor request can land yet, and nothing new is needed for one.**
`crates/nvs-lsp/src/document.rs:315` calls `parse_file`, so an `Analysed` carries statements while
the trivia and the `SyntaxIndex` the same parse builds are dropped on the floor — that is the next
group, and it is all in this crate.

## Next group

**Stage 4: the resilient parse reaches the analysis, and the first cursor request with it** — one
file set: `crates/nvs-lsp/src/document.rs`, `crates/nvs-lsp/src/folding.rs`,
`crates/nvs-lsp/src/server.rs`, `crates/nvs-lsp/src/suite.rs`, with
`crates/nvs-syntax/src/parser/mod.rs` and `crates/nvs-syntax/src/index.rs` read only. Everything
the four remaining requests wait on is already built in `nvs-syntax`.

- [ ] **`Analysed` carries the entry document's whole `Parsed`.** `nvs_syntax::parse`
      (`crates/nvs-syntax/src/parser/mod.rs:871`) answers the same statements plus the trivia and
      the `SyntaxIndex` (`crates/nvs-syntax/src/index.rs:103`); keep both beside the statements on
      `Analysed` (`crates/nvs-lsp/src/document.rs:253`), and call it from
      `crates/nvs-lsp/src/document.rs:315`. A required file keeps the strict parse: a cursor is only
      ever in the open one. `rule:ide/one-grammar-one-tree`.
- [ ] **`selectionRange`, which is the ancestor list unchanged.** `SyntaxIndex::at`
      (`crates/nvs-syntax/src/index.rs:136`) into `crates/nvs-lsp/src/render.rs:386`'s `ancestry`,
      with the arms at `crates/nvs-lsp/src/server.rs:153` and `crates/nvs-lsp/src/suite.rs:181` and
      the first `<|>` cases under `tests/lsp/selection/`. `rule:ide/the-index-answers-the-cursor`.
- [ ] **`foldingRange`'s comment blocks, out of that trivia.** A run of `LineComment`/`DocComment`
      trivia is one fold carrying LSP's `comment` kind, which
      `crates/nvs-lsp/src/folding.rs:58`'s walk is the one kind it cannot reach today.
      `rule:ide/one-grammar-one-tree`.

## Backlog

- An `autoload` root has no literal-to-path edge to read; adding one beside `Loaded::requires`
  is what a link for it needs — `crates/nvs-lsp/src/links.rs`' module doc.
- `hover`, `definition` and `completion` are prose stage 6 and all wait on the same index —
  `docs/agent/loop-goal.md` § *Stage 6*.
- `semanticTokens/full` is prose stage 7's other half and needs the qualifiers the type phase
  computes, not the tree.
- The `.lspt` floor is 160 passing cases against 20 on disk; every request slice ships its own.
