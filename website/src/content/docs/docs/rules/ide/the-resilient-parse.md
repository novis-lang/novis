---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "The resilient parse"
description: "One grammar, one tree. The parser always returns something, a node it invented says so, and the file is reproducible byte for byte."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/ide/
  label: "The editor"
next:
  link: /docs/rules/ide/the-language-server/
  label: "The language server"
---

<p class="nv-section-lead">One grammar, one tree. The parser always returns something, a node it invented says so, and the file is reproducible byte for byte.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">8</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">0</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">8</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">2</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#one-grammar-one-tree">The resilient parse is the one grammar's AST plus a trivia layer and an offset index, never a second tree</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#tokens-plus-trivia-reproduce-the-file">Every token and every trivium, concatenated in offset order, reproduce the file byte for byte</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#recovery-is-explicit">A node the parser invented says so — <code>MemberName::Missing</code> and a spanned <code>ExprKind::Error</code> — and an empty span is never the signal</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#the-tree-survives-a-syntax-error">The parser always returns a tree, a node it invented says so, and an offset maps to the innermost node even inside a malformed region</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#positions-have-one-home">Position encoding is negotiated, every offset conversion lives in <code>nvs-diagnostics</code>, and a document's bytes are never normalised</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#the-index-answers-the-cursor"><code>SyntaxIndex.at(offset)</code> answers the innermost node and its ancestors, and is rebuilt per analysis</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#a-full-reanalysis-stays-under-a-bound">A full re-analysis of a ~1,000-line document stays under a named bound, and the guard is a test rather than an assumption</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#ast-json-schema-is-frozen"><code>nvs ast --json</code> has a frozen node schema, is resilient by default, and its only type-dependent field is a <code>secret</code> literal's placeholder</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li></ol>

<div class="nv-rule" id="one-grammar-one-tree">

## The resilient parse is the one grammar's AST plus a trivia layer and an offset index, never a second tree

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#one-grammar-one-tree"><code>ide/one-grammar-one-tree</code></a>
</div>

There is one grammar and one tree. The resilient parse is not a second parser mode or a `rowan`-shaped
CST beside the AST; it is the same `parse` returning a `Parsed { stmts, trivia, modifiers, index }` — the
statements whose spans cover the file, every comment and whitespace run by span in source order, where each
declaration's modifiers were written, and an offset-to-node index. `nvs check`, `nvs run` and every other compile path are that same parse followed by "refuse if
anything was reported", which is what they already do, so no call site changes and nothing has to be kept
in step with a grammar that is still moving.

