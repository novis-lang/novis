# Handoff

## State

**Goal 14 — `nvs lsp` — has just started; nothing of it has landed yet.** Goal 13's whole list is this
goal's Stage 1 floor. `crates/nvs-lsp` does not exist.

**Stage 0 is empty because goal 12 emptied it.** The `SyntaxIndex`, explicit recovery
(`MemberName::Missing`, a span on `ExprKind::Error`) and `utf16_col`/`offset_of` are all on disk. **No
position conversion is written in this crate** — `nvs-diagnostics` is that rule's one home, and a second
copy here is the bug the rule exists to prevent.

The order of the first two stages is not a preference: **no request handler is written until the case
format and its runner are green.** A request built before `.lspt` exists is a request nobody can prove,
and the driver has no other way to see an answer that nothing prints.

## Next group

**Stage 2: the crate, the subcommand, the handshake, and the wire** — one file set: a new
`crates/nvs-lsp/`, `crates/nvs-cli/src/main.rs`, and the workspace manifests.

- [ ] **The crate and its two dependencies.** `lsp-server` and `lsp-types`, synchronous, stdio. Each owes
      three things: the `[workspace.dependencies]` line saying why that crate, `cargo deny check`, and
      `python tools/gen-attribution.py`.
- [ ] **`nvs lsp`**, and an `initialize` that declares exactly this goal's capabilities and no others, and
      reports the binary's version.
- [ ] **Position encoding negotiated per LSP 3.17** — offer `utf-8` and `utf-16`, take `utf-8` when
      offered, over goal 12's conversions.
- [ ] **Nothing but the protocol writes to stdout.** No `clippy::print_stdout` allowance in this crate, and
      the named test that no crate the server links calls `println!`.

## Backlog

- Stage 3 (`.lspt`, `nvs lsp-test`, `nvs_lsp::render`) is the same file set plus `crates/nvs-test`, and it
  is the next group whatever else happens. `docs/agent/conventions.md` § *An `.lspt` case* already
  specifies the format — implement that, do not redesign it.
- Stages 4 to 9 are the requests, in the prose's order. Stage 6 (`hover`, `definition`, `completion`) is
  one group of three and never fewer than one per session; stage 7's five projections are one walk each.
- **Off path:** anything under `editors/`, the TextMate grammars, `nvs fmt`, rename, extract, workspace
  symbol search, inlay hints, signature help, `documentHighlight`. The first three of those look adjacent
  to this goal and are not — `rule:ide/five-features-are-one-reference-index` owns them at M10.
- When this goal's last check goes green the chain advances to goal 15 — `editors/vscode`, the last of
  M4B's four. Two entries follow it: goal 16 (the body rule and the JSON readers) and goal 17
  (`Core\Test::request`'s shape), added after this file was written.
