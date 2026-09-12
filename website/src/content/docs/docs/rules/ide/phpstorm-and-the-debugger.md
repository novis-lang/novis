---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "PhpStorm and the debugger"
description: "The same server behind another editor, and a debug adapter that is complete before any debugger UI exists."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/ide/the-vs-code-extension/
  label: "Inside the VS Code extension"
next:
  link: /docs/rules/ide/security-in-the-editor/
  label: "Security in the editor"
---

<p class="nv-section-lead">The same server behind another editor, and a debug adapter that is complete before any debugger UI exists.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">3</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">0</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">3</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">3</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#phpstorm-bridges-to-the-same-server">PhpStorm drives the same <code>nvs lsp</code> and <code>nvs fmt</code> through JetBrains' LSP client, and builds no PSI tree, native refactoring or debugger UI until a later decision says so</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#the-debug-adapter-does-not-wait-for-an-editor"><code>nvs dap</code> is complete without any editor's debugger UI; VS Code's wiring is a descriptor factory and a <code>launch.json</code> schema, and PhpStorm's stays deferred</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#the-debugger-ui-is-as-deep-as-the-adapter">The debugger UI is only as deep as the capabilities <code>nvs dap</code> reports, so the adapter's capability list is the debugger's scope</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li></ol>

<div class="nv-rule" id="phpstorm-bridges-to-the-same-server">

## PhpStorm drives the same `nvs lsp` and `nvs fmt` through JetBrains' LSP client, and builds no PSI tree, native refactoring or debugger UI until a later decision says so

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#phpstorm-bridges-to-the-same-server"><code>ide/phpstorm-bridges-to-the-same-server</code></a>
</div>

