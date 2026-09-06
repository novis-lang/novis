# `rule:security/arena-is-an-ownership-root` — An isolate's arena is an ownership root, not an address range

- **Status:** Accepted
- **Date:** 2026-08-29
- **Scope:** what an isolate's *arena* is in this runtime, what one costs, what "released wholesale" runs,
  and how a value crosses the boundary in each direction. It does not decide the `spawn script` surface,
  the option names or failure-as-a-value — [0006](0006-isolated-script-execution.md) owns all three — nor
  the walk itself, which is [0023](0023-clone-serialize-and-cross-boundary-copy.md) § 2 and lives in
  `crates/nvs-runtime/src/graph.rs`. Enforcement of a byte cap is `[limits]`' and is goal 3's, not this
  ADR's; where the cap *attaches* is § 3 below.
- **Depends on:** [0006](0006-isolated-script-execution.md), [0023](0023-clone-serialize-and-cross-boundary-copy.md)

> **In short:** an isolate's arena is an **ownership root** — one `Ctx` and the values reachable from it —
> not a region of address space. Allocation keeps going through the process allocator, so entering an
> isolate maps no memory and leaving one unmaps none; what an isolate owns is reachability, and the
> boundary is enforced at the single place a value can move, `rule:classes/graph-copy`'s graph copy. "Released
> wholesale" is therefore one drain of `crate::release`'s worklist over that root, followed by § 2's sweep
> of the objects the refcounts could not free — together they run the native teardown a region free would
> skip — and a **move at refcount 1 across the boundary is a pointer handoff that costs nothing** — sound
> only because both sides allocate from the same place.

## Context

`rule:security/isolate-shares-nothing` fixes the isolate's *behaviour*: it shares immutable compiled
code and its parent's budget and nothing else, values cross by copy or by move at refcount 1, and its arena
is dropped wholesale when it ends. `docs/plan/design.md` § *Per-request isolation* says the same thing for a
request — "its own heap arena with a hard byte cap … at request end the arena is released wholesale" — and
§ *In-process isolated script execution* commits `nvs-host` to **one** `Isolate` type serving both paths.

Neither says what an arena *is*, and the word carries an implementation with it: a region of address space
handed out by bumping a pointer and reclaimed with one `munmap`. That is the shape a reader assumes, it is
the shape most runtimes that use the word mean, and it is not the shape this tree can have. Stage 6 cannot
lower `spawn script` to anything until the question is answered, because every following decision — what a
child's `Ctx` is built from, what cancellation reclaims, whether a crossing may adopt an allocation — is
downstream of it.

What is already on disk decides most of the answer:

- **Values are refcounted, and some of them own the outside world.** `crate::release` frees through one
  iterative worklist. A `Core\Db\Transaction` rolls back in its native drop, an open file closes in its
  own; `rule:concurrency/cancellation-runs-no-user-code` requires exactly that teardown to run even
  when the task is cancelled and no script code may.
- **The allocator is already per-thread, not per-request.** `crates/nvs-runtime/src/alloc.rs` is a bounded
  per-thread cache of small blocks in front of the system heap; its own module doc records that it is the
  half of design.md's per-request-arena decision "that needs no per-request accounting and no `Ctx`", and
  that it moved the userland median from 0.31× PHP to 0.54×.
- **`Ctx` is the per-request object compiled code already loads through**, it is `#[repr(C)]` with offsets
  baked into compiled code, and `Ctx::child` already builds one context from another for a task.
- **The walk exists and has two carriers.** `graph.rs`'s `Live` is the arena-to-arena one, and it already
  decides adoption per node at refcount 1.

## Decision

### 1. An arena is an ownership root

**An isolate's arena is its `Ctx` together with the set of values reachable from that context's roots.**
Nothing about being inside an isolate changes where a byte comes from: allocation goes through the process
allocator — `alloc::Pooled` in an optimized build, the platform heap in a debug one — exactly as it does on
the parent's own frames.

