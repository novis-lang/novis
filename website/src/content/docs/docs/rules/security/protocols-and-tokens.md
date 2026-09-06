---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Protocols, tokens and CSRF"
description: "One TLS client, a closed protocol roster, an algorithm that comes from the key, and CSRF on by default."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/security/extensions-and-qualifiers/
  label: "Qualifiers across an extension"
next:
  link: /docs/rules/security/bidi-and-passwords/
  label: "Bidirectional text and password hashes"
---

<p class="nv-section-lead">One TLS client, a closed protocol roster, an algorithm that comes from the key, and CSRF on by default.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">9</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">7</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">2</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">3</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#one-tls-client">There is one TLS client in the tree, and a driver reaches it over a generic transport rather than building a second</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#protocol-roster">A closed roster of application-layer security protocols lives in <code>Core</code>, and nothing joins it without meeting the test</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#protocol-admission-test">Three conditions and one boundary decide what may join the roster, and a flow never does</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#algorithm-comes-from-the-key">The algorithm comes from the key and never from the token</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#jwt-expiry-is-mandatory">A JWT without an expiry, or past one, fails verification, and no flag disables the check</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#verification-throws-and-compares-in-constant-time">Verification answers a value or throws, and no member exposes a raw comparison for the caller to get wrong</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#verification-does-not-launder">A verified signature proves origin, not safety: claims come back <code>tainted</code></a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#csrf-is-on-by-default">The server refuses an unsafe verb without a valid CSRF token, and the opt-out is written on the route itself</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#access-is-checked-for-presence-not-meaning">The compiler proves a route's access decision was written, never that it was honoured</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li></ol>

<div class="nv-rule" id="one-tls-client">

## There is one TLS client in the tree, and a driver reaches it over a generic transport rather than building a second

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#one-tls-client"><code>security/one-tls-client</code></a>
</div>

There is one TLS client in the tree, with one answer to "whose certificates do you believe" and one
place a configured anchor bundle plugs into. A driver that needs a handshake tunnelled inside its own
framing reaches that client over a **generic transport** — an ordinary read/write adapter — rather
than building a second session of its own.

A second TLS session inside a driver crate would be a second answer to a question already decided at
length, and the failure mode is not a compile error: it is one client verifying peers strictly and
another not. Full verification is the default with no spelling for turning it off, so the plaintext
phase of a connection is only ever the upgrade request itself.

What does not generalise is what belongs to the socket rather than to the session — the deadline and
the peer address — so there is still one clock, on the thing that waits.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/laundering/#db-pool-reset-is-a-boundary" title="A pooled database connection is proven clean before it is reused, and a failed reset destroys it"><code>security/db-pool-reset-is-a-boundary</code></a> <a href="/docs/rules/security/scopes-and-denial/#the-policy-lives-in-the-capability" title="The address policy lives in the capability, so every client obeys it and none of them may hold its own"><code>security/the-policy-lives-in-the-capability</code></a> <a href="/docs/rules/core-classes/connecting-to-a-database/#db-safe-connection-defaults" title="Three connection defaults close holes PHP leaves open, and none can be configured to the unsafe value"><code>core-classes/db-safe-connection-defaults</code></a> <a href="/docs/rules/core-classes/running-a-statement/#db-crate-boundary" title="The wire lives in nvs-db below the standard library, where a codec is borrowed and a state machine is written"><code>core-classes/db-crate-boundary</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0132.md">record 0132</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0058.md">record 0058</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-db/tests/handshake.rs"><code>crates/nvs-db/tests/handshake.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="protocol-roster">

## A closed roster of application-layer security protocols lives in `Core`, and nothing joins it without meeting the test

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#protocol-roster"><code>security/protocol-roster</code></a>
</div>

