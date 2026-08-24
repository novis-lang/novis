# ADR 0005 — `mwl.toml` states defaults, not ceilings

- **Status:** Accepted
- **Date:** 2026-08-20
- **Scope:** the directive registry, `Core\Config::set`/`::get`, per-request limit enforcement
- **Amends:** [0004](0004-memory-for-simplicity.md) — the enforceable per-request cap is now the *ceiling*
  directive, not the default one
- **Amended by:** 0064, 0072, 0073, 0074, 0076, 0078 — each fold is applied below; this body states the
  current rule. The registry's full block list, with the ADR that argues each block's directives, is
  [0064 § 2a](0064-configuration-file-format.md).

> **In short:** `mwl.toml` states defaults, not ceilings. Every directive carries a changeability
> class: `System` (settable in `mwl.toml` only), `Runtime` (`mwl.toml` gives the default and a request
> may set any value for itself, wider or narrower, up to the `[limits.hard]` ceiling), or
> `RuntimeTighten` (narrowing only — capabilities, plus the directives where PHP behaves that
> way too). A set refused by a ceiling or by a class returns `false` and leaves the value unchanged;
> it is **not** clamped. This document holds the only copy of the directive layout; the file's format is
> [0064](0064-configuration-file-format.md)'s.

## Context

- The project-start decision put every limit and capability in a single `RuntimeTighten` class ("narrow
  only, never widen") — clean for priority 1, but it breaks priority 2 for one of the most common idioms in
  the PHP corpus:
  ```php
  ini_set('memory_limit', '1G');   // php.ini says 128M
  set_time_limit(0);
  ```
  PHP allows this (`memory_limit`/`max_execution_time` are `PHP_INI_ALL`); `open_basedir` is PHP's one
  narrowing-only exception, special-cased precisely as the exception.
- Under tighten-only, converted code fails at runtime instead of at conversion time — as an out-of-memory or
  timeout error, exactly the thing the script was trying to prevent.
- Root problem: one directive conflated two questions — what a script gets by default, and what the host
  will tolerate from one request. Deployments hitting this resolved it by raising the global limit for
  everyone, worse for priority 1 than the fix below.

## Decision

**`mwl.toml` defines defaults. A directive is a limit that cannot be exceeded only when it cannot be changed
at runtime at all.** Where a value *can* change at runtime, the request sets whatever it wants — wider or
narrower — and the `mwl.toml` value is the starting point it inherits.

Three changeability classes, recorded per directive in the registry:

| Class | `mwl.toml` | `Core\Config::set` |
|---|---|---|
| `System` | the only place it can be set | fails, returns `false`, `E0602` |
| `Runtime` | the **default** a request starts with | any value, wider or narrower |
| `RuntimeTighten` | the default *and* an upper bound | narrowing only; widening fails with `E0602` |

`Runtime` is the default class for anything changeable. `RuntimeTighten` is a special case that has to be
argued per directive in the registry, not a policy — it exists for the directives where PHP itself behaves
that way (`open_basedir`) and for capability grants, where "the script may drop rights it holds, never add
rights it does not" is the whole point of the mechanism.

Every runtime change is **request-local**: `Core\Config::set` writes into the request's copy-on-write config
overlay, which is discarded when the request ends. `Core\Config::get` reads the effective value,
`Core\Config::restore` returns a directive to its `mwl.toml` value
([0064 § 5](0064-configuration-file-format.md) holds the signatures, and why the API is string-in/string-out
even though the file is typed). A widened limit is therefore never observable to another
request, and cannot outlive the one that set it — which is what makes widening a question about *this*
request's share of the host rather than about isolation.

### Ceilings are their own directives

What the host is willing to lose to one request is stated separately, in a `System`-class block using the
same key names:

```toml
[limits]                     # Runtime — what a request starts with
memory     = "128M"
cpu_time   = "5s"
wall_time  = "30s"
max_tasks  = 64
max_output = "32M"

[limits.hard]                # System — what one request may raise itself to
memory     = "2G"
cpu_time   = "60s"
wall_time  = "300s"
max_tasks  = 4096
max_output = "512M"
```

The shipped ceilings are deliberately generous: they are sized to stop a runaway, not to shape ordinary
code. A script that raises its own `memory` to 512M for an import is doing something the operator has
already permitted; a script that asks for 8G is refused. `[limits.hard] memory = false` removes the ceiling
entirely, giving literal PHP behaviour on a trusted single-tenant host, and per-app blocks may override a
ceiling downward the same way they override a capability grant, since those blocks already live in the root
config.

**A refused `Core\Config::set` returns `false` and leaves the value unchanged**, with a `W`-class diagnostic naming
the ceiling. It is not clamped to the ceiling: silently running with a different number than the one
requested is harder to diagnose than a false return, and `false` is already what PHP returns for a set it
will not perform.

### What is `System`

**A directive is `System` when changing it from inside a request would affect something other than that
request.** That is the whole rule, and it covers the `[[extension]]` entries and their hash pins,
`cache.dir`, `opcache.validate`, the per-app blocks, `[limits.hard]` itself, every `[[schedule]]` key
([0073](0073-scheduled-work-is-config.md) § 4), `[deferred] max_concurrent`
([0072](0072-core-task-structured-concurrency.md) § 7) and both observability blocks
([0076](0076-observability-export.md) § 6). A directive being `System` is what makes it a limit; there is no
separate notion of a "locked" value. A response header is the counter-example and is ordinary `Runtime`: a
request may set any of [0074](0074-http-defaults-safe-and-finite.md)'s policy directives for itself, because
it could already write the header directly and the change dies with the request.

This class answers one question only — *who may set a directive*. What applying a change **requires**, a new
snapshot or a restart, is a second field on the same registry entry, orthogonal to this one, and is
[0078](0078-config-reload-and-control-socket.md) § 2's. `System` is therefore no longer a synonym for "read
once at boot": most `System` directives reload.

## Consequences

The cost, stated as [0004](0004-memory-for-simplicity.md) requires: the enforceable per-request memory cap
becomes the ceiling, so a process's worst case is `in-flight requests × [limits.hard] memory` — 2 GiB per
request in the shipped configuration rather than 128 MiB. Sizing a deployment means sizing against the
ceiling, and the sentence in 0004 that names the `memory` directive as the enforceable cap should be read
as naming the ceiling. Operators who cannot afford that worst case lower one number in one root-owned file.
This is memory spent to buy priority 2, which is the ordering working as written.

Priority 1 is untouched, and the reason is that widening never crosses a boundary. No runtime set can grant
a capability, load an extension, reach another request's heap, read outside the granted paths, or exceed a
`System` value. What a request can now do is move its own resource budget within a range the operator fixed
at boot — the same authority it always had to *lower* that budget, in the other direction.

Alternatives rejected:

- **Tighten-only everywhere** — the previous decision. Breaks the single most common `ini_set` in the
  corpus, at runtime, in the direction of failure.
- **No ceilings at all** — literal PHP. PHP survives it because the isolation boundary is an OS process per
  request; MWL's is an arena inside a process serving hundreds of requests, so one script choosing `-1`
  would be every co-resident request's outage. The ceiling is what makes the freedom affordable.
- **Clamping to the ceiling instead of refusing** — a silent divergence from the requested value, and from
  PHP.
- **A separate "hardened mode"** that restores tighten-only globally. Two configuration semantics to
  implement, test and reason about, for something one number per directive already expresses.

## Revisiting

The registry is the artifact to argue with: if a directive's class is wrong, that is a one-line change with
a stated reason, not a redesign. Reopen the *decision* if MWL takes on genuinely multi-tenant hosting where
mutually hostile applications share a process — there the per-app ceiling override becomes the primary
control rather than a refinement, and it needs to be mandatory rather than optional.

Testable, unlike [0004](0004-memory-for-simplicity.md), and tested in M6: raising a `Runtime` limit above
its default succeeds and takes effect; raising it above its ceiling returns `false` and leaves the previous
value in place; a `System` set always fails; and a widened limit is not visible to the next request on the
same core.
