---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Lifetimes, signatures and what is absent"
description: "Anything with a lifetime is an object, a signature is taken over a payload, and every removed PHP built-in is accounted for by name."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/core-api/shape-parameters/
  label: "Shape parameters"
next:
  link: /docs/rules/core-classes/
  label: "The Core classes"
---

<p class="nv-section-lead">Anything with a lifetime is an object, a signature is taken over a payload, and every removed PHP built-in is accounted for by name.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">10</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">7</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">3</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">5</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#a-lifetime-is-an-object">Anything with a lifetime is an object, and <code>Core</code> never hands back a handle</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-lifetime-is-written">A signature's lifetime is a required key and <code>null</code> is the forever spelling</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#signing-is-over-a-payload">A signature is taken over a structured payload, never over assembled text, and a URL is signed through the canonical form its class already defines</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#one-refusal-except-expiry">Every way of failing a signature check is one refusal, and expiry is the single distinguishable one</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#each-door-takes-a-different-thing">Two members are not a second spelling when each takes a different thing and answers a different thing</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#two-cache-tiers">Cross-request state is reached through a member per tier, each with its own contract, never one API with a flag</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#one-temporary-directory-member">The whole temporary-file surface is one member handing out an owned directory</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#session-roster">The session roster is <code>start</code> and the members that work on the record it loaded, and a member called before <code>start</code> throws naming it</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#reference-card">An implemented <code>Core</code> member carries its reference card in the registry declaration beside its code</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#removals">A PHP built-in is absent for one of four standing reasons, and every removed name is accounted for by name</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li></ol>

<div class="nv-rule" id="a-lifetime-is-an-object">

## Anything with a lifetime is an object, and `Core` never hands back a handle

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-lifetime-is-an-object"><code>core-api/a-lifetime-is-an-object</code></a>
</div>

Anything with a lifetime is an object. There is no `resource` atom in any `Core` signature, no integer
handle, and no `$link`-first calling convention; a file, a connection, a compression stream and a hash
context are all objects with methods.

