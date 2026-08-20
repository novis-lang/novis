# ADR 0016 — IDE integration is a thin per-editor client over one language server; PhpStorm goes LSP-bridge before native

- **Status:** Accepted
- **Date:** 2026-08-20
- **Scope:** the editor-facing half of M10 — the VS Code extension, the PhpStorm plugin, how each drives
  `mwl-lsp`/`mwl-fmt`, what "tight IDE integration" includes and excludes for v1, and where the two client
  packages live in the repository layout
- **Amends:** [docs/implementation-plan.md](../implementation-plan.md) M10 — the verify line "VS Code and
  PhpStorm both drive the LSP" undersold what each editor actually needs to feel first-class; this ADR
  replaces it with the concrete per-editor client scope in *Decision*, and *Consequences* names what M10's
  estimate now carries that it did not before.
- **Relates to:** [0003](0003-extension-system.md) (the same phased shape: ship the portable, sandboxed
  answer first, and name the deeper native investment as a later, explicit decision rather than build it
  up front)

> **In short:** a language server is necessary but not sufficient in either editor. VS Code always needs a
> thin client extension regardless of the server behind it — something has to spawn `mwl lsp`, register the
> `.mwl` language and a TextMate grammar for instant syntax colour, and wire `mwl fmt` into format-on-save;
> LSP supplies the smarts (completion, hover, diagnostics, rename, go-to-definition, formatting), the
> extension supplies the editor-side plumbing. PhpStorm is the harder case: JetBrains IDEs are PSI-based, not
> LSP-native, so "just point PhpStorm at the LSP" is not a zero-cost default the way it nearly is in VS Code.
> MWL builds **exactly one** implementation of each smart feature — `mwl-lsp` and `mwl-fmt` — and ships two
> thin, per-editor clients around it: a VS Code extension as the reference client, and a PhpStorm plugin that
> bridges to the same server through JetBrains' LSP client support rather than reimplementing language
> smarts as PSI. A full native PhpStorm plugin (its own lexer/PSI/parser, PhpStorm-grade refactoring and
> debugger UI) is named as a later, explicit decision if usage justifies it — not built now, and not silently
> skipped either. Debug-adapter editor wiring (VS Code's `DebugAdapterDescriptorFactory` + `launch.json`
> schema, PhpStorm's native debugger UI) is likewise named and deferred: `mwl dap` itself still ships in M10,
> but hooking either editor's debugger UI to it is a tracked fast-follow, not a condition of "M10 done."

## Context

M10 already commits to `mwl fmt`, `mwl lsp` over `tower-lsp`, and `mwl dap`, and its verify line says "VS
Code and PhpStorm both drive the LSP" — phrased as if pointing an editor at a language server is the whole
job. It is not, in either editor, and the two editors fail in different ways if that is all that ships:

**VS Code.** The editor has no built-in concept of MWL at all until something registers one. Even with
`mwl-lsp` fully working, VS Code will not know `.mwl` is a language, will not know what binary to spawn or
how to restart it, and will render a freshly opened file with zero syntax colour until the server has
finished its first parse — which on a cold cache is exactly the moment a first impression is formed. None of
that is an LSP concern; all of it is client-extension plumbing that has no LSP-server substitute.

**PhpStorm.** IntelliJ Platform IDEs resolve, highlight, and refactor code through PSI (Program Structure
Interface) trees built by a language's own lexer/parser plugin — that is how PhpStorm's bundled PHP support
works, and it is not generic-LSP-shaped. JetBrains has since added LSP client support to the platform (and
the community ships LSP4IJ for IDEs without it built in), which lets a plugin drive an external language
server for the same core LSP feature set VS Code gets — completion, hover, diagnostics, rename, formatting.
What it does *not* give, compared to PhpStorm's native PHP experience, is PSI-level refactoring, structural
search/replace, and the deepest debugger UI integration. Building that instead means a genuine second
front end: a Kotlin plugin with its own lexer, parser, and PSI tree mirroring `mwl-syntax`'s grammar — a
standalone sub-project on the order of the PHP plugin itself, not an M10 line item.

**The decision this ADR resolves.** Given that gap, three questions had no answer on record: how deep the
PhpStorm integration goes (LSP-bridge vs. native PSI plugin), whether debugger UI wiring is part of "IDE
integration done" for v1, and where the two client packages live in the repository. All three are answered
below, deliberately in the same phased spirit as [ADR 0003](0003-extension-system.md)'s wasm-before-native
extension tiers: ship the portable, lower-cost answer, name the deeper one as a real option, and only build
it if the phased answer's limits actually bite.

## Decision

### 1. One server, two thin clients — the split of responsibility

`mwl-lsp` and `mwl-fmt` are the **only** place language smarts and formatting logic live. Neither editor
client re-implements completion, diagnostics, hover, rename, go-to-definition, or code formatting locally —
each is a thin adapter that starts the server/formatter process, translates its own editor's UI events into
LSP requests, and renders the results. This is the same "state a fact once" rule
[CLAUDE.md](../../CLAUDE.md) already applies to documentation, applied here to executable behaviour: two
independent reimplementations of, say, MWL's formatting rules would drift the moment one editor's plugin
fixes a bug the other's has not.

### 2. VS Code extension

A standard `vscode-languageclient` extension:

- Registers the `mwl` language ID, `.mwl` file association, and a `language-configuration.json` (bracket
  matching, comment toggles, auto-closing pairs, indentation rules) — largely PHP's, adjusted for
  `spawn script`, `type` aliases, and the type-annotation syntax [ADR 0007](0007-explicit-type-system.md)
  adds that PHP has no syntax for.
- Ships a **TextMate grammar** for the dual-mode `<?mwl ?>` / `<?php ?>` / `<?= ?>` + inline-HTML lexer mode
  M1 builds, giving instant, correct-enough syntax colour the moment a file opens — before the language
  server has warmed up or finished its first parse.
- Spawns `mwl lsp` (resolved via a configurable path setting, falling back to `PATH`) and layers **LSP
  semantic tokens** on top of the TextMate baseline once the server is live — the same two-layer pattern
  rust-analyzer and Deno's extension use, not a novel design.
- Wires `editor.formatOnSave` and the "Format Document"/"Format Selection" commands to
  `textDocument/formatting` / `textDocument/rangeFormatting` against `mwl-fmt` — no bundled formatter logic
  of its own.
- Surfaces `mwl run`/`mwl test` as VS Code Tasks and a "Run File" command. A full custom Test Explorer
  provider is not in this ADR's scope — see *Revisiting*.

### 3. PhpStorm plugin — LSP-bridge now, native PSI later

The PhpStorm plugin drives the **same** `mwl-lsp` binary the VS Code extension does, through JetBrains
Platform's LSP client support (falling back to the community LSP4IJ plugin if the bundled API proves
insufficient for a needed feature). Concretely, it must:

