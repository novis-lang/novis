# ADR 0097 — The built-in server is a development server and a proxied origin, and a URL never becomes a path

- **Status:** Accepted
- **Date:** 2026-08-26
- **Scope:** what the built-in server is for and, as a closed list, what it deliberately is not; the mount
  table and how a URL selects one entry point out of many; the `[server]` block, its listeners and its
  finite waits; which facts a proxy in front is trusted to assert and under what condition; the in-flight
  ceiling; and which HTTP conventions the server applies above
  [0077](0077-compile-time-routing.md)'s route table. Not in scope: the response policy — secure headers,
  CORS and cookies — which stays [0074](0074-http-defaults-safe-and-finite.md); rate limiting
  ([0075](0075-core-ratelimit.md)); the control socket and `mwl ctl`
  ([0078](0078-config-reload-and-control-socket.md)); the service installer
  ([0093](0093-a-service-is-one-stored-argv-and-the-installer-is-a-sink.md)); hot reload's mechanism
  ([0017](0017-hot-reload-without-restart.md)); the route table, its grammar and its three compile errors
  ([0077](0077-compile-time-routing.md)); persistent connections ([0083](0083-persistent-connections-are-isolates.md));
  how a message's spelling is read ([0095](0095-ambiguous-input-is-refused-never-repaired.md)); and the
  per-app capability and limit block, which is [0005](0005-config-changeability.md)'s and which § 10 records
  as undefined rather than defining here.
- **Amends:** [0017](0017-hot-reload-without-restart.md) — `opcache.validate` gains a mode-selected startup
  default, and the directory listings it already revalidates gain a second consumer in § 3's scan.
  [0025](0025-wasm-browser-target.md) — its precedent for a contingent milestone can no longer be M13, which
  § 1 closes. [0093](0093-a-service-is-one-stored-argv-and-the-installer-is-a-sink.md) — the "configured
  listen addresses" its `AmbientCapabilities` rule reads are § 5's `[server] listen`.
  [0051](0051-standard-library-tiers.md) § 3 — `Core\Compress`'s Tier 0 placement
  keeps its brotli and zstd rows on the decompression-bomb-is-policy reason that ADR already gives
  `Core\Zip`, not on a server request path that § 1 removes.
  [0005](0005-config-changeability.md) — its claim that per-app blocks "already live in the root config"
  now names the gap § 10 records. [0064](0064-configuration-file-format.md) § 2a — the block table
  gains `[server]` and `[[server.mount]]`, and `[limits]`/`[limits.hard]` gain `request_body`.
  [0074](0074-http-defaults-safe-and-finite.md) — HSTS is emitted when the effective scheme is `https`
  rather than only over a TLS connection MWL now never terminates, and its *Alternatives* line pushing edge
  concerns to a proxy is narrowed a second time: the proxy owns **size and rate**, MWL owns **never waiting
  forever** (§ 5). [0075](0075-core-ratelimit.md) — its "no IP limiting is a gap on paper" becomes a stated
  deployment requirement rather than an admitted hole. [0076](0076-observability-export.md) — a trace id is
  generated for **every** request regardless of the sampling decision, and is emitted on the response.
  [0077](0077-compile-time-routing.md) § 4 — the three conventions it handed to this milestone are decided
  in § 7, and `Core\Router::url` prepends the request's mount prefix.
  [0083](0083-persistent-connections-are-isolates.md) — its upgrade path is h1-only, so RFC 8441 extended
  `CONNECT` is not reachable and not needed. [0091](0091-run-mode-is-two-values-a-ceiling-and-a-list-of-defaults.md)
  — § 3 gains a `[log] access` row, a new § 3a carries the `Boot`/`System` defaults a mode selects, and § 5's
  mixed-application host gains its mechanism in a mount's `mode`.
  [0095](0095-ambiguous-input-is-refused-never-repaired.md) — its closed list gains two rows (§ 6), and its
  *Context* no longer argues from a deployment with no proxy.
  [docs/spec/01-core-library.md](../spec/01-core-library.md) § 15 — `Core\Request` gains `scheme`,
  `isHead`, `mountPrefix` and `bodyStream`, `clientIp` gains a defined source, and `Core\Server` gains
  `isDraining` and `traceId`.
  [docs/plan/m7.md](../plan/m7.md) and [docs/plan/m13.md](../plan/m13.md) — M7 gains this ADR's surface and
  loses TLS and h2c; M13 is deleted.
