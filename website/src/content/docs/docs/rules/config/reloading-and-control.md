---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Reloading and control"
description: "An edit reaches the next request without a restart, and the running process is controlled over one local socket and no network surface."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/config/changeability-classes/
  label: "Changeability classes"
next:
  link: /docs/rules/config/run-modes/
  label: "Run modes"
---

<p class="nv-section-lead">An edit reaches the next request without a restart, and the running process is controlled over one local socket and no network surface.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">8</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">8</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">0</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">5</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#an-edit-reaches-the-next-request-without-a-restart">An edited source file reaches the next request that resolves it, through lazy revalidation and one pointer swap, never a watcher or a restart</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-request-keeps-the-unit-it-resolved">A file is resolved once per isolate, so an edit is invisible to a request already running</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#a-broken-edit-fails-the-requests-that-resolve-it">A revalidated file that no longer compiles fails the requests that resolve it afterwards, loudly, and the last good version is never served in its place</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-reload-names-what-it-could-not-apply">A reload reports what it applied, names every changed <code>Boot</code> key it could not apply, and counts the units it invalidated</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#one-local-control-socket">The server is controlled over one local socket whose owner and mode are the authentication, and <code>nvs ctl</code> is its client</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#no-network-control-surface">There is no network-reachable control surface, in either direction of configuration</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#ctl-config-reports-the-live-snapshot"><code>nvs ctl config</code> reports what the running process actually holds, including an optional include that appeared after boot</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#check-and-dump-audit-the-tree-offline"><code>nvs config check</code> and <code>nvs config dump</code> resolve the tree with no server, and <code>dump --origin</code> names where every value came from</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li></ol>

<div class="nv-rule" id="an-edit-reaches-the-next-request-without-a-restart">

## An edited source file reaches the next request that resolves it, through lazy revalidation and one pointer swap, never a watcher or a restart

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#an-edit-reaches-the-next-request-without-a-restart"><code>config/an-edit-reaches-the-next-request-without-a-restart</code></a>
</div>

The compiled-unit cache is keyed by **content**, not by path: `UnitKey { path, content_hash,
env_hash } → CompileState`, and a `Ready` entry is write-once — nothing already in the map is ever
mutated or torn. In front of it sits one small indirection, `path → current content_hash`, which is
the pointer an edit swaps.

