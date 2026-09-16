---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Methods, bodies and static files"
description: "HEAD runs as GET, a preflight is answered before any code runs, and a route that never reads a body allocates nothing."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/http-server/resolving-a-request/
  label: "Resolving a request"
next:
  link: /docs/rules/http-server/headers-cors-and-cookies/
  label: "Response headers, CORS and cookies"
---

<p class="nv-section-lead">HEAD runs as GET, a preflight is answered before any code runs, and a route that never reads a body allocates nothing.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">7</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">6</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">1</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">7</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#head-runs-as-get"><code>HEAD</code> runs as <code>GET</code> with the body discarded and <code>Content-Length</code> kept; <code>method()</code> reports <code>Get</code> and <code>isHead()</code> tells the truth</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-preflight-is-answered-before-any-code-runs">A CORS preflight is answered from <code>[http.cors]</code> before the unit is loaded, and a plain <code>OPTIONS</code> is passed through</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#the-body-is-read-on-demand-under-two-caps"><code>[limits] request_body</code> bounds bytes parsed into memory and <code>upload_total</code> bounds a streamed multipart body, each with a hard ceiling, and a route that never reads a body allocates nothing</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#buffering-readers-share-the-body-and-streaming-readers-consume-it">A buffering body reader keeps what it read so another may follow it, and a streaming one consumes the body and is the only reader of it</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#static-serving-is-one-policy">Static serving is one policy in both modes: the exact file, never a listing, one <code>ETag</code>, one <code>Range</code>, and a <code>.nvs</code> file is never served as source</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#health-path-is-off-and-checks-nothing"><code>health_path</code> is off by default; set, it answers <code>200</code> while accepting and <code>503</code> while draining, and checks no dependency</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#the-trace-id-is-the-request-identifier">The trace id exists on every request regardless of sampling, is the only request identifier, is emitted on the response, and an inbound <code>X-Request-ID</code> is ignored</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li></ol>

<div class="nv-rule" id="head-runs-as-get">

## `HEAD` runs as `GET` with the body discarded and `Content-Length` kept; `method()` reports `Get` and `isHead()` tells the truth

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#head-runs-as-get"><code>http-server/head-runs-as-get</code></a>
</div>

`HEAD` is implemented, and it is conformance rather than convention: RFC 9110 requires a general-purpose server to support it. The request runs as `GET`, the body is discarded, and `Content-Length` is kept — a `HEAD` on a `Get`-only route returns the `GET` headers with no body and the same `Content-Length`.

**`Core\Request::method()` reports `Get`.** The application passes the method to `Core\Router::match` itself, so reporting `Head` would fail the match against a `Get` route and produce the 404 the feature exists to prevent. `Core\Request::isHead()` exposes the truth for the rare caller that wants it, and it is the only member that carries the difference.