- **Relates to:** 0004, 0005, 0006, 0012, 0024, 0042, 0059, 0061, 0072, 0079, 0080, 0092

> **In short:** the server has exactly **two deployments** and no third — a **development server** that
> serves static files beside `.mwl`, and a **proxied production origin** that replaces FastCGI. Everything a
> proxy does earlier and better is dropped by name: no TLS listener, no h2c, no compression in either
> direction, no edge limiting, no asset-caching policy. What replaces "one entry point" is a single
> governing rule — **a filesystem path is never derived from a URL at request time**. FastCGI's RCE family
> exists precisely because it is; here a URL *selects* a mount from a table that a glob expanded against
> disk **at boot**. So a fleet of modules costs one line of configuration, wildcards are free, and
> `mwl info --config` still prints every path that can ever execute.

## Context

- **The scope had no home.** [docs/adr/README.md](README.md) § *Where to look* routed "the built-in HTTP
  server" to [0017](0017-hot-reload-without-restart.md), which owns hot reload. With no document stating
  what the server is for, a premise stated once travelled: "MWL is frequently deployed with no proxy at all"
  reached [0095](0095-ambiguous-input-is-refused-never-repaired.md)'s *Context*,
  [0074](0074-http-defaults-safe-and-finite.md)'s *Alternatives* and
  [0075](0075-core-ratelimit.md)'s *Consequences* without ever being decided anywhere.
- **FastCGI's vulnerability class is not "many entry points", it is "a URL-derived path".** The
  `SCRIPT_FILENAME`/`PATH_INFO`/`cgi.fix_pathinfo` family exists because the web server *computes* a
  filesystem path from the request and hands it to a runtime that trusts it. The count of entry points is
  irrelevant; the derivation is the whole defect. A design that enumerates its entry points ahead of time is
  immune at any number of them.
- **Multi-entry deployments are the normal shape, not an exception.** A document root holding many modules,
  each with its own `public/index.mwl`, reached by vhost rewrite rules, is how a large fraction of PHP is
  actually deployed. A server that accepts one entry point per process answers it with N processes, N unit
  caches and N times the resident memory — discarding the shared compiled-code cache that is one of MWL's
  headline properties.
- **Thread-per-core makes HTTP/2 upstream actively harmful**, which the earlier "h2c is nearly free"
  reasoning did not weigh. [docs/plan/design.md](../plan/design.md) pins one single-threaded runtime per
  core and balances *connections* across them, and a request never migrates — that is what buys non-atomic
  refcounts. A proxy speaking h2c deliberately opens few connections and multiplexes heavily, so its streams
  concentrate on one core while the rest idle, and the design cannot rebalance them by construction.
- **[0080](0080-the-audience-mwl-is-built-for.md) sets the direction for every trust question here.** On a
  multi-tenant host, "trust the loopback peer" is wrong: another tenant can connect over loopback and forge
  a header. Trust must be written down, and the shipped default must be the safe one.

## Decision

### 1. Two deployments, and a closed list of what this is not

**a. A development server.** HTTP only, on a laptop or in a container. It serves static files beside `.mwl`
and picks up an edit without a restart.

**b. A proxied production origin.** `.mwl` only, behind nginx, Caddy, HAProxy or Envoy. It is a **FastCGI
replacement** and is scoped as one.

There is no third. **The list of what a proxy owns is closed, and each entry is dropped by name:**

| Not in the server | Because |
|---|---|
| **A TLS listener** | The proxy terminates. Also removes cert loading, rotation across a reload, SNI, ALPN, cipher and version policy — and the session-ticket key, which is process-wide shared secret state that [0059](0059-cross-request-state-is-explicit.md) would have to carve an exception for. `rustls` stays for outbound `Core\Http\Client` and `Core\Db`. |
| **h2c and HTTP/2** | *Context* above. Also drops the Rapid Reset and `CONTINUATION`-flood classes, and keeps [0083](0083-persistent-connections-are-isolates.md)'s upgrade on h1 where it is native. **HTTP/1.1 only.** |
| **FastCGI** | HTTP over a Unix socket already serves every deployment a FastCGI transport was reserved for, with no bespoke record parser and no `SCRIPT_FILENAME` handed in from outside. This is [0051](0051-standard-library-tiers.md)'s ordinary "something better replaces it" rather than a [0052](0052-closed-doors.md) door, so this table is its only home. |
| **Compression, both directions** | A proxy or CDN compresses responses better and off the request path. An inbound `Content-Encoding` body is passed through untouched — the application calls `Core\Compress` with its own output bound, so the decompression-bomb budget is the request's own `[limits] memory` rather than a policy the server picks blind. |
| **Edge limiting** | Per-IP connection caps, flood limiting and request-size shedding stay the proxy's, which is the line [0075](0075-core-ratelimit.md) already draws. |
| **An asset-caching policy** | No configurable `max-age`, no `immutable`, no precompressed variant lookup. § 4's static serving is a development convenience and a fallback, not a CDN. |
| **Rewrite rules** | Capture reordering, query-string rewriting and conditional rules are what a proxy is for. They would also put a regex on the request path and make § 3's table un-enumerable, which is the property § 2 exists to keep. |

