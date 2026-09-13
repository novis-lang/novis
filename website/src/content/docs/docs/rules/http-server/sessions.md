---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Sessions"
description: "A session is a record its store issued. Four operations, loaded once, written whole only when it changed, and never locked."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/http-server/headers-cors-and-cookies/
  label: "Response headers, CORS and cookies"
next:
  link: /docs/rules/http-server/uploads/
  label: "Uploads"
---

<p class="nv-section-lead">A session is a record its store issued. Four operations, loaded once, written whole only when it changed, and never locked.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">6</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">6</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">0</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">6</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#a-session-store-answers-four-operations">A session is a record its store issued: a backend answers <code>issue</code>, <code>load</code>, <code>save</code> and <code>destroy</code>, and <code>load</code> answering absent is the whole of the strict-id rule</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#session-backend-is-shared-or-db-and-local-is-refused-at-boot"><code>[session] backend</code> admits <code>shared</code> and <code>db</code>, and writing <code>local</code> is a boot refusal naming the file it was written in</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#no-session-block-means-no-store">A tree with no <code>[session]</code> block has no session store, and <code>start</code> throws naming the block to write</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-session-is-loaded-once-and-written-whole">A session record is loaded once at <code>start</code>, written back whole only when it changed, and never locked</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#session-expiry-belongs-to-the-store"><code>[session] ttl</code> is written onto the record and the store expires it; there is no sweeper and no probability of one</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-session-holds-a-secret-only-sealed">A user's own secret lives in their session, sealed under a key ring through <code>setSecret</code> and <code>getSecret</code></a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li></ol>

<div class="nv-rule" id="a-session-store-answers-four-operations">

## A session is a record its store issued: a backend answers `issue`, `load`, `save` and `destroy`, and `load` answering absent is the whole of the strict-id rule

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-session-store-answers-four-operations"><code>http-server/a-session-store-answers-four-operations</code></a>
</div>

A session is one record in a store that can answer *did I issue this id*. A backend answers four
operations and no more: `issue` mints an identifier this store has never issued, writes an empty
record under it and answers the id; `load` answers the record under an id, or **absent**; `save`
replaces the record under an id, refreshing its expiry; `destroy` forgets it.

**`load` answering absent is the whole of the strict-id rule.** An id the store did not issue, one
it issued and has since expired, and one an attacker minted are the same answer, and `start`
responds to all three identically: discard it and `issue` a fresh one. There is no separate
`validateId`, because a second question is a second thing that can disagree with the first. A
presented id that is not 22 base64url characters is absent by construction, before any round trip.

