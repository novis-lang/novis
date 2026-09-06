---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Inside the VS Code extension"
description: "What the extension contributes, why it builds no UI the editor already has, and how its dependencies stay honest."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/ide/one-server-thin-clients/
  label: "One server, thin clients"
next:
  link: /docs/rules/ide/phpstorm-and-the-debugger/
  label: "PhpStorm and the debugger"
---

<p class="nv-section-lead">What the extension contributes, why it builds no UI the editor already has, and how its dependencies stay honest.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">8</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">0</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">8</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">1</span><span class="nv-count-label">differs from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#tasks-carry-a-problem-matcher"><code>nvs run</code> and <code>nvs test</code> are Tasks with a <code>problemMatcher</code> over the renderer's own format, so a diagnostic is a Problems-panel entry</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#the-extension-is-a-workspace-extension">The extension declares itself a workspace extension, because <code>nvs lsp</code> must be the binary next to the code</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#the-extension-runs-where-the-binary-is">The extension is <code>nvs-lang.nvs</code>, language <code>nvs</code>, <code>extensionKind: [&quot;workspace&quot;]</code>, built as a <code>.vsix</code> and published nowhere</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#the-extension-refuses-a-binary-it-does-not-understand">On a version mismatch at <code>initialize</code> the client says so in the status item and does not start</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#the-extension-builds-no-ui-the-editor-already-has">Coverage, server health, profiles and the debugger reach the editor through its own APIs and open formats — <code>FileCoverage</code>, <code>LanguageStatusItem</code>, DAP's UI, speedscope — and the extension builds none of them</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#the-ast-panel-shells-out-to-the-cli">The AST panel renders <code>nvs ast --json</code> for the active file, on the resilient tree by default, and never runs <code>Core\Ast</code></a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#dependencies-are-allowlisted">The extension holds no language logic, and its <code>package.json</code> dependencies are checked against an allowlist by its own tests</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#the-lockfile-is-committed-and-build-output-is-not"><code>package-lock.json</code> is committed; <code>node_modules/</code>, <code>out/</code>, <code>.vscode-test/</code> and <code>*.vsix</code> are ignored</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li></ol>

<div class="nv-rule" id="tasks-carry-a-problem-matcher">

## `nvs run` and `nvs test` are Tasks with a `problemMatcher` over the renderer's own format, so a diagnostic is a Problems-panel entry

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#tasks-carry-a-problem-matcher"><code>ide/tasks-carry-a-problem-matcher</code></a>
</div>

`nvs run` and `nvs test` are contributed as Tasks, and each carries a `problemMatcher`. Without one the
Tasks print text into a terminal; with one, every diagnostic is a clickable entry in the Problems panel.

