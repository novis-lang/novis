# Loop goal 36 — The memory ceiling stops a single operation

`[limits] memory` stops the request that breaks it, whatever that request was doing — a loop over
runtime primitives that calls nothing, or one operation that asks for more than the whole budget at
once. Today neither is true: a breach is only ever *noticed*, at the next member call or the next
safepoint slow path, and nothing puts the request over the ceiling into either. `while (true) { $a
.= $a; }` under a 16 MiB ceiling runs until the allocator fails and `handle_alloc_error` aborts the
process, taking every other in-flight request on it.

It sits here because it is M6's own acceptance line — *memory/CPU caps terminate runaway scripts as
a `FATAL`* — and that line has never been true for a program that calls nothing. It is a priority-1
item, not a priority-3 one: the end state is a process abort, so this is request isolation, not
fairness.

Goal 35's whole acceptance list is this goal's floor, and it is never traded.

## Stage 0 — the catch-up

Nothing on disk contradicts this goal's rule; `rule:errors/on-limit` already says what a breach owes
and the ladder already delivers it correctly *once something notices*. What is owed is the two
fixtures that say nothing notices, and they run red the day they are written:

1. **A pure-primitive loop is stopped.** A `.nvst` case whose loop body calls nothing —
   `$a .= $a;` and an `int` step — under a `[limits] memory` ceiling, expecting the `FATAL` and
   `Core\Fatal::onLimit`'s report. Today the loop runs to completion and the breach is reported at
   the `echo` after it, which is what the case must not accept.
2. **A single operation is stopped before it allocates.** A case whose one statement asks for more
   than the ceiling in one step, expecting the `FATAL` **and** a peak that never held the request's
   whole ask. `tests/conformance/error/a-limit-fatal-is-not-catchable.nvst` is the shape to copy for
   the handler half.

`docs/agent/playbook.md`'s bullet *A memory-limit breach is observed at the first `run_helper`
member call after it* is this gap written as a trap. Its `[until:]` trailer names the comment in
`crates/nvs-runtime/src/abi.rs` that stage 3 rewrites, so the wrap retires the bullet when that
lands. Do not reword it before then.

## Stage 1 — the floor

Goal 35's whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded for anything above it.

## Stage 2 — the keystone: the poll word moves out of `Ctx`

Nothing can publish a breach into the word compiled code polls while that word lives inside `Ctx`: the
allocator runs on the same thread as the `&mut Ctx` a helper body holds, so a write through a stashed
pointer aliases a live unique reference. The word therefore moves out of line, and `Ctx` keeps its
address in the hot slot compiled code already loads.

1. **`crates/nvs-runtime/src/ctx/mod.rs:@Ctx`** — `safepoint` becomes the *address* of an
   `AtomicU64`, with the word itself owned by a handle beside the cold fields. `SAFEPOINT_OFFSET`
   and the hot-line arrangement `ctx::tests::the_hot_words_come_first_and_are_a_word_apart` pins are
   unchanged, because a pointer is the same eight bytes the flags word was.
2. **One word per request *tree*, not per context**, exactly as `Self::deadline` already is and for
   its reason: `rule:security/isolate-shares-nothing` gives a tree one ceiling to divide, and a child
   built after the flag was set must not be born clean. `Ctx::child` and `Ctx::isolate` clone the
   handle.
3. **`crates/nvs-runtime/src/ctx/safepoint.rs`** — the accessors read and write through the handle.
   Every Rust-side path goes through it; only compiled code follows the raw pointer.
4. **`crates/nvs-codegen/src/emit.rs:@emit_safepoint`** — the poll follows the pointer, and **the
   handle is loaded once in the ABI entry block** beside `ctx_p`, not once per poll. A value defined
   there dominates every block below it, so a back edge keeps the single load it already had.

**This is measured, not estimated.** Built and run: the poll drops from four instructions to three,
because the hoisted handle lives in its own register and the poll loses the context re-materialization
Cranelift emits today. The tightest loop goes 23 → 22 instructions per iteration and
`benches/userland/01-arith-loop.nvs` 51 → 50. Wall clock does not move — that loop runs about six
instructions per nanosecond and is not issue-limited. The conformance tree is unchanged.

## Stage 3 — the allocator publishes, which bounds a loop

5. **`crates/nvs-runtime/src/budget.rs:@add`** — an armed absolute threshold beside the existing
   counters, and a compare per growing allocation. Crossing it sets the memory bit in the word
   stage 2 relocated. An uncapped request arms zero and short-circuits on the first compare.
6. **`crates/nvs-runtime/src/ctx/limits.rs:@refresh_limits`** — the one place ceilings are computed
   is the one place the threshold is armed, so `Core\Config::set` moving `[limits] memory`
   mid-request cannot leave a stale copy behind. Saved and restored around an isolate sharing the
   thread.
