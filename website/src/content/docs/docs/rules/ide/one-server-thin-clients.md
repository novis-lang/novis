---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "One server, thin clients"
description: "Language smarts and formatting have one implementation each. An editor client holds none of either."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/ide/testing-the-editor/
  label: "Testing the editor"
next:
  link: /docs/rules/ide/the-vs-code-extension/
  label: "Inside the VS Code extension"
---

<p class="nv-section-lead">Language smarts and formatting have one implementation each. An editor client holds none of either.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">8</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">0</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">8</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">5</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#one-server-two-thin-clients">Language smarts and formatting of Novis have one implementation each, <code>nvs-lsp</code> and <code>nvs-fmt</code>, and an editor client holds none of either</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#editor-clients-live-under-editors">An editor client lives under <code>editors/&lt;editor&gt;</code>, outside the Cargo workspace, and is created when its milestone starts rather than scaffolded ahead of it</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#vscode-is-the-reference-client">The VS Code extension is a <code>vscode-languageclient</code> shell: it registers <code>.nvs</code>, colours from a TextMate grammar until the server answers, and routes everything else to <code>nvs lsp</code> and <code>nvs fmt</code></a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#nvs-is-its-own-file-type"><code>.nvs</code> is registered as its own language in every editor, activates nothing on <code>.php</code>, and is never handed to a PHP plugin</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#the-extension-claims-nvs-only">The extension activates on <code>.nvs</code> and never claims <code>.php</code></a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#language-configuration-is-content"><code>language-configuration.json</code> carries comments, pairs, indentation, folding and a <code>wordPattern</code> that includes <code>$</code></a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#doc-comment-authoring-is-the-editors-own">Writing a <code>///</code> run is the editor's job: <code>onEnterRules</code> continues one, a paste provider prefixes one, and neither contributes a name</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#contributions-are-frozen-and-only-ever-added">A setting name and a command id are public API: the roster is frozen, and anything later is added, never renamed</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li></ol>

<div class="nv-rule" id="one-server-two-thin-clients">

## Language smarts and formatting of Novis have one implementation each, `nvs-lsp` and `nvs-fmt`, and an editor client holds none of either

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#one-server-two-thin-clients"><code>ide/one-server-two-thin-clients</code></a>
</div>

