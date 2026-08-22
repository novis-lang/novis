# ADR 0006 — Running another script is an in-process isolate, not a subprocess

- **Status:** Accepted
- **Date:** 2026-08-20
- **Scope:** the `spawn script` construct, `mwl-host`'s isolation boundary, the value-crossing rules, the
  `script.spawn` capability, per-tree limit accounting
- **Validated by:** `benches/abi-probe/src/process.rs` + `tests/perf_guards.rs`
  (`an_os_process_costs_orders_of_magnitude_more_than_a_task`)
- **Relates to:** [0002](0002-error-propagation.md) (nothing unwinds across the boundary either),
  [0003](0003-extension-system.md) (this is not a sandbox for foreign code),
  [0004](0004-memory-for-simplicity.md) (what an isolate spends),
  [0005](0005-config-changeability.md) (how a child's config is derived),
  [0008](0008-static-and-global.md) (what "fresh globals and statics" is a list of, and why it is a short
  one — there is no function-scope `static` to reset)
- **Amended by:** [0012](0012-no-superglobals.md) — `$_ARGS` becomes `Core\Script::args()`, and a spawned
  isolate calling `Core\Request`/`Core\Server`/`Core\Session` throws rather than seeing fresh-and-empty
  state. [0021](0021-single-file-inclusion-construct.md) — every mention below of PHP's same-frame
  inclusion keywords is updated to name `require` alone, since `include`/`include_once`/`require_once` are
  rejected there; the isolation boundary this ADR defines is unchanged. [0023](0023-clone-serialize-and-cross-boundary-copy.md) —
  § *Values cross by copy* below is now that ADR's formal graph-copy definition, applied to this boundary;
  the rules for what crosses and what refuses are unchanged, and `serialize()`/`unserialize()` gain the
  identical operation as a second, externalized-to-bytes caller.

> **In short:** `spawn script 'file.mwl'` runs another file in-process as a child isolate —
> fresh arena, fresh globals and statics, its own config overlay, sharing nothing but immutable
> compiled code. Three invariants no optimisation may trade away: values cross by the *same*
> deep-copy-or-move rules as cross-core worker dispatch (never a pointer, never a shared heap);
> every limit is accounted at the **root of the request tree**, never per isolate; and a child's
> grants are its parent's, optionally narrowed. An inbound request *is* the root isolate of its own
> tree, so the server path and the `spawn script` path are one implementation — a second arena
> setup or teardown path is a bug, not an optimisation.

## Context

- A script often needs to run another script without being affected by it (a queue worker running a job, a
  plugin, a CLI tool running user code) — the callee must not see or clobber the caller's
  globals/statics/handles/output, a fatal or infinite loop in it must not take the caller down, and it must
  reach nothing the caller could not.
- PHP's only real answer is spawning another `php` process over pipes — every in-process construct
  (`include`/`require`, `eval`, `Fiber`/generator) shares the same symbol table, heap and statics, isolating
  nothing.
- Measured on this machine: a bare MWL task costs 4.29 µs; PHP 8.5.8 booting and exiting costs 35.9 ms —
  **roughly 8000× a task**, before the child has even parsed a line. (`CreateProcess` is dearer than
  `fork`+`exec`, so Linux would be smaller, but not by three orders of magnitude.)
- A child process is also worse at the isolation it's used for: it inherits ambient authority (env, cwd,
  handles, OS-user rights — the same failure mode [ADR 0003](0003-extension-system.md) rejected `dlopen`
  for), cannot be governed (the parent can only kill it — no CPU/memory accounting, no cooperative
  cancellation), and re-enters through the front door (arguments serialised onto a command line).
- MWL already has the machinery for this, built for requests: a per-request arena with a hard cap, fresh
  request/session state ([ADR 0012](0012-no-superglobals.md)), a copy-on-write config overlay, a coroutine
  tree, safepoint-driven limits, a process-wide compiled-unit cache. This ADR exposes that machinery to
  script authors instead of keeping it server-only.

## Decision

**MWL gets a first-class construct for executing another `.mwl` file as an isolated unit of work inside the
same process — an *isolate*. It shares nothing with its parent except immutable compiled code and its
parent's resource budget.**

The provisional surface, spelled out here so the semantics have something to hang on. The *semantics* below
are what this ADR fixes; the exact spelling is pinned down in `docs/spec/` during M5, and is the part to
bikeshed then:

```php
$job = spawn script 'jobs/report.mwl' with(
    args:   ['month' => 7],                      // deep-copied in
    limits: ['memory' => '256M', 'cpu_time' => '10s'],
    grants: ['fs.read' => '/srv/www/data'],      // narrowing only
    output: 'capture',                           // or 'inherit'
    on:     'worker',                            // or the parent's core, the default
);

$result = await $job;                            // a ScriptResult, never a throw from the child
if (!$result->ok) { log($result->error->message); }
```

