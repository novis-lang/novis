# ADR 0078 — `mwl.toml` reloads over a local control socket, and the extension set joins the compilation key

- **Status:** Accepted
- **Date:** 2026-08-24
- **Scope:** the config snapshot and how it is replaced, the registry's reloadability field, the control
  socket and `mwl ctl`, and the environment component of both compiled-unit cache keys. Not in scope: any
  network-reachable control surface, which § 6 defers explicitly.
- **Amends:** [0017](0017-hot-reload-without-restart.md) § *Decision* — `UnitKey` gains the environment
  digest, so an in-memory unit compiled against one extension set is never reused against another.
  [0005](0005-config-changeability.md) § *What is `System`* — reloadability becomes a second,
  orthogonal field rather than a property of the `System` class; [0064](0064-configuration-file-format.md)
  §§ 1 and 6 — the file is no longer read only at boot;
  [0042](0042-on-disk-artifact-cache-format.md) §§ 2 and 6 — the key gains the extension set;
  [0003](0003-extension-system.md) § *Isolation, limits and loading* — the extension set is reloadable.
- **Amended by:** 0091, 0103, 0106

> **In short:** the parsed config is one immutable `Arc<Config>`; a request clones it at start and is
> unaffected by anything that happens afterwards. `mwl ctl reload` re-reads `mwl.toml` over a
> **local socket only** — no TCP listener, no token, no TLS, because the socket's owner and mode
> *are* the authentication — parses and validates the whole file **before** publishing anything, and
> keeps the old snapshot untouched if any part of it fails. Every directive carries a second field
> beside its changeability class: **`Reload`** or **`Boot`**, where `Boot` means applying it would
> rebind an OS resource, and reload *names the `Boot` keys that changed* rather than silently ignoring
> them. The `[[extension]]` set is `Reload`, made possible by folding it into one `env_hash` that both
> the in-memory and on-disk cache keys carry — which also closes a latent hole where an artifact
> compiled against one extension set could be reused against another.

## Context

- [0064](0064-configuration-file-format.md) read `mwl.toml` once at boot, so every configuration change —
  a limit, a capability grant, an extension — cost a restart. MWL is a *single process* serving its whole
  concurrency ([0017](0017-hot-reload-without-restart.md) targets 10k+), so a restart drops every in-flight
  request rather than a fraction of a worker pool. That is a materially worse trade than the same restart
  costs PHP-FPM, and it is the reason this is worth solving at all.
- The obvious shape — re-read the file on each request, since requests are isolated anyway — misreads what
  isolation requires. Requests must share nothing *mutable*; an immutable snapshot shared by all of them is
  the same category as the compiled code [0017](0017-hot-reload-without-restart.md) § *Decision* already
  shares. [0005](0005-config-changeability.md) went further still: a request already gets a copy-on-write
  overlay over the base, so the per-request half of config isolation exists. What was missing was a safe way
  to replace the *base*.
- A latent correctness hole sharpened the problem. [0042](0042-on-disk-artifact-cache-format.md)'s key covered
  the source, the target triple, the CPU features and the compiler build — but not the extension set, even
  though [0003](0003-extension-system.md) § *Extension functions are statically typed* has codegen emit a
  **direct call to an extension trampoline**. Restarting with a changed set could therefore reuse an artifact
  holding a direct call into a trampoline that had moved, and 0042's claim that content-addressed entries
  "never need invalidating for correctness" held only because extensions did not exist yet.
- Those two turn out to be one problem. Making the extension set part of a unit's *identity* fixes the hole
  and makes extension reload fall out of machinery 0017 already has, with no invalidation pass at all.

## Decision

### 1. The config is an immutable snapshot, replaced whole

The parsed registry is an `Arc<Config>`. A request clones the `Arc` when it starts and reads from that clone
for its whole life, so a reload is never visible to a request already running and no request ever observes
half of one file and half of another. `Core\Config::set` continues to write into the request's copy-on-write
overlay over that snapshot, exactly as [0005](0005-config-changeability.md) specifies; nothing about the
per-request half changes.

Replacement is **validate-then-publish**, in this order, and any failure before the last step leaves the
running configuration completely untouched:

