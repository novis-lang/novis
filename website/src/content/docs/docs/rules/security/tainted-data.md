---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Tainted data and its sinks"
description: "Everything from outside arrives tainted, it poisons what it touches, and a sink is any parameter a parser will execute."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/security/scopes-and-denial/
  label: "Scopes, denial and the address policy"
next:
  link: /docs/rules/security/laundering/
  label: "Laundering"
---

<p class="nv-section-lead">Everything from outside arrives <code>tainted</code>, it poisons what it touches, and a sink is any parameter a parser will execute.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">10</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">9</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">1</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">1</span><span class="nv-count-label">differs from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#tainted-qualifier"><code>tainted</code> is a compile-time qualifier on <code>string</code>, <code>bytes</code> and a shape of them, spellable in any declaration and erased before codegen</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#tainted-sources">Every accessor that hands a program bytes from outside answers the <code>tainted</code> form, and the list of them is enumerable</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#taint-propagation">A tainted operand poisons every operation it takes part in, and only a checked conversion launders for free</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#sink-predicate">A <code>string</code> or <code>bytes</code> parameter is a sink when its content becomes an instruction a parser executes</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#unclassified-parameter-refuses-tainted">A <code>Core</code> parameter nobody classified refuses a tainted argument, and refuses to ship at all</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#every-grammar-is-a-sink">Every grammar the library parses is a sink, and none of the four gets a launderer</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#regex-pattern-is-a-sink">A regex pattern is a sink with no launderer; the subject is data and may be tainted</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#outbound-url-is-a-sink">An outbound URL or address is a sink, and a tainted one is refused where it is written</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#metric-label-refuses-tainted">A metric label value refuses <code>tainted</code>, and there is deliberately no launderer for one</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#log-is-not-a-sink">A log field is data, so it takes a tainted value and wants one</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li></ol>

<div class="nv-rule" id="tainted-qualifier">

## `tainted` is a compile-time qualifier on `string`, `bytes` and a shape of them, spellable in any declaration and erased before codegen

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#tainted-qualifier"><code>security/tainted-qualifier</code></a>
</div>

`tainted string` and `tainted bytes` join the type grammar as a qualified form of the two scalar
types, and `tainted {…}` writes the same fact over a whole shape of them. It is not a class, not a
wrapper and not a run-time tag: it is checked once, while checking, and carries no representation past
that point — no extra byte in the value's header, no refcount change, nothing on the hot path.

**Over a shape it distributes and then disappears.** `tainted {a: string, b: {c: bytes}}` is rewritten
while parsing to the shape whose text-carrying fields are their tainted forms, transitively — through a
nested shape, a nullable, a union member and an `array<string>` element — so it produces exactly the
field-by-field spelling it saves, and no layer past the parser knows it was written. A shape carrying no
`string` and no `bytes` anywhere is a diagnostic rather than a no-op: a qualifier that promises nothing
still reads as a promise.

**`mixed` is not qualifiable, and that is the same boundary.** The checker cannot distribute a qualifier
through an erased container, so `tainted mixed` would promise what nothing enforces; structured input
stays `array<mixed>` and the qualifier is about the payload once it is named. A request body recovers its
taint by being converted into a shape that carries it — not by qualifying the container it arrived in.

