---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Regex, HTML and introspection"
description: "A two-tier regex engine, auto-escaping output with one typed bypass, read-only reflection, and an inert parse tree."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/core-classes/processes-and-files/
  label: "Processes and files"
next:
  link: /docs/rules/core-classes/connecting-to-a-database/
  label: "Connecting to a database"
---

<p class="nv-section-lead">A two-tier regex engine, auto-escaping output with one typed bypass, read-only reflection, and an inert parse tree.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">13</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">11</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">2</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">9</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#regex-two-tiers">A pattern runs on the linear engine unless it cannot, and the backtracking tier's step budget throws when exhausted</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#regex-literal-tiering">A literal pattern is validated and tiered while checking, and a malformed one is a compile error</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#regex-syntax">Regex syntax is PCRE's with no <code>u</code> modifier, and a construct neither engine supports is diagnosed by name</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#html-auto-escape"><code>echo</code> in an HTTP request escapes everything it is given, and <code>Core\Html\Markup</code> is the only raw-write bypass</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#html-escape-answers-markup"><code>Core\Html::escape</code> answers a <code>Markup</code>, so the eager-escape habit stops compiling</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#html-to-source"><code>Core\Html::toSource</code> is the one way out of a <code>Markup</code>, and it takes a written reason</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#html-sanitize"><code>Core\Html::sanitize</code> answers a <code>Markup</code> by rebuilding the document, never by filtering it</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#html-parsing">HTML parses by the WHATWG algorithm onto <code>Core\Xml</code>'s own tree, and that parse never fails</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#validate-has-no-type-predicates"><code>Core\Validate</code> carries no predicate that names a type, because <code>as ?T</code> already is one</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#secret-reveal"><code>Core\Secret::reveal</code> is the one named way out of <code>secret</code>, and it carries a written reason</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#reflect"><code>Core\Reflect</code> is read-only structural introspection, and it is a first-class feature rather than an extension</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#ast-is-inert"><code>Core\Ast</code> runs the compiler's own parser and hands back typed, inert nodes</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#topic"><code>Core\Topic</code> is the only way two connections meet, and a slow subscriber is closed rather than tolerated</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li></ol>

<div class="nv-rule" id="regex-two-tiers">

## A pattern runs on the linear engine unless it cannot, and the backtracking tier's step budget throws when exhausted

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#regex-two-tiers"><code>core-classes/regex-two-tiers</code></a>
</div>

Every pattern compiles to the **linear tier** if the linear engine can express it, and to the
**backtracking tier** otherwise. The choice is made by the engine, never by the developer and never
by a modifier: there is no way to ask for backtracking, only to write a pattern that requires it.

