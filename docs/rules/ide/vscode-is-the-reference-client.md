The VS Code extension is a standard `vscode-languageclient` package and the reference client. It
registers the `nvs` language ID and the `.nvs` association (`rule:ide/nvs-is-its-own-file-type`), a
`language-configuration.json` for bracket matching, comment toggles, auto-closing pairs and indentation
— covering `spawn script`, `type` aliases and the type-annotation syntax — and a
TextMate grammar for the `<?nvs ?>` / `<?= ?>` plus inline-HTML lexer mode, so a file has correct-enough
colour the moment it opens and before the server has parsed anything.

It spawns `nvs lsp` from a configurable path setting, falling back to `PATH`, and layers LSP semantic
tokens over the TextMate baseline once the server is live — the two-layer pattern rust-analyzer and
Deno use. `editor.formatOnSave` and the format commands go to `textDocument/formatting` and
`rangeFormatting` against `nvs-fmt`; `nvs run` and `nvs test` are VS Code Tasks and a "Run File"
command. A `secret` value's bytes are concealed by default on ranges the server hands over, and
`tainted` gets no default decoration (`rule:ide/tainted-has-no-default-decoration`).

Nothing in that list is language logic (`rule:ide/one-server-two-thin-clients`). The concrete
contribution roster — setting and command identifiers — is frozen elsewhere; this is the shape.
