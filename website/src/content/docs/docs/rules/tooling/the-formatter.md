---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "The formatter"
description: "One canonical style, no configuration, no reflowing of what you wrote — and a fixed point over the input bytes."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/tooling/the-terminal/
  label: "The terminal and command-line programs"
next:
  link: /docs/rules/tooling/doc-comments-and-metadata/
  label: "Doc comments and `nvs meta`"
---

<p class="nv-section-lead">One canonical style, no configuration, no reflowing of what you wrote — and a fixed point over the input bytes.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">13</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">13</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">0</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">10</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#fmt-is-one-canonical-style"><code>nvs fmt</code> has one style, takes no configuration, and its output is a pure function of the file it is given</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#fmt-base-style-is-per">The base style is PER: four-space indentation, K&amp;R braces on control structures, Allman braces on declarations, and one canonical modifier order</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#fmt-never-reflows"><code>nvs fmt</code> never decides where an expression breaks: the author's own line breaks are kept and only what surrounds them is normalized</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#fmt-quotes">A string literal is rewritten to single quotes unless it interpolates or contains a single quote, and heredoc bodies and comments are never touched</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#fmt-trailing-commas">A comma-separated list that spans more than one line gets a trailing comma, and a list on one line never does</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#fmt-sorts-the-use-block">Consecutive <code>use</code> declarations are sorted by full path, ascending and case-sensitive, with no blank line between them</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#fmt-novis-constructs">Every construct PER never saw — qualifiers, <code>lateinit</code>, <code>fn</code> closures, <code>match</code>, object literals, shape types, enum cases, markup literals, a <code>?&gt;</code> on its own line — has exactly one layout</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#fmt-normalizes-only-reserved-spellings"><code>nvs fmt</code> lower-cases a mis-cased reserved spelling only where that spelling has no other legal meaning — a duration unit and the open tag, never a keyword or an identifier</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#fmt-never-inserts-visibility"><code>nvs fmt</code> never inserts a visibility keyword; <code>nvs convert</code> inserts <code>public</code> as an E-tier rewrite</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#fmt-never-reorders-members"><code>nvs fmt</code> never reorders class members, because declaration order is observable in what a program prints and sends</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#fmt-is-idempotent"><code>nvs fmt</code> is a fixed point over the input bytes: byte-stable on every machine, and deliberately not source-independent</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#fmt-is-never-a-diagnostic"><code>nvs fmt</code> is a separate opt-in tool: no compiler command runs it, an unformatted file is never a diagnostic, and an editor composes it with quick fixes in the client</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#fmt-check-writes-nothing"><code>nvs fmt</code> rewrites in place; <code>--check</code> writes nothing and exits non-zero naming every file that would change; <code>--diff</code> prints the diff instead</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li></ol>

<div class="nv-rule" id="fmt-is-one-canonical-style">

## `nvs fmt` has one style, takes no configuration, and its output is a pure function of the file it is given

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#fmt-is-one-canonical-style"><code>tooling/fmt-is-one-canonical-style</code></a>
</div>