`with(…)` is not new syntax: it reuses PHP's existing named-argument call grammar verbatim, the same as any
other named argument in this language — the *pinned* grammar is [`docs/spec/00-overview.md`](../spec/00-overview.md)'s
job, not this ADR's. The array literals inside it (`['month' => 7]`) are ordinary array syntax; the two look
different because a fixed set of known option names and a dynamic map are different things, not because a
third syntax was invented for either.

and the callee is an ordinary script, receiving its arguments through `Core\Script::args()` and answering
with a top-level `return` — which is what `require` already means in PHP, so nothing new has to be learned
beyond the one accessor call ([ADR 0012](0012-no-superglobals.md) fixes that it is a method, not a magic
variable):

```php
<?mwl
use Core\Script;

mixed $month = Script::args()['month'];
return ['rows' => Reports::build($month)];
```

`spawn script` joins `spawn` (a task on this core, same heap) and `spawn worker` (a task on another core,
values deep-copied) rather than introducing a parallel concurrency vocabulary: it is awaited like them, it
dies with its parent like them, and `on: 'worker'` composes the two axes instead of multiplying the syntax.

### What is and is not shared

| | shared with the parent |
|---|---|
| Compiled code (`Arc<CompiledUnit>`) | **yes** — immutable, process-wide, content-addressed. 10 000 isolates of one file compile it once |
| The resource budget | **yes**, deliberately — see *Budgets* below |
| Heap arena, refcounts, values | no. Its own arena, dropped wholesale when it ends |
| Globals, class statics, constants defined at runtime | no. Fresh |
| Request/server/session state (`Core\Request`, `Core\Server`, `Core\Session`, [ADR 0012](0012-no-superglobals.md)) | no — **throws** inside the child rather than returning the parent's data or a fresh-and-empty result; `Core\Script::args()` is what the child gets instead |
| Output buffer | no. Captured separately |
| Open resources — files, sockets, DB connections | no, and they cannot be passed |
| Config overlay | derived, never shared: a copy of the parent's *effective* config, which the spawn may narrow |
| Include graph, autoloader state | no. It resolves its own |
| The coroutine tree | linked, not shared: the isolate is a child task, so it dies with its parent |

### Values cross by copy, using the rules that already exist

Arguments in and the result out are **deep-copied, or moved when the refcount is 1** — the identical
mechanism and identical restrictions as `spawn worker` ([the ADR index](README.md)). This is not a new
marshalling design, and that is the main reason to state it: two boundaries with different value rules
would be two sets of rules for developers to learn and two implementations to keep correct.
[ADR 0023](0023-clone-serialize-and-cross-boundary-copy.md) now gives this operation its one formal
definition — a recursive, cycle-safe graph copy — and gives it a second caller: `serialize()`/`unserialize()`
run the identical walk, externalized to bytes instead of moved directly between two live arenas. The three
bullets below are that ADR's rules, restated here because this is the boundary a reader lands on first:

- The copy is a **graph** copy, not a tree copy: shared substructure stays shared and cycles terminate, so
  `$a['self'] = $a` crosses instead of hanging.
- **Closures, references (`&$x`) and resources cannot cross.** A closure captures a heap and a scope, a
  reference is an alias, and a resource is a host handle; none of the three has a meaning in another heap.
  **The idiom for a script that needs a live resource inside the child is to pass what identifies it, not the
  handle itself** — a DSN, a path, a credential reference — as an ordinary `args:` value, and have the child
  open its own resource from it. This is not a workaround; it is the isolation model's actual point (no
  ambient authority, no shared handle), the same reason a spawned isolate cannot see the parent's
  `Core\Request` either.
- An object whose class the receiving side cannot resolve is **refused with a diagnostic naming the class**,
  not degraded into a stub. PHP's `__PHP_Incomplete_Class` is what silent degradation looks like, and it
  fails later and further from the cause.

### Budgets are accounted at the root of the request tree

This is the part that keeps priority 1 intact, and it is the decision most easily got wrong in the other
direction.

**A request and everything it spawns form one tree, and limits are accounted against the tree's root.** An
isolate does not receive a budget of its own *in addition* to its parent's; it spends the parent's. So:

- The worst case of a process is still `in-flight requests × [limits.hard] memory`
  ([0005](0005-config-changeability.md)), **unchanged by isolates**. A request that spawns 50 of them does
  not get 50× the ceiling; it gets one ceiling to divide.
- `limits:` at the spawn site sets a **sub-cap**: tighter than what remains, never wider. Inside the child,
  `ini_set` behaves exactly as [0005](0005-config-changeability.md) says — free movement up to the
  `[limits.hard]` ceiling — and the tree's remaining budget is what actually bounds it. A widened limit in
  a child dies with the child, like any other request-local set.