Resolving a `require` or an inbound request's entry file walks five steps: reuse the known hash
under `opcache.validate = "never"` or inside `revalidate_freq`, with no syscall; otherwise `stat`
(and under `hash`, or on an `mtime` mismatch, re-hash) the file, and continue with no compile if
the content is unchanged; on a change, compile the new content through the same single-flight
machinery a cold compile uses, on the compile pool, never on a request-serving core; on success
swap the path's pointer, publishing only if nobody moved it since; on failure leave the pointer
alone ([`config/a-broken-edit-fails-the-requests-that-resolve-it`](/docs/rules/config/reloading-and-control/#a-broken-edit-fails-the-requests-that-resolve-it "A revalidated file that no longer compiles fails the requests that resolve it afterwards, loudly, and the last good version is never served in its place")).

`mtime` is a cheap pre-filter; only the content hash is trusted as the key, so a filesystem with a
coarse clock cannot serve stale code. There is no filesystem watcher, no stop-the-world phase and no
second process, and a client cannot trigger a recompile — only the file's own content changing does.
The rate cap bounds `stat` overhead to `N ⁄ revalidate_freq` per file, and laziness means only files
a request actually resolves ever recompile, however many a deploy touched.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>opcache.validate_timestamps</code> shape, but the cache key is the file's content hash rather than its path, so a stale <code>mtime</code> never serves stale code, and there is no <code>opcache_reset()</code></p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/config/reloading-and-control/#a-request-keeps-the-unit-it-resolved" title="A file is resolved once per isolate, so an edit is invisible to a request already running"><code>config/a-request-keeps-the-unit-it-resolved</code></a> <a href="/docs/rules/config/reloading-and-control/#a-broken-edit-fails-the-requests-that-resolve-it" title="A revalidated file that no longer compiles fails the requests that resolve it afterwards, loudly, and the last good version is never served in its place"><code>config/a-broken-edit-fails-the-requests-that-resolve-it</code></a> <a href="/docs/rules/config/stores-and-caches/#opcache-revalidation-is-system-class" title="opcache.validate and its rate cap are System, and validate's startup default is chosen by the run mode"><code>config/opcache-revalidation-is-system-class</code></a> <a href="/docs/rules/config/stores-and-caches/#the-extension-set-is-in-every-unit-key" title="The extension set is folded into one env_hash that both compiled-unit cache keys carry, so an extension change is an ordinary cache miss"><code>config/the-extension-set-is-in-every-unit-key</code></a> <a href="/docs/rules/concurrency/connections/#a-connection-keeps-its-compiled-unit" title="An open connection runs to completion on the code it started with, and an edit reaches only the next one"><code>concurrency/a-connection-keeps-its-compiled-unit</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0017.md">record 0017</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0078.md">record 0078</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0042.md">record 0042</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-cli/src/script.rs"><code>crates/nvs-cli/src/script.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-request-keeps-the-unit-it-resolved">

## A file is resolved once per isolate, so an edit is invisible to a request already running

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#a-request-keeps-the-unit-it-resolved"><code>config/a-request-keeps-the-unit-it-resolved</code></a>
</div>

**A file is resolved once per isolate, the first time execution reaches it, and never re-resolved
on a second reference within the same run.** The compiled unit a running isolate holds was cloned
out of the cache at the moment of first touch, and it is never mutated in place — only ever replaced
at the path-pointer layer above it. A request that resolved a file before an edit completes on the
pre-edit unit even if the edit and a successful recompile land before it finishes; the next request
to resolve the same path gets the new one.

The compiled code is the *only* thing shared across requests. A request's heap arena, its limits and
ceilings, its `Core\Request`/`Core\Server`/`Core\Session` state and its panic containment are
untouched by a swap: a hot-reload event changes what code a *future* request compiles to, never how
isolated any request's execution of that code is ([`security/isolate-shares-nothing`](/docs/rules/security/isolates/#isolate-shares-nothing "Running another script is an in-process isolate that shares nothing with its parent but compiled code")). A
connection isolate is the long-lived case of the same rule
([`concurrency/a-connection-keeps-its-compiled-unit`](/docs/rules/concurrency/connections/#a-connection-keeps-its-compiled-unit "An open connection runs to completion on the code it started with, and an edit reaches only the next one")).

Old generations are retained only while some in-flight request still holds one, bounded by that
request's own wall-clock and CPU limits — O(in-flight), never O(edits ever made).

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/config/reloading-and-control/#an-edit-reaches-the-next-request-without-a-restart" title="An edited source file reaches the next request that resolves it, through lazy revalidation and one pointer swap, never a watcher or a restart"><code>config/an-edit-reaches-the-next-request-without-a-restart</code></a> <a href="/docs/rules/security/isolates/#isolate-shares-nothing" title="Running another script is an in-process isolate that shares nothing with its parent but compiled code"><code>security/isolate-shares-nothing</code></a> <a href="/docs/rules/concurrency/connections/#a-connection-keeps-its-compiled-unit" title="An open connection runs to completion on the code it started with, and an edit reaches only the next one"><code>concurrency/a-connection-keeps-its-compiled-unit</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0017.md">record 0017</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0006.md">record 0006</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-cli/src/script.rs"><code>crates/nvs-cli/src/script.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-broken-edit-fails-the-requests-that-resolve-it">

## A revalidated file that no longer compiles fails the requests that resolve it afterwards, loudly, and the last good version is never served in its place

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-broken-edit-fails-the-requests-that-resolve-it"><code>config/a-broken-edit-fails-the-requests-that-resolve-it</code></a>
</div>

When a revalidated file no longer compiles, the path's pointer is left where it was — it still names
the last content that compiled — but the `Failed` state the resolution just reached is what **this**
caller gets, as ordinary checked-return data. A later request landing on the same content sees the
same `Failed` entry, because it is the same key, and is answered from the table rather than compiled
again, so a request storm against a broken file costs one compile and one rendering of its spans, not
one per request.

Requests already running are unaffected ([`config/a-request-keeps-the-unit-it-resolved`](/docs/rules/config/reloading-and-control/#a-request-keeps-the-unit-it-resolved "A file is resolved once per isolate, so an edit is invisible to a request already running")); only
requests that newly resolve the broken file fail, and they fail loudly. Silently continuing to serve
the last good version after an edit — especially a security fix — is the worse failure mode, and it
would diverge from PHP's own `validate_timestamps` behaviour to buy availability nothing needs.

The same policy covers an extension removed while source still references it: nothing proves that at
reload time, and the units that call it fail when a request next resolves them, and only those.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>The same behaviour as <code>validate_timestamps</code>, chosen over the more forgiving keep-serving policy some caches adopt</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/config/reloading-and-control/#an-edit-reaches-the-next-request-without-a-restart" title="An edited source file reaches the next request that resolves it, through lazy revalidation and one pointer swap, never a watcher or a restart"><code>config/an-edit-reaches-the-next-request-without-a-restart</code></a> <a href="/docs/rules/config/reloading-and-control/#a-request-keeps-the-unit-it-resolved" title="A file is resolved once per isolate, so an edit is invisible to a request already running"><code>config/a-request-keeps-the-unit-it-resolved</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0017.md">record 0017</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0078.md">record 0078</a></dd></div></dl>

</div>

<div class="nv-rule" id="a-reload-names-what-it-could-not-apply">

## A reload reports what it applied, names every changed `Boot` key it could not apply, and counts the units it invalidated

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-reload-names-what-it-could-not-apply"><code>config/a-reload-names-what-it-could-not-apply</code></a>
</div>

The answer to a reload names, in one place: the directives applied and now in force; the **`Boot`
keys whose values changed and therefore did not take effect, each named individually**; and how many
compiled units were invalidated, so an operator knows a recompile wave is coming. A validation failure
reports the offending line and states that the running configuration is unchanged.

Naming the ignored `Boot` keys is the difference between a reload an operator can trust and one they
have to guess about: silently ignoring a changed listen address is how a deployment ends up believing
it applied a change it did not. The report and the carry are one operation — the published snapshot
still holds the *running* value of each named key, so the change exists nowhere but the report until
a restart. A changed `Boot` key is therefore absent from the applied list rather than present in
both.

The unit count follows [`config/the-extension-set-is-in-every-unit-key`](/docs/rules/config/stores-and-caches/#the-extension-set-is-in-every-unit-key "The extension set is folded into one env_hash that both compiled-unit cache keys carry, so an extension change is an ordinary cache miss"): a changed `env_hash`
invalidates every unit and an unchanged one invalidates none. What a reload cannot catch is an
extension removed while source still references it — those units fail when next resolved
([`config/a-broken-edit-fails-the-requests-that-resolve-it`](/docs/rules/config/reloading-and-control/#a-broken-edit-fails-the-requests-that-resolve-it "A revalidated file that no longer compiles fails the requests that resolve it afterwards, loudly, and the last good version is never served in its place")).

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A changed listen address is named in the reply rather than silently ignored until the next restart</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/config/changeability-classes/#reloadability-is-its-own-field" title="Reloadability is a second registry field, Reload or Boot, orthogonal to the changeability class"><code>config/reloadability-is-its-own-field</code></a> <a href="/docs/rules/config/changeability-classes/#the-config-is-an-immutable-snapshot" title="The configuration is one immutable snapshot a request clones at start, and a reload replaces it whole after validating everything"><code>config/the-config-is-an-immutable-snapshot</code></a> <a href="/docs/rules/config/reloading-and-control/#one-local-control-socket" title="The server is controlled over one local socket whose owner and mode are the authentication, and nvs ctl is its client"><code>config/one-local-control-socket</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0078.md">record 0078</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-server/src/control.rs"><code>crates/nvs-server/src/control.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/tests/snapshot.rs"><code>crates/nvs-config/tests/snapshot.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="one-local-control-socket">

## The server is controlled over one local socket whose owner and mode are the authentication, and `nvs ctl` is its client

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#one-local-control-socket"><code>config/one-local-control-socket</code></a>
</div>

```toml
[control]
socket = "/run/nvs/control.sock"   # \\.\pipe\nvs-control on Windows; `false` disables
```

**Local socket only. There is no TCP listener, no token, no TLS and no auth middleware** — the
socket's owner and mode are the authentication. It is created mode `0600` (a DACL naming this account
on Windows), owned by the runtime's account, and **the server refuses to start if the directory
holding it is writable by any other account**, the same trust check every configuration file gets.
A tree that writes no `[control]` block gets no control surface at all. The socket exists only where
a long-running server does; `nvs run` compiles one file and exits.

The wire protocol is HTTP over that socket, not a bespoke line protocol: `curl --unix-socket` debugs
it with no special tooling. **`nvs ctl` is the client**, a namespace of its own because every other
subcommand acts on files with no server involved; `--socket` addresses one of several servers on a
host. `reload` re-reads the whole configuration tree and publishes it; `ctl config` prints the live
snapshot with each directive's origin. Operations serialize, so two reloads cannot interleave two
snapshots. **No control operation runs user Novis code, ever** — one that could would be
[`security/no-eval`](/docs/rules/security/closed-doors/#no-eval "There is no eval, and no Core member compiles a string produced at run time")'s door with a different name on it.

The wire shape is unstable until 1.0: every response carries the server version, and `nvs ctl`
refuses a mismatch. Every reload is written to `Core\Log` with its outcome.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no <code>SIGUSR2</code>, no <code>php-fpm reload</code> and no signal at all; the operator speaks HTTP to a local socket and gets a report back</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/config/changeability-classes/#the-config-is-an-immutable-snapshot" title="The configuration is one immutable snapshot a request clones at start, and a reload replaces it whole after validating everything"><code>config/the-config-is-an-immutable-snapshot</code></a> <a href="/docs/rules/config/reloading-and-control/#a-reload-names-what-it-could-not-apply" title="A reload reports what it applied, names every changed Boot key it could not apply, and counts the units it invalidated"><code>config/a-reload-names-what-it-could-not-apply</code></a> <a href="/docs/rules/config/reloading-and-control/#no-network-control-surface" title="There is no network-reachable control surface, in either direction of configuration"><code>config/no-network-control-surface</code></a> <a href="/docs/rules/security/closed-doors/#no-eval" title="There is no eval, and no Core member compiles a string produced at run time"><code>security/no-eval</code></a> <a href="/docs/rules/config/reloading-and-control/#ctl-config-reports-the-live-snapshot" title="nvs ctl config reports what the running process actually holds, including an optional include that appeared after boot"><code>config/ctl-config-reports-the-live-snapshot</code></a> <a href="/docs/rules/config/includes-and-ownership/#ownership-is-the-trust-boundary" title="Every file the configuration reads, and its directory, must be owned by the runtime account or root and writable by nobody else"><code>config/ownership-is-the-trust-boundary</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0078.md">record 0078</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0103.md">record 0103</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0042.md">record 0042</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-server/src/control.rs"><code>crates/nvs-server/src/control.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-cli/src/ctl.rs"><code>crates/nvs-cli/src/ctl.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-cli/src/serve.rs"><code>crates/nvs-cli/src/serve.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="no-network-control-surface">

## There is no network-reachable control surface, in either direction of configuration

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#no-network-control-surface"><code>config/no-network-control-surface</code></a>
</div>

There is no TCP listener, in either direction of configuration. `[control] socket` accepts a local
endpoint or `false`, and a value that would be reached over a network — a URL, a host and a port, a
bare port number — is refused at boot by name rather than bound. A remote control plane is reachable
today by running `nvs ctl` over the operator's existing access path, SSH or the container runtime's
exec, which every orchestrator already has.

A network listener is the only part of the control design that would carry an authentication
surface, and nothing yet needs one, so this is deferred rather than rejected. What it would take is
already recorded: a second listener absent unless configured and never sharing the application
listener; per-effect-class enablement (`observe` / `operate` / `lifecycle`) so a liveness probe cannot
be handed `shutdown`; a token read from a file, refused at boot if absent; TLS required for any
non-loopback bind; and `lifecycle` withheld from a network listener entirely.

A control endpoint on the application listener is rejected outright: every path-normalization bug
and proxy misconfiguration would become privilege escalation, and a reserved prefix would collide
permanently with the compile-time route table.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/config/reloading-and-control/#one-local-control-socket" title="The server is controlled over one local socket whose owner and mode are the authentication, and nvs ctl is its client"><code>config/one-local-control-socket</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0078.md">record 0078</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-server/src/control.rs"><code>crates/nvs-server/src/control.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="ctl-config-reports-the-live-snapshot">

## `nvs ctl config` reports what the running process actually holds, including an optional include that appeared after boot

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#ctl-config-reports-the-live-snapshot"><code>config/ctl-config-reports-the-live-snapshot</code></a>
</div>

`nvs ctl config --origin` answers the question the offline pair cannot: **what the running process
actually holds** — what the last reload published, including an `optional` include that has appeared
since boot, and every `Boot` key whose changed value was reported and left unapplied. It is the second
operation on the control socket, which the reload rule reserved for exactly this kind of read.

The output is `nvs config dump --origin`'s, taken from the live snapshot rather than from the files on
disk, so the two can be diffed: a difference between them is a reload that has not happened, a file
that changed since the last one, or a directory-mode change that will refuse the next one.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/config/reloading-and-control/#check-and-dump-audit-the-tree-offline" title="nvs config check and nvs config dump resolve the tree with no server, and dump --origin names where every value came from"><code>config/check-and-dump-audit-the-tree-offline</code></a> <a href="/docs/rules/config/the-file-and-the-tree/#the-resolved-root-is-announced-and-stored" title="The resolved root is canonicalized once at boot, announced with every file it reached, and re-read from that stored path on reload"><code>config/the-resolved-root-is-announced-and-stored</code></a> <a href="/docs/rules/config/reloading-and-control/#one-local-control-socket" title="The server is controlled over one local socket whose owner and mode are the authentication, and nvs ctl is its client"><code>config/one-local-control-socket</code></a> <a href="/docs/rules/config/reloading-and-control/#a-reload-names-what-it-could-not-apply" title="A reload reports what it applied, names every changed Boot key it could not apply, and counts the units it invalidated"><code>config/a-reload-names-what-it-could-not-apply</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0103.md">record 0103</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0078.md">record 0078</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-cli/src/ctl.rs"><code>crates/nvs-cli/src/ctl.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-server/src/control.rs"><code>crates/nvs-server/src/control.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="check-and-dump-audit-the-tree-offline">

## `nvs config check` and `nvs config dump` resolve the tree with no server, and `dump --origin` names where every value came from

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#check-and-dump-audit-the-tree-offline"><code>config/check-and-dump-audit-the-tree-offline</code></a>
</div>

[`config/later-wins-and-every-override-is-recorded`](/docs/rules/config/includes-and-ownership/#later-wins-and-every-override-is-recorded "The tree is one ordered stream, a later assignment wins, and every override is recorded with both origins") is only safe while it is auditable, so the
reporting is part of the rule rather than tooling around it. `config` is a namespace beside `ctl` and
`service`, and stays out of `nvs check`, which checks source.

```console
$ nvs config check --config /etc/nvs/nvs.toml
ok: 5 files, 47 directives set, 3 overrides, 1 warning

$ nvs config dump --origin
limits.memory             = "512M"    prod.toml:4   (overrides base.toml:2)
capabilities.process.exec = true      conf.d/host.toml:3
db.main.password          = <secret>  /run/secrets/db_password (conf.d/20-db.toml:5)

$ nvs config dump --toml > effective.toml     # one canonical file, for diffing environments
```

Both are **offline and need no server**, so a tree is validated in CI before it is deployed. `check`
stops at the first refusal, because a file that does not parse has no keys to carry into the rest of
the stream, and its four counts are what CI reads; `dump --origin` is where each override is named,
per key, with the file and line that set it and the one it overrode. A secret renders `<secret>` and
names the file it came from, never the value, and `--toml` serializes the merged table whole. Both
read without the ownership check
([`config/the-ownership-check-runs-where-it-can-be-answered`](/docs/rules/config/includes-and-ownership/#the-ownership-check-runs-where-it-can-be-answered "The ownership check runs on nvs serve and on reload; nvs run, config check and config dump read without it, and an unchecked file can grant nothing")). What a reload actually published is
[`config/ctl-config-reports-the-live-snapshot`](/docs/rules/config/reloading-and-control/#ctl-config-reports-the-live-snapshot "nvs ctl config reports what the running process actually holds, including an optional include that appeared after boot").

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>php --ini</code> names the files read and <code>phpinfo()</code> prints values from inside a request; here one offline command validates the whole tree in CI and names, per key, the file and line that set it and the one it overrode</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/config/includes-and-ownership/#later-wins-and-every-override-is-recorded" title="The tree is one ordered stream, a later assignment wins, and every override is recorded with both origins"><code>config/later-wins-and-every-override-is-recorded</code></a> <a href="/docs/rules/config/reloading-and-control/#ctl-config-reports-the-live-snapshot" title="nvs ctl config reports what the running process actually holds, including an optional include that appeared after boot"><code>config/ctl-config-reports-the-live-snapshot</code></a> <a href="/docs/rules/config/includes-and-ownership/#the-ownership-check-runs-where-it-can-be-answered" title="The ownership check runs on nvs serve and on reload; nvs run, config check and config dump read without it, and an unchecked file can grant nothing"><code>config/the-ownership-check-runs-where-it-can-be-answered</code></a> <a href="/docs/rules/config/includes-and-ownership/#a-secret-is-a-file-whose-content-is-the-value" title="A secret directive has a _file sibling, exactly one of the pair is set, and the file's whole content is the value"><code>config/a-secret-is-a-file-whose-content-is-the-value</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0103.md">record 0103</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0093.md">record 0093</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/config/a-later-file-wins-and-both-origins-are-reported.nvst"><code>tests/conformance/config/a-later-file-wins-and-both-origins-are-reported.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/config/a-value-array-replaces-where-a-table-appends.nvst"><code>tests/conformance/config/a-value-array-replaces-where-a-table-appends.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/config/a-secret-is-rendered-as-secret-and-names-the-file-it-came-from.nvst"><code>tests/conformance/config/a-secret-is-rendered-as-secret-and-names-the-file-it-came-from.nvst</code></a></dd></div></dl>

</div>