`nvs-lsp` and `nvs-fmt` are the only place completion, hover, diagnostics, rename, go-to-definition
and formatting of Novis are implemented. An editor client is a thin adapter: it starts the server or the
formatter, translates its own editor's events into LSP requests, and renders what comes back. It
decides nothing about the language — not what a name resolves to, not where a line breaks, not even
which range is a `secret` ([`ide/redaction-ranges-come-from-the-server`](/docs/rules/ide/security-in-the-editor/#redaction-ranges-come-from-the-server "The editor conceals a secret value by default, and the ranges come from the language server rather than a client guess")).

The reason is the same one that gives every fact one home in the documentation, applied to executable
behaviour: two implementations of the formatting rules drift the first time one editor's plugin fixes a
bug the other's has not, and the verification that both editors produce byte-identical diagnostics and
formatted output for one file only holds while there is one implementation to agree with.

**Markup is the one exception, and it belongs to the editor rather than to the client.** Inside an
inline-HTML region the services and the formatter are the editor's own HTML ones
([`ide/a-template-region-gets-the-editors-services-and-formatter`](/docs/rules/ide/highlighting-and-completion/#a-template-region-gets-the-editors-services-and-formatter "An inline-HTML region gets the editor's own HTML, CSS and JavaScript services on boundaries the server reports, and the editor's HTML formatter after nvs fmt, indented from the Novis code around it")): the client forwards to them on
boundaries the server reports and implements neither. So byte-identical formatted output across the two
editors holds for the Novis bytes of a file; its markup bytes are each editor's HTML formatter's.

The VS Code extension ([`ide/vscode-is-the-reference-client`](/docs/rules/ide/one-server-thin-clients/#vscode-is-the-reference-client "The VS Code extension is a vscode-languageclient shell: it registers .nvs, colours from a TextMate grammar until the server answers, and routes everything else to nvs lsp and nvs fmt")) and the PhpStorm plugin
([`ide/phpstorm-bridges-to-the-same-server`](/docs/rules/ide/phpstorm-and-the-debugger/#phpstorm-bridges-to-the-same-server "PhpStorm drives the same nvs lsp and nvs fmt through JetBrains' LSP client, and builds no PSI tree, native refactoring or debugger UI until a later decision says so")) are the two clients, and a dependency-allowlist test on
the extension is what enforces "holds no language logic" rather than review.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>PhpStorm's PHP support is a native PSI plugin with its own formatter and refactorings, whereas its Novis plugin is a bridge to the same server VS Code drives, so nothing about <code>.nvs</code> is decided in the editor</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/ide/one-server-thin-clients/#vscode-is-the-reference-client" title="The VS Code extension is a vscode-languageclient shell: it registers .nvs, colours from a TextMate grammar until the server answers, and routes everything else to nvs lsp and nvs fmt"><code>ide/vscode-is-the-reference-client</code></a> <a href="/docs/rules/ide/phpstorm-and-the-debugger/#phpstorm-bridges-to-the-same-server" title="PhpStorm drives the same nvs lsp and nvs fmt through JetBrains' LSP client, and builds no PSI tree, native refactoring or debugger UI until a later decision says so"><code>ide/phpstorm-bridges-to-the-same-server</code></a> <a href="/docs/rules/ide/the-language-server/#one-crate-and-one-extension-grow-in-place" title="nvs-lsp and editors/vscode are one crate and one package that grow in place; no prototype is built to be discarded"><code>ide/one-crate-and-one-extension-grow-in-place</code></a> <a href="/docs/rules/ide/security-in-the-editor/#redaction-ranges-come-from-the-server" title="The editor conceals a secret value by default, and the ranges come from the language server rather than a client guess"><code>ide/redaction-ranges-come-from-the-server</code></a> <a href="/docs/rules/tooling/the-formatter/#fmt-is-one-canonical-style" title="nvs fmt has one style, takes no configuration, and its output is a pure function of the file it is given"><code>tooling/fmt-is-one-canonical-style</code></a> <a href="/docs/rules/ide/one-server-thin-clients/#contributions-are-frozen-and-only-ever-added" title="A setting name and a command id are public API: the roster is frozen, and anything later is added, never renamed"><code>ide/contributions-are-frozen-and-only-ever-added</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0016.md">record 0016</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0040.md">record 0040</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0173.md">record 0173</a></dd></div></dl>

</div>

<div class="nv-rule" id="editor-clients-live-under-editors">

## An editor client lives under `editors/<editor>`, outside the Cargo workspace, and is created when its milestone starts rather than scaffolded ahead of it

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#editor-clients-live-under-editors"><code>ide/editor-clients-live-under-editors</code></a>
</div>

The editor clients sit at the repository root, beside `crates/` and outside the Rust workspace:

```
editors/
  vscode/      TextMate grammar, language-configuration.json, LSP client extension
  phpstorm/    file-type registration, LSP-bridge plugin (Kotlin/Gradle)
```

Neither is a Cargo crate — the VS Code extension is TypeScript and Node tooling, the PhpStorm plugin is
Kotlin, Gradle and the IntelliJ Platform SDK — so neither is governed by the workspace `Cargo.toml`, and
each brings a build toolchain (`npm`/`vsce`, Gradle) that is a genuinely new kind of CI job next to
everything `cargo` builds. `nv verify` runs the extension's headless suites last, for that reason,
and treats the directory being absent as a real state rather than an error.

A directory is created when its milestone starts, never scaffolded empty ahead of it — the rule every
crate already follows. The VS Code package and the server crate that back it are then the only ones
there will be ([`ide/one-crate-and-one-extension-grow-in-place`](/docs/rules/ide/the-language-server/#one-crate-and-one-extension-grow-in-place "nvs-lsp and editors/vscode are one crate and one package that grow in place; no prototype is built to be discarded")).

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/ide/the-language-server/#one-crate-and-one-extension-grow-in-place" title="nvs-lsp and editors/vscode are one crate and one package that grow in place; no prototype is built to be discarded"><code>ide/one-crate-and-one-extension-grow-in-place</code></a> <a href="/docs/rules/ide/one-server-thin-clients/#vscode-is-the-reference-client" title="The VS Code extension is a vscode-languageclient shell: it registers .nvs, colours from a TextMate grammar until the server answers, and routes everything else to nvs lsp and nvs fmt"><code>ide/vscode-is-the-reference-client</code></a> <a href="/docs/rules/ide/phpstorm-and-the-debugger/#phpstorm-bridges-to-the-same-server" title="PhpStorm drives the same nvs lsp and nvs fmt through JetBrains' LSP client, and builds no PSI tree, native refactoring or debugger UI until a later decision says so"><code>ide/phpstorm-bridges-to-the-same-server</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0016.md">record 0016</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0040.md">record 0040</a></dd></div></dl>

</div>

<div class="nv-rule" id="vscode-is-the-reference-client">

## The VS Code extension is a `vscode-languageclient` shell: it registers `.nvs`, colours from a TextMate grammar until the server answers, and routes everything else to `nvs lsp` and `nvs fmt`

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#vscode-is-the-reference-client"><code>ide/vscode-is-the-reference-client</code></a>
</div>

The VS Code extension is a standard `vscode-languageclient` package and the reference client. It
registers the `nvs` language ID and the `.nvs` association ([`ide/nvs-is-its-own-file-type`](/docs/rules/ide/one-server-thin-clients/#nvs-is-its-own-file-type ".nvs is registered as its own language in every editor, activates nothing on .php, and is never handed to a PHP plugin")), a
`language-configuration.json` for bracket matching, comment toggles, auto-closing pairs and indentation
— PHP's, adjusted for `spawn script`, `type` aliases and the type-annotation syntax PHP lacks — and a
TextMate grammar for the `<?nvs ?>` / `<?= ?>` plus inline-HTML lexer mode, so a file has correct-enough
colour the moment it opens and before the server has parsed anything.

It spawns `nvs lsp` from a configurable path setting, falling back to `PATH`, and layers LSP semantic
tokens over the TextMate baseline once the server is live — the two-layer pattern rust-analyzer and
Deno use. `editor.formatOnSave` and the format commands go to `textDocument/formatting` and
`rangeFormatting` against `nvs-fmt`; `nvs run` and `nvs test` are VS Code Tasks and a "Run File"
command. A `secret` value's bytes are concealed by default on ranges the server hands over, and
`tainted` gets no default decoration ([`ide/tainted-has-no-default-decoration`](/docs/rules/ide/security-in-the-editor/#tainted-has-no-default-decoration "tainted ships no default editor decoration, and the marker is opt-in")).

Nothing in that list is language logic ([`ide/one-server-two-thin-clients`](/docs/rules/ide/one-server-thin-clients/#one-server-two-thin-clients "Language smarts and formatting of Novis have one implementation each, nvs-lsp and nvs-fmt, and an editor client holds none of either")). The concrete
contribution roster — setting and command identifiers — is frozen elsewhere; this is the shape.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/ide/one-server-thin-clients/#one-server-two-thin-clients" title="Language smarts and formatting of Novis have one implementation each, nvs-lsp and nvs-fmt, and an editor client holds none of either"><code>ide/one-server-two-thin-clients</code></a> <a href="/docs/rules/ide/one-server-thin-clients/#nvs-is-its-own-file-type" title=".nvs is registered as its own language in every editor, activates nothing on .php, and is never handed to a PHP plugin"><code>ide/nvs-is-its-own-file-type</code></a> <a href="/docs/rules/ide/the-language-server/#every-feature-is-staged-behind-its-dependency" title="The VS Code client goes as deep as the editor allows, and each feature waits for the language or runtime piece it needs rather than shipping as a stub"><code>ide/every-feature-is-staged-behind-its-dependency</code></a> <a href="/docs/rules/ide/security-in-the-editor/#redaction-ranges-come-from-the-server" title="The editor conceals a secret value by default, and the ranges come from the language server rather than a client guess"><code>ide/redaction-ranges-come-from-the-server</code></a> <a href="/docs/rules/ide/security-in-the-editor/#tainted-has-no-default-decoration" title="tainted ships no default editor decoration, and the marker is opt-in"><code>ide/tainted-has-no-default-decoration</code></a> <a href="/docs/rules/ide/highlighting-and-completion/#highlighting-is-two-layers" title="Syntax highlighting is a TextMate grammar and a semantic-token provider, each with its own test, and each must cover what makes Novis not PHP"><code>ide/highlighting-is-two-layers</code></a> <a href="/docs/rules/ide/one-server-thin-clients/#contributions-are-frozen-and-only-ever-added" title="A setting name and a command id are public API: the roster is frozen, and anything later is added, never renamed"><code>ide/contributions-are-frozen-and-only-ever-added</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0016.md">record 0016</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0040.md">record 0040</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0099.md">record 0099</a></dd></div></dl>

</div>

<div class="nv-rule" id="nvs-is-its-own-file-type">

## `.nvs` is registered as its own language in every editor, activates nothing on `.php`, and is never handed to a PHP plugin

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#nvs-is-its-own-file-type"><code>ide/nvs-is-its-own-file-type</code></a>
</div>

Every editor client registers `.nvs` as its own file type and language, distinct from whatever PHP
support the editor bundles, and activates on nothing else — the VS Code extension does not activate on
`.php`, and the PhpStorm plugin does not let PhpStorm's PHP plugin or a generic-text fallback claim a
`.nvs` file.

The registration is not polish. Without it PhpStorm's own PHP plugin may take the file, or a plain-text
fallback will, and either failure looks to the user like "the plugin does not work" with no diagnostic
pointing at the real cause. The verification for both clients is the same: opening a `.nvs` file
invokes Novis's client and never the editor's PHP support.

A Novis file is not a PHP file to the editor for the same reason it is not one to the compiler
([`statements/nvs-is-the-only-open-tag`](/docs/rules/statements/names-and-require/#nvs-is-the-only-open-tag "<?nvs is the only code-mode open tag; <?php is refused")): the grammar, the type syntax and the diagnostics are
different enough that a PHP tool over the file would colour valid Novis as an error and valid PHP that
Novis rejects as fine.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A <code>.nvs</code> file is not a PHP file to the editor, so Intelephense and PhpStorm's bundled PHP support never open it and none of their settings reach it</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/ide/one-server-thin-clients/#vscode-is-the-reference-client" title="The VS Code extension is a vscode-languageclient shell: it registers .nvs, colours from a TextMate grammar until the server answers, and routes everything else to nvs lsp and nvs fmt"><code>ide/vscode-is-the-reference-client</code></a> <a href="/docs/rules/ide/phpstorm-and-the-debugger/#phpstorm-bridges-to-the-same-server" title="PhpStorm drives the same nvs lsp and nvs fmt through JetBrains' LSP client, and builds no PSI tree, native refactoring or debugger UI until a later decision says so"><code>ide/phpstorm-bridges-to-the-same-server</code></a> <a href="/docs/rules/statements/names-and-require/#nvs-is-the-only-open-tag" title="&lt;?nvs is the only code-mode open tag; &lt;?php is refused"><code>statements/nvs-is-the-only-open-tag</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0016.md">record 0016</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0040.md">record 0040</a></dd></div></dl>

</div>

<div class="nv-rule" id="the-extension-claims-nvs-only">

## The extension activates on `.nvs` and never claims `.php`

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#the-extension-claims-nvs-only"><code>ide/the-extension-claims-nvs-only</code></a>
</div>

The extension registers `.nvs` and does not claim `.php`, even though `nvs-syntax` parses it. Claiming it
would fight every PHP extension a user already has, and losing that fight silently looks like Novis being
broken. An opt-in setting is M10's if anyone converting a codebase asks for it.

The extension-host run proves activation on `.nvs` and its absence on `.php`.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Opening a <code>.php</code> file gets the PHP extension already installed, not Novis, even though <code>nvs-syntax</code> could parse it</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/ide/one-server-thin-clients/#contributions-are-frozen-and-only-ever-added" title="A setting name and a command id are public API: the roster is frozen, and anything later is added, never renamed"><code>ide/contributions-are-frozen-and-only-ever-added</code></a> <a href="/docs/rules/ide/the-vs-code-extension/#the-extension-runs-where-the-binary-is" title="The extension is novis-lang.nvs, language nvs, extensionKind: [workspace], built as a .vsix and published nowhere"><code>ide/the-extension-runs-where-the-binary-is</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0099.md">record 0099</a></dd></div></dl>

</div>

<div class="nv-rule" id="language-configuration-is-content">

## `language-configuration.json` carries comments, pairs, indentation, folding and a `wordPattern` that includes `$`

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#language-configuration-is-content"><code>ide/language-configuration-is-content</code></a>
</div>

`language-configuration.json` is content, not a checkbox: comments (`//`, `#`, `/* */`), brackets,
auto-closing and surrounding pairs, `indentationRules`, `onEnterRules` continuing a `///` run
([`ide/doc-comment-authoring-is-the-editors-own`](/docs/rules/ide/one-server-thin-clients/#doc-comment-authoring-is-the-editors-own "Writing a /// run is the editor's job: onEnterRules continues one, a paste provider prefixes one, and neither contributes a name")), and folding markers.

Two entries are where a file borrowed from a PHP extension goes wrong. The first is `onEnterRules`,
which there continues a `/** */` block — in Novis an ordinary comment nothing reads
([`tooling/doc-comment-is-three-slashes`](/docs/rules/tooling/doc-comments-and-metadata/#doc-comment-is-three-slashes "A doc comment is exactly ///; //// and longer runs are ordinary comments, and # never is one")) — so it carries a shape the language does not document
with and leaves the shape it does uncontinued.

The second is that **`wordPattern` must include `$`**. Without it, double-clicking `$total` selects
`total`, every rename-adjacent interaction is off by one character, and word-based completion suggests
the wrong token.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Double-clicking <code>$total</code> selects <code>$total</code>; a configuration borrowed from a PHP extension selects <code>total</code> and puts every rename-adjacent interaction off by one character</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/ide/one-server-thin-clients/#doc-comment-authoring-is-the-editors-own" title="Writing a /// run is the editor's job: onEnterRules continues one, a paste provider prefixes one, and neither contributes a name"><code>ide/doc-comment-authoring-is-the-editors-own</code></a> <a href="/docs/rules/ide/one-server-thin-clients/#contributions-are-frozen-and-only-ever-added" title="A setting name and a command id are public API: the roster is frozen, and anything later is added, never renamed"><code>ide/contributions-are-frozen-and-only-ever-added</code></a> <a href="/docs/rules/ide/highlighting-and-completion/#highlighting-is-two-layers" title="Syntax highlighting is a TextMate grammar and a semantic-token provider, each with its own test, and each must cover what makes Novis not PHP"><code>ide/highlighting-is-two-layers</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0099.md">record 0099</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0149.md">record 0149</a></dd></div></dl>

</div>

<div class="nv-rule" id="doc-comment-authoring-is-the-editors-own">

## Writing a `///` run is the editor's job: `onEnterRules` continues one, a paste provider prefixes one, and neither contributes a name

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#doc-comment-authoring-is-the-editors-own"><code>ide/doc-comment-authoring-is-the-editors-own</code></a>
</div>

Writing a `///` run is the editor's job, and it is two affordances that contribute no name.
`onEnterRules` continues the run — Enter on a line opening with `///` starts the next one at the same
indentation, and it keeps arriving until the author deletes it, as in Rust and C#. A
`DocumentPasteEditProvider` offers *Paste as doc comment* when multi-line text is pasted with the cursor
in a run: each line takes the run's marker and indentation, and an empty line becomes a bare `///`.

Both exist because [`tooling/doc-comment-is-three-slashes`](/docs/rules/tooling/doc-comments-and-metadata/#doc-comment-is-three-slashes "A doc comment is exactly ///; //// and longer runs are ordinary comments, and # never is one") is line-oriented on purpose — the per-line
marker is what stops anything in the body from ending the comment — and the cost of that is authoring,
not reading. VS Code's toggle-line-comment inserts `//`, so it answers neither half.

Neither is a command. The editor already surfaces paste alternatives in its own widget
([`ide/the-extension-builds-no-ui-the-editor-already-has`](/docs/rules/ide/the-vs-code-extension/#the-extension-builds-no-ui-the-editor-already-has "Coverage, server health, profiles and the debugger reach the editor through its own APIs and open formats — FileCoverage, LanguageStatusItem, DAP's UI, speedscope — and the extension builds none of them")), so nothing is added to a menu or a
keymap, and [`ide/contributions-are-frozen-and-only-ever-added`](/docs/rules/ide/one-server-thin-clients/#contributions-are-frozen-and-only-ever-added "A setting name and a command id are public API: the roster is frozen, and anything later is added, never renamed")'s roster does not move — except that
the paste edit's **kind id** reaches a user's `editor.pasteAs.preferences` and is frozen on that rule's
own terms. Prefixing lines with a marker is not language logic, so
[`ide/dependencies-are-allowlisted`](/docs/rules/ide/security-in-the-editor/#dependencies-are-allowlisted "The extension holds no language logic, and its package.json dependencies are checked against an allowlist by its own tests")'s allowlist is unchanged; an aid that had to understand the prose
inside a doc comment is the first thing here that would have to answer it.

Each client owns its own copy ([`ide/one-server-two-thin-clients`](/docs/rules/ide/one-server-thin-clients/#one-server-two-thin-clients "Language smarts and formatting of Novis have one implementation each, nvs-lsp and nvs-fmt, and an editor client holds none of either")): these are editing affordances,
not server answers, so the PhpStorm plugin writes both again or goes without.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A PHP editor continues a <code>/** */</code> docblock and autocompletes tags into it; here the continuation is a <code>///</code> run and there is no tag to complete</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/ide/one-server-thin-clients/#language-configuration-is-content" title="language-configuration.json carries comments, pairs, indentation, folding and a wordPattern that includes $"><code>ide/language-configuration-is-content</code></a> <a href="/docs/rules/tooling/doc-comments-and-metadata/#doc-comment-is-three-slashes" title="A doc comment is exactly ///; //// and longer runs are ordinary comments, and # never is one"><code>tooling/doc-comment-is-three-slashes</code></a> <a href="/docs/rules/ide/the-vs-code-extension/#the-extension-builds-no-ui-the-editor-already-has" title="Coverage, server health, profiles and the debugger reach the editor through its own APIs and open formats — FileCoverage, LanguageStatusItem, DAP's UI, speedscope — and the extension builds none of them"><code>ide/the-extension-builds-no-ui-the-editor-already-has</code></a> <a href="/docs/rules/ide/one-server-thin-clients/#contributions-are-frozen-and-only-ever-added" title="A setting name and a command id are public API: the roster is frozen, and anything later is added, never renamed"><code>ide/contributions-are-frozen-and-only-ever-added</code></a> <a href="/docs/rules/ide/security-in-the-editor/#dependencies-are-allowlisted" title="The extension holds no language logic, and its package.json dependencies are checked against an allowlist by its own tests"><code>ide/dependencies-are-allowlisted</code></a> <a href="/docs/rules/ide/one-server-thin-clients/#one-server-two-thin-clients" title="Language smarts and formatting of Novis have one implementation each, nvs-lsp and nvs-fmt, and an editor client holds none of either"><code>ide/one-server-two-thin-clients</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0149.md">record 0149</a></dd></div></dl>

</div>

<div class="nv-rule" id="contributions-are-frozen-and-only-ever-added">

## A setting name and a command id are public API: the roster is frozen, and anything later is added, never renamed

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#contributions-are-frozen-and-only-ever-added"><code>ide/contributions-are-frozen-and-only-ever-added</code></a>
</div>

A setting name lives in somebody's `settings.json` and a command id in their keybindings, so renaming one
later breaks a user's configuration silently. The identifiers are therefore public API, frozen on first
contribution, and anything added later is added, never renamed.

M4B's roster. Settings: `nvs.path` (the binary, falling back to `PATH`), `nvs.lsp.enable`,
`nvs.lsp.debounce`, `nvs.lsp.trace.server`, and — added under this rule by
[`ide/reveal-is-explicit-and-window-local`](/docs/rules/ide/security-in-the-editor/#reveal-is-explicit-and-window-local "A reveal is per range, window-local, and does not survive the editor closing") and [`ide/tainted-has-no-default-decoration`](/docs/rules/ide/security-in-the-editor/#tainted-has-no-default-decoration "tainted ships no default editor decoration, and the marker is opt-in")
— `nvs.secrets.redact` (default `true`) and `nvs.taint.mark` (default `off`). Commands: `nvs.run`,
`nvs.test`, `nvs.showAst`, `nvs.restartServer`, and from the same source `nvs.revealSecret` and
`nvs.hideSecrets`. Nothing else is contributed at M4B.

M10 adds, under the same rule and not as an exception to it: the settings `nvs.check.scope`,
`nvs.codeLens.enable`, `nvs.template.services` and `nvs.completion.phpNames` (`all`/`resolved`/`off`,
default `all`), the command `nvs.checkWorkspace`, and a second request of Novis's own, `nvs/regions`. A
contributions test asserts `package.json` declares exactly what the roster names.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/ide/security-in-the-editor/#reveal-is-explicit-and-window-local" title="A reveal is per range, window-local, and does not survive the editor closing"><code>ide/reveal-is-explicit-and-window-local</code></a> <a href="/docs/rules/ide/security-in-the-editor/#tainted-has-no-default-decoration" title="tainted ships no default editor decoration, and the marker is opt-in"><code>ide/tainted-has-no-default-decoration</code></a> <a href="/docs/rules/ide/one-server-thin-clients/#the-extension-claims-nvs-only" title="The extension activates on .nvs and never claims .php"><code>ide/the-extension-claims-nvs-only</code></a> <a href="/docs/rules/ide/security-in-the-editor/#dependencies-are-allowlisted" title="The extension holds no language logic, and its package.json dependencies are checked against an allowlist by its own tests"><code>ide/dependencies-are-allowlisted</code></a> <a href="/docs/rules/ide/the-language-server/#check-json-is-the-diagnostic-record-as-a-document" title="nvs check --json writes the same diagnostic records the terminal renderer prints, as one schemaVersion: 1 document"><code>ide/check-json-is-the-diagnostic-record-as-a-document</code></a> <a href="/docs/rules/php-migration/converting-and-completing/#completion-php-names-setting" title="nvs.completion.phpNames quiets or removes the PHP-name layer, and never touches Core completion"><code>php-migration/completion-php-names-setting</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0099.md">record 0099</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0101.md">record 0101</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0108.md">record 0108</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0111.md">record 0111</a></dd></div></dl>

</div>