**The parsing half is not delegated.** [0095](0095-ambiguous-input-is-refused-never-repaired.md) stands
whole and its reason is *strengthened* here, not weakened: request smuggling **is** a proxy/origin parser
differential, so a deployment that always has a proxy in front is exactly the deployment where a lenient
origin parser is dangerous.

### 2. A filesystem path is never derived from a URL at request time

This is the rule the rest of the ADR is built to keep. A request may **select** an entry point from a set
enumerated before the request arrived; it may never **construct** one.

Every path the server can execute is therefore known at boot, printable by `mwl info --config`, and fixed
until a reload. That is what makes § 3's wildcards safe: expanding a glob against the disk at boot produces
a literal table, whereas the same glob evaluated per request would be `cgi.fix_pathinfo` with a different
spelling.

### 3. A mount table, and a glob that expands at boot

```toml
[server]
root = "/www"                             # every mount path must resolve inside this

[[server.mount]]                          # one rule, every module present and future
scan   = "*/public/index.mwl"             # a glob under [server] root; * captures one path segment
prefix = "/{1}"                           # or host = "{1}.example.com"
mode   = "production"                     # optional, § 10

[[server.mount]]                          # an irregular module, overriding the scan at its key
prefix = "/admin"
entry  = "Backoffice/public/index.mwl"
```

- **A mount matches on `prefix`, on `host`, or on both**, and names **either** `entry` (one literal file)
  **or** `scan` (a glob). Naming both, or neither, is a boot error.
- **A scan is expanded against the disk at boot**, producing ordinary mounts. `{1}` is the first `*`
  capture. `*` matches exactly one path segment; a capture must match `[A-Za-z0-9._-]+`, may not begin with
  a dot, and is refused if it names a reserved Windows device
  ([0095](0095-ambiguous-input-is-refused-never-repaired.md) § 5 owns that list).
- **Every resolved path is checked to resolve inside `[server] root`** — once, at boot, not per request.
- **An explicit mount overrides a scanned one at the same key**, so one irregular module costs one block.
  Two explicit mounts at one key is a boot error.
- **Expansion re-runs on `mwl ctl reload`.** In development it may also re-run under
  [0017](0017-hot-reload-without-restart.md)'s revalidation, whose directory-listing set
  [0061](0061-compile-time-autoload-and-program-discovery.md) § *Interaction* already established for this
  milestone — so a newly dropped-in module appearing without a reload reuses a planned mechanism rather than
  adding one.
- **With no `[[server.mount]]` written at all**, there is one implicit mount:
  `{ prefix = "/", entry = "public/index.mwl" }`.

**The matched prefix is stripped.** `Core\Request::path()` is the remainder, `Core\Request::mountPrefix()`
is what was removed, and `Core\Router::url` prepends it. A module is therefore **relocatable**: the same
compiled [0077](0077-compile-time-routing.md) route table declaring `#[Route(path: "/users/{id}")]` serves
at `/ModuleA`, at `/ModuleB` or at `/`, with no recompile and no base-path setting. Because `url` is already
that ADR's launderer for the URL-path sink, an application is already required to route link generation
through it rather than concatenating strings, so nothing new is asked of anyone.

### 4. How a request resolves, in five steps

```
1. longest match:  host mounts by prefix  ->  host-less mounts by prefix  ->  404
2. strip the prefix
3. [server] static  &&  the remainder is an existing non-.mwl file under the mount root  -> serve it
4. dispatch == "path"  &&  the remainder is an existing .mwl file under the mount root   -> run it
5. otherwise                                                                             -> run the mount's entry
```

