---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Laundering"
description: "The only ways out of tainted: a member named for the one sink it is safe for, a typed capture, or a written assertion."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/security/tainted-data/
  label: "Tainted data and its sinks"
next:
  link: /docs/rules/security/secrets/
  label: "Secrets"
---

<p class="nv-section-lead">The only ways out of <code>tainted</code>: a member named for the one sink it is safe for, a typed capture, or a written assertion.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">8</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">8</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">0</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">2</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#launderers-are-sink-named">The only way out of <code>tainted</code> is a <code>Core</code> member whose contract names the one sink it is safe for</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#launderer-answers-a-carrier">A launderer answers its sink's carrier when that sink launders on its own and the transform is not idempotent, and a plain type otherwise</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#capture-answers-the-carrier">Capturing a sink yields that sink's carrier, never a plain <code>string</code></a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#route-capture-is-laundered-by-its-type">A route capture is laundered by the parameter's own type, and a <code>string</code> capture stays tainted</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#derived-codec-qualifiers">A derived codec adds no qualifier rule, and a <code>secret</code> field is refused at the declaration rather than silently omitted</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#assert-trusted"><code>Core\Taint::assertTrusted</code> is the one generic way out, and it carries a written reason at the call site</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#response-body-is-one-typed-member">A response body is written by one typed member, and mixing <code>echo</code> with one of them does not compile</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#db-pool-reset-is-a-boundary">A pooled database connection is proven clean before it is reused, and a failed reset destroys it</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li></ol>

<div class="nv-rule" id="launderers-are-sink-named">

## The only way out of `tainted` is a `Core` member whose contract names the one sink it is safe for

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#launderers-are-sink-named"><code>security/launderers-are-sink-named</code></a>
</div>

Short of a checked conversion, the only way to remove `tainted` is a `Core` member whose contract
states which **one** sink it is safe for: an HTML escape for HTML text, an identifier quote for a
dynamic table or column name, a path containment check for a path component, and one per sink as each
class is designed.

There is deliberately no generic `sanitize()` or `clean()`. A value safe for HTML text is not safe for
a shell argument or a path, and a single catch-all invites exactly the false confidence the qualifier
exists to prevent. The roster grows by adding a named member to the class that owns the sink, never by
widening an existing one.

