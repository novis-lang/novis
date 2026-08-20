# ADR 0004 — Memory is spent for security, speed and simplicity

- **Status:** Accepted
- **Date:** 2026-08-20
- **Scope:** project-wide; constrains every later design decision rather than one subsystem

> **In short:** memory is the resource MWL spends to buy security, semantics, latency and
> simplicity, in that order. It is *not* licence to leak: memory must stay attributable to a
> request, under an enforceable cap, and O(in-flight) rather than O(requests served). The ordering
> itself is restated in [CLAUDE.md](../../CLAUDE.md); before invoking this ADR to justify a design,
> read **What this does not license**.

## Context

MWL exists to execute two kinds of code securely and fast: web requests inside a long-lived server
process, and command-line programs. Both run on server-class or developer-class machines, where RAM is the
cheapest resource to add and the easiest to size for in advance. Neither workload is an embedded one.

What is actually scarce is everything else:

- **The security surface.** One process serves every request, so a sandbox escape or a cross-request leak
  is not a bug in one request — it is a bug in all of them.
- **Latency on the request path.** A request budget is measured in milliseconds, and everything MWL does
  per call, per value and per allocation is spent out of it.
- **Human attention.** How much of the language a developer must hold in their head to write correct code,
  and how much of the implementation a maintainer must hold to change it safely. Every invariant that is
  enforced by discipline rather than by construction is drawn from this account.

RAM can be bought. An invariant that every future contributor has to remember cannot, and the places where
MWL is most likely to be wrong — the unsafe modules, the codegen call sites, the sandbox boundary — are
exactly the places where a memory-saving trick would have to live.

This needs recording because the project has already made the same trade at least six times, each argued
on a different axis and none of them naming the rule: the 64 KiB stack per task, taken to avoid `async`
colouring; the 16-byte value, taken because PHP needs the full `i64` range; the deep copy across worker
boundaries, taken so refcounts can stay non-atomic; copy-on-write arrays, taken for PHP value semantics;
the request heap dropped wholesale, taken so cycle leaks cannot accumulate; a fresh wasm instance per
request, taken so extension state cannot leak. The same instinct governs a decision that costs CPU rather
than memory — the baseline tier lowering every operation to a runtime-helper call because that is quick to
get correct.

Left unstated, each of those stays individually re-arguable, and the argument is unfair in a predictable
direction: a reviewer looking at one line can always show that the smaller-memory option is cheaper *there*,
because what it costs is diffuse and lands in a different file. Naming the ordering puts the burden of proof
where it belongs.

## Decision

**When a design trades memory footprint against security, semantics, latency or simplicity, MWL pays the
memory.**

The ordering, highest first. A lower item is spent to buy a higher one, never the reverse:

1. **Security and request isolation.** Not traded for anything, including for the four below.
2. **Correctness of language semantics.** PHP-compatible observable behaviour is not negotiable against
   footprint. A representation that is smaller but cannot express what the language must express is not a
   candidate.
3. **Latency and throughput on the request path.**
4. **Simplicity** — of the language surface first, then of the implementation. If two designs are equally
   safe, correct and fast, the one a reader has to think less about wins even if it holds more bytes.
5. **Memory footprint.** Last, and spent deliberately to buy any of the above.

Stated as a non-goal, because it is the part that gets assumed: **MWL is not a low-footprint runtime.** It
does not target `no_std`, microcontrollers, or minimum-RSS deployments, and there will be no cut-down build
that trades semantics for size. "Uses less memory" is not on its own an argument for a change. "Uses less
memory and is no more complicated" is simply a better design, needs no appeal to this ADR, and is always
welcome.

## What this does not license

Memory is a currency, not a landfill. Three bounds, and one review obligation.

**Bounded, not merely modest.** Every per-request allocation stays under a cap the runtime can *enforce* —
the `[limits.hard]` memory ceiling in the root-owned `mwl.ini` for script code, which a request may spend up
to but not past ([0005](0005-config-changeability.md)), and `StoreLimits` for wasm guests. Those
caps exist so that a hostile or buggy request cannot exhaust the host, which makes them priority 1, and
this ADR does not touch them. Spending generously *inside* a cap is the point of the ADR; a design whose
consumption cannot be attributed to a request and capped is not a memory trade-off, it is a hole in
isolation.

**O(in-flight), not O(served).** The cost must be proportional to concurrent work and released wholesale
when that work ends. Growth proportional to the number of requests a process has served is a leak, and no
ordering makes a leak acceptable. Where a cost can be *reserved* rather than *committed* it should be: the
coroutine stack is lazily grown, which is the only reason 64 KiB per task is affordable at the tens of
thousands of in-flight tasks MWL targets (`benches/abi-probe/benches/coroutine.rs`).