In production (`dispatch = "entry"`, `static = false`) steps 3 and 4 do not run: match, strip, run the
entry. In development the sequence is `try_files $uri /index.mwl` — the pattern every PHP application
already deploys under.

**Static serving is one policy in both modes**, because a second policy is a second security model. Exact
file only and **never a directory listing**; `index.html` is the sole default document; the MIME type comes
from a fixed extension table and an unknown one is `application/octet-stream`, which
[0074](0074-http-defaults-safe-and-finite.md)'s `nosniff` renders inert. Freshness is
`Cache-Control: no-cache` with a strong `ETag` over `(size, mtime_nanos)` and `If-None-Match` — one
validator, exact, and specifically **not** `Last-Modified`, whose one-second granularity serves stale bytes
for two edits inside the same second. A single `Range` is honoured; a multi-range request is refused.
**A `.mwl` file is never served as source**, under any dispatch, from any mount.

`[server] static` is a boolean and the *path* is each mount's own root, which is what makes assets work for
a fleet of modules rather than only for one.

### 5. The `[server]` block

```toml
[server]                                  # Boot — a change here needs a restart
root               = "/www"
listen             = ["127.0.0.1:8000"]   # "host:port", or an absolute path meaning a Unix socket
socket_mode        = "0660"               # Unix-socket entries only
dispatch           = "entry"              # § 3a of ADR 0091 — development default "path"
static             = false                # § 3a of ADR 0091 — development default true
trusted_proxies    = []                   # § 6, fail-closed
health_path        = ""                   # off
max_in_flight      = 10000
header_timeout     = "10s"
body_idle_timeout  = "30s"
write_idle_timeout = "30s"
keepalive_timeout  = "75s"
```

- **One flat `listen` array.** An entry beginning with a separator is a Unix socket; no `host:port` can be
  spelled that way, so the overload is unambiguous. A Unix socket is the transport a proxy should prefer and
  is what makes FastCGI unnecessary. **Unix sockets are Unix-only**: no Windows proxy connects to a named
  pipe upstream, so Windows listens on TCP loopback. `mwl serve --listen`/`--port` overrides the file, on
  [0091](0091-run-mode-is-two-values-a-ceiling-and-a-list-of-defaults.md) § 2's precedent that the flag is
  the last word.
- **The default is `127.0.0.1:8000` in both modes.** Loopback is the proxied shape as well as the
  development one, so requiring an explicit `listen` in production would be friction with no safety in it.
- **Four waits, all finite with nothing configured, and all *idle* rather than total** — so a slow 2 GB
  upload completes while a stalled socket does not. This narrows
  [0074](0074-http-defaults-safe-and-finite.md)'s delegation rather than contradicting it: **a proxy owns
  size and rate; MWL owns never waiting forever**, which is that ADR's own headline. They are `Boot`-class
  because `header_timeout` and `keepalive_timeout` both apply before any MWL code exists on that connection,
  so a `Runtime` class would be a promise two of the four could not keep.
- **`keepalive_timeout` must exceed the proxy's upstream keep-alive.** If the origin closes an idle
  connection the proxy still believes is live, the proxy writes into a closing socket and the client gets an
  intermittent 502. nginx's upstream default is 60s; 75s is deliberately above it, and a deployment that
  raises the proxy's must raise this one.
- **`max_in_flight` is a process-wide safety valve, not a worker pool.** At the ceiling the server answers a
  fixed `503` with `Retry-After: 1` **before allocating an isolate, compiling anything or running any MWL
  code** — a cap that allocates in order to refuse does not protect what it exists to protect, and the
  consequence to accept is that this path has no custom error page. It is counted process-wide through one
  relaxed atomic rather than per core, so one hot core cannot refuse while its neighbours idle; that counter
  is not on the value path the non-atomic-refcount decision protects. Not accepting at all was rejected:
  behind a proxy it surfaces as a 504 blamed on the wrong component, and gives the proxy no signal to fail
  over on.
- **`health_path` is off by default**, so no URL is silently reserved. When set it answers `200` while
  accepting and `503` while draining, with an empty body, and is skipped by the access log. It performs
  **no dependency checks** — a health endpoint that pings the database converts a slow database into a
  simultaneous outage across every instance — and reports no version or build information.
  `Core\Server::isDraining()` gives an application the same fact for an endpoint of its own. A built-in
  probe reports that the *process* is alive even when the application fails to compile, where an
  application-route probe would fail and produce a restart loop that cannot fix a compile error.