1. Read and parse `mwl.toml`. A syntax error, an unknown key or a duplicate key ends it here.
2. Verify every `[[extension]]` entry: the file exists and its content matches its `sha256` pin.
3. Load those extensions' manifests and register the `mwl.toml` directives they contribute
   ([0003](0003-extension-system.md) § *Extension functions are statically typed*).
4. Validate the assembled registry — every directive known, every value in range, every capability grant
   well-formed. Steps 3 and 4 are in this order deliberately: an extension can add directives, so the file
   cannot be fully validated before its extensions are known.
5. Compute `env_hash` (§ 4) and publish the new `Arc`.

**The pins are re-verified on every reload, not only at first load.** A reload is precisely the moment a
file that arrived from outside might have changed underneath its pin.

### 2. Reloadability is its own field, orthogonal to the changeability class

[0005](0005-config-changeability.md)'s three classes answer *who may set a directive* — a request, or only
the file. This ADR adds a second field answering a different question, *what applying a change requires*:

| Field | Meaning |
|---|---|
| `Reload` | a new snapshot is enough |
| `Boot` | applying it would rebind an OS resource or re-create the runtime |

The two are independent: a `System` directive may be `Reload` (a capability grant, `[limits.hard]`), and
every `Runtime`/`RuntimeTighten` directive is `Reload` by construction, since such a directive *is* a value
read out of the snapshot.

`Boot` is the narrow set: `cache.dir`, the server's listen addresses, the thread-per-core count, and
`[control] socket` itself. Everything else is `Reload`, including the `[[extension]]` array and its pins,
`opcache.validate` and its rate cap, the per-app blocks, `[[schedule]]`, `[deferred] max_concurrent` and both
observability blocks. A `[[schedule]]` firing already in flight runs to completion; the new set arms from the
next tick.

This *replaces* [0005](0005-config-changeability.md)'s older definition of `System` as "everything read once
at boot", which had already needed a second criterion bolted beside it. With reloadability held in its own
field, `System` means one thing again: a request may not set it.

### 3. One local socket, and `mwl ctl`

```toml
[control]
socket = "/run/mwl/control.sock"   # \\.\pipe\mwl-control on Windows; `false` disables
```

- **Local socket only. There is no TCP listener, no token, no TLS and no auth middleware** — the socket's
  owner and mode are the authentication, the same permission-check argument
  [0042](0042-on-disk-artifact-cache-format.md) § 5 already makes for the cache directory. The socket is
  created mode `0600`, owned by the runtime's account, and **the server refuses to start if its directory is
  world-writable**.
- The socket exists only where a long-running server does. It is meaningless for `mwl run`, which compiles
  one file and exits, and for the wasm32 target ([0025](0025-wasm-browser-target.md)), which has no host.
- **The wire protocol is HTTP over that socket**, not a bespoke line protocol: `hyper` is already present for
  the server itself, `curl --unix-socket` debugs it with no special tooling, and adding a network listener
  later becomes a second `bind` rather than a second protocol.
- **`mwl ctl` is the client**, a namespace of its own because every other subcommand (`run`, `check`, `test`,
  `fmt`, `info`, `config`) acts on files with no server involved. `--socket` addresses one of several
  servers on a host. `reload` and **`ctl config`** are the operations:
  [0103 § 9](0103-configuration-is-a-tree-of-files.md) adds the second, which prints the live snapshot with
  each directive's origin. It is a read, it runs no MWL code, and it is what the offline `mwl config dump`
  cannot answer — what a reload actually published, including an `optional` include that has appeared since
  boot. What `reload` re-reads is the whole **tree** that ADR describes, re-running its ownership checks on
  every file, so a file that became group-writable since boot refuses the swap and leaves the previous
  snapshot serving.
- **Operations serialize**, single-flighting on the same [0017](0017-hot-reload-without-restart.md) machinery
  a concurrent compile already uses, so two reloads cannot interleave two snapshots.
- **No control operation runs user MWL code**, ever. One that could would be
  [0052](0052-closed-doors.md) § 4's `eval` door with a different name on it.
- **The wire shape is explicitly unstable until 1.0.** Every response carries the server version and
  `mwl ctl` refuses a mismatch, so a changed shape is a clear error rather than a misparse. It is therefore
  not part of [0068](0068-dependency-currency-and-the-version-contract.md)'s enumerated surface.
- Every reload is written to `Core\Log` with its outcome.