`nvs fmt` rewrites a `.nvs` file into one canonical layout. There is no config file, no per-project or
per-directory override, and no flag that changes output: the result for a given input is a pure function
of that input and nothing else, the stance [`core-api/identifier-casing`](/docs/rules/core-api/naming-and-shape/#identifier-casing "An identifier's casing is fixed by its category and a mismatch is a hard compile error") takes for naming extended to
layout. The only flags are I/O modes — `--check`, `--diff`, `--stdin` and the target paths — never a
style knob. A configurable knob would let two files in one project, or two projects run through the same
tool, disagree about what "formatted" means, which is the exact question a canonical formatter exists to
close.

The style is PER wherever Novis's grammar matches PHP's ([`tooling/fmt-base-style-is-per`](/docs/rules/tooling/the-formatter/#fmt-base-style-is-per "The base style is PER: four-space indentation, K&R braces on control structures, Allman braces on declarations, and one canonical modifier order")), extended
with one layout for each construct PER has never seen ([`tooling/fmt-novis-constructs`](/docs/rules/tooling/the-formatter/#fmt-novis-constructs "Every construct PER never saw — qualifiers, lateinit, fn closures, match, object literals, shape types, enum cases, markup literals, a ?> on its own line — has exactly one layout")). It is
deterministic in the gofmt sense, not Prettier's: it never reflows an expression to fit a width
([`tooling/fmt-never-reflows`](/docs/rules/tooling/the-formatter/#fmt-never-reflows "nvs fmt never decides where an expression breaks: the author's own line breaks are kept and only what surrounds them is normalized")), running it twice changes nothing ([`tooling/fmt-is-idempotent`](/docs/rules/tooling/the-formatter/#fmt-is-idempotent "nvs fmt is a fixed point over the input bytes: byte-stable on every machine, and deliberately not source-independent")),
and it is a separate opt-in tool no compiler command ever runs ([`tooling/fmt-is-never-a-diagnostic`](/docs/rules/tooling/the-formatter/#fmt-is-never-a-diagnostic "nvs fmt is a separate opt-in tool: no compiler command runs it, an unformatted file is never a diagnostic, and an editor composes it with quick fixes in the client")).

Changing any of these rules later is a real diff across every already-formatted file, not a settings
change, so the rule set is stable once it ships. The formatter reads the lossless tree, never the strict
parse that drops comments — a walk over the strict tree would delete every comment in the file.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no <code>.php-cs-fixer.php</code>, no ruleset and no style flag; every knob php-cs-fixer and PHP_CodeSniffer expose is one fixed answer here</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/tooling/the-formatter/#fmt-base-style-is-per" title="The base style is PER: four-space indentation, K&amp;R braces on control structures, Allman braces on declarations, and one canonical modifier order"><code>tooling/fmt-base-style-is-per</code></a> <a href="/docs/rules/tooling/the-formatter/#fmt-never-reflows" title="nvs fmt never decides where an expression breaks: the author's own line breaks are kept and only what surrounds them is normalized"><code>tooling/fmt-never-reflows</code></a> <a href="/docs/rules/tooling/the-formatter/#fmt-is-idempotent" title="nvs fmt is a fixed point over the input bytes: byte-stable on every machine, and deliberately not source-independent"><code>tooling/fmt-is-idempotent</code></a> <a href="/docs/rules/tooling/the-formatter/#fmt-is-never-a-diagnostic" title="nvs fmt is a separate opt-in tool: no compiler command runs it, an unformatted file is never a diagnostic, and an editor composes it with quick fixes in the client"><code>tooling/fmt-is-never-a-diagnostic</code></a> <a href="/docs/rules/core-api/naming-and-shape/#identifier-casing" title="An identifier's casing is fixed by its category and a mismatch is a hard compile error"><code>core-api/identifier-casing</code></a> <a href="/docs/rules/core-api/naming-and-shape/#casing-has-no-suppression" title="There is no suppression mechanism for a casing error, in any category"><code>core-api/casing-has-no-suppression</code></a> <a href="/docs/rules/ide/one-server-thin-clients/#one-server-two-thin-clients" title="Language smarts and formatting of Novis have one implementation each, nvs-lsp and nvs-fmt, and an editor client holds none of either"><code>ide/one-server-two-thin-clients</code></a> <a href="/docs/rules/ide/the-resilient-parse/#the-tree-survives-a-syntax-error" title="The parser always returns a tree, a node it invented says so, and an offset maps to the innermost node even inside a malformed region"><code>ide/the-tree-survives-a-syntax-error</code></a> <a href="/docs/rules/ide/the-resilient-parse/#one-grammar-one-tree" title="The resilient parse is the one grammar's AST plus a trivia layer and an offset index, never a second tree"><code>ide/one-grammar-one-tree</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0039.md">record 0039</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-fmt/tests/fixtures.rs"><code>crates/nvs-fmt/tests/fixtures.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-cli/tests/fmt.rs"><code>crates/nvs-cli/tests/fmt.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="fmt-base-style-is-per">

## The base style is PER: four-space indentation, K&R braces on control structures, Allman braces on declarations, and one canonical modifier order

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#fmt-base-style-is-per"><code>tooling/fmt-base-style-is-per</code></a>
</div>

Four-space indentation and no tabs; one statement per line; a file ends with exactly one newline and no
line carries trailing whitespace.

Braces split by *kind of body*. Every control structure — `if`/`elseif`/`else`, `while`, `do`/`while`,
`for`, `foreach`, `switch`, `try`/`catch`/`finally` — is K&R: the opening brace stays on the keyword's
line after one space, the closing brace starts its own line, and `elseif`, `else`, `catch` and `finally`
continue on the closing brace's line. `elseif` is one word, never `else if`. Every declaration with a
body — `class`, `interface`, `enum`, a named function or method, including an interface method that
carries a body — is Allman: the opening brace starts its own line at the declaration's indentation.

Exactly one blank line follows a `namespace` line, one follows the `use` block, and one separates two
members that each have a body; adjacent simple property and constant declarations get none. A comment
between two members belongs to the member below it, so the blank line goes above the comment.

Modifier order is canonical, not the author's: `abstract`/`final`, then visibility (including
`private(set)`), then `static`, then `readonly`, then `lateinit`, one space between each. The parser
accepts these in any order, which is exactly why the formatter must fix one — `static public $x;` and
`public static $x;` both compile and would otherwise never converge. A missing modifier is never
supplied: a formatter that changes meaning is not a formatter ([`core-api/written-visibility`](/docs/rules/core-api/everything-is-written/#written-visibility "Every member declaration writes exactly one visibility keyword, and there is no implicit public")).

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>PER's modifier order is enforced rather than recommended, and <code>else if</code> is always written <code>elseif</code></p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/tooling/the-formatter/#fmt-is-one-canonical-style" title="nvs fmt has one style, takes no configuration, and its output is a pure function of the file it is given"><code>tooling/fmt-is-one-canonical-style</code></a> <a href="/docs/rules/tooling/the-formatter/#fmt-sorts-the-use-block" title="Consecutive use declarations are sorted by full path, ascending and case-sensitive, with no blank line between them"><code>tooling/fmt-sorts-the-use-block</code></a> <a href="/docs/rules/tooling/the-formatter/#fmt-novis-constructs" title="Every construct PER never saw — qualifiers, lateinit, fn closures, match, object literals, shape types, enum cases, markup literals, a ?&gt; on its own line — has exactly one layout"><code>tooling/fmt-novis-constructs</code></a> <a href="/docs/rules/core-api/everything-is-written/#written-visibility" title="Every member declaration writes exactly one visibility keyword, and there is no implicit public"><code>core-api/written-visibility</code></a> <a href="/docs/rules/classes/properties/#lateinit" title="A lateinit property is assigned after the constructor rather than inside it"><code>classes/lateinit</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0039.md">record 0039</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-fmt/tests/base_style.rs"><code>crates/nvs-fmt/tests/base_style.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="fmt-never-reflows">

## `nvs fmt` never decides where an expression breaks: the author's own line breaks are kept and only what surrounds them is normalized

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#fmt-never-reflows"><code>tooling/fmt-never-reflows</code></a>
</div>

Whether an expression, a call-argument list, an array, object or shape literal, a `match` arm list or an
enum-case list spans one line or several is the author's choice, and `nvs fmt` preserves it exactly. It
normalizes what surrounds that choice — the indentation of continuation lines, spacing, and brace
placement — and nothing inside it. A hand-wrapped multi-line call is never collapsed onto one line; a long
one-line call is never split.

There is deliberately no line-length rule anywhere, soft or hard. With no reflow decision to make, a width
limit would be advisory prose with nothing in the tool to enforce it.

The cost is stated and accepted: the formatter cannot repair a badly wrapped call by itself, and a human
still decides when an expression is long enough to wrap. What that buys is a formatter that is a
whitespace, brace and order normalizer walking the existing parse tree rather than a width-fitting doc
printer this project has no other user of — and one that stays trivially byte-for-byte deterministic as the
parser evolves ([`tooling/fmt-is-idempotent`](/docs/rules/tooling/the-formatter/#fmt-is-idempotent "nvs fmt is a fixed point over the input bytes: byte-stable on every machine, and deliberately not source-independent")).

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Php-cs-fixer and Prettier-style tools re-wrap to a column limit; here there is no line-length rule at all, soft or hard</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/tooling/the-formatter/#fmt-is-idempotent" title="nvs fmt is a fixed point over the input bytes: byte-stable on every machine, and deliberately not source-independent"><code>tooling/fmt-is-idempotent</code></a> <a href="/docs/rules/tooling/the-formatter/#fmt-trailing-commas" title="A comma-separated list that spans more than one line gets a trailing comma, and a list on one line never does"><code>tooling/fmt-trailing-commas</code></a> <a href="/docs/rules/expressions/pipeline-and-catch/#pipeline-substitution" title="|&gt; substitutes one hole at parse time, and has no run-time representation"><code>expressions/pipeline-substitution</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0039.md">record 0039</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-fmt/tests/never_reflows.rs"><code>crates/nvs-fmt/tests/never_reflows.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="fmt-quotes">

## A string literal is rewritten to single quotes unless it interpolates or contains a single quote, and heredoc bodies and comments are never touched

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#fmt-quotes"><code>tooling/fmt-quotes</code></a>
</div>

A string literal is rewritten to single quotes, with two exceptions that both use double quotes instead:
a literal that interpolates — only a double-quoted string or a heredoc can, unchanged from PHP — and a
literal containing a single quote that single-quoting would force to be escaped.

A literal holding a backslash escape keeps its double quotes, which is the same exception read over
the whole escape grammar rather than over the one character: `\n` is a newline between double quotes
and a backslash and an `n` between single ones, and single quotes have only `\\` and `\'` to offer
back. Respelling one would change the string's value, so the test the formatter actually applies is
that both spellings name the same string — a body with no `'` and no `\` in it.

Heredoc and nowdoc bodies and every comment are left byte-for-byte untouched. Rewriting a heredoc's body
would change the program's own string value, not its layout; and a comment is content the formatter has no
opinion about.

This is the one rewrite, with [`tooling/fmt-trailing-commas`](/docs/rules/tooling/the-formatter/#fmt-trailing-commas "A comma-separated list that spans more than one line gets a trailing comma, and a list on one line never does"), that goes beyond whitespace. A
whitespace-only formatter was rejected because two semantically identical files would still differ
byte-for-byte after formatting, which undercuts the point of having one canonical layout at all.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/tooling/the-formatter/#fmt-is-one-canonical-style" title="nvs fmt has one style, takes no configuration, and its output is a pure function of the file it is given"><code>tooling/fmt-is-one-canonical-style</code></a> <a href="/docs/rules/tooling/the-formatter/#fmt-is-idempotent" title="nvs fmt is a fixed point over the input bytes: byte-stable on every machine, and deliberately not source-independent"><code>tooling/fmt-is-idempotent</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0039.md">record 0039</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-fmt/tests/token_rules.rs"><code>crates/nvs-fmt/tests/token_rules.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="fmt-trailing-commas">

## A comma-separated list that spans more than one line gets a trailing comma, and a list on one line never does

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#fmt-trailing-commas"><code>tooling/fmt-trailing-commas</code></a>
</div>

Every comma-separated list that spans more than one line gets a trailing comma after its last element:
call arguments, parameter lists, array literals, shape-type fields, object literals, `match` arm lists
including the `default` arm, and multi-line enum-case lists. A list kept on one line never gets one.

The test that decides it is where the **closing delimiter** sits: the comma is there when a line break
separates the last element from that delimiter, and is not when the delimiter follows the element on the
element's own line. Those are the same question wherever a closer opens a line of its own, which is what
spreading a list over several lines ordinarily means. Where they come apart — an author who wrote
`foo(\n    $a, $b)` spread the list over two lines and still ended it on one — the comma would sit in
front of the `)` and buy none of the one-line diff this rule exists for, so it is not written. A comma
written there by hand is deleted for the same reason one is inserted: a canonical style has one spelling
of a list, not two.

Whether a list spans several lines is the author's decision, which [`tooling/fmt-never-reflows`](/docs/rules/tooling/the-formatter/#fmt-never-reflows "nvs fmt never decides where an expression breaks: the author's own line breaks are kept and only what surrounds them is normalized")
preserves; the comma follows from that decision mechanically. This removes the one place PHP's grammar
leaves a genuinely free stylistic choice with no way to derive the right answer from context, and it is
what makes adding an element to a multi-line list a one-line diff.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Php-cs-fixer's <code>trailing_comma_in_multiline</code> is a choice; here it is the only layout</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/tooling/the-formatter/#fmt-never-reflows" title="nvs fmt never decides where an expression breaks: the author's own line breaks are kept and only what surrounds them is normalized"><code>tooling/fmt-never-reflows</code></a> <a href="/docs/rules/tooling/the-formatter/#fmt-novis-constructs" title="Every construct PER never saw — qualifiers, lateinit, fn closures, match, object literals, shape types, enum cases, markup literals, a ?&gt; on its own line — has exactly one layout"><code>tooling/fmt-novis-constructs</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0039.md">record 0039</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-fmt/tests/token_rules.rs"><code>crates/nvs-fmt/tests/token_rules.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="fmt-sorts-the-use-block">

## Consecutive `use` declarations are sorted by full path, ascending and case-sensitive, with no blank line between them

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#fmt-sorts-the-use-block"><code>tooling/fmt-sorts-the-use-block</code></a>
</div>

Consecutive `use` declarations are sorted lexicographically by their full path, ascending and
case-sensitive, with no blank line between them. Exactly one blank line separates that block from the
`namespace` line above and from the first real declaration below.

A `use` declaration names exactly one imported path, so there is no PHP-style grouped `use A\{B, C};`
form to order or to expand. This is the only reordering the formatter performs anywhere: class members
keep the order their author wrote ([`tooling/fmt-never-reorders-members`](/docs/rules/tooling/the-formatter/#fmt-never-reorders-members "nvs fmt never reorders class members, because declaration order is observable in what a program prints and sends")), because an import's
position is not observable and a member's is.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no grouped <code>use A\{B, C};</code> form to order, because Novis's <code>use</code> names exactly one path</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/tooling/the-formatter/#fmt-base-style-is-per" title="The base style is PER: four-space indentation, K&amp;R braces on control structures, Allman braces on declarations, and one canonical modifier order"><code>tooling/fmt-base-style-is-per</code></a> <a href="/docs/rules/tooling/the-formatter/#fmt-never-reorders-members" title="nvs fmt never reorders class members, because declaration order is observable in what a program prints and sends"><code>tooling/fmt-never-reorders-members</code></a> <a href="/docs/rules/statements/names-and-require/#a-qualified-name-is-absolute" title="A name containing a separator is read from the root, and one without it through the imports"><code>statements/a-qualified-name-is-absolute</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0039.md">record 0039</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-fmt/tests/use_block.rs"><code>crates/nvs-fmt/tests/use_block.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="fmt-novis-constructs">

## Every construct PER never saw — qualifiers, `lateinit`, `fn` closures, `match`, object literals, shape types, enum cases, markup literals, a `?>` on its own line — has exactly one layout

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#fmt-novis-constructs"><code>tooling/fmt-novis-constructs</code></a>
</div>

Each construct with no PER precedent has one layout, chosen once:

- A `tainted` or `secret` qualifier sits one space before the type it qualifies, and a nullable type's
  `?` is written in front of the qualifier: `?tainted string $x`, `secret tainted bytes $y`. The grammar
  already fixes that order — a qualifier takes a scalar or a shape, and `?` wraps the qualified type
  rather than sitting inside it ([`security/tainted-qualifier`](/docs/rules/security/tainted-data/#tainted-qualifier "tainted is a compile-time qualifier on string, bytes and a shape of them, spellable in any declaration and erased before codegen")) — so this is only the spacing.
- `lateinit` takes its place at the end of the modifier order ([`tooling/fmt-base-style-is-per`](/docs/rules/tooling/the-formatter/#fmt-base-style-is-per "The base style is PER: four-space indentation, K&R braces on control structures, Allman braces on declarations, and one canonical modifier order")).
- An `fn` closure is an expression, so its body brace stays on the line of its parameter list, return
  type and `=>`, as PER already lays out an anonymous function: `fn(int $x): int => { return $x + 1; }`.
  With a multi-line body only the closing brace gets its own line.
- A `match` expression puts each arm on its own line unless the whole arm list was written on one, and a
  multi-line list ends in a trailing comma.
- An object literal or shape type on one line has one space inside each brace, `{ a: 1, b: 2 }`; across
  lines it is one field per line, indented one level, with a trailing comma. An empty one has no inside
  to space and stays `{}`.
- Enum cases are one per line when the author wrote them that way, with a trailing comma when multi-line.
- A markup literal's body is never touched — not reflowed, not re-indented, not re-quoted — exactly as a
  heredoc body and an inline-HTML region are not ([`tooling/fmt-quotes`](/docs/rules/tooling/the-formatter/#fmt-quotes "A string literal is rewritten to single quotes unless it interpolates or contains a single quote, and heredoc bodies and comments are never touched"),
  [`core-classes/html-literal`](/docs/rules/core-classes/regex-html-and-introspection/#html-literal "html…  is a Core\Html\Markup whose segments are trusted and whose holes are escaped")). Only its surroundings are laid out, so the bytes between the
  backticks survive formatting unchanged and `nvs fmt` stays idempotent over a template.
- A `?>` that begins its line is indented to the depth of the block it sits in — the column a statement
  there would start at — so the markup after it can start from the code around it
  ([`ide/a-template-region-gets-the-editors-services-and-formatter`](/docs/rules/ide/highlighting-and-completion/#a-template-region-gets-the-editors-services-and-formatter "An inline-HTML region gets the editor's own HTML, CSS and JavaScript services on boundaries the server reports, and the editor's HTML formatter after nvs fmt, indented from the Novis code around it")). Only its leading whitespace
  moves, and that whitespace is code, so nothing the program prints changes: a `?>` is never moved onto
  or off a line, and the markup after it is the editor's to lay out, never `nvs fmt`'s.
- Attributes need no rule, since Novis has no annotation syntax.

Several of these have exactly one contributor and no convention to defer to. Changing one later is a
breaking rewrite of every formatted file, the same cost class casing already accepted.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/tooling/the-formatter/#fmt-base-style-is-per" title="The base style is PER: four-space indentation, K&amp;R braces on control structures, Allman braces on declarations, and one canonical modifier order"><code>tooling/fmt-base-style-is-per</code></a> <a href="/docs/rules/tooling/the-formatter/#fmt-trailing-commas" title="A comma-separated list that spans more than one line gets a trailing comma, and a list on one line never does"><code>tooling/fmt-trailing-commas</code></a> <a href="/docs/rules/security/tainted-data/#tainted-qualifier" title="tainted is a compile-time qualifier on string, bytes and a shape of them, spellable in any declaration and erased before codegen"><code>security/tainted-qualifier</code></a> <a href="/docs/rules/security/secrets/#secret-qualifier" title="secret is a second, independent compile-time qualifier, written before tainted and in that order alone"><code>security/secret-qualifier</code></a> <a href="/docs/rules/types/closures/#closure-literal" title="fn is the only closure literal, with an expression body or a block body"><code>types/closure-literal</code></a> <a href="/docs/rules/types/objects-and-shapes/#object-literal" title="{name: value} builds an anonymous, methodless object and nothing more"><code>types/object-literal</code></a> <a href="/docs/rules/types/objects-and-shapes/#shape-type" title="{name: T} in type position is a structural shape checked by width subtyping, and {name?: T} marks a key that may be absent"><code>types/shape-type</code></a> <a href="/docs/rules/enums/#declaration" title="A case with no written value counts on from the one before it, starting at zero"><code>enums/declaration</code></a> <a href="/docs/rules/core-classes/regex-html-and-introspection/#html-literal" title="html…  is a Core\Html\Markup whose segments are trusted and whose holes are escaped"><code>core-classes/html-literal</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0039.md">record 0039</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0169.md">record 0169</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0173.md">record 0173</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-fmt/tests/novis_constructs.rs"><code>crates/nvs-fmt/tests/novis_constructs.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-fmt/tests/two_modes.rs"><code>crates/nvs-fmt/tests/two_modes.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="fmt-normalizes-only-reserved-spellings">

## `nvs fmt` lower-cases a mis-cased reserved spelling only where that spelling has no other legal meaning — a duration unit and the open tag, never a keyword or an identifier

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#fmt-normalizes-only-reserved-spellings"><code>tooling/fmt-normalizes-only-reserved-spellings</code></a>
</div>

`nvs fmt` normalizes a mis-cased reserved spelling to its lower-case form only where that mis-cased
spelling has no other legal meaning. Two qualify, and each already carries the diagnostic that names the
fix: a duration literal's unit, `5Min` → `5min` ([`types/duration-literal`](/docs/rules/types/text-and-literal-types/#duration-literal "1h30m is a Core\Time\Duration constant, in one grammar shared by source, parse and nvs.toml")), and the open tag,
`<?NVS` → `<?nvs` ([`classes/reserved-spellings-are-lower-case`](/docs/rules/classes/declaring-a-class/#reserved-spellings-are-lower-case "Keywords, contextual keywords and the <?nvs tag are lower case and nothing else")).

Both spellings are errors, and that is how `nvs fmt` reaches them rather than what stops it: the
formatter refuses a file for an error it does not itself rewrite, not for reporting one at all. A
mis-cased reserved spelling leaves a whole tree behind it — the lexer read the tag or the duration
literal it was handed, and `E_RESERVED_SPELLING_CASE`'s primary span is exactly the bytes to
lower-case — so the file is formatted and the spelling goes out in its one form. The alternative,
leaving the rewrite to an editor's code action and having `nvs fmt` refuse the file, would make this
rule name two rewrites nothing performs. What the formatter does not promise is that the result
compiles: a literal that is mis-cased *and* out of order comes back lower-cased and still refused,
by the diagnostic that was always its own.

The criterion is what generalizes, not the list. A keyword never qualifies: `IF` and `ECHO` are legal
`PascalCase` class names under [`core-api/casing-checks-the-leading-character`](/docs/rules/core-api/naming-and-shape/#casing-checks-the-leading-character "Only the leading character's case is checked, so HTTPClient and HttpClient both pass"), so nothing lexical
separates a mis-typed keyword from a deliberate class reference, and a formatter that rewrote one would be
the only place in the toolchain that guesses. An identifier never qualifies for a different reason: fixing
its case is a *rename*, which must reach every use site across the workspace, and `nvs fmt` is a
single-file walk — that rename is an editor's workspace-wide code action. A duration unit and an open tag
can be nothing else, which is why they and only they are here. Normalizing PHP's case-insensitive
reserved words is the converter's job, where the input is known to be PHP.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>PHP's case-insensitive keywords are not normalized; <code>IF</code> is a legal class name and fixing an identifier's case is a rename, not a layout</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/text-and-literal-types/#duration-literal" title="1h30m is a Core\Time\Duration constant, in one grammar shared by source, parse and nvs.toml"><code>types/duration-literal</code></a> <a href="/docs/rules/classes/declaring-a-class/#reserved-spellings-are-lower-case" title="Keywords, contextual keywords and the &lt;?nvs tag are lower case and nothing else"><code>classes/reserved-spellings-are-lower-case</code></a> <a href="/docs/rules/core-api/naming-and-shape/#casing-checks-the-leading-character" title="Only the leading character's case is checked, so HTTPClient and HttpClient both pass"><code>core-api/casing-checks-the-leading-character</code></a> <a href="/docs/rules/tooling/the-formatter/#fmt-is-never-a-diagnostic" title="nvs fmt is a separate opt-in tool: no compiler command runs it, an unformatted file is never a diagnostic, and an editor composes it with quick fixes in the client"><code>tooling/fmt-is-never-a-diagnostic</code></a> <a href="/docs/rules/tooling/nvs-convert/#convert-one-table-two-modes" title="nvs convert is one rule table read through two modes, and every branch carries a tier"><code>tooling/convert-one-table-two-modes</code></a> <a href="/docs/rules/ide/the-language-server/#every-feature-is-staged-behind-its-dependency" title="The VS Code client goes as deep as the editor allows, and each feature waits for the language or runtime piece it needs rather than shipping as a stub"><code>ide/every-feature-is-staged-behind-its-dependency</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0039.md">record 0039</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-fmt/tests/token_rules.rs"><code>crates/nvs-fmt/tests/token_rules.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="fmt-never-inserts-visibility">

## `nvs fmt` never inserts a visibility keyword; `nvs convert` inserts `public` as an E-tier rewrite

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#fmt-never-inserts-visibility"><code>tooling/fmt-never-inserts-visibility</code></a>
</div>

[`core-api/written-visibility`](/docs/rules/core-api/everything-is-written/#written-visibility "Every member declaration writes exactly one visibility keyword, and there is no implicit public") makes an omitted visibility keyword a compile error. Two tools
meet that error, and they answer it in opposite ways.

**`nvs fmt` never inserts the keyword.** The formatter orders modifiers and does not supply a missing
one. A formatter that inserted `public` would make a file's *meaning* depend on whether a tool had
been run over it, and would restore PHP's implicit default through the back door for anyone who
formats on save. A file that does not compile still does not compile after `nvs fmt`.

**`nvs convert` does insert it**, as an **E-tier** row of its rule table
([`tooling/convert-one-table-two-modes`](/docs/rules/tooling/nvs-convert/#convert-one-table-two-modes "nvs convert is one rule table read through two modes, and every branch carries a tier")): PHP's omission provably means `public`, so writing the
word is a behaviour-identical rewrite, discharged by a differential case like any other E branch
([`tooling/convert-equivalent-is-proven`](/docs/rules/tooling/nvs-convert/#convert-equivalent-is-proven "A tier-E rewrite names a differential case against the PHP oracle, or it is not E")). Porting a PHP file therefore costs the author nothing
here, and the ported member reports the level PHP actually gave it.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Php-cs-fixer's <code>visibility_required</code> fixer writes the missing <code>public</code>; the formatter here never changes meaning, and the insertion happens only in <code>nvs convert</code></p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-api/everything-is-written/#written-visibility" title="Every member declaration writes exactly one visibility keyword, and there is no implicit public"><code>core-api/written-visibility</code></a> <a href="/docs/rules/tooling/nvs-convert/#convert-one-table-two-modes" title="nvs convert is one rule table read through two modes, and every branch carries a tier"><code>tooling/convert-one-table-two-modes</code></a> <a href="/docs/rules/tooling/nvs-convert/#convert-equivalent-is-proven" title="A tier-E rewrite names a differential case against the PHP oracle, or it is not E"><code>tooling/convert-equivalent-is-proven</code></a> <a href="/docs/rules/tooling/the-formatter/#fmt-base-style-is-per" title="The base style is PER: four-space indentation, K&amp;R braces on control structures, Allman braces on declarations, and one canonical modifier order"><code>tooling/fmt-base-style-is-per</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0094.md">record 0094</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0089.md">record 0089</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0039.md">record 0039</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-fmt/tests/never_rewrites_a_declaration.rs"><code>crates/nvs-fmt/tests/never_rewrites_a_declaration.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="fmt-never-reorders-members">

## `nvs fmt` never reorders class members, because declaration order is observable in what a program prints and sends

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#fmt-never-reorders-members"><code>tooling/fmt-never-reorders-members</code></a>
</div>

Methods, properties, class constants and enum cases keep the order their author wrote. No "group by
visibility, constants before properties before methods" rule exists, because declaration order is
observable: [`core-classes/derive-field-list`](/docs/rules/core-classes/codecs-sessions-and-signatures/#derive-field-list "The field list is the declared property list, and every field is a constructor parameter of the same name") makes a derived codec's encode order the property
declaration order, on purpose, for ETags and cached fixtures; and [`testing/bench-counters`](/docs/rules/testing/measuring-performance/#bench-counters "#[Bench] reports counted semantic work, and CI may gate on the counts but never on wall-clock") makes the
test runner's report order the declaration order of the cases. A formatter that reordered members would
change what a program prints and sends, and a formatter that changes meaning is not a formatter — the
same line [`core-api/written-visibility`](/docs/rules/core-api/everything-is-written/#written-visibility "Every member declaration writes exactly one visibility keyword, and there is no implicit public") takes when it refuses to let the formatter insert a missing
`public`, and [`classes/comparable`](/docs/rules/classes/no-magic/#comparable "Two objects order only when their class implements Comparable, and there is no property-walk fallback") takes for the converter.

PER has no member-ordering rule to defer to in any case; the convention people associate with it is one
PHP tool's. A developer who wants the reordering can have it as a deliberate, diff-visible code action. It
is never something a formatter does on save. The `use` block ([`tooling/fmt-sorts-the-use-block`](/docs/rules/tooling/the-formatter/#fmt-sorts-the-use-block "Consecutive use declarations are sorted by full path, ascending and case-sensitive, with no blank line between them")) is
the only reordering anywhere.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Php-cs-fixer's <code>ordered_class_elements</code> has no counterpart; a member's position is kept</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/tooling/the-formatter/#fmt-sorts-the-use-block" title="Consecutive use declarations are sorted by full path, ascending and case-sensitive, with no blank line between them"><code>tooling/fmt-sorts-the-use-block</code></a> <a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#derive-field-list" title="The field list is the declared property list, and every field is a constructor parameter of the same name"><code>core-classes/derive-field-list</code></a> <a href="/docs/rules/testing/measuring-performance/#bench-counters" title="#[Bench] reports counted semantic work, and CI may gate on the counts but never on wall-clock"><code>testing/bench-counters</code></a> <a href="/docs/rules/core-api/everything-is-written/#written-visibility" title="Every member declaration writes exactly one visibility keyword, and there is no implicit public"><code>core-api/written-visibility</code></a> <a href="/docs/rules/classes/no-magic/#comparable" title="Two objects order only when their class implements Comparable, and there is no property-walk fallback"><code>classes/comparable</code></a> <a href="/docs/rules/tooling/nvs-convert/#convert-one-table-two-modes" title="nvs convert is one rule table read through two modes, and every branch carries a tier"><code>tooling/convert-one-table-two-modes</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0039.md">record 0039</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-fmt/tests/never_rewrites_a_declaration.rs"><code>crates/nvs-fmt/tests/never_rewrites_a_declaration.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="fmt-is-idempotent">

## `nvs fmt` is a fixed point over the input bytes: byte-stable on every machine, and deliberately not source-independent

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#fmt-is-idempotent"><code>tooling/fmt-is-idempotent</code></a>
</div>

`nvs fmt` is a fixed point: reformatting its own output changes nothing, across the whole corpus. That is
what makes `--check` well-defined ([`tooling/fmt-check-writes-nothing`](/docs/rules/tooling/the-formatter/#fmt-check-writes-nothing "nvs fmt rewrites in place; --check writes nothing and exits non-zero naming every file that would change; --diff prints the diff instead")), and it holds for a range as
well as a file — the LSP's range formatting applies the identical rule set to a sub-range, a restriction on
where the rules apply and never a second rule set.

Deterministic here means **byte-stable, not source-independent**, and the difference is the whole cost of
[`tooling/fmt-never-reflows`](/docs/rules/tooling/the-formatter/#fmt-never-reflows "nvs fmt never decides where an expression breaks: the author's own line breaks are kept and only what surrounds them is normalized"). Output is a pure function of the input bytes: the same file formats to
the same result on every machine, forever. It is deliberately not a function of the parsed program. Two
files that differ only in where their author wrapped a call have the same AST and still format to
different bytes, because those line breaks are kept. A formatter whose output depended only on the AST
would have to choose every line break itself, which is exactly the width-fitting printer this design
declines to build. What converges is one file, run twice — never two semantically identical files.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/tooling/the-formatter/#fmt-never-reflows" title="nvs fmt never decides where an expression breaks: the author's own line breaks are kept and only what surrounds them is normalized"><code>tooling/fmt-never-reflows</code></a> <a href="/docs/rules/tooling/the-formatter/#fmt-check-writes-nothing" title="nvs fmt rewrites in place; --check writes nothing and exits non-zero naming every file that would change; --diff prints the diff instead"><code>tooling/fmt-check-writes-nothing</code></a> <a href="/docs/rules/ide/one-server-thin-clients/#one-server-two-thin-clients" title="Language smarts and formatting of Novis have one implementation each, nvs-lsp and nvs-fmt, and an editor client holds none of either"><code>ide/one-server-two-thin-clients</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0039.md">record 0039</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-fmt/tests/corpus.rs"><code>crates/nvs-fmt/tests/corpus.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="fmt-is-never-a-diagnostic">

## `nvs fmt` is a separate opt-in tool: no compiler command runs it, an unformatted file is never a diagnostic, and an editor composes it with quick fixes in the client

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#fmt-is-never-a-diagnostic"><code>tooling/fmt-is-never-a-diagnostic</code></a>
</div>

Nothing wires `nvs fmt` into `nvs check`, `nvs run`, `nvs test` or any other compiler command. An
unformatted file is never a diagnostic — not an error, not a warning — and never blocks compilation or
execution. `nvs fmt` is a separate tool a person or a CI job chooses to run, the boundary `cargo fmt` keeps
from `cargo build`. It is the deliberate counterpart of [`core-api/identifier-casing`](/docs/rules/core-api/naming-and-shape/#identifier-casing "An identifier's casing is fixed by its category and a mismatch is a hard compile error"): casing is a
hard compile error with no suppression, and formatting sits at the opposite end of the same axis, entirely
outside the compiler's diagnostic surface, where a stylistic diagnostic would blur "this program is wrong"
against "this program looks different than I would write it".

An editor may run `nvs fmt` and a set of quick fixes together on one keystroke, and that composition
happens in the client, never inside `nvs fmt`. Format-on-save beside a fix-all-on-save list is how both
target editors are already shaped. Each fix is a diagnostic-backed code action — a mis-ordered
`tainted secret string` ([`security/secret-qualifier`](/docs/rules/security/secrets/#secret-qualifier "secret is a second, independent compile-time qualifier, written before tainted and in that order alone")) is a parse error whose diagnostic already names
the fix — applied against the resilient tree, to a file that may not parse at all.

Keeping the two contracts apart is what `--check` needs: it fails for exactly one reason, so a CI job
never has to tell "laid out differently" from "semantically wrong". A separate `nvs fix` batch verb was
declined for the same reason — a third rule table beside the formatter's and the converter's, which no one
has asked for.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>No <code>php -l</code>-style check ever holds a stylistic opinion, and formatting never lands in the same report as an error</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/tooling/the-formatter/#fmt-check-writes-nothing" title="nvs fmt rewrites in place; --check writes nothing and exits non-zero naming every file that would change; --diff prints the diff instead"><code>tooling/fmt-check-writes-nothing</code></a> <a href="/docs/rules/tooling/the-formatter/#fmt-is-one-canonical-style" title="nvs fmt has one style, takes no configuration, and its output is a pure function of the file it is given"><code>tooling/fmt-is-one-canonical-style</code></a> <a href="/docs/rules/core-api/naming-and-shape/#identifier-casing" title="An identifier's casing is fixed by its category and a mismatch is a hard compile error"><code>core-api/identifier-casing</code></a> <a href="/docs/rules/security/secrets/#secret-qualifier" title="secret is a second, independent compile-time qualifier, written before tainted and in that order alone"><code>security/secret-qualifier</code></a> <a href="/docs/rules/tooling/nvs-convert/#convert-one-table-two-modes" title="nvs convert is one rule table read through two modes, and every branch carries a tier"><code>tooling/convert-one-table-two-modes</code></a> <a href="/docs/rules/ide/the-language-server/#every-feature-is-staged-behind-its-dependency" title="The VS Code client goes as deep as the editor allows, and each feature waits for the language or runtime piece it needs rather than shipping as a stub"><code>ide/every-feature-is-staged-behind-its-dependency</code></a> <a href="/docs/rules/ide/one-server-thin-clients/#one-server-two-thin-clients" title="Language smarts and formatting of Novis have one implementation each, nvs-lsp and nvs-fmt, and an editor client holds none of either"><code>ide/one-server-two-thin-clients</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0039.md">record 0039</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-cli/tests/fmt.rs"><code>crates/nvs-cli/tests/fmt.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="fmt-check-writes-nothing">

## `nvs fmt` rewrites in place; `--check` writes nothing and exits non-zero naming every file that would change; `--diff` prints the diff instead

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#fmt-check-writes-nothing"><code>tooling/fmt-check-writes-nothing</code></a>
</div>

`nvs fmt <path>...` rewrites the named files in place. `nvs fmt --check <path>...` (alias `--dry-run`)
writes nothing: it prints which files would change and exits non-zero if any would, in both cases without
touching disk — the mode a CI job or a pre-commit hook runs, mirroring `rustfmt --check`.
`nvs fmt --diff <path>...` prints a unified diff instead of a bare file list, and `--stdin` formats what
it is given.

These, with the target paths, are the only flags: every one is an I/O mode, and none changes the output
([`tooling/fmt-is-one-canonical-style`](/docs/rules/tooling/the-formatter/#fmt-is-one-canonical-style "nvs fmt has one style, takes no configuration, and its output is a pure function of the file it is given")). `--check` exits `0` on an already-formatted file because the
formatter is a fixed point ([`tooling/fmt-is-idempotent`](/docs/rules/tooling/the-formatter/#fmt-is-idempotent "nvs fmt is a fixed point over the input bytes: byte-stable on every machine, and deliberately not source-independent")); a non-zero exit names the file, and it means
one thing only, since formatting and repair are never one command
([`tooling/fmt-is-never-a-diagnostic`](/docs/rules/tooling/the-formatter/#fmt-is-never-a-diagnostic "nvs fmt is a separate opt-in tool: no compiler command runs it, an unformatted file is never a diagnostic, and an editor composes it with quick fixes in the client")).

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>php-cs-fixer fix --dry-run</code> is the habit; the flag is <code>--check</code> and its exit status is the whole contract</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/tooling/the-formatter/#fmt-is-never-a-diagnostic" title="nvs fmt is a separate opt-in tool: no compiler command runs it, an unformatted file is never a diagnostic, and an editor composes it with quick fixes in the client"><code>tooling/fmt-is-never-a-diagnostic</code></a> <a href="/docs/rules/tooling/the-formatter/#fmt-is-idempotent" title="nvs fmt is a fixed point over the input bytes: byte-stable on every machine, and deliberately not source-independent"><code>tooling/fmt-is-idempotent</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0039.md">record 0039</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-cli/tests/fmt.rs"><code>crates/nvs-cli/tests/fmt.rs</code></a></dd></div></dl>

</div>