A handle has nowhere to enforce a capability and nothing to hang an API on, so every operation on it
becomes a free function taking the handle first — which is how PHP ended up with `fopen` beside
`SplFileObject` beside `DirectoryIterator` ([`core-api/one-paradigm-per-operation`](/docs/rules/core-api/one-way-to-do-each-thing/#one-paradigm-per-operation "No operation is reachable two ways, and a domain class's statics never mirror an object's own methods")). An object has both
a place for the capability check and a place for the methods. The `resource` atom survives in the type
grammar ([`types/grammar`](/docs/rules/types/declarations-and-numbers/#grammar "The type grammar is a closed set of atoms under unions and intersections")) only for opaque handles an extension supplies, and `Core` never produces
one.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no <code>resource</code>, no integer handle and no <code>$link</code>-first convention anywhere in <code>Core</code></p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-api/one-way-to-do-each-thing/#one-paradigm-per-operation" title="No operation is reachable two ways, and a domain class's statics never mirror an object's own methods"><code>core-api/one-paradigm-per-operation</code></a> <a href="/docs/rules/classes/no-magic/#no-destructors" title="There are no destructors: cleanup is an explicit method call the holder makes"><code>classes/no-destructors</code></a> <a href="/docs/rules/types/declarations-and-numbers/#grammar" title="The type grammar is a closed set of atoms under unions and intersections"><code>types/grammar</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0063.md">record 0063</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0003.md">record 0003</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/registry.rs"><code>crates/nvs-stdlib/src/registry.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-lifetime-is-written">

## A signature's lifetime is a required key and `null` is the forever spelling

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#a-lifetime-is-written"><code>core-api/a-lifetime-is-written</code></a>
</div>

A signature's lifetime is a **required key holding a nullable value**: both a written instant and a written
`null` are spellings, and omitting the key does not compile. `null` is the forever spelling.

An omission is not a default. A permanent signed link is a permanent bearer credential — written into
browser history, `Referer` headers, proxy logs and chat unfurls, and it never stops being one — so
"forever" should be something a person typed rather than something a missing key chose. Making expiry
mandatory would be the other answer, and it is rejected here as belonging to a specific token format rather
than to the general operation.

This is the one place a required key deliberately holds a nullable value, and it reads differently from
[`core-api/a-written-null-removes`](/docs/rules/core-api/parameters-and-options/#a-written-null-removes "Wherever a Core member admits a written null it means remove, and  is never a removal spelling"): there `null` clears an optional field, here it selects the
unbounded lifetime on a field that must be written either way.

**Designed, not shipped.**

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-api/lifetimes-and-absences/#signing-is-over-a-payload" title="A signature is taken over a structured payload, never over assembled text, and a URL is signed through the canonical form its class already defines"><code>core-api/signing-is-over-a-payload</code></a> <a href="/docs/rules/core-api/parameters-and-options/#a-written-null-removes" title="Wherever a Core member admits a written null it means remove, and  is never a removal spelling"><code>core-api/a-written-null-removes</code></a> <a href="/docs/rules/core-api/parameters-and-options/#omission-is-not-a-written-null" title="An options bag tells an omitted key from a written null"><code>core-api/omission-is-not-a-written-null</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0146.md">record 0146</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0096.md">record 0096</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0060.md">record 0060</a></dd></div></dl>

</div>

<div class="nv-rule" id="signing-is-over-a-payload">

## A signature is taken over a structured payload, never over assembled text, and a URL is signed through the canonical form its class already defines

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#signing-is-over-a-payload"><code>core-api/signing-is-over-a-payload</code></a>
</div>

Signing takes a **structured payload**, never assembled text. The signing member canonicalizes the payload
itself — keys sorted, each value encoded with its type — so there is no assembled string for the signing
side and the verifying side to disagree about, and the lifetime travels *inside* the signed bytes rather
than beside them. The token is URL-safe by construction, so nothing downstream escapes it again.

A URL is a payload the URI class already canonicalizes. Signing a URL signs the normalization its own
equivalence comparison defines — scheme and host folded to lower case, escape digits upper-cased, an escape
spelling an unreserved character decoded, dot segments removed — reached a second time rather than invented
a second time. That is the whole point: every framework's signed-URL bug class comes from a canonical form
used by nothing but the signature, so nothing else exercises it and no other test constrains it. Reusing a
form that is already load-bearing turns a new risk into a second caller of an existing one, and a drift
becomes one bug in one function that the existing corpus catches.

Every component present is covered, so appending any parameter invalidates the signature; there is no
option naming which parameters are signed, because that option is where every framework's bypass has lived.
The fragment is never signed, since it is not sent to the server.

**Two doors of the three are on disk.** `Core\Signature` signs and verifies a payload map, and
`$uri->sign`/`$uri->verifySignature` sign the form `$uri->compareTo` normalizes, reserving the `_sig`
query parameter for the token. `Core\Router`'s pair — the one that signs a route's name and parameters,
so a signature survives a remount — is not registered yet.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-api/lifetimes-and-absences/#a-lifetime-is-written" title="A signature's lifetime is a required key and null is the forever spelling"><code>core-api/a-lifetime-is-written</code></a> <a href="/docs/rules/core-api/lifetimes-and-absences/#one-refusal-except-expiry" title="Every way of failing a signature check is one refusal, and expiry is the single distinguishable one"><code>core-api/one-refusal-except-expiry</code></a> <a href="/docs/rules/core-api/lifetimes-and-absences/#each-door-takes-a-different-thing" title="Two members are not a second spelling when each takes a different thing and answers a different thing"><code>core-api/each-door-takes-a-different-thing</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0146.md">record 0146</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0060.md">record 0060</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0058.md">record 0058</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0097.md">record 0097</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/uri.rs"><code>crates/nvs-stdlib/src/uri.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="one-refusal-except-expiry">

## Every way of failing a signature check is one refusal, and expiry is the single distinguishable one

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#one-refusal-except-expiry"><code>core-api/one-refusal-except-expiry</code></a>
</div>

Every way of not being authentic is **one error with one sentence**: text that is not a token, a payload
too short, one flipped bit, a key retired past the end of the ring, a missing signature parameter, two of
them. A distinguishable "wrong key" would say which key of a rotating ring a forgery should be aimed at.

**Expiry is the one distinguishable failure**, and the exception is safe because of the *ordering*: the
signature is checked first and the clock only after, so the expired error is reachable only by someone
already holding a valid signature and discloses nothing they do not have. That buys the distinction an
application actually needs — "this link has expired, request another" against "this link is not valid" —
without a disclosure, which is why the ordering is a rule rather than an implementation detail.

Verification answers nothing and throws, because there are no claims to return and a `bool` is a value a
caller can drop on the floor ([`core-api/failure-throws`](/docs/rules/core-api/one-way-to-do-each-thing/#failure-throws "A Core member throws on failure and returns ?T for absence; false is never an answer")).

**Designed, not shipped.**

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-api/lifetimes-and-absences/#signing-is-over-a-payload" title="A signature is taken over a structured payload, never over assembled text, and a URL is signed through the canonical form its class already defines"><code>core-api/signing-is-over-a-payload</code></a> <a href="/docs/rules/core-api/one-way-to-do-each-thing/#failure-throws" title="A Core member throws on failure and returns ?T for absence; false is never an answer"><code>core-api/failure-throws</code></a> <a href="/docs/rules/errors/ambiguous-input/#ambiguous-input-refused" title="Ambiguous input is refused whole, never repaired"><code>errors/ambiguous-input-refused</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0146.md">record 0146</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0060.md">record 0060</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/signed_cookie.rs"><code>crates/nvs-stdlib/src/signed_cookie.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="each-door-takes-a-different-thing">

## Two members are not a second spelling when each takes a different thing and answers a different thing

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#each-door-takes-a-different-thing"><code>core-api/each-door-takes-a-different-thing</code></a>
</div>

Two members are not a second spelling of one operation when **each takes a different thing and answers a
different thing.** A payload signer takes a map and answers a token; a URL's own signing member takes a URL
and answers a URL; a router's signing pair takes a route identity and answers a path. None substitutes for
another, so none is the duplication [`core-api/one-paradigm-per-operation`](/docs/rules/core-api/one-way-to-do-each-thing/#one-paradigm-per-operation "No operation is reachable two ways, and a domain class's statics never mirror an object's own methods") refuses.

The test is on the *types at the boundary*, not on what the implementation shares underneath. Two members
may compile to the same construction and stay two members, and a class may be implemented as another plus
one more step without becoming the same member. Conversely, two members that take the same thing and answer
the same thing are one member with a flag, whatever they are named — that is the case R17 exists to catch.

A program that assembles a URL by hand, signs the text and hand-rolls the parameter walk has written the
URL member badly rather than reached it twice, which is the difference between a second door and a second
API.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-api/one-way-to-do-each-thing/#one-paradigm-per-operation" title="No operation is reachable two ways, and a domain class's statics never mirror an object's own methods"><code>core-api/one-paradigm-per-operation</code></a> <a href="/docs/rules/core-api/naming-and-shape/#one-name-one-signature" title="One name has one signature, and two behaviours need two names"><code>core-api/one-name-one-signature</code></a> <a href="/docs/rules/core-api/lifetimes-and-absences/#two-cache-tiers" title="Cross-request state is reached through a member per tier, each with its own contract, never one API with a flag"><code>core-api/two-cache-tiers</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0146.md">record 0146</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0063.md">record 0063</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0060.md">record 0060</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/signed_cookie.rs"><code>crates/nvs-stdlib/src/signed_cookie.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/registry.rs"><code>crates/nvs-stdlib/src/registry.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="two-cache-tiers">

## Cross-request state is reached through a member per tier, each with its own contract, never one API with a flag

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#two-cache-tiers"><code>core-api/two-cache-tiers</code></a>
</div>

Cross-request state is reached through **a member per tier, each with its own contract**, not one member
with a flag. The local tier is per-core and in-process: any entry may be absent at any time for any reason,
and a write on one core is not visible on another. The process tier is one store per serving process,
coherent across every core of it, in memory only and gone when the process ends
([`concurrency/the-process-tier-is-one-store-per-process`](/docs/rules/concurrency/deferred-and-cross-request-state/#the-process-tier-is-one-store-per-process "Core\Cache::process() is one store per serving process, coherent across its cores and gone when it ends")). The shared tier is a real store over the
network, coherent across cores and machines, and gated by the capability that names the store an operator
configured. No member takes an argument, so there is no flag to have written.

A program that would be incorrect if a read returned nothing is using the wrong tier, and the whole value of
a member per tier is that the choice is made in the source and visible in review. One name covering
different guarantees invites using the weaker one by accident — the same reason a generic sanitizer is
refused. A tier is added by adding a member, which is why this rule's id counts two while its title does
not: an id is the fragment's path and a path does not move.

Neither weak tier needs a capability, because a capability is checked at the door to an *effect* and neither
has a door: nothing leaves the process, no name is resolved and no file is opened. What is left to bound is
footprint, and a configured size cap is the instrument for that — `[cache.local] max_size` per core and
`[cache.process] max_size` per process; a boolean grant is not one, and adding it would price a tier as an
authority question every deployment then has to answer.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>apcu_*</code> and its shared-memory relatives are gone; the tier a program wants is the member it calls</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-api/naming-and-shape/#one-name-one-signature" title="One name has one signature, and two behaviours need two names"><code>core-api/one-name-one-signature</code></a> <a href="/docs/rules/core-api/lifetimes-and-absences/#each-door-takes-a-different-thing" title="Two members are not a second spelling when each takes a different thing and answers a different thing"><code>core-api/each-door-takes-a-different-thing</code></a> <a href="/docs/rules/core-api/one-way-to-do-each-thing/#one-paradigm-per-operation" title="No operation is reachable two ways, and a domain class's statics never mirror an object's own methods"><code>core-api/one-paradigm-per-operation</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0059.md">record 0059</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0142.md">record 0142</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0118.md">record 0118</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0181.md">record 0181</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/cache-local-and-shared-are-two-members-with-two-contracts.nvst"><code>tests/conformance/core/cache-local-and-shared-are-two-members-with-two-contracts.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/cache-a-local-entry-may-be-absent-at-any-time.nvst"><code>tests/conformance/core/cache-a-local-entry-may-be-absent-at-any-time.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/cache-local-forgets-an-entry-rather-than-failing-the-write.nvst"><code>tests/conformance/core/cache-local-forgets-an-entry-rather-than-failing-the-write.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="one-temporary-directory-member">

## The whole temporary-file surface is one member handing out an owned directory

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#one-temporary-directory-member"><code>core-api/one-temporary-directory-member</code></a>
</div>

One member is the whole temporary-file surface: it hands the caller an owned directory, and the program
names files inside it with ordinary writes to paths it got back. There is no member that hands out a single
temporary file.

A program that needs one temporary file needs somewhere to put the second, so a per-file member is the
shape that gets called in a loop and leaves the cleanup question open at every call. A directory answers
both at once: it is one thing to own, one thing to sweep, and the paths inside it need no further
authority beyond the one the member already granted.

The cost is one extra join at each call site that genuinely wanted a single file.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>tmpfile</code>, <code>tempnam</code> and <code>sys_get_temp_dir</code> collapse into one member, and there is no <code>temporaryFile</code></p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-api/naming-and-shape/#one-name-one-signature" title="One name has one signature, and two behaviours need two names"><code>core-api/one-name-one-signature</code></a> <a href="/docs/rules/core-api/one-way-to-do-each-thing/#one-paradigm-per-operation" title="No operation is reachable two ways, and a domain class's statics never mirror an object's own methods"><code>core-api/one-paradigm-per-operation</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0131.md">record 0131</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0063.md">record 0063</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/io-a-temporary-directory-is-made-not-named.nvst"><code>tests/conformance/core/io-a-temporary-directory-is-made-not-named.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/io.rs"><code>crates/nvs-stdlib/src/io.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="session-roster">

## The session roster is `start` and the members that work on the record it loaded, and a member called before `start` throws naming it

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#session-roster"><code>core-api/session-roster</code></a>
</div>

The session surface is `start` plus the members that work on the record it loaded — read, write, remove,
clear, regenerate, destroy, and the sealed pair [`http-server/a-session-holds-a-secret-only-sealed`](/docs/rules/http-server/sessions/#a-session-holds-a-secret-only-sealed "A user's own secret lives in their session, sealed under a key ring through setSecret and getSecret")
adds. `start` is the only one that talks to the store: it is where a presented identifier is checked and
the record is loaded, and the rest operate on the record already in hand.

**A member called before `start` throws, naming the member that opens one.** That single rule is what a
per-request "this request uses sessions" declaration was buying — the fact is a line in the source — and it
is worth nothing if the first read can silently start one. One rule for all of them means they agree
rather than each growing a refusal of its own — the sealed pair answers it before it looks at a key ring
— and the throw is catchable at the root, so a program that cannot use sessions can say so.

`regenerate` issues a new identifier, moves the record to it and destroys the old entry, in that order, and
takes no argument: PHP's delete-old-session flag chose between a fixation window and a lost session, and
only one of those is correct.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>$_SESSION</code> does not exist and no member silently starts a session — <code>session_start</code> has to be written</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-api/one-way-to-do-each-thing/#failure-throws" title="A Core member throws on failure and returns ?T for absence; false is never an answer"><code>core-api/failure-throws</code></a> <a href="/docs/rules/core-api/what-belongs-in-core/#reserved-namespace" title="The Core namespace is the compiler's: nothing may declare under it and its rosters are closed while compiling"><code>core-api/reserved-namespace</code></a> <a href="/docs/rules/statements/where-state-lives/#no-host-populated-variables" title="No variable is ever populated by the host; every superglobal is a Core class member"><code>statements/no-host-populated-variables</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0139.md">record 0139</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0012.md">record 0012</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0124.md">record 0124</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/session-a-member-called-before-start-throws-naming-it.nvst"><code>tests/conformance/core/session-a-member-called-before-start-throws-naming-it.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/session-every-member-refuses-a-record-nobody-opened.nvst"><code>tests/conformance/core/session-every-member-refuses-a-record-nobody-opened.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/session-the-refusal-before-start-is-catchable-at-the-root.nvst"><code>tests/conformance/core/session-the-refusal-before-start-is-catchable-at-the-root.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/session.rs"><code>crates/nvs-stdlib/src/session.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="reference-card">

## An implemented `Core` member carries its reference card in the registry declaration beside its code

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#reference-card"><code>core-api/reference-card</code></a>
</div>

An implemented `Core` member's reference documentation lives in its registry declaration, next to the code
it documents: a short description of one or two sentences, a name and description per parameter — and for a
shape-typed parameter, each key's type and description — a return description, and a list of thrown errors,
each described. An enum carries a card of its own with one line per case, and a constant carries one
sentence, since a constant has a value and no signature.

The registry is the one artifact that provably matches shipped behaviour, because it is the data the
runtime dispatches on; and it already has to carry every parameter's name
([`core-api/parameters-are-callable-by-name`](/docs/rules/core-api/parameters-and-options/#parameters-are-callable-by-name "Every Core parameter is callable by the name the spec writes, and that name is compatibility surface")), so a documentation scheme that put descriptions anywhere
else would create the duplicate that name guard exists to prevent. **Every row carries its card** — a
member without one fails the crate's tests, so a member lands documented or does not land.

Extended prose is deliberately excluded. Long-form text inside Rust string literals is the worst reading
surface available, so anything beyond the reference card stays in the website's pages. The cost is static
strings in the binary — per process, not per request, on the order of a few hundred bytes per documented
member — which is the cheap side of the trade and strippable behind a build feature if a deployment ever
cares.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-api/shape-parameters/#field-wise-precedence" title="Where the registry and the spec both describe an implemented member, precedence is field by field"><code>core-api/field-wise-precedence</code></a> <a href="/docs/rules/core-api/parameters-and-options/#parameters-are-callable-by-name" title="Every Core parameter is callable by the name the spec writes, and that name is compatibility surface"><code>core-api/parameters-are-callable-by-name</code></a> <a href="/docs/rules/core-api/shape-parameters/#shape-parameter" title="A fixed-key shape parameter is one type carrying its arms, and it is only ever a whole parameter"><code>core-api/shape-parameter</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0117.md">record 0117</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0011.md">record 0011</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0051.md">record 0051</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0063.md">record 0063</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/registry.rs"><code>crates/nvs-stdlib/src/registry.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-cli/tests/meta.rs"><code>crates/nvs-cli/tests/meta.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="removals">

## A PHP built-in is absent for one of four standing reasons, and every removed name is accounted for by name

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#removals"><code>core-api/removals</code></a>
</div>

A PHP built-in that has no successor here is absent for one of four standing reasons, and every removed
name is accounted for individually rather than by category:

1. **A pure alias** — `sizeof`, `join`, `chop`, `key_exists`, `pos`, `fputs`, `is_integer`, `doubleval`.
2. **Dead or dying in PHP itself** — `ereg*`, `mysql_*`, `mcrypt`, `create_function`, `each`,
   `money_format`, `utf8_encode`, `strptime`, `get_browser`.
3. **Already closed by another rule** — about 120 functions whose removal follows from declared types, from
   closures being the only callable, from the closed doors, from the escalation ladder
   ([`errors/escalation-ladder`](/docs/rules/errors/the-escalation-ladder/#escalation-ladder "A failure escalates through four tiers, and no tier is retried")), from having no superglobals
   ([`statements/no-host-populated-variables`](/docs/rules/statements/where-state-lives/#no-host-populated-variables "No variable is ever populated by the host; every superglobal is a Core class member")), or from having no cross-request ambient state.
4. **Structurally wrong here** — the internal array pointer, because a mutable cursor inside a
   copy-on-write value is incoherent; every by-reference mutator ([`core-api/nothing-mutates`](/docs/rules/core-api/one-way-to-do-each-thing/#nothing-mutates "No Core member mutates its argument and none takes a reference"));
   `array_merge` and the `+` operator over two arrays, whose rule is chosen by a key's *type* in a language
   with one key type ([`types/array-combination`](/docs/rules/types/arrays-and-property-keys/#array-combination "Three combining members treat every key alike, and array + array does not compile")); the half-escapers, whose false confidence taint
   tracking exists to prevent; and `settype`/`gettype`/`strval` ([`types/no-legacy-cast`](/docs/rules/types/unions-and-conversion/#no-legacy-cast "PHP's (T)expr cast does not parse, and the diagnostic names the as that replaces it")).

The reason is not the record. A prose reason cannot be audited — "about 120 functions follow from declared
types" leaves no way to notice the twelfth one nobody thought about — so every PHP name gets a row naming
its outcome, and a CI check asserts the vendored built-in list has no name without one. That file is also
where `nvs convert` gets its diagnostic, so a removed name produces a message naming the replacement rather
than an unresolved call.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Roughly 1,900 built-ins become about 450 members covering strictly more ground</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/core-api/one-way-to-do-each-thing/#no-ambient-state" title="No Core member reads ambient state: there is no default timezone, locale, array pointer or error global"><code>core-api/no-ambient-state</code></a> <a href="/docs/rules/core-api/one-way-to-do-each-thing/#one-paradigm-per-operation" title="No operation is reachable two ways, and a domain class's statics never mirror an object's own methods"><code>core-api/one-paradigm-per-operation</code></a> <a href="/docs/rules/types/unions-and-conversion/#no-legacy-cast" title="PHP's (T)expr cast does not parse, and the diagnostic names the as that replaces it"><code>types/no-legacy-cast</code></a> <a href="/docs/rules/types/arrays-and-property-keys/#array-combination" title="Three combining members treat every key alike, and array + array does not compile"><code>types/array-combination</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0063.md">record 0063</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0051.md">record 0051</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/spec/02-php-migration.md"><code>docs/spec/02-php-migration.md</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tools/check-migration.py"><code>tools/check-migration.py</code></a></dd></div></dl>

</div>
