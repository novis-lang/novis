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

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">20</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">20</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">0</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">18</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#no-spelling-for-an-unbounded-wait"><code>Core\Http\Client</code> has no spelling for &quot;wait forever&quot;: every bound is a <code>Duration</code>, an omitted one inherits <code>[http.client]</code>, and expiry throws <code>TimeoutError</code></a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#an-outbound-request-carries-one-body">An outbound request carries at most one body, named by which of four flat keys it is written under, and a second key or a body on <code>get</code> is a compile-time diagnostic</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#one-deadline-covers-the-whole-call">One <code>deadline</code> covers the whole call: the connection, every redirect hop, every retry attempt and every backoff between them</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-streamed-reply-is-bounded-by-idle-and-a-lifetime"><code>deadline</code> ends at the head of a streamed reply, its body runs under <code>idle</code> and <code>maxDuration</code>, and the body is read one way, once</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#retry-is-opt-in-jittered-and-closed">Retry is opt-in, exponential with full jitter that cannot be turned off, and retries only a connection failure, a timeout, <code>429</code>, <code>502</code>, <code>503</code> and <code>504</code></a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-non-idempotent-retry-needs-an-idempotency-key">A <code>POST</code> or <code>PATCH</code> is not retried without <code>retryIdempotencyKey</code>, and at an ordinary call site the missing key is a compile-time diagnostic</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#allow-url-pins-the-address"><code>Core\Http::allowUrl</code> resolves, checks and pins: it answers a <code>Target</code> carrying the URL and every approved address, and the connection is made to one of them</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#an-outbound-call-tries-every-approved-address">Every address a name resolves to is checked, one denied refuses the host, and the call falls back across the approved set without ever resolving twice</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#an-outbound-call-names-its-address-only-under-a-grant"><code>connectTo</code> names an outbound call's address only where <code>net.connect_to</code> names the host, and the address policy and the certificate's host check still apply</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#redirects-are-off-and-every-hop-is-re-pinned">Redirects are not followed by default; when enabled every hop is re-checked and re-pinned, a denied hop fails the request, and a retry never re-resolves</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#an-https-redirect-never-becomes-plaintext">A redirect from <code>https</code> to <code>http</code> needs <code>net.downgrade</code> for its host and <code>redirectToHttp</code> at the call; a plain <code>http</code> URL asked for directly stays allowed</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-cross-origin-redirect-drops-credentials">A redirect hop to another origin carries none of the caller's own headers, the three named credentials and every <code>secret</code> value among them</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#an-outbound-connection-is-pooled-per-core-and-stays-pinned">An outbound connection is pooled per core under a key carrying everything the check approved, and returns to the pool only after a reply read to the end under known framing</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#an-outbound-proxy-is-operator-configured"><code>[http.client.proxy]</code> is the only way an outbound call is proxied: <code>System</code> class, written by the operator, and never a call option or an environment variable</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise"><code>resolve</code> is mandatory: <code>local</code> tunnels to the address Novis approved and keeps the pin, <code>proxy</code> narrows the policy to the URL's text and warns at every boot</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#the-client-trust-roots-are-the-operators"><code>[http.client.tls]</code> is the operator's alone: the bundled roots unless files are named, TLS 1.2 unless the floor is raised, and no key log in <code>production</code></a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-reply-reports-its-tls-session"><code>Response::tls()</code> reports the session a reply arrived over — version, cipher, whether the peer was verified, and its chain — and is <code>null</code> when there was none</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#an-outbound-socket-is-opened-like-an-outbound-call">A WebSocket is opened by <code>Core\Http\Client::openSocket</code> under every rule an outbound call obeys, <code>ws</code> and <code>wss</code> serve that row alone, and a socket is never pooled</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#an-outbound-socket-belongs-to-the-task-that-opened-it">An outbound socket is charged to the task that opened it and closed with <code>1001</code> when that task ends, and it is never a root isolate</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#an-outbound-socket-is-bounded-by-idle-a-lifetime-and-a-message-cap"><code>idle</code>, <code>maxDuration</code>, <code>maxMessage</code> and <code>sendTimeout</code> bound every outbound socket, each inherits a finite default, and none has an unbounded spelling</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li></ol>

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

<div class="nv-rule" id="an-outbound-request-carries-one-body">

## An outbound request carries at most one body, named by which of four flat keys it is written under, and a second key or a body on `get` is a compile-time diagnostic

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#an-outbound-request-carries-one-body"><code>http-server/an-outbound-request-carries-one-body</code></a>
</div>