### 4. The extension set joins one `env_hash`, carried by both cache keys

```
extension_set_hash = BLAKE3(sorted sha256 pins of the [[extension]] array)
env_hash           = BLAKE3(target_triple ‖ cpu_feature_bitset ‖ compiler_version_hash ‖ extension_set_hash)
```

`env_hash` replaces the three environment fields that [0042](0042-on-disk-artifact-cache-format.md) § 2
carried separately, and is now carried by **both** caches:

- **On disk** — the key becomes `BLAKE3(source_content ‖ env_hash)`, which closes the hole described in
  *Context* and makes 0042's "never need invalidating for correctness" true rather than nearly true.
- **In memory** — [0017](0017-hot-reload-without-restart.md)'s `UnitKey { path, content_hash }` becomes
  `UnitKey { path, content_hash, env_hash }`.

Because a manifest lives *inside* its `.mwlx` as a custom section ([0003](0003-extension-system.md) § *Tier 1*),
hashing the pins covers each extension's declared classes, signatures and
[0055](0055-extension-qualifier-declarations.md) qualifier declarations with no separate manifest hash.
**Duplicate class names across extensions are refused at load**, so the set is order-independent and the
sorted hash is well defined.

Extension reload then needs no invalidation pass whatsoever: a changed set changes `env_hash`, every unit key
changes with it, every lookup is an ordinary miss, and 0017's existing lazy per-path revalidation recompiles
each unit on the compile pool as some request resolves it. Old generations are retained only while in-flight
requests hold them, under the O(in-flight) bound 0017 § *Consequences* already carries.

**Invalidation is coarse: changing the set rekeys every unit, not only units that call an extension.** Making
it finer needs per-unit dependency tracking *including negative dependencies* — a unit that failed to compile
because `Image` was unknown must be retried when `Image` appears — which is a real subsystem for a modest win.

### 5. Reload reports what it did, and what it could not do

The response names, in one place: the directives applied; the **`Boot` keys whose values changed and
therefore did not take effect, each named individually**; and how many compiled units were invalidated, so an
operator knows a recompile wave is coming. A validation failure reports the offending line and states that
the running configuration is unchanged.

Naming the ignored `Boot` keys is the difference between a reload an operator can trust and one they have to
guess about. Silently ignoring a changed listen address is how a deployment ends up believing it applied a
change it did not.

**Removing an extension that source code still references** is not caught here — proving it would mean
compiling every unit before the swap. Those units fail to compile when a request next resolves them, failing
that request loudly, which is exactly the policy [0017](0017-hot-reload-without-restart.md) § *Consequences*
already chose over silently serving stale code.

### 6. No network-reachable control surface

There is no TCP listener, in either direction of configuration. A remote control plane is reachable today by
running `mwl ctl` over the operator's existing access path — SSH, or the container runtime's exec — which
every orchestrator already has.

This is deferred rather than rejected, and *Revisiting* records what it would take, so that the design is not
re-derived from scratch. The reason to defer is that a network listener is the only part of this design that
would carry an authentication surface, and nothing yet needs one.

## Consequences

**Cost, as [0004](0004-memory-for-simplicity.md) requires it be stated.**

- **Memory, per process:** two `Config` snapshots live during a swap, plus one per in-flight request still
  holding an older one — kilobytes each, bounded by concurrency in flight, never by reloads performed.
- **Memory, on an extension change:** one extra generation of compiled units, retained exactly as long as
  some in-flight request holds it. This is the O(in-flight) bound
  [0017](0017-hot-reload-without-restart.md) already carries and accepts, not a new one.
- **Request-path latency:** one `Arc` clone at request start. **No syscall** — unlike 0017's source
  revalidation, config is never polled from the request path; a reload is pushed by the operator.
- **After an extension change:** the first request to resolve each path pays a compile. The wave is lazy,
  spread across paths, and runs on the compile pool rather than a request-serving core. It is a real latency
  bump and is still strictly better than the restart it replaces, which pays the same compiles cold *and*
  drops every in-flight request. **The wave is also staggered**: single-flight dedupes concurrent compiles
  of one unit, but an `env_hash` change invalidates every unit at once, and a live server would otherwise
  meet that as a recompile of its whole working set inside one moment. The compile pool bounds how much of
  the wave is in flight together
  ([0106](0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md) § 10).

