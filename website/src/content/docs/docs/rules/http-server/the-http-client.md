---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "The HTTP client"
description: "No spelling for \"wait forever\", one deadline over the whole call, opt-in jittered retry, and an address pinned before connecting."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/http-server/uploads/
  label: "Uploads"
next:
  link: /docs/rules/http-server/containment/
  label: "Containment"
---

<p class="nv-section-lead">No spelling for &quot;wait forever&quot;, one deadline over the whole call, opt-in jittered retry, and an address pinned before connecting.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">6</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">6</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">0</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">6</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#no-spelling-for-an-unbounded-wait"><code>Core\Http\Client</code> has no spelling for &quot;wait forever&quot;: every bound is a <code>Duration</code>, an omitted one inherits <code>[http.client]</code>, and expiry throws <code>TimeoutError</code></a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#one-deadline-covers-the-whole-call">One <code>deadline</code> covers the whole call: the connection, every redirect hop, every retry attempt and every backoff between them</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#retry-is-opt-in-jittered-and-closed">Retry is opt-in, exponential with full jitter that cannot be turned off, and retries only a connection failure, a timeout, <code>429</code>, <code>502</code>, <code>503</code> and <code>504</code></a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-non-idempotent-retry-needs-an-idempotency-key">A <code>POST</code> or <code>PATCH</code> is not retried without <code>retryIdempotencyKey</code>, and at an ordinary call site the missing key is a compile-time diagnostic</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#allow-url-pins-the-address"><code>Core\Http::allowUrl</code> resolves, checks and pins: it answers a <code>Target</code> carrying the URL and the one approved address, and the connection is made to that address</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#redirects-are-off-and-every-hop-is-re-pinned">Redirects are not followed by default; when enabled every hop is re-checked and re-pinned, a denied hop fails the request, and a retry never re-resolves</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li></ol>

<div class="nv-rule" id="no-spelling-for-an-unbounded-wait">

## `Core\Http\Client` has no spelling for "wait forever": every bound is a `Duration`, an omitted one inherits `[http.client]`, and expiry throws `TimeoutError`

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#no-spelling-for-an-unbounded-wait"><code>http-server/no-spelling-for-an-unbounded-wait</code></a>
</div>

