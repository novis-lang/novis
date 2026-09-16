---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Secrets"
description: "A second, independent qualifier. Every sink refuses it, it crosses no boundary, comparing two is constant-time, and no rendering the toolchain prints…"
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/security/laundering/
  label: "Laundering"
next:
  link: /docs/rules/security/extensions-and-qualifiers/
  label: "Qualifiers across an extension"
---

<p class="nv-section-lead">A second, independent qualifier. Every sink refuses it, it crosses no boundary, comparing two is constant-time, and no rendering the toolchain prints carries one in the clear.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">8</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">7</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">1</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">3</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#secret-qualifier"><code>secret</code> is a second, independent compile-time qualifier, written before <code>tainted</code> and in that order alone</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#secret-has-no-ambient-source">Nothing grants <code>secret</code> ambiently; it appears where a developer wrote it, and at one member that is its own contract</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#secret-propagation">A <code>secret</code> operand poisons its own axis independently of <code>tainted</code>, and a checked conversion strips it</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#secret-sinks-refuse">Every output sink refuses a <code>secret</code> value outright, and no auto-escape or neutralisation bypasses one</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#secret-crosses-no-boundary">A <code>secret</code> value is refused at the one graph copy, so it reaches neither <code>serialize</code> nor any spawn</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#secret-comparison-is-constant-time">Comparing two <code>secret</code> values with <code>==</code> lowers to a constant-time helper, decided by the compiler</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#secret-in-a-test-report">A <code>secret</code> operand is compared but never rendered, and no flag lifts the redaction</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#redaction-reaches-the-tools-own-renderings">No rendering the toolchain itself produces prints a <code>secret</code> in the clear, the AST dump included</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li></ol>

<div class="nv-rule" id="secret-qualifier">

## `secret` is a second, independent compile-time qualifier, written before `tainted` and in that order alone

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#secret-qualifier"><code>security/secret-qualifier</code></a>
</div>

`secret` is a second, independent compile-time qualifier on `string` and `bytes`. It and `tainted` are
independent bits rather than a combined enum: a value can be plain, `tainted`, `secret`, or `secret
tainted`. When both are spelled together, **`secret` comes first** — the only accepted order, and
`tainted secret string` is a diagnostic naming the required order rather than a second valid spelling
of the same type.