**It is grammar, not only a type-checker fact.** Every binding carries a written type
([`types/declaration`](/docs/rules/types/declarations-and-numbers/#declaration "Every binding declares its type, and no binding's type ever changes")), so a function that receives a tainted value and passes it on has nowhere
to put that fact unless `tainted` can be spelled in an ordinary declaration. Without it, taint would
disappear silently at the first call boundary — which is the hole the qualifier exists to close — or
every request-handling function would have to launder on its first line.

`tainted` is written after `secret` when both appear, and `tainted secret string` is a diagnostic
naming the required order rather than a second spelling ([`security/secret-qualifier`](/docs/rules/security/secrets/#secret-qualifier "secret is a second, independent compile-time qualifier, written before tainted and in that order alone")).

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Untrusted input has a type PHP has no way to spell, and a program that mixes it into a sink does not compile</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/tainted-data/#tainted-sources" title="Every accessor that hands a program bytes from outside answers the tainted form, and the list of them is enumerable"><code>security/tainted-sources</code></a> <a href="/docs/rules/security/tainted-data/#taint-propagation" title="A tainted operand poisons every operation it takes part in, and only a checked conversion launders for free"><code>security/taint-propagation</code></a> <a href="/docs/rules/security/tainted-data/#sink-predicate" title="A string or bytes parameter is a sink when its content becomes an instruction a parser executes"><code>security/sink-predicate</code></a> <a href="/docs/rules/types/declarations-and-numbers/#grammar" title="The type grammar is a closed set of atoms under unions and intersections"><code>types/grammar</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0009.md">record 0009</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0033.md">record 0033</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0157.md">record 0157</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/tainted.rs"><code>crates/nvs-types/tests/tainted.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/a-tainted-value-answers-what-its-unqualified-twin-does.nvst"><code>tests/conformance/lang/a-tainted-value-answers-what-its-unqualified-twin-does.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="tainted-sources">

## Every accessor that hands a program bytes from outside answers the `tainted` form, and the list of them is enumerable

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#tainted-sources"><code>security/tainted-sources</code></a>
</div>

Every member of the request, server, session, environment and CLI accessors, and a script's own
arguments, answers the `tainted` form of whatever it already returned. Header values and any other
client-influenced field are included; a fixed enum-shaped field such as the request method is not
attacker-shaped in the same way and is not required to be.

**An outbound reply's body is input in the same sense**, so a response body read back from a client
answers `tainted string`: pinning an address settles which host the bytes came from and says nothing
about what is in them, and a reply a program asked for is no safer than one it was sent. Its status
code is not tainted — three digits carry nothing a sink can misread. Values read back out of a
database are `tainted` under the same standing rule, which is what closes stored injection by the same
mechanism as reflected.

Structured input stays `array<mixed>`; the qualifier is about the scalar payload once it is pulled out
of `mixed`. The list of sources being **enumerable** is what lets the qualifier attach itself
automatically, and is exactly what `secret` has no equivalent of
([`security/secret-has-no-ambient-source`](/docs/rules/security/secrets/#secret-has-no-ambient-source "Nothing grants secret ambiently; it appears where a developer wrote it, and at one member that is its own contract")).

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/tainted-data/#tainted-qualifier" title="tainted is a compile-time qualifier on string, bytes and a shape of them, spellable in any declaration and erased before codegen"><code>security/tainted-qualifier</code></a> <a href="/docs/rules/security/secrets/#secret-has-no-ambient-source" title="Nothing grants secret ambiently; it appears where a developer wrote it, and at one member that is its own contract"><code>security/secret-has-no-ambient-source</code></a> <a href="/docs/rules/statements/where-state-lives/#no-host-populated-variables" title="No variable is ever populated by the host; every superglobal is a Core class member"><code>statements/no-host-populated-variables</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0012.md">record 0012</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0058.md">record 0058</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-request-value-arrives-tainted-and-is-laundered-to-be-used.nvst"><code>tests/conformance/core/a-request-value-arrives-tainted-and-is-laundered-to-be-used.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-response-body-is-tainted-and-its-status-is-not.nvst"><code>tests/conformance/reject/a-response-body-is-tainted-and-its-status-is-not.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/env-a-variable-name-is-a-sink-and-every-value-is-tainted.nvst"><code>tests/conformance/core/env-a-variable-name-is-a-sink-and-every-value-is-tainted.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="taint-propagation">

## A tainted operand poisons every operation it takes part in, and only a checked conversion launders for free

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#taint-propagation"><code>security/taint-propagation</code></a>
</div>

Any operation combining a tainted operand with an untainted one — concatenation, interpolation, a
string member, an array of scalars — produces a tainted result. This is the poisoned shape the type
system already uses on other axes, applied to a new one, and `secret` poisons independently beside it
([`security/secret-propagation`](/docs/rules/security/secrets/#secret-propagation "A secret operand poisons its own axis independently of tainted, and a checked conversion strips it")).

A checked `as` conversion to a type that already throws on a malformed shape — `as uint`, `as int`,
`as float`, `as bool`, an enum's backing type — **removes the qualifier on success**. No new syntax is
needed: a value that survived the check has had its shape proven, which is what laundering means for a
non-string type. `as ?T` decides the qualifier by exactly this rule and launders nothing of its own
([`expressions/conversion-keeps-qualifiers`](/docs/rules/expressions/conversion-and-intrinsics/#conversion-keeps-qualifiers "as ?T decides tainted and secret by exactly the rule as T uses, and launders nothing of its own")).

`bytes as string` and `string as bytes` **preserve** the qualifier in either direction. UTF-8 validity
says nothing about whether the content is safe for a given sink, and a conversion that laundered here
would be a one-word bypass of every rule below.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/tainted-data/#tainted-qualifier" title="tainted is a compile-time qualifier on string, bytes and a shape of them, spellable in any declaration and erased before codegen"><code>security/tainted-qualifier</code></a> <a href="/docs/rules/security/laundering/#launderers-are-sink-named" title="The only way out of tainted is a Core member whose contract names the one sink it is safe for"><code>security/launderers-are-sink-named</code></a> <a href="/docs/rules/types/unions-and-conversion/#conversion" title="expr as T is the only conversion, and it produces a T or throws"><code>types/conversion</code></a> <a href="/docs/rules/expressions/conversion-and-intrinsics/#conversion-keeps-qualifiers" title="as ?T decides tainted and secret by exactly the rule as T uses, and launders nothing of its own"><code>expressions/conversion-keeps-qualifiers</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0066.md">record 0066</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/tainted.rs"><code>crates/nvs-types/tests/tainted.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/tainted-and-secret-widen-but-never-narrow.nvst"><code>tests/conformance/reject/tainted-and-secret-widen-but-never-narrow.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="sink-predicate">

## A `string` or `bytes` parameter is a sink when its content becomes an instruction a parser executes

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#sink-predicate"><code>security/sink-predicate</code></a>
</div>

> A `string`/`bytes` parameter is a `tainted` sink when its content becomes an instruction that a
> parser executes. It is not a sink when its content is data that a parser returns, or that a
> serializer or a protocol frames.

That one line derives every worked case, which is the test of it being the right predicate rather than
a restatement. A query's text is a sink and its bound parameters are not, because the wire protocol
frames each value. An argv element is data *because* there is no shell to execute it, while the
executable path is an instruction. A header value is a sink because `CR`/`LF` re-frames the message; a
body is not, because `Content-Length` frames it. A path component is a sink because `..` and
separators direct the resolver; a file's contents are not. A regex pattern is a sink; its subject is
not.

A new sink therefore needs no decision record of its own — it needs the predicate applied. The four
grammars the library parses are all instructions by it
([`security/every-grammar-is-a-sink`](/docs/rules/security/tainted-data/#every-grammar-is-a-sink "Every grammar the library parses is a sink, and none of the four gets a launderer")), and a member nobody classified fails closed
([`security/unclassified-parameter-refuses-tainted`](/docs/rules/security/tainted-data/#unclassified-parameter-refuses-tainted "A Core parameter nobody classified refuses a tainted argument, and refuses to ship at all")).

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/tainted-data/#unclassified-parameter-refuses-tainted" title="A Core parameter nobody classified refuses a tainted argument, and refuses to ship at all"><code>security/unclassified-parameter-refuses-tainted</code></a> <a href="/docs/rules/security/tainted-data/#log-is-not-a-sink" title="A log field is data, so it takes a tainted value and wants one"><code>security/log-is-not-a-sink</code></a> <a href="/docs/rules/security/tainted-data/#every-grammar-is-a-sink" title="Every grammar the library parses is a sink, and none of the four gets a launderer"><code>security/every-grammar-is-a-sink</code></a> <a href="/docs/rules/security/laundering/#launderers-are-sink-named" title="The only way out of tainted is a Core member whose contract names the one sink it is safe for"><code>security/launderers-are-sink-named</code></a> <a href="/docs/rules/core-classes/running-a-statement/#db-parameters" title="One placeholder is one value, and expanding a list into IN is written with Core\Db::inList"><code>core-classes/db-parameters</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0088.md">record 0088</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0063.md">record 0063</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/every-grammar-sink-refuses-a-tainted-argument.nvst"><code>tests/conformance/reject/every-grammar-sink-refuses-a-tainted-argument.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-core-io-path-refuses-a-tainted-argument.nvst"><code>tests/conformance/reject/a-core-io-path-refuses-a-tainted-argument.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/db-query-refuses-a-tainted-statement-and-binds-one-freely.nvst"><code>tests/conformance/core/db-query-refuses-a-tainted-statement-and-binds-one-freely.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="unclassified-parameter-refuses-tainted">

## A `Core` parameter nobody classified refuses a tainted argument, and refuses to ship at all

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#unclassified-parameter-refuses-tainted"><code>security/unclassified-parameter-refuses-tainted</code></a>
</div>

A `Core` member whose `string`/`bytes` parameter carries no classification **rejects a tainted
argument**. *Contagious* is a thing an author writes rather than a thing an author gets by forgetting,
and the failure direction inverts: before, a member nobody classified accepted tainted data at run
time in production; now it refuses where the call is written.

The classification lives once, beside the parameter list and the return type in the member registry,
and a member with an unclassified parameter **fails the library's own test suite** — the default is
what a program sees, the test is what stops such a member from shipping at all.

A **union parameter** carries the mark its arms declare — `Pattern|string` is a sink because the
`string` arm is written as one — and carries none where its arms disagree or where an arm that could
hold a mark does not. The classification is still written once beside the parameter, on the arm that
has a cell for it, and the refusing default still covers everything nobody classified.

A mark that admits `tainted` admits a union carrying it, arm by arm, and the contagion is read back
out with at least that reach. The two directions are deliberately asymmetric: the answer that decides
whether to *set* the qualifier on a result reaches further than the narrowing that decides whether to
*admit* an argument, because reaching too far over-taints in the first case and leaks in the second.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/tainted-data/#sink-predicate" title="A string or bytes parameter is a sink when its content becomes an instruction a parser executes"><code>security/sink-predicate</code></a> <a href="/docs/rules/security/tainted-data/#tainted-qualifier" title="tainted is a compile-time qualifier on string, bytes and a shape of them, spellable in any declaration and erased before codegen"><code>security/tainted-qualifier</code></a> <a href="/docs/rules/testing/coverage-and-probes/#capability-closure-test" title="Every member of a capability-bearing class declares a capability or declares none, and there is no allowlist"><code>testing/capability-closure-test</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0088.md">record 0088</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-contagious-parameter-taints-the-answer-and-a-neutral-one-does-not.nvst"><code>tests/conformance/core/a-contagious-parameter-taints-the-answer-and-a-neutral-one-does-not.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-tainted-argument-is-refused-where-the-answer-cannot-carry-it.nvst"><code>tests/conformance/reject/a-tainted-argument-is-refused-where-the-answer-cannot-carry-it.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-union-carrying-tainted-still-refuses-where-the-atom-does.nvst"><code>tests/conformance/reject/a-union-carrying-tainted-still-refuses-where-the-atom-does.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="every-grammar-is-a-sink">

## Every grammar the library parses is a sink, and none of the four gets a launderer

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#every-grammar-is-a-sink"><code>security/every-grammar-is-a-sink</code></a>
</div>

The library parses exactly four grammars — a regex pattern, a format template, a date pattern, and a
byte-packing format — and every one of them is an instruction under
[`security/sink-predicate`](/docs/rules/security/tainted-data/#sink-predicate "A string or bytes parameter is a sink when its content becomes an instruction a parser executes"). So each of the four is a sink, and the count that fixes the library's
grammar surface doubles as the roster.

**No launderer is added for any of them, deliberately.** A grammar is written by the program, not
received by it, so the fix at a failing call site is to use a literal — which is folded at compile time
for all four ([`expressions/intrinsic-literals`](/docs/rules/expressions/conversion-and-intrinsics/#intrinsic-literals "A literal argument to an intrinsic Core call is validated while checking and prepared into the artifact")) — or, for a genuinely dynamic template drawn from
a translation catalogue, [`security/assert-trusted`](/docs/rules/security/laundering/#assert-trusted "Core\Taint::assertTrusted is the one generic way out, and it carries a written reason at the call site") with its written reason.

A regex quoting member remains the one exception on the roster, because a regex is the one grammar
that routinely needs a runtime value *inside* it rather than *as* it
([`security/regex-pattern-is-a-sink`](/docs/rules/security/tainted-data/#regex-pattern-is-a-sink "A regex pattern is a sink with no launderer; the subject is data and may be tainted")).

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/tainted-data/#sink-predicate" title="A string or bytes parameter is a sink when its content becomes an instruction a parser executes"><code>security/sink-predicate</code></a> <a href="/docs/rules/security/tainted-data/#regex-pattern-is-a-sink" title="A regex pattern is a sink with no launderer; the subject is data and may be tainted"><code>security/regex-pattern-is-a-sink</code></a> <a href="/docs/rules/security/laundering/#assert-trusted" title="Core\Taint::assertTrusted is the one generic way out, and it carries a written reason at the call site"><code>security/assert-trusted</code></a> <a href="/docs/rules/expressions/conversion-and-intrinsics/#intrinsic-literals" title="A literal argument to an intrinsic Core call is validated while checking and prepared into the artifact"><code>expressions/intrinsic-literals</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0088.md">record 0088</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0063.md">record 0063</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0057.md">record 0057</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0056.md">record 0056</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/every-grammar-sink-refuses-a-tainted-argument.nvst"><code>tests/conformance/reject/every-grammar-sink-refuses-a-tainted-argument.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/bytes-pack-and-unpack-refuse-a-tainted-format.nvst"><code>tests/conformance/reject/bytes-pack-and-unpack-refuse-a-tainted-format.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="regex-pattern-is-a-sink">

## A regex pattern is a sink with no launderer; the subject is data and may be tainted

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#regex-pattern-is-a-sink"><code>security/regex-pattern-is-a-sink</code></a>
</div>

A regex **pattern** requires the plain, unqualified `string`. A user-supplied pattern is both a
denial-of-service vector and a logic-injection vector: a pattern an attacker controls can be made to
match anything, which turns a validation check into an approval.

**There is no laundering member for it**, because there is no meaningful way to make an arbitrary
attacker-authored pattern safe. A program that genuinely needs one uses
[`security/assert-trusted`](/docs/rules/security/laundering/#assert-trusted "Core\Taint::assertTrusted is the one generic way out, and it carries a written reason at the call site") and says why. That is the shape every grammar on the roster takes
([`security/every-grammar-is-a-sink`](/docs/rules/security/tainted-data/#every-grammar-is-a-sink "Every grammar the library parses is a sink, and none of the four gets a launderer")); the regex quoting member is not a counter-example, because
it escapes a value to sit *inside* a pattern rather than to *be* one.

The **subject** may be tainted, and contagion applies unchanged: a substring matched out of a tainted
subject is tainted ([`security/taint-propagation`](/docs/rules/security/tainted-data/#taint-propagation "A tainted operand poisons every operation it takes part in, and only a checked conversion launders for free")). The denial-of-service half is answered
separately, by tiering the pattern onto a linear-time engine and budgeting the backtracking tier.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/tainted-data/#sink-predicate" title="A string or bytes parameter is a sink when its content becomes an instruction a parser executes"><code>security/sink-predicate</code></a> <a href="/docs/rules/security/tainted-data/#every-grammar-is-a-sink" title="Every grammar the library parses is a sink, and none of the four gets a launderer"><code>security/every-grammar-is-a-sink</code></a> <a href="/docs/rules/security/laundering/#assert-trusted" title="Core\Taint::assertTrusted is the one generic way out, and it carries a written reason at the call site"><code>security/assert-trusted</code></a> <a href="/docs/rules/core-classes/regex-html-and-introspection/#regex-two-tiers" title="A pattern runs on the linear engine unless it cannot, and the backtracking tier's step budget throws when exhausted"><code>core-classes/regex-two-tiers</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0056.md">record 0056</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0088.md">record 0088</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/every-grammar-sink-refuses-a-tainted-argument.nvst"><code>tests/conformance/reject/every-grammar-sink-refuses-a-tainted-argument.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/regex-tiers-and-the-backtracking-budget.nvst"><code>tests/conformance/core/regex-tiers-and-the-backtracking-budget.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="outbound-url-is-a-sink">

## An outbound URL or address is a sink, and a tainted one is refused where it is written

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#outbound-url-is-a-sink"><code>security/outbound-url-is-a-sink</code></a>
</div>

The URL parameter of an HTTP client's request members, and the address parameter of a raw connect, are
unqualified: a `tainted` value at either position is a compile-time diagnostic, exactly as at a query
text parameter. Server-side request forgery starts with an attacker-chosen address, and this is the
position where that is visible.

The plain form covers URLs the program itself authored — a literal, a configuration value, a composed
path — and a literal is additionally validated while checking
([`expressions/intrinsic-literals`](/docs/rules/expressions/conversion-and-intrinsics/#intrinsic-literals "A literal argument to an intrinsic Core call is validated while checking and prepared into the artifact")). A URL that genuinely came from outside goes through the
laundering member that also **pins** the address it resolved to, so the check and the connection
cannot disagree about which host was approved.

Refusing at the call site is only half of it: the address is judged again at run time against the
capability's own policy ([`security/net-address-policy`](/docs/rules/security/scopes-and-denial/#net-address-policy "net.connect carries an address policy that denies the private ranges, and an exception is one written IP address")), because a hardcoded hostname can still
resolve into a private range.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/scopes-and-denial/#net-address-policy" title="net.connect carries an address policy that denies the private ranges, and an exception is one written IP address"><code>security/net-address-policy</code></a> <a href="/docs/rules/security/scopes-and-denial/#the-policy-lives-in-the-capability" title="The address policy lives in the capability, so every client obeys it and none of them may hold its own"><code>security/the-policy-lives-in-the-capability</code></a> <a href="/docs/rules/security/tainted-data/#sink-predicate" title="A string or bytes parameter is a sink when its content becomes an instruction a parser executes"><code>security/sink-predicate</code></a> <a href="/docs/rules/expressions/conversion-and-intrinsics/#intrinsic-literals" title="A literal argument to an intrinsic Core call is validated while checking and prepared into the artifact"><code>expressions/intrinsic-literals</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0058.md">record 0058</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0088.md">record 0088</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/every-client-member-refuses-a-tainted-url.nvst"><code>tests/conformance/core/every-client-member-refuses-a-tainted-url.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/an-approved-url-is-pinned-to-one-address.nvst"><code>tests/conformance/core/an-approved-url-is-pinned-to-one-address.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="metric-label-refuses-tainted">

## A metric label value refuses `tainted`, and there is deliberately no launderer for one

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#metric-label-refuses-tainted"><code>security/metric-label-refuses-tainted</code></a>
</div>

A metric label value is a sink: a query parameter, a header, a path segment or a database column
cannot become a label.

There is deliberately **no launderer for it**, because there is no sanitisation that would make one
safe. The hazard is not the value's content, it is that the value is drawn from an unbounded set, and
an unbounded label set is how a metrics collector falls over. What exists instead is everything that
is already unqualified and is what a label should have been: an enum case or an integer converted with
`as`, which laundering makes free ([`security/taint-propagation`](/docs/rules/security/tainted-data/#taint-propagation "A tainted operand poisons every operation it takes part in, and only a checked conversion launders for free")); a route name from the compiled
table; a literal, a class constant or a configured value; and, for a genuinely bounded user-derived
set, [`security/assert-trusted`](/docs/rules/security/laundering/#assert-trusted "Core\Taint::assertTrusted is the one generic way out, and it carries a written reason at the call site").

`secret` is refused there too and needs no new rule: a metric export is output, and every output sink
already refuses it ([`security/secret-sinks-refuse`](/docs/rules/security/secrets/#secret-sinks-refuse "Every output sink refuses a secret value outright, and no auto-escape or neutralisation bypasses one")).

The `Core\Metrics` surface this governs is not on disk: nothing in the tree exports a series or takes
a label, so the refusal has no implementation to verify against.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/tainted-data/#sink-predicate" title="A string or bytes parameter is a sink when its content becomes an instruction a parser executes"><code>security/sink-predicate</code></a> <a href="/docs/rules/security/laundering/#assert-trusted" title="Core\Taint::assertTrusted is the one generic way out, and it carries a written reason at the call site"><code>security/assert-trusted</code></a> <a href="/docs/rules/security/laundering/#route-capture-is-laundered-by-its-type" title="A route capture is laundered by the parameter's own type, and a string capture stays tainted"><code>security/route-capture-is-laundered-by-its-type</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0076.md">record 0076</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0033.md">record 0033</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0077.md">record 0077</a></dd></div></dl>

</div>

<div class="nv-rule" id="log-is-not-a-sink">

## A log field is data, so it takes a tainted value and wants one

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#log-is-not-a-sink"><code>security/log-is-not-a-sink</code></a>
</div>

A log field is data: the writer is a structured serializer, so a field is framed by the serializer and
never by string concatenation into a line. Log forging is therefore closed by the writer's shape
([`errors/log-write`](/docs/rules/errors/diagnostics-and-logging/#log-write "One write path, and the engine floor is its other caller")), independently of the qualifier system, and logging tainted content is
*desired* rather than tolerated — recording exactly what an attacker sent is the point of a security
log.

The same holds for a bidirectional control, which the writer escapes by construction
([`security/bidi-boundaries`](/docs/rules/security/bidi-and-passwords/#bidi-boundaries "The lexer refuses an unterminated control and the sinks substitute it, and nobody writes a second predicate")), and for a JSON decode, whose input is data the parse returns to the
program and whose result is `tainted`.

The `secret` axis does not follow: a log field **refuses** a `secret` value, because that axis is
about confidentiality rather than structure and a log is an output
([`security/secret-sinks-refuse`](/docs/rules/security/secrets/#secret-sinks-refuse "Every output sink refuses a secret value outright, and no auto-escape or neutralisation bypasses one")). Stating both here is what stops a future reader from applying
one rule's answer to the other axis.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/tainted-data/#sink-predicate" title="A string or bytes parameter is a sink when its content becomes an instruction a parser executes"><code>security/sink-predicate</code></a> <a href="/docs/rules/security/secrets/#secret-sinks-refuse" title="Every output sink refuses a secret value outright, and no auto-escape or neutralisation bypasses one"><code>security/secret-sinks-refuse</code></a> <a href="/docs/rules/errors/diagnostics-and-logging/#log-fields" title="A record's fields are named and typed, not a stringly bag"><code>errors/log-fields</code></a> <a href="/docs/rules/errors/diagnostics-and-logging/#log-write" title="One write path, and the engine floor is its other caller"><code>errors/log-write</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0088.md">record 0088</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0020.md">record 0020</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0087.md">record 0087</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/log-write-refuses-a-secret-field.nvst"><code>tests/conformance/reject/log-write-refuses-a-secret-field.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/tainted.rs"><code>crates/nvs-types/tests/tainted.rs</code></a></dd></div></dl>

</div>