- Register `.mwl` as its **own** file type and language, distinct from PhpStorm's bundled PHP support — this
  is not optional polish. Without an explicit registration, PhpStorm's own PHP plugin may attempt to claim
  `.mwl` files (or a generic-text fallback will), and either failure mode looks like "the plugin doesn't
  work" to a user with no diagnostic pointing at the real cause.
- Provide a TextMate-or-equivalent baseline grammar for the same instant-colour reason as the VS Code
  extension — PhpStorm supports bundling a TextMate grammar for a custom file type without writing a full
  IDEA lexer.
- Route completion, hover, diagnostics, rename, and go-to-definition through the LSP bridge to `mwl-lsp`.
- Route formatting through the LSP bridge to `mwl-fmt`. This is a deliberate cost, named up front rather
  than discovered later: PhpStorm's native Formatter framework and Code Style settings page do not apply,
  so "Reformat Code" runs MWL's one canonical formatter with no PhpStorm-native style customization UI. That
  is the price of *not* forking formatting logic into a second implementation; see *Alternatives rejected*.
- **Explicitly not build**, in this phase: a native PSI tree, PhpStorm-grade refactoring (rename-across-files
  beyond what the LSP `rename` request already gives, structural search/replace, intention actions backed by
  its own inspector), or PhpStorm's native debugger UI. These are the real quality gap between the
  LSP-bridge plugin and PhpStorm's PHP support, and they are the reason *Revisiting* keeps a native plugin
  on the table rather than closing the question.