### 6. What a proxy is trusted to assert

**`[server] trusted_proxies` defaults to empty, and empty means the headers are not read at all.**

- **Empty** ⇒ `Core\Request::clientIp()` is the socket peer and `X-Forwarded-For` is never parsed.
  `Core\Request::header` still returns it, `tainted`, for an application that wants to decide for itself.
- **Non-empty** ⇒ `clientIp` is the **rightmost** `X-Forwarded-For` entry that is not itself trusted,
  walking right-to-left from the peer. Leftmost-wins is fully attacker-controlled and is the version most
  frameworks shipped first.
- **A Unix-socket listener is implicitly trusted**, because the OS enforces who may connect to it. The
  operator warning that belongs beside that: a `0660` socket is trusted by *group membership*, so adding a
  tenant to that group on a multi-tenant host grants them the ability to forge these headers.
- **One `Warn` at boot** when the mode is `production`, every listener is loopback or a Unix socket, and
  `trusted_proxies` is empty — the shape of a proxied deployment that forgot the line and will now log the
  proxy's address as every client's.

**`X-Forwarded-Proto` from a trusted peer sets the effective scheme, and feeds exactly two things**:
`Core\Request::scheme()` and HSTS emission. It does **not** feed redirects: `Core\Response::redirect` emits
`Location` as given and relative is permitted, so no scheme is ever reconstructed — the downgrade-and-loop
bug behind a TLS-terminating proxy is removed rather than handled. It does not feed the cookie `Secure`
flag, which [0074](0074-http-defaults-safe-and-finite.md) sets unconditionally and correctly. And there is
**no `X-Forwarded-Host` and no absolute-URL generation from `Host`**: [0077](0077-compile-time-routing.md)
§ 4 already makes `Core\Router::url` answer with a path, so MWL never needs to know its own external origin,
and deriving one from a header is host-header injection.

**`X-Forwarded-For` is the only forwarded-address header read.** RFC 7239 `Forwarded` is not read at all —
supporting both is what *creates* an ambiguity that would then have to be refused, and no common proxy emits
`Forwarded` by default. The defects resolve four ways, and they do not resolve alike:

| Input | Behaviour |
|---|---|
| Several `X-Forwarded-For` field lines | joined with commas, then walked — RFC 7230 § 3.2.2 permits combining a list-valued field, so every conforming reader agrees and this is *unusual*, not *ambiguous* |
| `Forwarded` present, no `X-Forwarded-For`, peer trusted | ignored, request served, **one** `Warn` — a detectable proxy misconfiguration, and refusing would take a site down over a header MWL chose not to support |
| The token the walk lands on does not parse as an IP | **400** — the value was about to be used and cannot be |
| `X-Forwarded-For` from an untrusted peer | **ignored silently, never refused** |

The last row is load-bearing. Any client can set that header, so refusing on its mere presence would let
anyone deny service by sending it — or by getting a scanner to. Refusal is reserved for the case where the
value would have been *used*.

**Host matching.** An absent `Host` on HTTP/1.1 is a `400`. Comparison is on the host part only: the port is
stripped, the value is ASCII-lowercased and one trailing dot is removed — all three are equivalences the
relevant specifications define, so this is [0095](0095-ambiguous-input-is-refused-never-repaired.md)'s
*accept verbatim* branch rather than a repair. Non-ASCII in `Host` is refused; punycode matches literally. A
deployment that declares only host mounts gets a `404` for an unknown host with no allowlist directive to
remember, which is the failure Django had to add `ALLOWED_HOSTS` for. **The proxy note that belongs in the
operator documentation**: nginx's `proxy_pass` sends `Host: $proxy_host` unless told otherwise, so a
host-mounted deployment must set `proxy_set_header Host $host;` or every request arrives with the wrong name
and lands on the fallback — silently.

**Two rows join [0095](0095-ambiguous-input-is-refused-never-repaired.md) § 2's closed list**: an
`X-Forwarded-For` token in the trusted-walk position that does not parse as an IP address, and a request
path containing a dot-segment or an encoded separator (`%2f`). The second also removes any possibility of
one mount's prefix being confused for another's.

### 7. The three conventions above ADR 0077's table