This is one of three conventions the server applies above the compiled route table; the other two are [`http-server/a-preflight-is-answered-before-any-code-runs`](/docs/rules/http-server/methods-bodies-and-static-files/#a-preflight-is-answered-before-any-code-runs "A CORS preflight is answered from [http.cors] before the unit is loaded, and a plain OPTIONS is passed through") and [`http-server/a-trailing-slash-is-never-normalised`](/docs/rules/http-server/resolving-a-request/#a-trailing-slash-is-never-normalised "A trailing slash is never normalised: /users and /users/ are two URIs"). They are three different questions, which is why they are three rules.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>$_SERVER['REQUEST_METHOD']</code> would say <code>HEAD</code>; <code>Core\Request::method()</code> says <code>Get</code> so the route matches, and <code>isHead()</code> is the only place the difference shows</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/methods-bodies-and-static-files/#a-preflight-is-answered-before-any-code-runs" title="A CORS preflight is answered from [http.cors] before the unit is loaded, and a plain OPTIONS is passed through"><code>http-server/a-preflight-is-answered-before-any-code-runs</code></a> <a href="/docs/rules/http-server/resolving-a-request/#a-trailing-slash-is-never-normalised" title="A trailing slash is never normalised: /users and /users/ are two URIs"><code>http-server/a-trailing-slash-is-never-normalised</code></a> <a href="/docs/rules/routing/matching/#matching-is-not-dispatching" title="The router stops at matching: a match is a name and typed parameters, and nothing here dispatches"><code>routing/matching-is-not-dispatching</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0097.md">record 0097</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0077.md">record 0077</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-head-request-matches-a-get-route-and-is-head-says-so.nvst"><code>tests/conformance/core/a-head-request-matches-a-get-route-and-is-head-says-so.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-preflight-is-answered-before-any-code-runs">

## A CORS preflight is answered from `[http.cors]` before the unit is loaded, and a plain `OPTIONS` is passed through

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-preflight-is-answered-before-any-code-runs"><code>http-server/a-preflight-is-answered-before-any-code-runs</code></a>
</div>

A CORS preflight — an `OPTIONS` carrying both `Origin` and `Access-Control-Request-Method` — is answered from `[http.cors]` **before any application code runs and therefore before the compiled unit is loaded**. The response policy owns that decision and it is closed by default, so with nothing configured a preflight is a `403`: the verb is one the server implements, and what is refused is the origin, not the method.

That is the one point in a request's life at which the server genuinely has no route table to ask. Everything afterwards runs with the unit in hand, and the unit carries the table, which is what lets the server match once ([`routing/matched-once-before-the-handler`](/docs/rules/routing/matching/#matched-once-before-the-handler "A request is matched once, before any application code runs, and Core\Request::route() is that match")) and what the CSRF check reads ([`security/csrf-is-on-by-default`](/docs/rules/security/protocols-and-tokens/#csrf-is-on-by-default "The server refuses an unsafe verb without a valid CSRF token, and the opt-out is written on the route itself")). Answering the preflight here is also what keeps it honest: an application asked to answer an `OPTIONS` would be answering a question the policy above it had already decided, and the two could disagree.

A plain `OPTIONS` — missing either header — is not a preflight and is **passed through** rather than answered. An application that wants to answer one has `Core\Router::methodsFor` for the `Allow:` header, and which convention to adopt stays its choice rather than the server's.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>No <code>header('Access-Control-Allow-Origin: …')</code> in application code answers a preflight; the policy answers it before the program exists</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/methods-bodies-and-static-files/#head-runs-as-get" title="HEAD runs as GET with the body discarded and Content-Length kept; method() reports Get and isHead() tells the truth"><code>http-server/head-runs-as-get</code></a> <a href="/docs/rules/routing/matching/#matched-once-before-the-handler" title="A request is matched once, before any application code runs, and Core\Request::route() is that match"><code>routing/matched-once-before-the-handler</code></a> <a href="/docs/rules/security/protocols-and-tokens/#csrf-is-on-by-default" title="The server refuses an unsafe verb without a valid CSRF token, and the opt-out is written on the route itself"><code>security/csrf-is-on-by-default</code></a> <a href="/docs/rules/http-server/headers-cors-and-cookies/#cors-is-closed-until-origins-are-named" title="CORS is closed until [http.cors] origins names one: no header is emitted and a preflight is answered 403"><code>http-server/cors-is-closed-until-origins-are-named</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0097.md">record 0097</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0074.md">record 0074</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0102.md">record 0102</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-server/src/cors.rs"><code>crates/nvs-server/src/cors.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="the-body-is-read-on-demand-under-two-caps">

## `[limits] request_body` bounds bytes parsed into memory and `upload_total` bounds a streamed multipart body, each with a hard ceiling, and a route that never reads a body allocates nothing

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#the-body-is-read-on-demand-under-two-caps"><code>http-server/the-body-is-read-on-demand-under-two-caps</code></a>
</div>

Two caps bound a request body, each a `Runtime` default with a `[limits.hard]` ceiling — the existing pair gaining two rows, not a third instance of the pattern ([`config/ceilings-are-their-own-directives`](/docs/rules/config/changeability-classes/#ceilings-are-their-own-directives "A ceiling is a System directive with the default's key name, and false removes it")). `[limits] request_body` bounds **bytes parsed into memory**, at `"8M"` with a `"64M"` ceiling; `[limits] upload_total` bounds a **streamed multipart body**, at `"256M"` with a `"2G"` ceiling. Because `Core\Request::body()` is a call, the body is read on demand: a route that never reads one allocates nothing, and a route may raise its own cap before reading. A body over `request_body` is refused; the same body on a route that raised its cap first is accepted.

**An uploaded file is a stream, and `Core\Request::files(): Iterable<Part>` is the only way to receive one.** There is no temp file — no `tmp_name`, no `move_uploaded_file`, no temp directory to configure, permission or clean up after a crash — because where a part lands is an application's explicit call under `fs.write` ([`core-classes/io-write-stream`](/docs/rules/core-classes/processes-and-files/#io-write-stream "Core\IO::writeStream is where every stream reaches disk, and a failed write removes its partial file")), never the runtime's default.

`Core\Request::bodyStream(): Iterable<bytes>` is the raw-body reader, for a body larger than a request's memory budget or a content type `files()` does not describe. It yields `tainted` chunks and is a **streaming** reader, so it keeps nothing, consumes the body, and may only be the first reader of it — [`http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it`](/docs/rules/http-server/methods-bodies-and-static-files/#buffering-readers-share-the-body-and-streaming-readers-consume-it "A buffering body reader keeps what it read so another may follow it, and a streaming one consumes the body and is the only reader of it") is which readers may follow which, for every member that reads a body. It is bounded by `request_body` on what a consumer retains, by the connection's `body_idle_timeout`, and by the multipart part-count cap ([`errors/multipart-part-count`](/docs/rules/errors/ambiguous-input/#multipart-part-count "A multipart body is capped by part count, not only by size")), which no byte cap bounds.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>upload_max_filesize</code>, <code>post_max_size</code> and <code>$_FILES['tmp_name']</code> have no counterpart: there is no temp file, <code>Core\Request::files()</code> yields streams, and a route raises its own cap before reading</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/listening-and-admission/#four-idle-waits-all-finite" title="Four waits bound a connection, all finite with nothing configured and all idle rather than total"><code>http-server/four-idle-waits-all-finite</code></a> <a href="/docs/rules/core-classes/processes-and-files/#io-write-stream" title="Core\IO::writeStream is where every stream reaches disk, and a failed write removes its partial file"><code>core-classes/io-write-stream</code></a> <a href="/docs/rules/errors/ambiguous-input/#multipart-part-count" title="A multipart body is capped by part count, not only by size"><code>errors/multipart-part-count</code></a> <a href="/docs/rules/config/changeability-classes/#ceilings-are-their-own-directives" title="A ceiling is a System directive with the default's key name, and false removes it"><code>config/ceilings-are-their-own-directives</code></a> <a href="/docs/rules/security/tainted-data/#tainted-sources" title="Every accessor that hands a program bytes from outside answers the tainted form, and the list of them is enumerable"><code>security/tainted-sources</code></a> <a href="/docs/rules/http-server/uploads/#a-part-is-a-file-iff-it-carries-a-filename" title="A part is a file part iff its Content-Disposition carries filename; every other part is buffered into post()"><code>http-server/a-part-is-a-file-iff-it-carries-a-filename</code></a> <a href="/docs/rules/http-server/uploads/#there-is-no-temp-file" title="There is no temp file: a part lands on disk only where the application names a path under fs.write, and disk is the resource it must bound"><code>http-server/there-is-no-temp-file</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0097.md">record 0097</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0105.md">record 0105</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0005.md">record 0005</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0053.md">record 0053</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0095.md">record 0095</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0156.md">record 0156</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/an-upload-is-received-only-through-files.nvst"><code>tests/conformance/core/an-upload-is-received-only-through-files.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-body-stream-chunk-is-tainted-and-a-plain-bytes-binding-refuses-it.nvst"><code>tests/conformance/core/a-body-stream-chunk-is-tainted-and-a-plain-bytes-binding-refuses-it.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-body-stream-is-named-before-it-is-walked-and-refuses-there.nvst"><code>tests/conformance/core/a-body-stream-is-named-before-it-is-walked-and-refuses-there.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="buffering-readers-share-the-body-and-streaming-readers-consume-it">

## A buffering body reader keeps what it read so another may follow it, and a streaming one consumes the body and is the only reader of it

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#buffering-readers-share-the-body-and-streaming-readers-consume-it"><code>http-server/buffering-readers-share-the-body-and-streaming-readers-consume-it</code></a>
</div>

A request body is read either by a **buffering** reader — `Core\Request::body()`, `::bytes()`, `::post()`, `::json()`, `::jsonAs<T>()` — which keeps what it read, so another buffering reader may follow it; or by a **streaming** reader — `::bodyStream()`, `::files()` — which hands the octets to the program as they arrive, keeps none of them, and so consumes the body. A buffering reader fills the request's hold, bounded by `[limits] request_body` ([`http-server/the-body-is-read-on-demand-under-two-caps`](/docs/rules/http-server/methods-bodies-and-static-files/#the-body-is-read-on-demand-under-two-caps "[limits] request_body bounds bytes parsed into memory and upload_total bounds a streamed multipart body, each with a hard ceiling, and a route that never reads a body allocates nothing")), and answers out of it: any buffering reader may follow any other, in any order, and each answers the same octets. A streaming reader may only be the first reader, and once it has run nothing reads those octets again.

| Reader | Kind | Keeps |
|---|---|---|
| `body()` | buffering | the octets |
| `bytes()` | buffering | the octets |
| `post()` | buffering | the octets and the fields it decoded — over a multipart body, the fields alone |
| `json()` | buffering | the octets, and the `Value` it decoded |
| `jsonAs<T>()` | buffering | the octets |
| `bodyStream()` | streaming | nothing |
| `files()` | streaming | nothing of a file part; the non-file fields, which it buffers |

A reader that arrives after the body was consumed throws `LogicError` naming the member that consumed it. It is refused rather than answered empty, because an empty answer from an exhausted stream is indistinguishable from a body that was empty, and that is the ambiguity [`errors/ambiguous-input-refused`](/docs/rules/errors/ambiguous-input/#ambiguous-input-refused "Ambiguous input is refused whole, never repaired") exists to refuse. The refusal is a defect in the program, never something a peer can provoke.

**`post()` after `files()` is this rule rather than an exception to it.** A `files()` walk streams the file parts and buffers everything else: a multipart form's non-file fields are held as the walk passes them ([`http-server/a-part-is-a-file-iff-it-carries-a-filename`](/docs/rules/http-server/uploads/#a-part-is-a-file-iff-it-carries-a-filename "A part is a file part iff its Content-Disposition carries filename; every other part is buffered into post()")), charged against `request_body` like any other buffered body, and `post()` answers out of that hold. It drains whatever the walk did not reach before answering, so it reports every field rather than the ones that arrived ahead of the part the walk stopped on — which is why a handler that wants the uploads takes `files()` first. `body()` after a walk is still refused: the hold carries the decoded fields, never the raw octets.

**A multipart body's hold is whichever reader filled it, and only one of the two kinds can hold the octets.** `post()` and `files()` parse such a body off the wire as it arrives, so what they leave behind is the buffered fields and `body()` after either is refused for the reason above. `body()`, `json()` and `jsonAs<T>()` hold the octets instead, under `request_body`, and a `post()` following one parses *those* rather than the wire, which is drained by then and would answer a form with no fields in it. Any buffering reader still follows any other; what the order changes is only which hold the request has. This is the two caps of [`http-server/the-body-is-read-on-demand-under-two-caps`](/docs/rules/http-server/methods-bodies-and-static-files/#the-body-is-read-on-demand-under-two-caps "[limits] request_body bounds bytes parsed into memory and upload_total bounds a streamed multipart body, each with a hard ceiling, and a route that never reads a body allocates nothing") doing what they say rather than a case being carved out: a multipart body larger than `request_body` is read by `post()` and `files()`, which never hold one whole, and refused to `body()`, which does.

`json()` keeps its decoded `Value` because `Core\Json::decode` produces only arrays and scalars, and an array is copy-on-write, so a second caller can be handed a refcount bump safely. `jsonAs<T>()` keeps nothing beyond the octets and decodes per call, because `decodeAs` builds objects and two callers must never be handed the same one. **What this spends**, per [`programs/memory-priority`](/docs/rules/programs/claims-and-priorities/#memory-priority "Memory buys security, correctness, latency and simplicity — bounded, attributable and stated"): the held octets, at most `[limits] request_body` per in-flight request — already `post()`'s bill before this rule — plus one decoded value for a request that called `json()`. Both are freed with the request, so the cost is O(in-flight) and never O(requests served).

The kinds are not a roster to maintain: a member is buffering exactly when it fills the hold, which is a fact the runtime already has, so a body reader added later classifies itself. The alternative — a closed exclusive set of three members with `post()` and then `json()` each carved out of it — was rejected on the second carve-out, because two exceptions to a rule are the rule, unwritten.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>$_POST</code> is populated before the program runs and leaves <code>php://input</code> empty for a multipart body; here nothing is read until a member asks, and which reader may follow which is stated rather than decided by <code>enable_post_data_reading</code></p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/methods-bodies-and-static-files/#the-body-is-read-on-demand-under-two-caps" title="[limits] request_body bounds bytes parsed into memory and upload_total bounds a streamed multipart body, each with a hard ceiling, and a route that never reads a body allocates nothing"><code>http-server/the-body-is-read-on-demand-under-two-caps</code></a> <a href="/docs/rules/http-server/uploads/#a-part-is-a-file-iff-it-carries-a-filename" title="A part is a file part iff its Content-Disposition carries filename; every other part is buffered into post()"><code>http-server/a-part-is-a-file-iff-it-carries-a-filename</code></a> <a href="/docs/rules/http-server/uploads/#an-upload-is-received-only-through-files" title="Core\Request::files() yields uploaded parts lazily, one at a time, and is the only way to receive one"><code>http-server/an-upload-is-received-only-through-files</code></a> <a href="/docs/rules/errors/ambiguous-input/#ambiguous-input-refused" title="Ambiguous input is refused whole, never repaired"><code>errors/ambiguous-input-refused</code></a> <a href="/docs/rules/programs/claims-and-priorities/#memory-priority" title="Memory buys security, correctness, latency and simplicity — bounded, attributable and stated"><code>programs/memory-priority</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0156.md">record 0156</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-body-read-twice-answers-the-same-octets.nvst"><code>tests/conformance/core/a-body-read-twice-answers-the-same-octets.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-body-stream-refuses-every-later-reader.nvst"><code>tests/conformance/core/a-body-stream-refuses-every-later-reader.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-request-json-answers-the-same-document-twice.nvst"><code>tests/conformance/core/a-request-json-answers-the-same-document-twice.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="static-serving-is-one-policy">

## Static serving is one policy in both modes: the exact file, never a listing, one `ETag`, one `Range`, and a `.nvs` file is never served as source

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#static-serving-is-one-policy"><code>http-server/static-serving-is-one-policy</code></a>
</div>

Static serving is one policy in both modes, because a second policy is a second security model. The exact file only, and **never a directory listing**; `index.html` is the sole default document; the MIME type comes from a fixed extension table and an unknown extension is `application/octet-stream`, which the response policy's `nosniff` renders inert.

Freshness is `Cache-Control: no-cache` with a strong `ETag` over `(size, mtime_nanos)` and `If-None-Match` — one validator, exact, and specifically **not** `Last-Modified`, whose one-second granularity serves stale bytes for two edits inside the same second. A single `Range` is honoured; a multi-range request, a unit other than `bytes`, or a range the file cannot satisfy is a `416` carrying `Content-Range: bytes */len`, never a silent `200` with the whole body. There is no configurable `max-age`, no `immutable` and no precompressed-variant lookup: this is a development convenience and a fallback, not a CDN.

**A `.nvs` file is never served as source**, under any dispatch, from any mount.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no <code>Last-Modified</code>, no directory index, no <code>.htaccess</code> and no <code>max-age</code>; <code>php -S</code> serving any file under the root, source included, has no counterpart</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/resolving-a-request/#a-request-resolves-in-five-steps" title="A request resolves by longest mount match, prefix strip, static file, path dispatch, then the entry — and production runs only match, strip, entry"><code>http-server/a-request-resolves-in-five-steps</code></a> <a href="/docs/rules/http-server/listening-and-admission/#an-unsafe-or-unbounded-default-is-a-defect" title="A default that is unsafe inbound or unbounded outbound is a defect, not a neutral starting point"><code>http-server/an-unsafe-or-unbounded-default-is-a-defect</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0097.md">record 0097</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0074.md">record 0074</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-server/src/statics.rs"><code>crates/nvs-server/src/statics.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="health-path-is-off-and-checks-nothing">

## `health_path` is off by default; set, it answers `200` while accepting and `503` while draining, and checks no dependency

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#health-path-is-off-and-checks-nothing"><code>http-server/health-path-is-off-and-checks-nothing</code></a>
</div>

`[server] health_path` is off by default, so no URL is silently reserved. When set it is matched ahead of every mount, answers `200` while the process is accepting and `503` while it is draining, with an empty body, and is skipped by the access log.

It performs **no dependency checks** — a health endpoint that pings the database converts a slow database into a simultaneous outage across every instance — and reports no version or build information. A built-in probe reports that the *process* is alive even when the application fails to compile, where an application-route probe would fail and produce a restart loop that cannot fix a compile error.

`Core\Server::isDraining()` gives an application the same bit for an endpoint of its own; a program that is not being served reads `false`, which is the answer rather than an error. What draining does to the connections still open is [`concurrency/a-drain-closes-a-connection-cleanly`](/docs/rules/concurrency/connections/#a-drain-closes-a-connection-cleanly "A shutdown or a reload closes a connection with a defined code after a drain, never with a reset").

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Php-fpm's <code>ping.path</code> and <code>pm.status_path</code> have a counterpart that reports no status page, no version and no pool statistics</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/listening-and-admission/#the-server-block-is-boot-class" title="The [server] block is Boot-class, listen is one flat array defaulting to 127.0.0.1:8000 in both modes, and the flag is the last word"><code>http-server/the-server-block-is-boot-class</code></a> <a href="/docs/rules/concurrency/connections/#a-drain-closes-a-connection-cleanly" title="A shutdown or a reload closes a connection with a defined code after a drain, never with a reset"><code>concurrency/a-drain-closes-a-connection-cleanly</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0097.md">record 0097</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-server/src/mount.rs"><code>crates/nvs-server/src/mount.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/server-is-draining-answers-a-bool-and-its-readings-agree.nvst"><code>tests/conformance/core/server-is-draining-answers-a-bool-and-its-readings-agree.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="the-trace-id-is-the-request-identifier">

## The trace id exists on every request regardless of sampling, is the only request identifier, is emitted on the response, and an inbound `X-Request-ID` is ignored

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#the-trace-id-is-the-request-identifier"><code>http-server/the-trace-id-is-the-request-identifier</code></a>
</div>

The trace id is the request identifier, and there is no second one. A trace id exists for every request because W3C TraceContext generates one regardless of the sampling decision — sampling governs only whether the trace is *exported* — so the id needed to join a log line to an error page to a proxy log entry is already present on every request, with `[trace] sample` at zero included.

`Core\Server::traceId()` reads it, every log record and every error rendering carries it ([`errors/log-fields`](/docs/rules/errors/diagnostics-and-logging/#log-fields "A record's fields are named and typed, not a stringly bag")), and it is emitted on the response so a proxy can log it with one `log_format` line. An inbound `traceparent` is continued; a missing one is generated.

**An inbound `X-Request-ID` is ignored** and appears nowhere. Honouring it would need a validation rule against log injection for a fact Novis already has, and two identifiers for one fact is how people grep the wrong one.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no <code>X-Request-ID</code> and no <code>$_SERVER['UNIQUE_ID']</code>; the W3C trace id is the one identifier on every log line, error page and response</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/headers-cors-and-cookies/#the-access-log-is-a-mode-default" title="[log] access is a mode default — on in development, off in production — and a 5xx is logged whatever it says"><code>http-server/the-access-log-is-a-mode-default</code></a> <a href="/docs/rules/errors/diagnostics-and-logging/#log-fields" title="A record's fields are named and typed, not a stringly bag"><code>errors/log-fields</code></a> <a href="/docs/rules/observability/metrics/#a-trace-id-exists-for-every-request" title="A trace id exists for every request whatever the sampling decision, and it is the only request identifier"><code>observability/a-trace-id-exists-for-every-request</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0097.md">record 0097</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0076.md">record 0076</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-server/src/trace.rs"><code>crates/nvs-server/src/trace.rs</code></a></dd></div></dl>

</div>