Like `tainted` it is a reserved keyword and a production in the type grammar, checked once and erased
before codegen, so a `secret` value costs nothing at run time
([`security/tainted-qualifier`](/docs/rules/security/tainted-data/#tainted-qualifier "tainted is a compile-time qualifier on string, bytes and a shape of them, spellable in any declaration and erased before codegen")).

Two orthogonal questions — trust and confidentiality — get their own checked axis instead of being
conflated into one. They compose for the case that matters most in practice, a submitted password,
without needing a third combined concept.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Confidentiality is a type, so a credential reaching a page, a log or an exception message is a compile error</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/secrets/#secret-has-no-ambient-source" title="Nothing grants secret ambiently; it appears where a developer wrote it, and at one member that is its own contract"><code>security/secret-has-no-ambient-source</code></a> <a href="/docs/rules/security/secrets/#secret-propagation" title="A secret operand poisons its own axis independently of tainted, and a checked conversion strips it"><code>security/secret-propagation</code></a> <a href="/docs/rules/security/secrets/#secret-sinks-refuse" title="Every output sink refuses a secret value outright, and no auto-escape or neutralisation bypasses one"><code>security/secret-sinks-refuse</code></a> <a href="/docs/rules/types/declarations-and-numbers/#grammar" title="The type grammar is a closed set of atoms under unions and intersections"><code>types/grammar</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0033.md">record 0033</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/secret.rs"><code>crates/nvs-types/tests/secret.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/tainted-and-secret-widen-but-never-narrow.nvst"><code>tests/conformance/reject/tainted-and-secret-widen-but-never-narrow.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="secret-has-no-ambient-source">

## Nothing grants `secret` ambiently; it appears where a developer wrote it, and at one member that is its own contract

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#secret-has-no-ambient-source"><code>security/secret-has-no-ambient-source</code></a>
</div>

Unlike `tainted`, **nothing in Novis grants `secret` ambiently.** The five host accessors are why
`tainted` can attach itself automatically — every one is a named, enumerable place untrusted data
enters ([`security/tainted-sources`](/docs/rules/security/tainted-data/#tainted-sources "Every accessor that hands a program bytes from outside answers the tainted form, and the list of them is enumerable")). There is no equivalent list for secrecy: an environment
read, a session value, a database row and a source literal all look identical to the type checker
whether or not their content happens to be a credential.

`secret` therefore appears only where a developer spells it on a declaration — a parameter, return
type, property or local. A helper that loads an API key is expected to declare its own return type
`secret string`; the language gives no free ride.

**One member originates the qualifier**, and it is the exception that proves the rule: reading a
password at a terminal prompt with echo disabled answers `secret tainted string`, because the
*member's contract* is confidentiality and there is no reading of its result that is not a credential.
That is a declared return type, not an ambient grant.

The cost is stated: `secret` protects only what a developer remembers to annotate. That is a real
coverage gap relative to `tainted`, not an oversight.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/secrets/#secret-qualifier" title="secret is a second, independent compile-time qualifier, written before tainted and in that order alone"><code>security/secret-qualifier</code></a> <a href="/docs/rules/security/tainted-data/#tainted-sources" title="Every accessor that hands a program bytes from outside answers the tainted form, and the list of them is enumerable"><code>security/tainted-sources</code></a> <a href="/docs/rules/security/secrets/#secret-propagation" title="A secret operand poisons its own axis independently of tainted, and a checked conversion strips it"><code>security/secret-propagation</code></a> <a href="/docs/rules/core-classes/processes-and-files/#cli-arguments" title="Core\Cli::arguments is how a program reads the words it was started with, at every depth"><code>core-classes/cli-arguments</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0033.md">record 0033</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0086.md">record 0086</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0012.md">record 0012</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/cli-secret-originates-a-secret-no-sink-will-write.nvst"><code>tests/conformance/core/cli-secret-originates-a-secret-no-sink-will-write.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="secret-propagation">

## A `secret` operand poisons its own axis independently of `tainted`, and a checked conversion strips it

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#secret-propagation"><code>security/secret-propagation</code></a>
</div>

Any operation combining a `secret` operand with a non-`secret` one produces a `secret` result — the
poisoning shape [`security/taint-propagation`](/docs/rules/security/tainted-data/#taint-propagation "A tainted operand poisons every operation it takes part in, and only a checked conversion launders for free") defines, applied to this axis **independently**. A
`secret tainted string` interpolated with a plain `string` stays `secret tainted string`; each axis
tracks on its own.

A successful checked `as` conversion **removes `secret`**, mirroring the `tainted` rule for grammar
and implementation consistency rather than because the underlying justification transfers — it does
not. A shape proof says nothing about confidentiality, so a numeric credential that round-trips
through `as uint` becomes a plain, loggable, displayable value with no diagnostic. **That is a known,
accepted gap**, chosen for consistency and recorded here rather than discovered later.

Conversions between `string` and `bytes` preserve `secret` in either direction, the same way they
preserve `tainted`.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/secrets/#secret-qualifier" title="secret is a second, independent compile-time qualifier, written before tainted and in that order alone"><code>security/secret-qualifier</code></a> <a href="/docs/rules/security/tainted-data/#taint-propagation" title="A tainted operand poisons every operation it takes part in, and only a checked conversion launders for free"><code>security/taint-propagation</code></a> <a href="/docs/rules/expressions/conversion-and-intrinsics/#conversion-keeps-qualifiers" title="as ?T decides tainted and secret by exactly the rule as T uses, and launders nothing of its own"><code>expressions/conversion-keeps-qualifiers</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0033.md">record 0033</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0066.md">record 0066</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/secret.rs"><code>crates/nvs-types/tests/secret.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/tainted-and-secret-widen-but-never-narrow.nvst"><code>tests/conformance/reject/tainted-and-secret-widen-but-never-narrow.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="secret-sinks-refuse">

## Every output sink refuses a `secret` value outright, and no auto-escape or neutralisation bypasses one

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#secret-sinks-refuse"><code>security/secret-sinks-refuse</code></a>
</div>

Every **output** sink refuses a `secret` value, and no neutralisation bypasses one: HTML and response
output refuse outright with no auto-escape, terminal output refuses with no carrier bypass, a log
field refuses, a serialiser refuses anywhere in the value it walks — including where the encode is
inside the member, as an enqueued job's payload is — a `Throwable` message requires the
plain type, an attribute payload refuses, and a debug dump renders a fixed placeholder instead of the
value. Escaping fully neutralises injection and does **nothing** for confidentiality — an escaped
credential is still a leaked credential, just HTML-safe.

"Terminal" understates the reach, which is the argument for the row: a scheduled script, a job worker,
a test method and a spawned isolate all write through that sink, so it covers a CI log, a job log and
a test report — which is where a credential in practice leaks.

**Three positions are deliberately not on the list**: bound database parameters, a process argv, and
an outbound request's headers and body. A credential legitimately needs to reach a driver, a
subprocess or an outbound call, and refusing there would make the qualifier unusable for its own
purpose. This closes *accidental* exposure, not *intentional, narrow* use. An attribute payload is the
one sink with no way out at all, being folded into the program's own metadata; the fix is to carry the
name of where to read the secret from.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>echo $apiKey</code>, <code>var_dump($key)</code> and a credential interpolated into an exception message are all compile errors</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/secrets/#secret-crosses-no-boundary" title="A secret value is refused at the one graph copy, so it reaches neither serialize nor any spawn"><code>security/secret-crosses-no-boundary</code></a> <a href="/docs/rules/security/secrets/#secret-in-a-test-report" title="A secret operand is compared but never rendered, and no flag lifts the redaction"><code>security/secret-in-a-test-report</code></a> <a href="/docs/rules/security/tainted-data/#log-is-not-a-sink" title="A log field is data, so it takes a tainted value and wants one"><code>security/log-is-not-a-sink</code></a> <a href="/docs/rules/errors/diagnostics-and-logging/#record-transformations" title="Redaction, control bytes, bidi and elision are decided in the record"><code>errors/record-transformations</code></a> <a href="/docs/rules/core-classes/regex-html-and-introspection/#secret-reveal" title="Core\Secret::reveal is the one named way out of secret, and it carries a written reason"><code>core-classes/secret-reveal</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0033.md">record 0033</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0086.md">record 0086</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0088.md">record 0088</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0092.md">record 0092</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0028.md">record 0028</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/echo-refuses-a-secret.nvst"><code>tests/conformance/reject/echo-refuses-a-secret.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/log-write-refuses-a-secret-field.nvst"><code>tests/conformance/reject/log-write-refuses-a-secret-field.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/json-encode-refuses-a-secret.nvst"><code>tests/conformance/reject/json-encode-refuses-a-secret.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/queue-push-refuses-a-secret.nvst"><code>tests/conformance/reject/queue-push-refuses-a-secret.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-secret-value-cannot-be-dumped.nvst"><code>tests/conformance/reject/a-secret-value-cannot-be-dumped.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-secret-class-constant-cannot-reach-an-attribute-payload.nvst"><code>tests/conformance/reject/a-secret-class-constant-cannot-reach-an-attribute-payload.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="secret-crosses-no-boundary">

## A `secret` value is refused at the one graph copy, so it reaches neither `serialize` nor any spawn

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#secret-crosses-no-boundary"><code>security/secret-crosses-no-boundary</code></a>
</div>

A `secret` value is refused at the one recursive graph-copy operation, for both of its callers alike —
serialization to bytes and every crossing into a task, a worker or an isolate — rather than drawing a
new distinction between "crossing to a live isolate" and "externalizing to bytes."

The refusal has two halves because the walk cannot see everything. At run time the walk refuses a
`secret`-typed property alongside the closures, aliases and host handles it already refuses
([`security/isolate-values-cross-by-copy`](/docs/rules/security/isolates/#isolate-values-cross-by-copy "A value crosses an isolate boundary as one graph copy, and a closure, an alias or a host handle does not cross at all")). The half a run-time walk cannot see is refused earlier,
by reading a call's **written arguments** while checking; the three spawn forms hand that check their
own argument list rather than growing a second rule.

Where a worker genuinely needs a credential to do its job, the conspicuous reveal before the call is
the intended escape — explicit, greppable, and at the one call site where the decision belongs. That
this may add friction to a worker that exists specifically to isolate credential handling is a stated
cost of keeping one copy operation rather than two.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/secrets/#secret-sinks-refuse" title="Every output sink refuses a secret value outright, and no auto-escape or neutralisation bypasses one"><code>security/secret-sinks-refuse</code></a> <a href="/docs/rules/security/isolates/#isolate-values-cross-by-copy" title="A value crosses an isolate boundary as one graph copy, and a closure, an alias or a host handle does not cross at all"><code>security/isolate-values-cross-by-copy</code></a> <a href="/docs/rules/security/extensions-and-qualifiers/#secret-does-not-cross-an-extension" title="A secret value is refused at every extension boundary, in either direction"><code>security/secret-does-not-cross-an-extension</code></a> <a href="/docs/rules/core-classes/regex-html-and-introspection/#secret-reveal" title="Core\Secret::reveal is the one named way out of secret, and it carries a written reason"><code>core-classes/secret-reveal</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0033.md">record 0033</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0023.md">record 0023</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0116.md">record 0116</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0006.md">record 0006</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-secret-value-does-not-cross-a-boundary.nvst"><code>tests/conformance/reject/a-secret-value-does-not-cross-a-boundary.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-secret-value-cannot-cross-the-graph-copy.nvst"><code>tests/conformance/reject/a-secret-value-cannot-cross-the-graph-copy.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-secret-argument-is-refused-where-a-tainted-one-is-admitted.nvst"><code>tests/conformance/reject/a-secret-argument-is-refused-where-a-tainted-one-is-admitted.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="secret-comparison-is-constant-time">

## Comparing two `secret` values with `==` lowers to a constant-time helper, decided by the compiler

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#secret-comparison-is-constant-time"><code>security/secret-comparison-is-constant-time</code></a>
</div>

`$provided == $expected`, where both operands are statically `secret`, lowers to a **constant-time**
comparison helper rather than the short-circuiting one every other operand pair uses. Nothing new is
spelled: `==` is the only equality operator ([`expressions/one-equality-operator`](/docs/rules/expressions/truthiness-and-equality/#one-equality-operator "== and != are the whole of equality, and ===, !== and <> do not parse")), the qualifier
is already known at the comparison, and the lowering picks the helper. Where exactly one operand is
`secret`, the qualifier has already poisoned the other
([`security/secret-propagation`](/docs/rules/security/secrets/#secret-propagation "A secret operand poisons its own axis independently of tainted, and a checked conversion strips it")), so the pair is `secret` and the rule applies. That includes the
other side arriving as a `mixed` — a decoded request field, a header, a cache read — where the helper
reads the tag: a `string` or `bytes` payload of the side's own tag is compared in constant time, and
every other tag answers `false`, which is what the short-circuiting row answers for the same pair.

The gap it closes is narrow and real. Constant-time comparison is guaranteed inside the protocol
roster, and a dedicated equality member is available to anyone who knows to reach for it — but a
program comparing its own session token, API key or signature with `==` sits outside both, is a timing
oracle, and receives no diagnostic anywhere.

The cost, stated: roughly eight extra nanoseconds per comparison of two 32-byte values, invisible at
any scale a request reaches, and the comparison cannot early-exit, which is the entire point.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>$a == $b</code> over two credentials is not a timing oracle, and no call site has to remember <code>hash_equals</code></p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/secrets/#secret-qualifier" title="secret is a second, independent compile-time qualifier, written before tainted and in that order alone"><code>security/secret-qualifier</code></a> <a href="/docs/rules/security/protocols-and-tokens/#verification-throws-and-compares-in-constant-time" title="Verification answers a value or throws, and no member exposes a raw comparison for the caller to get wrong"><code>security/verification-throws-and-compares-in-constant-time</code></a> <a href="/docs/rules/expressions/truthiness-and-equality/#one-equality-operator" title="== and != are the whole of equality, and ===, !== and &lt;&gt; do not parse"><code>expressions/one-equality-operator</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0033.md">record 0033</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0090.md">record 0090</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0060.md">record 0060</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/comparing-two-secrets-answers-the-same-as-comparing-two-strings.nvst"><code>tests/conformance/lang/comparing-two-secrets-answers-the-same-as-comparing-two-strings.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/a-secret-compared-against-a-mixed-is-constant-time.nvst"><code>tests/conformance/lang/a-secret-compared-against-a-mixed-is-constant-time.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="secret-in-a-test-report">

## A `secret` operand is compared but never rendered, and no flag lifts the redaction

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#secret-in-a-test-report"><code>security/secret-in-a-test-report</code></a>
</div>

An assertion diff is simultaneously output, a log line, a dump and a `Throwable` message — all four of
the things a `secret` value may not become ([`security/secret-sinks-refuse`](/docs/rules/security/secrets/#secret-sinks-refuse "Every output sink refuses a secret value outright, and no auto-escape or neutralisation bypasses one")). The assertion still
runs; the **report is redacted, always**, with no flag and no build mode that lifts it, and the
failure object's own diff is redacted too, so catching the failure does not recover the value.

Byte length is reported, because the length is not the secret. Debugging a redacted failure is
genuinely harder, and the intended answer is to assert on a derived value — a digest of the token
rather than the token.

Separately and always, the reporter escapes control characters in every value it renders. A `tainted`
string is not confidential and may be shown, but a fixture full of terminal escapes must not be able
to rewrite the developer's terminal from inside a failure message.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/secrets/#secret-sinks-refuse" title="Every output sink refuses a secret value outright, and no auto-escape or neutralisation bypasses one"><code>security/secret-sinks-refuse</code></a> <a href="/docs/rules/testing/writing-a-test/#failure-ledger" title="A failed assertion is a catchable Throwable and a ledger entry the test cannot erase"><code>testing/failure-ledger</code></a> <a href="/docs/rules/errors/diagnostics-and-logging/#record-transformations" title="Redaction, control bytes, bidi and elision are decided in the record"><code>errors/record-transformations</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0079.md">record 0079</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0033.md">record 0033</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0092.md">record 0092</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/test-a-secret-property-is-redacted-but-still-compared.nvst"><code>tests/conformance/core/test-a-secret-property-is-redacted-but-still-compared.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/test-assert-matches-inline-never-holds-a-secret-property.nvst"><code>tests/conformance/core/test-assert-matches-inline-never-holds-a-secret-property.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="redaction-reaches-the-tools-own-renderings">

## No rendering the toolchain itself produces prints a `secret` in the clear, the AST dump included

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#redaction-reaches-the-tools-own-renderings"><code>security/redaction-reaches-the-tools-own-renderings</code></a>
</div>

A `secret` value may not become output, a log line, a dump or a `Throwable` message
([`security/secret-sinks-refuse`](/docs/rules/security/secrets/#secret-sinks-refuse "Every output sink refuses a secret value outright, and no auto-escape or neutralisation bypasses one")), and the toolchain is held to the bar it enforces on everyone
else: no command this project ships prints one in the clear. The diagnostic record already carries
the redaction as a node kind, which is what makes plaintext, JSON and HTML renderings agree from one
place ([`errors/record-transformations`](/docs/rules/errors/diagnostics-and-logging/#record-transformations "Redaction, control bytes, bidi and elision are decided in the record")).

The AST dump is the surface that would otherwise disagree: a node's own scalar fields include a string
literal's text, so anything reading the dump would render a value every other rendering redacts. A
literal node whose static type carries `secret` emits the same fixed placeholder a dumped property
gets, **in the JSON itself** rather than in whatever displays it, so two readers of one dump cannot
diverge.

The cost is that a frozen dump schema now has a type-dependent field value, and a reader can no longer
assume a literal node's text is the source text.

This rule is about what the toolchain **prints**, and it is enforced by the compiler for every caller.
What an editor draws over a buffer it did not print is a separate mechanism in a separate chapter
([`ide/redaction-ranges-come-from-the-server`](/docs/rules/ide/security-in-the-editor/#redaction-ranges-come-from-the-server "The editor conceals a secret value by default, and the ranges come from the language server rather than a client guess")), and neither one covers for the other: the
placeholder here holds with no editor running, and no decoration anywhere puts a value back into a
dump this rule has already redacted.

**Not on disk.** The dump emits no such placeholder.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/secrets/#secret-sinks-refuse" title="Every output sink refuses a secret value outright, and no auto-escape or neutralisation bypasses one"><code>security/secret-sinks-refuse</code></a> <a href="/docs/rules/errors/diagnostics-and-logging/#record-transformations" title="Redaction, control bytes, bidi and elision are decided in the record"><code>errors/record-transformations</code></a> <a href="/docs/rules/ide/security-in-the-editor/#redaction-ranges-come-from-the-server" title="The editor conceals a secret value by default, and the ranges come from the language server rather than a client guess"><code>ide/redaction-ranges-come-from-the-server</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0101.md">record 0101</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0033.md">record 0033</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0099.md">record 0099</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0092.md">record 0092</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0158.md">record 0158</a></dd></div></dl>

</div>