[0077](0077-compile-time-routing.md) § 4 handed three to this milestone. They are three different questions.

- **`HEAD` is implemented.** RFC 9110 requires a general-purpose server to support it, so this is
  conformance rather than convention: the request runs as `GET`, the body is discarded and `Content-Length`
  is kept. **`Core\Request::method()` reports `Get`** — the application passes it to `Core\Router::match`
  itself, so reporting `Head` would fail the match against a `Get` route and produce the 404 the feature
  exists to prevent. `Core\Request::isHead()` exposes the truth for the rare caller that wants it.
- **A CORS preflight is answered, and nothing else about `OPTIONS` is.** A preflight — `OPTIONS` carrying
  `Origin` and `Access-Control-Request-Method` — is answered from `[http.cors]` before any application code,
  because [0074](0074-http-defaults-safe-and-finite.md) already owns that policy and it is closed by
  default. A plain `OPTIONS` is passed through: answering it means emitting `Allow:` for that path, and in
  entry dispatch the server has no route table to ask.
- **A trailing slash is never normalised.** `/users` and `/users/` are different URIs and whether they name
  one resource is application knowledge. It is also the only one of the three a proxy does trivially, and
  the one [0095](0095-ambiguous-input-is-refused-never-repaired.md)'s never-repair instinct argues hardest
  against.

### 8. The body, uploads, and the streaming reader

The cap is **`[limits] request_body` with a `[limits.hard]` ceiling** — [0005](0005-config-changeability.md)'s
existing `Runtime`-default-plus-`System`-ceiling pair, not a third instance of it, which is what keeps
[0091](0091-run-mode-is-two-values-a-ceiling-and-a-list-of-defaults.md)'s "second and last" wording true.
Defaults are `"8M"` and `"64M"`. Because `Core\Request::body()` is a call, the body is read on demand: a
route that never reads one allocates nothing, and an upload route may raise its own cap before reading.

**An uploaded file is `tainted bytes` in memory. There is no temp file** — no `tmp_name`, no
`move_uploaded_file`, no temp directory to configure, permission, defend against symlinks, clean up after a
crash, or bound against disk exhaustion that no memory cap covers. This is [0004](0004-memory-for-simplicity.md)'s
priority ordering applied literally: memory is last and simplicity is above it, and the bytes stay
attributable to one request, under an enforceable cap, and O(in-flight).

```mwl
foreach (Core\Request::files() as $f) {
    $f->name;         // tainted string — the client's claimed filename, never a path
    $f->contentType;  // tainted string
    $f->content;      // tainted bytes
}
```

**`Core\Request::bodyStream(): Iterable<bytes>`** is the second path, for a body larger than a request's
memory budget: it yields chunks over [0053](0053-iteration-and-generators.md)'s protocol, each `tainted`,
and consuming it is exclusive with `body()` and `files()` on one request. `request_body` bounds the buffered
path; the streaming path is bounded by the connection's `body_idle_timeout` and by whatever the consumer
does with each chunk. [0095](0095-ambiguous-input-is-refused-never-repaired.md)'s multipart **part count**
cap applies to both, since part accounting is a cost no byte cap bounds.

### 9. Logging, and one identifier

**`[log] access` is a fifth row in [0091](0091-run-mode-is-two-values-a-ceiling-and-a-list-of-defaults.md)
§ 3** — `true` in development, `false` in production. In production an access line is a third copy of a fact
the proxy's log and [0076](0076-observability-export.md)'s root span already hold, written with request-path
I/O; in development there is no proxy and traces are sampled, so it is the most useful thing the server
prints. It renders through [0092](0092-one-diagnostic-record-three-renderings.md)'s single record — text in
development, JSON in production — and there is deliberately **no Common or Combined Log Format**, which
would be a second renderer that ADR says should not exist. The fields are method, path, status, duration,
response bytes, `clientIp`, the [0077](0077-compile-time-routing.md) route name when the application matched
one, and the trace id.

**An error is logged unconditionally**, whatever `access` says: a `5xx` is a diagnostic rather than access
telemetry, and losing one because access logging was off would be the wrong failure.

**The trace id is the request identifier. There is no second one.** A trace id exists for every request
because W3C TraceContext generates one regardless of the sampling decision — sampling governs only whether
the trace is *exported* — so the id needed to join a log line to an error page to a proxy log entry is
already present on every request. `Core\Server::traceId()` reads it, every log record and every error
rendering carries it, and it is emitted on the response so a proxy can log it with one `log_format` line.
**An inbound `X-Request-ID` is ignored**, which avoids a validation rule against log injection for a fact
MWL already has.

