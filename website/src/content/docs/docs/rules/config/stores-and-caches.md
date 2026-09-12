---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Stores, sockets and the compiled-unit cache"
description: "Where a Unix socket may be written, why the extension set is in every cache key, and which opcache directives an operator alone owns."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/config/scheduled-work/
  label: "Scheduled work"
next:
  link: /docs/rules/packaging/
  label: "Packaging"
---

<p class="nv-section-lead">Where a Unix socket may be written, why the extension set is in every cache key, and which opcache directives an operator alone owns.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">9</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">5</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">4</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">5</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#cache-shared-is-the-grant-over-the-configured-store">A store an operator configured is authorized by the configuring — <code>cache.shared</code> is the grant, unscoped, and asks no address</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-unix-socket-is-admitted-only-where-an-operator-wrote-it">A Unix socket is admitted in <code>[cache.shared] url</code> and <code>[db.&lt;name&gt;] host</code>, and refused from any program-supplied endpoint</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#unix-scheme-in-a-url-and-a-bare-path-in-a-host">A socket is <code>unix:</code> where the key is a URL and a bare absolute path where the key is a host</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-unix-spelling-with-no-af-unix-transport-refuses-at-boot">A Unix spelling on a build with no <code>AF_UNIX</code> transport is refused at boot, never read as loopback TCP</a><span class="nv-rule-list-status" data-status="designed">Designed</span></li><li><a href="#net-local-is-named-and-not-on-the-roster">A program-supplied socket path needs <code>net.local</code>, a path-scoped grant carrying no address policy and governing both ends</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#the-extension-set-is-in-every-unit-key">The extension set is folded into one <code>env_hash</code> that both compiled-unit cache keys carry, so an extension change is an ordinary cache miss</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#opcache-revalidation-is-system-class"><code>opcache.validate</code> and its rate cap are <code>System</code>, and <code>validate</code>'s startup default is chosen by the run mode</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#opcache-file-cache-directives-are-system">The five <code>opcache.file_cache*</code> directives are <code>System</code>-class, because where compiled code is read from is a code-injection primitive</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#telemetry-and-update-endpoints-are-configuration">The update-check and telemetry endpoints are compiled-in defaults overridable by <code>NVS_UPDATE_URL</code> and <code>NVS_TELEMETRY_URL</code></a><span class="nv-rule-list-status" data-status="designed">Designed</span></li></ol>

<div class="nv-rule" id="cache-shared-is-the-grant-over-the-configured-store">

## A store an operator configured is authorized by the configuring — `cache.shared` is the grant, unscoped, and asks no address

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#cache-shared-is-the-grant-over-the-configured-store"><code>config/cache-shared-is-the-grant-over-the-configured-store</code></a>
</div>

