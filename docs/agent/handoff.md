# Handoff

## State

**Goal `workspace-index`, stage 3 is open and two of its five readers have landed.** The index now has
a home: `serve` holds one `SymbolIndex` beside `Documents` at `crates/nvs-lsp/src/server.rs:146` and
refreshes it from the `Changed` every document-sync notification produces, so a file nobody opened is
indexed and stays current. `textDocument/references` and `textDocument/documentHighlight` are answered
from it, the second being the first filtered to the open document.

`crates/nvs-lsp/tests/references.rs` drives the real server over `Connection::memory()` and pins the
four claims: the answer reaches a file the client never opened, `includeDeclaration` adds the
declaration and only then, a highlight never leaves the open file, and an edit is in the next answer.
`call_hierarchy_is_not_answered` is green. Stage 3's gate still owes `all_five_readers_query_the_one_index`
and `unused_member_dimming_is_silent_at_open_scope_and_correct_at_workspace_scope`.

**Nothing transports a setting to the server, and both remaining readers are gated on one** — CodeLens
on `nvs.codeLens.enable`, dimming on `nvs.check.scope`. Neither `initializationOptions` nor
`workspace/configuration` is read anywhere in `crates/nvs-lsp/src/server.rs`, so that is one slice of
its own and it is the next group's first. Nothing is blocked. The goal's one ADR is still unopened.

## Next group

**Stage 3: the settings a reader is gated on, then the readers** — one file set:
`crates/nvs-lsp/src/server.rs`, `crates/nvs-lsp/src/index.rs`, `crates/nvs-lsp/src/capabilities.rs`,
`crates/nvs-lsp/src/diagnostics.rs`, `crates/nvs-lsp/tests/handshake.rs`.

- [ ] **The two settings reach the server.** Read `initializationOptions` off the `InitializeParams`
      already deserialized at `crates/nvs-lsp/src/server.rs:121` and hold `nvs.check.scope` and
      `nvs.codeLens.enable` beside the index at `crates/nvs-lsp/src/server.rs:146`, which today passes
      `CheckScope::default()` and a `None` root. `rule:ide/contributions-are-frozen-and-only-ever-added`
      is the roster both names are frozen in and `rule:ide/check-scope-defaults-to-open-documents` is
      the default and what a `Workspace` pass needs the root for.
- [ ] **A declaration records its visibility.** `crates/nvs-lsp/src/index.rs:69` is the gap this goal
      owns, and it closes in the member walk at `crates/nvs-lsp/src/index.rs:512`: a field on
      `Declaration` read off `ClassMember`'s own modifiers where the name is already being read, never
      a second walk. Dimming is the only reader that needs it.
- [ ] **Unused-member dimming, silent at the default scope.**
      `rule:ide/check-scope-defaults-to-open-documents`: one diagnostic per private declaration with no
      occurrence in the index, carrying LSP's `Unnecessary` tag, produced where the rest are at
      `crates/nvs-lsp/src/diagnostics.rs:139` and reaching the wire through the index
      `crates/nvs-lsp/src/server.rs:146` holds.
- [ ] **CodeLens and type hierarchy**, behind `nvs.codeLens.enable`. Declared at
      `crates/nvs-lsp/src/capabilities.rs:152` and dispatched from `answer` at
      `crates/nvs-lsp/src/server.rs:198`; the lens counts `SymbolIndex::occurrences` and the hierarchy
      reads `declarations_in`, so neither walks the front end.
      `rule:ide/five-features-are-one-reference-index` names both.

## Backlog

- The goal's one ADR is unopened and next free is 0171 — `docs/agent/goals/38-workspace-index.md`
  § *Standing decisions* says what it covers.
- No `.lspt` case exists for either landed request; stage 3's `nvs-suite --coverage` check needs a
  `Request` variant in `crates/nvs-lsp/src/case.rs` and a `Response` one in `render.rs`.
- `[context]` gap: stage 3's file set does not name `crates/nvs-lsp/tests/handshake.rs`, and every
  request admission in stages 3 to 6 edits its closed capability list.
- A cursor on a *declaration* is answered nothing: `definition::named_at` reads the expression table
  and a declared name is not an expression — `crates/nvs-lsp/src/index.rs` module doc.