### 10. A mount carries `mode`, and the per-app block is a gap this ADR does not fill

A mount may set `mode`, bounded by `[mode] ceiling`. That fulfils
[0091](0091-run-mode-is-two-values-a-ceiling-and-a-list-of-defaults.md) § 5's mixed-application host, whose
third row promised per-application mode selection and had no configuration mechanism to point at.

**A mount carries nothing else, and the reason is a doc bug this ADR records rather than fixes.**
[0005](0005-config-changeability.md) states that per-app blocks "already live in the root config";
[0064](0064-configuration-file-format.md) § *Revisiting* discusses their layout and calls it 0005's
question; [0091](0091-run-mode-is-two-values-a-ceiling-and-a-list-of-defaults.md) § 5 builds a deployment on
them. **No document defines what one is keyed on or how one is spelled.** Defining capabilities and limits
here would put a second home on a fact 0005 nominally owns, trading one doc bug for a worse one.

**The constraint to carry into that design:** an application's identity should be its **entry file path**,
not its mount. A mount-keyed block would leave `mwl run` on the CLI with no per-application identity at all,
and would make per-app configuration unreachable until M7 even though 0005 lands in M6.

## Consequences

- **MWL cannot be a single-binary HTTPS origin.** Go services and Caddy can; this closes that door for as
  long as § 1 stands. [0048](0048-portable-single-file-executables.md)'s bundle and
  [0093](0093-a-service-is-one-stored-argv-and-the-installer-is-a-sink.md)'s installed service both now
  describe something that runs behind a proxy.
- **A production deployment with no proxy has no edge protection at all** — no per-IP limiting, no flood
  shedding, no TLS. [0075](0075-core-ratelimit.md)'s "gap on paper" is now a stated requirement: put a proxy
  in front. What MWL still guarantees alone is the parsing half
  ([0095](0095-ambiguous-input-is-refused-never-repaired.md)) and the finite waits of § 5.
- **The largest buffered upload equals a request's memory budget.** Past that, § 8's streaming reader is the
  answer and the application writes the chunks somewhere itself.
- **Browsers always speak HTTP/1.1 to MWL**, since h2 requires TLS and § 1 removes it. This costs nothing in
  development and nothing behind a proxy, which terminates h2 or h3 for the client either way.
- **A mount scan reads the filesystem at boot and on reload**, so a deployment adding a module makes it
  reachable by dropping a directory in place — and a deployment that did *not* intend that must not put an
  unexpected directory under `[server] root`. `mwl info --config` printing the expanded table is what makes
  that inspectable rather than a surprise.
- **[0091](0091-run-mode-is-two-values-a-ceiling-and-a-list-of-defaults.md)'s single-sentence property
  becomes two.** "No row is `System`-class" was clean; § 3a splits it, and every future directive now has to
  answer which table it belongs in. Accepted because the alternative is that `mwl serve --mode=development`
  enables none of hot reload, static files or path dispatch — `php.ini-development` reinvented as a file
  people copy without reading.

## Alternatives rejected

- **One entry point per process.** The original form of this decision. Rejected once the multi-module
  deployment was stated: it answers a document root of N modules with N processes and N unit caches,
  discarding the shared compiled-code cache, and it defends an invariant (§ 2) that a table of any size
  already satisfies.
- **Apache-style rewrite rules evaluated per request.** Maximum flexibility, and exactly the FastCGI defect
  with different syntax: a URL segment interpolated into a filesystem path at request time. The boot-time
  expansion in § 3 gives the same one-line-covers-every-module ergonomics while leaving the executable set
  enumerable, which is the property [0091](0091-run-mode-is-two-values-a-ceiling-and-a-list-of-defaults.md)
  established as non-negotiable for anything a mode or a convenience controls.
- **A URL path mapped to a file in production too** (`php -S` in both modes). Rejected: production has a
  front controller and a compiled route table, and every additional URL-to-file resolution is surface on the
  hot path for an ergonomic that only helps a scratch workflow.
- **Keeping the `Transport` trait for a future FastCGI.** Rejected with M13: an abstraction justified by one
  hypothetical implementor that HTTP-over-a-Unix-socket already covers. The internal request-to-response
  function boundary stays, because [0079](0079-testing-is-a-language-feature.md) § 18's synthetic request
  and `#[Test(server: true)]` are two real callers of it.