- Child tasks count against the root's `max_tasks`, child output against the root's `max_output`, child CPU
  against the root's `cpu_time`. A fork bomb is therefore bounded by arithmetic rather than by a heuristic,
  and one new directive — `[limits] max_script_depth`, default 8 — bounds nesting so that a runaway
  recursion is diagnosed as such instead of as a memory limit.

What an isolate *spends*, as [0004](0004-memory-for-simplicity.md) requires to be stated: **one 64 KiB
reserved task stack (committed lazily) plus its own arena**, held to its peak until it ends, drawn from the
root's budget. Concurrent isolates each hold their own arena, so peak retention in a tree is the sum over
live isolates — under the root's cap, by the paragraph above. Nothing is retained after an isolate ends: the
arena is released wholesale, which is the same property that makes cycle leaks structurally
non-accumulating for requests. A long-running CLI script gains a way to reclaim a leaky job's memory
wholesale that it did not have before.

### Executing code is its own capability

Spawning an isolate requires a new deny-by-default grant naming the roots that code may be executed from:

```ini
[capabilities]
script.spawn = /srv/www/jobs:/srv/www/app/tasks      # RuntimeTighten
```

Being able to *read* a file is not permission to *run* it, so `fs.read` does not imply `script.spawn`; the
two grants answer different questions and a template directory that is readable by design should not become
an execution root by accident. The path is canonicalised and then prefix-checked against the granted roots,
which closes traversal by construction rather than by validation. A dynamic path is allowed — this has to
work for a queue worker — but it can only ever land inside a root the operator wrote down.

The child's grants are the parent's effective grants, optionally narrowed at the spawn site. Nothing widens:
capability grants are `RuntimeTighten` already ([0005](0005-config-changeability.md)), and an isolate is a
new place that rule applies, not an exception to it. A parent that has dropped `net.out` cannot regain it by
spawning.

### Failure is a value, not an exception

A child's failure arrives as **data on the result**, and never as an exception unwinding into the parent:

| in the child | at the parent |
|---|---|
| top-level `return $v` | `ok = true`, `value = ` a copy of `$v` |
| uncaught throw | `ok = false`, `error` carrying the class name, message, code and rendered trace — **as copied data, not the exception object** |
| limit breach (memory, CPU, wall, depth) | `ok = false`, `error->limit` naming the directive |
| a contained runtime panic (`FATAL`, [0002](0002-error-propagation.md)) | `ok = false`, and the parent keeps running — a strict improvement over PHP, where a fatal in the child is at least a lost process and in-process is a lost everything |
| parent cancelled or timed out | the child is cancelled at its next safepoint; its arena is dropped; no orphan |

Rethrowing the child's exception object into the parent was rejected: it would have to copy an arbitrary
object graph across the boundary — which the rules above forbid for good reasons — and it would encourage
reading `spawn script` as a function call, which is exactly what it is not. Code that wants the terse form
asks for it explicitly (`$result->valueOrThrow()`), and the diagnostic then names the isolate, not the
parent frame it never had.

Nothing unwinds across the boundary, for the same reason nothing unwinds across a JIT frame
([0002](0002-error-propagation.md)): the status is checked, and the child's arena teardown is explicit code
on the failure path rather than an implicit landing pad.

### Output is captured by default

`output: 'capture'` (the default) puts the child's output on the result, charged to the tree's `max_output`.
`output: 'inherit'` appends it to the parent's output stream **when the result is awaited**, which keeps
ordering deterministic under concurrency — a genuinely interleaved write into a live HTTP response needs an
ordering model, and that question is deferred to M7 rather than answered badly here. Capture is the default
because the alternative silently mixes another script's bytes into a response the parent is responsible for.

### One isolation implementation, not two

`mwl-host` grows a single `Isolate` type, and **an inbound HTTP request becomes the root isolate of a
request tree**. The server path and the `spawn script` path are then the same code: one arena setup, one
construction of the `Core` accessor classes' backing state, one config-overlay derivation, one teardown, one
place where a limit is enforced.
That is worth more than it sounds — it means the cross-request state-bleed suite in M7 is simultaneously the
state-bleed suite for isolates, and that a fix on either path cannot forget the other.

## Consequences

**Positive**

- The requirement is met at three to four orders of magnitude below PHP's cost, and the shared compiled-unit
  cache means the second isolate of a script pays no compilation at all.
- Isolation *improves* relative to the subprocess it replaces: no ambient authority, capability grants still
  enforced, CPU and memory attributed to the causing request, cooperative cancellation, structured
  concurrency instead of orphan reaping.
- No new isolation machinery, no new value-marshalling rules, no new concurrency vocabulary. Three existing
  mechanisms get a second caller each.