### 4. Debugger UI wiring is deferred, `mwl dap` itself is not

M10 still builds `mwl dap` on schedule — the debug adapter, using safepoints for breakpoints. What this ADR
defers is the **editor-side wiring**: VS Code's `DebugAdapterDescriptorFactory` registration and
`launch.json` configuration schema, and PhpStorm's native `XDebugger` UI wired to a DAP-speaking backend.
Both are real, separate engineering work — a working debug adapter and a working debugger *UI* in a given
editor are two different integrations, the same way a language server and a syntax-highlighting extension
are two different integrations in *Decision §§ 1–2*. Shipping `mwl dap` without either editor's UI wired to
it is a complete, testable milestone deliverable on its own; the editor wiring is named in *Revisiting* as
a tracked fast-follow, not folded silently into "M10 done" and not silently dropped either.

### 5. Repository layout

```
editors/
  vscode/      TextMate grammar, language-configuration.json, LSP client extension    M10
  phpstorm/    file-type registration, LSP-bridge plugin (Kotlin/Gradle)              M10
```

Created when M10 starts, the same rule [CLAUDE.md](../../CLAUDE.md) and the
[README](../../README.md#repository-layout) already state for every crate: nothing sits scaffolded and
empty ahead of its milestone. Neither directory is a Cargo crate — the VS Code extension is TypeScript/Node
tooling, the PhpStorm plugin is Kotlin/Gradle/IntelliJ Platform tooling — so both sit outside the Rust
workspace `Cargo.toml` governs, alongside (not inside) `crates/`.

## Consequences

**Positive**

- Formatting and language semantics have exactly one implementation each (`mwl-fmt`, `mwl-lsp`), reached
  identically by both editors — the two-formatter-drift failure mode named in *Decision § 1* cannot occur
  structurally, the same reasoning [ADR 0015](0015-no-name-aliasing.md) already applies to a *name* applied
  here to *behaviour*.
- VS Code gets a normal, well-understood extension shape with no open design questions.
- PhpStorm gets real IDE support — completion, hover, diagnostics, rename, formatting, instant syntax colour
  — without committing to a multi-month native-plugin investment before anyone has used the language in
  PhpStorm at all.
- The debugger split in *Decision § 4* means a stalled or under-scoped editor-debugger integration cannot
  block `mwl dap` itself from shipping and being independently useful (e.g., a minimal CLI-driven debug
  client, or a third editor's own DAP wiring) inside M10.

**Negative**

- **PhpStorm's ceiling is lower than its own PHP support for as long as the LSP-bridge stands.** No
  PSI-level refactoring or structural search, and PhpStorm's native Formatter/Code Style UI does not apply to
  `.mwl` files — a PhpStorm user comparing MWL support to PHP support directly will notice the gap. Accepted
  deliberately; see *Alternatives rejected* for why closing it now was rejected.
- **M10's estimate absorbs real, previously-implicit scope**: a TextMate grammar (or equivalent) for both
  editors, a `language-configuration.json`, a Kotlin/Gradle PhpStorm plugin project with its own build and
  packaging, and the file-type-collision handling in *Decision § 3* — none of which "VS Code and PhpStorm
  both drive the LSP" named as work. The milestone's time estimate should be revised upward to reflect it
  rather than discovered as slip once M10 starts.
- **No debugger UI in either editor at the end of M10** under this ADR, even though `mwl dap` exists —
  a user who expects "set a breakpoint, click run" the moment M10 ships will not get it. This is the explicit
  trade named in *Decision § 4*, not an oversight.
- Two build toolchains (`npm`/`vsce` for VS Code, Gradle/IntelliJ Platform Plugin SDK for PhpStorm) join the
  project's CI surface, neither of them Rust — a genuinely new kind of CI job, distinct from everything
  `cargo` already builds.

## Alternatives rejected

- **Full native PhpStorm plugin from the start** (own PSI/lexer/parser). Rejected for now: it is a
  multi-month, ongoing-maintenance sub-project on the scale of a second front end, undertaken before any
  PhpStorm user has touched the language — the same "don't build the expensive tier before the cheap one
  proves insufficient" reasoning [ADR 0003](0003-extension-system.md) already used for wasm-before-native
  extensions. Kept on the table, not closed — see *Revisiting*.
- **LSP-bridge only, permanently, with no native option ever revisited.** Rejected: it forecloses a
  legitimate future decision for no reason — if MWL gains real PhpStorm adoption, the refactoring/debugger
  gap in *Consequences, Negative* becomes a real cost worth paying down, and this ADR should not be the
  reason that conversation never happens.
- **A second, PhpStorm-native formatter implementation**, to get Code Style settings and format-on-paste
  parity with PhpStorm's PHP support. Rejected under *Decision § 1*: two formatting implementations will
  drift the first time either one's rules change without the other noticing, which is a worse outcome than
  a PhpStorm user occasionally reaching for "Reformat Code" and getting MWL's one canonical style with no
  local knobs.
- **Wire debug-adapter editor UI into the same M10 pass as the language client work.** Considered, and
  rejected for this ADR specifically because the user requirement this ADR answers scoped it out explicitly
  — treated as a tracked fast-follow instead of bundled in, per *Decision § 4*.
- **VS Code only, treat PhpStorm as out of scope.** Rejected outright — the requirement this ADR answers
  names PhpStorm explicitly, and a PHP-migration-target language skipping PHP developers' other major editor
  would undercut the adoption story [docs/implementation-plan.md](../implementation-plan.md) already argues
  for `mwl convert`.

## Revisiting

- **A full native PhpStorm PSI plugin**, if real usage shows the LSP-bridge ceiling (refactoring, structural
  search, debugger UI, native formatter integration) is actually costing adoption — see *Alternatives
  rejected*. Not scheduled to any milestone; this ADR only keeps the door open.
- **Debug-adapter editor wiring** — VS Code's `DebugAdapterDescriptorFactory` + `launch.json` schema,
  PhpStorm's `XDebugger` UI — as a fast-follow once `mwl dap` itself is verified, per *Decision § 4*.
- **Marketplace publishing and branding** (VS Code Marketplace publisher ID, JetBrains Marketplace listing,
  icon/branding assets, release cadence relative to the `mwl` binary's own versioning) is packaging detail
  this ADR does not resolve.
- **A VS Code Test Explorer provider** for `.mwlt`/`mwl test`, beyond the plain Tasks integration
  *Decision § 2* commits to, if the plain version proves too thin in practice.
- **Whether the PhpStorm LSP bridge should use JetBrains' bundled LSP client API or the community LSP4IJ
  plugin** is left open pending a concrete evaluation once M10's PhpStorm work starts — *Decision § 3* names
  both as viable, not one as chosen.

Verification, in the order it becomes possible (M10):

- The VS Code extension activates on `.mwl`, shows TextMate-grammar colour before the language server has
  responded once, then shows semantic-token colour once it has; completion, hover, diagnostics,
  go-to-definition and rename round-trip through `mwl lsp`; format-on-save and the format commands round-trip
  through `mwl fmt`; no formatting or language-analysis logic lives in the extension's own source.
- The PhpStorm plugin registers `.mwl` as its own file type — opening a `.mwl` file does **not** invoke
  PhpStorm's bundled PHP support — and gets the same completion/hover/diagnostics/rename/formatting round
  trip through the identical `mwl lsp`/`mwl fmt` binaries the VS Code extension uses, evidenced by both
  editors agreeing on the same file's diagnostics and formatted output byte-for-byte.
- Neither extension ships a debugger UI; this is confirmed absent deliberately, not found missing by
  accident, and its own verification is deferred to the fast-follow named in *Revisiting*.