The isolation `rule:security/isolate-shares-nothing` promises is therefore a property of **reachability**, enforced at the one place a
value can move between two contexts. There is no second place: a compiled function reaches heap state
through the arguments it was passed, the statics word in its `Ctx` (§ 4), and nothing else, so a value the
graph copy did not carry across is a value the child has no name for.

Three facts make the region the wrong implementation rather than merely a heavier one:

- **Native teardown must still run.** Freeing a region with one call runs no `Drop`, so an isolate holding
  a transaction would leave it open and an isolate holding a file would leak the descriptor. To avoid that
  a region arena has to walk its live set before it unmaps — and walking the live set releasing each value
  is what refcount release already is, so the region buys a second mechanism for the same work.
- **Values resize.** An array is copy-on-write and `nvs_str_append` grows a string in place at refcount 1;
  both allocate replacements and free their predecessors. A bump region cannot reuse what it freed, so a
  loop appending to one string holds every intermediate and the isolate's footprint becomes O(work done)
  rather than O(live). By `rule:programs/memory-priority` that is a leak, not a trade.
- **The latency a region is reached for is already collected.** The pooled allocator is the fast path a
  bump pointer would be competing with, and it is measured; an isolate-lifetime region would win the same
  allocations twice and lose the frees.

### 2. "Released wholesale" is one drain of the release worklist over that root

When an isolate ends — returning, throwing, breaching a limit, or being cancelled — its context's roots are
released through `crate::release`'s single worklist. That worklist is iterative, so a graph nested a
million deep costs no stack; it is bounded by what is **live** at that moment, not by what the isolate ever
allocated; and it runs every native drop on the way.

A cancelled isolate takes the same path and no other. `crates/nvs-host/src/scheduler.rs`'s forced unwind
runs on the scheduler's own stack, releasing the dead task's context there, which is what makes `rule:security/isolate-shares-nothing`'s
"its arena is dropped; no orphan" a consequence of the machinery Stage 3 already landed rather than a
second teardown to keep correct.

The one thing this costs against a region is honest and worth naming: reclamation is **O(live values)**
rather than O(1). A region's `munmap` does not care how many objects were in it. In exchange the drain is
the only reclamation path in the runtime, so it is the only one that can be wrong.

