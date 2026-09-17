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

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">12</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">10</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">2</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">6</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#one-tls-client">There is one TLS client in the tree, and a driver reaches it over a generic transport rather than building a second</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#tls-trust-is-relaxed-only-under-a-host-grant">Verification is relaxed only where a <code>capabilities.tls</code> grant names the host and the call asks for it, and no such grant has a <code>true</code> spelling</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#protocol-roster">A closed roster of application-layer security protocols lives in <code>Core</code>, and nothing joins it without meeting the test</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#protocol-admission-test">Three conditions and one boundary decide what may join the roster, and a flow never does</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#algorithm-comes-from-the-key">The algorithm comes from the key and never from the token</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#jwt-expiry-is-mandatory">A JWT without an expiry, or past one, fails verification, and no flag disables the check</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#verification-throws-and-compares-in-constant-time">Verification answers a value or throws, and no member exposes a raw comparison for the caller to get wrong</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#verification-does-not-launder">A verified signature proves origin, not safety: claims come back <code>tainted</code></a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#jwe-compact-subset"><code>Core\Jwe</code> speaks compact JWE with <code>A256GCM</code> alone, and the static that built the key picks the algorithm</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#jws-issued-subset">A token another party issued verifies against a key found by <code>kid</code>, never one that is tried, and its claims come back as a <code>tainted</code> shape</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#csrf-is-on-by-default">The server refuses an unsafe verb without a valid CSRF token, and the opt-out is written on the route itself</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#access-is-checked-for-presence-not-meaning">The compiler proves a route's access decision was written, never that it was honoured</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li></ol>

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
another not. Verification is strict by default, and relaxed only where a `capabilities.tls` grant names
the host and the call asks for it ([`security/tls-trust-is-relaxed-only-under-a-host-grant`](/docs/rules/security/protocols-and-tokens/#tls-trust-is-relaxed-only-under-a-host-grant "Verification is relaxed only where a capabilities.tls grant names the host and the call asks for it, and no such grant has a true spelling")) — which
a caller reaches by handing in a policy value the module builds a session from, never a session of its
own. The plaintext phase of a connection is only ever the upgrade request itself.

What does not generalise is what belongs to the socket rather than to the session — the deadline and
the peer address — so there is still one clock, on the thing that waits.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/laundering/#db-pool-reset-is-a-boundary" title="A pooled database connection is proven clean before it is reused, and a failed reset destroys it"><code>security/db-pool-reset-is-a-boundary</code></a> <a href="/docs/rules/security/scopes-and-denial/#the-policy-lives-in-the-capability" title="The address policy lives in the capability, so every client obeys it and none of them may hold its own"><code>security/the-policy-lives-in-the-capability</code></a> <a href="/docs/rules/security/protocols-and-tokens/#tls-trust-is-relaxed-only-under-a-host-grant" title="Verification is relaxed only where a capabilities.tls grant names the host and the call asks for it, and no such grant has a true spelling"><code>security/tls-trust-is-relaxed-only-under-a-host-grant</code></a> <a href="/docs/rules/core-classes/connecting-to-a-database/#db-safe-connection-defaults" title="Three connection defaults close holes PHP leaves open, and none can be configured to the unsafe value"><code>core-classes/db-safe-connection-defaults</code></a> <a href="/docs/rules/core-classes/running-a-statement/#db-crate-boundary" title="The wire lives in nvs-db below the standard library, where a codec is borrowed and a state machine is written"><code>core-classes/db-crate-boundary</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0132.md">record 0132</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0067.md">record 0067</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0058.md">record 0058</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0180.md">record 0180</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-db/tests/handshake.rs"><code>crates/nvs-db/tests/handshake.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="tls-trust-is-relaxed-only-under-a-host-grant">

## Verification is relaxed only where a `capabilities.tls` grant names the host and the call asks for it, and no such grant has a `true` spelling

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#tls-trust-is-relaxed-only-under-a-host-grant"><code>security/tls-trust-is-relaxed-only-under-a-host-grant</code></a>
</div>

Certificate verification on an outbound call is strict by default and is relaxed only when two
independent things agree: a `[capabilities.tls]` grant names the host, and the call itself asks. Four
options, four grants:

| Option | Grant | Does |
|---|---|---|
| `tlsCa: string` | `tls.anchors` | trusts exactly the PEM certificates given, for this call, in place of `[http.client.tls] roots` |
| `tlsPin: string\|array<string>` | `tls.pin` | accepts a peer whose SubjectPublicKeyInfo hashes to one of the `sha256//<base64>` values, with no chain |
| `tlsVerifyHost: false` | `tls.any_name` | builds and checks the chain, and skips only the name |
| `tlsVerify: false` | `tls.insecure` | checks neither |

**No grant has a `true` spelling.** Each is a list of hosts, matched as `net.connect` matches them, for
`net.internal`'s reason: what a deployment relaxed stays legible host by host in review, where a boolean
is one line nobody reads again. The grant and the option are both owed — a grant changes nothing about a
call that does not ask, and an option whose host the grant does not list throws before a connection is
made, naming the grant, asked of the request's own configuration snapshot per
[`security/capability-question-is-grant-and-scope`](/docs/rules/security/capabilities/#capability-question-is-grant-and-scope "A capability question is a grant and a scope, asked of the request's own configuration snapshot"). Two halves because each answers a different
party's question: the deployment says *where* this may happen, the code says *here*.

A relaxed session still checks the handshake signature against the key the peer presented, so the peer
does hold that key; what is skipped is only the question of whose key it is. The verifiers plug into
`rustls`'s custom-verifier seam inside `nvs_host::tls`, so [`security/one-tls-client`](/docs/rules/security/protocols-and-tokens/#one-tls-client "There is one TLS client in the tree, and a driver reaches it over a generic transport rather than building a second") still holds and
a caller still cannot hand in a session — it hands in a policy value the module builds one from, and that
policy is part of the pool key
([`http-server/an-outbound-connection-is-pooled-per-core-and-stays-pinned`](/docs/rules/http-server/the-http-client/#an-outbound-connection-is-pooled-per-core-and-stays-pinned "An outbound connection is pooled per core under a key carrying everything the check approved, and returns to the pool only after a reply read to the end under known framing")), so a connection opened
under `tlsVerify: false` never serves a call that verifies. `tlsMinVersion: "1.3"` needs no grant,
because it can only tighten. And the boot prints every relaxed grant, every start, one line per grant and
host: a weakening nobody is reminded of outlives the incident it was added for.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>CURLOPT_SSL_VERIFYPEER = false</code> is one line of userland; here the same effect needs the deployment to name the host in a grant and the call to ask for it, and the boot prints every grant that was written</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/protocols-and-tokens/#one-tls-client" title="There is one TLS client in the tree, and a driver reaches it over a generic transport rather than building a second"><code>security/one-tls-client</code></a> <a href="/docs/rules/security/capabilities/#capability-question-is-grant-and-scope" title="A capability question is a grant and a scope, asked of the request's own configuration snapshot"><code>security/capability-question-is-grant-and-scope</code></a> <a href="/docs/rules/http-server/the-http-client/#the-client-trust-roots-are-the-operators" title="[http.client.tls] is the operator's alone: the bundled roots unless files are named, TLS 1.2 unless the floor is raised, and no key log in production"><code>http-server/the-client-trust-roots-are-the-operators</code></a> <a href="/docs/rules/http-server/the-http-client/#an-outbound-connection-is-pooled-per-core-and-stays-pinned" title="An outbound connection is pooled per core under a key carrying everything the check approved, and returns to the pool only after a reply read to the end under known framing"><code>http-server/an-outbound-connection-is-pooled-per-core-and-stays-pinned</code></a> <a href="/docs/rules/http-server/the-http-client/#a-reply-reports-its-tls-session" title="Response::tls() reports the session a reply arrived over — version, cipher, whether the peer was verified, and its chain — and is null when there was none"><code>http-server/a-reply-reports-its-tls-session</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0180.md">record 0180</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/src/capability.rs"><code>crates/nvs-config/src/capability.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/tests/capability.rs"><code>crates/nvs-config/tests/capability.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/http/transport.rs"><code>crates/nvs-stdlib/src/http/transport.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-host/src/tls.rs"><code>crates/nvs-host/src/tls.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="protocol-roster">

## A closed roster of application-layer security protocols lives in `Core`, and nothing joins it without meeting the test

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#protocol-roster"><code>security/protocol-roster</code></a>
</div>

A closed roster of application-layer security protocols lives in `Core`: signed and encrypted cookies
(authenticated encryption only, with key rotation), CSRF tokens, TOTP, JWT — signed and verified under a
shared key, and verified when another party issued it ([`security/jws-issued-subset`](/docs/rules/security/protocols-and-tokens/#jws-issued-subset "A token another party issued verifies against a key found by kid, never one that is tried, and its claims come back as a tainted shape")) — JWE
([`security/jwe-compact-subset`](/docs/rules/security/protocols-and-tokens/#jwe-compact-subset "Core\Jwe speaks compact JWE with A256GCM alone, and the static that built the key picks the algorithm")), and detached signatures over a canonical payload. Nothing joins it
without meeting the admission test ([`security/protocol-admission-test`](/docs/rules/security/protocols-and-tokens/#protocol-admission-test "Three conditions and one boundary decide what may join the roster, and a flow never does")).

They live in `Core` rather than in an extension for two reasons, and the second is structural. Token
verification sits on the hot path of every authenticated request, so a boundary crossing per request
is a real cost paid universally. And a `secret` value may not cross an extension boundary
([`security/secret-does-not-cross-an-extension`](/docs/rules/security/extensions-and-qualifiers/#secret-does-not-cross-an-extension "A secret value is refused at every extension boundary, in either direction")), so an extension-based signer would need a
reveal at every call site — turning a deliberately conspicuous escape hatch into boilerplate, which
destroys its value as a signal. A userland implementation is still possible and cannot be prevented;
the claim is not exclusivity but that the obvious, documented option is the correct one.

**Every entry above is registered and carries conformance cases**, `Jwt::verifyIssued` included —
so a token another party issued is verified against a `Jwt\KeySet` here rather than in userland.
What the roster still owes is not a member but the flow around one: fetching, caching and rotating
a key set are a package's job ([`security/protocol-admission-test`](/docs/rules/security/protocols-and-tokens/#protocol-admission-test "Three conditions and one boundary decide what may join the roster, and a flow never does")), and every member here is
stateless over the key it is handed.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/protocols-and-tokens/#protocol-admission-test" title="Three conditions and one boundary decide what may join the roster, and a flow never does"><code>security/protocol-admission-test</code></a> <a href="/docs/rules/security/protocols-and-tokens/#algorithm-comes-from-the-key" title="The algorithm comes from the key and never from the token"><code>security/algorithm-comes-from-the-key</code></a> <a href="/docs/rules/security/protocols-and-tokens/#verification-does-not-launder" title="A verified signature proves origin, not safety: claims come back tainted"><code>security/verification-does-not-launder</code></a> <a href="/docs/rules/security/extensions-and-qualifiers/#secret-does-not-cross-an-extension" title="A secret value is refused at every extension boundary, in either direction"><code>security/secret-does-not-cross-an-extension</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0060.md">record 0060</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0033.md">record 0033</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0055.md">record 0055</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0146.md">record 0146</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0179.md">record 0179</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/csrf-binds-a-token-to-one-session-and-verify-is-the-only-comparison.nvst"><code>tests/conformance/core/csrf-binds-a-token-to-one-session-and-verify-is-the-only-comparison.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/signed-cookie-seals-with-the-newest-key-and-opens-against-the-ring.nvst"><code>tests/conformance/core/signed-cookie-seals-with-the-newest-key-and-opens-against-the-ring.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/jwt-sign-and-verify-agree-over-a-sweep-of-claim-shapes.nvst"><code>tests/conformance/core/jwt-sign-and-verify-agree-over-a-sweep-of-claim-shapes.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/totp-accepts-a-code-once-inside-a-window-with-no-widening-argument.nvst"><code>tests/conformance/core/totp-accepts-a-code-once-inside-a-window-with-no-widening-argument.nvst</code></a></dd></div></dl>

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

<div class="nv-rule" id="jwe-compact-subset">

## `Core\Jwe` speaks compact JWE with `A256GCM` alone, and the static that built the key picks the algorithm

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#jwe-compact-subset"><code>security/jwe-compact-subset</code></a>
</div>

`Core\Jwe` reads and writes compact JWE with `A256GCM` alone, and the static that built the key is what
picks the key-management algorithm.

[`security/algorithm-comes-from-the-key`](/docs/rules/security/protocols-and-tokens/#algorithm-comes-from-the-key "The algorithm comes from the key and never from the token") applied to encryption: `Jwe\Key::shared` means `dir`,
`Jwe\Key::password` means `PBES2-HS256+A128KW`, and `Jwe\Key::recipient` and `Jwe\Key::own` mean
`ECDH-ES` — direct agreement, no key wrap, empty `apu` and `apv`. The header's `alg` and `enc` are read
**only to be compared**, and a mismatch is a refusal. A named constructor rather than a union because a
union parameter carries no qualifier classification and so refuses every `tainted` argument
([`security/unclassified-parameter-refuses-tainted`](/docs/rules/security/tainted-data/#unclassified-parameter-refuses-tainted "A Core parameter nobody classified refuses a tainted argument, and refuses to ship at all")) — and a password is tainted, whether it came
off a form or out of `Core\Cli::secret`. A union of `secret` arms is otherwise expressible; what
disqualifies it is that the one key kind a human types could not be passed to it, where
`Core\Crypto::deriveKey`'s whole-parameter `secret string` takes that same value today, a classification
being read on a parameter and never on a union's member. What the constructor buys back is that the
algorithm comes from a name the caller wrote rather than from an inference the caller cannot see.

**The allowed protected-header parameters are `alg`, `enc`, `epk`, `p2s`, `p2c`, `kid`, `typ` and
`cty`, and everything else is refused** — `zip` because it is a decompression bomb
([`core-classes/decompression-bound`](/docs/rules/core-classes/uris-and-images/#decompression-bound "Every decompression runs under an output ceiling and a ratio, and there is no spelling for turning either off")), `jku`, `x5u` and `x5c` because they are fetches the token is
talking the program into, `crit` and a `jwk` other than `epk` because they ask the verifier to act on
what the token brought with it. An unknown member in a ciphertext header is either an extension we do
not implement or an attack, with no third reading. The protected header is length-capped before it is
parsed.

**`p2s` and `p2c` are held to the member's own bounds**: PBKDF2's iteration count is refused below
100,000 and above 2,000,000 and its salt below 16 octets, checked before the first HMAC. `p2c` is
attacker-supplied, so without the ceiling one token buys unbounded CPU on the request path.

**`encrypt` writes its header canonically** — members sorted, no whitespace, at every level — which is
what lets it be held to the frozen vector set byte for byte rather than only round-tripped against
itself.

The payload is `string` in and **`tainted string`** out, as `Core\Signature::verify`'s is
([`security/verification-does-not-launder`](/docs/rules/security/protocols-and-tokens/#verification-does-not-launder "A verified signature proves origin, not safety: claims come back tainted")): decrypting proves who wrote it, never that it is safe.
Every refusal is one `RuntimeError` with one sentence. Decrypt's key ring is tried in order, except that
a ring holding a password key holds exactly one key, because every try costs a full derivation and an
attacker choosing the ring's length is the iteration ceiling defeated one layer up.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no <code>zip</code>, no <code>jku</code> and no <code>x5u</code>, so a token cannot talk the verifier into a fetch or a decompression</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/protocols-and-tokens/#algorithm-comes-from-the-key" title="The algorithm comes from the key and never from the token"><code>security/algorithm-comes-from-the-key</code></a> <a href="/docs/rules/security/protocols-and-tokens/#jws-issued-subset" title="A token another party issued verifies against a key found by kid, never one that is tried, and its claims come back as a tainted shape"><code>security/jws-issued-subset</code></a> <a href="/docs/rules/security/protocols-and-tokens/#verification-does-not-launder" title="A verified signature proves origin, not safety: claims come back tainted"><code>security/verification-does-not-launder</code></a> <a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#crypto-interop-tier" title="Core\Crypto's algorithm roster is closed, every algorithm argument is required, and the sealed layout is what a browser reads"><code>core-classes/crypto-interop-tier</code></a> <a href="/docs/rules/core-classes/uris-and-images/#decompression-bound" title="Every decompression runs under an output ceiling and a ratio, and there is no spelling for turning either off"><code>core-classes/decompression-bound</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0179.md">record 0179</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/jwe-opens-every-token-webcrypto-sealed.nvst"><code>tests/conformance/core/jwe-opens-every-token-webcrypto-sealed.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/jwe-a-token-opens-only-under-the-static-that-sealed-it.nvst"><code>tests/conformance/core/jwe-a-token-opens-only-under-the-static-that-sealed-it.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/jwe-refuses-every-unopenable-token-with-one-sentence.nvst"><code>tests/conformance/core/jwe-refuses-every-unopenable-token-with-one-sentence.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/jwe-round-trips-under-every-key-and-answers-a-tainted-payload.nvst"><code>tests/conformance/core/jwe-round-trips-under-every-key-and-answers-a-tainted-payload.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="jws-issued-subset">

## A token another party issued verifies against a key found by `kid`, never one that is tried, and its claims come back as a `tainted` shape

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#jws-issued-subset"><code>security/jws-issued-subset</code></a>
</div>

`Core\Jwt` verifies a token another party issued against a key it **finds** by `kid`, never one it tries,
and answers the claims as a `tainted` shape.

`Jwt::verifyIssued<T>` is the roster's reading half for a token this program did not sign. The algorithm
comes from the key — HS256 under a shared key, and ES256, EdDSA, RS256 or PS256 from a pair or public
key's kind — so the header's `alg` is read only to be compared, as `Core\Jwt` already does.
[`security/algorithm-comes-from-the-key`](/docs/rules/security/protocols-and-tokens/#algorithm-comes-from-the-key "The algorithm comes from the key and never from the token") is the rule; what this adds is what happens either side of
the signature check.

**A key is found, never tried.** `kid` is a lookup into the `Jwt\KeySet` and selects nothing else, and a
token carrying no `kid` verifies only against a set holding exactly one key. There is no try-every-key
loop, so a token costs at most one signature check however large the set is — a bound that matters
because the token's sender chooses the `kid` and RSA verification is the dearest operation in the
roster.

**The header policy is deliberately looser than [`security/jwe-compact-subset`](/docs/rules/security/protocols-and-tokens/#jwe-compact-subset "Core\Jwe speaks compact JWE with A256GCM alone, and the static that built the key picks the algorithm")'s.** `jku`, `x5u`,
`x5c`, `jwk`, `crit`, `b64`, `zip` and `cty` are refused — a key or a fetch the token brings, an
extension, an unencoded or compressed payload, a nested token — and **every other member is ignored**,
because an issuer sends hints such as `x5t` and a verifier refusing them refuses real ID tokens. A JWE
header member we refuse is one we would have had to act on; a JWS header member we ignore cannot select
anything, the key having already decided the algorithm. The token is length-capped before it is parsed.

**The clock and the claims are reached only under a signature that held.** `exp` is required and `nbf` is
checked when present, both under a `leeway` that is `60s` by default and a `LogicError` when negative or
above `5m`. `iss` equals the issuer asked for; `aud` is the audience asked for or a list holding it, and
a list of more than one requires `azp` equal to that audience. A `nonce` asked for is compared in
constant time and an absent one is refused; `typ` is compared case-insensitively with any `application/`
prefix removed; `maxAge` requires an `auth_time` no older than `maxAge + leeway`.

**The order is shape, header policy, key, signature, then the clock and the claims**, and policy and
authenticity are the one `RuntimeError` sentence. Expiry keeps its own message, because only the holder
of a genuinely signed token ever sees it, and a claims refusal names the claim for the same reason.

**The claims come back as a shape, not as flattened text.** `T` is held to
[`security/derived-codec-qualifiers`](/docs/rules/security/laundering/#derived-codec-qualifiers "A derived codec adds no qualifier rule, and a secret field is refused at the declaration rather than silently omitted") at the call site exactly as `Core\Request::jsonAs<T>`'s is — an
inline shape must be `tainted {…}`, a class must declare `tainted` on every text field reachable from
it, and anything else is a diagnostic naming the field. That is how
[`security/verification-does-not-launder`](/docs/rules/security/protocols-and-tokens/#verification-does-not-launder "A verified signature proves origin, not safety: claims come back tainted") is kept here: by the type, rather than by refusing every
structured claim. `Jwt::verify` is untouched and keeps its `array<tainted string>` answer.

**`Jwt\KeySet::read` skips what it has no use for and refuses what is wrong.** It skips a key marked
`use: enc` and a key whose `alg` or kind is off the roster, because a real JWKS carries keys for purposes
we do not serve and refusing the set over one of them makes every rotation an outage. It refuses the
whole document for a private member (`d`, `p`, `q`, `dp`, `dq`, `qi`, `k`), an RSA key under 2048 bits,
two keys under one `kid`, an `alg` its kind cannot carry, more than 16 keys, a body that is not JSON, and
an RSA key carrying no `alg` when no `rsaScheme` was named. Fetching, caching and discovering a key set
are a flow and therefore a package, not `Core` ([`security/protocol-admission-test`](/docs/rules/security/protocols-and-tokens/#protocol-admission-test "Three conditions and one boundary decide what may join the roster, and a flow never does")).

**Structured claims are signed only under a key pair**, by a member whose key parameter is a
`Crypto\KeyPair` alone, so a token this program signs under a shared key still carries only what
`Jwt::verify` reads back.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A token with no <code>kid</code> cannot be walked against a key set, so verification costs one signature check whatever the sender wrote</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/protocols-and-tokens/#algorithm-comes-from-the-key" title="The algorithm comes from the key and never from the token"><code>security/algorithm-comes-from-the-key</code></a> <a href="/docs/rules/security/protocols-and-tokens/#jwt-expiry-is-mandatory" title="A JWT without an expiry, or past one, fails verification, and no flag disables the check"><code>security/jwt-expiry-is-mandatory</code></a> <a href="/docs/rules/security/protocols-and-tokens/#verification-does-not-launder" title="A verified signature proves origin, not safety: claims come back tainted"><code>security/verification-does-not-launder</code></a> <a href="/docs/rules/security/laundering/#derived-codec-qualifiers" title="A derived codec adds no qualifier rule, and a secret field is refused at the declaration rather than silently omitted"><code>security/derived-codec-qualifiers</code></a> <a href="/docs/rules/security/protocols-and-tokens/#jwe-compact-subset" title="Core\Jwe speaks compact JWE with A256GCM alone, and the static that built the key picks the algorithm"><code>security/jwe-compact-subset</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0179.md">record 0179</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/jwt-verify-issued-finds-one-key-and-tries-no-other.nvst"><code>tests/conformance/core/jwt-verify-issued-finds-one-key-and-tries-no-other.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/jwt-verify-issued-refuses-a-header-a-verifier-may-not-honour.nvst"><code>tests/conformance/core/jwt-verify-issued-refuses-a-header-a-verifier-may-not-honour.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/jwt-verify-issued-answers-the-written-type-from-either-key-spelling.nvst"><code>tests/conformance/core/jwt-verify-issued-answers-the-written-type-from-either-key-spelling.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/jwt-verify-issued-reaches-the-clock-and-the-claims-only-under-a-signature.nvst"><code>tests/conformance/core/jwt-verify-issued-reaches-the-clock-and-the-claims-only-under-a-signature.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/jwt-a-key-set-admits-what-it-serves-and-skips-the-rest.nvst"><code>tests/conformance/core/jwt-a-key-set-admits-what-it-serves-and-skips-the-rest.nvst</code></a></dd></div></dl>

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

**The refusal has two grounds, and what arms each of them differs.** A covered request whose `Origin`
names somewhere other than where it arrived is refused with nothing configured at all — that is the
forgery this is named for, and it reads only headers the request brought. A covered request without a
token this deployment issued, bound to the session it rides under, is refused as soon as `[http]
csrf_key` names the key that says what a valid token is; until it does, there is no expected token for
any request to be missing. A valid token passes whatever the origin says, because it proves the page
the request came from was this application's.

What a deployment that names no key therefore does **not** refuse is a covered request that sent no
`Origin` — every non-browser client, and so every API caller. Naming the key is what covers it. A key
written so the door cannot read it is refused at boot rather than left unarmed, because a deployment
whose configuration says the check is on and whose door verifies nothing is the one way this must not
fail.

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
