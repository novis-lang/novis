The VS Code extension is a standard `vscode-languageclient` package and the reference client. It
registers the `nvs` language ID and the `.nvs` association (`rule:ide/nvs-is-its-own-file-type`), a
`language-configuration.json` for bracket matching, comment toggles, auto-closing pairs and indentation
— covering `spawn script`, `type` aliases and the type-annotation syntax — and a
TextMate grammar for the `<?nvs ?>` / `<?= ?>` plus inline-HTML lexer mode, so a file has correct-enough
colour the moment it opens and before the server has parsed anything.

It spawns `nvs lsp` from a configurable path setting, falling back to `PATH`, and layers LSP semantic
tokens over the TextMate baseline once the server is live — the two-layer pattern rust-analyzer and
Deno use. `editor.formatOnSave` and *Format Document* run `nvs fmt --stdin` over the buffer from the
client (`editors/vscode/src/format.ts`), then the editor's own HTML formatter over the markup regions
`nvs/regions` reports (`rule:ide/a-template-region-gets-the-editors-services-and-formatter`). The
server declares no formatting provider, so a `.nvs` file has one formatter
(`rule:tooling/fmt-is-never-a-diagnostic`), and the extension registers whole-document formatting only.
`nvs run` and `nvs test` are VS Code Tasks and commands. A `secret` value's bytes are concealed by
default on ranges the server gives, and `tainted` gets no default decoration
(`rule:ide/tainted-has-no-default-decoration`).

Nothing in that list is language logic (`rule:ide/one-server-two-thin-clients`). The concrete
contribution roster — setting and command identifiers — is frozen by
`rule:ide/contributions-are-frozen-and-only-ever-added`; this is the shape.

**Declined:**
- **A PhpStorm plugin** bridging JetBrains' LSP client to the same `nvs lsp` and `nvs fmt`. It is a
  second editor toolchain (Kotlin, Gradle and the IntelliJ Platform SDK) with its own CI job, and every
  editing aid a client owns would be written twice. VS Code is the one editor client.
- **Formatting a selection.** `nvs fmt` lays out a whole file and has no range form, so a range
  formatter would be a second layout implementation in the client or the server.