7. **`crates/nvs-runtime/src/abi.rs:@run_helper`** — its comment currently says the breach is
   observed at the first member call after it. That stops being the whole truth here; rewrite it as
   a whole rather than appending to it.

## Stage 4 — the value allocators refuse, which bounds a single operation

A flag bounds a loop and cannot bound one operation: detection is at the next poll, so an operation
that asks for four gigabytes still gets them. Refusing in the global allocator would cover
everything and abort the process, because a `Vec` or `Box` inside helper code turns a null into
Rust's own alloc-error path. So the refusal goes one level up, at the runtime's own value
allocators — where the requested size is known *before* the allocation and the caller is ours.

8. **`crates/nvs-runtime/src/string.rs:@NvsStr::try_alloc_uninit`** and the grow path beside it —
   the pre-check, and **`alloc_uninit`'s aborting wrapper is deleted** so the fallible constructor
   is the only way to make a string. `object.rs`'s aborting site goes the same way.
9. **The primitives that already carry a `ctx` and a status refuse through it** —
   `nvs_array_append`, `nvs_array_spread`, `nvs_object_key_set` and `nvs_object_slot_set` are
   `(ctx, ..) -> i32` today and need no new channel.
10. **The ctx-less primitives refuse by returning a degenerate value** — `nvs_str_append` returns
    its `target` unchanged, which exactly balances the one reference `nvs_ir::ir::InstKind::StrAppend`
    says it consumes; `nvs_str_concat`/`nvs_str_concat_n` return a static immortal empty string,
    whose release is already a no-op; `nvs_array_set`/`nvs_array_set_index` return their array
    unchanged. None acquires an argument or a status, so no call site and no lowering changes.

## Stage 5 — the record, and the comment that argues the opposite

11. **`crates/nvs-runtime/src/string.rs`'s block comment above the primitives** argues that these
    cannot fail *because* allocation aborts rather than unwinds. Stage 4 makes that false. It is
    rewritten as a whole, and ADR 0157 is where the reasoning goes.

## Standing decisions

- **The poll word goes out of line and the handle is hoisted, and the extra indirection is not a
  cost to be avoided.** It was built and measured before this goal was written; the result is one
  instruction *cheaper* per poll. If a session finds the handle spilling under register pressure in
  some function, that is break-even with today and still not a reason to fold the word back into
  `Ctx` — the aliasing question is why it is out of line.
- **The refusal never goes in the global allocator.** A null from `Accounting::alloc` reaches
  `handle_alloc_error` or a `Vec`'s own path and aborts the process, which is worse than the bug.
  This is closed; do not reopen it on the grounds that the allocator is the tidier chokepoint.
- **A ctx-less primitive never acquires a status ABI to carry this.** The degenerate return is the
  mechanism, and it is sound because the request is already dead: between the refusal and the next
  poll a program can build wrong values and compare them, and it cannot write output, reach a
  `Core` member or touch anything durable — every one of those passes `run_helper`, which asks
  ahead of the body. Write that invariant into ADR 0157 as the load-bearing claim it is.
- **A refusal is a complete no-op, including not separating a shared array.** `foreach` walks the
  snapshot it started on and a write inside the body separates
  (`crates/nvs-runtime/src/array.rs:@nvs_array_next_slot`'s own doc comment); a refused write that
  separated but did not write, or wrote into the shared original, is the one way this could reach
  `nvs_array_value_at`'s `.expect` on a live cursor. It must do neither.
- **Objects are out of scope for the pre-check.** An object's allocation is sized by its class, so a
  single `nvs_object_new` or `nvs_object_clone` cannot be driven unbounded by a program; they stay
  on stage 3's flag path. `nvs_array_new` needs nothing either — it hands out a per-thread singleton
  and allocates only on a thread's first empty array.
- **The fallback, if the array half cannot be proven.** Ship strings alone: give
  `nvs_str_concat`/`nvs_str_concat_n`/`nvs_str_append` the pre-check and leave `nvs_array_set` on the
  flag path. Strings are where the unbounded doubling lives, and half the fix landed with its
  invariant proven beats all of it landed with one asserted. Record which half shipped in ADR 0157.
- **This goal opens ADR 0157**, whose `changes:` block names the rule it adds — that a runtime value
  allocator refuses before allocating and reports through a degenerate value rather than an abort —
  and cites `rule:errors/on-limit` and `rule:programs/memory-priority` as the rules it works inside.
  No existing record is amended.
- **What it spends, per `rule:programs/memory-priority`'s *say what you spend*:** one thread-local
  load and one compare per growing allocation, in front of a platform-heap round trip
  `docs/perf/userland-gap.md` measures at 28.7 ns; one word per request tree; and nothing on the
  poll path, which gets cheaper. If a session finds a case where the compare is measurable against
  the allocation it guards, that is a finding worth the handoff, not a reason to skip the check.
