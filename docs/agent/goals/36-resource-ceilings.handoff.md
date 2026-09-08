# Handoff

## State

**Goal 36 — every resource ceiling stops the request that breaks it — has just started; nothing of it
has landed yet.** Goal 35's whole acceptance list is this goal's floor.

Three holes, all reproduced or read out of the tree rather than inferred, and the goal's stages are
ordered by what an attacker reaches for first rather than by difficulty:

**The cheapest attack is `while (true) {}`.** There is no CPU or wall-clock enforcement anywhere in
the running system: `Ctx::cpu_limit`'s field doc says the flag is raised by tests alone, and every
caller of `Ctx::expire_deadline` in the tree is in fact a test. The watchdog reads the reactor's timer
wheel and a bare loop arms no timer; even on a report,
`rule:http-server/a-wedged-core-is-shed-never-killed` sheds rather than kills, so the core is gone
for the life of the process. N such requests retire N cores. Stage 3.

**The memory half is the one that was audited first.** `while (true) { $a .= $a; }` ran to completion
holding 268,450,240 bytes against a 16 MiB ceiling — the breach was reported by the `echo` after the
loop — and unbounded it ends at `handle_alloc_error`, which aborts the worker. Stages 4 and 5.

**And a request's ceiling is not its own.** `Ctx::memory_used` is a delta against a *thread* balance,
so freeing what an earlier request allocated buys headroom. `Core\Cache::local` is a `thread_local`
that outlives requests: fill it in one request, evict it from the next. Stage 6.

**Stage 2 is already measured and is the keystone for two of them.** The out-of-line poll word with
the handle hoisted into the ABI entry block was built and benchmarked before this goal was written —
the poll drops from four instructions to three, the tightest loop from 23 to 22 per iteration, wall
clock does not move, conformance is unchanged. It is what lets a *non-owning thread* publish, which
is what stage 3's sampler and stage 4's allocator both need. Do not re-litigate the indirection; the
standing decisions say why.

## Next group

**Stage 0 and stage 2 together.** Stage 0 is three `.nvst` cases and no Rust; stage 2 is one file set
— `Ctx`, its accessors, and the poll's emit site — and every stage above it publishes into what it
builds.

- [ ] **Write stage 0's three cases** into `tests/conformance/error/`, copying the handler and reserve
      shape from `a-limit-fatal-is-not-catchable.nvst`. All three run red. The memory-loop case must
      assert that nothing printed *before* the breach — that is what distinguishes the loop being
      stopped from the `echo` after it being stopped.
- [ ] **The CPU case needs `[limits] cpu_time` and a body that allocates nothing.** It is the one case
      that hangs if the stage it belongs to regresses, so give it a ceiling small enough that the
      suite notices in seconds rather than minutes.
- [ ] **Then move the poll word out of `Ctx`** — `crates/nvs-runtime/src/ctx/mod.rs:@Ctx`, the
      accessors in `ctx/safepoint.rs`, and `Ctx::child`/`Ctx::isolate` cloning the handle so a tree
      shares one word. `Self::deadline` is the worked example for every one of those decisions.
- [ ] **Then hoist the handle** — `crates/nvs-codegen/src/emit.rs:@emit_safepoint` loads it from the
      value bound beside `ctx_p` in the ABI entry block, not from `Ctx` at each poll. Confirm with
      `nvs run --dump-asm` on a bare `while`: the back edge must hold one load, one test and one
      branch, and no context re-materialization.
- [ ] **Do not fold the word back into `Ctx`** however tempting one fewer indirection looks. The
      allocator and the sampler writing into a live `&mut Ctx` is the aliasing problem the move exists
      to solve, and it is invisible in a build that happens to work.
- [ ] **Stage 3 is the next group after this one**, and it is the highest-severity stage in the goal.
      It is a small addition on top of stage 2: `nvs_safepoint`'s CPU arm and `bounded_loop`'s
      deadline poll are both already written and have only ever been reached by tests. What is new is
      the per-thread clock on the watchdog thread that raises them.