`Core\Http\Client` has no spelling for "wait forever". Every bound in `Core\Http\Options` —
`deadline`, `connectTimeout`, `retryBackoff` — is a `Duration`, which has no infinite value
([`types/duration-literal`](/docs/rules/types/text-and-literal-types/#duration-literal "1h30m is a Core\Time\Duration constant, in one grammar shared by source, parse and nvs.toml")); there is no `deadline: null` and no `0` meaning unbounded; and a
call that omits the field inherits `[http.client] deadline` (shipped `30s`) or `connect_timeout`
(shipped `5s`) rather than removing the bound. An unbounded outbound call is therefore not
something a program can express, the same way a shell string is not something `Core\Process` can
express ([`core-classes/process-is-argv-only`](/docs/rules/core-classes/processes-and-files/#process-is-argv-only "Core\Process is the one way to run another program, and there is no shell string anywhere in it")): the guarantee comes from the absence of a
spelling, not from a check.

The bag is a closed set of keys, and the three retry keys — `retryAttempts`, `retryBackoff`,
`retryIdempotencyKey` — are flat rather than a nested `retry` shape, because a bag flattens to one
ABI argument per option and a bag nested inside one has nothing to flatten into
([`core-api/shape-rules`](/docs/rules/core-api/naming-and-shape/#shape-rules "Every Core member obeys the same twenty shape rules, R1–R20") R2). The prefix keeps the grouping legible at a call site.

Expiry throws `TimeoutError`, never a falsy return ([`core-api/failure-throws`](/docs/rules/core-api/one-way-to-do-each-thing/#failure-throws "A Core member throws on failure and returns ?T for absence; false is never an answer")), and the
deadline it reports is the one that covers the whole call
([`http-server/one-deadline-covers-the-whole-call`](/docs/rules/http-server/the-http-client/#one-deadline-covers-the-whole-call "One deadline covers the whole call: the connection, every redirect hop, every retry attempt and every backoff between them")).

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Curl defaults to no timeout and <code>file_get_contents</code> to <code>default_socket_timeout</code>'s sixty seconds; here there is no <code>null</code>, no <code>0</code> and no option that means unbounded, so a long call names its own <code>deadline</code></p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/the-http-client/#one-deadline-covers-the-whole-call" title="One deadline covers the whole call: the connection, every redirect hop, every retry attempt and every backoff between them"><code>http-server/one-deadline-covers-the-whole-call</code></a> <a href="/docs/rules/http-server/the-http-client/#retry-is-opt-in-jittered-and-closed" title="Retry is opt-in, exponential with full jitter that cannot be turned off, and retries only a connection failure, a timeout, 429, 502, 503 and 504"><code>http-server/retry-is-opt-in-jittered-and-closed</code></a> <a href="/docs/rules/types/text-and-literal-types/#duration-literal" title="1h30m is a Core\Time\Duration constant, in one grammar shared by source, parse and nvs.toml"><code>types/duration-literal</code></a> <a href="/docs/rules/core-api/one-way-to-do-each-thing/#failure-throws" title="A Core member throws on failure and returns ?T for absence; false is never an answer"><code>core-api/failure-throws</code></a> <a href="/docs/rules/core-classes/processes-and-files/#process-is-argv-only" title="Core\Process is the one way to run another program, and there is no shell string anywhere in it"><code>core-classes/process-is-argv-only</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0074.md">record 0074</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/no-client-member-accepts-an-unbounded-wait.nvst"><code>tests/conformance/core/no-client-member-accepts-an-unbounded-wait.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/core_members.rs"><code>crates/nvs-types/tests/core_members.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/tests/tree.rs"><code>crates/nvs-config/tests/tree.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="one-deadline-covers-the-whole-call">

## One `deadline` covers the whole call: the connection, every redirect hop, every retry attempt and every backoff between them

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#one-deadline-covers-the-whole-call"><code>http-server/one-deadline-covers-the-whole-call</code></a>
</div>

`deadline` covers the **whole call**: the connection, every redirect hop, every retry attempt and
every backoff between them. A single stated number is what a caller can reason about; the
per-attempt timeout most clients offer is the one that turns "5 seconds" into fifteen.

Two consequences are rules of their own. The deadline is never extended: when it would expire
during a backoff, the call throws immediately rather than sleeping and then failing, and a
`Retry-After` the server sent is clamped to the remaining deadline. And the total elapsed time of
a retried call does not exceed its deadline plus one connection timeout, which is the bound a
test asserts.

The redirect chain and every retry attempt share this one bound with the pinning rule they run
under: a hop is re-checked and re-pinned, a retry reuses the pinned address, and neither buys
itself more time ([`http-server/redirects-are-off-and-every-hop-is-re-pinned`](/docs/rules/http-server/the-http-client/#redirects-are-off-and-every-hop-is-re-pinned "Redirects are not followed by default; when enabled every hop is re-checked and re-pinned, a denied hop fails the request, and a retry never re-resolves")).

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Curl's <code>CURLOPT_TIMEOUT</code> bounds one transfer, so three attempts at five seconds is a fifteen-second call; here the stated number is the bound on everything</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/the-http-client/#no-spelling-for-an-unbounded-wait" title="Core\Http\Client has no spelling for wait forever: every bound is a Duration, an omitted one inherits [http.client], and expiry throws TimeoutError"><code>http-server/no-spelling-for-an-unbounded-wait</code></a> <a href="/docs/rules/http-server/the-http-client/#retry-is-opt-in-jittered-and-closed" title="Retry is opt-in, exponential with full jitter that cannot be turned off, and retries only a connection failure, a timeout, 429, 502, 503 and 504"><code>http-server/retry-is-opt-in-jittered-and-closed</code></a> <a href="/docs/rules/http-server/the-http-client/#redirects-are-off-and-every-hop-is-re-pinned" title="Redirects are not followed by default; when enabled every hop is re-checked and re-pinned, a denied hop fails the request, and a retry never re-resolves"><code>http-server/redirects-are-off-and-every-hop-is-re-pinned</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0074.md">record 0074</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0058.md">record 0058</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/http/transport.rs"><code>crates/nvs-stdlib/src/http/transport.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/every-client-member-answers-a-response-with-the-same-two-readers.nvst"><code>tests/conformance/core/every-client-member-answers-a-response-with-the-same-two-readers.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="retry-is-opt-in-jittered-and-closed">

## Retry is opt-in, exponential with full jitter that cannot be turned off, and retries only a connection failure, a timeout, `429`, `502`, `503` and `504`

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#retry-is-opt-in-jittered-and-closed"><code>http-server/retry-is-opt-in-jittered-and-closed</code></a>
</div>

`retryAttempts` absent means one attempt. Present, it is the **total** number of attempts
including the first, and must be at least 1. `retryBackoff` is the base delay, shipped `100ms`,
growing exponentially per attempt with **full jitter** — the actual wait is uniformly random in
`[0, base × 2^n]`. Jitter is not optional and not configurable: unjittered retries from many
hosts synchronise into a burst against a service that is already failing, which is the failure
mode retry is supposed to relieve.

What is retried is a closed set: a connection failure, a timeout, and status `429`, `502`, `503`
and `504`. Nothing else — a `400` or a `403` is an answer, and retrying it is a load generator; a
`500` is usually a real application error and is deliberately not on the list. A `Retry-After`
header on a `429` or `503` replaces the computed backoff, clamped to the remaining deadline.

Every attempt runs under the one deadline ([`http-server/one-deadline-covers-the-whole-call`](/docs/rules/http-server/the-http-client/#one-deadline-covers-the-whole-call "One deadline covers the whole call: the connection, every redirect hop, every retry attempt and every backoff between them"))
and reuses the `Core\Http\Target` the launderer pinned, so a retry performs no second resolution
([`http-server/redirects-are-off-and-every-hop-is-re-pinned`](/docs/rules/http-server/the-http-client/#redirects-are-off-and-every-hop-is-re-pinned "Redirects are not followed by default; when enabled every hop is re-checked and re-pinned, a denied hop fails the request, and a retry never re-resolves")). A `POST` or `PATCH` is not
retried at all without a key ([`http-server/a-non-idempotent-retry-needs-an-idempotency-key`](/docs/rules/http-server/the-http-client/#a-non-idempotent-retry-needs-an-idempotency-key "A POST or PATCH is not retried without retryIdempotencyKey, and at an ordinary call site the missing key is a compile-time diagnostic")).

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>PHP's clients have no retry, so every application writes its own loop; here it is three flat options, the jitter is not configurable, and a <code>Retry-After</code> header replaces the computed wait</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/the-http-client/#one-deadline-covers-the-whole-call" title="One deadline covers the whole call: the connection, every redirect hop, every retry attempt and every backoff between them"><code>http-server/one-deadline-covers-the-whole-call</code></a> <a href="/docs/rules/http-server/the-http-client/#a-non-idempotent-retry-needs-an-idempotency-key" title="A POST or PATCH is not retried without retryIdempotencyKey, and at an ordinary call site the missing key is a compile-time diagnostic"><code>http-server/a-non-idempotent-retry-needs-an-idempotency-key</code></a> <a href="/docs/rules/http-server/the-http-client/#redirects-are-off-and-every-hop-is-re-pinned" title="Redirects are not followed by default; when enabled every hop is re-checked and re-pinned, a denied hop fails the request, and a retry never re-resolves"><code>http-server/redirects-are-off-and-every-hop-is-re-pinned</code></a> <a href="/docs/rules/http-server/the-http-client/#no-spelling-for-an-unbounded-wait" title="Core\Http\Client has no spelling for wait forever: every bound is a Duration, an omitted one inherits [http.client], and expiry throws TimeoutError"><code>http-server/no-spelling-for-an-unbounded-wait</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0074.md">record 0074</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0058.md">record 0058</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/http/transport.rs"><code>crates/nvs-stdlib/src/http/transport.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/every-client-member-judges-a-request-before-it-leaves-the-process.nvst"><code>tests/conformance/core/every-client-member-judges-a-request-before-it-leaves-the-process.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-non-idempotent-retry-needs-an-idempotency-key">

## A `POST` or `PATCH` is not retried without `retryIdempotencyKey`, and at an ordinary call site the missing key is a compile-time diagnostic

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-non-idempotent-retry-needs-an-idempotency-key"><code>http-server/a-non-idempotent-retry-needs-an-idempotency-key</code></a>
</div>

`GET`, `HEAD`, `PUT`, `DELETE`, `OPTIONS` and `TRACE` retry freely. **`POST` and `PATCH` require
`retryIdempotencyKey`**, sent as an `Idempotency-Key` header identical across every attempt — the
convention every payment API already implements, and the difference between a retried request
and a card charged twice.

Because the options bag is a compile-time-constant shape literal ([`core-api/shape-rules`](/docs/rules/core-api/naming-and-shape/#shape-rules "Every Core member obeys the same twenty shape rules, R1–R20")
R2) and the verb is the member's own name (`Client::post`), both halves are statically known at
an ordinary call site, and a `post` that asks for retries without the key is a **diagnostic**
naming the field. It is the verb that decides, asked of every member rather than of one.

Where the verb is genuinely dynamic — `Client::send($request)` with a runtime method — the check
moves to the call and **throws before the first attempt** rather than before the second, so a test
run finds it rather than production finding it on the one retry that matters. A key supplied for
a verb that does not need one is accepted and sent; some servers want it regardless, and refusing
it would buy nothing. A key is never generated automatically: one minted per call is a different
key on the next request, which makes the header present and useless.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A retried <code>POST</code> in PHP does whatever the loop the developer wrote does, including charging a card twice; here the missing key is a red squiggle before the program runs</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/the-http-client/#retry-is-opt-in-jittered-and-closed" title="Retry is opt-in, exponential with full jitter that cannot be turned off, and retries only a connection failure, a timeout, 429, 502, 503 and 504"><code>http-server/retry-is-opt-in-jittered-and-closed</code></a> <a href="/docs/rules/core-api/naming-and-shape/#shape-rules" title="Every Core member obeys the same twenty shape rules, R1–R20"><code>core-api/shape-rules</code></a> <a href="/docs/rules/core-api/parameters-and-options/#options-bag" title="A member's optional knobs are one trailing shape literal, never a flag or a bitmask"><code>core-api/options-bag</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0074.md">record 0074</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-post-retried-without-an-idempotency-key-is-refused-while-compiling.nvst"><code>tests/conformance/core/a-post-retried-without-an-idempotency-key-is-refused-while-compiling.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/core_members.rs"><code>crates/nvs-types/tests/core_members.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="allow-url-pins-the-address">

## `Core\Http::allowUrl` resolves, checks and pins: it answers a `Target` carrying the URL and the one approved address, and the connection is made to that address

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#allow-url-pins-the-address"><code>http-server/allow-url-pins-the-address</code></a>
</div>

```
Core\Http::allowUrl(tainted string $url): Core\Http\Target
```

The launderer parses the URL, refuses a scheme outside the grant, resolves the host, checks every
resolved address against [`security/net-address-policy`](/docs/rules/security/scopes-and-denial/#net-address-policy "net.connect carries an address policy that denies the private ranges, and an exception is one written IP address"), and answers a `Target` carrying
**both the URL and the specific address that was approved**. It throws, naming which check
failed, rather than returning a falsy value. The text is judged before the deployment is asked,
so a refusal on the URL itself never depends on a grant.

The `Target` return is the load-bearing part. A launderer answering a plain `string` would leave a
gap between the check and the connection in which a second DNS resolution could return a
different address — the classic rebinding attack. Because every `Core\Http\Client` member
connects to the address inside the `Target`, there is no second resolution to poison. That is why
this is the one launderer in [`security/launderers-are-sink-named`](/docs/rules/security/laundering/#launderers-are-sink-named "The only way out of tainted is a Core member whose contract names the one sink it is safe for")'s roster whose output is a
value, and why `Target` has no members: a program that could read the approved address back out
could rebuild a request around a different one.

A plain `string` URL — the form kept for a URL the program itself authored — passes the same four
questions at the member that connects, so there is one implementation of the policy and not two,
and it lives in the capability rather than the client
([`security/the-policy-lives-in-the-capability`](/docs/rules/security/scopes-and-denial/#the-policy-lives-in-the-capability "The address policy lives in the capability, so every client obeys it and none of them may hold its own")).

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>file_get_contents($userUrl)</code> and <code>curl_setopt(CURLOPT_URL, $userUrl)</code> are a compile-time diagnostic, and the laundered value is a <code>Target</code> the program cannot read an address back out of, not a string</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/tainted-data/#outbound-url-is-a-sink" title="An outbound URL or address is a sink, and a tainted one is refused where it is written"><code>security/outbound-url-is-a-sink</code></a> <a href="/docs/rules/security/scopes-and-denial/#net-address-policy" title="net.connect carries an address policy that denies the private ranges, and an exception is one written IP address"><code>security/net-address-policy</code></a> <a href="/docs/rules/security/scopes-and-denial/#the-policy-lives-in-the-capability" title="The address policy lives in the capability, so every client obeys it and none of them may hold its own"><code>security/the-policy-lives-in-the-capability</code></a> <a href="/docs/rules/security/laundering/#launderers-are-sink-named" title="The only way out of tainted is a Core member whose contract names the one sink it is safe for"><code>security/launderers-are-sink-named</code></a> <a href="/docs/rules/http-server/the-http-client/#redirects-are-off-and-every-hop-is-re-pinned" title="Redirects are not followed by default; when enabled every hop is re-checked and re-pinned, a denied hop fails the request, and a retry never re-resolves"><code>http-server/redirects-are-off-and-every-hop-is-re-pinned</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0058.md">record 0058</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0074.md">record 0074</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/an-approved-url-is-pinned-to-one-address.nvst"><code>tests/conformance/core/an-approved-url-is-pinned-to-one-address.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/every-client-member-refuses-a-tainted-url.nvst"><code>tests/conformance/core/every-client-member-refuses-a-tainted-url.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/an-outbound-url-is-judged-before-the-capability-is-consulted.nvst"><code>tests/conformance/core/an-outbound-url-is-judged-before-the-capability-is-consulted.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="redirects-are-off-and-every-hop-is-re-pinned">

## Redirects are not followed by default; when enabled every hop is re-checked and re-pinned, a denied hop fails the request, and a retry never re-resolves

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#redirects-are-off-and-every-hop-is-re-pinned"><code>http-server/redirects-are-off-and-every-hop-is-re-pinned</code></a>
</div>

Redirects are **not followed by default**: `[http.client] max_redirects` ships as `0`, and a
`followRedirects` count at the call is the only way to raise it. When a hop is taken it is
re-checked and re-pinned by the same procedure the first URL passed
([`http-server/allow-url-pins-the-address`](/docs/rules/http-server/the-http-client/#allow-url-pins-the-address "Core\Http::allowUrl resolves, checks and pins: it answers a Target carrying the URL and the one approved address, and the connection is made to that address")), and a redirect to a denied address **fails the
request** rather than being silently dropped from the chain — a redirect is the standard way to
defeat a check applied only to the first URL. A redirect that arrives when no hop was asked for
is simply the answer.

A **retry** is the opposite case and must not re-resolve: every attempt of a retried call reuses
the `Target` the launderer pinned, so retrying opens no second resolution for a rebinding attack to
poison, and a retried call performs exactly one DNS resolution. A redirect hop re-resolves and
re-checks; a retry attempt does neither.

Both the redirect chain and every retry attempt are covered by one `deadline`
([`http-server/one-deadline-covers-the-whole-call`](/docs/rules/http-server/the-http-client/#one-deadline-covers-the-whole-call "One deadline covers the whole call: the connection, every redirect hop, every retry attempt and every backoff between them")).

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Curl's <code>FOLLOWLOCATION</code> is off by default too, but a hop it follows is never checked against anything; here each hop passes the same policy the first URL did and a denied one fails the call rather than being dropped from the chain</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/the-http-client/#allow-url-pins-the-address" title="Core\Http::allowUrl resolves, checks and pins: it answers a Target carrying the URL and the one approved address, and the connection is made to that address"><code>http-server/allow-url-pins-the-address</code></a> <a href="/docs/rules/http-server/the-http-client/#one-deadline-covers-the-whole-call" title="One deadline covers the whole call: the connection, every redirect hop, every retry attempt and every backoff between them"><code>http-server/one-deadline-covers-the-whole-call</code></a> <a href="/docs/rules/http-server/the-http-client/#retry-is-opt-in-jittered-and-closed" title="Retry is opt-in, exponential with full jitter that cannot be turned off, and retries only a connection failure, a timeout, 429, 502, 503 and 504"><code>http-server/retry-is-opt-in-jittered-and-closed</code></a> <a href="/docs/rules/security/scopes-and-denial/#net-address-policy" title="net.connect carries an address policy that denies the private ranges, and an exception is one written IP address"><code>security/net-address-policy</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0058.md">record 0058</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0074.md">record 0074</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/http/transport.rs"><code>crates/nvs-stdlib/src/http/transport.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/tests/tree.rs"><code>crates/nvs-config/tests/tree.rs</code></a></dd></div></dl>

</div>
