The first `nvs lsp` is built on `lsp-server` and `lsp-types`, synchronously, with no async runtime in
the workspace. It answers six requests, and the list is closed: `textDocument/publishDiagnostics` by
running the `nvs check` pipeline over the resilient tree (`rule:ide/the-tree-survives-a-syntax-error`);
`hover` from declared types, a `Core` member's registry signature and a declaration's own doc comment
out of the trivia layer; `definition`; `completion` restricted to keywords, members off a resolved
receiver type and enum cases — no cross-file symbol search, which needs the workspace index;
`semanticTokens/full`; and `documentSymbol`. A `LanguageStatusItem` reports the server's health and
version.

Plus exactly two code actions: the casing fix (`rule:core-api/identifier-casing`) and `(int)$x` →
`$x as int` (`rule:types/no-legacy-cast`). They are admitted because their replacement text already sits
in the diagnostic, and that fact — not a judgement about cost — is the boundary that keeps the list from
creeping toward the full catalog (`rule:ide/a-quick-fix-is-a-diagnostics-own-suggestion`).

Not in the first server: format-on-save, because `nvs fmt` does not exist yet; rename; and any code
action beyond the two. Everything else waits for its dependency
(`rule:ide/every-feature-is-staged-behind-its-dependency`).

The first server and extension land as soon as a non-trivial CLI program runs correctly, not after the
concurrency, configuration, HTTP and extension work. An editor exercising the language early surfaces
rough edges in the type system, the stdlib's shape and the diagnostics' wording while they are still
cheap to change.

**Declined:**
- **The editor client lands only with the full tooling milestone.** Real use in an editor would start
  after the server, the stdlib and the extension system, when what it finds is expensive to change.
