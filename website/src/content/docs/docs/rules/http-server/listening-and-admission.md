---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Listening and admission"
description: "What the server is and is not, how mounts expand at boot, and the valve that answers 503 before an isolate exists."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/http-server/
  label: "The HTTP server"
next:
  link: /docs/rules/http-server/resolving-a-request/
  label: "Resolving a request"
---

<p class="nv-section-lead">What the server is and is not, how mounts expand at boot, and the valve that answers 503 before an isolate exists.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">13</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">11</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">2</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">11</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#two-deployments-and-nothing-a-proxy-owns">The server is a development server or a proxied production origin, and nothing a proxy does earlier is in it</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#the-server-block-is-boot-class">The <code>[server]</code> block is <code>Boot</code>-class, <code>listen</code> is one flat array defaulting to <code>127.0.0.1:8000</code> in both modes, and the flag is the last word</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-mount-table-expands-at-boot">A mount matches on <code>prefix</code> or <code>host</code>, names one <code>entry</code> or one <code>scan</code>, and a scan expands against the disk at boot</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-mount-carries-no-policy">A mount's key set is <code>prefix</code>, <code>host</code>, <code>scan</code>, <code>entry</code> and <code>origin</code>; <code>mode</code>, limits and capabilities belong to the <code>[[app]]</code> block</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#a-unix-socket-listener">A <code>listen</code> entry beginning with a separator is a Unix socket, Unix-only, with <code>socket_mode</code>, and it is the transport a proxy should prefer</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#the-mode-ceiling-defaults-to-the-startup-mode"><code>[mode] ceiling</code> is <code>System</code>-class, bounds a runtime flip and not the startup value, and unset equals the mode the server started in</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#a-development-server-on-a-public-interface-warns-and-serves">A development-mode server binding a non-loopback address prints a banner, writes one <code>Warn</code> record naming the address, and serves</a><span class="nv-rule-list-status" data-status="designed">Designed</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#an-unsafe-or-unbounded-default-is-a-defect">A default that is unsafe inbound or unbounded outbound is a defect, not a neutral starting point</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#the-accept-fan-out-is-one-worker-per-core">Every <code>[server] listen</code> entry is bound before any core accepts, each core holds its own handle on every listener, and <code>[server] workers</code> bounds the count over the machine's available parallelism</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#max-in-flight-refuses-before-allocating"><code>max_in_flight</code> is a process-wide valve that answers a fixed <code>503</code> with <code>Retry-After</code> before an isolate exists</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#admission-is-arithmetic-not-a-number">The effective in-flight ceiling is the smaller of <code>max_in_flight</code> and what the memory budget affords, and a clamp is logged once at boot</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#four-idle-waits-all-finite">Four waits bound a connection, all finite with nothing configured and all idle rather than total</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#the-accept-loop-backs-off">An <code>accept</code> that fails on descriptor exhaustion is retried under a bounded backoff and logged once per window</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li></ol>

<div class="nv-rule" id="two-deployments-and-nothing-a-proxy-owns">

## The server is a development server or a proxied production origin, and nothing a proxy does earlier is in it

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#two-deployments-and-nothing-a-proxy-owns"><code>http-server/two-deployments-and-nothing-a-proxy-owns</code></a>
</div>

The built-in server has exactly two deployments. A **development server** speaks HTTP on a laptop or in a container, serves static files beside `.nvs`, and picks up an edit without a restart. A **proxied production origin** serves `.nvs` only, behind nginx, Caddy, HAProxy or Envoy, and is scoped as a FastCGI replacement. There is no third.

Everything a proxy does earlier and better is absent by name, and the list is closed: no TLS listener (`rustls` stays for the outbound client and `Core\Db`), no h2c or HTTP/2 — **HTTP/1.1 only**, because a proxy's multiplexed streams would pile onto one core of a thread-per-core runtime that never migrates a request — no FastCGI, no compression in either direction (an inbound `Content-Encoding` body passes through untouched and the application decompresses under its own memory bound), no per-IP or flood limiting, no asset-caching policy, and no rewrite rules. A production deployment with no proxy therefore has no edge protection at all; putting one in front is a stated requirement, not a gap.