It is a two-line regex over the renderer's existing format — `error[E0301]: message`, then
`  --> file:line:col` ([`errors/renderings`](/docs/rules/errors/diagnostics-and-logging/#renderings "The sink in force picks the rendering, and no call site may name one")) — and it is the difference between the Tasks being
useful and being decorative. A failing `nvs test` populating the Problems panel through the matcher is
part of the extension-host run.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/errors/diagnostics-and-logging/#renderings" title="The sink in force picks the rendering, and no call site may name one"><code>errors/renderings</code></a> <a href="/docs/rules/ide/one-server-thin-clients/#contributions-are-frozen-and-only-ever-added" title="A setting name and a command id are public API: the roster is frozen, and anything later is added, never renamed"><code>ide/contributions-are-frozen-and-only-ever-added</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0099.md">record 0099</a></dd></div></dl>

</div>

<div class="nv-rule" id="the-extension-is-a-workspace-extension">

## The extension declares itself a workspace extension, because `nvs lsp` must be the binary next to the code

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#the-extension-is-a-workspace-extension"><code>ide/the-extension-is-a-workspace-extension</code></a>
</div>

The extension's `package.json` declares `extensionKind: ["workspace"]`. It spawns `nvs lsp`, which must
be the binary next to the code — in a WSL distro, over SSH, or inside a devcontainer — and a workspace
extension runs where the code is rather than where the editor's window is.

That one line is the difference between working in every remote configuration and failing in all of
them with a message about `nvs` not being on `PATH`. The same contributions test that checks the frozen
identifiers asserts it.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/ide/the-language-server/#check-scope-defaults-to-open-documents" title="Diagnostics are published for open documents and their require graph by default, and a workspace pass is one setting or one command"><code>ide/check-scope-defaults-to-open-documents</code></a> <a href="/docs/rules/ide/one-server-thin-clients/#contributions-are-frozen-and-only-ever-added" title="A setting name and a command id are public API: the roster is frozen, and anything later is added, never renamed"><code>ide/contributions-are-frozen-and-only-ever-added</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0108.md">record 0108</a></dd></div></dl>

</div>

<div class="nv-rule" id="the-extension-runs-where-the-binary-is">

## The extension is `nvs-lang.nvs`, language `nvs`, `extensionKind: ["workspace"]`, built as a `.vsix` and published nowhere

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#the-extension-runs-where-the-binary-is"><code>ide/the-extension-runs-where-the-binary-is</code></a>
</div>

The extension id is `nvs-lang.nvs`, the language id is `nvs`, and `extensionKind` is `["workspace"]`. The
client spawns `nvs lsp`, which has to be the binary next to the code, so a WSL distro, an SSH host and a
devcontainer all get the remote's toolchain rather than a missing one.

CI produces an installable `.vsix` artifact. Nothing is published — no Marketplace publisher, no listing,
no branding; that decision is open and M4B does not close it. `editors/vscode` is a TypeScript package
outside the Cargo workspace.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/ide/one-server-thin-clients/#the-extension-claims-nvs-only" title="The extension activates on .nvs and never claims .php"><code>ide/the-extension-claims-nvs-only</code></a> <a href="/docs/rules/ide/the-vs-code-extension/#the-extension-refuses-a-binary-it-does-not-understand" title="On a version mismatch at initialize the client says so in the status item and does not start"><code>ide/the-extension-refuses-a-binary-it-does-not-understand</code></a> <a href="/docs/rules/ide/the-vs-code-extension/#the-lockfile-is-committed-and-build-output-is-not" title="package-lock.json is committed; node_modules/, out/, .vscode-test/ and .vsix are ignored"><code>ide/the-lockfile-is-committed-and-build-output-is-not</code></a> <a href="/docs/rules/ide/one-server-thin-clients/#editor-clients-live-under-editors" title="An editor client lives under editors/&lt;editor&gt;, outside the Cargo workspace, and is created when its milestone starts rather than scaffolded ahead of it"><code>ide/editor-clients-live-under-editors</code></a> <a href="/docs/rules/ide/one-server-thin-clients/#one-server-two-thin-clients" title="Language smarts and formatting have one implementation each, nvs-lsp and nvs-fmt, and an editor client holds none of either"><code>ide/one-server-two-thin-clients</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0099.md">record 0099</a></dd></div></dl>

</div>

<div class="nv-rule" id="the-extension-refuses-a-binary-it-does-not-understand">

## On a version mismatch at `initialize` the client says so in the status item and does not start

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#the-extension-refuses-a-binary-it-does-not-understand"><code>ide/the-extension-refuses-a-binary-it-does-not-understand</code></a>
</div>

`nvs lsp` reports its version at `initialize`. On a mismatch with the extension's own, the
`LanguageStatusItem` says so and the client does not start, rather than running and producing confusing
answers.

An old `nvs` earlier on `PATH` than the intended one is the single most likely support question this
extension will ever get, and it costs one comparison to answer it out loud. A client reporting a mismatched
version gets a refusal and a status item, not a session.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/ide/the-vs-code-extension/#the-extension-runs-where-the-binary-is" title="The extension is nvs-lang.nvs, language nvs, extensionKind: [workspace], built as a .vsix and published nowhere"><code>ide/the-extension-runs-where-the-binary-is</code></a> <a href="/docs/rules/ide/the-language-server/#the-server-is-synchronous" title="nvs-lsp is synchronous on lsp-server and lsp-types — a reader thread, a writer thread, one analysis thread, and no async runtime"><code>ide/the-server-is-synchronous</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0099.md">record 0099</a></dd></div></dl>

</div>

<div class="nv-rule" id="the-extension-builds-no-ui-the-editor-already-has">

## Coverage, server health, profiles and the debugger reach the editor through its own APIs and open formats — `FileCoverage`, `LanguageStatusItem`, DAP's UI, speedscope — and the extension builds none of them

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#the-extension-builds-no-ui-the-editor-already-has"><code>ide/the-extension-builds-no-ui-the-editor-already-has</code></a>
</div>

Where the editor or an open format already renders something, the extension feeds it and draws nothing
of its own. Four surfaces follow that rule. Server health and version are a `LanguageStatusItem`, the
API VS Code sanctions for it, not a hand-rolled status-bar item. Coverage flows through VS Code's
finalized Testing API and its `FileCoverage` model — per-file statement, branch and declaration counts
from the Clover/lcov exporters `nvs test` already produces ([`testing/debug-probes`](/docs/rules/testing/coverage-and-probes/#debug-probes "Coverage, tracing and profiling are one per-request flag word checked at fixed probe sites, never a second compiled tier"),
[`testing/report-formats`](/docs/rules/testing/doubles-and-the-runner/#report-formats "One verdict, three renderings, and a machine format owns stdout alone")) — and VS Code's own gutter and summary UI shows it; no gutter renderer
is written. A profile from `nvs run --profile` is emitted in the open speedscope JSON format, and a
"View Profile" command opens it in speedscope.app or an embedded webview that speaks the same format,
because VS Code's built-in flame chart is V8-specific and no bespoke flamegraph is built. The debugger
is DAP's existing UI over `nvs dap` ([`ide/the-debug-adapter-does-not-wait-for-an-editor`](/docs/rules/ide/phpstorm-and-the-debugger/#the-debug-adapter-does-not-wait-for-an-editor "nvs dap is complete without any editor's debugger UI; VS Code's wiring is a descriptor factory and a launch.json schema, and PhpStorm's stays deferred")).

That is four pieces of UI infrastructure Novis neither builds nor maintains, which is the simplicity
priority applied directly. The extension's own code is the descriptor factory, the schema contribution,
the test provider and the command that hands a file to a viewer.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Xdebug's profiler writes cachegrind for KCachegrind or PhpStorm's own viewer; a Novis profile is speedscope JSON and opens in speedscope.app or an embedded view of it</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/ide/phpstorm-and-the-debugger/#the-debug-adapter-does-not-wait-for-an-editor" title="nvs dap is complete without any editor's debugger UI; VS Code's wiring is a descriptor factory and a launch.json schema, and PhpStorm's stays deferred"><code>ide/the-debug-adapter-does-not-wait-for-an-editor</code></a> <a href="/docs/rules/testing/coverage-and-probes/#debug-probes" title="Coverage, tracing and profiling are one per-request flag word checked at fixed probe sites, never a second compiled tier"><code>testing/debug-probes</code></a> <a href="/docs/rules/testing/doubles-and-the-runner/#report-formats" title="One verdict, three renderings, and a machine format owns stdout alone"><code>testing/report-formats</code></a> <a href="/docs/rules/ide/the-language-server/#every-feature-is-staged-behind-its-dependency" title="The VS Code client goes as deep as the editor allows, and each feature waits for the language or runtime piece it needs rather than shipping as a stub"><code>ide/every-feature-is-staged-behind-its-dependency</code></a> <a href="/docs/rules/observability/traces/#trace-events-carry-a-kind" title="A trace event carries one of four kinds — call, gc, spawn, query — and a call keeps its probe shape"><code>observability/trace-events-carry-a-kind</code></a> <a href="/docs/rules/ide/phpstorm-and-the-debugger/#the-debugger-ui-is-as-deep-as-the-adapter" title="The debugger UI is only as deep as the capabilities nvs dap reports, so the adapter's capability list is the debugger's scope"><code>ide/the-debugger-ui-is-as-deep-as-the-adapter</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0040.md">record 0040</a></dd></div></dl>

</div>

<div class="nv-rule" id="the-ast-panel-shells-out-to-the-cli">

## The AST panel renders `nvs ast --json` for the active file, on the resilient tree by default, and never runs `Core\Ast`

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#the-ast-panel-shells-out-to-the-cli"><code>ide/the-ast-panel-shells-out-to-the-cli</code></a>
</div>

The AST explorer panel renders `nvs ast --json` for the active file as a tree view — the resilient tree
by default, so the panel works on a file that does not compile. `nvs ast` ships the `--json` flag with a
frozen schema for that purpose; the `{stmts:#?}` debug print has no stability contract and is not what
the panel reads.

It does not use `Core\Ast` ([`core-classes/ast-is-inert`](/docs/rules/core-classes/regex-html-and-introspection/#ast-is-inert "Core\Ast runs the compiler's own parser and hands back typed, inert nodes")). That is the language-level reflective
parse a running Novis program calls; the editor panel is simpler and shells out to the CLI, the same way
`nvs check` backs diagnostics. Both read the one tree there is ([`ide/one-grammar-one-tree`](/docs/rules/ide/the-resilient-parse/#one-grammar-one-tree "The resilient parse is the one grammar's AST plus a trivia layer and an offset index, never a second tree")), so the
panel and the compiler cannot disagree about a file's shape.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/regex-html-and-introspection/#ast-is-inert" title="Core\Ast runs the compiler's own parser and hands back typed, inert nodes"><code>core-classes/ast-is-inert</code></a> <a href="/docs/rules/ide/the-resilient-parse/#one-grammar-one-tree" title="The resilient parse is the one grammar's AST plus a trivia layer and an offset index, never a second tree"><code>ide/one-grammar-one-tree</code></a> <a href="/docs/rules/ide/the-resilient-parse/#the-tree-survives-a-syntax-error" title="The parser always returns a tree, a node it invented says so, and an offset maps to the innermost node even inside a malformed region"><code>ide/the-tree-survives-a-syntax-error</code></a> <a href="/docs/rules/ide/the-resilient-parse/#ast-json-schema-is-frozen" title="nvs ast --json has a frozen node schema, is resilient by default, and its only type-dependent field is a secret literal's placeholder"><code>ide/ast-json-schema-is-frozen</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0040.md">record 0040</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0099.md">record 0099</a></dd></div></dl>

</div>

<div class="nv-rule" id="dependencies-are-allowlisted">

## The extension holds no language logic, and its `package.json` dependencies are checked against an allowlist by its own tests

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#dependencies-are-allowlisted"><code>ide/dependencies-are-allowlisted</code></a>
</div>

The extension may hold no language logic — no parser, no formatter, no type table — and this is enforced
rather than intended: its `package.json` `dependencies` are checked against an allowlist by its own test
suite, so a second implementation cannot arrive as a dependency, and the reviewer is not the only thing
standing between the repository and one.

The same allowlist is what keeps the client free of language logic when a feature is added. The redaction
of [`security/redaction-ranges-come-from-the-server`](/docs/rules/security/redaction/#redaction-ranges-come-from-the-server "The editor conceals a secret value by default, and the ranges come from the language server rather than a client guess") is a range list from the server and a decoration;
there is nothing in it a parser would help with, and the test is unchanged by it.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/ide/one-server-thin-clients/#contributions-are-frozen-and-only-ever-added" title="A setting name and a command id are public API: the roster is frozen, and anything later is added, never renamed"><code>ide/contributions-are-frozen-and-only-ever-added</code></a> <a href="/docs/rules/ide/testing-the-editor/#headless-gates-the-loop-the-host-run-gates-the-milestone" title="The headless suites run every iteration with no editor; the extension-host run is CI-only, under xvfb-run, with an isolated profile"><code>ide/headless-gates-the-loop-the-host-run-gates-the-milestone</code></a> <a href="/docs/rules/security/redaction/#redaction-ranges-come-from-the-server" title="The editor conceals a secret value by default, and the ranges come from the language server rather than a client guess"><code>security/redaction-ranges-come-from-the-server</code></a> <a href="/docs/rules/ide/one-server-thin-clients/#one-server-two-thin-clients" title="Language smarts and formatting have one implementation each, nvs-lsp and nvs-fmt, and an editor client holds none of either"><code>ide/one-server-two-thin-clients</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0099.md">record 0099</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0101.md">record 0101</a></dd></div></dl>

</div>

<div class="nv-rule" id="the-lockfile-is-committed-and-build-output-is-not">

## `package-lock.json` is committed; `node_modules/`, `out/`, `.vscode-test/` and `*.vsix` are ignored

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#the-lockfile-is-committed-and-build-output-is-not"><code>ide/the-lockfile-is-committed-and-build-output-is-not</code></a>
</div>

`.gitignore` carries `node_modules/`, `out/`, `.vscode-test/` and `*.vsix`: a session that commits
`node_modules` is a session whose commit nobody can review.

`package-lock.json` **is** committed, because `npm ci` is what the acceptance run uses and it requires one,
and because an unpinned dependency tree makes the grammar snapshots reproducible only by luck. CI grows two
jobs beside the ones already there — the headless suites on all three platforms and the extension-host run
on Linux — and `ci.yml` is the count of those.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/ide/testing-the-editor/#headless-gates-the-loop-the-host-run-gates-the-milestone" title="The headless suites run every iteration with no editor; the extension-host run is CI-only, under xvfb-run, with an isolated profile"><code>ide/headless-gates-the-loop-the-host-run-gates-the-milestone</code></a> <a href="/docs/rules/ide/the-vs-code-extension/#the-extension-runs-where-the-binary-is" title="The extension is nvs-lang.nvs, language nvs, extensionKind: [workspace], built as a .vsix and published nowhere"><code>ide/the-extension-runs-where-the-binary-is</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0099.md">record 0099</a></dd></div></dl>

</div>
