# ADR 0016 — IDE integration is a thin per-editor client over one language server; PhpStorm goes LSP-bridge before native

- **Status:** Accepted
- **Date:** 2026-08-20
- **Scope:** the editor-facing half of M10 — the VS Code extension, the PhpStorm plugin, how each drives
  `nvs-lsp`/`nvs-fmt`, what "tight IDE integration" includes and excludes for v1, and where the two client
  packages live in the repository layout
- **Amends:** [docs/implementation-plan.md](../implementation-plan.md) M10 — the verify line "VS Code and
  PhpStorm both drive the LSP" undersold what each editor actually needs to feel first-class; this ADR
  replaces it with the concrete per-editor client scope in *Decision*, and *Consequences* names what M10's
  estimate now carries that it did not before.
- **Amended by:** 0040, 0099, 0101

> **In short:** a language server is necessary but not sufficient in either editor. VS Code always needs a
> thin client extension regardless of the server behind it — something has to spawn `nvs lsp`, register the
> `.nvs` language and a TextMate grammar for instant syntax colour, and wire `nvs fmt` into format-on-save;
> LSP supplies the smarts (completion, hover, diagnostics, rename, go-to-definition, formatting), the
> extension supplies the editor-side plumbing. PhpStorm is the harder case: JetBrains IDEs are PSI-based, not
> LSP-native, so "just point PhpStorm at the LSP" is not a zero-cost default the way it nearly is in VS Code.
> Novis builds **exactly one** implementation of each smart feature — `nvs-lsp` and `nvs-fmt` — and ships two
> thin, per-editor clients around it: a VS Code extension as the reference client, and a PhpStorm plugin that
> bridges to the same server through JetBrains' LSP client support rather than reimplementing language
> smarts as PSI. A full native PhpStorm plugin (its own lexer/PSI/parser, PhpStorm-grade refactoring and
> debugger UI) is named as a later, explicit decision if usage justifies it — not built now, and not silently
> skipped either. Debug-adapter editor wiring (VS Code's `DebugAdapterDescriptorFactory` + `launch.json`
> schema, PhpStorm's native debugger UI) is likewise named and deferred: `nvs dap` itself still ships in M10,
> but hooking either editor's debugger UI to it is a tracked fast-follow, not a condition of "M10 done."

## Context

- M10 already commits to `nvs fmt`, `nvs lsp` (`tower-lsp`), and `nvs dap`; its verify line "VS Code and
  PhpStorm both drive the LSP" undersold the real per-editor client work needed.
- VS Code: LSP alone gives no `.nvs` language registration and no syntax colour before the server's first
  parse — pure client-extension plumbing with no LSP-server substitute.
- PhpStorm: JetBrains IDEs resolve/highlight/refactor through PSI trees, not LSP-native. JetBrains' LSP
  client support (or community LSP4IJ) gets the same core LSP feature set as VS Code, but not PSI-level
  refactoring, structural search/replace, or deep debugger UI — closing that gap means a genuine second
  front end (a Kotlin plugin with its own lexer/parser/PSI mirroring `nvs-syntax`'s grammar), a sub-project on
  the scale of the PHP plugin itself.
- Three open questions this ADR resolves: how deep PhpStorm integration goes (LSP-bridge vs. native PSI),
  whether debugger UI wiring counts toward v1 "done," and where the client packages live — resolved in the
  same phased spirit as `rule:packaging/an-extension-is-a-sandboxed-wasm-component`'s wasm-before-native tiers.

## Decision

### 1. One server, two thin clients — the split of responsibility

`nvs-lsp` and `nvs-fmt` are the **only** place language smarts and formatting logic live. Neither editor
client re-implements completion, diagnostics, hover, rename, go-to-definition, or code formatting locally —
each is a thin adapter that starts the server/formatter process, translates its own editor's UI events into
LSP requests, and renders the results. This is the same "state a fact once" rule
[AGENTS.md](../../AGENTS.md) already applies to documentation, applied here to executable behaviour: two
independent reimplementations of, say, Novis's formatting rules would drift the moment one editor's plugin
fixes a bug the other's has not.

### 2. VS Code extension

A standard `vscode-languageclient` extension:

- Registers the `nvs` language ID, `.nvs` file association, and a `language-configuration.json` (bracket
  matching, comment toggles, auto-closing pairs, indentation rules) — largely PHP's, adjusted for
  `spawn script`, `type` aliases, and the type-annotation syntax `rule:types/declaration`
  adds that PHP has no syntax for.
- Ships a **TextMate grammar** for the dual-mode `<?nvs ?>` / `<?php ?>` / `<?= ?>` + inline-HTML lexer mode
  M1 builds, giving instant, correct-enough syntax colour the moment a file opens — before the language
  server has warmed up or finished its first parse.
- Spawns `nvs lsp` (resolved via a configurable path setting, falling back to `PATH`) and layers **LSP
  semantic tokens** on top of the TextMate baseline once the server is live — the same two-layer pattern
  rust-analyzer and Deno's extension use, not a novel design.
- Wires `editor.formatOnSave` and the "Format Document"/"Format Selection" commands to
  `textDocument/formatting` / `textDocument/rangeFormatting` against `nvs-fmt` — no bundled formatter logic
  of its own.
- Surfaces `nvs run`/`nvs test` as VS Code Tasks and a "Run File" command. A full custom Test Explorer
  provider is not in this ADR's scope — see *Revisiting*.
- **Conceals a `secret` value's bytes by default**, on ranges the server computes and hands over through
  `nvs/redactions` — the client draws a decoration and decides nothing, per *Decision § 1*. `tainted` gets
  no default decoration at all, for a reason that is this ADR's own split rather than a preference: the
  server knows which is which and the client would have to guess.
  `rule:security/redaction-ranges-come-from-the-server` owns the rule,
  the reveal, and the list of leak surfaces VS Code gives no way to close.

**The concrete contribution roster for all of the above is [ADR 0099](0099-the-resilient-tree-is-the-ast-plus-trivia.md)
§ 6**, which moves the extension to M4B and freezes its setting and command identifiers; this section is the
shape, that one is the list.

### 3. PhpStorm plugin — LSP-bridge now, native PSI later

The PhpStorm plugin drives the **same** `nvs-lsp` binary the VS Code extension does, through JetBrains
Platform's LSP client support (falling back to the community LSP4IJ plugin if the bundled API proves
insufficient for a needed feature). Concretely, it must:

- Register `.nvs` as its **own** file type and language, distinct from PhpStorm's bundled PHP support — this
  is not optional polish. Without an explicit registration, PhpStorm's own PHP plugin may attempt to claim
  `.nvs` files (or a generic-text fallback will), and either failure mode looks like "the plugin doesn't
  work" to a user with no diagnostic pointing at the real cause.
- Provide a TextMate-or-equivalent baseline grammar for the same instant-colour reason as the VS Code
  extension — PhpStorm supports bundling a TextMate grammar for a custom file type without writing a full
  IDEA lexer.
- Route completion, hover, diagnostics, rename, and go-to-definition through the LSP bridge to `nvs-lsp`.
- Route formatting through the LSP bridge to `nvs-fmt`. This is a deliberate cost, named up front rather
  than discovered later: PhpStorm's native Formatter framework and Code Style settings page do not apply,
  so "Reformat Code" runs Novis's one canonical formatter with no PhpStorm-native style customization UI. That
  is the price of *not* forking formatting logic into a second implementation; see *Alternatives rejected*.
- **Explicitly not build**, in this phase: a native PSI tree, PhpStorm-grade refactoring (rename-across-files
  beyond what the LSP `rename` request already gives, structural search/replace, intention actions backed by
  its own inspector), or PhpStorm's native debugger UI. These are the real quality gap between the
  LSP-bridge plugin and PhpStorm's PHP support, and they are the reason *Revisiting* keeps a native plugin
  on the table rather than closing the question.

### 4. Debugger UI wiring is deferred, `nvs dap` itself is not

M10 still builds `nvs dap` on schedule — the debug adapter, using safepoints for breakpoints. What this ADR
defers is the **editor-side wiring**: VS Code's `DebugAdapterDescriptorFactory` registration and
`launch.json` configuration schema, and PhpStorm's native `XDebugger` UI wired to a DAP-speaking backend.
Both are real, separate engineering work — a working debug adapter and a working debugger *UI* in a given
editor are two different integrations, the same way a language server and a syntax-highlighting extension
are two different integrations in *Decision §§ 1–2*. Shipping `nvs dap` without either editor's UI wired to
it is a complete, testable milestone deliverable on its own; the editor wiring is named in *Revisiting* as
a tracked fast-follow, not folded silently into "M10 done" and not silently dropped either.

### 5. Repository layout

```
editors/
  vscode/      TextMate grammar, language-configuration.json, LSP client extension    M10
  phpstorm/    file-type registration, LSP-bridge plugin (Kotlin/Gradle)              M10
```

Created when M10 starts, the same rule [AGENTS.md](../../AGENTS.md) and the
[README](../../README.md#repository-layout) already state for every crate: nothing sits scaffolded and
empty ahead of its milestone. Neither directory is a Cargo crate — the VS Code extension is TypeScript/Node
tooling, the PhpStorm plugin is Kotlin/Gradle/IntelliJ Platform tooling — so both sit outside the Rust
workspace `Cargo.toml` governs, alongside (not inside) `crates/`.

## Consequences

**Positive**

- Formatting and language semantics have exactly one implementation each (`nvs-fmt`, `nvs-lsp`), reached
  identically by both editors — the two-formatter-drift failure mode named in *Decision § 1* cannot occur
  structurally, the same reasoning `rule:statements/nothing-gets-a-second-name` already applies to a *name* applied
  here to *behaviour*.
- VS Code gets a normal, well-understood extension shape with no open design questions.
- PhpStorm gets real IDE support — completion, hover, diagnostics, rename, formatting, instant syntax colour
  — without committing to a multi-month native-plugin investment before anyone has used the language in
  PhpStorm at all.
- The debugger split in *Decision § 4* means a stalled or under-scoped editor-debugger integration cannot
  block `nvs dap` itself from shipping and being independently useful (e.g., a minimal CLI-driven debug
  client, or a third editor's own DAP wiring) inside M10.

**Negative**

- **PhpStorm's ceiling is lower than its own PHP support for as long as the LSP-bridge stands.** No
  PSI-level refactoring or structural search, and PhpStorm's native Formatter/Code Style UI does not apply to
  `.nvs` files — a PhpStorm user comparing Novis support to PHP support directly will notice the gap. Accepted
  deliberately; see *Alternatives rejected* for why closing it now was rejected.
- **M10's estimate absorbs real, previously-implicit scope**: a TextMate grammar (or equivalent) for both
  editors, a `language-configuration.json`, a Kotlin/Gradle PhpStorm plugin project with its own build and
  packaging, and the file-type-collision handling in *Decision § 3* — none of which "VS Code and PhpStorm
  both drive the LSP" named as work. The milestone's time estimate should be revised upward to reflect it
  rather than discovered as slip once M10 starts.
- **No debugger UI in either editor at the end of M10** under this ADR, even though `nvs dap` exists —
  a user who expects "set a breakpoint, click run" the moment M10 ships will not get it. This is the explicit
  trade named in *Decision § 4*, not an oversight.
- Two build toolchains (`npm`/`vsce` for VS Code, Gradle/IntelliJ Platform Plugin SDK for PhpStorm) join the
  project's CI surface, neither of them Rust — a genuinely new kind of CI job, distinct from everything
  `cargo` already builds.

## Alternatives rejected

- **Full native PhpStorm plugin from the start** (own PSI/lexer/parser). Rejected for now: a multi-month
  sub-project undertaken before any PhpStorm user has touched the language, same reasoning
  `rule:packaging/an-extension-is-a-sandboxed-wasm-component` used for wasm-before-native extensions — kept on the table, see
  *Revisiting*.
- **LSP-bridge only, permanently, no native option ever revisited.** Rejected: forecloses a legitimate future
  decision if real PhpStorm adoption makes the refactoring/debugger gap worth paying down.
- **A second, PhpStorm-native formatter implementation.** Rejected under *Decision § 1*: two implementations
  will drift the first time either one's rules change without the other noticing.
- **Wire debug-adapter editor UI into the same M10 pass as the language client work.** Rejected: the
  requirement this ADR answers scoped it out explicitly — tracked as a fast-follow instead, per *Decision §
  4*.
- **VS Code only, treat PhpStorm as out of scope.** Rejected: the requirement names PhpStorm explicitly, and
  skipping it would undercut the `nvs convert` adoption story.

## Revisiting

- **A full native PhpStorm PSI plugin**, if real usage shows the LSP-bridge ceiling (refactoring, structural
  search, debugger UI, native formatter integration) is actually costing adoption — see *Alternatives
  rejected*. Not scheduled to any milestone; this ADR only keeps the door open.
- **Debug-adapter editor wiring** — VS Code's `DebugAdapterDescriptorFactory` + `launch.json` schema,
  PhpStorm's `XDebugger` UI — as a fast-follow once `nvs dap` itself is verified, per *Decision § 4*.
- **Marketplace publishing and branding** (VS Code Marketplace publisher ID, JetBrains Marketplace listing,
  icon/branding assets, release cadence relative to the `nvs` binary's own versioning) is packaging detail
  this ADR does not resolve.
- **A VS Code Test Explorer provider** for `.nvst`/`nvs test`, beyond the plain Tasks integration
  *Decision § 2* commits to, if the plain version proves too thin in practice.
- **Whether the PhpStorm LSP bridge should use JetBrains' bundled LSP client API or the community LSP4IJ
  plugin** is left open pending a concrete evaluation once M10's PhpStorm work starts — *Decision § 3* names
  both as viable, not one as chosen.

Verification, in the order it becomes possible (M10):

- The VS Code extension activates on `.nvs`, shows TextMate-grammar colour before the language server has
  responded once, then shows semantic-token colour once it has; completion, hover, diagnostics,
  go-to-definition and rename round-trip through `nvs lsp`; format-on-save and the format commands round-trip
  through `nvs fmt`; no formatting or language-analysis logic lives in the extension's own source.
- The PhpStorm plugin registers `.nvs` as its own file type — opening a `.nvs` file does **not** invoke
  PhpStorm's bundled PHP support — and gets the same completion/hover/diagnostics/rename/formatting round
  trip through the identical `nvs lsp`/`nvs fmt` binaries the VS Code extension uses, evidenced by both
  editors agreeing on the same file's diagnostics and formatted output byte-for-byte.
- Neither extension ships a debugger UI; this is confirmed absent deliberately, not found missing by
  accident, and its own verification is deferred to the fast-follow named in *Revisiting*.