The parser is already infallible — every production returns a node, never a `Result`, a missing token is
reported at its empty span without consuming what follows, and every bare-sequence loop forces a token of
progress. What the strict tree lacked was the trivia the lexer discards in `skip_trivia`, its single site,
an index, and the span each modifier was written at — recorded by the one loop every declaration's
modifiers already go through, because a `Modifier` says what a declaration is and a formatter putting a
list in one order ([`tooling/fmt-base-style-is-per`](/docs/rules/tooling/the-formatter/#fmt-base-style-is-per "The base style is PER: four-space indentation, K&R braces on control structures, Allman braces on declarations, and one canonical modifier order")) has to know where the word is. Those additions
are the whole resilient mode: each is a side channel a compile path collects none of, and none of them is
a node.

The trade is named: without `rowan`'s red/green design there is no free incremental reparse, so each
analysis reparses the document, and [`ide/a-full-reanalysis-stays-under-a-bound`](/docs/rules/ide/the-resilient-parse/#a-full-reanalysis-stays-under-a-bound "A full re-analysis of a ~1,000-line document stays under a named bound, and the guard is a test rather than an assumption") is what keeps that a
measured claim. If it fails, the first move is item-level caching over the index, not a second tree.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A PHP editor — PhpStorm, Intelephense — carries a parser of its own beside <code>php</code>; here the editor parses with the compiler's one grammar, so what the editor accepts and what <code>nvs check</code> accepts cannot drift</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/ide/the-resilient-parse/#tokens-plus-trivia-reproduce-the-file" title="Every token and every trivium, concatenated in offset order, reproduce the file byte for byte"><code>ide/tokens-plus-trivia-reproduce-the-file</code></a> <a href="/docs/rules/ide/the-resilient-parse/#recovery-is-explicit" title="A node the parser invented says so — MemberName::Missing and a spanned ExprKind::Error — and an empty span is never the signal"><code>ide/recovery-is-explicit</code></a> <a href="/docs/rules/ide/the-resilient-parse/#the-index-answers-the-cursor" title="SyntaxIndex.at(offset) answers the innermost node and its ancestors, and is rebuilt per analysis"><code>ide/the-index-answers-the-cursor</code></a> <a href="/docs/rules/ide/the-resilient-parse/#a-full-reanalysis-stays-under-a-bound" title="A full re-analysis of a ~1,000-line document stays under a named bound, and the guard is a test rather than an assumption"><code>ide/a-full-reanalysis-stays-under-a-bound</code></a> <a href="/docs/rules/tooling/the-formatter/#fmt-quotes" title="A string literal is rewritten to single quotes unless it interpolates or contains a single quote, and heredoc bodies and comments are never touched"><code>tooling/fmt-quotes</code></a> <a href="/docs/rules/ide/the-resilient-parse/#the-tree-survives-a-syntax-error" title="The parser always returns a tree, a node it invented says so, and an offset maps to the innermost node even inside a malformed region"><code>ide/the-tree-survives-a-syntax-error</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0099.md">record 0099</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0040.md">record 0040</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0137.md">record 0137</a></dd></div></dl>

</div>

<div class="nv-rule" id="tokens-plus-trivia-reproduce-the-file">

## Every token and every trivium, concatenated in offset order, reproduce the file byte for byte

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#tokens-plus-trivia-reproduce-the-file"><code>ide/tokens-plus-trivia-reproduce-the-file</code></a>
</div>

Concatenating every token's and every trivium's source text, in offset order, must equal the file byte
for byte. Losslessness is a property that is tested, not asserted: one test over the whole corpus —
`examples/`, `tests/`, and the source inside every `.nvst` case — proves that no byte of a source file
disappears between the lexer and the tree.

`Trivia { kind, span }` comes from one edit: `Lexer` gains a flag and `skip_trivia` pushes a trivium
instead of only advancing. `TriviaKind` is `Whitespace`, `LineComment` (`//`, `#`, and a run of four or
more slashes), `BlockComment`, or `DocComment` — exactly three slashes, the only variant anything but a
formatter reads. Nothing else in the lexer changes, because nothing else discards a byte.

This is the prerequisite `nvs fmt` rests on: a formatter that promises comments survive it cannot keep
that promise over a stream that drops them.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/ide/the-resilient-parse/#one-grammar-one-tree" title="The resilient parse is the one grammar's AST plus a trivia layer and an offset index, never a second tree"><code>ide/one-grammar-one-tree</code></a> <a href="/docs/rules/tooling/doc-comments-and-metadata/#doc-comment-is-three-slashes" title="A doc comment is exactly ///; //// and longer runs are ordinary comments, and # never is one"><code>tooling/doc-comment-is-three-slashes</code></a> <a href="/docs/rules/tooling/the-formatter/#fmt-quotes" title="A string literal is rewritten to single quotes unless it interpolates or contains a single quote, and heredoc bodies and comments are never touched"><code>tooling/fmt-quotes</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0099.md">record 0099</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0137.md">record 0137</a></dd></div></dl>

</div>

<div class="nv-rule" id="recovery-is-explicit">

## A node the parser invented says so — `MemberName::Missing` and a spanned `ExprKind::Error` — and an empty span is never the signal

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#recovery-is-explicit"><code>ide/recovery-is-explicit</code></a>
</div>

Where a production synthesises a node at the point of a failure, the node says so. A member name the
parser invented at the cursor is `MemberName::Missing(Span)` beside `MemberName::Ident`, and
`ExprKind::Error` carries the span of what it stood in for.

A consumer must never have to guess whether an identifier is one the user wrote or one the parser
invented. Completion's whole behaviour hangs on that distinction — `$u->` with nothing after it parses to
a property access whose name is missing, and that missing name is precisely the node member completion
needs — and an empty span is a coincidence, not a contract.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/ide/the-resilient-parse/#one-grammar-one-tree" title="The resilient parse is the one grammar's AST plus a trivia layer and an offset index, never a second tree"><code>ide/one-grammar-one-tree</code></a> <a href="/docs/rules/ide/the-resilient-parse/#the-index-answers-the-cursor" title="SyntaxIndex.at(offset) answers the innermost node and its ancestors, and is rebuilt per analysis"><code>ide/the-index-answers-the-cursor</code></a> <a href="/docs/rules/ide/the-resilient-parse/#the-tree-survives-a-syntax-error" title="The parser always returns a tree, a node it invented says so, and an offset maps to the innermost node even inside a malformed region"><code>ide/the-tree-survives-a-syntax-error</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0099.md">record 0099</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0040.md">record 0040</a></dd></div></dl>

</div>

<div class="nv-rule" id="the-tree-survives-a-syntax-error">

## The parser always returns a tree, a node it invented says so, and an offset maps to the innermost node even inside a malformed region

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#the-tree-survives-a-syntax-error"><code>ide/the-tree-survives-a-syntax-error</code></a>
</div>

A live editor spends most of its time on a syntactically invalid document — mid-statement, an unclosed
brace, a half-typed identifier — so completion and hover that go dark on the first error rarely work
when they matter. The parser therefore never fails to produce a tree: every `parse_*` returns a node
rather than a `Result`, a missing token is reported at the empty span where it should have been without
consuming what follows, and `$u->` with nothing after it parses to a property access whose name was
synthesized at the cursor.

Recovery is explicit, never inferred. A node the parser invented says so — `MemberName::Missing`, a span
on `ExprKind::Error` naming what it stood in for — because completion's whole behaviour turns on telling
a name the user wrote from one the parser made up at the cursor, and an empty span cannot carry that.

One walk builds a `SyntaxIndex` answering "the innermost node at this byte offset, and its ancestors",
so the server maps a cursor back to a syntax node even inside a malformed region, with no second
position-mapping mechanism. The test is direct: an unclosed brace or a trailing `->` does not stop
completion on the well-formed code around it. What this gives up is incremental reparse — every analysis
reparses the document — and a latency bound keeps that a measured trade.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>An editor over PHP goes dark below the first syntax error until it is fixed; here completion and hover keep working on the well-formed code around it</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/ide/the-resilient-parse/#one-grammar-one-tree" title="The resilient parse is the one grammar's AST plus a trivia layer and an offset index, never a second tree"><code>ide/one-grammar-one-tree</code></a> <a href="/docs/rules/ide/the-language-server/#the-first-server-answers-a-closed-list" title="The first nvs lsp answers six requests and two code actions, and nothing else until the workspace index exists"><code>ide/the-first-server-answers-a-closed-list</code></a> <a href="/docs/rules/ide/code-actions/#a-quick-fix-is-a-diagnostics-own-suggestion" title="An inspection is a code action backed by a diagnostic the checker emits, runs on the resilient tree, and is off by default under source.fixAll.nvs so it composes with format-on-save while nvs fmt stays layout-only"><code>ide/a-quick-fix-is-a-diagnostics-own-suggestion</code></a> <a href="/docs/rules/ide/one-server-thin-clients/#contributions-are-frozen-and-only-ever-added" title="A setting name and a command id are public API: the roster is frozen, and anything later is added, never renamed"><code>ide/contributions-are-frozen-and-only-ever-added</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0040.md">record 0040</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0099.md">record 0099</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-syntax/tests/prefixes.rs"><code>crates/nvs-syntax/tests/prefixes.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="positions-have-one-home">

## Position encoding is negotiated, every offset conversion lives in `nvs-diagnostics`, and a document's bytes are never normalised

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#positions-have-one-home"><code>ide/positions-have-one-home</code></a>
</div>

Position encoding is negotiated per LSP 3.17: the server offers `utf-8` and `utf-16`, takes `utf-8` when
the client's `general.positionEncodings` offers it, and falls back to `utf-16`, which is what VS Code sends
today. `SourceFile::line_col` counts `char`s, which is neither, so `nvs-diagnostics` gains `utf16_col(pos)`
and `offset_of(line, col, encoding)` beside it. Position arithmetic has one home and this is it; getting it
wrong is invisible on ASCII and puts every diagnostic on the wrong column the moment a file holds a `ß`.

A document is UTF-8, which is already the language's rule ([`types/bytes`](/docs/rules/types/text-and-literal-types/#bytes "bytes is a primitive peer to string for data that carries no encoding")). A buffer that is not valid
UTF-8 gets one diagnostic and no further analysis rather than a panic further in. A leading BOM is skipped
and counted, so every offset after it still lands. CRLF is preserved exactly as the document sent it —
spans are byte offsets, so normalising line endings server-side would shift every column in the file, and
the document store is the one place that could happen.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/text-and-literal-types/#bytes" title="bytes is a primitive peer to string for data that carries no encoding"><code>types/bytes</code></a> <a href="/docs/rules/ide/the-language-server/#the-server-is-synchronous" title="nvs-lsp is synchronous on lsp-server and lsp-types — a reader thread, a writer thread, one analysis thread, and no async runtime"><code>ide/the-server-is-synchronous</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0099.md">record 0099</a></dd></div></dl>

</div>

<div class="nv-rule" id="the-index-answers-the-cursor">

## `SyntaxIndex.at(offset)` answers the innermost node and its ancestors, and is rebuilt per analysis

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#the-index-answers-the-cursor"><code>ide/the-index-answers-the-cursor</code></a>
</div>

`SyntaxIndex` is built by one walk over the tree and answers `at(offset) -> NodePath`: the innermost node
containing the offset plus its ancestors. Hover, definition and completion each need a different depth of
that path, and `selectionRange` is the ancestor list itself, so the request is a projection of the index
rather than a feature built on top of it.

The index is rebuilt per analysis. Making it incremental belongs with the rest of incrementality, and the
ancestor paths are what would make item-level caching expressible if [`ide/a-full-reanalysis-stays-under-a-bound`](/docs/rules/ide/the-resilient-parse/#a-full-reanalysis-stays-under-a-bound "A full re-analysis of a ~1,000-line document stays under a named bound, and the guard is a test rather than an assumption")
ever fails.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/ide/the-resilient-parse/#one-grammar-one-tree" title="The resilient parse is the one grammar's AST plus a trivia layer and an offset index, never a second tree"><code>ide/one-grammar-one-tree</code></a> <a href="/docs/rules/ide/the-language-server/#the-request-set-is-closed" title="M4B answers nine standard requests and exactly one of Novis's own, M10 adds eight more on the same test, and that test keeps the list from growing"><code>ide/the-request-set-is-closed</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0099.md">record 0099</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0040.md">record 0040</a></dd></div></dl>

</div>

<div class="nv-rule" id="a-full-reanalysis-stays-under-a-bound">

## A full re-analysis of a ~1,000-line document stays under a named bound, and the guard is a test rather than an assumption

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#a-full-reanalysis-stays-under-a-bound"><code>ide/a-full-reanalysis-stays-under-a-bound</code></a>
</div>

A full re-analysis of a ~1,000-line document stays under a named bound, and the guard has the shape
`benches/abi-probe/tests/perf_guards.rs` already uses.

[`ide/one-grammar-one-tree`](/docs/rules/ide/the-resilient-parse/#one-grammar-one-tree "The resilient parse is the one grammar's AST plus a trivia layer and an offset index, never a second tree") traded incremental reparse away, so every analysis reparses the whole
document; this measurement is what says the trade still holds. If the guard ever fails, the answer is real
work — item-level caching over the `SyntaxIndex`, then a decision to revisit with a number in hand — and
never a smaller number in the test. Architecture assumptions are tested, not remembered, and this is the
one thing M4B built nothing to protect.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/ide/the-resilient-parse/#one-grammar-one-tree" title="The resilient parse is the one grammar's AST plus a trivia layer and an offset index, never a second tree"><code>ide/one-grammar-one-tree</code></a> <a href="/docs/rules/ide/the-language-server/#an-open-document-is-its-own-entry-point" title="Each open document is analysed as its own entry point, with open buffers overlaid on disk, and diagnostics are published only for open documents"><code>ide/an-open-document-is-its-own-entry-point</code></a> <a href="/docs/rules/testing/measuring-performance/#perf-two-mechanisms" title="A per-PR guard and the historical dashboard are separate measurements, and never one"><code>testing/perf-two-mechanisms</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0099.md">record 0099</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0040.md">record 0040</a></dd></div></dl>

</div>

<div class="nv-rule" id="ast-json-schema-is-frozen">

## `nvs ast --json` has a frozen node schema, is resilient by default, and its only type-dependent field is a `secret` literal's placeholder

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#ast-json-schema-is-frozen"><code>ide/ast-json-schema-is-frozen</code></a>
</div>

`nvs ast` gains `--json`, because the AST panel needs a stable shape and `{stmts:#?}` — Rust's derived
`Debug` — has none: any field reordering in any AST struct changes it. The schema is a node object of
`kind`, `span` as `[start, end]`, the node's own scalar fields, and `children`. Trivia and recovery nodes
are included, because a panel that hides them is least useful on exactly the file the developer is looking
at the panel to understand, and `--resilient` is the default: the panel's whole value is on a file that
does not compile. A snapshot test over `examples/` freezes it.

One scalar field is not the source text, and it is the only place this output depends on anything past the
parse: a literal node whose static type carries `secret` emits the fixed placeholder
[`security/secret-sinks-refuse`](/docs/rules/security/secrets/#secret-sinks-refuse "Every output sink refuses a secret value outright, and no auto-escape or neutralisation bypasses one") gives a dumped property, rather than its own bytes
([`security/redaction-reaches-the-tools-own-renderings`](/docs/rules/security/secrets/#redaction-reaches-the-tools-own-renderings "No rendering the toolchain itself produces prints a secret in the clear, the AST dump included")). Putting that in the JSON rather than in the
panel is what stops `--json` and the webview from disagreeing about it.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/secrets/#redaction-reaches-the-tools-own-renderings" title="No rendering the toolchain itself produces prints a secret in the clear, the AST dump included"><code>security/redaction-reaches-the-tools-own-renderings</code></a> <a href="/docs/rules/security/secrets/#secret-sinks-refuse" title="Every output sink refuses a secret value outright, and no auto-escape or neutralisation bypasses one"><code>security/secret-sinks-refuse</code></a> <a href="/docs/rules/ide/the-resilient-parse/#one-grammar-one-tree" title="The resilient parse is the one grammar's AST plus a trivia layer and an offset index, never a second tree"><code>ide/one-grammar-one-tree</code></a> <a href="/docs/rules/ide/the-language-server/#every-feature-is-staged-behind-its-dependency" title="The VS Code client goes as deep as the editor allows, and each feature waits for the language or runtime piece it needs rather than shipping as a stub"><code>ide/every-feature-is-staged-behind-its-dependency</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0099.md">record 0099</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0101.md">record 0101</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0040.md">record 0040</a></dd></div></dl>

</div>