The PhpStorm plugin drives the same `nvs lsp` binary the VS Code extension does, through JetBrains
Platform's LSP client support — falling back to the community LSP4IJ plugin if the bundled API lacks a
needed feature; which of the two is left to a concrete evaluation when the work starts. Completion,
hover, diagnostics, rename and go-to-definition route through that bridge, and so does formatting:
"Reformat Code" runs `nvs fmt`, and PhpStorm's native Formatter framework and Code Style settings page
do not apply to `.nvs` files. That is the price of not forking the formatter into a second
implementation ([`ide/one-server-two-thin-clients`](/docs/rules/ide/one-server-thin-clients/#one-server-two-thin-clients "Language smarts and formatting of Novis have one implementation each, nvs-lsp and nvs-fmt, and an editor client holds none of either")), named up front.

The plugin registers `.nvs` as its own file type ([`ide/nvs-is-its-own-file-type`](/docs/rules/ide/one-server-thin-clients/#nvs-is-its-own-file-type ".nvs is registered as its own language in every editor, activates nothing on .php, and is never handed to a PHP plugin")) and ships a
TextMate-or-equivalent baseline grammar for the same instant-colour reason VS Code does.

It explicitly does not build a native PSI tree, PhpStorm-grade refactoring beyond what the LSP `rename`
request gives, structural search and replace, intention actions backed by its own inspector, or
PhpStorm's native debugger UI ([`ide/the-debug-adapter-does-not-wait-for-an-editor`](/docs/rules/ide/phpstorm-and-the-debugger/#the-debug-adapter-does-not-wait-for-an-editor "nvs dap is complete without any editor's debugger UI; VS Code's wiring is a descriptor factory and a launch.json schema, and PhpStorm's stays deferred")). Those are the
real quality gap between an LSP bridge and PhpStorm's PHP support, and a full native plugin is a later,
explicit decision if usage justifies it — kept open, not silently skipped, and not scheduled.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Reformat Code runs <code>nvs fmt</code> with no PhpStorm Code Style page behind it, and PSI-level refactoring, structural search and the native debugger are absent for <code>.nvs</code></p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/ide/one-server-thin-clients/#one-server-two-thin-clients" title="Language smarts and formatting of Novis have one implementation each, nvs-lsp and nvs-fmt, and an editor client holds none of either"><code>ide/one-server-two-thin-clients</code></a> <a href="/docs/rules/ide/one-server-thin-clients/#nvs-is-its-own-file-type" title=".nvs is registered as its own language in every editor, activates nothing on .php, and is never handed to a PHP plugin"><code>ide/nvs-is-its-own-file-type</code></a> <a href="/docs/rules/ide/phpstorm-and-the-debugger/#the-debug-adapter-does-not-wait-for-an-editor" title="nvs dap is complete without any editor's debugger UI; VS Code's wiring is a descriptor factory and a launch.json schema, and PhpStorm's stays deferred"><code>ide/the-debug-adapter-does-not-wait-for-an-editor</code></a> <a href="/docs/rules/ide/one-server-thin-clients/#editor-clients-live-under-editors" title="An editor client lives under editors/&lt;editor&gt;, outside the Cargo workspace, and is created when its milestone starts rather than scaffolded ahead of it"><code>ide/editor-clients-live-under-editors</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0016.md">record 0016</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0040.md">record 0040</a></dd></div></dl>

</div>

<div class="nv-rule" id="the-debug-adapter-does-not-wait-for-an-editor">

## `nvs dap` is complete without any editor's debugger UI; VS Code's wiring is a descriptor factory and a `launch.json` schema, and PhpStorm's stays deferred

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#the-debug-adapter-does-not-wait-for-an-editor"><code>ide/the-debug-adapter-does-not-wait-for-an-editor</code></a>
</div>

`nvs dap` — the debug adapter, using safepoints for breakpoints ([`testing/debug-probes`](/docs/rules/testing/coverage-and-probes/#debug-probes "Coverage, tracing and profiling are one per-request flag word checked at fixed probe sites, never a second compiled tier")) — is a
complete, testable deliverable on its own. A working adapter and a working debugger UI in a given editor
are two different integrations, the same way a language server and a syntax-highlighting extension are,
so a stalled or under-scoped editor integration cannot block the adapter from shipping and being useful
to a CLI-driven client or a third editor.

For VS Code the editor-side wiring is small and lands with the adapter: a `DebugAdapterDescriptorFactory`
and a `launch.json` configuration schema targeting `nvs dap`. DAP is a wire protocol, and VS Code
already renders breakpoints, call stack, variables and watches for any adapter that speaks it, so the
extension authors no debugger UI — the acceptance is a breakpoint set in VS Code's UI hitting in
JIT-compiled code with correct variable values, through the descriptor factory and schema alone
([`ide/the-extension-builds-no-ui-the-editor-already-has`](/docs/rules/ide/the-vs-code-extension/#the-extension-builds-no-ui-the-editor-already-has "Coverage, server health, profiles and the debugger reach the editor through its own APIs and open formats — FileCoverage, LanguageStatusItem, DAP's UI, speedscope — and the extension builds none of them")). How deep that UI goes is the adapter's
capability set, not the extension's.

PhpStorm's `XDebugger` UI wired to a DAP backend stays deferred ([`ide/phpstorm-bridges-to-the-same-server`](/docs/rules/ide/phpstorm-and-the-debugger/#phpstorm-bridges-to-the-same-server "PhpStorm drives the same nvs lsp and nvs fmt through JetBrains' LSP client, and builds no PSI tree, native refactoring or debugger UI until a later decision says so")).
The two clients are deliberately asymmetric here; a future PhpStorm-depth pass has to address or
explicitly accept that.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no Xdebug-style listener to configure in the editor; the editor points its own debugger UI at <code>nvs dap</code> and renders what the adapter reports</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/ide/the-vs-code-extension/#the-extension-builds-no-ui-the-editor-already-has" title="Coverage, server health, profiles and the debugger reach the editor through its own APIs and open formats — FileCoverage, LanguageStatusItem, DAP's UI, speedscope — and the extension builds none of them"><code>ide/the-extension-builds-no-ui-the-editor-already-has</code></a> <a href="/docs/rules/ide/phpstorm-and-the-debugger/#phpstorm-bridges-to-the-same-server" title="PhpStorm drives the same nvs lsp and nvs fmt through JetBrains' LSP client, and builds no PSI tree, native refactoring or debugger UI until a later decision says so"><code>ide/phpstorm-bridges-to-the-same-server</code></a> <a href="/docs/rules/testing/coverage-and-probes/#debug-probes" title="Coverage, tracing and profiling are one per-request flag word checked at fixed probe sites, never a second compiled tier"><code>testing/debug-probes</code></a> <a href="/docs/rules/ide/phpstorm-and-the-debugger/#the-debugger-ui-is-as-deep-as-the-adapter" title="The debugger UI is only as deep as the capabilities nvs dap reports, so the adapter's capability list is the debugger's scope"><code>ide/the-debugger-ui-is-as-deep-as-the-adapter</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0016.md">record 0016</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0040.md">record 0040</a></dd></div></dl>

</div>

<div class="nv-rule" id="the-debugger-ui-is-as-deep-as-the-adapter">

## The debugger UI is only as deep as the capabilities `nvs dap` reports, so the adapter's capability list is the debugger's scope

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#the-debugger-ui-is-as-deep-as-the-adapter"><code>ide/the-debugger-ui-is-as-deep-as-the-adapter</code></a>
</div>

`nvs dap` is wired into VS Code's existing debugger UI, and every feature below renders in a UI that
already exists — it appears only if the adapter reports the corresponding capability at `initialize`. The
adapter's capability list is therefore the debugger's scope, and it is this:

- **Conditional breakpoints, hit counts and logpoints** — `supportsConditionalBreakpoints`,
  `supportsHitConditionalBreakpoints`, `supportsLogPoints`. A logpoint that does not stop the program is
  the debugging most users actually do.
- **Exception filters** — `exceptionBreakpointFilters`, so "break on uncaught" and "break on thrown" are
  separate switches. [`errors/escalation-ladder`](/docs/rules/errors/the-escalation-ladder/#escalation-ladder "A failure escalates through four tiers, and no tier is retried")'s single `Throwable` channel is what makes this two
  filters rather than PHP's five categories.
- **Stepping exclusions** — a `launch.json` glob list, so stepping does not descend into package code and
  a handled throw inside it does not stop the session.
- **Path mappings**, because the container case is the normal case: the file the adapter reports and the
  file in the editor differ whenever the program runs anywhere but the workspace root.
- **The value a function just returned**, in the variables pane after stepping out.
- **A `spawn`ed isolate is a DAP thread** — the standard presentation, needing no protocol extension. The
  *tree* of isolates would need one and is not built.

A fixture session exercises each of these, and `nvs dap` reports each capability at `initialize`.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Break-on-uncaught and break-on-thrown are two exception filters rather than PHP's five categories, and a spawned isolate appears as a thread rather than as a second session</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/errors/the-escalation-ladder/#escalation-ladder" title="A failure escalates through four tiers, and no tier is retried"><code>errors/escalation-ladder</code></a> <a href="/docs/rules/security/isolates/#isolate-shares-nothing" title="Running another script is an in-process isolate that shares nothing with its parent but compiled code"><code>security/isolate-shares-nothing</code></a> <a href="/docs/rules/ide/the-language-server/#every-feature-is-staged-behind-its-dependency" title="The VS Code client goes as deep as the editor allows, and each feature waits for the language or runtime piece it needs rather than shipping as a stub"><code>ide/every-feature-is-staged-behind-its-dependency</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0108.md">record 0108</a></dd></div></dl>

</div>
