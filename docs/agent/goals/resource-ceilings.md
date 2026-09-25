---
milestone: M6
---
# Loop goal 40 — Every resource ceiling stops the request that breaks it

`[limits]` bounds the request that breaks it, whatever that request was doing — burning a core in a
loop that allocates nothing, growing a string through primitives that call nothing, or asking for
more than its whole budget in one operation. None of those is true today. Every ceiling in
`rule:errors/on-limit` is enforced at a poll that a runaway request never reaches, or by a timer that
does not exist.

It sits here because these are M6's own acceptance line — *memory/CPU caps terminate runaway
scripts as a `FATAL`* — and that line has never held for a program that calls no member. This is
priority 1 against the editor goals' priority 4: one of the two holes ends in a process abort and the
other retires a core for the life of the process, and both are request isolation rather than
fairness.

Goal `editor-surfaces`'s whole acceptance list is this goal's floor, and it is never traded.

## Why here

The chain's first correctness item since goal `carried-gaps`, and it sits ahead of the dossier because a proof
written over an unenforced ceiling proves the wrong thing. Both are priority 1 against the editor
goals' priority 4 — request isolation rather than fairness — which is why the goal is placed after
them and not behind the dossier. One keystone serves both and is already measured: a poll word a
non-owning thread can write, which is what a CPU sampler and the allocator each need to publish
into.

## What an attacker does today, in one line each

Written down because the stages below are ordered by these and not by how hard they are:

1. **`while (true) {}`** — no allocation, no output, no member call. Nothing stops it. The safepoint
   fires on every back edge, reads a word nothing ever sets, and continues. The watchdog reads the
   reactor's timer wheel, and a bare loop arms no timer; even when it does fire,
   `rule:http-server/a-wedged-core-is-shed-never-killed` reports and sheds rather than killing, so
   the core is subtracted from `max_in_flight` and gone for the life of the process. **N such
   requests retire N cores.** Stage 3.
2. **`while (true) { $a .= $a; }`** — reaches no poll that looks at memory, ends at
   `handle_alloc_error`, which aborts the worker and every other request on it. Stages 4 and 5.
3. **One operation past the ceiling** — detection at the next poll is detection after the
   allocation. Stage 5.
4. **Fill `Core\Cache::local` in one request, evict it in the next** — the freed bytes land on the
   second request's thread balance and buy it headroom it never earned. Stage 6.

## Stage 0 — the catch-up

Nothing on disk contradicts this goal's rules; the ladder is correct *once something notices*. What
is owed is the cases that say nothing notices, and all three run red the day they are written:

1. **A loop that allocates nothing is stopped**, under `[limits] cpu_time`, expecting the `FATAL` and
   `Core\Fatal::onLimit`'s report naming `cpu_time`.
2. **A loop that calls nothing is stopped**, under `[limits] memory` — `$a .= $a;` and an `int` step.
   It must assert that nothing printed *before* the breach, which is what says the loop was stopped
   rather than the `echo` after it.
3. **One operation past the ceiling is refused before it allocates**, expecting the `FATAL` and a
   peak that never held the request's whole ask.

`tests/conformance/error/a-limit-fatal-is-not-catchable.nvst` is the shape to copy for the handler
half of all three. `docs/agent/playbook.md`'s bullet *A memory-limit breach is observed at the first
`run_helper` member call after it* is case 2 written as a trap; its `[until:]` trailer names the
comment stage 4 rewrites, so the wrap retires it then. Do not reword it before that.

## Stage 1 — the floor

Goal `editor-surfaces`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded for anything above it.

## Stage 2 — the keystone: the poll word moves out of `Ctx`

Every stage after this one needs a word a thread that does not own the request can write. Nothing can
publish into the word compiled code polls while it lives inside `Ctx`: the writer would alias the
`&mut Ctx` a helper body holds. So the word moves out of line and `Ctx` keeps its address in the hot
slot compiled code already loads — which is what `Self::deadline` already does, for the same reason,
and is the model for every decision here.

1. **`crates/nvs-runtime/src/ctx/mod.rs:@Ctx`** — `safepoint` becomes the *address* of an
   `AtomicU64`, the word owned by a handle beside the cold fields. `SAFEPOINT_OFFSET` and the
   arrangement `ctx::tests::the_hot_words_come_first_and_are_a_word_apart` pins are unchanged: a
   pointer is the same eight bytes the flags word was.
2. **One word per request *tree*.** `rule:security/isolate-shares-nothing` gives a tree one ceiling
   to divide, and a child built after a flag was raised must not be born clean. `Ctx::child` and
   `Ctx::isolate` clone the handle.
3. **`crates/nvs-runtime/src/ctx/safepoint.rs`** — the accessors read and write through the handle.
   Only compiled code follows the raw pointer.
4. **`crates/nvs-codegen/src/emit.rs:@emit_safepoint`** — the poll follows the pointer, and the
   handle is loaded **once in the ABI entry block** beside `ctx_p` rather than once per poll. A value
   defined there dominates every block below it, so a back edge keeps the single load it had.