- **h2c behind a directive, defaulting off.** Rejected: the attack surface still ships, the core-imbalance
  behaviour becomes a support question rather than an impossibility, and "nearly free" stops being true once
  it needs a directive, documentation and tests.
- **Transparent decompression of a request body.** Rejected: the server would choose a bomb-ratio policy for
  an application whose intent it cannot know, needing its own directive and class, and it silently transforms
  input.
- **Reading RFC 7239 `Forwarded` beside `X-Forwarded-For`.** Rejected: two sources that can disagree is the
  definition of the defect [0095](0095-ambiguous-input-is-refused-never-repaired.md) refuses, bought for a
  header no common proxy emits by default.
- **Refusing any request carrying `X-Forwarded-For` from an untrusted peer.** The strictest reading, and a
  denial-of-service footgun: the header is client-settable, so anyone could refuse a deployment by sending
  it.
- **Trusting loopback by default.** Convenient, and wrong for [0080](0080-the-audience-mwl-is-built-for.md)'s
  first audience, where another local tenant can forge the header.
- **`Last-Modified` for static files.** Rejected in § 4: one-second granularity serves stale bytes for two
  edits inside one second, which costs an hour before anyone suspects the server.
- **A separate `X-Request-ID`.** Rejected in § 9: a trace id already exists on every request, and two
  identifiers for one fact is how people grep the wrong one.
- **Defining per-app capability and limit blocks here.** Rejected in § 10: 0005 owns that fact, and a second
  home is worse than the missing one.

## Verification

In M7, alongside the fixtures [docs/plan/m7.md](../plan/m7.md) already lists:

- **§ 2, the governing rule.** A test enumerates every path the server can execute after boot and asserts
  it equals the expanded mount table; no request, however spelled, adds to that set. A path containing a
  dot-segment or `%2f` is refused before any mount is selected.
- **§ 3, the mount table.** A scan over a fixture tree of three modules expands to three mounts;
  `mwl info --config` prints all three with their source; an explicit mount at a scanned key wins; two
  explicit mounts at one key refuse at boot; a scan capture containing a separator, a leading dot or a
  reserved Windows device name is refused; an expanded path resolving outside `[server] root` is refused.
  A module mounted at `/ModuleA` and the same module mounted at `/` both serve, and `Core\Router::url`
  answers with the mount's prefix in each.
- **§ 4, resolution.** In production a request for an existing `.mwl` file under the mount root runs the
  *entry*, not that file. In development it runs that file. A `.mwl` file is never returned as source in
  either. A directory request never lists. A second `GET` of an unchanged asset is a `304`; an asset edited
  twice within one second serves the second edit.
- **§ 5.** Each of the four waits fires on a stalled connection and none fires on a slow-but-progressing
  one. `max_in_flight` returns `503` with `Retry-After` and a counter asserts **no isolate was allocated**
  for the refused request. A Unix-socket listener serves; a `curl` over it is byte-identical to TCP.
- **§ 6.** The `X-Forwarded-For` walk returns the rightmost untrusted entry across a fixture matrix
  including a spoofed leading entry; with `trusted_proxies` empty the header is not parsed and `clientIp` is
  the peer; an untrusted peer's header is ignored rather than refused; a non-IP token in the walk position
  is a `400`; HSTS is emitted with `X-Forwarded-Proto: https` from a trusted peer and not from an untrusted
  one; `redirect` never emits a scheme. An absent `Host` is a `400`; `Host: A.EXAMPLE.COM:8080.` matches the
  mount declared for `a.example.com`.
- **§ 7.** `HEAD` on a `Get`-only route returns the `GET` headers with no body and the same
  `Content-Length`; a preflight is answered without the application running; `/users/` does not reach a
  `/users` route.
- **§ 8.** An upload arrives as `tainted bytes` with no path anywhere in the value; a body over
  `[limits] request_body` is refused and a route that raised its own cap first accepts it; `bodyStream`
  consumes a body larger than the buffered cap at bounded memory; using both on one request is refused.
- **§ 9.** Every log record and every error rendering carries the trace id, with `[trace] sample` at zero;
  an inbound `X-Request-ID` appears nowhere; a `5xx` is logged with `[log] access = false`.