**Other consequences.**

- An operator can change limits, capability grants, HTTP policy defaults and the extension set without
  dropping a request. Listen addresses, `cache.dir` and the core count still need a restart, and reload says
  so by name.
- [0042](0042-on-disk-artifact-cache-format.md) gets simpler: three environment fields in its key and header
  collapse into one, and a latent correctness hole closes on the way.
- The security surface added is one socket whose permissions are checked at startup. No token to leak, no
  certificate to rotate, no listener to firewall.
- `mwl ctl` is a new client binary surface that must version-match its server. Acceptable while the wire
  shape is unstable; it is what makes the instability safe.

## Alternatives rejected

- **Re-read `mwl.toml` on every request**, on the grounds that requests are isolated anyway. Isolation
  forbids shared *mutable* state, which an immutable snapshot is not; the per-request half already exists as
  0005's overlay. Re-reading would add a syscall and a parse to the request path, would turn one malformed
  edit into a fleet-wide per-request failure instead of a refused reload, and would still not make
  `[[extension]]` reloadable, since that needs the cache keys to change rather than the file to be re-read.
- **Lazy, stat-gated revalidation of `mwl.toml`**, mirroring what
  [0017](0017-hot-reload-without-restart.md) does for source. Right for source, wrong for config: source is
  edited constantly and its changes are per-file, while config is edited rarely and every change is global.
  Above all a config error must reach the operator who made it — under lazy revalidation it is a log line
  nobody reads, and under `mwl ctl reload` it is a diagnostic at the terminal where the edit happened.
- **`SIGHUP`.** No Windows equivalent, no payload, and no response — so it cannot report which `Boot` keys
  were ignored, which § 5 argues is the property that makes reload trustworthy.
- **A TCP control listener now**, with a token or mTLS. Deferred; see § 6 and *Revisiting*.
- **A control endpoint on the application listener** instead of a separate transport. Every path
  normalization bug, proxy misconfiguration and routing edge case would become privilege escalation, and a
  reserved path prefix would collide permanently with [0077](0077-compile-time-routing.md)'s compile-time
  route table.
- **Fine-grained invalidation on an extension change.** See § 4 — it needs negative dependency tracking.
- **Restart only**, the status quo. Defensible behind a load balancer with rolling deploys, and the reason it
  is still not enough is in *Context*: one process holds the whole concurrency.

## Revisiting

Reopen for a **network-reachable control surface** when something needs one that `mwl ctl` over the
operator's existing access path cannot serve — a controller reloading a fleet, or a scrape endpoint. The
design that follows from this one, recorded so it is not re-derived: a second listener that is absent unless
configured, never sharing the application listener; per-**effect-class** enablement (`observe` / `operate` /
`lifecycle`) rather than one switch, so a liveness probe cannot be handed `shutdown`; a token read from a
file rather than written inline, refused at boot if absent; TLS required for any non-loopback bind, refused
at boot if absent; and `lifecycle` withheld from a network listener entirely, since every orchestrator that
would want it already has signals or exec.

Reopen the **coarse invalidation** decision if a deployment with a large unit count and frequent extension
changes measures the recompile wave as a real cost — the answer would be a `--warm` pass that precompiles
entry points before the swap, not per-unit dependency tracking.

## Verification

- **M6**, where the registry lives and no server is needed: a `Runtime` directive's value changes across a
  snapshot swap; a request that started before a swap reads the old value to completion while a request
  started after reads the new one; a malformed file leaves the previous snapshot serving and reports the
  offending line; a changed `Boot` key is named in the result and does not take effect.
- **M7**, with the server: the socket is refused when its directory is world-writable; two concurrent
  reloads produce one swap, not two interleaved; a reload during sustained load drops no request; `mwl ctl`
  refuses a version-mismatched server.
- **M9**, with extensions: an artifact compiled under one extension set is not reused under another — the
  regression the *Context* hole would have caused; adding an extension makes its classes resolvable to
  requests arriving after the reload, with no restart; replacing one with a different pin recompiles rather
  than calling the old trampoline; a reload whose pin does not match the file on disk is refused whole,
  leaving the previous set live; removing an extension whose classes are still referenced fails the requests
  that resolve those units, and only those.