**Measured, not estimated.** Built and benchmarked before this goal was written: the poll drops from
four instructions to three, the tightest loop from 23 to 22 per iteration, `01-arith-loop` from 51 to
50. Wall clock does not move — that loop runs about six instructions per nanosecond and is not
issue-limited. The conformance tree is unchanged.

## Stage 3 — the clock, which is the hole an attacker picks first

`Ctx::cpu_limit`'s field doc says the sampling is the host's and that until a timer exists the flag
is raised by tests alone. It still is: every caller of `Ctx::expire_deadline` in the tree is a test.
So `[limits] cpu_time` is a number nothing reads.

The expensive half of this is stage 2, which is why it is here rather than in a goal of its own.

5. **A sampler on the process's existing watchdog thread** — `crates/nvs-host/src/watchdog.rs` is
   already one thread for the process, already holds one entry per registered core, and already
   wakes on an interval. It gains the per-thread clock: `GetThreadTimes` on Windows,
   `CLOCK_THREAD_CPUTIME_ID` on Unix. **CPU time, never wall clock** —
   `Ctx::cpu_limit`'s own doc owns why, and it is the whole difference between stopping a runaway
   loop and killing a slow database query.
6. **It raises both halves.** `SafepointFlags::CPU_LIMIT` in the word stage 2 relocated, which is
   what compiled code polls; and `Ctx::expire_deadline`, which is what
   `rule:http-server/time-is-bounded-inside-a-helper`'s `bounded_loop` polls inside a member whose
   runtime scales with its input. Neither branch is new — `nvs_safepoint`'s CPU arm is written,
   handles the tier-1 handler and re-widens the reserve, and has only ever been reached by a test.
7. **The watchdog starts working as a consequence.** Its rule reads a core whose oldest in-flight
   request has passed its deadline; until now no request had one. Nothing in
   `crates/nvs-host/src/watchdog.rs` changes for this — it is the sampler above giving it something
   to read.

## Stage 4 — the allocator publishes, which bounds a loop

8. **`crates/nvs-runtime/src/budget.rs:@add`** — an armed absolute threshold beside the existing
   counters, and a compare per growing allocation; crossing it sets the memory bit. An uncapped
   request arms zero and short-circuits on the first compare.
9. **`crates/nvs-runtime/src/ctx/limits.rs:@refresh_limits`** — the one place a ceiling is computed
   is the one place the threshold is armed, so `Core\Config::set` moving `[limits] memory`
   mid-request leaves no stale copy. Saved and restored around an isolate sharing the thread.
10. **`crates/nvs-runtime/src/abi.rs:@run_helper`** — its comment says the breach is observed at the
    first member call after it. That stops being the whole truth; rewrite it as a whole.

## Stage 5 — the value allocators refuse, which bounds a single operation

A flag bounds a loop and cannot bound one operation: detection at the next poll is detection after
the allocation. Refusing in the global allocator would cover everything and abort the process,
because a `Vec` or `Box` inside helper code turns a null into Rust's own alloc-error path. So the
refusal goes one level up, where the requested size is known before the allocation and the caller is
ours.

11. **`crates/nvs-runtime/src/string.rs:@NvsStr::try_alloc_uninit`** and the grow path beside it —
    the pre-check, and **`alloc_uninit`'s aborting wrapper is deleted** so the fallible constructor
    is the only way to make a string. `object.rs`'s aborting site goes the same way.
12. **The primitives that already carry a `ctx` and a status refuse through it** —
    `nvs_array_append`, `nvs_array_spread`, `nvs_object_key_set` and `nvs_object_slot_set` are
    `(ctx, ..) -> i32` today and need no new channel.
13. **The ctx-less primitives refuse by returning a degenerate value** — `nvs_str_append` returns its
    `target` unchanged, which exactly balances the one reference `nvs_ir::ir::InstKind::StrAppend`
    says it consumes; `nvs_str_concat`/`nvs_str_concat_n` return a static immortal empty string,
    whose release is already a no-op; `nvs_array_set`/`nvs_array_set_index` return their array
    unchanged. None acquires an argument or a status, so no call site and no lowering changes.

## Stage 6 — a request is charged for its own allocations, not for its thread's balance

`Ctx::memory_used` is `live_bytes()` less the balance when the context was made, and `live_bytes` is
a **thread** balance. Freeing memory an *earlier* request allocated drives it down and buys the
current request headroom it never earned. `Core\Cache::local` is exactly such a store — a
`thread_local` that outlives requests, capped at 32 MiB by default and uncapped where an operator
writes `[cache.local] max_size = false`. Fill it in one request, evict it from the next, and the
second request's ceiling is `[limits] memory` plus what it evicted.

14. **A detached-accounting bracket in `crates/nvs-runtime/src/budget.rs`** — allocations and frees
    inside it move the process's counters and not the running request's measured usage. Every
    cross-request store takes it: `crates/nvs-stdlib/src/cache.rs`'s local tier first, and any other
    `thread_local` or process-lifetime store that a request can grow or shrink.