Which return type a launderer takes is a predicate rather than a per-member choice
([`security/launderer-answers-a-carrier`](/docs/rules/security/laundering/#launderer-answers-a-carrier "A launderer answers its sink's carrier when that sink launders on its own and the transform is not idempotent, and a plain type otherwise")). Where no built-in launderer fits, the way out is
[`security/assert-trusted`](/docs/rules/security/laundering/#assert-trusted "Core\Taint::assertTrusted is the one generic way out, and it carries a written reason at the call site") — written, greppable, and carrying a reason — and never a silent cast.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no <code>sanitize()</code>, no <code>clean()</code> and no <code>filter_var</code> catch-all; a value safe for HTML is not safe for a path</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/laundering/#assert-trusted" title="Core\Taint::assertTrusted is the one generic way out, and it carries a written reason at the call site"><code>security/assert-trusted</code></a> <a href="/docs/rules/security/laundering/#launderer-answers-a-carrier" title="A launderer answers its sink's carrier when that sink launders on its own and the transform is not idempotent, and a plain type otherwise"><code>security/launderer-answers-a-carrier</code></a> <a href="/docs/rules/security/tainted-data/#taint-propagation" title="A tainted operand poisons every operation it takes part in, and only a checked conversion launders for free"><code>security/taint-propagation</code></a> <a href="/docs/rules/core-classes/regex-html-and-introspection/#html-auto-escape" title="echo in an HTTP request escapes everything it is given, and Core\Html\Markup is the only raw-write bypass"><code>core-classes/html-auto-escape</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0133.md">record 0133</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0055.md">record 0055</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-launder-row-hands-a-tainted-value-to-the-sink-it-names.nvst"><code>tests/conformance/core/a-launder-row-hands-a-tainted-value-to-the-sink-it-names.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/html-escape-launders-a-tainted-value-and-writes-all-five-characters.nvst"><code>tests/conformance/core/html-escape-launders-a-tainted-value-and-writes-all-five-characters.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="launderer-answers-a-carrier">

## A launderer answers its sink's carrier when that sink launders on its own and the transform is not idempotent, and a plain type otherwise

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#launderer-answers-a-carrier"><code>security/launderer-answers-a-carrier</code></a>
</div>

A `Core` member that removes `tainted` answers its sink's **carrier type** rather than the plain type
when **both** hold: its sink launders automatically — a value reaches that sink and is transformed
with no call written at the site — and its transform is **not idempotent**, so applying it to its own
output changes the output. Otherwise it answers the plain type.

The two conditions are one question asked twice: *a second application the source does not show, of a
transform a second application changes.* Only HTML output meets both today, which is why the HTML
escape answers a carrier and every other launderer on the roster — identifier quoting, URI component
and form-value encoding, regex quoting, terminal escaping — answers a `string`, by the predicate
rather than by exemption. Their results are legitimately concatenated into a larger string, and a
carrier would force a builder API onto four classes to close a hazard whose second call is already
visible in the source.

A launderer written for a *new* sink is measured against the two conditions, not against today's
table, and a sink that acquires an automatic launder reclassifies its own member the day it does.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>htmlspecialchars($x)</code> in a string-building position does not compile, because escaping twice is a corruption the type can refuse</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/laundering/#launderers-are-sink-named" title="The only way out of tainted is a Core member whose contract names the one sink it is safe for"><code>security/launderers-are-sink-named</code></a> <a href="/docs/rules/security/laundering/#capture-answers-the-carrier" title="Capturing a sink yields that sink's carrier, never a plain string"><code>security/capture-answers-the-carrier</code></a> <a href="/docs/rules/security/tainted-data/#sink-predicate" title="A string or bytes parameter is a sink when its content becomes an instruction a parser executes"><code>security/sink-predicate</code></a> <a href="/docs/rules/core-classes/regex-html-and-introspection/#html-escape-answers-markup" title="Core\Html::escape answers a Markup, so the eager-escape habit stops compiling"><code>core-classes/html-escape-answers-markup</code></a> <a href="/docs/rules/core-classes/regex-html-and-introspection/#html-to-source" title="Core\Html::toSource is the one way out of a Markup, and it takes a written reason"><code>core-classes/html-to-source</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0133.md">record 0133</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0086.md">record 0086</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0056.md">record 0056</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/an-escaped-value-is-a-carrier-and-cannot-be-escaped-a-second-time.nvst"><code>tests/conformance/core/an-escaped-value-is-a-carrier-and-cannot-be-escaped-a-second-time.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/to-source-hands-back-the-bytes-of-every-way-a-markup-is-obtained.nvst"><code>tests/conformance/core/to-source-hands-back-the-bytes-of-every-way-a-markup-is-obtained.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/markup-refuses-a-computed-operand-and-a-half-markup-sum.nvst"><code>tests/conformance/core/markup-refuses-a-computed-operand-and-a-half-markup-sum.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="capture-answers-the-carrier">

## Capturing a sink yields that sink's carrier, never a plain `string`

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#capture-answers-the-carrier"><code>security/capture-answers-the-carrier</code></a>
</div>

Capturing output answers **the carrier of the sink in force**, not a plain `string`. The reason is not
symmetry: the captured bytes have *already* been through the sink, so handing them back as a `string`
and re-emitting them would escape them a second time and corrupt the page.

The same rule gives a spawned isolate's captured result its type
([`security/isolate-output-is-captured`](/docs/rules/security/isolates/#isolate-output-is-captured "An isolate's output is captured by default and carries the parent's sink carrier")), by the same argument and with the same fix. Inheriting
rather than capturing needs no rule at all: the child's carrier is the parent's, so appending composes
the way carrier addition already does.

This is the third instance of one rule rather than three members that each argued it alone — the other
two being a laundering escape and an approved URL — and stating it once is what keeps a future
capturing member from answering the wrong type ([`security/launderer-answers-a-carrier`](/docs/rules/security/laundering/#launderer-answers-a-carrier "A launderer answers its sink's carrier when that sink launders on its own and the transform is not idempotent, and a plain type otherwise")).

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/isolates/#isolate-output-is-captured" title="An isolate's output is captured by default and carries the parent's sink carrier"><code>security/isolate-output-is-captured</code></a> <a href="/docs/rules/security/laundering/#launderer-answers-a-carrier" title="A launderer answers its sink's carrier when that sink launders on its own and the transform is not idempotent, and a plain type otherwise"><code>security/launderer-answers-a-carrier</code></a> <a href="/docs/rules/security/laundering/#response-body-is-one-typed-member" title="A response body is written by one typed member, and mixing echo with one of them does not compile"><code>security/response-body-is-one-typed-member</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0088.md">record 0088</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0006.md">record 0006</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0133.md">record 0133</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/out-capture-answers-the-sinks-carrier.nvst"><code>tests/conformance/core/out-capture-answers-the-sinks-carrier.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/out-capture-refuses-a-through-that-answers-anything-but-the-carrier.nvst"><code>tests/conformance/core/out-capture-refuses-a-through-that-answers-anything-but-the-carrier.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="route-capture-is-laundered-by-its-type">

## A route capture is laundered by the parameter's own type, and a `string` capture stays tainted

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#route-capture-is-laundered-by-its-type"><code>security/route-capture-is-laundered-by-its-type</code></a>
</div>

Every capture in a route path corresponds to a parameter of the same name, and the parameter's
declared type is what the segment is converted to during matching. A failed conversion is **not a
match**: matching continues, and if nothing else matches the result is a 404 — which is what every
framework otherwise writes by hand as a digit constraint on the placeholder.

**A capture typed at a class built from text is the one exception, and it fails the other way.** The
matcher narrows on the conversions it reads natively — `int`, `uint`, `decimal`, `Core\Uuid` and a
closed set — and a capture declared at any *other* class implementing `Parses` matches on **shape**.
Matching runs at the door with no program installed, so calling that class's `parse` there would put an
implementor's body over every request URL, including the ones that match no route, ahead of everything
that rate-limits it — the priority-1 objection this rule already makes to a regex, and a `parse` body is
strictly more than a regex. The class's `parse` runs at the binding site instead, where the match
crosses into the program, so a segment it refuses is a **`400`** over a route that did match rather than
a `404` ([`routing/a-bad-query-value-is-a-400`](/docs/rules/routing/declaring-a-route/#a-bad-query-value-is-a-400 "A query value that fails to convert is a 400, where a path capture that fails is a 404")). Two captures spelled the same way therefore fail
two ways, and what decides which is who runs at the door.

**A converted capture arrives unqualified.** A checked conversion launders
([`security/taint-propagation`](/docs/rules/security/tainted-data/#taint-propagation "A tainted operand poisons every operation it takes part in, and only a checked conversion launders for free")), so an integer or enum capture is a plain value, while a `string`
capture and a trailing-segment capture stay `tainted string`, because nothing about them was checked.
A capture at a class built from text arrives as an instance of that class, which carries no qualifier
because `tainted` is a property of `string` and `bytes` and never of a class
([`security/tainted-qualifier`](/docs/rules/security/tainted-data/#tainted-qualifier "tainted is a compile-time qualifier on string, bytes and a shape of them, spellable in any declaration and erased before codegen")) — an implementor that keeps the text in a plain `string` field is
refused by the assignability rule that already stands. No new sink, no new launderer, no new rule — the
existing one arriving somewhere useful.

**A regex constraint is not among the admitted types and never will be.** An application-authored
pattern over the request path runs before any rate limiting, which makes catastrophic backtracking an
unauthenticated denial of service; a closed set is spelled as a union of literal types or a subset of
an enum's cases instead.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/tainted-data/#taint-propagation" title="A tainted operand poisons every operation it takes part in, and only a checked conversion launders for free"><code>security/taint-propagation</code></a> <a href="/docs/rules/security/protocols-and-tokens/#access-is-checked-for-presence-not-meaning" title="The compiler proves a route's access decision was written, never that it was honoured"><code>security/access-is-checked-for-presence-not-meaning</code></a> <a href="/docs/rules/types/text-and-literal-types/#literal-types" title="A string or int literal is its own type, and a union of them is a closed set"><code>types/literal-types</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0077.md">record 0077</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0102.md">record 0102</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0160.md">record 0160</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-route-capture-is-the-value-the-match-converted-not-the-segment-text.nvst"><code>tests/conformance/core/a-route-capture-is-the-value-the-match-converted-not-the-segment-text.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/router-match-converts-a-capture-before-anything-decodes-it.nvst"><code>tests/conformance/core/router-match-converts-a-capture-before-anything-decodes-it.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/routes.rs"><code>crates/nvs-types/tests/routes.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="derived-codec-qualifiers">

## A derived codec adds no qualifier rule, and a `secret` field is refused at the declaration rather than silently omitted

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#derived-codec-qualifiers"><code>security/derived-codec-qualifiers</code></a>
</div>

A derived codec introduces no qualifier rule of its own. A decoder assigns into declared property
types, so a payload that carries `tainted` requires the fields receiving it to declare it, and the
check happens where the qualifier is statically known — the **call site** that decodes, not inside the
codec. Only `string` and `bytes` carry a qualifier, so an integer, a decimal, an enum or an instant
field needs nothing.

A `secret` property on a class carrying a derive attribute is a **compile error at the declaration**,
with skipping the field as the stated fix. This adds no sink — encoding was already one
([`security/secret-sinks-refuse`](/docs/rules/security/secrets/#secret-sinks-refuse "Every output sink refuses a secret value outright, and no auto-escape or neutralisation bypasses one")) — it moves the report from wherever the value happened to reach
the encoder to the declaration that put it on the wire contract, and it replaces a silent omission
with a written one.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/secrets/#secret-sinks-refuse" title="Every output sink refuses a secret value outright, and no auto-escape or neutralisation bypasses one"><code>security/secret-sinks-refuse</code></a> <a href="/docs/rules/security/tainted-data/#taint-propagation" title="A tainted operand poisons every operation it takes part in, and only a checked conversion launders for free"><code>security/taint-propagation</code></a> <a href="/docs/rules/security/tainted-data/#tainted-sources" title="Every accessor that hands a program bytes from outside answers the tainted form, and the list of them is enumerable"><code>security/tainted-sources</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0071.md">record 0071</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0033.md">record 0033</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-json-derive-refuses-a-secret-or-lateinit-field.nvst"><code>tests/conformance/reject/a-json-derive-refuses-a-secret-or-lateinit-field.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/derive.rs"><code>crates/nvs-types/tests/derive.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="assert-trusted">

## `Core\Taint::assertTrusted` is the one generic way out, and it carries a written reason at the call site

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#assert-trusted"><code>security/assert-trusted</code></a>
</div>

`Core\Taint::assertTrusted(tainted string, string $reason): string` is the one generic escape from the
qualifier, for the case where the developer has validated the value themselves and needs to say so. It
is modelled on this project's own `unsafe` policy: forbidden by default, rare, greppable, and carrying
a written reason at the call site rather than a silent cast.

It is the answer at every position that has no launderer *and cannot have one* — a metric label, whose
hazard is unbounded cardinality rather than content ([`security/metric-label-refuses-tainted`](/docs/rules/security/tainted-data/#metric-label-refuses-tainted "A metric label value refuses tainted, and there is deliberately no launderer for one")); a
regex pattern, where no transform makes an attacker-authored pattern safe
([`security/regex-pattern-is-a-sink`](/docs/rules/security/tainted-data/#regex-pattern-is-a-sink "A regex pattern is a sink with no launderer; the subject is data and may be tainted")); a format template drawn from a translation catalogue
([`security/every-grammar-is-a-sink`](/docs/rules/security/tainted-data/#every-grammar-is-a-sink "Every grammar the library parses is a sink, and none of the four gets a launderer")).

It removes `tainted` and nothing else: a `secret` operand is refused there, because confidentiality is
a separate axis and this member makes no claim about it.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/laundering/#launderers-are-sink-named" title="The only way out of tainted is a Core member whose contract names the one sink it is safe for"><code>security/launderers-are-sink-named</code></a> <a href="/docs/rules/security/tainted-data/#every-grammar-is-a-sink" title="Every grammar the library parses is a sink, and none of the four gets a launderer"><code>security/every-grammar-is-a-sink</code></a> <a href="/docs/rules/security/tainted-data/#metric-label-refuses-tainted" title="A metric label value refuses tainted, and there is deliberately no launderer for one"><code>security/metric-label-refuses-tainted</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0088.md">record 0088</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0076.md">record 0076</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/taint-assert-trusted-launders-for-every-sink-and-names-none.nvst"><code>tests/conformance/core/taint-assert-trusted-launders-for-every-sink-and-names-none.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/taint-assert-trusted-is-the-identity-and-the-reason-reaches-no-byte.nvst"><code>tests/conformance/core/taint-assert-trusted-is-the-identity-and-the-reason-reaches-no-byte.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/assert-trusted-removes-tainted-and-refuses-a-secret.nvst"><code>tests/conformance/reject/assert-trusted-removes-tainted-and-refuses-a-secret.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="response-body-is-one-typed-member">

## A response body is written by one typed member, and mixing `echo` with one of them does not compile

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#response-body-is-one-typed-member"><code>security/response-body-is-one-typed-member</code></a>
</div>

A response body is written by one of seven typed members, each owning a body shape and setting its own
content type. The HTML member takes the carrier and so has nothing to refuse; the JSON and text
members are contagious; the bytes member is contagious in its body and a sink in its content type; the
file member's path is a sink.

**Two of the seven write their body over time rather than at once, and classification follows the
shape and not the timing.** The streaming member declares a media type it is told, so that argument
is the bytes member's sink for the bytes member's reason — it becomes an instruction the peer obeys
about how to read everything after it — while each chunk is a union of text and bytes, which carries
no classification at all and therefore refuses a tainted argument outright. That is the fail-closed
direction of the two: the text member accepts a tainted body and a chunk does not.

The event-stream member takes no media type — the protocol's is the only one it could have — and its
`send` splits three ways. The payload is contagious, for the JSON member's reason: framing belongs to
us and to the serializer, and normalization happens before the payload is split across `data:` lines,
so it cannot reach any other line. The event name and the id are **sinks**: a client dispatches on
the name and echoes the id back in its next request's `Last-Event-ID`, so an attacker-chosen one is
the cross-tenant hazard a tainted topic name is.

The JSON member accepts a tainted value freely, because the framing belongs to the serializer and
never to concatenation — a tainted string becomes a JSON string value and cannot escape it. The text
member accepts one only because a no-sniff header is on by default with nothing configured, so
`text/plain` is not re-parsed as HTML; that dependency is stated so removing the default is visibly a
change to two rules.

**`echo` and a typed writer on the same response is a compile error.** They disagree about the body's
type and its content type, and silently letting the last one win is how a JSON endpoint acquires an
HTML prelude.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/tainted-data/#sink-predicate" title="A string or bytes parameter is a sink when its content becomes an instruction a parser executes"><code>security/sink-predicate</code></a> <a href="/docs/rules/security/laundering/#capture-answers-the-carrier" title="Capturing a sink yields that sink's carrier, never a plain string"><code>security/capture-answers-the-carrier</code></a> <a href="/docs/rules/security/laundering/#launderer-answers-a-carrier" title="A launderer answers its sink's carrier when that sink launders on its own and the transform is not idempotent, and a plain type otherwise"><code>security/launderer-answers-a-carrier</code></a> <a href="/docs/rules/core-classes/regex-html-and-introspection/#html-auto-escape" title="echo in an HTTP request escapes everything it is given, and Core\Html\Markup is the only raw-write bypass"><code>core-classes/html-auto-escape</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0088.md">record 0088</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0074.md">record 0074</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0177.md">record 0177</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-response-json-body-frames-a-tainted-value.nvst"><code>tests/conformance/core/a-response-json-body-frames-a-tainted-value.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-response-text-body-accepts-a-tainted-argument.nvst"><code>tests/conformance/core/a-response-text-body-accepts-a-tainted-argument.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/response.rs"><code>crates/nvs-types/tests/response.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="db-pool-reset-is-a-boundary">

## A pooled database connection is proven clean before it is reused, and a failed reset destroys it

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#db-pool-reset-is-a-boundary"><code>security/db-pool-reset-is-a-boundary</code></a>
</div>

A connection released at request teardown returns to a per-core pool, and **a released connection is
reset before it is reusable**. The reset is not best-effort: a connection that cannot be proven clean
is closed, and a driver with no reset primitive is not poolable at all.

What the reset must remove is stated as a **property, not a command list**: after it, no transaction,
no temporary table, no session variable, no assumed role, no advisory lock, no listener, no open
cursor and no prepared statement the cache does not still account for. A backend added later satisfies
that property or is not pooled.

**The pool key includes every credential**, so two configuration blocks are two pools and two database
users never share a connection. It is additionally scoped to the configuration generation it was read
from: a reload can publish the same block name under a different user, and a pool keyed on the name
alone would hand the new generation's request a connection authenticated as the old one's.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/closed-doors/#no-cross-request-state" title="Nothing a request does is observable by another request except through an explicit, capability-gated store"><code>security/no-cross-request-state</code></a> <a href="/docs/rules/security/protocols-and-tokens/#one-tls-client" title="There is one TLS client in the tree, and a driver reaches it over a generic transport rather than building a second"><code>security/one-tls-client</code></a> <a href="/docs/rules/security/scopes-and-denial/#net-address-policy" title="net.connect carries an address policy that denies the private ranges, and an exception is one written IP address"><code>security/net-address-policy</code></a> <a href="/docs/rules/core-classes/connecting-to-a-database/#db-connection-is-named" title="A connection is named in configuration or built from settings, and both memoize for the request"><code>core-classes/db-connection-is-named</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0078.md">record 0078</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0074.md">record 0074</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-db/tests/pool_reuse.rs"><code>crates/nvs-db/tests/pool_reuse.rs</code></a></dd></div></dl>

</div>