The backtracking tier runs under a bounded step count. Exhausting it throws an ordinary catchable
`Throwable` naming the pattern and the budget. It never returns "no match", never returns a falsy
value, and never truncates the search — a search that stopped early and a search that found nothing
are different facts, and PHP's `preg_*` conflates them into `false`. Per
[`errors/escalation-ladder`](/docs/rules/errors/the-escalation-ladder/#escalation-ladder "A failure escalates through four tiers, and no tier is retried") this is an ordinary throw rather than a resource-limit fatal, so the
request may catch it and answer 400. The linear tier has no budget, because it needs none.

What this costs is that a pattern's performance class is a property of the pattern rather than
something a caller can override. That is the trade taken deliberately: an engine choice a developer
cannot see is the failure mode the whole design exists to avoid.

The budget's default is a stated constant today rather than a configuration key, because there is no
configuration subsystem in front of it yet — `crates/nvs-stdlib/src/regex.rs` names it and records
the gap.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no modifier that asks for backtracking, and <code>preg_*</code>'s silent <code>false</code> on a backtrack limit is a catchable throw here</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/regex-html-and-introspection/#regex-literal-tiering" title="A literal pattern is validated and tiered while checking, and a malformed one is a compile error"><code>core-classes/regex-literal-tiering</code></a> <a href="/docs/rules/core-classes/regex-html-and-introspection/#regex-syntax" title="Regex syntax is PCRE's with no u modifier, and a construct neither engine supports is diagnosed by name"><code>core-classes/regex-syntax</code></a> <a href="/docs/rules/errors/the-escalation-ladder/#escalation-ladder" title="A failure escalates through four tiers, and no tier is retried"><code>errors/escalation-ladder</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0056.md">record 0056</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0005.md">record 0005</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0020.md">record 0020</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/regex-tiers-and-the-backtracking-budget.nvst"><code>tests/conformance/core/regex-tiers-and-the-backtracking-budget.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="regex-literal-tiering">

## A literal pattern is validated and tiered while checking, and a malformed one is a compile error

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#regex-literal-tiering"><code>core-classes/regex-literal-tiering</code></a>
</div>

A literal pattern argument is validated and compiled while checking, under
[`expressions/intrinsic-literals`](/docs/rules/expressions/conversion-and-intrinsics/#intrinsic-literals "A literal argument to an intrinsic Core call is validated while checking and prepared into the artifact"), and the tier it landed in is written down where later stages
read it back. The tier is settleable there because it is a property of the pattern text alone —
decided by which engine's parser refused a construct — so a checking run and a request cannot
disagree about it.

Three consequences follow, none of which costs anything at run time. **A malformed pattern is a
compile error**, not a run-time throw on the first request that reaches it. **The tier is known
statically**, so a check run can report which patterns require backtracking. And **an operator can
refuse them**: `[regex] backtracking = "allow" | "warn" | "deny"` makes a backtracking pattern
respectively silent, a warning, or a compile-time error, so a deployment running untrusted or
high-volume code can know no request can be made to backtrack at all.

A pattern assembled at run time is compiled at run time and gets the same tiering and the same
budget, with none of the three benefits. That is a reason to write patterns as literals, stated here
rather than discovered.

The third consequence is not built: there is no `[regex]` block, so `deny` cannot be written yet.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/regex-html-and-introspection/#regex-two-tiers" title="A pattern runs on the linear engine unless it cannot, and the backtracking tier's step budget throws when exhausted"><code>core-classes/regex-two-tiers</code></a> <a href="/docs/rules/expressions/conversion-and-intrinsics/#intrinsic-literals" title="A literal argument to an intrinsic Core call is validated while checking and prepared into the artifact"><code>expressions/intrinsic-literals</code></a> <a href="/docs/rules/expressions/conversion-and-intrinsics/#intrinsic-list-is-closed" title="The intrinsic roster is a closed set in the compiler's own source, and nothing declares itself into it"><code>expressions/intrinsic-list-is-closed</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0056.md">record 0056</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0057.md">record 0057</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0005.md">record 0005</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/intrinsics.rs"><code>crates/nvs-types/tests/intrinsics.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="regex-syntax">

## Regex syntax is PCRE's with no `u` modifier, and a construct neither engine supports is diagnosed by name

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#regex-syntax"><code>core-classes/regex-syntax</code></a>
</div>

The accepted syntax is PCRE's, across both tiers, with three fixed points.

**There is no `u` modifier.** [`types/string-is-utf8`](/docs/rules/types/text-and-literal-types/#string-is-utf8 "A string is valid UTF-8 for its whole lifetime, and its unmarked unit is the grapheme cluster") guarantees a `string` is UTF-8, so Unicode
mode is not optional and not a flag, and `.` is a code point. Matching over `bytes` is a separate,
explicitly byte-oriented entry point rather than the same members under a switch.

**A construct neither engine supports is a compile-time diagnostic naming it** — recursion,
subroutine calls and callouts among them. It is never silently ignored and never approximated,
because a behaviour difference the developer cannot see is precisely what the two-tier design exists
to avoid.

`/e` and the other spellings PHP has already removed are not reintroduced.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Unicode mode is not a flag because <code>string</code> is always UTF-8, and <code>/e</code> and its relatives are not reintroduced</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/regex-html-and-introspection/#regex-two-tiers" title="A pattern runs on the linear engine unless it cannot, and the backtracking tier's step budget throws when exhausted"><code>core-classes/regex-two-tiers</code></a> <a href="/docs/rules/types/text-and-literal-types/#string-is-utf8" title="A string is valid UTF-8 for its whole lifetime, and its unmarked unit is the grapheme cluster"><code>types/string-is-utf8</code></a> <a href="/docs/rules/types/text-and-literal-types/#bytes" title="bytes is a primitive peer to string for data that carries no encoding"><code>types/bytes</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0056.md">record 0056</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0009.md">record 0009</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/regex-compile-carries-the-four-flags.nvst"><code>tests/conformance/core/regex-compile-carries-the-four-flags.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/regex-match-offset-is-a-grapheme-index-from-zero-to-the-subject-length.nvst"><code>tests/conformance/core/regex-match-offset-is-a-grapheme-index-from-zero-to-the-subject-length.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="html-auto-escape">

## `echo` in an HTTP request escapes everything it is given, and `Core\Html\Markup` is the only raw-write bypass

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#html-auto-escape"><code>core-classes/html-auto-escape</code></a>
</div>

`echo` inside an HTTP request is an escaping sink. It accepts only `Core\Html\Markup`, implies
`Content-Type: text/html`, and auto-escapes any non-`Markup` value interpolated into a
`Markup`-building position, lifting the result. It never distinguishes tainted from untainted,
because escaping neutralizes either one structurally. Every other body shape is a typed response
member — `json`, `text`, `bytes`, `sendFile` — each framing its own content, and mixing `echo` with
one of them on a single response is a compile error.

This is a deliberate exception to the standing rule that nothing happens by position, only by
declaration. Security ranks above simplicity, and an omitted escape call is the single most common
real-world XSS root cause, so the priority is spent explicitly rather than holding the no-magic line
for its own sake. It is one of only two such exceptions.

`Markup` is a small value type, peer to `string` the way `bytes` is. A **source-literal** string
converted with `as Markup` is trusted — it is exactly what the developer wrote. A runtime-computed or
`tainted` string can never become `Markup` that way, which closes the obvious bypass.
`Markup + Markup` is `Markup`, so composing trusted fragments stays cheap; `.` has no row for a
carrier, and a mixed `$markup + "x"` is refused rather than escaped, because `+` is not a sink.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Output is escaped by the sink rather than by a call the developer remembers, and a runtime-computed string can never become <code>Markup</code> through <code>as</code></p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/regex-html-and-introspection/#html-escape-answers-markup" title="Core\Html::escape answers a Markup, so the eager-escape habit stops compiling"><code>core-classes/html-escape-answers-markup</code></a> <a href="/docs/rules/core-classes/regex-html-and-introspection/#html-to-source" title="Core\Html::toSource is the one way out of a Markup, and it takes a written reason"><code>core-classes/html-to-source</code></a> <a href="/docs/rules/core-classes/regex-html-and-introspection/#html-sanitize" title="Core\Html::sanitize answers a Markup by rebuilding the document, never by filtering it"><code>core-classes/html-sanitize</code></a> <a href="/docs/rules/errors/ambiguous-input/#ambiguous-input-refused" title="Ambiguous input is refused whole, never repaired"><code>errors/ambiguous-input-refused</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0133.md">record 0133</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0087.md">record 0087</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0088.md">record 0088</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0086.md">record 0086</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/markup-lifts-a-literal-and-writes-what-escape-would-not.nvst"><code>tests/conformance/core/markup-lifts-a-literal-and-writes-what-escape-would-not.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/markup-composes-with-plus-and-stays-markup.nvst"><code>tests/conformance/core/markup-composes-with-plus-and-stays-markup.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/markup-refuses-a-computed-operand-and-a-half-markup-sum.nvst"><code>tests/conformance/core/markup-refuses-a-computed-operand-and-a-half-markup-sum.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="html-escape-answers-markup">

## `Core\Html::escape` answers a `Markup`, so the eager-escape habit stops compiling

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#html-escape-answers-markup"><code>core-classes/html-escape-answers-markup</code></a>
</div>

`Core\Html::escape(tainted string $text): Core\Html\Markup` answers the carrier its sink accepts, not
a `string`. The bytes it produces are unchanged; only the wrapper is new.

**The eager-escape habit stops compiling**, which is the point. A developer arriving from
`htmlspecialchars()` writes the escape call, then finds the result cannot be concatenated back into
the surrounding string, because `.` has no row for a carrier. They are corrected at the call site
instead of shipping `&amp;amp;`, and for most code the correction is to delete the escape call
entirely — the sink was always going to do it ([`core-classes/html-auto-escape`](/docs/rules/core-classes/regex-html-and-introspection/#html-auto-escape "echo in an HTTP request escapes everything it is given, and Core\Html\Markup is the only raw-write bypass")).

`escape` additionally **neutralizes an unterminated bidirectional control**, substituting a
replacement character. Escaping `<`, `>`, `&` and quotes does nothing about display order, so without
that rule a bidi payload would survive the auto-escape sink intact. A balanced control is legitimate
mixed-direction text and passes through.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>htmlspecialchars</code>'s replacement does not answer a <code>string</code>, so its result cannot be concatenated back into one and double-escaped</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/regex-html-and-introspection/#html-auto-escape" title="echo in an HTTP request escapes everything it is given, and Core\Html\Markup is the only raw-write bypass"><code>core-classes/html-auto-escape</code></a> <a href="/docs/rules/core-classes/regex-html-and-introspection/#html-to-source" title="Core\Html::toSource is the one way out of a Markup, and it takes a written reason"><code>core-classes/html-to-source</code></a> <a href="/docs/rules/core-classes/regex-html-and-introspection/#html-sanitize" title="Core\Html::sanitize answers a Markup by rebuilding the document, never by filtering it"><code>core-classes/html-sanitize</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0133.md">record 0133</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0087.md">record 0087</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/html-escape-launders-a-tainted-value-and-writes-all-five-characters.nvst"><code>tests/conformance/core/html-escape-launders-a-tainted-value-and-writes-all-five-characters.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/html-escape-changes-nothing-else-and-escapes-what-already-reads-as-a-reference.nvst"><code>tests/conformance/core/html-escape-changes-nothing-else-and-escapes-what-already-reads-as-a-reference.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/html-escape-neutralizes-an-unterminated-bidi-control-and-passes-a-balanced-one.nvst"><code>tests/conformance/core/html-escape-neutralizes-an-unterminated-bidi-control-and-passes-a-balanced-one.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="html-to-source">

## `Core\Html::toSource` is the one way out of a `Markup`, and it takes a written reason

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#html-to-source"><code>core-classes/html-to-source</code></a>
</div>

`Core\Html::toSource(Core\Html\Markup $markup, string $reason): string` hands back the markup's
source text, and it is the only way out. There is **no `Markup as string` conversion**, because one
would reopen the hole in a keystroke: an escape whose result is immediately cast back to `string` and
concatenated with tainted text is the original bug with an extra word in it.

The shape is the project's standing escape-hatch form — rare, greppable, and carrying a written
reason at the site rather than a silent cast, the same shape [`core-classes/secret-reveal`](/docs/rules/core-classes/regex-html-and-introspection/#secret-reveal "Core\Secret::reveal is the one named way out of secret, and it carries a written reason") takes.
`$reason` is a source literal and an empty one is refused: a reason that can be computed is a reason
nobody wrote.

Its legitimate callers are the ones that need the bytes and not the guarantee — caching a rendered
fragment, storing one in a column, writing one to a file, handing one to a sink that is not this one.

The name is deliberate. `to…` is the conversion verb, `source` is spelled out, and it is neither
`raw` — which in every template language means the opposite direction — nor `unescape`, which is
reserved for the operation that actually inverts `escape` and which this is not.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/regex-html-and-introspection/#html-escape-answers-markup" title="Core\Html::escape answers a Markup, so the eager-escape habit stops compiling"><code>core-classes/html-escape-answers-markup</code></a> <a href="/docs/rules/core-classes/regex-html-and-introspection/#html-auto-escape" title="echo in an HTTP request escapes everything it is given, and Core\Html\Markup is the only raw-write bypass"><code>core-classes/html-auto-escape</code></a> <a href="/docs/rules/core-classes/regex-html-and-introspection/#secret-reveal" title="Core\Secret::reveal is the one named way out of secret, and it carries a written reason"><code>core-classes/secret-reveal</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0133.md">record 0133</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0063.md">record 0063</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/to-source-hands-back-the-bytes-of-every-way-a-markup-is-obtained.nvst"><code>tests/conformance/core/to-source-hands-back-the-bytes-of-every-way-a-markup-is-obtained.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/to-source-refuses-an-empty-reason-and-takes-any-written-one.nvst"><code>tests/conformance/core/to-source-refuses-an-empty-reason-and-takes-any-written-one.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="html-sanitize">

## `Core\Html::sanitize` answers a `Markup` by rebuilding the document, never by filtering it

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#html-sanitize"><code>core-classes/html-sanitize</code></a>
</div>

`Core\Html::sanitize` answers `Core\Html\Markup`, which makes it the fourth way to obtain one and the
only one that takes a runtime-computed string.

That is exactly why it must be a parser that **rebuilds the document from a known-good grammar**, and
never a filter that deletes what looks dangerous. A filter answering a carrier would be a generic
sanitizer wearing a type — it would claim a guarantee it cannot establish, because "what looks
dangerous" is a list an attacker gets to extend.

**Not shipped.** `crates/nvs-stdlib/src/html.rs` carries `escape` and `toSource` and no sanitizer.
The member waits on the WHATWG tree ([`core-classes/html-parsing`](/docs/rules/core-classes/regex-html-and-introspection/#html-parsing "HTML parses by the WHATWG algorithm onto Core\Xml's own tree, and that parse never fails")), which is what it would parse
into.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/regex-html-and-introspection/#html-escape-answers-markup" title="Core\Html::escape answers a Markup, so the eager-escape habit stops compiling"><code>core-classes/html-escape-answers-markup</code></a> <a href="/docs/rules/core-classes/regex-html-and-introspection/#html-parsing" title="HTML parses by the WHATWG algorithm onto Core\Xml's own tree, and that parse never fails"><code>core-classes/html-parsing</code></a> <a href="/docs/rules/core-classes/regex-html-and-introspection/#html-auto-escape" title="echo in an HTTP request escapes everything it is given, and Core\Html\Markup is the only raw-write bypass"><code>core-classes/html-auto-escape</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0133.md">record 0133</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0051.md">record 0051</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0122.md">record 0122</a></dd></div></dl>

</div>

<div class="nv-rule" id="html-parsing">

## HTML parses by the WHATWG algorithm onto `Core\Xml`'s own tree, and that parse never fails

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#html-parsing"><code>core-classes/html-parsing</code></a>
</div>

HTML parses through `Core\Html`, by the WHATWG parsing algorithm, and the parse **never fails**:
implied tags, error recovery and foster parenting are the specified output every conforming parser
produces, so recovering here does not violate the refuse-never-repair rule — nothing is guessed,
because the specification fixes the answer. `Core\Xml` keeps the opposite contract: malformed XML
throws. One API flipping between refuse-hard and recover-always under a flag is the ambiguity being
retired, and it is what PHP's libxml2 surface is.

Both parsers materialise **the same node family**. Queries, traversal and the tree's memory story are
written once, and which door parsed a document does not change what a program can do with it.
Serialization follows the door: WHATWG rules through one, XML rules through the other. The engine is
`html5ever` driving a tree builder we own, so it builds request-attributed nodes directly rather than
through its sample DOM, and the same crate is compiled into the PDF component — the language and its
PDF renderer parse HTML identically: one behaviour to document, one parser to fuzz.

What it spends, per parse: the materialised tree, proportional to the document, attributed to the
request and gone with it.

**Not shipped**, and neither half may land alone: the milestone that schedules `Core\Xml`'s tree API
builds this parse in the same milestone, because the tree and the builder interface are one
implementation and the second one built would otherwise be shaped by whichever landed first.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no <code>LIBXML_*</code> flag matrix and no document class that flips between recovering and throwing — the two front doors are two classes</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/regex-html-and-introspection/#html-sanitize" title="Core\Html::sanitize answers a Markup by rebuilding the document, never by filtering it"><code>core-classes/html-sanitize</code></a> <a href="/docs/rules/core-classes/pdf-and-spreadsheets/#pdf-one-engine" title="The only input language is HTML plus a CSS subset, and every backend answers that one interface"><code>core-classes/pdf-one-engine</code></a> <a href="/docs/rules/errors/ambiguous-input/#ambiguous-input-refused" title="Ambiguous input is refused whole, never repaired"><code>errors/ambiguous-input-refused</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0122.md">record 0122</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0095.md">record 0095</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0063.md">record 0063</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0121.md">record 0121</a></dd></div></dl>

</div>

<div class="nv-rule" id="validate-has-no-type-predicates">

## `Core\Validate` carries no predicate that names a type, because `as ?T` already is one

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#validate-has-no-type-predicates"><code>core-classes/validate-has-no-type-predicates</code></a>
</div>

`Core\Validate` carries no predicate whose job is to ask whether a string names a value of some type.
`Validate::isInteger($s)` and `$s as ?int != null` are the same predicate, and one operation gets one
spelling — so `isInteger`, `isFloat` and `isBoolean` do not exist, and neither does a `ctype_digit`
equivalent, which is `$s as ?uint != null`.

One implementation now exists because there is one operation, not because two were required to agree.
The conversion table is the definition of what parses; a second table maintained beside it is a
second table to drift.

What survives is the roster that names no type: `isEmail`, `isIp`, `isMac`, `isDomain`, `isAscii` and
`isPrintable`. None has an `as` equivalent, because none names a type
([`expressions/nullable-conversion`](/docs/rules/expressions/conversion-and-intrinsics/#nullable-conversion "expr as ?T yields the converted value or null, and never throws")).

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>is_numeric</code>, <code>ctype_digit</code>, <code>filter_var(FILTER_VALIDATE_INT)</code> and their kind have no replacement — the conversion is the test</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/expressions/conversion-and-intrinsics/#nullable-conversion" title="expr as ?T yields the converted value or null, and never throws"><code>expressions/nullable-conversion</code></a> <a href="/docs/rules/expressions/conversion-and-intrinsics/#try-parse" title="A failable single-string parse is spelled tryParse, and no separate validity predicate stands beside it"><code>expressions/try-parse</code></a> <a href="/docs/rules/types/unions-and-conversion/#conversion" title="expr as T is the only conversion, and it produces a T or throws"><code>types/conversion</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0066.md">record 0066</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0063.md">record 0063</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/validate-members.nvst"><code>tests/conformance/core/validate-members.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="secret-reveal">

## `Core\Secret::reveal` is the one named way out of `secret`, and it carries a written reason

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#secret-reveal"><code>core-classes/secret-reveal</code></a>
</div>

`Core\Secret::reveal(secret string $value, string $reason): string`, with a `bytes` overload, is the
one narrow way out of the `secret` qualifier outside a checked conversion. It is forbidden by
default, rare, greppable, and carries a written reason at the call site, and the reason reaches no
byte of the answer.

There is deliberately no generic `unwrap()` or `expose()`. A catch-all invites false confidence, and
the whole value of a qualifier is that removing it is visible where it happens.

A second, more common removal path is a **purpose-built function that consumes a `secret` and returns
a genuinely non-secret derivative** — password hashing is the canonical case: it takes a
`secret string` and its output is not confidential in the same way, so it may declare a plain
`string` return. That is not a loophole; it is the ordinary shape of "the secret goes in, something
safe to keep comes out", and each such function's author carries responsibility for it being true.

`reveal` removes `secret` and nothing else: a value that was also `tainted` stays `tainted`.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no generic unwrap; a confidential value leaves its qualifier only through this member or through a function that consumes it and returns something else</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/regex-html-and-introspection/#html-to-source" title="Core\Html::toSource is the one way out of a Markup, and it takes a written reason"><code>core-classes/html-to-source</code></a> <a href="/docs/rules/core-classes/processes-and-files/#process-options" title="ProcessOptions carries a working directory, a replaced environment and a timeout, and nothing else"><code>core-classes/process-options</code></a> <a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#ratelimit-gcra" title="Both tiers run GCRA over one stored timestamp per key, and the Decision computes retryAfter"><code>core-classes/ratelimit-gcra</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0033.md">record 0033</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/reveal-removes-secret-and-not-tainted.nvst"><code>tests/conformance/reject/reveal-removes-secret-and-not-tainted.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-reveal-drops-the-secret-qualifier.nvst"><code>tests/conformance/core/a-reveal-drops-the-secret-qualifier.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-revealed-value-is-accepted-by-every-secret-sink.nvst"><code>tests/conformance/core/a-revealed-value-is-accepted-by-every-secret-sink.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/secret-the-reason-reaches-no-byte-of-the-answer.nvst"><code>tests/conformance/core/secret-the-reason-reaches-no-byte-of-the-answer.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/secret-reveal-bytes-is-the-identity-over-every-octet.nvst"><code>tests/conformance/core/secret-reveal-bytes-is-the-identity-over-every-octet.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/secret-both-members-agree-across-the-string-bytes-boundary.nvst"><code>tests/conformance/core/secret-both-members-agree-across-the-string-bytes-boundary.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="reflect">

## `Core\Reflect` is read-only structural introspection, and it is a first-class feature rather than an extension

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#reflect"><code>core-classes/reflect</code></a>
</div>

`Core\Reflect` is read-only structural introspection, and it is a first-class feature rather than an
extension a deployment might not have compiled in. It reaches classes, interfaces, enums, methods,
properties, constants, parameters and attributes, from a value or from a class name, and it reports
which methods an interface declares as part of its contract versus as an internal helper. Traits are
absent because they do not exist.

Reflective access **enforces the same checks ordinary code would**: there is no
`setAccessible(true)`, so a private property is not readable through this door either, and a
reflective write runs the property observer an ordinary write would run. The refusal is
distinguishable from a misspelling, which is what makes the answer useful rather than merely safe.

What it costs is that a serializer or a container cannot reach state its author did not expose. That
is the trade: the alternative is that every access modifier in the language is advisory, which is
what PHP's reflection makes them.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Reflection is always present rather than a compiled-in extension, it covers no traits because there are none, and it never widens access</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/regex-html-and-introspection/#ast-is-inert" title="Core\Ast runs the compiler's own parser and hands back typed, inert nodes"><code>core-classes/ast-is-inert</code></a> <a href="/docs/rules/enums/#reflection" title="Reflection describes an enum's shape and grants it nothing a class has"><code>enums/reflection</code></a> <a href="/docs/rules/types/objects-and-shapes/#erased-member-access" title="A member reached through an erased receiver is answered at run time, and a failure is a throw"><code>types/erased-member-access</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0019.md">record 0019</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0043.md">record 0043</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/reflect-forclass-reaches-a-description-by-name-and-absence-is-null.nvst"><code>tests/conformance/core/reflect-forclass-reaches-a-description-by-name-and-absence-is-null.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/reflect-describes-a-class-by-the-properties-visible-from-outside.nvst"><code>tests/conformance/core/reflect-describes-a-class-by-the-properties-visible-from-outside.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/reflect-typeof-follows-the-value-and-forclass-follows-the-name.nvst"><code>tests/conformance/core/reflect-typeof-follows-the-value-and-forclass-follows-the-name.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/reflect-reads-a-promoted-parameters-visibility-from-its-keyword.nvst"><code>tests/conformance/core/reflect-reads-a-promoted-parameters-visibility-from-its-keyword.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="ast-is-inert">

## `Core\Ast` runs the compiler's own parser and hands back typed, inert nodes

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#ast-is-inert"><code>core-classes/ast-is-inert</code></a>
</div>

`Core\Ast::parse` and `::parseFile` call directly into the same lexer and parser the compiler itself
runs, so a construct that parses when a file is compiled parses identically when a running program
parses the same text, and a rejected construct is rejected identically in both places. There is no
second grammar implementation anywhere in the project.

The return value is a **typed** node tree — one type per production — never an untyped array or a
stringly-keyed structure. Handing back the parse tree as untyped data would be exactly the shortcut
`token_get_all()` takes, reintroduced at the one place a fully-typed alternative is easiest to give.

**A parsed tree is inert. There is no path from an AST value back into execution.** `eval` does not
exist and stays rejected: a string has no stable identity, no cache key, and no capability-grantable
path. A program can walk a tree, print it, or rewrite it into a new source string to hand to a human
or a file — never a way to run what it describes. Shipping this is therefore not `eval` under a
different name; it is the same refusal restated.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>token_get_all</code>'s untyped arrays are replaced by one type per production, and there is no <code>eval</code> for a rewritten tree to reach</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/regex-html-and-introspection/#reflect" title="Core\Reflect is read-only structural introspection, and it is a first-class feature rather than an extension"><code>core-classes/reflect</code></a> <a href="/docs/rules/types/declarations-and-numbers/#declaration" title="Every binding declares its type, and no binding's type ever changes"><code>types/declaration</code></a> <a href="/docs/rules/programs/names-and-files/#compile-target" title="A compile target changes the host context, never the language"><code>programs/compile-target</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0019.md">record 0019</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0006.md">record 0006</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/core-ast-parse-answers-the-compilers-own-tree.nvst"><code>tests/conformance/core/core-ast-parse-answers-the-compilers-own-tree.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/core-ast-parse-stops-where-the-compiler-stops.nvst"><code>tests/conformance/core/core-ast-parse-stops-where-the-compiler-stops.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/core-ast-node-walk-is-a-whole-tree-asserted-by-counting.nvst"><code>tests/conformance/core/core-ast-node-walk-is-a-whole-tree-asserted-by-counting.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/core-ast-nodes-is-children-closed-transitively.nvst"><code>tests/conformance/core/core-ast-nodes-is-children-closed-transitively.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="topic">

## `Core\Topic` is the only way two connections meet, and a slow subscriber is closed rather than tolerated

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#topic"><code>core-classes/topic</code></a>
</div>

`Core\Topic` is the only way two persistent connections meet: `subscribe`, `publish` and
`unsubscribe`, runtime-owned, in-process, and reaching across every core. This is the one place the
thread-per-core design is crossed on purpose, and it is a bounded message hand-off rather than shared
state. A published value is graph-copied, so subscribers share nothing with the publisher or with
each other.

**A slow subscriber is closed, never tolerated.** Each subscriber has a bounded queue, drained by its
own connection and by nothing else; on overflow *that subscriber's connection* is closed with a
defined code and a metric increments. The publisher is never blocked and no queue grows without
bound — a fan-out to ten thousand clients must not become a way for one of them to stall the other
nine thousand nine hundred and ninety-nine.

A topic name **refuses `tainted`**, for the reason a metric label does: a name derived from user input
is how one tenant subscribes to another's stream. All three members refuse it in the same words, and
before the connection is consulted. A `secret` may never be published.

It is **not** built on the shared cache, which is deliberately lossy — right for a cache and wrong
for a message a subscriber is waiting on. Cross-machine fan-out is not the runtime's: a fleet bridges
topics to a broker in application code.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#ratelimit-two-members" title="consume and shed are two jobs with two verbs, and neither is a tier of the other"><code>core-classes/ratelimit-two-members</code></a> <a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#queue-storage-is-a-table" title="The job queue is two tables in a connection the operator names, converged by an explicit command"><code>core-classes/queue-storage-is-a-table</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0083.md">record 0083</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0023.md">record 0023</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0033.md">record 0033</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0059.md">record 0059</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0051.md">record 0051</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0076.md">record 0076</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-topic-name-is-a-sink-and-refuses-one-that-came-from-outside.nvst"><code>tests/conformance/core/a-topic-name-is-a-sink-and-refuses-one-that-came-from-outside.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-topic-needs-a-connection-to-subscribe-on.nvst"><code>tests/conformance/core/a-topic-needs-a-connection-to-subscribe-on.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-topic-name-is-refused-in-the-same-words-by-all-three-members.nvst"><code>tests/conformance/core/a-topic-name-is-refused-in-the-same-words-by-all-three-members.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-topic-name-is-refused-before-the-connection-is-asked-about.nvst"><code>tests/conformance/core/a-topic-name-is-refused-before-the-connection-is-asked-about.nvst</code></a></dd></div></dl>

</div>
