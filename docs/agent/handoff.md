# Handoff

## State

**Goal `workspace-index`, stage 2 is closed and green.** `crates/nvs-lsp/src/index.rs` is the one
workspace symbol index: `SymbolIndex::build` walks the tree `CheckScope` selects, `absorb` is the only
place a file's entry is built, `refresh` drops the file that changed and the files whose analysis read it
and rebuilds exactly those, and the queries are `declaration`, `occurrences`, `declarations_in` and
`occurrences_in`. Its module doc carries the four decisions behind that shape; the known gaps are in it
too, and the one stage 3 needs is that nothing records member **visibility** yet.

**The index has no home in the server.** Nothing in `crates/nvs-lsp/src/server.rs` holds one, so it is
built by `tests/index.rs` and `tests/latency.rs` and by nobody else. Giving it one — beside `Documents`
in `serve`, refreshed from `apply`'s `Changed` — is stage 3's first slice and its first design question.

`analyse` is now two functions: `analyse_file(documents, path, version)` at
`crates/nvs-lsp/src/document.rs:381` is the walk with the entry named by path, and `analyse` is that call
with the open buffer's own version. That is what lets a file nobody opened be indexed.

**The `\` defect is still live and is stage 5's**, exactly as `docs/agent/loop-goal.md:30` says;
`capabilities.rs:174`'s trigger list is deliberately not trimmed. Nothing is blocked. Stages 3, 4, 5 and
6 are open; stage 1 is goal `editor-install`'s floor.

## Next group

**Stage 3: the five readers, and only five** — one file set: `crates/nvs-lsp/src/index.rs`,
`crates/nvs-lsp/src/server.rs`, `crates/nvs-lsp/src/capabilities.rs`, `crates/nvs-lsp/src/render.rs`,
`crates/nvs-lsp/tests/index.rs`. `rule:ide/five-features-are-one-reference-index` names the five and
excludes call hierarchy with a reason; every item below is one query against the index that already
exists, and none of them walks the front end.

- [ ] **The index gets a home, and references and highlight are one query.** Hold a `SymbolIndex`
      beside `Documents` in `serve` at `crates/nvs-lsp/src/server.rs:111` and refresh it from the
      `Changed` an edit produces at `crates/nvs-lsp/src/server.rs:559`; dispatch the two requests from
      `answer` at `crates/nvs-lsp/src/server.rs:171`, declaring them at
      `crates/nvs-lsp/src/capabilities.rs:152`. The cursor becomes a symbol through
      `crate::definition::named_at` at `crates/nvs-lsp/src/definition.rs:271` and the symbol becomes an
      answer through `SymbolIndex::occurrences` at `crates/nvs-lsp/src/index.rs:306` — highlight is the
      same query filtered to the open file, which is the whole of why it waited for this.
- [ ] **CodeLens and type hierarchy**, behind `nvs.codeLens.enable`. The reference count is
      `occurrences` again; implementors and overrides are `nvs_hir::ClassLinks` at
      `crates/nvs-hir/src/hierarchy.rs:89`, read back rather than re-derived. Test:
      `all_five_readers_query_the_one_index`, in `crates/nvs-lsp/tests/index.rs:136` beside the
      structural one it extends.
- [ ] **Unused-member dimming, silent at the default scope.** `rule:ide/check-scope-defaults-to-open-documents`
      is why it is silent rather than wrong under `"open"`. It needs the visibility the index does not
      record yet — add it where the member is already read, at `crates/nvs-lsp/src/index.rs:170`'s
      `Declaration`, not in a second walk. Test:
      `unused_member_dimming_is_silent_at_open_scope_and_correct_at_workspace_scope`.
- [ ] **`call_hierarchy_is_not_answered`**, in `crates/nvs-lsp/tests/index.rs:136`: the rule excludes it
      because it is a different index, and a test is what keeps the five from becoming six. A `.lspt`
      case per added request lands with each item above, answered from
      `crates/nvs-lsp/src/suite.rs:258`.

## Backlog

- `nvs.check.scope` and `nvs.checkWorkspace` are not contributed in `editors/vscode/package.json` yet;
  they are frozen in `rule:ide/contributions-are-frozen-and-only-ever-added`'s roster and are added once
  the server reads a scope from the client, not before.
- This goal's one ADR is still unopened — `docs/agent/loop-goal.md:149` says it covers stage 2's index
  shape with stages 4, 5 and 6. Stage 2's shape is in `crates/nvs-lsp/src/index.rs`'s module doc until
  it is written; the record is frozen once, so it waits for the other three stages.
- `rule:ide/five-features-are-one-reference-index` is still `designed`; it ships when the five readers
  do, and its `guardedBy` should name `crates/nvs-lsp/tests/index.rs` then.
- An enum case occurrence is recorded against its enum — `crates/nvs-lsp/src/index.rs`'s known gaps.