- `mwl convert` gains a real target for two PHP patterns it would otherwise have to give up on:
  `exec('php …')`-style job dispatch, and `eval` of a file's contents.
- Long-running CLI programs get bounded memory for unbounded work — run the job in an isolate, get the arena
  back.

**Negative**

- **A new language construct**, which is a cost against priority 4 (simplicity of the language surface
  first). Accepted because the alternative — telling authors to shell out — is not simpler, merely absent
  from our documentation, and because `spawn script` is one keyword on an existing verb rather than a new
  subsystem.
- **The boundary rules must be learned**: what cannot cross, and that a child's throw is not the parent's.
  Mitigated by their being *the same* rules as `spawn worker`, and by refusals being diagnostics at the
  boundary rather than surprises later.
- **A confusion risk with `require`.** Two constructs run a file and they isolate opposite amounts. The
  spec must define them next to each other, and the diagnostic for "undefined variable that the parent had"
  should name the isolate boundary as the reason.
- **The isolate is not a sandbox for hostile code beyond what the request boundary already provides.** It is
  as strong as MWL's request boundary — memory-safe by construction, capability-checked, resource-capped —
  and no stronger. Running *foreign* code that must be assumed adversarial at the memory-safety level is
  what Tier 1 wasm is for ([0003](0003-extension-system.md)), and this ADR does not compete with it.
- Cross-core isolates (`on: 'worker'`) pay the deep copy twice, in and out, so large results argue for
  keeping the isolate on the parent's core.

## Alternatives rejected

- **Spawn an `mwl` subprocess** — PHP's answer; inherits the whole cost/ambient-authority objection above.
  Remains possible behind the `process.exec` capability for cases that genuinely want a separate OS
  process; not the answer to this requirement.
- **`eval` of a source string.** Rejected already for the pragmatic superset — a file has a stable identity
  (cache key, source map, `mwl check`-ability, an auditable granted path); a string has none of these.
- **`require` with a fresh symbol table** — isolation by scope only. Shares the heap, so it shares
  refcounts, statics, output, resources and a fatal error; satisfies the letter of "runs another file" and
  none of the requirement.
- **A thread with a shared heap.** Invalidates the non-atomic-refcount premise of the thread-per-core model
  to save one copy at a boundary.
- **Run the child inside a wasm sandbox** ([0003](0003-extension-system.md)). Costs a second compilation of
  code already compiled natively, loses the shared unit cache, and buys a memory-safety guarantee MWL code
  already has by construction.
- **Isolation by HTTP loopback to our own server.** Works, but needs a listener, an authentication story
  and a serialisation format to do what an arena boundary does in microseconds.
- **An independent budget per isolate** rather than a share of the root's. A hole in isolation, not a
  trade-off: memory would stop being attributable to a request, forbidden by
  [0004](0004-memory-for-simplicity.md)'s *bounded, not merely modest* clause.

## Revisiting

Reopen if a use case needs an isolate to **outlive its parent** — a fire-and-forget background job that
survives the response. That is a different feature with different lifetime and accounting rules (whose
budget does it spend after its parent is gone?), and it should not be smuggled in by relaxing the tree
accounting here. The same applies to *persistent* isolates reused across requests: attractive for warm
state, and it directly contradicts strict shared-nothing, so it needs its own argument in its own ADR.

Verification, in the order it becomes possible:

- **Committed now.** `an_os_process_costs_orders_of_magnitude_more_than_a_task` guards the premise: if the
  gap between an OS process and a task ever closes, this ADR's cost argument is gone and should be re-read.
  `benches/isolation.rs` tracks both numbers.
- **M5**, when the construct lands: the isolate's own end-to-end spawn-to-result figure for a trivial child
  on a warm cache goes into `benches/isolation.rs` next to the baseline it beats, with a guard test — the
  target is single-digit microseconds, and anything at millisecond scale means the arena or the globals are
  being built the expensive way. Plus: a child cannot see a parent variable, global or static, and a
  `Core\Request`/`Core\Server`/`Core\Session` call inside it throws rather than seeing the parent's request
  ([ADR 0012](0012-no-superglobals.md)); a closure, reference or resource is refused at the boundary; a
  cyclic argument crosses; a child's uncaught
  throw and a child's contained panic both leave the parent running; a cancelled parent leaves no orphan.
- **M6**, when limits and capabilities land: spawning without `script.spawn` fails; a path outside the
  granted roots fails, including via `..`; a child cannot widen a capability the parent narrowed; N isolates
  cannot together exceed the root's memory or CPU budget; `max_script_depth` is what a runaway recursion
  reports.
- **M7**: the cross-request state-bleed suite runs against isolates as well as requests, which the shared
  `Isolate` implementation makes automatic rather than additional.