A closed roster of application-layer security protocols lives in `Core`: signed and encrypted cookies
(authenticated encryption only, with key rotation), CSRF tokens, TOTP, JWT, and detached signatures
over a canonical payload. Nothing joins it without meeting the admission test
([`security/protocol-admission-test`](/docs/rules/security/protocols-and-tokens/#protocol-admission-test "Three conditions and one boundary decide what may join the roster, and a flow never does")).

They live in `Core` rather than in an extension for two reasons, and the second is structural. Token
verification sits on the hot path of every authenticated request, so a boundary crossing per request
is a real cost paid universally. And a `secret` value may not cross an extension boundary
([`security/secret-does-not-cross-an-extension`](/docs/rules/security/extensions-and-qualifiers/#secret-does-not-cross-an-extension "A secret value is refused at every extension boundary, in either direction")), so an extension-based signer would need a
reveal at every call site — turning a deliberately conspicuous escape hatch into boilerplate, which
destroys its value as a signal. A userland implementation is still possible and cannot be prevented;
the claim is not exclusivity but that the obvious, documented option is the correct one.

**Four of the five are on disk** — signed cookies, CSRF, TOTP and JWT, each with conformance cases.
`Core\Signature` is not: no member, no module, no case. The roster is therefore not yet the closed set
this rule describes.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/protocols-and-tokens/#protocol-admission-test" title="Three conditions and one boundary decide what may join the roster, and a flow never does"><code>security/protocol-admission-test</code></a> <a href="/docs/rules/security/protocols-and-tokens/#algorithm-comes-from-the-key" title="The algorithm comes from the key and never from the token"><code>security/algorithm-comes-from-the-key</code></a> <a href="/docs/rules/security/protocols-and-tokens/#verification-does-not-launder" title="A verified signature proves origin, not safety: claims come back tainted"><code>security/verification-does-not-launder</code></a> <a href="/docs/rules/security/extensions-and-qualifiers/#secret-does-not-cross-an-extension" title="A secret value is refused at every extension boundary, in either direction"><code>security/secret-does-not-cross-an-extension</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0060.md">record 0060</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0033.md">record 0033</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0055.md">record 0055</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0146.md">record 0146</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/csrf-binds-a-token-to-one-session-and-verify-is-the-only-comparison.nvst"><code>tests/conformance/core/csrf-binds-a-token-to-one-session-and-verify-is-the-only-comparison.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/signed-cookie-seals-with-the-newest-key-and-opens-against-the-ring.nvst"><code>tests/conformance/core/signed-cookie-seals-with-the-newest-key-and-opens-against-the-ring.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/jwt-sign-and-verify-agree-over-a-sweep-of-claim-shapes.nvst"><code>tests/conformance/core/jwt-sign-and-verify-agree-over-a-sweep-of-claim-shapes.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/totp-accepts-a-code-once-inside-a-window-with-no-widening-argument.nvst"><code>tests/conformance/core/totp-accepts-a-code-once-inside-a-window-with-no-widening-argument.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="protocol-admission-test">

## Three conditions and one boundary decide what may join the roster, and a flow never does

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#protocol-admission-test"><code>security/protocol-admission-test</code></a>
</div>

Three conditions must all hold for a protocol to belong in `Core`. **The failure mode is a library bug
rather than an application bug** — the mistake lives in the implementation of the protocol, not in how
the application uses it. **The failure is silent** — a wrong implementation returns a plausible result
rather than an error. **The need is near-universal** for the kind of program Novis exists to run.

One structural boundary decides the edge: **the operation is stateless over a key.** A protocol
requiring network round trips, a redirect dance, or stored per-flow state is a *flow*, not a token
operation. The major delegated-authentication and hardware-credential protocols are all flows: they
need clients, discovery documents, nonce storage and expiry policy, and their designs vary per
provider. They are out permanently, and are ordinary userland or package code built on this roster.

The closed list will be argued with, and this test is the answer — recording the boundary is what
makes it a decision rather than a negotiation.

**Not on disk.** Nothing enforces the test; it governs what a future member may be.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/protocols-and-tokens/#protocol-roster" title="A closed roster of application-layer security protocols lives in Core, and nothing joins it without meeting the test"><code>security/protocol-roster</code></a> <a href="/docs/rules/security/protocols-and-tokens/#algorithm-comes-from-the-key" title="The algorithm comes from the key and never from the token"><code>security/algorithm-comes-from-the-key</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0060.md">record 0060</a></dd></div></dl>

</div>

<div class="nv-rule" id="algorithm-comes-from-the-key">

## The algorithm comes from the key and never from the token

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#algorithm-comes-from-the-key"><code>security/algorithm-comes-from-the-key</code></a>
</div>

The algorithm comes from the key, never from the token. A token's declared algorithm header is
*checked against* the key's algorithm and rejected on mismatch; it is never consulted to select one.

That removes the "no signature" family and the asymmetric-to-symmetric confusion class in one stroke,
because there is **no code path in which an attacker-supplied string selects a verifier**. It is not a
check that could be forgotten at a call site — the caller does not choose an algorithm, so there is
nothing for the caller to get wrong.

The same shape runs through the roster: a key of the wrong kind is a verdict on the token, while a
value that is not a key at all is a bug in the program, and the two are reported differently
([`security/verification-throws-and-compares-in-constant-time`](/docs/rules/security/protocols-and-tokens/#verification-throws-and-compares-in-constant-time "Verification answers a value or throws, and no member exposes a raw comparison for the caller to get wrong")).

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>alg: none</code> and the RS256-to-HS256 confusion have no code path, because no attacker-supplied string selects a verifier</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/protocols-and-tokens/#jwt-expiry-is-mandatory" title="A JWT without an expiry, or past one, fails verification, and no flag disables the check"><code>security/jwt-expiry-is-mandatory</code></a> <a href="/docs/rules/security/protocols-and-tokens/#verification-throws-and-compares-in-constant-time" title="Verification answers a value or throws, and no member exposes a raw comparison for the caller to get wrong"><code>security/verification-throws-and-compares-in-constant-time</code></a> <a href="/docs/rules/security/protocols-and-tokens/#protocol-roster" title="A closed roster of application-layer security protocols lives in Core, and nothing joins it without meeting the test"><code>security/protocol-roster</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0060.md">record 0060</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/jwt-refuses-every-unverifiable-token-with-one-sentence.nvst"><code>tests/conformance/core/jwt-refuses-every-unverifiable-token-with-one-sentence.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/csrf-a-wrong-key-is-a-verdict-and-a-non-key-is-a-bug.nvst"><code>tests/conformance/core/csrf-a-wrong-key-is-a-verdict-and-a-non-key-is-a-bug.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="jwt-expiry-is-mandatory">

## A JWT without an expiry, or past one, fails verification, and no flag disables the check

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#jwt-expiry-is-mandatory"><code>security/jwt-expiry-is-mandatory</code></a>
</div>

A JWT without an expiry, or past one, fails verification. There is no flag to disable the check and no
way for a caller to set an unbounded lifetime: a caller wanting a non-expiring credential is not using
JWT for what JWT is.

**This is JWT's rule, not the roster's.** Its reasoning is about what a JWT is for, and the signed
cookie has carried no lifetime since it landed. Where a lifetime is genuinely a parameter — a detached
signature over a payload — it is *written and never omitted*, `null` included, which is a different
answer to a different question and is why the two are not one rule.

Expiry being unrepresentably absent is what makes a leaked token bounded rather than permanent, which
is the failure the historical implementations kept shipping ([`security/algorithm-comes-from-the-key`](/docs/rules/security/protocols-and-tokens/#algorithm-comes-from-the-key "The algorithm comes from the key and never from the token")).

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/protocols-and-tokens/#algorithm-comes-from-the-key" title="The algorithm comes from the key and never from the token"><code>security/algorithm-comes-from-the-key</code></a> <a href="/docs/rules/security/protocols-and-tokens/#protocol-roster" title="A closed roster of application-layer security protocols lives in Core, and nothing joins it without meeting the test"><code>security/protocol-roster</code></a> <a href="/docs/rules/security/protocols-and-tokens/#verification-throws-and-compares-in-constant-time" title="Verification answers a value or throws, and no member exposes a raw comparison for the caller to get wrong"><code>security/verification-throws-and-compares-in-constant-time</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0060.md">record 0060</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0146.md">record 0146</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/jwt-signs-with-an-expiry-the-caller-cannot-set-and-verify-answers-every-claim.nvst"><code>tests/conformance/core/jwt-signs-with-an-expiry-the-caller-cannot-set-and-verify-answers-every-claim.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/jwt-a-token-verifies-for-its-whole-lifetime-and-not-one-second-past-it.nvst"><code>tests/conformance/core/jwt-a-token-verifies-for-its-whole-lifetime-and-not-one-second-past-it.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="verification-throws-and-compares-in-constant-time">

## Verification answers a value or throws, and no member exposes a raw comparison for the caller to get wrong

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#verification-throws-and-compares-in-constant-time"><code>security/verification-throws-and-compares-in-constant-time</code></a>
</div>

Verification returns claims or throws. It never returns a falsy value that a loose comparison could
mistake for success, which is the same reasoning a budget exhaustion gets: a result that can be
misread as a result is worse than an error.

**Every comparison of a secret-derived value is constant-time, and no member exposes the raw value for
the caller to compare themselves.** Where a token is compared, the comparison *is* the exposed
operation, so a caller cannot write `==` over it by accident — and where a program compares its own
credentials, the operator is right by construction anyway
([`security/secret-comparison-is-constant-time`](/docs/rules/security/secrets/#secret-comparison-is-constant-time "Comparing two secret values with == lowers to a constant-time helper, decided by the compiler")).

A refusal says one sentence and the same sentence for every forgery, so the failure carries no oracle
about which part of the token was wrong.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>No verifier returns a falsy value a loose comparison could read as success</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/secrets/#secret-comparison-is-constant-time" title="Comparing two secret values with == lowers to a constant-time helper, decided by the compiler"><code>security/secret-comparison-is-constant-time</code></a> <a href="/docs/rules/security/protocols-and-tokens/#algorithm-comes-from-the-key" title="The algorithm comes from the key and never from the token"><code>security/algorithm-comes-from-the-key</code></a> <a href="/docs/rules/security/protocols-and-tokens/#protocol-roster" title="A closed roster of application-layer security protocols lives in Core, and nothing joins it without meeting the test"><code>security/protocol-roster</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0060.md">record 0060</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0033.md">record 0033</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0056.md">record 0056</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/csrf-binds-a-token-to-one-session-and-verify-is-the-only-comparison.nvst"><code>tests/conformance/core/csrf-binds-a-token-to-one-session-and-verify-is-the-only-comparison.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/csrf-refuses-every-single-character-mutation-of-a-token-without-throwing.nvst"><code>tests/conformance/core/csrf-refuses-every-single-character-mutation-of-a-token-without-throwing.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/signed-cookie-open-refuses-every-forgery-with-one-message.nvst"><code>tests/conformance/core/signed-cookie-open-refuses-every-forgery-with-one-message.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="verification-does-not-launder">

## A verified signature proves origin, not safety: claims come back `tainted`

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#verification-does-not-launder"><code>security/verification-does-not-launder</code></a>
</div>

Claims returned from token verification are **`tainted`**. A signature proves origin, not safety for
any sink: the payload may be authored by a third-party issuer, and even a self-issued token routinely
carries user-supplied data. Treating verification as laundering would be exactly the false confidence
refused everywhere else ([`security/launderers-are-sink-named`](/docs/rules/security/laundering/#launderers-are-sink-named "The only way out of tainted is a Core member whose contract names the one sink it is safe for")).

Cookie payloads the application itself sealed are the one case where the value round-trips through our
own authenticated encryption unchanged, and they come back **unqualified** — sealing is an
authenticated operation over a value that was already plain when it went in. The asymmetry between the
two is the rule, not an inconsistency: one is a value we had, the other is a value we were handed.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/laundering/#launderers-are-sink-named" title="The only way out of tainted is a Core member whose contract names the one sink it is safe for"><code>security/launderers-are-sink-named</code></a> <a href="/docs/rules/security/tainted-data/#tainted-sources" title="Every accessor that hands a program bytes from outside answers the tainted form, and the list of them is enumerable"><code>security/tainted-sources</code></a> <a href="/docs/rules/security/protocols-and-tokens/#protocol-roster" title="A closed roster of application-layer security protocols lives in Core, and nothing joins it without meeting the test"><code>security/protocol-roster</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0060.md">record 0060</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/signed-cookie-open-launders-where-jwt-verify-does-not.nvst"><code>tests/conformance/core/signed-cookie-open-launders-where-jwt-verify-does-not.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="csrf-is-on-by-default">

## The server refuses an unsafe verb without a valid CSRF token, and the opt-out is written on the route itself

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#csrf-is-on-by-default"><code>security/csrf-is-on-by-default</code></a>
</div>

The server refuses an unsafe verb — the four that change state — without a valid token, using the
match it already made to know which handler is which. Generation and constant-time verification belong
to the protocol roster ([`security/protocol-roster`](/docs/rules/security/protocols-and-tokens/#protocol-roster "A closed roster of application-layer security protocols lives in Core, and nothing joins it without meeting the test")); this decides only the default, and the
default is on with nothing configured.

A route that legitimately needs no token — a webhook authenticated by signature, an API authenticated
by a bearer token — says so **in its own declaration**, per route, in the attribute that route already
carries: never a group and never a configuration key. Writing it on the route is what keeps the
exemption reviewable next to the handler it exempts.

Enforcing only when a session cookie is present was rejected within this rule. It sounds tighter,
since forgery can only target cookie-authenticated requests, but it makes protection depend on what
the client sent rather than on what the code says, which is harder to reason about and harder to test.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A deployment with nothing configured already rejects a forged <code>POST</code></p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/protocols-and-tokens/#access-is-checked-for-presence-not-meaning" title="The compiler proves a route's access decision was written, never that it was honoured"><code>security/access-is-checked-for-presence-not-meaning</code></a> <a href="/docs/rules/security/protocols-and-tokens/#protocol-roster" title="A closed roster of application-layer security protocols lives in Core, and nothing joins it without meeting the test"><code>security/protocol-roster</code></a> <a href="/docs/rules/attributes/#access-payload" title="#[Access] names one resolvable decision in allow, and one of them covers a method"><code>attributes/access-payload</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0096.md">record 0096</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0102.md">record 0102</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0060.md">record 0060</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0074.md">record 0074</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-server/src/route.rs"><code>crates/nvs-server/src/route.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/csrf-binds-a-token-to-one-session-and-verify-is-the-only-comparison.nvst"><code>tests/conformance/core/csrf-binds-a-token-to-one-session-and-verify-is-the-only-comparison.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/http-method-is-eight-verbs-whose-unsafe-four-are-the-tail.nvst"><code>tests/conformance/core/http-method-is-eight-verbs-whose-unsafe-four-are-the-tail.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="access-is-checked-for-presence-not-meaning">

## The compiler proves a route's access decision was written, never that it was honoured

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#access-is-checked-for-presence-not-meaning"><code>security/access-is-checked-for-presence-not-meaning</code></a>
</div>

The compiler verifies that a route's access attribute is there and that the name inside it resolves.
It never asks what the name means, never calls anything, and has no opinion about roles, policies or
sessions. An omission is an error rather than a public default — that half is
[`attributes/access-is-a-required-sibling`](/docs/rules/attributes/#access-is-a-required-sibling "A method carrying #[Route] carries #[Access], and an omission does not compile"), and the payload's shape is
[`attributes/access-payload`](/docs/rules/attributes/#access-payload "#[Access] names one resolvable decision in allow, and one of them covers a method").

**Interpretation belongs to whoever dispatches.** The declared name rides on the match the server made
as uninterpreted data, and the application's own dispatch reads and enforces it. **The server enforces
CSRF and nothing else** ([`security/csrf-is-on-by-default`](/docs/rules/security/protocols-and-tokens/#csrf-is-on-by-default "The server refuses an unsafe verb without a valid CSRF token, and the opt-out is written on the route itself")): interpreting a role would need a
session, a user model and a role source, all three of which are deliberately outside the binary. Two
enforcement points is how a route ends up checked twice in development and not at all in production,
so there is one.

**The limit is stated rather than implied: the compiler guarantees the decision was *written*, not
that it was *honoured*.** An application that hand-rolls dispatch and never reads the access name gets
no enforcement from anyone. Closing that would require recognising a dispatch site, which is an
opinion the route table refuses to hold.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/attributes/#access-is-a-required-sibling" title="A method carrying #[Route] carries #[Access], and an omission does not compile"><code>attributes/access-is-a-required-sibling</code></a> <a href="/docs/rules/attributes/#access-payload" title="#[Access] names one resolvable decision in allow, and one of them covers a method"><code>attributes/access-payload</code></a> <a href="/docs/rules/security/protocols-and-tokens/#csrf-is-on-by-default" title="The server refuses an unsafe verb without a valid CSRF token, and the opt-out is written on the route itself"><code>security/csrf-is-on-by-default</code></a> <a href="/docs/rules/security/laundering/#route-capture-is-laundered-by-its-type" title="A route capture is laundered by the parameter's own type, and a string capture stays tainted"><code>security/route-capture-is-laundered-by-its-type</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0096.md">record 0096</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0102.md">record 0102</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0077.md">record 0077</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0082.md">record 0082</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-stray-access-attribute-is-refused.nvst"><code>tests/conformance/reject/a-stray-access-attribute-is-refused.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-matched-routes-name-is-the-units-own-and-its-captures-are-the-peers.nvst"><code>tests/conformance/core/a-matched-routes-name-is-the-units-own-and-its-captures-are-the-peers.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-server/src/route.rs"><code>crates/nvs-server/src/route.rs</code></a></dd></div></dl>

</div>