An endpoint an operator wrote into root-owned configuration is authorized **by that writing**, so the
grant over it names the *store* and not a host. `Core\Cache::shared()` asks `cache.shared` at
`Scope::Unscoped`; so does `Core\RateLimit::consume`, which reaches the same store through the same
door and differs only in the sentence it appends to a refusal. Neither asks `net.connect`, and neither
consults [`security/net-address-policy`](/docs/rules/security/scopes-and-denial/#net-address-policy "net.connect carries an address policy that denies the private ranges, and an exception is one written IP address")'s denied-range table.

That is `mail.send`'s shape and `db.connect`'s reasoning ([`core-classes/db-capabilities`](/docs/rules/core-classes/connecting-to-a-database/#db-capabilities "Three deny-by-default capabilities gate naming a database, dialling one, and issuing DDL to it")): the
endpoint was written by the authority that grants the capability, so it is pre-approved and there is
nothing for an attacker to influence. A third key of the same kind gets the same treatment rather than
a fourth rule; the cost of the old question was that a loopback Redis needed `net.internal` to except
the very range every ordinary deployment puts its store in.

**Unscoped, because there is nothing to scope on.** A deployment has one shared store; `db.connect` is
scoped by name because there are many `[db.<name>]` blocks. The grant means "this program may reach
the coherent tier", and *where* that tier is stays the operator's answer — so moving it between a
container, a loopback daemon and a socket changes one block and no app's grant list.

A `[cache.shared] url` set with no `cache.shared` grant is reported at boot as a `Warn` naming both
keys (`W1008`), not discovered on a request. It is an advisory rather than a refusal: one `nvs.toml`
may serve an application that reaches the tier and one that does not.

The grant is asked at the door and after the directive is read — a deployment that configured no
store hears that first, because there is no tier for a grant to be about yet.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A Redis on loopback needs one grant naming the store, not a host allow-list plus an exception for the loopback range the store lives in</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/security/scopes-and-denial/#net-address-policy" title="net.connect carries an address policy that denies the private ranges, and an exception is one written IP address"><code>security/net-address-policy</code></a> <a href="/docs/rules/security/capabilities/#capability-question-is-grant-and-scope" title="A capability question is a grant and a scope, asked of the request's own configuration snapshot"><code>security/capability-question-is-grant-and-scope</code></a> <a href="/docs/rules/core-classes/connecting-to-a-database/#db-capabilities" title="Three deny-by-default capabilities gate naming a database, dialling one, and issuing DDL to it"><code>core-classes/db-capabilities</code></a> <a href="/docs/rules/core-api/lifetimes-and-absences/#two-cache-tiers" title="Cross-request state is two members with two contracts, never one API with a flag"><code>core-api/two-cache-tiers</code></a> <a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#ratelimit-two-members" title="consume and shed are two jobs with two verbs, and neither is a tier of the other"><code>core-classes/ratelimit-two-members</code></a> <a href="/docs/rules/config/stores-and-caches/#a-unix-socket-is-admitted-only-where-an-operator-wrote-it" title="A Unix socket is admitted in [cache.shared] url and [db.&lt;name&gt;] host, and refused from any program-supplied endpoint"><code>config/a-unix-socket-is-admitted-only-where-an-operator-wrote-it</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0142.md">record 0142</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0058.md">record 0058</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0059.md">record 0059</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/src/capability.rs"><code>crates/nvs-config/src/capability.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/src/store.rs"><code>crates/nvs-config/src/store.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-stdlib/src/cache.rs"><code>crates/nvs-stdlib/src/cache.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-unix-socket-is-admitted-only-where-an-operator-wrote-it">

## A Unix socket is admitted in `[cache.shared] url` and `[db.<name>] host`, and refused from any program-supplied endpoint

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-unix-socket-is-admitted-only-where-an-operator-wrote-it"><code>config/a-unix-socket-is-admitted-only-where-an-operator-wrote-it</code></a>
</div>

`[cache.shared] url` and `[db.<name>] host` may name a Unix domain socket. `Core\Net::connect`,
`Core\Db::open`'s program-supplied settings and `Core\Http\Client` may not, and a path reaching any of
them is refused **as a target this deployment has no way to authorize** — not as a file that could not
be opened.

The asymmetry is the address policy's own, read for what it is about. The denied-range table
([`security/net-address-policy`](/docs/rules/security/scopes-and-denial/#net-address-policy "net.connect carries an address policy that denies the private ranges, and an exception is one written IP address")) is not a list of unpleasant networks; it is the mechanism that
keeps a *program-supplied* endpoint off the local machine, which is why loopback heads it. A
program-supplied socket path is a way onto the local machine the table cannot see — there is no
address to match — and the reachable set on an ordinary host is worse than loopback's: it includes
this runtime's own `[control] socket`, a container daemon's socket and whatever else a distribution
puts in `/run`. Admitting one would hand a program the exact capability the policy spends a resolution
and a pin to deny.

An operator-written endpoint is the case the policy already distinguishes, for the reason it already
gives ([`config/cache-shared-is-the-grant-over-the-configured-store`](/docs/rules/config/stores-and-caches/#cache-shared-is-the-grant-over-the-configured-store "A store an operator configured is authorized by the configuring — cache.shared is the grant, unscoped, and asks no address")). A deny-list of socket paths
is not the alternative: the address ranges are few, closed and named by a standard, while the set of
sockets on a host is open, distribution-specific and grows when anything is installed. A check that
must enumerate what to refuse is wrong on the machine nobody tested. The grant a program-supplied
path would need is [`config/net-local-is-named-and-not-on-the-roster`](/docs/rules/config/stores-and-caches/#net-local-is-named-and-not-on-the-roster "A program-supplied socket path needs net.local, a path-scoped grant carrying no address policy and governing both ends").

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>Redis::connect('/run/redis.sock')</code> and a socket path in a DSN a program builds are refused — the path has to be in root-owned configuration</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/config/stores-and-caches/#cache-shared-is-the-grant-over-the-configured-store" title="A store an operator configured is authorized by the configuring — cache.shared is the grant, unscoped, and asks no address"><code>config/cache-shared-is-the-grant-over-the-configured-store</code></a> <a href="/docs/rules/config/stores-and-caches/#unix-scheme-in-a-url-and-a-bare-path-in-a-host" title="A socket is unix: where the key is a URL and a bare absolute path where the key is a host"><code>config/unix-scheme-in-a-url-and-a-bare-path-in-a-host</code></a> <a href="/docs/rules/config/stores-and-caches/#net-local-is-named-and-not-on-the-roster" title="A program-supplied socket path needs net.local, a path-scoped grant carrying no address policy and governing both ends"><code>config/net-local-is-named-and-not-on-the-roster</code></a> <a href="/docs/rules/security/scopes-and-denial/#net-address-policy" title="net.connect carries an address policy that denies the private ranges, and an exception is one written IP address"><code>security/net-address-policy</code></a> <a href="/docs/rules/security/tainted-data/#outbound-url-is-a-sink" title="An outbound URL or address is a sink, and a tainted one is refused where it is written"><code>security/outbound-url-is-a-sink</code></a> <a href="/docs/rules/config/reloading-and-control/#one-local-control-socket" title="The server is controlled over one local socket whose owner and mode are the authentication, and nvs ctl is its client"><code>config/one-local-control-socket</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0142.md">record 0142</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0058.md">record 0058</a></dd></div></dl>

</div>

<div class="nv-rule" id="unix-scheme-in-a-url-and-a-bare-path-in-a-host">

## A socket is `unix:` where the key is a URL and a bare absolute path where the key is a host

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#unix-scheme-in-a-url-and-a-bare-path-in-a-host"><code>config/unix-scheme-in-a-url-and-a-bare-path-in-a-host</code></a>
</div>

```toml
[cache.shared]
url = "unix:/run/redis.sock"          # or redis://host[:port]

[db.main]
driver = "postgres"
host   = "/var/run/postgresql"        # a socket directory
```

Two spellings because the two keys are two different things, and each takes the one that reads as
itself. `[cache.shared] url` holds a URL and its parser already dispatches on scheme — it strips
`redis://` and refuses `rediss://` and a database index with a sentence apiece — so `unix:` is one
more arm on machinery that exists, and the value stays a URL as the key's name promises. The scheme
names no protocol and does not need to: the block speaks RESP and nothing else, so its whole job is to
say *which transport*.

`[db.<name>] host` is not a URL and never was, so it takes the overload the listening side already
established for `[server] listen`: a value beginning with a path separator is a socket, and no
`host:port` can be spelled that way. What the path means per driver — a directory for Postgres, the
socket file for MySQL and MariaDB, a refusal for MSSQL — is
[`core-classes/db-unix-socket-path`](/docs/rules/core-classes/connecting-to-a-database/#db-unix-socket-path "A Unix-socket host is the string that deployment already holds, and MSSQL refuses one")'s.

A bare path in `url` is refused rather than read: a non-URL in a key called `url` is a thing to
re-litigate rather than a thing to read.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>redis://</code> and <code>unix:</code> are the only schemes <code>[cache.shared] url</code> reads — <code>rediss://</code> and a database index are refused with a sentence apiece</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/config/stores-and-caches/#a-unix-socket-is-admitted-only-where-an-operator-wrote-it" title="A Unix socket is admitted in [cache.shared] url and [db.&lt;name&gt;] host, and refused from any program-supplied endpoint"><code>config/a-unix-socket-is-admitted-only-where-an-operator-wrote-it</code></a> <a href="/docs/rules/core-classes/connecting-to-a-database/#db-unix-socket-path" title="A Unix-socket host is the string that deployment already holds, and MSSQL refuses one"><code>core-classes/db-unix-socket-path</code></a> <a href="/docs/rules/config/stores-and-caches/#a-unix-spelling-with-no-af-unix-transport-refuses-at-boot" title="A Unix spelling on a build with no AF_UNIX transport is refused at boot, never read as loopback TCP"><code>config/a-unix-spelling-with-no-af-unix-transport-refuses-at-boot</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0142.md">record 0142</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0097.md">record 0097</a></dd></div></dl>

</div>

<div class="nv-rule" id="a-unix-spelling-with-no-af-unix-transport-refuses-at-boot">

## A Unix spelling on a build with no `AF_UNIX` transport is refused at boot, never read as loopback TCP

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#a-unix-spelling-with-no-af-unix-transport-refuses-at-boot"><code>config/a-unix-spelling-with-no-af-unix-transport-refuses-at-boot</code></a>
</div>

A Unix spelling on a platform this build has no Unix transport for is refused **where the key was
written**, at boot, with a note naming the platform and pointing at loopback TCP. The Unix transport
is `#[cfg(unix)]`, and there is no Windows fallback because `AF_UNIX` exists there but the reactor's
I/O layer does not carry it.

At boot rather than at run time, which is the opposite of the answer a session `backend = "db"` gets
([`core-api/session-roster`](/docs/rules/core-api/lifetimes-and-absences/#session-roster "The session roster is start and six members, and a member called before start throws naming it")), and the difference is what each is waiting for. `db` names a store
the roster admits whose second half is unwritten, so refusing it at the key would claim the decision
was wrong rather than that the build has not caught up. A Unix socket on a platform with no `AF_UNIX`
is not waiting for this project — the deployment has to be spelled differently, and every request
until someone notices is one a boot refusal would have prevented. So it sits with `backend = "local"`:
refused where it is written, so a deployment cannot run believing it has a store it will never reach.

**Silently reading it as loopback TCP is refused**, for the reason `rediss://` is refused rather than
half-served: a configuration that reads as one transport and runs as another is worse than one that
does not run, because the difference is invisible in exactly the review that would have caught it.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/config/stores-and-caches/#unix-scheme-in-a-url-and-a-bare-path-in-a-host" title="A socket is unix: where the key is a URL and a bare absolute path where the key is a host"><code>config/unix-scheme-in-a-url-and-a-bare-path-in-a-host</code></a> <a href="/docs/rules/core-api/lifetimes-and-absences/#session-roster" title="The session roster is start and six members, and a member called before start throws naming it"><code>core-api/session-roster</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0142.md">record 0142</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0139.md">record 0139</a></dd></div></dl>

</div>

<div class="nv-rule" id="net-local-is-named-and-not-on-the-roster">

## A program-supplied socket path needs `net.local`, a path-scoped grant carrying no address policy and governing both ends

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#net-local-is-named-and-not-on-the-roster"><code>config/net-local-is-named-and-not-on-the-roster</code></a>
</div>

A program that supplies a socket path — connecting to one or binding one — needs `net.local`, a
capability **separate** from `net.connect`, asked at `Scope::Path` under
[`security/path-scope-canonicalise-then-prefix`](/docs/rules/security/scopes-and-denial/#path-scope-canonicalise-then-prefix "A path scope is canonicalise-then-prefix over whole components, and a path that does not exist yet is its deepest existing ancestor"), whose entries are absolute paths or directory
prefixes and which carries no address policy because there is no address:

```toml
[app.capabilities.net]
local = ["/run/redis.sock", "/var/run/mysqld/"]
```

**It governs both ends of a path.** Binding one is granted the same way as connecting to one: a
program that may create a socket at a path is a program that whatever else on the host finds it may
speak to, and a path the operator did not name is one they cannot have intended.

It is deliberately not `net.connect` widened to admit paths. That grant's whole documented character
is that it carries an address policy ([`security/net-address-policy`](/docs/rules/security/scopes-and-denial/#net-address-policy "net.connect carries an address policy that denies the private ranges, and an exception is one written IP address")), and entries of two kinds
under one name — hosts governed by a table, paths governed by nothing — is one name covering two
guarantees. The general form of that separation is
[`security/net-listen-is-a-separate-grant-from-net-connect`](/docs/rules/security/scopes-and-denial/#net-listen-is-a-separate-grant-from-net-connect "The grant follows what the program is doing rather than the transport: reaching out is net.connect, binding is net.listen, and neither widens the other"): the grant follows what the program is
doing rather than the transport it does it over.

[`config/a-unix-socket-is-admitted-only-where-an-operator-wrote-it`](/docs/rules/config/stores-and-caches/#a-unix-socket-is-admitted-only-where-an-operator-wrote-it "A Unix socket is admitted in [cache.shared] url and [db.<name>] host, and refused from any program-supplied endpoint") is unaffected and stays exactly
as strict. `Core\Net::connect` still may not take a path, because it is not the member that takes one —
a host and a path are separately-typed arguments to separate members, which is
[`security/a-path-is-not-a-url`](/docs/rules/security/closed-doors/#a-path-is-not-a-url "A path is a filesystem path: no member dispatches on a scheme prefix, and nothing may register one") holding by construction rather than by a check.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/config/stores-and-caches/#a-unix-socket-is-admitted-only-where-an-operator-wrote-it" title="A Unix socket is admitted in [cache.shared] url and [db.&lt;name&gt;] host, and refused from any program-supplied endpoint"><code>config/a-unix-socket-is-admitted-only-where-an-operator-wrote-it</code></a> <a href="/docs/rules/security/scopes-and-denial/#net-listen-is-a-separate-grant-from-net-connect" title="The grant follows what the program is doing rather than the transport: reaching out is net.connect, binding is net.listen, and neither widens the other"><code>security/net-listen-is-a-separate-grant-from-net-connect</code></a> <a href="/docs/rules/security/scopes-and-denial/#path-scope-canonicalise-then-prefix" title="A path scope is canonicalise-then-prefix over whole components, and a path that does not exist yet is its deepest existing ancestor"><code>security/path-scope-canonicalise-then-prefix</code></a> <a href="/docs/rules/security/closed-doors/#a-path-is-not-a-url" title="A path is a filesystem path: no member dispatches on a scheme prefix, and nothing may register one"><code>security/a-path-is-not-a-url</code></a> <a href="/docs/rules/security/scopes-and-denial/#net-address-policy" title="net.connect carries an address policy that denies the private ranges, and an exception is one written IP address"><code>security/net-address-policy</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0142.md">record 0142</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0162.md">record 0162</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/net-a-host-grant-and-an-endpoint-grant-do-not-buy-a-socket-path.nvst"><code>tests/conformance/core/net-a-host-grant-and-an-endpoint-grant-do-not-buy-a-socket-path.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/net-a-socket-path-is-one-grant-at-both-ends-and-reaches-what-an-operator-wrote.nvst"><code>tests/conformance/core/net-a-socket-path-is-one-grant-at-both-ends-and-reaches-what-an-operator-wrote.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="the-extension-set-is-in-every-unit-key">

## The extension set is folded into one `env_hash` that both compiled-unit cache keys carry, so an extension change is an ordinary cache miss

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#the-extension-set-is-in-every-unit-key"><code>config/the-extension-set-is-in-every-unit-key</code></a>
</div>

```
extension_set_hash = BLAKE3(sorted sha256 pins of the [[extension]] array)
env_hash           = BLAKE3(target_triple ‖ cpu_feature_bitset ‖ compiler_version_hash ‖ extension_set_hash)
content_hash       = BLAKE3(source_content)
artifact_key       = BLAKE3(content_hash ‖ env_hash)
```

One `env_hash` is carried by **both** compiled-unit caches: the on-disk key is
`BLAKE3(content_hash ‖ env_hash)`, and the in-memory key is `UnitKey { path, content_hash,
env_hash }`. It is derived from the same `content_hash` the in-memory key carries, so a unit's bytes
are hashed once for both. It is constant for the life of a configuration and costs the request path
nothing.

That is the whole of extension reload: a changed set changes `env_hash`, every unit key changes with
it, every lookup is an ordinary miss, and the lazy per-path revalidation of
[`config/an-edit-reaches-the-next-request-without-a-restart`](/docs/rules/config/reloading-and-control/#an-edit-reaches-the-next-request-without-a-restart "An edited source file reaches the next request that resolves it, through lazy revalidation and one pointer swap, never a watcher or a restart") recompiles each unit on the compile
pool as some request resolves it. No invalidation pass exists. It also closes the hole where an
artifact compiled against one extension set — holding a direct call to a trampoline that has since
moved — could be reused against another.

Invalidation is coarse by design: changing the set rekeys every unit, not only units that call an
extension. Finer would need per-unit dependency tracking including negative dependencies, a real
subsystem for a modest win. The compile pool bounds how much of the resulting wave is in flight at
once.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/config/reloading-and-control/#an-edit-reaches-the-next-request-without-a-restart" title="An edited source file reaches the next request that resolves it, through lazy revalidation and one pointer swap, never a watcher or a restart"><code>config/an-edit-reaches-the-next-request-without-a-restart</code></a> <a href="/docs/rules/config/reloading-and-control/#a-reload-names-what-it-could-not-apply" title="A reload reports what it applied, names every changed Boot key it could not apply, and counts the units it invalidated"><code>config/a-reload-names-what-it-could-not-apply</code></a> <a href="/docs/rules/programs/names-and-files/#no-runtime-autoload" title="A name reaches its file while compiling; there is no runtime loader of any kind"><code>programs/no-runtime-autoload</code></a> <a href="/docs/rules/config/stores-and-caches/#opcache-file-cache-directives-are-system" title="The five opcache.file_cache directives are System-class, because where compiled code is read from is a code-injection primitive"><code>config/opcache-file-cache-directives-are-system</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0078.md">record 0078</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0017.md">record 0017</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0042.md">record 0042</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/tests/snapshot.rs"><code>crates/nvs-config/tests/snapshot.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="opcache-revalidation-is-system-class">

## `opcache.validate` and its rate cap are `System`, and `validate`'s startup default is chosen by the run mode

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#opcache-revalidation-is-system-class"><code>config/opcache-revalidation-is-system-class</code></a>
</div>

`opcache.validate` — `never`, `mtime` or `hash` — and `opcache.revalidate_freq` are `System`-class:
a request cannot loosen how often, or whether, the process re-checks source files. Letting a request
set `validate = "never"` for itself would be a way to pin a version of the code past a since-shipped
fix, and letting it lower `revalidate_freq` would be a way to force a `stat`/hash storm on a hot
file. Neither is a request-local decision ([`config/system-means-a-request-may-not-set-it`](/docs/rules/config/changeability-classes/#system-means-a-request-may-not-set-it "A directive is System when changing it from inside a request would affect something other than that request, and that is all System means")).

`validate`'s **startup default** is selected by the run mode — `never` in `production`, `mtime` in
`development` — as one of the startup rows in [`config/a-startup-default-is-never-flipped`](/docs/rules/config/run-modes/#a-startup-default-is-never-flipped "A mode also selects three startup defaults that are fixed at boot, never re-derived, and never flippable from code"). That
does not loosen the paragraph above: the row is chosen by root-owned configuration before any request
exists, is never re-derived by a runtime mode flip, and stays unflippable from code. An `[opcache]
validate` written beside `mode = "development"` still wins, because the mode supplies a default and
nothing more.

`revalidate_freq` is deliberately not a mode row: no value of it a developer's machine needs differs
from an operator's, so it keeps its own default under either mode.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>opcache.validate_timestamps</code> and <code>revalidate_freq</code> are <code>PHP_INI_ALL</code> in PHP; here no request may touch either</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/config/changeability-classes/#system-means-a-request-may-not-set-it" title="A directive is System when changing it from inside a request would affect something other than that request, and that is all System means"><code>config/system-means-a-request-may-not-set-it</code></a> <a href="/docs/rules/config/run-modes/#a-startup-default-is-never-flipped" title="A mode also selects three startup defaults that are fixed at boot, never re-derived, and never flippable from code"><code>config/a-startup-default-is-never-flipped</code></a> <a href="/docs/rules/config/reloading-and-control/#an-edit-reaches-the-next-request-without-a-restart" title="An edited source file reaches the next request that resolves it, through lazy revalidation and one pointer swap, never a watcher or a restart"><code>config/an-edit-reaches-the-next-request-without-a-restart</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0017.md">record 0017</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0091.md">record 0091</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0005.md">record 0005</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/tests/directives.rs"><code>crates/nvs-config/tests/directives.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/tests/snapshot.rs"><code>crates/nvs-config/tests/snapshot.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="opcache-file-cache-directives-are-system">

## The five `opcache.file_cache*` directives are `System`-class, because where compiled code is read from is a code-injection primitive

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#opcache-file-cache-directives-are-system"><code>config/opcache-file-cache-directives-are-system</code></a>
</div>

The on-disk artifact cache is governed by five directives in `[opcache]`: `file_cache` (bool, default
on), `file_cache_dir` (a path, root-owned, defaulting to a fixed per-build location), `file_cache_max_size`
(bytes), and the `file_cache_gc_probability` / `file_cache_gc_divisor` pair, which mirrors PHP's own
`session.gc_probability`/`gc_divisor` because eviction rides the cold-compile path at a small
probability rather than costing a warm hit anything.

`file_cache_dir` is the **only** spelling of where that cache lives. `[cache]` is `Core\Cache`'s two
tiers and holds nothing about compiled artifacts, so `[cache] dir` is `E0601` like any other key the
registry does not know (`docs/decisions/0175.md` § 4). It is also the one of the five that applies at
boot rather than at reload, which is [`config/reloadability-is-its-own-field`](/docs/rules/config/changeability-classes/#reloadability-is-its-own-field "Reloadability is a second registry field, Reload or Boot, orthogonal to the changeability class")'s second field
answering a second question and not this rule's class.

**All five are `System`-class**, for the identical reason `opcache.validate` is: a script that could
redirect where the process reads "already-compiled, about-to-be-trusted" native code from would be
handing itself a code-injection primitive, not a performance knob. A request cannot tighten them
either — there is no safe direction for a key that decides which bytes become executable.

The consequence is a boot-time trust: a cache directory whose ownership changes after the process
started is not re-checked mid-run, consistent with every other `System` directive. What the cache
looks like on disk, how an entry is verified before it is mapped executable, and the refusal of a
world-writable directory are the packaging chapter's; this rule is only the roster and its class.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>opcache.file_cache</code> and <code>opcache.file_cache_dir</code> cannot be moved by <code>ini_set</code> or <code>.user.ini</code>; only the root-owned file sets them</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/config/scheduled-work/#every-schedule-key-is-system" title="Every [[schedule]] key is System, and not even RuntimeTighten reaches one"><code>config/every-schedule-key-is-system</code></a> <a href="/docs/rules/config/changeability-classes/#three-changeability-classes" title="nvs.toml states defaults, not ceilings, and every directive carries one of three changeability classes"><code>config/three-changeability-classes</code></a> <a href="/docs/rules/config/stores-and-caches/#opcache-revalidation-is-system-class" title="opcache.validate and its rate cap are System, and validate's startup default is chosen by the run mode"><code>config/opcache-revalidation-is-system-class</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0042.md">record 0042</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0005.md">record 0005</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0017.md">record 0017</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0175.md">record 0175</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/tests/directives.rs"><code>crates/nvs-config/tests/directives.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="telemetry-and-update-endpoints-are-configuration">

## The update-check and telemetry endpoints are compiled-in defaults overridable by `NVS_UPDATE_URL` and `NVS_TELEMETRY_URL`

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<a class="nv-rule-id" href="#telemetry-and-update-endpoints-are-configuration"><code>config/telemetry-and-update-endpoints-are-configuration</code></a>
</div>

The update-check URL and the telemetry upload URL have compiled-in defaults, overridable by environment
variable: `NVS_UPDATE_URL` and `NVS_TELEMETRY_URL`. Neither is a key in `nvs.toml` — they configure
the *tool*, not a deployment, and a serving process never uploads anything regardless of where the
endpoint points.

Configurability is what makes the rest of the telemetry design falsifiable. The test suite runs every
claim — the weekly upload gate, the three-second timeout, the never-uploads-from-serve rule, the exact
payload `nvs telemetry show` prints — against a local listener, and CI never touches the real host. The
consents, the counter schema and the two endpoints' contracts are the tooling chapter's.

**Not shipped.** Nothing in `crates/` reads either variable; implementation waits until the parity
chain's goals are done, and the reference says nothing of it until then.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0130.md">record 0130</a></dd></div></dl>

</div>