The drain alone is not the whole of reclamation, because it frees only what the refcounts say is dead and
a cycle's members hold each other above zero. An object is the one shape that can close a cycle — a string
is immutable and an array copies on write (`graph.rs`'s identity decision) — so the drain is followed by a
**sweep**: every object links into its context's intrusive live list when it is allocated and out when it
is dismantled, and what the drain leaves on that list is dismantled through the same worklist, so native
teardown runs there too. What the sweep spends, per `rule:programs/memory-priority`: two pointers
per live object, and a few non-atomic stores at each object's allocation and death. Decided 2026-09-01,
after review found the drain-only teardown retained a cycle for the life of the process.

**What is left on the list is not by itself proof that it is garbage**, and the sweep does not treat it as
such. A `Value` does leave a context — `crate::abi`'s `call` answers one to its Rust caller — so the sweep
first tallies, per member, how many of its references come from another member's field slot; a member
whose count that tally does not exactly account for is reachable from outside, as is everything under it,
and is left alone. A survivor is then taken off the list, because the list dies with the context and the
object does not. Only the remainder is dismantled. That is priority 1 deciding a question priority 5 would have
answered the other way: freeing memory somebody still holds is a use-after-free, and leaving a cycle whose
only closing edge is inside an `array<T>` — the case the tally deliberately does not read — is a leak the
crate's own known gaps name. `crates/nvs-runtime/src/object.rs` is the mechanism's one home.

### 3. What an isolate spends

Per `rule:programs/memory-priority`'s *say what you spend*, for one **in-flight** isolate:

| what | how much |
|---|---|
| its `Ctx` | one context: its own output buffer, its own assertion ledger, its own statics store (§ 4), and the origin, debug flags, error class and deadline word copied from its parent |
| its task stack | 1 MiB of reserved address space, resident only in the pages its code touched, pooled per worker and recycled when the task ends (`rule:concurrency/a-task-stack-is-reserved-wide-and-pooled`) |
| its values | whatever it allocates, charged against the budget accounted **at the root of the request tree** ([ADR 0006](0006-isolated-script-execution.md) § *Budgets*) |
| a crossing, transiently | one hash-map entry per distinct object in the graph being copied, for the length of that one copy (`graph.rs` § *What it spends*) |
| compiled code | nothing: the `Arc<CompiledUnit>` is shared, and 10 000 isolates of one file compile it once |

Every row is O(in-flight isolates) and no row is O(isolates created): a finished isolate's context is
dropped, its stack returns to the worker's pool, and its blocks return to the thread's allocator cache.

The byte cap attaches where `affordable` already says it does — as **arithmetic** against the tree's
budget at the allocation site — rather than as the bound of a mapping. That is the direct consequence of
§ 1 and it is a real difference in behaviour: an isolate that asks for more than its share is refused at
the call that asked, with a limit report naming the directive, and never by a page fault at an address
nobody chose.

### 4. An isolate does not alias its parent's statics — the one place it parts from `Ctx::child`

`Ctx::child` **aliases** the request's static-property base, because a task under
`rule:concurrency/all-answers-a-typed-shape` shares the request and a child with a store of its
own would give one request two copies of every static. `rule:security/isolate-shares-nothing`'s table says the opposite for an isolate:
globals, class statics and runtime-defined constants are *fresh*.

So an isolate's context is built with a **statics base of its own**, and that single word is the whole of
"a child cannot read or write a parent static". Compiled code loads a static inline through it, so the same
compiled function reads a different store depending only on which context it is running under — which is
why sharing code between isolates costs nothing and needs no per-isolate compilation.

The two constructions therefore stay two constructions. A helper that "shares what a task shares and
freshens what an isolate freshens" would be one function with a boolean that means *which ADR am I*, and
the next field added to `Ctx` would have to be classified by whoever adds it rather than by whoever needs
it fresh.

### 5. A value crosses exactly twice, and both crossings are the same walk

- **In, at the spawn:** the `args:` value, through `Live`, from the parent's ownership root into the
  child's.
- **Out, at the `await`:** the child's top-level `return` value, plus its captured output and — on failure
  — its error rendered as data ([ADR 0006](0006-isolated-script-execution.md) § *Failure is a value*),
  through `Live`, back the other way.

Nothing else moves. The parent's output buffer, its `Core\Request`/`Core\Server`/`Core\Session` backing
state (`rule:statements/no-host-populated-variables`) and any open handle are not reachable from the child's root and
are not made reachable by either crossing.

The refusals are the walk's, not the boundary's, which is the point of there being one walk: a closure, an
`inout` binding, an object holding a host handle and an unresolvable class are refused in `graph.rs`, and a
`secret`-typed property is refused there too by `field_is_secret`. The half a run-time walk cannot see is
refused earlier: `nvs_types::expr::quals::reject_secret_boundary_argument` (`E0775`) reads a call's written
arguments, and `spawn`/`spawn worker`/`spawn script` hand it their own argument list rather than growing a
second rule (`rule:security/secret-sinks-refuse`).

**The out crossing runs before the child's root is released**, and § 1 is what makes it cheap: because both
contexts allocate from the same place, `Live`'s move at refcount 1 is a pointer handoff — the allocation
the child uniquely owned is now uniquely owned by the parent, with no copy and no bookkeeping. Under a
region arena that same move would be a copy every time, since the child's bytes are about to be unmapped.
A child returning a large array therefore costs the walk and not the array.

### 6. Across cores, the walk is the same and the move is not available

`on: 'worker'` puts the isolate on another core. The walk does not change — the same `Live` carrier, the
same refusals — but it may **not adopt**: refcounts in this runtime are non-atomic because a value is
reachable from one core only (`docs/plan/design.md` § *Thread-per-core, shared-nothing runtime*), and while
the source arena is still alive on the sending core an allocation reachable from both is a refcount two
threads can touch. The cross-core crossing is therefore a copy at every node, and that cost is a property
of the placement option rather than of the boundary.

How an isolate is *placed* on another core is `crates/nvs-host`'s, over Stage 2's machinery; this ADR
decides only what the crossing may do once it is.

## Consequences

- **The walk is the audited surface.** With no address range to fall back on, a bug in `graph.rs` is the
  only way state can bleed between isolates. That is a concentration, not an exposure — it is one file
  with one test suite instead of an invariant spread across every allocation site — but it is why `rule:classes/graph-copy`'s "one operation, two carriers" is load-bearing here rather than merely tidy.
- **Spawn-to-result stays in microseconds**, which is what `rule:security/isolate-shares-nothing`'s M5 acceptance figure needs: entering
  an isolate is a `Ctx` construction and a pooled stack, with no mapping syscall in it. A region arena
  would put an `mmap`/`munmap` pair on every isolate, which for a trivial child is most of its lifetime.
- **One implementation, as design.md requires.** An inbound HTTP request is the root isolate of its tree
  and is built the same way, so M7's server path inherits § 4's fresh statics base and § 2's teardown
  without a second setup.
- **A limit breach is reported at a call, not at a fault**, per § 3 — which is the shape
  `rule:errors/escalation-ladder`'s ladder already wants, since a page fault has no directive
  to name.
- **A cycle inside an isolate outlives the drain, and § 2's sweep is what reclaims it.** The drain frees
  only what the refcounts say is dead, so without the sweep a cyclic object graph is retained for the life
  of the *process* — in the server, a leak growing with requests served, which is what made the sweep an
  obligation rather than an option. The optional in-flight collector for a long-running CLI script that
  builds cycles *between* teardowns is a separate decision and remains open.

## Alternatives rejected

- **A bump-pointer region per isolate**, reclaimed with one call. The classic answer, and the one the word
  "arena" implies. Rejected on § 1's three facts: it skips native teardown, it cannot reuse freed
  intermediates so its footprint tracks work rather than live data, and it competes for a latency win the
  pooled allocator has already taken.
- **A separate allocator instance per isolate** — per-isolate free lists, keeping the process allocator's
  reuse but making accounting a property of the allocator. Rejected because it costs a pointer in `Ctx`
  and a load on every allocation to buy accounting that § 3 gets from arithmetic at the allocation site,
  and because it fragments the per-thread cache that made the allocator worth having.
- **An arena pointer in `Ctx`.** `Ctx` is `#[repr(C)]` and compiled code loads its fields at baked
  offsets, so a field is an ABI change, not a struct change — the same reasoning `crates/nvs-runtime/src/host.rs`
  records for putting the host in a thread-local. There is nothing for the field to point at anyway once
  § 1 is decided.
- **An OS process per isolate.** Already rejected by `rule:security/isolate-shares-nothing` on cost
  and ambient authority; restated here only because "an address range you cannot reach" is the property a
  process gives for free and § 1 gives up.
- **Segregating isolates by address range and checking pointers at the boundary.** A check cheap enough
  for the request path can only ask which region an address is in, which cannot see that a property is
  `secret` or that a class is unresolvable on the far side. The walk answers all three questions in one
  pass, so the range check would be a second, weaker mechanism.

## Verification

- `cargo test -p nvs-host`, over the boundary this ADR implements: a child cannot read or write a parent
  variable or static (§ 4), a child cannot see the parent's output buffer (§ 5), a closure, reference or
  resource and an unresolvable class are refused at the boundary (§ 5), a cancelled parent leaves no
  orphan and no leaked arena (§ 2).
- The WSL valgrind leg over `examples/isolate.nvs`: § 2's claim is that the drain frees everything an
  isolate held, and a debug build is deliberately on the platform heap so the checker can see it
  (`docs/agent/playbook.md`).
- The WSL valgrind leg over `examples/cycles.nvs`: § 2's sweep claim — a cyclic object graph is reclaimed
  when its context drops, not at process exit. Goal 6 item 31 is the same claim under the server: live
  bytes stay flat across a soak of requests that build cycles.
- `rule:security/isolate-shares-nothing`'s own M5 figure — spawn-to-result for a trivial child on a warm cache, in single-digit
  microseconds, next to the process baseline in `benches/isolation.rs` — is the measurement that holds
  this ADR's central claim against the region it rejected.