**Footprint, not traffic.** Bytes held are cheap; bytes moved are not. An extra cache miss or an allocation
on a hot path is a *latency* cost and is governed by priority 3, not priority 5, however much it looks like
a memory question. The worked example is already in the tree: the pending-error slot is a
`Cow<'static, str>` rather than a `String` because one allocation per throw cost more than the entire
propagation path it was meant to measure ([ADR 0002](0002-error-propagation.md)). That decision reads as
frugality and is not — it is priority 3 beating priority 5, which is the ordering working, not an exception
to it.

**Say what you spend.** A decision that takes memory states the amount, per request or per task, in the
document that records it — the way the coroutine paragraph states 64 KiB. The ordering is only usable by a
future reader if the costs are written down as they are incurred; otherwise "a bit more memory" six times
becomes a number nobody chose. This is the same reflex as *architecture assumptions are tested, not
remembered* ([the ADR index](README.md)), applied to cost instead of behaviour.

## Where this already applies

Every row is a decision MWL has taken, not an aspiration. This ADR names what they have in common.

| decision | memory paid | what it buys |
|---|---|---|
| Stackful coroutines ([index](README.md)) | 64 KiB reserved per in-flight task, committed lazily | No `async` colouring: any function may do I/O, so converted PHP call chains become concurrent without rewriting |
| 16-byte tagged values (`benches/abi-probe/src/lib.rs`) | 8 bytes per value over a NaN-boxed 64-bit representation | The full `i64` range, which PHP integer semantics require |
| Copy-on-write arrays and strings | A refcount per value, and a full clone on the first write to a shared one | PHP value semantics, with no aliasing rules for the developer to learn |
| Isolated workers for CPU parallelism ([index](README.md)) | A deep copy of every value crossing a core boundary, moved only when the refcount is 1 | Non-atomic refcounts, and data races impossible by construction rather than by discipline |
| Shared-nothing requests, heap dropped wholesale ([index](README.md)) | Peak rather than average retention inside a request | Cycle leaks cannot accumulate in a long-lived server, and no collector runs on the request path |
| A fresh wasm instance per request ([ADR 0003](0003-extension-system.md)) | A linear memory per extension a request actually calls | Extension state cannot leak between requests — a guarantee PHP does not offer |

## Consequences

**Positive**

- Six decisions defended separately become one decision defended once. A reviewer who wants the
  lower-memory option is now arguing against this ADR, which is the right place for that argument.
- PHP-compatible semantics stay reachable. Most of what makes them expensive is memory — 16-byte values,
  copy-on-write, per-request heaps. A footprint-first project would have to compromise on observable
  behaviour to get them; MWL does not have to.
- Fewer remembered invariants, therefore a smaller unsafe surface. The tricks this ADR declines are
  precisely the ones that would need auditing.
- The trade-off is visible to operators as a sizing question rather than as unexplained resident memory.

**Negative**

- Resident memory per process is higher than a footprint-tuned runtime's, and it scales with in-flight
  *concurrency* rather than with request rate. Operators must size for concurrency — tasks × stack, plus
  concurrent requests × their cap — so deployment documentation has to state that shape rather than quote a
  single typical RSS.
- MWL will lose hello-world-RSS comparisons against interpreters. It should not contest them; the honest
  comparison is throughput and latency per core at a given concurrency, with isolation intact.
- The rule is quotable as an excuse. "ADR 0004 says memory is cheap" is not an argument for an unbounded
  cache, a leak, or an uncapped buffer. The three bounds above are the answer, and a review should ask for
  the number.

## Revisiting

Reopen this if MWL takes on a deployment target where memory is the billed or hard-limited unit: per-tenant
containers sized in tens of MiB, serverless instances with a fixed sub-128 MiB budget, or embedded use. Then
footprint moves from priority 5 to a product requirement, and several rows of the table come back into
question — most obviously the 64 KiB stack, which is what bounds in-flight concurrency per GiB. That would
be a change of direction, not a tuning exercise, and it should be made deliberately.

Unlike [0002](0002-error-propagation.md) and [0003](0003-extension-system.md), nothing here can be checked
by a test: this is a statement of what MWL is for, and it changes only when that changes. The second bound
*can* be checked, and should be — a soak test asserting that a server's resident memory returns to its
baseline after a burst of traffic distinguishes memory spent from memory leaked, and belongs in the
built-in HTTP server milestone for that reason.
