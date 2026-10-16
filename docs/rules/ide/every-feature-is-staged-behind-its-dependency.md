VS Code is the reference client and gets real depth, not an LSP passthrough with a grammar file:
inspections and quick fixes, refactorings, signature help and workspace symbol search, inlay hints, a
native Test Explorer with coverage, an AST panel, a profiler view, debugger wiring, HTML/CSS/JS services
inside an inline-HTML region (the template, given `rule:programs/first-party-framework`), and the four
"write" actions that implement a member, override a method, declare the function just called, or narrow
an `array<mixed>` annotation to its literal.

Each is a commitment tagged with the language or runtime piece it needs, and lands with that piece
rather than ahead of it as a stub. The server's request set is `rule:ide/the-request-set-is-closed`.
The workspace index, `nvs fmt`, `nvs dap` and the profiler gate the rest: format-on-save waits for the
formatter, the debugger UI for the adapter (`rule:ide/the-debug-adapter-does-not-wait-for-an-editor`),
references, CodeLens, type hierarchy and unused-member dimming for the one index that answers all of
them, `nvs ext` commands for `.nvsx` extensions existing. Two are committed to no milestone at all: a
`Core\Reflect`-backed live object inspector, and a request-tree view of `spawn` during a debug session,
which needs a DAP protocol extension nobody has designed.

**Built:** the quick fixes a diagnostic carries, signature help, inlay hints, references, document
highlight, CodeLens, type hierarchy, unused-member dimming, the Test Explorer with coverage, the AST
panel, the inline-HTML region's services and formatting through `nvs fmt`
(`crates/nvs-lsp/src/capabilities.rs`, `editors/vscode/src/`). **Missing:** rename, extract to method or
variable, workspace symbol search, the profiler view, the debugger wiring and the four write actions.

Completion from the compiler's own tables — route names, `nvs.toml` directives, `#[Api]` fields — is
offered only where the compiler already derives the value for another reason, never from a convention
scan or an annotation dialect. That closed rule is the whole answer to "framework support", and why no
per-framework module enters `nvs-lsp`.

**Declined:**
- **Diagnostic severity configurable per file**, through `.editorconfig`, settings globs or tags in the
  source. It is the door `rule:core-api/casing-has-no-suppression` keeps closed.
