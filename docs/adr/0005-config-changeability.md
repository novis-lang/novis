# ADR 0005 — `mwl.ini` states defaults, not ceilings

- **Status:** Accepted
- **Date:** 2026-08-20
- **Scope:** the directive registry, `ini_set`/`ini_get`, per-request limit enforcement
- **Amends:** [0004](0004-memory-for-simplicity.md) — the enforceable per-request cap is now the *ceiling*
  directive, not the default one

> **In short:** `mwl.ini` states defaults, not ceilings. Every directive carries a changeability
> class: `System` (settable in `mwl.ini` only), `Runtime` (`mwl.ini` gives the default and a request
> may set any value for itself, wider or narrower, up to the `[limits.hard]` ceiling), or
> `RuntimeTighten` (narrowing only — capabilities, plus the directives where PHP behaves that
> way too). A set refused by a ceiling or by a class returns `false` and leaves the value unchanged;
> it is **not** clamped. This document holds the only copy of the directive layout.

## Context

- The project-start decision put every limit and capability in a single `RuntimeTighten` class ("a script
  may narrow a limit but never widen one") — a clean win for priority 1, but it breaks priority 2 for one
  of the most common idioms in the PHP corpus:
  ```php
  ini_set('memory_limit', '1G');   // php.ini says 128M
  set_time_limit(0);
  ```
  In PHP this succeeds — `memory_limit` and `max_execution_time` are `PHP_INI_ALL`, changeable to any
  value; `open_basedir` is PHP's one narrowing-only exception, special-cased precisely because it is the
  exception.
- Under tighten-only, converted code doesn't fail loudly at conversion time — it fails at runtime, in
  production, as an out-of-memory or timeout error whose cause is exactly what the script was trying to fix.
- The root problem: one directive conflated two questions — what a script gets without asking, and what the
  host is willing to lose to one request. Every deployment that hit the resulting choice resolved it by
  raising the global limit for everyone, which is worse for priority 1 than the fix below.

## Decision

**`mwl.ini` defines defaults. A directive is a limit that cannot be exceeded only when it cannot be changed
at runtime at all.** Where a value *can* change at runtime, the request sets whatever it wants — wider or
narrower — and the `mwl.ini` value is the starting point it inherits.

Three changeability classes, recorded per directive in the registry:

| Class | `mwl.ini` | `ini_set` |
|---|---|---|
| `System` | the only place it can be set | fails, returns `false`, `E0602` |
| `Runtime` | the **default** a request starts with | any value, wider or narrower |
| `RuntimeTighten` | the default *and* an upper bound | narrowing only; widening fails with `E0602` |

`Runtime` is the default class for anything changeable. `RuntimeTighten` is a special case that has to be
argued per directive in the registry, not a policy — it exists for the directives where PHP itself behaves
that way (`open_basedir`) and for capability grants, where "the script may drop rights it holds, never add
rights it does not" is the whole point of the mechanism.

Every runtime change is **request-local**: `ini_set` writes into the request's copy-on-write config
overlay, which is discarded when the request ends. `ini_get` reads the effective value, `ini_restore`
returns a directive to its `mwl.ini` value. A widened limit is therefore never observable to another
request, and cannot outlive the one that set it — which is what makes widening a question about *this*
request's share of the host rather than about isolation.

### Ceilings are their own directives

What the host is willing to lose to one request is stated separately, in a `System`-class block using the
same key names:

```ini
[limits]                     ; Runtime — what a request starts with
memory       = 128M
cpu_time     = 5s
wall_time    = 30s
max_tasks    = 64
max_output   = 32M

[limits.hard]                ; System — what one request may raise itself to
memory       = 2G
cpu_time     = 60s
wall_time    = 300s
max_tasks    = 4096
max_output   = 512M
```

The shipped ceilings are deliberately generous: they are sized to stop a runaway, not to shape ordinary
code. A script that raises its own `memory` to 512M for an import is doing something the operator has
already permitted; a script that asks for 8G is refused. `[limits.hard] memory = off` removes the ceiling
entirely, giving literal PHP behaviour on a trusted single-tenant host, and per-app blocks may override a
ceiling downward the same way they override a capability grant, since those blocks already live in the root
config.

**A refused `ini_set` returns `false` and leaves the value unchanged**, with a `W`-class diagnostic naming
the ceiling. It is not clamped to the ceiling: silently running with a different number than the one
requested is harder to diagnose than a false return, and `false` is already what PHP returns for a set it
will not perform.

### What is `System`

Everything read once at boot, and everything whose enforcement is the reason it exists: `extension =` and
its hash pins, `cache.dir`, `opcache.validate`, the per-app blocks, and `[limits.hard]` itself. A directive
being `System` is what makes it a limit; there is no separate notion of a "locked" value.

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
