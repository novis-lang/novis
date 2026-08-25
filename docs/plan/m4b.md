# M4B — Minimal `mwl-lsp` and the VS Code extension (~3 weeks)

Pulled ahead of M10 by [ADR 0040](../adr/0040-vscode-deep-tooling-and-resilient-parsing.md) so real-world
testing in an editor starts the moment M4 makes MWL a usable CLI language, rather than after M5–M9.
`crates/mwl-syntax` gains a second, error-recovering parse entry point — a lossless tree, in the shape
rust-analyzer's `rowan` popularized, that keeps producing a usable structure around a syntax error instead
of aborting — used only by the pieces below; `mwl check`/`mwl run` keep the existing strict, all-or-nothing
parse unchanged. `crates/mwl-lsp` (`tower-lsp`) ships its first, minimal slice: diagnostics (via `mwl
check` run against the resilient tree), hover (declared types), go-to-definition, and keyword/member
completion — no workspace-wide symbol search or code actions yet, that's M10. `editors/vscode` ships
alongside it: `.mwl` registration, a TextMate grammar, `language-configuration.json`, `mwl lsp` process
spawning, a `LanguageStatusItem` for server health, `mwl run`/`mwl test` as VS Code Tasks, and an AST
explorer panel backed by the CLI's existing `mwl ast` command (no new language feature needed for that
one). No formatting support yet (`mwl fmt` doesn't exist until M10) and no PhpStorm work — PhpStorm stays
entirely at M10, per [ADR 0016](../adr/0016-ide-integration.md).

**Verify:** typing an incomplete statement (unclosed brace, trailing `->`) does not stop
diagnostics/hover/completion from working on the well-formed code around it — the resilient-parse mode's
core claim. The VS Code extension activates on `.mwl`, shows TextMate colour immediately and semantic-token
colour once `mwl-lsp` responds, and diagnostics/hover/go-to-definition/completion round-trip through it
with no logic duplicated into the extension. The AST panel renders `mwl ast --json`'s tree for the active
file.