The identifier is 128 bits from the CSPRNG `Core\Crypto` draws from, rendered base64url — not a
counter and not a hash of anything the client supplied: an id is a bearer credential for the
length of its life, and the only property it needs is that guessing one is not a strategy. The
record crosses the store boundary as the byte carrier a `Core\Cache` entry does
([`concurrency/a-cached-value-is-copied-across-the-boundary`](/docs/rules/concurrency/deferred-and-cross-request-state/#a-cached-value-is-copied-across-the-boundary "A value is copied into the cache and copied back out, by the same graph copy the isolate boundary uses")). `start` is the one member that
talks to the store ([`core-api/session-roster`](/docs/rules/core-api/lifetimes-and-absences/#session-roster "The session roster is start and the members that work on the record it loaded, and a member called before start throws naming it")).

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no <code>SessionHandlerInterface</code> and no <code>validateId</code>; an id the store never issued, one that expired and one an attacker minted are all the same absent answer, and <code>start</code> issues a fresh record for each</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-api/lifetimes-and-absences/#session-roster" title="The session roster is start and the members that work on the record it loaded, and a member called before start throws naming it"><code>core-api/session-roster</code></a> <a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#session-is-started-explicitly" title="A session is opened by calling Core\Session::start, and there is no ambient session array"><code>core-classes/session-is-started-explicitly</code></a> <a href="/docs/rules/http-server/sessions/#session-backend-is-shared-or-db-and-local-is-refused-at-boot" title="[session] backend admits shared and db, and writing local is a boot refusal naming the file it was written in"><code>http-server/session-backend-is-shared-or-db-and-local-is-refused-at-boot</code></a> <a href="/docs/rules/http-server/sessions/#session-expiry-belongs-to-the-store" title="[session] ttl is written onto the record and the store expires it; there is no sweeper and no probability of one"><code>http-server/session-expiry-belongs-to-the-store</code></a> <a href="/docs/rules/concurrency/deferred-and-cross-request-state/#a-cached-value-is-copied-across-the-boundary" title="A value is copied into the cache and copied back out, by the same graph copy the isolate boundary uses"><code>concurrency/a-cached-value-is-copied-across-the-boundary</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0139.md">record 0139</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0124.md">record 0124</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0059.md">record 0059</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/session-start-takes-the-presented-identifier-or-none-at-all.nvst"><code>tests/conformance/core/session-start-takes-the-presented-identifier-or-none-at-all.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/session.rs"><code>crates/nvs-stdlib/src/session.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="session-backend-is-shared-or-db-and-local-is-refused-at-boot">

## `[session] backend` admits `shared` and `db`, and writing `local` is a boot refusal naming the file it was written in

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#session-backend-is-shared-or-db-and-local-is-refused-at-boot"><code>http-server/session-backend-is-shared-or-db-and-local-is-refused-at-boot</code></a>
</div>

```toml
[session]
backend = "shared"      # or "db"; there is no third value and no default that reaches a store
ttl     = "2h"          # how long an untouched record survives; the store enforces it
cookie  = "nvsid"       # the name the identifier rides under
```

`shared` is the coherent tier reached at `[cache.shared] url`
([`config/cache-shared-is-the-grant-over-the-configured-store`](/docs/rules/config/stores-and-caches/#cache-shared-is-the-grant-over-the-configured-store "A store an operator configured is authorized by the configuring — cache.shared is the grant, unscoped, and asks no address")); `db` is a table in a
`[db.<name>]`. `backend = "local"` is `E0626`, and its note names
[`concurrency/the-local-tier-cannot-hold-what-must-be-coherent`](/docs/rules/concurrency/deferred-and-cross-request-state/#the-local-tier-cannot-hold-what-must-be-coherent "Anything a program relies on the value of goes to the shared tier, and a weaker tier is not offered for it") and the file the key was
written in. That is what "enforced rather than documented" means: the value is refused where it
is written, not where it is used, so a deployment cannot be running on a per-core session store
while believing otherwise. A generic unknown-value refusal would not do it — an operator who wrote
`local` because APCu was where their sessions lived needs the sentence explaining why the fast
answer is the wrong one.

The roster is a type with no local variant, and every store operation is written against the
shared tier's connection rather than the tier enum, so the per-core map is absent rather than
merely unselected. The key is `System` class and `Boot` apply ([`config/system-means-a-request-may-not-set-it`](/docs/rules/config/changeability-classes/#system-means-a-request-may-not-set-it "A directive is System when changing it from inside a request would affect something other than that request, and that is all System means")): where a fleet's sessions live is a
deployment decision, and moving it mid-flight would strand every live session. An absent block is
not a default backend ([`http-server/no-session-block-means-no-store`](/docs/rules/http-server/sessions/#no-session-block-means-no-store "A tree with no [session] block has no session store, and start throws naming the block to write")).

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>session.save_handler = files</code> and an APCu-backed handler have no counterpart; the choices are the shared tier or the database, and <code>local</code> is <code>E0626</code> where it is written rather than a session that forgets people</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/concurrency/deferred-and-cross-request-state/#the-local-tier-cannot-hold-what-must-be-coherent" title="Anything a program relies on the value of goes to the shared tier, and a weaker tier is not offered for it"><code>concurrency/the-local-tier-cannot-hold-what-must-be-coherent</code></a> <a href="/docs/rules/http-server/sessions/#no-session-block-means-no-store" title="A tree with no [session] block has no session store, and start throws naming the block to write"><code>http-server/no-session-block-means-no-store</code></a> <a href="/docs/rules/config/stores-and-caches/#cache-shared-is-the-grant-over-the-configured-store" title="A store an operator configured is authorized by the configuring — cache.shared is the grant, unscoped, and asks no address"><code>config/cache-shared-is-the-grant-over-the-configured-store</code></a> <a href="/docs/rules/config/changeability-classes/#system-means-a-request-may-not-set-it" title="A directive is System when changing it from inside a request would affect something other than that request, and that is all System means"><code>config/system-means-a-request-may-not-set-it</code></a> <a href="/docs/rules/core-api/lifetimes-and-absences/#two-cache-tiers" title="Cross-request state is reached through a member per tier, each with its own contract, never one API with a flag"><code>core-api/two-cache-tiers</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0139.md">record 0139</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0059.md">record 0059</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0005.md">record 0005</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/src/session.rs"><code>crates/nvs-config/src/session.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/session.rs"><code>crates/nvs-stdlib/src/session.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-session-survives-a-request-on-another-core.nvst"><code>tests/conformance/core/a-session-survives-a-request-on-another-core.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/session-start-reaches-the-configured-shared-store-and-never-one-of-its-own.nvst"><code>tests/conformance/core/session-start-reaches-the-configured-shared-store-and-never-one-of-its-own.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="no-session-block-means-no-store">

## A tree with no `[session]` block has no session store, and `start` throws naming the block to write

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#no-session-block-means-no-store"><code>http-server/no-session-block-means-no-store</code></a>
</div>

`[session]` absent is not "sessions off with a default backend". It is a session surface that
throws on `Core\Session::start`, naming the block to write. The premise that a deployment with
nothing configured is safe ([`http-server/an-unsafe-or-unbounded-default-is-a-defect`](/docs/rules/http-server/listening-and-admission/#an-unsafe-or-unbounded-default-is-a-defect "A default that is unsafe inbound or unbounded outbound is a defect, not a neutral starting point")) has
one answer for a store nobody chose: no store. Every other `Core\Session` member already throws
until `start` has run ([`core-classes/session-is-started-explicitly`](/docs/rules/core-classes/codecs-sessions-and-signatures/#session-is-started-explicitly "A session is opened by calling Core\Session::start, and there is no ambient session array")), so a program that
cannot use sessions can say so at the root.

A block that names `shared` but a tree with no `[cache.shared]` is the same shape one step later:
`start` reaches the one store that block names, and says which block is missing when there is
none, rather than opening a store of its own.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>session_start()</code> with nothing configured writes files under <code>session.save_path</code>; here a store nobody chose is no store, and the first <code>start</code> says which block would choose one</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/sessions/#session-backend-is-shared-or-db-and-local-is-refused-at-boot" title="[session] backend admits shared and db, and writing local is a boot refusal naming the file it was written in"><code>http-server/session-backend-is-shared-or-db-and-local-is-refused-at-boot</code></a> <a href="/docs/rules/http-server/listening-and-admission/#an-unsafe-or-unbounded-default-is-a-defect" title="A default that is unsafe inbound or unbounded outbound is a defect, not a neutral starting point"><code>http-server/an-unsafe-or-unbounded-default-is-a-defect</code></a> <a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#session-is-started-explicitly" title="A session is opened by calling Core\Session::start, and there is no ambient session array"><code>core-classes/session-is-started-explicitly</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0139.md">record 0139</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0074.md">record 0074</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/session-start-refuses-a-tree-that-configured-no-store.nvst"><code>tests/conformance/core/session-start-refuses-a-tree-that-configured-no-store.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-session-is-loaded-once-and-written-whole">

## A session record is loaded once at `start`, written back whole only when it changed, and never locked

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-session-is-loaded-once-and-written-whole"><code>http-server/a-session-is-loaded-once-and-written-whole</code></a>
</div>

`start` reads the record once; `set`, `remove` and `clear` mutate the copy in the request's own
heap; the record is written back when the program that opened it ends — after any exit hooks,
since a hook is user code that may still write — and immediately for `regenerate` and `destroy`.
**Writing only when the record changed** keeps a read-only request off the write path: a request
that starts a session and reads it makes one round trip, not two.

There is no lock. Two requests writing one session concurrently is last-write-wins over the
**whole record**, and this is stated rather than repaired. A lock held for the length of a
request is a cross-request channel ([`security/closed-doors`](/docs/rules/security/closed-doors/#closed-doors "Four doors are closed by construction, and no configuration reopens any of them")), and a request that dies holding
one wedges every later request for that session until it expires. The cost belongs to a narrow
case — two concurrent requests that write *different keys* of one session lose one of the two
writes — and an application for which that matters has state that is not session state; locks,
counters and idempotency keys go to the shared tier or the database directly
([`concurrency/cross-request-state-is-explicit`](/docs/rules/concurrency/deferred-and-cross-request-state/#cross-request-state-is-explicit "A value outlives the request that made it only by being put into a named store")).

Memory is O(in-flight): one encoded record per request that started a session, released with the
request heap. A cancelled task is the one end that sends nothing, because the send parks.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>PHP holds an exclusive lock on the session for the length of the request, so two requests from one browser serialize; here there is no lock, and two concurrent writes to one session are last-write-wins over the whole record</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/sessions/#a-session-store-answers-four-operations" title="A session is a record its store issued: a backend answers issue, load, save and destroy, and load answering absent is the whole of the strict-id rule"><code>http-server/a-session-store-answers-four-operations</code></a> <a href="/docs/rules/security/closed-doors/#closed-doors" title="Four doors are closed by construction, and no configuration reopens any of them"><code>security/closed-doors</code></a> <a href="/docs/rules/concurrency/deferred-and-cross-request-state/#cross-request-state-is-explicit" title="A value outlives the request that made it only by being put into a named store"><code>concurrency/cross-request-state-is-explicit</code></a> <a href="/docs/rules/core-api/lifetimes-and-absences/#session-roster" title="The session roster is start and the members that work on the record it loaded, and a member called before start throws naming it"><code>core-api/session-roster</code></a> <a href="/docs/rules/http-server/containment/#a-requests-blast-radius-is-bounded-at-four-tiers" title="Nothing a request can send terminates or wedges a worker: its blast radius is bounded at four named tiers, and the residue is one stated fault class"><code>http-server/a-requests-blast-radius-is-bounded-at-four-tiers</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0139.md">record 0139</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0052.md">record 0052</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/session.rs"><code>crates/nvs-stdlib/src/session.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="session-expiry-belongs-to-the-store">

## `[session] ttl` is written onto the record and the store expires it; there is no sweeper and no probability of one

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#session-expiry-belongs-to-the-store"><code>http-server/session-expiry-belongs-to-the-store</code></a>
</div>

`[session] ttl` is written onto the entry — `SET … EX` on the shared tier; an `expires_at` column
and a `DELETE … WHERE expires_at < now()` in the database backend's own schema, run by the worker
[`concurrency/who-runs-a-job-is-configuration`](/docs/rules/concurrency/running-a-job/#who-runs-a-job-is-configuration "Which process runs a job is configuration, and every spelling drives the identical isolate") already names. There is no `session.gc_probability` equivalent and no sweep on a
percentage of requests: PHP's pair exists because a filesystem cannot expire anything by itself,
and both backends here can.

An expired record is absent, so `load` already answers it and `start` already issues a fresh id
([`http-server/a-session-store-answers-four-operations`](/docs/rules/http-server/sessions/#a-session-store-answers-four-operations "A session is a record its store issued: a backend answers issue, load, save and destroy, and load answering absent is the whole of the strict-id rule")). `save` refreshes the expiry, so
`ttl` bounds how long an *untouched* record survives. Expiry needs no code path of its own, which
is the point of choosing backends that expire.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>session.gc_probability</code> and <code>session.gc_divisor</code> have no counterpart, because both backends can expire an entry themselves and a filesystem cannot</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/sessions/#a-session-store-answers-four-operations" title="A session is a record its store issued: a backend answers issue, load, save and destroy, and load answering absent is the whole of the strict-id rule"><code>http-server/a-session-store-answers-four-operations</code></a> <a href="/docs/rules/http-server/sessions/#session-backend-is-shared-or-db-and-local-is-refused-at-boot" title="[session] backend admits shared and db, and writing local is a boot refusal naming the file it was written in"><code>http-server/session-backend-is-shared-or-db-and-local-is-refused-at-boot</code></a> <a href="/docs/rules/concurrency/running-a-job/#who-runs-a-job-is-configuration" title="Which process runs a job is configuration, and every spelling drives the identical isolate"><code>concurrency/who-runs-a-job-is-configuration</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0139.md">record 0139</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/session.rs"><code>crates/nvs-stdlib/src/session.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-session-holds-a-secret-only-sealed">

## A user's own secret lives in their session, sealed under a key ring through `setSecret` and `getSecret`

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-session-holds-a-secret-only-sealed"><code>http-server/a-session-holds-a-secret-only-sealed</code></a>
</div>

A user's own secret — the access and refresh token a web application holds on their behalf — lives in their
session, through `Core\Session::setSecret(string $key, secret string $value, array<secret bytes> $keys)`
and `Core\Session::getSecret(string $key, array<secret bytes> $keys)`, and only **sealed**. Static like
every member of that class ([`core-api/session-roster`](/docs/rules/core-api/lifetimes-and-absences/#session-roster "The session roster is start and the members that work on the record it loaded, and a member called before start throws naming it")): the session is the request's, not an object
a program holds. The construction is the
cache's ([`concurrency/a-secret-is-cached-only-sealed`](/docs/rules/concurrency/deferred-and-cross-request-state/#a-secret-is-cached-only-sealed "A secret reaches a cache only as ciphertext, through putSecret and getSecret and a key ring")) under the session door's own domain byte, so a
value sealed for a cache never opens as a session value and the reverse.

It is the session and not a cache tier because a cache may evict at any time and a user would be logged out
by a footprint decision. There is no `ttl`: a session value lives as long as its session
([`http-server/session-expiry-belongs-to-the-store`](/docs/rules/http-server/sessions/#session-expiry-belongs-to-the-store "[session] ttl is written onto the record and the store expires it; there is no sweeper and no probability of one")), and the sealed plaintext carries no expiry of its
own.

**The additional data is the domain byte ‖ the app ‖ the key, and not the session id**, because
`regenerate` issues a new id over the same record and a value bound to the old id would stop opening at
exactly the moment a login hardens. Moving a ciphertext between two sessions takes write access to the
store, which already means owning every session in it.

`get` answers `null` for a sealed value and `set` still refuses a `secret`, so the sealed pair is the only
door here too; a sealed value that does not open under the ring is absent rather than an error. The record
crosses the store as the byte carrier it already is
([`http-server/a-session-store-answers-four-operations`](/docs/rules/http-server/sessions/#a-session-store-answers-four-operations "A session is a record its store issued: a backend answers issue, load, save and destroy, and load answering absent is the whole of the strict-id rule")), so no `secret` reaches the store and
[`security/secret-crosses-no-boundary`](/docs/rules/security/secrets/#secret-crosses-no-boundary "A secret value is refused at the one graph copy, so it reaches neither serialize nor any spawn") is unchanged.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>$_SESSION</code> holds whatever it is given in the clear, and a token in it is readable by anything that can read the store</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/concurrency/deferred-and-cross-request-state/#a-secret-is-cached-only-sealed" title="A secret reaches a cache only as ciphertext, through putSecret and getSecret and a key ring"><code>concurrency/a-secret-is-cached-only-sealed</code></a> <a href="/docs/rules/http-server/sessions/#a-session-store-answers-four-operations" title="A session is a record its store issued: a backend answers issue, load, save and destroy, and load answering absent is the whole of the strict-id rule"><code>http-server/a-session-store-answers-four-operations</code></a> <a href="/docs/rules/http-server/sessions/#session-expiry-belongs-to-the-store" title="[session] ttl is written onto the record and the store expires it; there is no sweeper and no probability of one"><code>http-server/session-expiry-belongs-to-the-store</code></a> <a href="/docs/rules/security/secrets/#secret-crosses-no-boundary" title="A secret value is refused at the one graph copy, so it reaches neither serialize nor any spawn"><code>security/secret-crosses-no-boundary</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0181.md">record 0181</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/session.rs"><code>crates/nvs-stdlib/src/session.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/session-a-secret-round-trips-sealed-and-survives-regenerate.nvst"><code>tests/conformance/core/session-a-secret-round-trips-sealed-and-survives-regenerate.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/session-get-answers-null-for-a-sealed-value.nvst"><code>tests/conformance/core/session-get-answers-null-for-a-sealed-value.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/session-set-refuses-a-secret-and-names-set-secret.nvst"><code>tests/conformance/reject/session-set-refuses-a-secret-and-names-set-secret.nvst</code></a></dd></div></dl>

</div>