15. **The bracket is the mechanism, not a convention to remember.** It is a guard type whose `Drop`
    restores, so a store adopts the shape and the obligation arrives with it — `bounded_loop`'s
    argument, applied to a second obligation.

## Stage 7 — the records, the rules, and the comment that argues the opposite

16. **`crates/nvs-runtime/src/string.rs`'s block comment above the primitives** argues they cannot
    fail *because* allocation aborts rather than unwinds. Stage 5 makes that false; rewrite it whole.
17. **This goal's own record** holds stage 5's degenerate-return invariant and stage 6's accounting
    boundary.
18. **A rule fragment under `docs/rules/core-classes/` that bounds expansion at the input.** A member
    whose output scales with its input either allocates through the value allocator stage 5 guards,
    or bounds the *input* the way `rule:core-classes/regex-two-tiers` already does for patterns. It
    is written now, before the member that needs it exists: there is no decompression member in the
    tree today, and the day one lands the residual in stage 5 goes from a small format factor to a
    thousandfold one through a door this goal otherwise leaves open.

## Standing decisions

- **The poll word goes out of line and the handle is hoisted, and the indirection is not a cost to
  avoid.** Measured before this goal was written: one instruction *cheaper* per poll. If the handle
  spills under register pressure somewhere, that is break-even with today and still not a reason to
  fold the word back into `Ctx` — the aliasing question is why it is out of line.
- **Stage 3 samples CPU time, never wall clock.** A wall clock makes a runaway loop and a slow
  database indistinguishable, and `[limits] cpu_time` exists to stop the first without touching the
  second. If the per-thread clock is unavailable on a platform, that platform gets no CPU ceiling and
  says so at boot — it does not silently get a wall-clock one.
- **Stage 3 does not kill a wedged core and does not try.**
  `rule:http-server/a-wedged-core-is-shed-never-killed` stands; what changes is that a runaway
  *request* now stops itself at its own next poll, so the core is never wedged in the first place.
- **The refusal never goes in the global allocator.** A null from `Accounting::alloc` reaches
  `handle_alloc_error` or a `Vec`'s own path and aborts the process, which is worse than the bug.
  Closed; do not reopen it on the grounds that the allocator is the tidier chokepoint.
- **A ctx-less primitive never acquires a status ABI to carry this.** The degenerate return is the
  mechanism, and it is sound because the request is already dead: between the refusal and the next
  poll a program can build wrong values and compare them, and it can write no output, reach no `Core`
  member and touch nothing durable — all of those pass `run_helper`, which asks ahead of the body.
  That invariant is this goal's record's load-bearing claim; write it as such.
- **A refusal is a complete no-op, including not separating a shared array.** `foreach` walks the
  snapshot it started on and a write inside the body separates
  (`crates/nvs-runtime/src/array.rs:@nvs_array_next_slot`'s doc comment); a refused write that
  separated but did not write, or wrote into the shared original, is the one way this reaches
  `nvs_array_value_at`'s `.expect` on a live cursor. It must do neither.
- **Objects are out of scope for the pre-check.** An object's allocation is sized by its class, so no
  program drives one unbounded; they stay on stage 4's flag path. `nvs_array_new` needs nothing —
  it hands out a per-thread singleton.
- **Stage 6 brackets the known stores; it does not solve provenance.** Charging a free to whoever
  allocated the block needs per-request provenance, which is what M6's arena gives and what this goal
  does not build. Bracket the cache and every store like it, and record in this goal's own record that
  the general property waits for the arena. A session that finds a store this goal did not name brackets it and
  says so in the handoff.
- **The fallback, if stage 5's array half cannot be proven.** Ship strings alone: pre-check
  `nvs_str_concat`/`nvs_str_concat_n`/`nvs_str_append` and leave `nvs_array_set` on the flag path.
  Strings are where the unbounded doubling lives, and half the fix landed with its invariant proven
  beats all of it landed with one asserted. Record which half shipped.
- **Out of scope, named so a session does not wander in.** The stack ceiling is asserted rather than
  discovered and stays that way until M6 sizes the request's own stack. The tier-1 handler's
  `fatal_reserve` is above the cap by design. Whether compilation on the compile pool is charged to
  anybody is **unverified** and belongs to whoever opens the arena, not here.
- **This goal opens one new record and no other number**, whose `changes:` block names the rules it
  adds — the refusal and its degenerate return, the accounting boundary, and stage 7's expansion
  rule — and cites `rule:errors/on-limit` and `rule:programs/memory-priority` as the rules it works
  inside. No existing record is amended. **Take the next free number when the slice is written**, one
  above the highest file in `docs/decisions/`: this goal is second from the end of the chain, so any
  number named here is a number some earlier goal has since claimed. 0157 was named while it was free
  and is now an accepted record about shape types.
- **What it spends, per `rule:programs/memory-priority`'s *say what you spend*:** one thread-local
  load and one compare per growing allocation, in front of a platform-heap round trip
  `docs/perf/userland-gap.md` measures at 28.7 ns; one word per request tree; one clock read per core
  per watchdog interval, on a thread that already wakes on that interval; and nothing on the poll
  path, which gets cheaper.