The parsing half is not delegated. Request smuggling is a proxy/origin parser differential, so an origin that always has a proxy in front is precisely where a lenient parser is dangerous: [`errors/http-message-defects`](/docs/rules/errors/ambiguous-input/#http-message-defects "The closed list of what makes an HTTP message ambiguous") stands whole.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no php-fpm and no FastCGI: the origin speaks HTTP/1.1 over TCP or a Unix socket, terminates no TLS and compresses nothing, so nginx's <code>fastcgi_pass</code> becomes <code>proxy_pass</code></p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/resolving-a-request/#a-path-is-never-derived-from-a-url" title="A filesystem path is never derived from a URL at request time"><code>http-server/a-path-is-never-derived-from-a-url</code></a> <a href="/docs/rules/errors/ambiguous-input/#http-message-defects" title="The closed list of what makes an HTTP message ambiguous"><code>errors/http-message-defects</code></a> <a href="/docs/rules/security/protocols-and-tokens/#one-tls-client" title="There is one TLS client in the tree, and a driver reaches it over a generic transport rather than building a second"><code>security/one-tls-client</code></a> <a href="/docs/rules/config/reloading-and-control/#an-edit-reaches-the-next-request-without-a-restart" title="An edited source file reaches the next request that resolves it, through lazy revalidation and one pointer swap, never a watcher or a restart"><code>config/an-edit-reaches-the-next-request-without-a-restart</code></a> <a href="/docs/rules/http-server/listening-and-admission/#an-unsafe-or-unbounded-default-is-a-defect" title="A default that is unsafe inbound or unbounded outbound is a defect, not a neutral starting point"><code>http-server/an-unsafe-or-unbounded-default-is-a-defect</code></a> <a href="/docs/rules/core-classes/codecs-sessions-and-signatures/#ratelimit-two-members" title="consume and shed are two jobs with two verbs, and neither is a tier of the other"><code>core-classes/ratelimit-two-members</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0097.md">record 0097</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0083.md">record 0083</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0075.md">record 0075</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0074.md">record 0074</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0095.md">record 0095</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-server/src/io.rs"><code>crates/nvs-server/src/io.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-server/src/serve.rs"><code>crates/nvs-server/src/serve.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="the-server-block-is-boot-class">

## The `[server]` block is `Boot`-class, `listen` is one flat array defaulting to `127.0.0.1:8000` in both modes, and the flag is the last word

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#the-server-block-is-boot-class"><code>http-server/the-server-block-is-boot-class</code></a>
</div>

```toml
[server]                                  # Boot — a change here needs a restart
root               = "/www"
listen             = ["127.0.0.1:8000"]   # "host:port", or an absolute path meaning a Unix socket
socket_mode        = "0660"               # Unix-socket entries only
dispatch           = "entry"              # a mode-selected startup default; development "path"
static             = false                # a mode-selected startup default; development true
trusted_proxies    = []                   # fail-closed
health_path        = ""                   # off
max_in_flight      = 10000
workers            = 4                    # accept cores; unwritten is this machine's parallelism
header_timeout     = "10s"
body_idle_timeout  = "30s"
write_idle_timeout = "30s"
keepalive_timeout  = "75s"
```

The whole block is `Boot`-class ([`config/three-changeability-classes`](/docs/rules/config/changeability-classes/#three-changeability-classes "nvs.toml states defaults, not ceilings, and every directive carries one of three changeability classes")): `header_timeout` and `keepalive_timeout` apply before any Novis code exists on a connection, so a `Runtime` class would be a promise the block could not keep. `listen` is one flat array — an entry beginning with a separator is a Unix socket ([`http-server/a-unix-socket-listener`](/docs/rules/http-server/listening-and-admission/#a-unix-socket-listener "A listen entry beginning with a separator is a Unix socket, Unix-only, with socket_mode, and it is the transport a proxy should prefer")), and no `host:port` can be spelled that way. **The default is `127.0.0.1:8000` in both modes**: loopback is the proxied shape as well as the development one, so demanding an explicit `listen` in production would be friction with no safety in it. `nvs serve --listen`/`--port` overrides the file, on the same precedent that makes the mode flag the last word ([`config/the-mode-flag-wins-over-the-file`](/docs/rules/config/run-modes/#the-mode-flag-wins-over-the-file "The mode a server starts in comes from nvs.toml or from nvs serve --mode=, and the flag is the last word")); `--port` alone keeps the host the file chose.

`workers` is the one key here the machine answers a default for, and it is the only one that says how many accept loops there are rather than what one of them does: [`http-server/the-accept-fan-out-is-one-worker-per-core`](/docs/rules/http-server/listening-and-admission/#the-accept-fan-out-is-one-worker-per-core "Every [server] listen entry is bound before any core accepts, each core holds its own handle on every listener, and [server] workers bounds the count over the machine's available parallelism") owns the count, what a core holds of its own and what every core shares.

The accept loop backs off on descriptor exhaustion and logs once per window rather than once per attempt, and a core that stops making progress is reported and shed by a watchdog reading the in-flight deadline each worker already keeps. The waits, the valve and the probe are their own rules: [`http-server/four-idle-waits-all-finite`](/docs/rules/http-server/listening-and-admission/#four-idle-waits-all-finite "Four waits bound a connection, all finite with nothing configured and all idle rather than total"), [`http-server/max-in-flight-refuses-before-allocating`](/docs/rules/http-server/listening-and-admission/#max-in-flight-refuses-before-allocating "max_in_flight is a process-wide valve that answers a fixed 503 with Retry-After before an isolate exists"), [`http-server/health-path-is-off-and-checks-nothing`](/docs/rules/http-server/methods-bodies-and-static-files/#health-path-is-off-and-checks-nothing "health_path is off by default; set, it answers 200 while accepting and 503 while draining, and checks no dependency").

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>listen</code> is the origin's own address rather than a php-fpm pool's, and <code>workers</code> counts accept cores rather than sizing a <code>pm.*</code> process pool</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/listening-and-admission/#four-idle-waits-all-finite" title="Four waits bound a connection, all finite with nothing configured and all idle rather than total"><code>http-server/four-idle-waits-all-finite</code></a> <a href="/docs/rules/http-server/listening-and-admission/#max-in-flight-refuses-before-allocating" title="max_in_flight is a process-wide valve that answers a fixed 503 with Retry-After before an isolate exists"><code>http-server/max-in-flight-refuses-before-allocating</code></a> <a href="/docs/rules/http-server/methods-bodies-and-static-files/#health-path-is-off-and-checks-nothing" title="health_path is off by default; set, it answers 200 while accepting and 503 while draining, and checks no dependency"><code>http-server/health-path-is-off-and-checks-nothing</code></a> <a href="/docs/rules/http-server/listening-and-admission/#the-accept-fan-out-is-one-worker-per-core" title="Every [server] listen entry is bound before any core accepts, each core holds its own handle on every listener, and [server] workers bounds the count over the machine's available parallelism"><code>http-server/the-accept-fan-out-is-one-worker-per-core</code></a> <a href="/docs/rules/http-server/listening-and-admission/#a-unix-socket-listener" title="A listen entry beginning with a separator is a Unix socket, Unix-only, with socket_mode, and it is the transport a proxy should prefer"><code>http-server/a-unix-socket-listener</code></a> <a href="/docs/rules/config/changeability-classes/#three-changeability-classes" title="nvs.toml states defaults, not ceilings, and every directive carries one of three changeability classes"><code>config/three-changeability-classes</code></a> <a href="/docs/rules/config/run-modes/#the-mode-flag-wins-over-the-file" title="The mode a server starts in comes from nvs.toml or from nvs serve --mode=, and the flag is the last word"><code>config/the-mode-flag-wins-over-the-file</code></a> <a href="/docs/rules/http-server/containment/#a-wedged-core-is-detected-by-its-deadline" title="A watchdog reads the in-flight deadline each worker already keeps, and reports a core whose oldest deadline is past by a margin"><code>http-server/a-wedged-core-is-detected-by-its-deadline</code></a> <a href="/docs/rules/http-server/listening-and-admission/#the-accept-loop-backs-off" title="An accept that fails on descriptor exhaustion is retried under a bounded backoff and logged once per window"><code>http-server/the-accept-loop-backs-off</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0097.md">record 0097</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0091.md">record 0091</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0005.md">record 0005</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0106.md">record 0106</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0161.md">record 0161</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/src/server.rs"><code>crates/nvs-config/src/server.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-cli/src/serve.rs"><code>crates/nvs-cli/src/serve.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-mount-table-expands-at-boot">

## A mount matches on `prefix` or `host`, names one `entry` or one `scan`, and a scan expands against the disk at boot

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-mount-table-expands-at-boot"><code>http-server/a-mount-table-expands-at-boot</code></a>
</div>

```toml
[server]
root = "/www"                             # every mount path must resolve inside this

[[server.mount]]
scan   = "*/public/index.nvs"             # a glob under root; * captures one segment
prefix = "/{1}"                           # or host = "{1}.example.com"
origin = "https://{1}.example.com"        # optional — what urlAbsolute prepends

[[server.mount]]
prefix = "/admin"
entry  = "Backoffice/public/index.nvs"    # an explicit mount overrides a scanned one
```

A mount matches on `prefix`, on `host`, or on both, and names **either** `entry` (one literal file) **or** `scan` (a glob); both or neither is a boot error. A scan expands against the disk **at boot** into ordinary mounts, and again on `nvs ctl reload` — in development also under hot reload's revalidation. `*` matches exactly one segment; a capture must match `[A-Za-z0-9._-]+`, may not begin with a dot, and is refused if it names a reserved Windows device. Every resolved path is checked to lie inside `[server] root`, once, at boot. An explicit mount overrides a scanned one at the same key; two explicit mounts at one key is a boot error. With no block written there is one implicit mount, `{ prefix = "/", entry = "public/index.nvs" }`.

The matched prefix is stripped: `Core\Request::path()` is the remainder, `Core\Request::mount()` answers what was removed and the `tainted` captures ([`routing/a-request-reads-its-mount`](/docs/rules/routing/matching/#a-request-reads-its-mount "Core\Request::mount() answers the prefix the door stripped and the tainted host captures, and it is never null")), and `Core\Router::url` prepends the prefix ([`routing/link-carries-the-mount-prefix`](/docs/rules/routing/links-and-the-api-document/#link-carries-the-mount-prefix "A link carries the mount prefix the request arrived under, so one table serves at any mount")). A module is therefore relocatable — the same compiled route table serves at `/ModuleA` or at `/` with no recompile. `origin` is per mount, `System`-class and reloadable, with the `[[app]]` block's `origin` as the fallback ([`routing/an-origin-is-per-mount-and-checked-at-boot`](/docs/rules/routing/links-and-the-api-document/#an-origin-is-per-mount-and-checked-at-boot "An origin is declared per mount with [app] origin as the fallback, a request cannot set it, and a mount that calls urlAbsolute without one is a boot error")).

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>A vhost's rewrite rules become one <code>[[server.mount]]</code> block, and a module is relocatable because the matched prefix is stripped before the route table sees the path</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/resolving-a-request/#a-path-is-never-derived-from-a-url" title="A filesystem path is never derived from a URL at request time"><code>http-server/a-path-is-never-derived-from-a-url</code></a> <a href="/docs/rules/http-server/listening-and-admission/#a-mount-carries-no-policy" title="A mount's key set is prefix, host, scan, entry and origin; mode, limits and capabilities belong to the [[app]] block"><code>http-server/a-mount-carries-no-policy</code></a> <a href="/docs/rules/routing/matching/#a-request-reads-its-mount" title="Core\Request::mount() answers the prefix the door stripped and the tainted host captures, and it is never null"><code>routing/a-request-reads-its-mount</code></a> <a href="/docs/rules/routing/links-and-the-api-document/#link-carries-the-mount-prefix" title="A link carries the mount prefix the request arrived under, so one table serves at any mount"><code>routing/link-carries-the-mount-prefix</code></a> <a href="/docs/rules/routing/links-and-the-api-document/#an-origin-is-per-mount-and-checked-at-boot" title="An origin is declared per mount with [app] origin as the fallback, a request cannot set it, and a mount that calls urlAbsolute without one is a boot error"><code>routing/an-origin-is-per-mount-and-checked-at-boot</code></a> <a href="/docs/rules/errors/ambiguous-input/#path-component-refusals" title="A path component that does not spell what it resolves to is refused, on every platform"><code>errors/path-component-refusals</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0097.md">record 0097</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0102.md">record 0102</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0104.md">record 0104</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0017.md">record 0017</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/tests/mount.rs"><code>crates/nvs-config/tests/mount.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-mounts-captures-carry-the-mark-and-its-prefix-does-not.nvst"><code>tests/conformance/core/a-mounts-captures-carry-the-mark-and-its-prefix-does-not.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-mount-carries-no-policy">

## A mount's key set is `prefix`, `host`, `scan`, `entry` and `origin`; `mode`, limits and capabilities belong to the `[[app]]` block

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#a-mount-carries-no-policy"><code>http-server/a-mount-carries-no-policy</code></a>
</div>

A `[[server.mount]]` carries no `mode`, no limits and no capabilities. Its key set is closed — `prefix`, `host`, `scan`, `entry` and `origin` — and those keys say where a request arrives and which file answers it. Everything about what the code answering it *may do* belongs to the `[[app]]` block, keyed on the entry file path ([`config/a-mount-routes-and-an-app-block-sets-policy`](/docs/rules/config/application-blocks/#a-mount-routes-and-an-app-block-sets-policy "A [[server.mount]] routes and carries no mode; an [[app]] block sets policy"), [`config/an-application-is-its-entry-file-path`](/docs/rules/config/application-blocks/#an-application-is-its-entry-file-path "An application is its entry file path, and an [[app]] block is keyed on a root directory or one entry file")).

```toml
[[server.mount]]
prefix = "/shop"                      # routing
entry  = "shop/public/index.nvs"

[[app]]
root = "/srv/www/shop"                # policy
mode = "production"
```

The two usually cover the same tree, and that is the intended shape. An application's identity is its entry file path, not its mount, so `nvs run` on the command line has one too and per-app configuration is reachable with no server at all. A mixed-application host — production by default, each application selecting its own mode — is expressed here, not in a mount key ([`http-server/the-mode-ceiling-defaults-to-the-startup-mode`](/docs/rules/http-server/listening-and-admission/#the-mode-ceiling-defaults-to-the-startup-mode "[mode] ceiling is System-class, bounds a runtime flip and not the startup value, and unset equals the mode the server started in")).

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/listening-and-admission/#a-mount-table-expands-at-boot" title="A mount matches on prefix or host, names one entry or one scan, and a scan expands against the disk at boot"><code>http-server/a-mount-table-expands-at-boot</code></a> <a href="/docs/rules/http-server/listening-and-admission/#the-mode-ceiling-defaults-to-the-startup-mode" title="[mode] ceiling is System-class, bounds a runtime flip and not the startup value, and unset equals the mode the server started in"><code>http-server/the-mode-ceiling-defaults-to-the-startup-mode</code></a> <a href="/docs/rules/config/application-blocks/#a-mount-routes-and-an-app-block-sets-policy" title="A [[server.mount]] routes and carries no mode; an [[app]] block sets policy"><code>config/a-mount-routes-and-an-app-block-sets-policy</code></a> <a href="/docs/rules/config/application-blocks/#an-application-is-its-entry-file-path" title="An application is its entry file path, and an [[app]] block is keyed on a root directory or one entry file"><code>config/an-application-is-its-entry-file-path</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0097.md">record 0097</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0104.md">record 0104</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0005.md">record 0005</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0091.md">record 0091</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/tests/tree.rs"><code>crates/nvs-config/tests/tree.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-unix-socket-listener">

## A `listen` entry beginning with a separator is a Unix socket, Unix-only, with `socket_mode`, and it is the transport a proxy should prefer

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-unix-socket-listener"><code>http-server/a-unix-socket-listener</code></a>
</div>

A `[server] listen` entry beginning with a path separator names a Unix-domain socket; because no `host:port` can begin that way the overload is unambiguous, and `socket_mode` applies to those entries only. A Unix socket is the transport a proxy should prefer, and it is what makes FastCGI unnecessary: HTTP over a Unix socket already serves every deployment a FastCGI transport was reserved for, with no bespoke record parser and no `SCRIPT_FILENAME` handed in from outside.

**Unix sockets are Unix-only.** No Windows proxy connects to a named pipe upstream, so a Windows host listens on TCP loopback. A `curl` over the socket is byte-identical to the same request over TCP.

A Unix-socket listener is implicitly trusted for the forwarded headers ([`http-server/trusted-proxies-is-empty-and-empty-reads-nothing`](/docs/rules/http-server/resolving-a-request/#trusted-proxies-is-empty-and-empty-reads-nothing "trusted_proxies defaults to empty, empty means the forwarded headers are never read, and a trusted walk takes the rightmost untrusted X-Forwarded-For entry")), because the operating system enforces who may connect to it. The operator warning that belongs beside that: a `0660` socket is trusted by *group membership*, so adding a tenant to that group on a multi-tenant host grants them the ability to forge those headers.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>The same <code>listen = /run/nvs.sock</code> shape as php-fpm, but a Windows host has no named-pipe equivalent and listens on TCP loopback</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/listening-and-admission/#the-server-block-is-boot-class" title="The [server] block is Boot-class, listen is one flat array defaulting to 127.0.0.1:8000 in both modes, and the flag is the last word"><code>http-server/the-server-block-is-boot-class</code></a> <a href="/docs/rules/http-server/resolving-a-request/#trusted-proxies-is-empty-and-empty-reads-nothing" title="trusted_proxies defaults to empty, empty means the forwarded headers are never read, and a trusted walk takes the rightmost untrusted X-Forwarded-For entry"><code>http-server/trusted-proxies-is-empty-and-empty-reads-nothing</code></a> <a href="/docs/rules/config/stores-and-caches/#a-unix-socket-is-admitted-only-where-an-operator-wrote-it" title="A Unix socket is admitted in [cache.shared] url and [db.&lt;name&gt;] host, and refused from any program-supplied endpoint"><code>config/a-unix-socket-is-admitted-only-where-an-operator-wrote-it</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0097.md">record 0097</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0093.md">record 0093</a></dd></div></dl>

</div>

<div class="nv-rule" id="the-mode-ceiling-defaults-to-the-startup-mode">

## `[mode] ceiling` is `System`-class, bounds a runtime flip and not the startup value, and unset equals the mode the server started in

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#the-mode-ceiling-defaults-to-the-startup-mode"><code>http-server/the-mode-ceiling-defaults-to-the-startup-mode</code></a>
</div>

`[mode] ceiling` states the most permissive mode any code on the host may select. It is `System`-class — a request can never raise it ([`config/ceilings-are-their-own-directives`](/docs/rules/config/changeability-classes/#ceilings-are-their-own-directives "A ceiling is a System directive with the default's key name, and false removes it")) — and **when unset it equals the mode the server started in.**

| Deployment | Written | Result |
|---|---|---|
| Production host | nothing | Started in `production`, ceiling `production`. No code path anywhere reaches development mode. |
| Developer's machine | `nvs serve --mode=development` | Ceiling `development`. Flips are free; nothing to configure. |
| One host, mixed applications | `[mode] default = "production"`, `[mode] ceiling = "development"` | Production by default, and each application selects its own — in its `[[app]]` block, or in code. |

**The ceiling bounds a runtime flip, not the startup value.** `nvs serve --mode=development` in a directory with no `nvs.toml` simply works: the flag sets the startup mode ([`config/the-mode-flag-wins-over-the-file`](/docs/rules/config/run-modes/#the-mode-flag-wins-over-the-file "The mode a server starts in comes from nvs.toml or from nvs serve --mode=, and the flag is the last word")) and the ceiling follows it. A ceiling that also bound startup would have made the most common first-run command fail. Above the ceiling, `Core\Config::set("mode.default", …)` returns `false` and leaves the mode unchanged ([`config/a-program-may-read-and-flip-its-mode`](/docs/rules/config/run-modes/#a-program-may-read-and-flip-its-mode "A program may read the mode and may flip it for its own request, bounded by a System ceiling")).

For the mixed host, per-app configuration is the primary answer and the in-code flip is the escape hatch: the `[[app]]` block involves no application code and nothing the application can get wrong ([`config/a-mount-routes-and-an-app-block-sets-policy`](/docs/rules/config/application-blocks/#a-mount-routes-and-an-app-block-sets-policy "A [[server.mount]] routes and carries no mode; an [[app]] block sets policy")). `Core\Config::set` is for when the application knows something the operator does not.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no <code>php.ini-development</code> to copy: a production host that wrote nothing is unreachable from code, and one root-owned line opens it</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/config/run-modes/#a-program-may-read-and-flip-its-mode" title="A program may read the mode and may flip it for its own request, bounded by a System ceiling"><code>config/a-program-may-read-and-flip-its-mode</code></a> <a href="/docs/rules/config/changeability-classes/#ceilings-are-their-own-directives" title="A ceiling is a System directive with the default's key name, and false removes it"><code>config/ceilings-are-their-own-directives</code></a> <a href="/docs/rules/config/run-modes/#the-mode-flag-wins-over-the-file" title="The mode a server starts in comes from nvs.toml or from nvs serve --mode=, and the flag is the last word"><code>config/the-mode-flag-wins-over-the-file</code></a> <a href="/docs/rules/config/application-blocks/#a-mount-routes-and-an-app-block-sets-policy" title="A [[server.mount]] routes and carries no mode; an [[app]] block sets policy"><code>config/a-mount-routes-and-an-app-block-sets-policy</code></a> <a href="/docs/rules/http-server/listening-and-admission/#a-mount-carries-no-policy" title="A mount's key set is prefix, host, scan, entry and origin; mode, limits and capabilities belong to the [[app]] block"><code>http-server/a-mount-carries-no-policy</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0091.md">record 0091</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0005.md">record 0005</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0104.md">record 0104</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/config-a-mode-flip-above-the-ceiling-is-refused.nvst"><code>tests/conformance/core/config-a-mode-flip-above-the-ceiling-is-refused.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/src/request.rs"><code>crates/nvs-config/src/request.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="a-development-server-on-a-public-interface-warns-and-serves">

## A development-mode server binding a non-loopback address prints a banner, writes one `Warn` record naming the address, and serves

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="designed">Designed</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#a-development-server-on-a-public-interface-warns-and-serves"><code>http-server/a-development-server-on-a-public-interface-warns-and-serves</code></a>
</div>

When the server binds a non-loopback address while the mode is `development`, it emits an unmissable startup banner **and** a `Warn` record naming the bound address ([`errors/log-level`](/docs/rules/errors/diagnostics-and-logging/#log-level "Five levels, and the mapping to syslog is fixed")), then serves. Bound to loopback it emits neither.

Refusing the bind outright behind an unlock directive is not taken: binding `0.0.0.0` inside a container or a VM is the normal case, not the exceptional one, and a refusal would put a required directive in front of every containerised development workflow. The banner alone is not enough either — it scrolls past in a container log — which is why it is *also* a `Warn` record: it lands in the same JSON-Lines stream everything else does, queryable after the fact rather than only observable at the moment of startup.

This is the one place where letting the mode flag win over the file ([`config/the-mode-flag-wins-over-the-file`](/docs/rules/config/run-modes/#the-mode-flag-wins-over-the-file "The mode a server starts in comes from nvs.toml or from nvs serve --mode=, and the flag is the last word")) is paid for, and the payment is deliberately visible: the banner is what stands between a stale `--mode=development` in a deploy script and a public debug surface.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>php -S 0.0.0.0:8000</code> says nothing; here the bind is announced on the console and in the log stream, and it is not refused</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/listening-and-admission/#the-server-block-is-boot-class" title="The [server] block is Boot-class, listen is one flat array defaulting to 127.0.0.1:8000 in both modes, and the flag is the last word"><code>http-server/the-server-block-is-boot-class</code></a> <a href="/docs/rules/config/run-modes/#the-mode-flag-wins-over-the-file" title="The mode a server starts in comes from nvs.toml or from nvs serve --mode=, and the flag is the last word"><code>config/the-mode-flag-wins-over-the-file</code></a> <a href="/docs/rules/errors/diagnostics-and-logging/#log-level" title="Five levels, and the mapping to syslog is fixed"><code>errors/log-level</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0091.md">record 0091</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0092.md">record 0092</a></dd></div></dl>

</div>

<div class="nv-rule" id="an-unsafe-or-unbounded-default-is-a-defect">

## A default that is unsafe inbound or unbounded outbound is a defect, not a neutral starting point

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#an-unsafe-or-unbounded-default-is-a-defect"><code>http-server/an-unsafe-or-unbounded-default-is-a-defect</code></a>
</div>

A default that is unsafe inbound or unbounded outbound is a defect, not a neutral starting point
a deployment is expected to improve on. The four `[http.*]` blocks exist to make that one sentence
true with nothing written: every response carries the secure header set
([`http-server/secure-headers-with-nothing-written`](/docs/rules/http-server/headers-cors-and-cookies/#secure-headers-with-nothing-written "A deployment that wrote no [http.headers] already sends nosniff, frame-ancestors 'none' and a referrer policy on every response")), CORS is closed
([`http-server/cors-is-closed-until-origins-are-named`](/docs/rules/http-server/headers-cors-and-cookies/#cors-is-closed-until-origins-are-named "CORS is closed until [http.cors] origins names one: no header is emitted and a preflight is answered 403")), every cookie is `Secure; HttpOnly;
SameSite=Lax` ([`http-server/cookies-are-secure-httponly-and-lax`](/docs/rules/http-server/headers-cors-and-cookies/#cookies-are-secure-httponly-and-lax "Every cookie Core\Response::addCookie writes is Secure; HttpOnly; SameSite=Lax; Path=/ unless the call site says otherwise")), and no outbound call can
wait forever ([`http-server/no-spelling-for-an-unbounded-wait`](/docs/rules/http-server/the-http-client/#no-spelling-for-an-unbounded-wait "Core\Http\Client has no spelling for wait forever: every bound is a Duration, an omitted one inherits [http.client], and expiry throws TimeoutError")).

The rule reaches past the client. A `[db.<name>]` pool bound, a socket wait, a terminal prompt and
a queue lease each inherit a finite default and refuse `false` or `0` as a spelling for "no
ceiling", because a wait that never ends is how one slow dependency becomes an outage and the
request-level `wall_time` only bounds the damage after the fact. It also answers what a store
nobody chose means — no store ([`http-server/no-session-block-means-no-store`](/docs/rules/http-server/sessions/#no-session-block-means-no-store "A tree with no [session] block has no session store, and start throws naming the block to write")) — since the
safe answer for an absent block is the one that cannot be wrong silently.

What a proxy does earlier and better — request-size caps, per-IP connection limits, flood
limiting — stays the proxy's. Two things do not: how a message is parsed, which is
[`errors/ambiguous-input-refused`](/docs/rules/errors/ambiguous-input/#ambiguous-input-refused "Ambiguous input is refused whole, never repaired")'s, and how long a connection may idle, which this rule
covers as it covers a client call.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Php-fpm behind nginx ships no security headers, no cookie flags and no outbound timeout until somebody writes them; here nothing written is already the safe and finite configuration</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/headers-cors-and-cookies/#secure-headers-with-nothing-written" title="A deployment that wrote no [http.headers] already sends nosniff, frame-ancestors 'none' and a referrer policy on every response"><code>http-server/secure-headers-with-nothing-written</code></a> <a href="/docs/rules/http-server/headers-cors-and-cookies/#cors-is-closed-until-origins-are-named" title="CORS is closed until [http.cors] origins names one: no header is emitted and a preflight is answered 403"><code>http-server/cors-is-closed-until-origins-are-named</code></a> <a href="/docs/rules/http-server/headers-cors-and-cookies/#cookies-are-secure-httponly-and-lax" title="Every cookie Core\Response::addCookie writes is Secure; HttpOnly; SameSite=Lax; Path=/ unless the call site says otherwise"><code>http-server/cookies-are-secure-httponly-and-lax</code></a> <a href="/docs/rules/http-server/the-http-client/#no-spelling-for-an-unbounded-wait" title="Core\Http\Client has no spelling for wait forever: every bound is a Duration, an omitted one inherits [http.client], and expiry throws TimeoutError"><code>http-server/no-spelling-for-an-unbounded-wait</code></a> <a href="/docs/rules/config/the-file-and-the-tree/#no-configuration-file-is-a-complete-configuration" title="A host with no configuration file runs on the shipped defaults, which are a complete configuration and not an error"><code>config/no-configuration-file-is-a-complete-configuration</code></a> <a href="/docs/rules/http-server/listening-and-admission/#the-server-block-is-boot-class" title="The [server] block is Boot-class, listen is one flat array defaulting to 127.0.0.1:8000 in both modes, and the flag is the last word"><code>http-server/the-server-block-is-boot-class</code></a> <a href="/docs/rules/http-server/methods-bodies-and-static-files/#the-body-is-read-on-demand-under-two-caps" title="[limits] request_body bounds bytes parsed into memory and upload_total bounds a streamed multipart body, each with a hard ceiling, and a route that never reads a body allocates nothing"><code>http-server/the-body-is-read-on-demand-under-two-caps</code></a> <a href="/docs/rules/http-server/containment/#a-requests-blast-radius-is-bounded-at-four-tiers" title="Nothing a request can send terminates or wedges a worker: its blast radius is bounded at four named tiers, and the residue is one stated fault class"><code>http-server/a-requests-blast-radius-is-bounded-at-four-tiers</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0074.md">record 0074</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-server/src/secure.rs"><code>crates/nvs-server/src/secure.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/no-client-member-accepts-an-unbounded-wait.nvst"><code>tests/conformance/core/no-client-member-accepts-an-unbounded-wait.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="the-accept-fan-out-is-one-worker-per-core">

## Every `[server] listen` entry is bound before any core accepts, each core holds its own handle on every listener, and `[server] workers` bounds the count over the machine's available parallelism

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#the-accept-fan-out-is-one-worker-per-core"><code>http-server/the-accept-fan-out-is-one-worker-per-core</code></a>
</div>

`nvs serve` binds every entry of `[server] listen` before it accepts anything, and then starts one worker per core, each holding its own handle on every one of those listeners — so a connection is accepted by whichever core reaches it first, rather than by one core that hands it on.

**`[server] workers` bounds the count, and the default is what the machine answers.** With the key left out the fan-out is [`std::thread::available_parallelism`](https://doc.rust-lang.org/std/thread/fn.available_parallelism.html), so a deployment moved onto a bigger box scales without a directive being edited, and a host that answers nothing at all is one core. A written count is the bound in **both** directions: it is neither raised to the machine's parallelism nor clamped down to it, because a core count a heuristic chose is one the file cannot state, and an operator who sized a container against its CPU quota has already answered the question. `0` is refused under `E0636` — every listener is accepted on by a worker, so a count of zero binds the addresses the tree names and then answers nobody on any of them, which is [`http-server/the-server-block-is-boot-class`](/docs/rules/http-server/listening-and-admission/#the-server-block-is-boot-class "The [server] block is Boot-class, listen is one flat array defaulting to 127.0.0.1:8000 in both modes, and the flag is the last word")'s empty `listen` array reached from the other end. There is no `auto` spelling: leaving the key out is what asks for the machine's own answer.

**What the cores share is compiled program text and the process cache tier, and nothing else.** Every mounted entry is compiled before the first socket is bound, published once behind an `Arc` that every core reads, so the compile count is one per distinct source rather than one per core — [`security/isolate-shares-nothing`](/docs/rules/security/isolates/#isolate-shares-nothing "Running another script is an in-process isolate that shares nothing with its parent but compiled code")'s "shares immutable compiled code", and the same single publisher an edit revalidates through ([`config/an-edit-reaches-the-next-request-without-a-restart`](/docs/rules/config/reloading-and-control/#an-edit-reaches-the-next-request-without-a-restart "An edited source file reaches the next request that resolves it, through lazy revalidation and one pointer swap, never a watcher or a restart")). Beside it stands the one mutable thing: the map behind `Core\Cache::process()`, created before the first core accepts, read and written by every one of them, sharded behind locks and bounded by `[cache.process] max_size` ([`concurrency/the-process-tier-is-one-store-per-process`](/docs/rules/concurrency/deferred-and-cross-request-state/#the-process-tier-is-one-store-per-process "Core\Cache::process() is one store per serving process, coherent across its cores and gone when it ends")). Everything else a core holds is its own: its scheduler, its run queue, its accept loop and the backoff that loop applies ([`http-server/the-accept-loop-backs-off`](/docs/rules/http-server/listening-and-admission/#the-accept-loop-backs-off "An accept that fails on descriptor exhaustion is retried under a bounded backoff and logged once per window")), which is what [`http-server/admission-is-arithmetic-not-a-number`](/docs/rules/http-server/listening-and-admission/#admission-is-arithmetic-not-a-number "The effective in-flight ceiling is the smaller of max_in_flight and what the memory budget affords, and a clamp is logged once at boot")'s relaxed in-flight counter and [`http-server/a-wedged-core-is-detected-by-its-deadline`](/docs/rules/http-server/containment/#a-wedged-core-is-detected-by-its-deadline "A watchdog reads the in-flight deadline each worker already keeps, and reports a core whose oldest deadline is past by a margin")'s per-worker watchdog are already written against.

**A listener the server cannot bind is refused once, before any core starts.** The classification of a `listen` entry belongs to the configuration and not to a core ([`http-server/a-unix-socket-listener`](/docs/rules/http-server/listening-and-admission/#a-unix-socket-listener "A listen entry beginning with a separator is a Unix socket, Unix-only, with socket_mode, and it is the transport a proxy should prefer")), so one deployment mistake is one refusal — a fan-out that bound as it started would report the same wrong entry once per core and would leave a process half-listening while it did.

What it costs, per [`programs/memory-priority`](/docs/rules/programs/claims-and-priorities/#memory-priority "Memory buys security, correctness, latency and simplicity — bounded, attributable and stated"): one scheduler, one accept loop and one listener handle per listener per core, all paid at process start and O(cores) rather than O(requests served). It holds strictly less than a per-core compiled-unit cache would, which is the shape this one replaces.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Php-fpm sizes a pool of request *processes* with <code>pm.*</code>; <code>workers</code> counts accept cores, and one core answers many requests at once</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/listening-and-admission/#the-server-block-is-boot-class" title="The [server] block is Boot-class, listen is one flat array defaulting to 127.0.0.1:8000 in both modes, and the flag is the last word"><code>http-server/the-server-block-is-boot-class</code></a> <a href="/docs/rules/http-server/listening-and-admission/#a-unix-socket-listener" title="A listen entry beginning with a separator is a Unix socket, Unix-only, with socket_mode, and it is the transport a proxy should prefer"><code>http-server/a-unix-socket-listener</code></a> <a href="/docs/rules/http-server/listening-and-admission/#the-accept-loop-backs-off" title="An accept that fails on descriptor exhaustion is retried under a bounded backoff and logged once per window"><code>http-server/the-accept-loop-backs-off</code></a> <a href="/docs/rules/http-server/listening-and-admission/#admission-is-arithmetic-not-a-number" title="The effective in-flight ceiling is the smaller of max_in_flight and what the memory budget affords, and a clamp is logged once at boot"><code>http-server/admission-is-arithmetic-not-a-number</code></a> <a href="/docs/rules/security/isolates/#isolate-shares-nothing" title="Running another script is an in-process isolate that shares nothing with its parent but compiled code"><code>security/isolate-shares-nothing</code></a> <a href="/docs/rules/config/reloading-and-control/#an-edit-reaches-the-next-request-without-a-restart" title="An edited source file reaches the next request that resolves it, through lazy revalidation and one pointer swap, never a watcher or a restart"><code>config/an-edit-reaches-the-next-request-without-a-restart</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0161.md">record 0161</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0181.md">record 0181</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/src/server.rs"><code>crates/nvs-config/src/server.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-cli/src/serve.rs"><code>crates/nvs-cli/src/serve.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="max-in-flight-refuses-before-allocating">

## `max_in_flight` is a process-wide valve that answers a fixed `503` with `Retry-After` before an isolate exists

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#max-in-flight-refuses-before-allocating"><code>http-server/max-in-flight-refuses-before-allocating</code></a>
</div>

`[server] max_in_flight` (`10000`) is a process-wide safety valve, not a worker pool. At the ceiling the server answers a fixed `503` with `Retry-After: 1` **before allocating an isolate, compiling anything or running any Novis code** — a cap that allocates in order to refuse does not protect what it exists to protect, and the consequence accepted is that this path has no custom error page.

The count is process-wide through one relaxed atomic rather than per core, so one hot core cannot refuse while its neighbours idle; that counter is not on the value path the non-atomic-refcount decision protects. Not accepting at all was rejected: behind a proxy a silent refusal surfaces as a 504 blamed on the wrong component, and gives the proxy no signal to fail over on.

The effective ceiling is an arithmetic, not this number alone: the smaller of what is configured here and what the memory budget affords against the per-request cap, clamped and logged once at boot when the two disagree. A concurrency ceiling and a per-request memory cap with no stated relationship bound nothing together, and their product is what the machine is actually asked to hold.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>Php-fpm's <code>pm.max_children</code> queues on the socket and the proxy reports a 504; here the origin answers <code>503</code> with <code>Retry-After: 1</code> itself, and no custom error page runs</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/listening-and-admission/#the-server-block-is-boot-class" title="The [server] block is Boot-class, listen is one flat array defaulting to 127.0.0.1:8000 in both modes, and the flag is the last word"><code>http-server/the-server-block-is-boot-class</code></a> <a href="/docs/rules/errors/the-escalation-ladder/#on-limit" title="Tier 1 — a resource limit reaches the request that spent it"><code>errors/on-limit</code></a> <a href="/docs/rules/http-server/listening-and-admission/#admission-is-arithmetic-not-a-number" title="The effective in-flight ceiling is the smaller of max_in_flight and what the memory budget affords, and a clamp is logged once at boot"><code>http-server/admission-is-arithmetic-not-a-number</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0097.md">record 0097</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0106.md">record 0106</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-server/src/admit.rs"><code>crates/nvs-server/src/admit.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="admission-is-arithmetic-not-a-number">

## The effective in-flight ceiling is the smaller of `max_in_flight` and what the memory budget affords, and a clamp is logged once at boot

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#admission-is-arithmetic-not-a-number"><code>http-server/admission-is-arithmetic-not-a-number</code></a>
</div>

`max_in_flight` refuses work before allocating an isolate, which is the right shape. But a ceiling on *concurrency* and a cap on *per-request memory* that have no stated relationship do not bound anything together: their product is what the machine must hold, and if that product exceeds what it has, the operating system's out-of-memory killer is the real admission control — and it terminates the process, which is tier A's failure arriving through a door every cap above was supposed to have closed.

**The effective ceiling is the smaller of the configured `max_in_flight` and what the memory budget affords** — the budget divided by the per-request cap, less the engine's own fixed footprint, with a container's limit preferred to the host's. When the configured number is the larger, it is clamped and the clamp is logged once at boot, naming both directives and both numbers. The refusal over the ceiling is a fixed `503` with `Retry-After` and no body, taken before a mount is selected, and the counter behind it is one relaxed atomic for the process.

Clamping rather than refusing to start: a server that will not boot because two directives disagree is a worse outage than the one being prevented, and the operator learns the same fact either way. Clamping *silently* was rejected because the observed capacity of a small instance drops where the clamp binds — a real change in a number people notice, and one an operator should find in the log rather than in a benchmark.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>pm.max_children</code> is a free number and the out-of-memory killer enforces its product with memory; here the ceiling is derived from the budget, the configured number is clamped rather than trusted, and the overflow is a <code>503</code></p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/uploads/#upload-total-is-enforced-on-the-wire" title="upload_total is enforced by the server on the wire: a declared oversize is refused before dispatch and a chunked one is stopped at the part being read"><code>http-server/upload-total-is-enforced-on-the-wire</code></a> <a href="/docs/rules/http-server/containment/#a-wedged-core-is-shed-never-killed" title="A reported core stops accepting and max_in_flight counts its share as unavailable; a wedged worker is never killed in-process"><code>http-server/a-wedged-core-is-shed-never-killed</code></a> <a href="/docs/rules/http-server/containment/#a-requests-blast-radius-is-bounded-at-four-tiers" title="Nothing a request can send terminates or wedges a worker: its blast radius is bounded at four named tiers, and the residue is one stated fault class"><code>http-server/a-requests-blast-radius-is-bounded-at-four-tiers</code></a> <a href="/docs/rules/security/isolates/#isolate-budget-is-the-trees" title="A request and everything it spawns share one budget, accounted at the tree's root"><code>security/isolate-budget-is-the-trees</code></a> <a href="/docs/rules/programs/claims-and-priorities/#memory-priority" title="Memory buys security, correctness, latency and simplicity — bounded, attributable and stated"><code>programs/memory-priority</code></a> <a href="/docs/rules/http-server/listening-and-admission/#the-server-block-is-boot-class" title="The [server] block is Boot-class, listen is one flat array defaulting to 127.0.0.1:8000 in both modes, and the flag is the last word"><code>http-server/the-server-block-is-boot-class</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0106.md">record 0106</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0097.md">record 0097</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0004.md">record 0004</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-server/src/admit.rs"><code>crates/nvs-server/src/admit.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-config/src/server.rs"><code>crates/nvs-config/src/server.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="four-idle-waits-all-finite">

## Four waits bound a connection, all finite with nothing configured and all idle rather than total

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#four-idle-waits-all-finite"><code>http-server/four-idle-waits-all-finite</code></a>
</div>

Four waits bound every connection — `header_timeout` (`10s`), `body_idle_timeout` (`30s`), `write_idle_timeout` (`30s`) and `keepalive_timeout` (`75s`) — and all four are finite with nothing configured. All four are **idle** waits rather than totals: a slow 2 GB upload that keeps moving completes, and a stalled socket does not. Each fires on a stalled connection and none on a slow-but-progressing one.

This is the division of labour with the proxy in one sentence: **a proxy owns size and rate; Novis owns never waiting forever.** Per-IP caps, flood limiting and request-size shedding stay at the edge.

`keepalive_timeout` must exceed the proxy's upstream keep-alive. If the origin closes an idle connection the proxy still believes is live, the proxy writes into a closing socket and the client sees an intermittent 502. nginx's upstream default is 60s; 75s is deliberately above it, and a deployment that raises the proxy's must raise this one. None of the four notices a worker that is alive and never returns; that is the watchdog's job, named in [`http-server/the-server-block-is-boot-class`](/docs/rules/http-server/listening-and-admission/#the-server-block-is-boot-class "The [server] block is Boot-class, listen is one flat array defaulting to 127.0.0.1:8000 in both modes, and the flag is the last word").

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>max_execution_time</code> and <code>request_terminate_timeout</code> bound a whole request; these bound a stalled socket only, so a slow 2 GB upload completes while a stalled one is closed</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/listening-and-admission/#the-server-block-is-boot-class" title="The [server] block is Boot-class, listen is one flat array defaulting to 127.0.0.1:8000 in both modes, and the flag is the last word"><code>http-server/the-server-block-is-boot-class</code></a> <a href="/docs/rules/concurrency/connections/#connection-bounds-are-finite" title="Every bound on an open connection is finite with nothing configured"><code>concurrency/connection-bounds-are-finite</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0097.md">record 0097</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0074.md">record 0074</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-server/src/io.rs"><code>crates/nvs-server/src/io.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-server/src/serve.rs"><code>crates/nvs-server/src/serve.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="the-accept-loop-backs-off">

## An `accept` that fails on descriptor exhaustion is retried under a bounded backoff and logged once per window

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#the-accept-loop-backs-off"><code>http-server/the-accept-loop-backs-off</code></a>
</div>

An `accept` that fails with `EMFILE`/`ENFILE` returns immediately and will fail again immediately, which turns descriptor exhaustion into a core pinned at full utilisation for as long as the condition lasts — and, because the log is written per iteration, into a disk filled at the speed of the loop.

**The loop applies a bounded backoff on a descriptor-exhaustion error and logs once per window**, not once per attempt: the wait doubles to a ceiling and a successful accept puts it back. No other accept failure is waited out, because no other one is a condition that will clear on its own. Descriptors are already charged to a request under [`errors/multipart-part-count`](/docs/rules/errors/ambiguous-input/#multipart-part-count "A multipart body is capped by part count, not only by size")'s per-request accounting, so the condition is bounded from the other side too; this closes the behaviour when it happens anyway.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/http-server/containment/#the-floor-cannot-fill-the-disk" title="The floor cannot fill the disk it writes to: a file target rotates under a retention bound, repeats coalesce into one record with a count, and a cache write failure compiles in memory"><code>http-server/the-floor-cannot-fill-the-disk</code></a> <a href="/docs/rules/http-server/containment/#a-requests-blast-radius-is-bounded-at-four-tiers" title="Nothing a request can send terminates or wedges a worker: its blast radius is bounded at four named tiers, and the residue is one stated fault class"><code>http-server/a-requests-blast-radius-is-bounded-at-four-tiers</code></a> <a href="/docs/rules/errors/ambiguous-input/#multipart-part-count" title="A multipart body is capped by part count, not only by size"><code>errors/multipart-part-count</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0106.md">record 0106</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0095.md">record 0095</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-server/src/serve.rs"><code>crates/nvs-server/src/serve.rs</code></a></dd></div></dl>

</div>