An outbound request carries at most one body, and which of four flat keys in `Core\Http\Options` it was
written under is what says how it is sent: `json` encodes the value as `application/json`, `form` encodes
a map as `application/x-www-form-urlencoded`, `body` sends the octets as given under an optional
`contentType`, and `multipart` sends a map as `multipart/form-data`. Four keys rather than one with a
mode beside it, because a mode string is refused ([`core-api/no-mode-strings`](/docs/rules/core-api/one-way-to-do-each-thing/#no-mode-strings "A mode is an enum, never a string or an integer constant, and only four grammars are exempt")) and sniffing cannot
tell a form from a JSON object by looking at an `array<string, string>`.

**Two body keys, a body on `get` or `head`, and `contentType` without `body` are compile-time
diagnostics**, each naming the key. Both halves of the question are in front of the checker: the bag is a
compile-time-constant shape literal ([`core-api/shape-rules`](/docs/rules/core-api/naming-and-shape/#shape-rules "Every Core member obeys the same twenty shape rules, R1–R20") R2) and the verb is the member's own
name, which is exactly why the missing idempotency key is a diagnostic too
([`http-server/a-non-idempotent-retry-needs-an-idempotency-key`](/docs/rules/http-server/the-http-client/#a-non-idempotent-retry-needs-an-idempotency-key "A POST or PATCH is not retried without retryIdempotencyKey, and at an ordinary call site the missing key is a compile-time diagnostic")). Where the verb is dynamic, the same
two checks throw before the first attempt.

A body position admits `tainted`, because posting what a user sent is ordinary and the qualifier is a
question about sinks a request body does not have; and it admits `secret`, as a header already does
([`security/secret-sinks-refuse`](/docs/rules/security/secrets/#secret-sinks-refuse "Every output sink refuses a secret value outright, and no auto-escape or neutralisation bypasses one")'s outbound exemption), which is why a `json` value is walked with
that exemption applied rather than handed to `Core\Json::encode`. `Core\Http\Part` is how a body is sent
without being held: `Part::file` streams from disk under `fs.read` with `Content-Length` from the file's
size. `Content-Length` is always written — every body's size is known before the first byte — so there is
no chunked request body. The body is built once per call and resent unchanged on every attempt, and a
file part is re-read per attempt.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Curl takes <code>CURLOPT_POSTFIELDS</code> as a string, an array or a file handle and infers the encoding from which one it got; here the key names what is sent, two keys are refused before the program runs, and a <code>Part::file</code> streams from disk rather than being held whole</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-api/naming-and-shape/#shape-rules" title="Every Core member obeys the same twenty shape rules, R1–R20"><code>core-api/shape-rules</code></a> <a href="/docs/rules/core-api/parameters-and-options/#options-bag" title="A member's optional knobs are one trailing shape literal, never a flag or a bitmask"><code>core-api/options-bag</code></a> <a href="/docs/rules/core-api/one-way-to-do-each-thing/#no-mode-strings" title="A mode is an enum, never a string or an integer constant, and only four grammars are exempt"><code>core-api/no-mode-strings</code></a> <a href="/docs/rules/http-server/the-http-client/#a-non-idempotent-retry-needs-an-idempotency-key" title="A POST or PATCH is not retried without retryIdempotencyKey, and at an ordinary call site the missing key is a compile-time diagnostic"><code>http-server/a-non-idempotent-retry-needs-an-idempotency-key</code></a> <a href="/docs/rules/security/secrets/#secret-sinks-refuse" title="Every output sink refuses a secret value outright, and no auto-escape or neutralisation bypasses one"><code>security/secret-sinks-refuse</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0180.md">record 0180</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/http/transport.rs"><code>crates/nvs-stdlib/src/http/transport.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/http-client-two-body-keys-are-refused.nvst"><code>tests/conformance/reject/http-client-two-body-keys-are-refused.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/http-client-a-body-on-get-or-head-is-refused.nvst"><code>tests/conformance/reject/http-client-a-body-on-get-or-head-is-refused.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/http-client-a-content-type-without-a-body-is-refused.nvst"><code>tests/conformance/reject/http-client-a-content-type-without-a-body-is-refused.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/http-client-sends-a-json-a-form-a-raw-and-a-multipart-body.nvst"><code>tests/conformance/core/http-client-sends-a-json-a-form-a-raw-and-a-multipart-body.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/http-client-every-body-carrying-verb-frames-a-body-alike.nvst"><code>tests/conformance/core/http-client-every-body-carrying-verb-frames-a-body-alike.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/http-client-streams-a-file-as-the-whole-body.nvst"><code>tests/conformance/core/http-client-streams-a-file-as-the-whole-body.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/http-part-refuses-what-it-cannot-send-and-draws-a-boundary-per-call.nvst"><code>tests/conformance/core/http-part-refuses-what-it-cannot-send-and-draws-a-boundary-per-call.nvst</code></a></dd></div></dl>

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

<div class="nv-rule" id="a-streamed-reply-is-bounded-by-idle-and-a-lifetime">

## `deadline` ends at the head of a streamed reply, its body runs under `idle` and `maxDuration`, and the body is read one way, once

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-streamed-reply-is-bounded-by-idle-and-a-lifetime"><code>http-server/a-streamed-reply-is-bounded-by-idle-and-a-lifetime</code></a>
</div>

`deadline` covers a streamed call's connection and its head and ends there; the body runs under two
further `Duration`s, `idle` — the longest silence allowed — and `maxDuration` — the longest the body may
take at all. Both inherit `[http.client] idle` and `max_duration` when a call omits them, and neither has
an unbounded spelling, so a stream has no more way to say *forever* than a buffered call does
([`http-server/no-spelling-for-an-unbounded-wait`](/docs/rules/http-server/the-http-client/#no-spelling-for-an-unbounded-wait "Core\Http\Client has no spelling for wait forever: every bound is a Duration, an omitted one inherits [http.client], and expiry throws TimeoutError")). Either key on a buffered member is a compile-time
diagnostic.

Two bounds rather than one because they catch different failures. A server that stops sending is caught
by `idle` in seconds. A server that dribbles one byte per second forever passes every idle check ever
written, and only a lifetime ends it — which is the case a single timeout on a streaming client always
misses.

`Client::stream(Core\Http\Method $method, $url, {…})` answers a `Core\Http\Stream` once the head has
arrived, with `status()`, `header()`, `headers()` and `tls()` readable and the body not yet read. **The
body is then read one way, once** — `events()`, `lines()`, `chunks()` or `saveTo($path, $max)` under
`fs.write`, whose bound is required for [`core-classes/io-write-stream`](/docs/rules/core-classes/processes-and-files/#io-write-stream "Core\IO::writeStream is where every stream reaches disk, and a failed write removes its partial file")'s reason — and a second read
throws, the same division the inbound half already makes between readers that share a body and readers
that consume it ([`http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it`](/docs/rules/http-server/methods-bodies-and-static-files/#buffering-readers-share-the-body-and-streaming-readers-consume-it "A buffering body reader keeps what it read so another may follow it, and a streaming one consumes the body and is the only reader of it")).
Every piece a reader yields is `tainted`, a line and one event's accumulated `data` are each
length-capped as a constant rather than a directive, and **a stream is retried only before its head**:
after the first byte of body a failure ends the stream, because the caller has already seen a prefix of
one answer and re-requesting would hand it the prefix of another.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A PHP stream has <code>default_socket_timeout</code> and nothing that bounds how long a body may take; here a streamed body has two finite bounds it cannot be without, and reading it twice throws</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/the-http-client/#one-deadline-covers-the-whole-call" title="One deadline covers the whole call: the connection, every redirect hop, every retry attempt and every backoff between them"><code>http-server/one-deadline-covers-the-whole-call</code></a> <a href="/docs/rules/http-server/the-http-client/#no-spelling-for-an-unbounded-wait" title="Core\Http\Client has no spelling for wait forever: every bound is a Duration, an omitted one inherits [http.client], and expiry throws TimeoutError"><code>http-server/no-spelling-for-an-unbounded-wait</code></a> <a href="/docs/rules/http-server/methods-bodies-and-static-files/#buffering-readers-share-the-body-and-streaming-readers-consume-it" title="A buffering body reader keeps what it read so another may follow it, and a streaming one consumes the body and is the only reader of it"><code>http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it</code></a> <a href="/docs/rules/core-classes/processes-and-files/#io-write-stream" title="Core\IO::writeStream is where every stream reaches disk, and a failed write removes its partial file"><code>core-classes/io-write-stream</code></a> <a href="/docs/rules/config/changeability-classes/#three-changeability-classes" title="nvs.toml states defaults, not ceilings, and every directive carries one of three changeability classes"><code>config/three-changeability-classes</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0180.md">record 0180</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/http/transport.rs"><code>crates/nvs-stdlib/src/http/transport.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/tests/tree.rs"><code>crates/nvs-config/tests/tree.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/http-stream-reads-lines-and-chunks-and-refuses-a-second-read.nvst"><code>tests/conformance/core/http-stream-reads-lines-and-chunks-and-refuses-a-second-read.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/http-stream-caps-a-line-and-one-events-data.nvst"><code>tests/conformance/core/http-stream-caps-a-line-and-one-events-data.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/http-stream-save-to-stops-at-its-bound.nvst"><code>tests/conformance/core/http-stream-save-to-stops-at-its-bound.nvst</code></a></dd></div></dl>

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
header on a `429` or `503` replaces the computed backoff in either of its forms, delay-seconds or
an HTTP-date, clamped to the remaining deadline — and a date already past keeps the jittered
backoff rather than becoming a zero wait, so the clients one date was handed to do not retry in
step.

Every attempt runs under the one deadline ([`http-server/one-deadline-covers-the-whole-call`](/docs/rules/http-server/the-http-client/#one-deadline-covers-the-whole-call "One deadline covers the whole call: the connection, every redirect hop, every retry attempt and every backoff between them"))
and reuses the `Core\Http\Target` the launderer pinned, so a retry performs no second resolution
([`http-server/redirects-are-off-and-every-hop-is-re-pinned`](/docs/rules/http-server/the-http-client/#redirects-are-off-and-every-hop-is-re-pinned "Redirects are not followed by default; when enabled every hop is re-checked and re-pinned, a denied hop fails the request, and a retry never re-resolves")). A `POST` or `PATCH` is not
retried at all without a key ([`http-server/a-non-idempotent-retry-needs-an-idempotency-key`](/docs/rules/http-server/the-http-client/#a-non-idempotent-retry-needs-an-idempotency-key "A POST or PATCH is not retried without retryIdempotencyKey, and at an ordinary call site the missing key is a compile-time diagnostic")).

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>PHP's clients have no retry, so every application writes its own loop; here it is three flat options, the jitter is not configurable, and a <code>Retry-After</code> header replaces the computed wait</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/the-http-client/#one-deadline-covers-the-whole-call" title="One deadline covers the whole call: the connection, every redirect hop, every retry attempt and every backoff between them"><code>http-server/one-deadline-covers-the-whole-call</code></a> <a href="/docs/rules/http-server/the-http-client/#a-non-idempotent-retry-needs-an-idempotency-key" title="A POST or PATCH is not retried without retryIdempotencyKey, and at an ordinary call site the missing key is a compile-time diagnostic"><code>http-server/a-non-idempotent-retry-needs-an-idempotency-key</code></a> <a href="/docs/rules/http-server/the-http-client/#redirects-are-off-and-every-hop-is-re-pinned" title="Redirects are not followed by default; when enabled every hop is re-checked and re-pinned, a denied hop fails the request, and a retry never re-resolves"><code>http-server/redirects-are-off-and-every-hop-is-re-pinned</code></a> <a href="/docs/rules/http-server/the-http-client/#no-spelling-for-an-unbounded-wait" title="Core\Http\Client has no spelling for wait forever: every bound is a Duration, an omitted one inherits [http.client], and expiry throws TimeoutError"><code>http-server/no-spelling-for-an-unbounded-wait</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0074.md">record 0074</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0058.md">record 0058</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0180.md">record 0180</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/http/transport.rs"><code>crates/nvs-stdlib/src/http/transport.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/every-client-member-judges-a-request-before-it-leaves-the-process.nvst"><code>tests/conformance/core/every-client-member-judges-a-request-before-it-leaves-the-process.nvst</code></a></dd></div></dl>

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

Where the verb is genuinely dynamic — `Client::request($method, $url)` — the check
moves to the call and **throws before the first attempt** rather than before the second, so a test
run finds it rather than production finding it on the one retry that matters. A key supplied for
a verb that does not need one is accepted and sent; some servers want it regardless, and refusing
it would buy nothing. A key is never generated automatically: one minted per call is a different
key on the next request, which makes the header present and useless.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A retried <code>POST</code> in PHP does whatever the loop the developer wrote does, including charging a card twice; here the missing key is a red squiggle before the program runs</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/the-http-client/#retry-is-opt-in-jittered-and-closed" title="Retry is opt-in, exponential with full jitter that cannot be turned off, and retries only a connection failure, a timeout, 429, 502, 503 and 504"><code>http-server/retry-is-opt-in-jittered-and-closed</code></a> <a href="/docs/rules/core-api/naming-and-shape/#shape-rules" title="Every Core member obeys the same twenty shape rules, R1–R20"><code>core-api/shape-rules</code></a> <a href="/docs/rules/core-api/parameters-and-options/#options-bag" title="A member's optional knobs are one trailing shape literal, never a flag or a bitmask"><code>core-api/options-bag</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0074.md">record 0074</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0180.md">record 0180</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-post-retried-without-an-idempotency-key-is-refused-while-compiling.nvst"><code>tests/conformance/core/a-post-retried-without-an-idempotency-key-is-refused-while-compiling.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/core_members.rs"><code>crates/nvs-types/tests/core_members.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="allow-url-pins-the-address">

## `Core\Http::allowUrl` resolves, checks and pins: it answers a `Target` carrying the URL and every approved address, and the connection is made to one of them

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#allow-url-pins-the-address"><code>http-server/allow-url-pins-the-address</code></a>
</div>

```
Core\Http::allowUrl(tainted string $url): Core\Http\Target
```

The launderer parses the URL, refuses a scheme outside the grant, resolves the host, checks every
resolved address against [`security/net-address-policy`](/docs/rules/security/scopes-and-denial/#net-address-policy "net.connect carries an address policy that denies the private ranges, and an exception is one written IP address") — one denied refuses the host — and
answers a `Target` carrying **the URL and every address that was approved**
([`http-server/an-outbound-call-tries-every-approved-address`](/docs/rules/http-server/the-http-client/#an-outbound-call-tries-every-approved-address "Every address a name resolves to is checked, one denied refuses the host, and the call falls back across the approved set without ever resolving twice")). It throws, naming which check
failed, rather than returning a falsy value. The text is judged before the deployment is asked,
so a refusal on the URL itself never depends on a grant.

The roster the scheme is checked against is four: `http` and `https` for a request, `ws` and `wss` for the
row that opens a socket ([`http-server/an-outbound-socket-is-opened-like-an-outbound-call`](/docs/rules/http-server/the-http-client/#an-outbound-socket-is-opened-like-an-outbound-call "A WebSocket is opened by Core\Http\Client::openSocket under every rule an outbound call obeys, ws and wss serve that row alone, and a socket is never pooled")). The
launderer admits all four and pins them identically, because what it approves is a host and its addresses
rather than an intention; which of the four a given row serves is the row's own refusal, so a URL laundered
for one use cannot be spent on the other.

The `Target` return is the load-bearing part. A launderer answering a plain `string` would leave a
gap between the check and the connection in which a second DNS resolution could return a
different address — the classic rebinding attack. Because every `Core\Http\Client` member
connects to an address inside the `Target`, there is no second resolution to poison. That is why
this is the one launderer in [`security/launderers-are-sink-named`](/docs/rules/security/laundering/#launderers-are-sink-named "The only way out of tainted is a Core member whose contract names the one sink it is safe for")'s roster whose output is a
value, and why `Target` has no members: a program that could read the approved addresses back out
could rebuild a request around an address that was never approved.

A plain `string` URL — the form kept for a URL the program itself authored — passes the same four
questions at the member that connects, so there is one implementation of the policy and not two,
and it lives in the capability rather than the client
([`security/the-policy-lives-in-the-capability`](/docs/rules/security/scopes-and-denial/#the-policy-lives-in-the-capability "The address policy lives in the capability, so every client obeys it and none of them may hold its own")).

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>file_get_contents($userUrl)</code> and <code>curl_setopt(CURLOPT_URL, $userUrl)</code> are a compile-time diagnostic, and the laundered value is a <code>Target</code> the program cannot read an address back out of, not a string</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/tainted-data/#outbound-url-is-a-sink" title="An outbound URL or address is a sink, and a tainted one is refused where it is written"><code>security/outbound-url-is-a-sink</code></a> <a href="/docs/rules/security/scopes-and-denial/#net-address-policy" title="net.connect carries an address policy that denies the private ranges, and an exception is one written IP address"><code>security/net-address-policy</code></a> <a href="/docs/rules/security/scopes-and-denial/#the-policy-lives-in-the-capability" title="The address policy lives in the capability, so every client obeys it and none of them may hold its own"><code>security/the-policy-lives-in-the-capability</code></a> <a href="/docs/rules/security/laundering/#launderers-are-sink-named" title="The only way out of tainted is a Core member whose contract names the one sink it is safe for"><code>security/launderers-are-sink-named</code></a> <a href="/docs/rules/http-server/the-http-client/#redirects-are-off-and-every-hop-is-re-pinned" title="Redirects are not followed by default; when enabled every hop is re-checked and re-pinned, a denied hop fails the request, and a retry never re-resolves"><code>http-server/redirects-are-off-and-every-hop-is-re-pinned</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0058.md">record 0058</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0074.md">record 0074</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0180.md">record 0180</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0183.md">record 0183</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/an-approved-url-is-pinned-to-one-address.nvst"><code>tests/conformance/core/an-approved-url-is-pinned-to-one-address.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/every-client-member-refuses-a-tainted-url.nvst"><code>tests/conformance/core/every-client-member-refuses-a-tainted-url.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/an-outbound-url-is-judged-before-the-capability-is-consulted.nvst"><code>tests/conformance/core/an-outbound-url-is-judged-before-the-capability-is-consulted.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="an-outbound-call-tries-every-approved-address">

## Every address a name resolves to is checked, one denied refuses the host, and the call falls back across the approved set without ever resolving twice

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#an-outbound-call-tries-every-approved-address"><code>http-server/an-outbound-call-tries-every-approved-address</code></a>
</div>

Every address a name resolves to is checked against [`security/net-address-policy`](/docs/rules/security/scopes-and-denial/#net-address-policy "net.connect carries an address policy that denies the private ranges, and an exception is one written IP address"), and **one denied
address refuses the whole host**, naming it. A name that answers both a public address and one the policy
denies is what a rebinding attack looks like from the resolver's side, so the denied answer is not
quietly dropped from the set and the rest used; `net.internal`'s exceptions still apply per address. An
IP literal, and a `connectTo` value ([`http-server/an-outbound-call-names-its-address-only-under-a-grant`](/docs/rules/http-server/the-http-client/#an-outbound-call-names-its-address-only-under-a-grant "connectTo names an outbound call's address only where net.connect_to names the host, and the address policy and the certificate's host check still apply")),
are a set of one.

`Core\Http\Target` carries **the approved set**, in the resolver's order and at most eight of it, and
still has no members, for [`http-server/allow-url-pins-the-address`](/docs/rules/http-server/the-http-client/#allow-url-pins-the-address "Core\Http::allowUrl resolves, checks and pins: it answers a Target carrying the URL and every approved address, and the connection is made to one of them")'s reason: a program that could
read the set back out could rebuild a request around an address nothing approved. A retry reuses the set
and never re-resolves, so a retried call still performs exactly one resolution; a redirect hop resolves
and checks anew.

The connection falls back across the set RFC 8305's way — families interleaved from the resolver's first
answer, the next attempt started when the previous one has not connected within a fixed attempt delay,
the first to connect kept and the rest closed — all under the one `connectTimeout`, itself clamped by the
deadline ([`http-server/one-deadline-covers-the-whole-call`](/docs/rules/http-server/the-http-client/#one-deadline-covers-the-whole-call "One deadline covers the whole call: the connection, every redirect hop, every retry attempt and every backoff between them")), so falling back introduces no new
bound. Every address failing is one `IOError` naming each, because an error naming only the last one
sends the reader to the wrong host. The lookup itself runs on the blocking pool per
[`http-server/a-core-is-never-blocked-on-a-syscall`](/docs/rules/http-server/containment/#a-core-is-never-blocked-on-a-syscall "A core is never blocked on a syscall: filesystem calls, name resolution and waiting on a child go to a blocking pool bounded at twice the core count"), with the grant asked on the core before it and
the address check back on the core after it.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Curl tries every address a name answers and checks none of them; here each is checked before any is tried, one denied refuses the whole host, and the lookup runs off the core</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/the-http-client/#allow-url-pins-the-address" title="Core\Http::allowUrl resolves, checks and pins: it answers a Target carrying the URL and every approved address, and the connection is made to one of them"><code>http-server/allow-url-pins-the-address</code></a> <a href="/docs/rules/security/scopes-and-denial/#net-address-policy" title="net.connect carries an address policy that denies the private ranges, and an exception is one written IP address"><code>security/net-address-policy</code></a> <a href="/docs/rules/http-server/containment/#a-core-is-never-blocked-on-a-syscall" title="A core is never blocked on a syscall: filesystem calls, name resolution and waiting on a child go to a blocking pool bounded at twice the core count"><code>http-server/a-core-is-never-blocked-on-a-syscall</code></a> <a href="/docs/rules/http-server/the-http-client/#redirects-are-off-and-every-hop-is-re-pinned" title="Redirects are not followed by default; when enabled every hop is re-checked and re-pinned, a denied hop fails the request, and a retry never re-resolves"><code>http-server/redirects-are-off-and-every-hop-is-re-pinned</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0180.md">record 0180</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/http.rs"><code>crates/nvs-stdlib/src/http.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/http/transport.rs"><code>crates/nvs-stdlib/src/http/transport.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/http/pool.rs"><code>crates/nvs-stdlib/src/http/pool.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/an-approved-url-is-pinned-to-one-address.nvst"><code>tests/conformance/core/an-approved-url-is-pinned-to-one-address.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-granted-host-is-still-refused-at-a-denied-address.nvst"><code>tests/conformance/core/a-granted-host-is-still-refused-at-a-denied-address.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="an-outbound-call-names-its-address-only-under-a-grant">

## `connectTo` names an outbound call's address only where `net.connect_to` names the host, and the address policy and the certificate's host check still apply

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#an-outbound-call-names-its-address-only-under-a-grant"><code>http-server/an-outbound-call-names-its-address-only-under-a-grant</code></a>
</div>

A call may name the address it connects to with `connectTo: string`, and only where a `net.connect_to`
grant lists the URL's host. The value is an IP literal; the call connects there instead of resolving the
host, and the certificate is still checked against the host the URL named.

The option widens nothing, and that is what makes it grantable. The address is judged by
[`security/net-address-policy`](/docs/rules/security/scopes-and-denial/#net-address-policy "net.connect carries an address policy that denies the private ranges, and an exception is one written IP address") and by `net.internal`'s exceptions exactly as a resolved one is, so
`connectTo` chooses *among addresses the deployment already allows* — it is `curl --resolve` with the
address policy still underneath, which is how a program reaches one node of a cluster, or a canary behind
a shared name, without the deployment having to loosen anything else.

A `Core\Http\Target` already carries the addresses its laundering approved
([`http-server/an-outbound-call-tries-every-approved-address`](/docs/rules/http-server/the-http-client/#an-outbound-call-tries-every-approved-address "Every address a name resolves to is checked, one denied refuses the host, and the call falls back across the approved set without ever resolving twice")), so `connectTo` beside one is a
`LogicError` rather than a silent override of the pin: the two are answers to the same question, and a
call that supplies both has not decided which one it meant.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>CURLOPT_RESOLVE</code> takes any host-to-address mapping a script writes; here the host must be granted, the address is judged by the same policy a resolved one is, and the certificate is still checked against the URL's host</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/the-http-client/#an-outbound-call-tries-every-approved-address" title="Every address a name resolves to is checked, one denied refuses the host, and the call falls back across the approved set without ever resolving twice"><code>http-server/an-outbound-call-tries-every-approved-address</code></a> <a href="/docs/rules/security/scopes-and-denial/#net-address-policy" title="net.connect carries an address policy that denies the private ranges, and an exception is one written IP address"><code>security/net-address-policy</code></a> <a href="/docs/rules/security/capabilities/#capability-question-is-grant-and-scope" title="A capability question is a grant and a scope, asked of the request's own configuration snapshot"><code>security/capability-question-is-grant-and-scope</code></a> <a href="/docs/rules/http-server/the-http-client/#allow-url-pins-the-address" title="Core\Http::allowUrl resolves, checks and pins: it answers a Target carrying the URL and every approved address, and the connection is made to one of them"><code>http-server/allow-url-pins-the-address</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0180.md">record 0180</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/http.rs"><code>crates/nvs-stdlib/src/http.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/src/capability.rs"><code>crates/nvs-config/src/capability.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/tests/capability.rs"><code>crates/nvs-config/tests/capability.rs</code></a></dd></div></dl>

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
([`http-server/allow-url-pins-the-address`](/docs/rules/http-server/the-http-client/#allow-url-pins-the-address "Core\Http::allowUrl resolves, checks and pins: it answers a Target carrying the URL and every approved address, and the connection is made to one of them")), and a redirect to a denied address **fails the
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

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/the-http-client/#allow-url-pins-the-address" title="Core\Http::allowUrl resolves, checks and pins: it answers a Target carrying the URL and every approved address, and the connection is made to one of them"><code>http-server/allow-url-pins-the-address</code></a> <a href="/docs/rules/http-server/the-http-client/#one-deadline-covers-the-whole-call" title="One deadline covers the whole call: the connection, every redirect hop, every retry attempt and every backoff between them"><code>http-server/one-deadline-covers-the-whole-call</code></a> <a href="/docs/rules/http-server/the-http-client/#retry-is-opt-in-jittered-and-closed" title="Retry is opt-in, exponential with full jitter that cannot be turned off, and retries only a connection failure, a timeout, 429, 502, 503 and 504"><code>http-server/retry-is-opt-in-jittered-and-closed</code></a> <a href="/docs/rules/security/scopes-and-denial/#net-address-policy" title="net.connect carries an address policy that denies the private ranges, and an exception is one written IP address"><code>security/net-address-policy</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0058.md">record 0058</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0074.md">record 0074</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/http/transport.rs"><code>crates/nvs-stdlib/src/http/transport.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/tests/tree.rs"><code>crates/nvs-config/tests/tree.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="an-https-redirect-never-becomes-plaintext">

## A redirect from `https` to `http` needs `net.downgrade` for its host and `redirectToHttp` at the call; a plain `http` URL asked for directly stays allowed

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#an-https-redirect-never-becomes-plaintext"><code>http-server/an-https-redirect-never-becomes-plaintext</code></a>
</div>

A redirect hop from `https` to `http` is refused unless its target host is in a `net.downgrade` grant
*and* the call says `redirectToHttp: true`. A plain `http` URL asked for directly stays allowed, and the
refusal names the grant.

Without this, a server can strip a call's TLS with one `Location` header and the caller still sees a
`200` — the credentials are gone by then only because
[`http-server/a-cross-origin-redirect-drops-credentials`](/docs/rules/http-server/the-http-client/#a-cross-origin-redirect-drops-credentials "A redirect hop to another origin carries none of the caller's own headers, the three named credentials and every secret value among them") takes them, and the body still travels in
the clear. Following a redirect is already opt-in
([`http-server/redirects-are-off-and-every-hop-is-re-pinned`](/docs/rules/http-server/the-http-client/#redirects-are-off-and-every-hop-is-re-pinned "Redirects are not followed by default; when enabled every hop is re-checked and re-pinned, a denied hop fails the request, and a retry never re-resolves")), so this is not a new decision for a
program to make: it is the one hop that opting in cannot silently include.

A direct `http` URL is untouched because the program chose it — an internal endpoint or a local service
is not being attacked by the code that asked for it — and because the address it reaches is judged the
same way either scheme is. Two conditions rather than one for [`security/capability-question-is-grant-and-scope`](/docs/rules/security/capabilities/#capability-question-is-grant-and-scope "A capability question is a grant and a scope, asked of the request's own configuration snapshot")'s
reason: the deployment says where a downgrade may happen, and the call says it expects one here.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Curl follows a hop from <code>https</code> to <code>http</code> silently, so one <code>Location</code> header strips a call's TLS; here it needs a grant and a call-site option, and the refusal names the grant</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/the-http-client/#redirects-are-off-and-every-hop-is-re-pinned" title="Redirects are not followed by default; when enabled every hop is re-checked and re-pinned, a denied hop fails the request, and a retry never re-resolves"><code>http-server/redirects-are-off-and-every-hop-is-re-pinned</code></a> <a href="/docs/rules/security/capabilities/#capability-question-is-grant-and-scope" title="A capability question is a grant and a scope, asked of the request's own configuration snapshot"><code>security/capability-question-is-grant-and-scope</code></a> <a href="/docs/rules/security/protocols-and-tokens/#tls-trust-is-relaxed-only-under-a-host-grant" title="Verification is relaxed only where a capabilities.tls grant names the host and the call asks for it, and no such grant has a true spelling"><code>security/tls-trust-is-relaxed-only-under-a-host-grant</code></a> <a href="/docs/rules/http-server/the-http-client/#a-cross-origin-redirect-drops-credentials" title="A redirect hop to another origin carries none of the caller's own headers, the three named credentials and every secret value among them"><code>http-server/a-cross-origin-redirect-drops-credentials</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0180.md">record 0180</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/http/transport.rs"><code>crates/nvs-stdlib/src/http/transport.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/src/capability.rs"><code>crates/nvs-config/src/capability.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/tests/capability.rs"><code>crates/nvs-config/tests/capability.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-cross-origin-redirect-drops-credentials">

## A redirect hop to another origin carries none of the caller's own headers, the three named credentials and every `secret` value among them

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-cross-origin-redirect-drops-credentials"><code>http-server/a-cross-origin-redirect-drops-credentials</code></a>
</div>

A redirect hop to another origin — the scheme, the host or the port differing — carries none of the
caller's own headers: `Authorization`, `Cookie` and `Proxy-Authorization` go, and with them every header
whose value was `secret` and every other one the program wrote. A hop inside one origin keeps them all.
This client's own headers are composed per hop either way, so the offer, the framing of a body and
`traceparent` cross a hop the caller's headers do not.

Forwarding a bearer token to whatever host a `Location` header names is a credential-exfiltration
primitive with one line of setup, and it is reached through a feature the caller asked for: following
redirects, which is why redirects are off until a count is written
([`http-server/redirects-are-off-and-every-hop-is-re-pinned`](/docs/rules/http-server/the-http-client/#redirects-are-off-and-every-hop-is-re-pinned "Redirects are not followed by default; when enabled every hop is re-checked and re-pinned, a denied hop fails the request, and a retry never re-resolves")). The `secret` clause is what makes the
rule general rather than a list: a program's own API-key header is dropped for the same reason the three
named ones are, without anyone having to enumerate the spelling each partner invented
([`security/secret-qualifier`](/docs/rules/security/secrets/#secret-qualifier "secret is a second, independent compile-time qualifier, written before tainted and in that order alone")).

**Every header and not a list, because `secret` is erased before codegen.** The qualifier is checked
once and costs nothing at run time ([`security/secret-qualifier`](/docs/rules/security/secrets/#secret-qualifier "secret is a second, independent compile-time qualifier, written before tainted and in that order alone")), so which header value carried one
is not a question the transport can ask: a `secret string` and a plain one are the same bytes by the time
a request is composed. Dropping all of them is the one way to drop every `secret` one without spending a
representation the language does not spend. What that costs is a program's own `Accept` on a hop it asked
to follow; what it buys is that no credential under a name nobody enumerated crosses to a host a
`Location` chose, and the priority ordering is what decides between those two.

No header value is written into a trace span, an access log record or an error message, on any hop. A
header guard that names the header and never the value is already how the transport reports a refusal,
and this is what makes that a tested property rather than a habit.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>An <code>Authorization</code> header set in a stream context is re-sent to whatever host a <code>Location</code> names; here a hop to another origin carries no header the program wrote, that one and every <code>secret</code> value included</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/the-http-client/#redirects-are-off-and-every-hop-is-re-pinned" title="Redirects are not followed by default; when enabled every hop is re-checked and re-pinned, a denied hop fails the request, and a retry never re-resolves"><code>http-server/redirects-are-off-and-every-hop-is-re-pinned</code></a> <a href="/docs/rules/security/secrets/#secret-qualifier" title="secret is a second, independent compile-time qualifier, written before tainted and in that order alone"><code>security/secret-qualifier</code></a> <a href="/docs/rules/security/secrets/#secret-sinks-refuse" title="Every output sink refuses a secret value outright, and no auto-escape or neutralisation bypasses one"><code>security/secret-sinks-refuse</code></a> <a href="/docs/rules/http-server/the-http-client/#an-https-redirect-never-becomes-plaintext" title="A redirect from https to http needs net.downgrade for its host and redirectToHttp at the call; a plain http URL asked for directly stays allowed"><code>http-server/an-https-redirect-never-becomes-plaintext</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0180.md">record 0180</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/http/transport.rs"><code>crates/nvs-stdlib/src/http/transport.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="an-outbound-connection-is-pooled-per-core-and-stays-pinned">

## An outbound connection is pooled per core under a key carrying everything the check approved, and returns to the pool only after a reply read to the end under known framing

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#an-outbound-connection-is-pooled-per-core-and-stays-pinned"><code>http-server/an-outbound-connection-is-pooled-per-core-and-stays-pinned</code></a>
</div>

An outbound HTTP/1.1 connection is kept alive in a **per-core** pool, keyed by the pinned address, the
port, the scheme, the TLS server name, the call's client identity and the call's TLS policy. Per core
because a core owns its requests and a shared pool is a lock on the request path, and because a bound per
core is a bound this process can state: O(cores × `pool_idle`), never O(requests served), which is the
accounting [`security/db-pool-reset-is-a-boundary`](/docs/rules/security/laundering/#db-pool-reset-is-a-boundary "A pooled database connection is proven clean before it is reused, and a failed reset destroys it") already holds the database pool to.

**The key is the load-bearing part**, and every element of it is there because leaving it out lets reuse
hand a call a connection its own check would have refused. The pinned address keeps
[`http-server/allow-url-pins-the-address`](/docs/rules/http-server/the-http-client/#allow-url-pins-the-address "Core\Http::allowUrl resolves, checks and pins: it answers a Target carrying the URL and every approved address, and the connection is made to one of them") true *through* reuse; the server name keeps two hosts behind
one address from sharing a session; the identity keeps a call presenting no certificate from reusing one
that did; the policy keeps a connection opened under a relaxed verification
([`security/tls-trust-is-relaxed-only-under-a-host-grant`](/docs/rules/security/protocols-and-tokens/#tls-trust-is-relaxed-only-under-a-host-grant "Verification is relaxed only where a capabilities.tls grant names the host and the call asks for it, and no such grant has a true spelling")) from ever serving a call that verifies. A
pool keyed on the URL's host — which is what most clients key on — would quietly undo the pin.

A call is approved for a **set** of addresses ([`http-server/an-outbound-call-tries-every-approved-address`](/docs/rules/http-server/the-http-client/#an-outbound-call-tries-every-approved-address "Every address a name resolves to is checked, one denied refuses the host, and the call falls back across the approved set without ever resolving twice")),
and a held connection to any member of it serves the call: each member was approved, and each connection
is filed under the address it actually goes to. The key is still one address; what widens is the lookup,
which is one per address of the set rather than one on whichever address the walk would have tried first.

A connection returns to the pool **only** after its reply was read to the end under known framing —
`Content-Length` or chunked's last chunk — with no `Connection: close` from either side; on any doubt it
is closed, because a connection whose remaining bytes are unknown is one that will hand the next request
someone else's body. A reused connection that fails before the request's last byte is written is replaced
once **without spending an attempt**, a server closing an idle connection not being a failed request; any
later failure is an attempt under [`http-server/retry-is-opt-in-jittered-and-closed`](/docs/rules/http-server/the-http-client/#retry-is-opt-in-jittered-and-closed "Retry is opt-in, exponential with full jitter that cannot be turned off, and retries only a connection failure, a timeout, 429, 502, 503 and 504"). The two caps
are `[http.client] pool_idle`, the idle connections one core may hold, and `pool_idle_timeout`, how long
one may sit idle before it is closed — both `System` class, because they bound a core's memory rather
than a request's ([`config/three-changeability-classes`](/docs/rules/config/changeability-classes/#three-changeability-classes "nvs.toml states defaults, not ceilings, and every directive carries one of three changeability classes")).

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Curl reuses a connection keyed on host and port inside one handle, and PHP's stream functions reuse nothing; here the key carries the pinned address, the client identity and the call's TLS policy, so reuse cannot widen what a check approved</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/the-http-client/#allow-url-pins-the-address" title="Core\Http::allowUrl resolves, checks and pins: it answers a Target carrying the URL and every approved address, and the connection is made to one of them"><code>http-server/allow-url-pins-the-address</code></a> <a href="/docs/rules/http-server/the-http-client/#redirects-are-off-and-every-hop-is-re-pinned" title="Redirects are not followed by default; when enabled every hop is re-checked and re-pinned, a denied hop fails the request, and a retry never re-resolves"><code>http-server/redirects-are-off-and-every-hop-is-re-pinned</code></a> <a href="/docs/rules/security/laundering/#db-pool-reset-is-a-boundary" title="A pooled database connection is proven clean before it is reused, and a failed reset destroys it"><code>security/db-pool-reset-is-a-boundary</code></a> <a href="/docs/rules/security/protocols-and-tokens/#tls-trust-is-relaxed-only-under-a-host-grant" title="Verification is relaxed only where a capabilities.tls grant names the host and the call asks for it, and no such grant has a true spelling"><code>security/tls-trust-is-relaxed-only-under-a-host-grant</code></a> <a href="/docs/rules/config/changeability-classes/#three-changeability-classes" title="nvs.toml states defaults, not ceilings, and every directive carries one of three changeability classes"><code>config/three-changeability-classes</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0180.md">record 0180</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/http/pool.rs"><code>crates/nvs-stdlib/src/http/pool.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/http/transport.rs"><code>crates/nvs-stdlib/src/http/transport.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/tests/tree.rs"><code>crates/nvs-config/tests/tree.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="an-outbound-proxy-is-operator-configured">

## `[http.client.proxy]` is the only way an outbound call is proxied: `System` class, written by the operator, and never a call option or an environment variable

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#an-outbound-proxy-is-operator-configured"><code>http-server/an-outbound-proxy-is-operator-configured</code></a>
</div>

A `Core\Http\Client` call leaves through a forward proxy when, and only when, an operator has written
`[http.client.proxy]` in `nvs.toml`: there is no call option, no client option, no program-side spelling,
and **no environment variable is read** — not `HTTP_PROXY`, not `HTTPS_PROXY`, not `NO_PROXY`, in any case
spelling.

The block is `System` class and `Reload` ([`config/three-changeability-classes`](/docs/rules/config/changeability-classes/#three-changeability-classes "nvs.toml states defaults, not ceilings, and every directive carries one of three changeability classes"),
[`config/reloadability-is-its-own-field`](/docs/rules/config/changeability-classes/#reloadability-is-its-own-field "Reloadability is a second registry field, Reload or Boot, orthogonal to the changeability class")): `Core\Config::set` fails on every key, and a changed value
takes effect on the next call, with the pool's key carrying the proxy so nothing the old value made can
serve one.

| Directive | Ships | Refuses |
|---|---|---|
| `url` | — | anything but an `http://` URL with a host |
| `resolve` | **none — mandatory** | a missing value, and any word but `local` or `proxy` |
| `bypass` | `[]` | an entry with a port, a scheme, a `*` or a `/` |
| `username` | — | — |
| `password` / `password_file` | — | both of the pair set |

Where every outbound byte goes is a deployment's decision, not a request's. A per-call proxy would be a
per-call way to choose who resolves the destination, and therefore a per-call way to narrow
[`security/net-address-policy`](/docs/rules/security/scopes-and-denial/#net-address-policy "net.connect carries an address policy that denies the private ranges, and an exception is one written IP address") — the widening that rule exists to refuse. The environment is the same
argument with a worse blast radius: read-only ambient state, shared by every request in the process,
settable by anything that can set a variable for it, and recorded nowhere in the deployment's own
configuration ([`security/no-cross-request-state`](/docs/rules/security/closed-doors/#no-cross-request-state "Nothing a request does is observable by another request except through an explicit, capability-gated store")).

**Only `Core\Http\Client` is proxied.** `Core\Net`, a database, the shared cache tier and mail connect
directly — each is either a raw socket the program asked for by address, which has no notion of a tunnel,
or an endpoint an operator already wrote into root-owned configuration.

**The proxy URL's scheme is `http`**: `CONNECT` over plain TCP, with `https://` refused at boot. TLS *to*
the proxy is a second trust decision and has no spelling here. Every destination is tunnelled, `http` ones
included, so there is one mechanism and no path on which the proxy is handed a full request in
absolute form.

`bypass` matches the URL's host text — each entry exact, or with a leading `.` for a suffix — and a
bypassed destination is reached directly and under the full address policy. No CIDR, no wildcard and no
port: matching happens before any resolution, and a range in that position hands back more than the
operator can see they are handing back.

`username` and `password` become `Proxy-Authorization: Basic` on the `CONNECT` request **alone** — never
sent to the destination, never re-sent on a redirect hop, and never written into a trace event, a log
record or an error message. `password` is a secret directive with the usual `_file` sibling
([`config/a-secret-is-a-file-whose-content-is-the-value`](/docs/rules/config/includes-and-ownership/#a-secret-is-a-file-whose-content-is-the-value "A secret directive has a _file sibling, exactly one of the pair is set, and the file's whole content is the value")). A `407` is a `RuntimeError` naming proxy
authentication and any other refusal of `CONNECT` is an `IOError`; neither is retried by `retryAttempts`
([`http-server/retry-is-opt-in-jittered-and-closed`](/docs/rules/http-server/the-http-client/#retry-is-opt-in-jittered-and-closed "Retry is opt-in, exponential with full jitter that cannot be turned off, and retries only a connection failure, a timeout, 429, 502, 503 and 504")), which retries an answer from the destination,
and a proxy that refused the tunnel is not one.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Curl and PHP's stream wrappers take a proxy from <code>HTTP_PROXY</code>/<code>HTTPS_PROXY</code> in the environment, or per handle from <code>CURLOPT_PROXY</code>; here it is one root-owned block, no environment variable is read, and no program-side spelling exists</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/the-http-client/#a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise" title="resolve is mandatory: local tunnels to the address Novis approved and keeps the pin, proxy narrows the policy to the URL's text and warns at every boot"><code>http-server/a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise</code></a> <a href="/docs/rules/http-server/the-http-client/#the-client-trust-roots-are-the-operators" title="[http.client.tls] is the operator's alone: the bundled roots unless files are named, TLS 1.2 unless the floor is raised, and no key log in production"><code>http-server/the-client-trust-roots-are-the-operators</code></a> <a href="/docs/rules/http-server/the-http-client/#an-outbound-connection-is-pooled-per-core-and-stays-pinned" title="An outbound connection is pooled per core under a key carrying everything the check approved, and returns to the pool only after a reply read to the end under known framing"><code>http-server/an-outbound-connection-is-pooled-per-core-and-stays-pinned</code></a> <a href="/docs/rules/http-server/the-http-client/#retry-is-opt-in-jittered-and-closed" title="Retry is opt-in, exponential with full jitter that cannot be turned off, and retries only a connection failure, a timeout, 429, 502, 503 and 504"><code>http-server/retry-is-opt-in-jittered-and-closed</code></a> <a href="/docs/rules/security/closed-doors/#no-cross-request-state" title="Nothing a request does is observable by another request except through an explicit, capability-gated store"><code>security/no-cross-request-state</code></a> <a href="/docs/rules/config/changeability-classes/#three-changeability-classes" title="nvs.toml states defaults, not ceilings, and every directive carries one of three changeability classes"><code>config/three-changeability-classes</code></a> <a href="/docs/rules/config/includes-and-ownership/#a-secret-is-a-file-whose-content-is-the-value" title="A secret directive has a _file sibling, exactly one of the pair is set, and the file's whole content is the value"><code>config/a-secret-is-a-file-whose-content-is-the-value</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0182.md">record 0182</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/tests/resolve.rs"><code>crates/nvs-config/tests/resolve.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/tests/secret.rs"><code>crates/nvs-config/tests/secret.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/http/transport.rs"><code>crates/nvs-stdlib/src/http/transport.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/http.rs"><code>crates/nvs-stdlib/src/http.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise">

## `resolve` is mandatory: `local` tunnels to the address Novis approved and keeps the pin, `proxy` narrows the policy to the URL's text and warns at every boot

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise"><code>http-server/a-proxied-call-keeps-its-pin-unless-the-operator-says-otherwise</code></a>
</div>

Every `[http.client.proxy]` block writes `resolve`, and there is no default: a block without it refuses
the boot naming both words, and so does a third word.

**`resolve = "local"` keeps everything the pin buys.** Novis resolves the destination and checks every
address against [`security/net-address-policy`](/docs/rules/security/scopes-and-denial/#net-address-policy "net.connect carries an address policy that denies the private ranges, and an exception is one written IP address") exactly as it does for a direct call, and then asks the
proxy to `CONNECT` to **an address it approved**, with `Host` naming the same. Over the tunnel it speaks
what it speaks today: TLS with the server name [`http-server/allow-url-pins-the-address`](/docs/rules/http-server/the-http-client/#allow-url-pins-the-address "Core\Http::allowUrl resolves, checks and pins: it answers a Target carrying the URL and every approved address, and the connection is made to one of them") approved, or
plain HTTP for an `http` URL. There is no second resolution anywhere in the path, so a proxy does not
reopen the check-then-connect gap; falling back across the approved set works unchanged, one `CONNECT` per
address.

**`resolve = "proxy"` is for the network where only the proxy can resolve a name** — an application subnet
with no DNS route outward. `CONNECT` carries the host name, Novis judges what the URL's text can be judged
on — the scheme, the `net.connect` grant's host list, the tainted-URL check — and cannot check the address,
because it never learns one. The address question moves to the proxy.

Because that is a real weakening, it is never silent:

- **Every boot writes one `Warn` record** naming the block, the word and [`security/net-address-policy`](/docs/rules/security/scopes-and-denial/#net-address-policy "net.connect carries an address policy that denies the private ranges, and an exception is one written IP address")
  — on every start, so a deployment that has run this way for a year still says so in today's log.
- **`nvs config dump` renders the key beside that rule's id**, so the offline audit
  ([`config/check-and-dump-audit-the-tree-offline`](/docs/rules/config/reloading-and-control/#check-and-dump-audit-the-tree-offline "nvs config check and nvs config dump resolve the tree with no server, and dump --origin names where every value came from")) shows the narrowing with no server running.

The word is mandatory for [`config/scope-has-no-default`](/docs/rules/config/scheduled-work/#scope-has-no-default "scope is mandatory with no default, and fleet with no shared store refuses to boot")'s reason. Both answers are commonly correct
and either default silently does the wrong thing in somebody's production: `local` fails outright where
only the proxy resolves, which makes the ordinary corporate deployment look broken, and `proxy` gives up
the pin in every deployment whose proxy could have dialled an address, with nothing at run time to
distinguish that from a deployment that meant it.

A program cannot tell which word is written. There is no member, option or constant reporting it, for the
same reason there is no per-call spelling: whether this deployment's egress is proxied is not a thing
program code decides or branches on.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Curl hands the proxy the destination's host name in every case, having no approved address to hand it; here that is one of two written answers, the other tunnels to the address the check approved, and the one that gives up the pin is announced at every boot</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/the-http-client/#an-outbound-proxy-is-operator-configured" title="[http.client.proxy] is the only way an outbound call is proxied: System class, written by the operator, and never a call option or an environment variable"><code>http-server/an-outbound-proxy-is-operator-configured</code></a> <a href="/docs/rules/http-server/the-http-client/#allow-url-pins-the-address" title="Core\Http::allowUrl resolves, checks and pins: it answers a Target carrying the URL and every approved address, and the connection is made to one of them"><code>http-server/allow-url-pins-the-address</code></a> <a href="/docs/rules/http-server/the-http-client/#an-outbound-call-tries-every-approved-address" title="Every address a name resolves to is checked, one denied refuses the host, and the call falls back across the approved set without ever resolving twice"><code>http-server/an-outbound-call-tries-every-approved-address</code></a> <a href="/docs/rules/security/scopes-and-denial/#net-address-policy" title="net.connect carries an address policy that denies the private ranges, and an exception is one written IP address"><code>security/net-address-policy</code></a> <a href="/docs/rules/config/scheduled-work/#scope-has-no-default" title="scope is mandatory with no default, and fleet with no shared store refuses to boot"><code>config/scope-has-no-default</code></a> <a href="/docs/rules/config/reloading-and-control/#check-and-dump-audit-the-tree-offline" title="nvs config check and nvs config dump resolve the tree with no server, and dump --origin names where every value came from"><code>config/check-and-dump-audit-the-tree-offline</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0182.md">record 0182</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/tests/resolve.rs"><code>crates/nvs-config/tests/resolve.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/http/transport.rs"><code>crates/nvs-stdlib/src/http/transport.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/http.rs"><code>crates/nvs-stdlib/src/http.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/config/resolve-at-the-proxy-is-dumped-beside-the-rule-it-narrows.nvst"><code>tests/conformance/config/resolve-at-the-proxy-is-dumped-beside-the-rule-it-narrows.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="the-client-trust-roots-are-the-operators">

## `[http.client.tls]` is the operator's alone: the bundled roots unless files are named, TLS 1.2 unless the floor is raised, and no key log in `production`

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#the-client-trust-roots-are-the-operators"><code>http-server/the-client-trust-roots-are-the-operators</code></a>
</div>

Which certificates the outbound client believes is the operator's decision and has no code-side spelling
at all. `[http.client.tls]` carries three keys, every one of them `System` class
([`config/three-changeability-classes`](/docs/rules/config/changeability-classes/#three-changeability-classes "nvs.toml states defaults, not ceilings, and every directive carries one of three changeability classes")) because they configure the one `ClientConfig` the process
shares ([`security/one-tls-client`](/docs/rules/security/protocols-and-tokens/#one-tls-client "There is one TLS client in the tree, and a driver reaches it over a generic transport rather than building a second")):

| Key | Ships | Allows |
|---|---|---|
| `roots` | `["bundled"]` | Each entry is `"bundled"` — the compiled-in Mozilla set — or a PEM file. `["bundled", "/etc/novis/corp-ca.pem"]` adds a company CA; a list without `"bundled"` trusts only its files. Each file is resolved and trust-checked at boot exactly as `[db.<name>] tls_ca_file` is, and parsed by `nvs_host::tls` alone. |
| `min_version` | `"1.2"` | `"1.3"` raises the floor for every call. There is nothing below `1.2` to write. |
| `keylog` | unset | A file every session's secrets are appended to in the `SSLKEYLOGFILE` format, so an operator can read their own traffic. **Refused at boot in `production`**, naming the key; in `development` the boot says it is on, every start. |

`1.2` as the shipped floor is the one place the client trades a stronger default for reach: a great many
corporate and payment endpoints still speak nothing else, and a client that cannot reach them is a client
a deployment replaces with `curl`. Raising the floor is one line, and `nvs config dump` says what it
currently is.

`keylog` is refused rather than warned about, because a file of live session secrets is not a
configuration mistake a warning improves — the process must not start with it. A program relaxes *its
own* call's verification only through [`security/tls-trust-is-relaxed-only-under-a-host-grant`](/docs/rules/security/protocols-and-tokens/#tls-trust-is-relaxed-only-under-a-host-grant "Verification is relaxed only where a capabilities.tls grant names the host and the call asks for it, and no such grant has a true spelling"), and
never through any of these three keys.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>curl.cainfo</code> and <code>openssl.cafile</code> are <code>ini</code> settings a script can widen with <code>ini_set</code>; here the three keys are <code>System</code> class and no code-side spelling for any of them exists</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/protocols-and-tokens/#one-tls-client" title="There is one TLS client in the tree, and a driver reaches it over a generic transport rather than building a second"><code>security/one-tls-client</code></a> <a href="/docs/rules/security/protocols-and-tokens/#tls-trust-is-relaxed-only-under-a-host-grant" title="Verification is relaxed only where a capabilities.tls grant names the host and the call asks for it, and no such grant has a true spelling"><code>security/tls-trust-is-relaxed-only-under-a-host-grant</code></a> <a href="/docs/rules/config/changeability-classes/#three-changeability-classes" title="nvs.toml states defaults, not ceilings, and every directive carries one of three changeability classes"><code>config/three-changeability-classes</code></a> <a href="/docs/rules/http-server/listening-and-admission/#an-unsafe-or-unbounded-default-is-a-defect" title="A default that is unsafe inbound or unbounded outbound is a defect, not a neutral starting point"><code>http-server/an-unsafe-or-unbounded-default-is-a-defect</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0180.md">record 0180</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/src/http.rs"><code>crates/nvs-config/src/http.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-cli/src/config.rs"><code>crates/nvs-cli/src/config.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-host/src/tls.rs"><code>crates/nvs-host/src/tls.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/tests/resolve.rs"><code>crates/nvs-config/tests/resolve.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/tests/tree.rs"><code>crates/nvs-config/tests/tree.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-reply-reports-its-tls-session">

## `Response::tls()` reports the session a reply arrived over — version, cipher, whether the peer was verified, and its chain — and is `null` when there was none

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-reply-reports-its-tls-session"><code>http-server/a-reply-reports-its-tls-session</code></a>
</div>

`Core\Http\Response::tls(): ?Core\Http\TlsInfo` reports the session a reply arrived over, read through
members as every `Core` instance is ([`core-api/a-lifetime-is-an-object`](/docs/rules/core-api/lifetimes-and-absences/#a-lifetime-is-an-object "Anything with a lifetime is an object, and Core never hands back a handle")): `version()`, `cipher()`,
`verified(): bool` — `true` only when the chain *and* the name were checked — `peerChain(): array<tainted
string>` as PEM, and the leaf's `subject()`, `issuer()` and `expiry()`. It answers `null` for a plain
`http` reply and for one a test's table answered
([`testing/an-outbound-call-is-answered-from-a-table`](/docs/rules/testing/doubles-and-the-runner/#an-outbound-call-is-answered-from-a-table "One registered answer makes every outbound call in that test come from the table, and an unmatched one throws rather than reaching the network")), because neither had a session.

`verified()` is the member this exists for. A deployment that relaxed verification for one partner host
([`security/tls-trust-is-relaxed-only-under-a-host-grant`](/docs/rules/security/protocols-and-tokens/#tls-trust-is-relaxed-only-under-a-host-grant "Verification is relaxed only where a capabilities.tls grant names the host and the call asks for it, and no such grant has a true spelling")) needs a way to assert, in a test and in
production telemetry, that every *other* call still verified — and without a reply-side answer the grant
is unobservable from inside the language.

The chain is `tainted` and the timings are not here: where a call's time went is the `http` trace event's
([`observability/trace-events-carry-a-kind`](/docs/rules/observability/traces/#trace-events-carry-a-kind "A trace event carries one of five kinds — call, gc, spawn, query, http — and a call keeps its probe shape")), and a second surface for one measurement is the copy
that disagrees.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>curl_getinfo</code> answers a flat array of thirty-odd keys mixing timings, sizes and TLS facts; here the session is an object read through members and the timings are the trace's</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/protocols-and-tokens/#tls-trust-is-relaxed-only-under-a-host-grant" title="Verification is relaxed only where a capabilities.tls grant names the host and the call asks for it, and no such grant has a true spelling"><code>security/tls-trust-is-relaxed-only-under-a-host-grant</code></a> <a href="/docs/rules/http-server/the-http-client/#the-client-trust-roots-are-the-operators" title="[http.client.tls] is the operator's alone: the bundled roots unless files are named, TLS 1.2 unless the floor is raised, and no key log in production"><code>http-server/the-client-trust-roots-are-the-operators</code></a> <a href="/docs/rules/security/tainted-data/#tainted-qualifier" title="tainted is a compile-time qualifier on string, bytes and a shape of them, spellable in any declaration and erased before codegen"><code>security/tainted-qualifier</code></a> <a href="/docs/rules/core-api/lifetimes-and-absences/#a-lifetime-is-an-object" title="Anything with a lifetime is an object, and Core never hands back a handle"><code>core-api/a-lifetime-is-an-object</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0180.md">record 0180</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/http.rs"><code>crates/nvs-stdlib/src/http.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/http-response-tls-is-null-for-a-reply-the-table-answered.nvst"><code>tests/conformance/core/http-response-tls-is-null-for-a-reply-the-table-answered.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/http-tls-info-is-read-only-behind-the-null-check.nvst"><code>tests/conformance/reject/http-tls-info-is-read-only-behind-the-null-check.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/http-tls-info-names-are-tainted-and-what-the-handshake-settled-is-not.nvst"><code>tests/conformance/reject/http-tls-info-names-are-tainted-and-what-the-handshake-settled-is-not.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="an-outbound-socket-is-opened-like-an-outbound-call">

## A WebSocket is opened by `Core\Http\Client::openSocket` under every rule an outbound call obeys, `ws` and `wss` serve that row alone, and a socket is never pooled

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#an-outbound-socket-is-opened-like-an-outbound-call"><code>http-server/an-outbound-socket-is-opened-like-an-outbound-call</code></a>
</div>

```
Core\Http\Client::openSocket(string|Core\Http\Target $url, ...$options): Core\Http\Socket
```

A WebSocket is opened through the door every outbound call passes, because the opening handshake **is** an
outbound call. The URL is [`security/outbound-url-is-a-sink`](/docs/rules/security/tainted-data/#outbound-url-is-a-sink "An outbound URL or address is a sink, and a tainted one is refused where it is written")'s sink on both its spellings, so a
`tainted` value at that position is a compile-time diagnostic and `Core\Http::allowUrl` is the only way
past it; the `net.connect` grant, [`security/net-address-policy`](/docs/rules/security/scopes-and-denial/#net-address-policy "net.connect carries an address policy that denies the private ranges, and an exception is one written IP address"), the pin over every approved address,
`connectTimeout`, the TLS policy options and their grants, a client identity, `headers` — a `secret` value
among them — and the operator's proxy tunnel
([`http-server/an-outbound-proxy-is-operator-configured`](/docs/rules/http-server/the-http-client/#an-outbound-proxy-is-operator-configured "[http.client.proxy] is the only way an outbound call is proxied: System class, written by the operator, and never a call option or an environment variable")) all apply exactly as they apply to `get`.
A door of its own would be a second implementation of the same policy, and the second copy is the one that
comes to be missing a check.

`ws` and `wss` join the scheme roster for this row alone
([`http-server/allow-url-pins-the-address`](/docs/rules/http-server/the-http-client/#allow-url-pins-the-address "Core\Http::allowUrl resolves, checks and pins: it answers a Target carrying the URL and every approved address, and the connection is made to one of them")): the request rows refuse them, and `openSocket` refuses
`http` and `https`, so what a URL is *for* is written in the row that takes it. `ws` is admitted as plain
`http` is, and there is no upgrade question, because a socket follows no redirect — a `3xx` answering an
upgrade is a `RuntimeError` naming the `Location` rather than a hop taken.

What the row answers is `Core\Http\Socket`, and what that answers is `Core\Socket\Message` — the shape a
server-side connection already answers in ([`concurrency/a-connection-is-a-root-isolate`](/docs/rules/concurrency/connections/#a-connection-is-a-root-isolate "A persistent connection is a root isolate, and the request that upgraded it ends")), because one
RFC 6455 frame gets one shape in this language. `protocols` offers subprotocols as
`Sec-WebSocket-Protocol`, a `101` choosing one that was not offered is refused, and the socket reports the
one chosen. The bag carries the connection keys and not the exchange keys: there is no `body`, no
`followRedirects` and no retry trio, because a bag is a closed set and a key that could never do anything
is one a program would write and then wait for.

**A socket is never pooled.** It consumes its connection and nothing goes back to
[`http-server/an-outbound-connection-is-pooled-per-core-and-stays-pinned`](/docs/rules/http-server/the-http-client/#an-outbound-connection-is-pooled-per-core-and-stays-pinned "An outbound connection is pooled per core under a key carrying everything the check approved, and returns to the pool only after a reply read to the end under known framing")'s pool, because a connection
that has been upgraded can no longer carry a request.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no outbound WebSocket in PHP's standard library at all — <code>ext-sockets</code> and a userland client are what a program reaches for, and neither passes an address policy</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/the-http-client/#allow-url-pins-the-address" title="Core\Http::allowUrl resolves, checks and pins: it answers a Target carrying the URL and every approved address, and the connection is made to one of them"><code>http-server/allow-url-pins-the-address</code></a> <a href="/docs/rules/security/tainted-data/#outbound-url-is-a-sink" title="An outbound URL or address is a sink, and a tainted one is refused where it is written"><code>security/outbound-url-is-a-sink</code></a> <a href="/docs/rules/concurrency/connections/#a-connection-is-a-root-isolate" title="A persistent connection is a root isolate, and the request that upgraded it ends"><code>concurrency/a-connection-is-a-root-isolate</code></a> <a href="/docs/rules/http-server/the-http-client/#an-outbound-connection-is-pooled-per-core-and-stays-pinned" title="An outbound connection is pooled per core under a key carrying everything the check approved, and returns to the pool only after a reply read to the end under known framing"><code>http-server/an-outbound-connection-is-pooled-per-core-and-stays-pinned</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0183.md">record 0183</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/http/socket.rs"><code>crates/nvs-stdlib/src/http/socket.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/http-socket-opens-a-wss-url-and-reports-its-protocol.nvst"><code>tests/conformance/core/http-socket-opens-a-wss-url-and-reports-its-protocol.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/http-socket-refuses-an-http-url-and-a-request-row-refuses-wss.nvst"><code>tests/conformance/core/http-socket-refuses-an-http-url-and-a-request-row-refuses-wss.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/an-outbound-socket-speaks-ws-and-every-other-row-refuses-it.nvst"><code>tests/conformance/core/an-outbound-socket-speaks-ws-and-every-other-row-refuses-it.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/an-outbound-socket-carries-both-payload-kinds-in-both-directions.nvst"><code>tests/conformance/core/an-outbound-socket-carries-both-payload-kinds-in-both-directions.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="an-outbound-socket-belongs-to-the-task-that-opened-it">

## An outbound socket is charged to the task that opened it and closed with `1001` when that task ends, and it is never a root isolate

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#an-outbound-socket-belongs-to-the-task-that-opened-it"><code>http-server/an-outbound-socket-belongs-to-the-task-that-opened-it</code></a>
</div>

An outbound socket is a value the program opened, so it belongs to the task that opened it: its
connection, its buffers and its reassembly space are charged to that task's budget, and when the task ends
the socket is closed with `1001` and its memory is released with everything else the task held. A request's
`wall_time` therefore bounds a socket opened inside a request without having to know what a socket is.

**It is never a root isolate.** A server-side connection is one because it *outlives* the request that
upgraded it ([`concurrency/a-connection-is-a-root-isolate`](/docs/rules/concurrency/connections/#a-connection-is-a-root-isolate "A persistent connection is a root isolate, and the request that upgraded it ends")), and the arena, globals, budget and
timeline entry it is given are what that escape costs. An outbound socket has no request to escape from:
it is held by the running program that opened it, so an isolate of its own would put a boundary between
the program and the socket it is reading, costing an arena and a copy per socket and buying nothing. Memory
stays O(open sockets), and because no socket outlives its task, that is O(in-flight) rather than O(sockets
opened).

It never crosses an isolate boundary — [`classes/graph-copy`](/docs/rules/classes/copying-and-serializing/#graph-copy "The graph copy is one recursive, cycle-safe walk with two carriers, and it refuses what has no meaning on the other side") refuses it as it refuses every other
resource — so there is no question of who closes one. A program that wants a socket to outlive a request
opens it in something that outlives a request: a command, a queue job, a spawned script. Two `receive`s
waiting at once on one socket is a `LogicError`, because one message has one recipient and the alternative
is a fan-out policy invented for what is a program bug.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/concurrency/connections/#a-connection-is-a-root-isolate" title="A persistent connection is a root isolate, and the request that upgraded it ends"><code>concurrency/a-connection-is-a-root-isolate</code></a> <a href="/docs/rules/classes/copying-and-serializing/#graph-copy" title="The graph copy is one recursive, cycle-safe walk with two carriers, and it refuses what has no meaning on the other side"><code>classes/graph-copy</code></a> <a href="/docs/rules/http-server/the-http-client/#an-outbound-socket-is-opened-like-an-outbound-call" title="A WebSocket is opened by Core\Http\Client::openSocket under every rule an outbound call obeys, ws and wss serve that row alone, and a socket is never pooled"><code>http-server/an-outbound-socket-is-opened-like-an-outbound-call</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0183.md">record 0183</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/http/socket.rs"><code>crates/nvs-stdlib/src/http/socket.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/an-outbound-socket-closes-once-and-answers-nothing-after.nvst"><code>tests/conformance/core/an-outbound-socket-closes-once-and-answers-nothing-after.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="an-outbound-socket-is-bounded-by-idle-a-lifetime-and-a-message-cap">

## `idle`, `maxDuration`, `maxMessage` and `sendTimeout` bound every outbound socket, each inherits a finite default, and none has an unbounded spelling

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#an-outbound-socket-is-bounded-by-idle-a-lifetime-and-a-message-cap"><code>http-server/an-outbound-socket-is-bounded-by-idle-a-lifetime-and-a-message-cap</code></a>
</div>

Four bounds cover an outbound socket, and none of them has a spelling for "forever".

| Option | Bounds | Inherits when omitted |
|---|---|---|
| `idle` | the longest silence | `[http.client] idle` |
| `maxDuration` | the socket's whole life | `[http.client] max_duration` |
| `maxMessage` | the largest message after reassembly | `[http.client.socket] max_message` |
| `sendTimeout` | how long a frame may wait to be written | `[http.client.socket] send_timeout` |

`idle` and `maxDuration` are the two bounds and the two directives a streamed reply already reads
([`http-server/a-streamed-reply-is-bounded-by-idle-and-a-lifetime`](/docs/rules/http-server/the-http-client/#a-streamed-reply-is-bounded-by-idle-and-a-lifetime "deadline ends at the head of a streamed reply, its body runs under idle and maxDuration, and the body is read one way, once")), for the same reason one level
down: an idle check alone never ends a peer that dribbles, and a lifetime alone lets a dead connection sit
until it expires. A message past `maxMessage` closes the socket with `1009`. `ping` is the one knob with an
off position and it is off by default, because a ping is traffic the peer did not ask for; a program that
sets it is choosing to have `idle` end a *dead* peer rather than a quiet one. A peer's ping is always
answered regardless — that is the protocol, not a policy.

Every bound is a `Duration` or a `uint` of bytes, and neither type has an infinite value
([`types/duration-literal`](/docs/rules/types/text-and-literal-types/#duration-literal "1h30m is a Core\Time\Duration constant, in one grammar shared by source, parse and nvs.toml"), [`core-api/units-are-types`](/docs/rules/core-api/one-way-to-do-each-thing/#units-are-types "A unit is a type: a duration is a Duration and a size is uint bytes")). There is no `null` and no `0` meaning
unbounded, so an outbound socket that waits forever is not something a program can express — the guarantee
comes from the absence of a spelling, exactly as it does for a call
([`http-server/no-spelling-for-an-unbounded-wait`](/docs/rules/http-server/the-http-client/#no-spelling-for-an-unbounded-wait "Core\Http\Client has no spelling for wait forever: every bound is a Duration, an omitted one inherits [http.client], and expiry throws TimeoutError")). Expiry throws `TimeoutError`
([`core-api/failure-throws`](/docs/rules/core-api/one-way-to-do-each-thing/#failure-throws "A Core member throws on failure and returns ?T for absence; false is never an answer")).

`[http.client.socket]` carries `max_message` and `send_timeout`, both `Runtime`
([`config/three-changeability-classes`](/docs/rules/config/changeability-classes/#three-changeability-classes "nvs.toml states defaults, not ceilings, and every directive carries one of three changeability classes")), as `[http.client] deadline` is and for its reason: each
bounds one call, and neither is a shared resource one request could spend on another's behalf. Shipped:
`max_message` is four mebibytes and `send_timeout` is thirty seconds — the numbers this process already
applies to the other half of RFC 6455, because one process holding two opinions about the size of one
message is how a program comes to work in one direction and not the other. Neither ships unbounded, which
[`http-server/an-unsafe-or-unbounded-default-is-a-defect`](/docs/rules/http-server/listening-and-admission/#an-unsafe-or-unbounded-default-is-a-defect "A default that is unsafe inbound or unbounded outbound is a defect, not a neutral starting point") requires of exactly this kind of wait.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/the-http-client/#no-spelling-for-an-unbounded-wait" title="Core\Http\Client has no spelling for wait forever: every bound is a Duration, an omitted one inherits [http.client], and expiry throws TimeoutError"><code>http-server/no-spelling-for-an-unbounded-wait</code></a> <a href="/docs/rules/http-server/the-http-client/#a-streamed-reply-is-bounded-by-idle-and-a-lifetime" title="deadline ends at the head of a streamed reply, its body runs under idle and maxDuration, and the body is read one way, once"><code>http-server/a-streamed-reply-is-bounded-by-idle-and-a-lifetime</code></a> <a href="/docs/rules/http-server/listening-and-admission/#an-unsafe-or-unbounded-default-is-a-defect" title="A default that is unsafe inbound or unbounded outbound is a defect, not a neutral starting point"><code>http-server/an-unsafe-or-unbounded-default-is-a-defect</code></a> <a href="/docs/rules/config/changeability-classes/#three-changeability-classes" title="nvs.toml states defaults, not ceilings, and every directive carries one of three changeability classes"><code>config/three-changeability-classes</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0183.md">record 0183</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/http/socket.rs"><code>crates/nvs-stdlib/src/http/socket.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/tests/directives.rs"><code>crates/nvs-config/tests/directives.rs</code></a></dd></div></dl>

</div>
