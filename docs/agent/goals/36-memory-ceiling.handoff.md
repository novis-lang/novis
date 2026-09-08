# Handoff

## State

**Goal 36 — the memory ceiling stops a single operation — has just started; nothing of it has landed
yet.** Goal 35's whole acceptance list is this goal's floor.

The gap is real and reproduced, not inferred. `while (true) { $a .= $a; }` under `[limits] memory`
is never stopped: `nvs_safepoint`'s memory branch is written but unreachable, because compiled code
branches on `Ctx`'s flags word and no `SafepointFlags` bit means "over memory". Run bounded at 22
doublings under a 16 MiB ceiling, the loop completed holding 268,450,240 bytes and the `FATAL` came
from the `echo` after it. Unbounded it ends at `handle_alloc_error`, which aborts the process.
`crates/nvs-runtime/src/budget.rs`'s module doc already records this as a known gap and names the two
candidate fixes.

**Stage 2 is already measured.** The out-of-line poll word with the handle hoisted into the ABI entry
block was built and benchmarked before this goal was written: the poll drops from four instructions
to three, the tightest loop from 23 to 22 per iteration, wall clock does not move, and the
conformance tree is unchanged. Do not re-litigate the indirection — the standing decisions say why.

**Stage 4 is the part with a design in it.** A flag bounds a loop and cannot bound one operation, so
the refusal goes in the runtime's own value allocators rather than in the global allocator, and a
ctx-less primitive reports it by returning a degenerate value rather than by acquiring a status ABI.
That rests on one invariant — between the refusal and the next poll a program can build wrong values
and compare them, and can reach nothing observable without passing `run_helper` — and the array half
of it is what the cursor test exists to prove.

## Next group

**Stage 0 and stage 2 together.** Stage 0 is two `.nvst` cases and no Rust at all; stage 2 is one
file set — `Ctx`, its accessors, and the poll's emit site — and it is the mechanism every stage after
it publishes into.

- [ ] **Write stage 0's two cases** into `tests/conformance/error/`, copying the handler and reserve
      shape from `a-limit-fatal-is-not-catchable.nvst`. Both run red. The loop case must assert that
      nothing printed *before* the breach, which is what says the loop itself was stopped rather than
      the `echo` after it.
- [ ] **Then move the poll word out of `Ctx`** — `crates/nvs-runtime/src/ctx/mod.rs:@Ctx`, the
      accessors in `ctx/safepoint.rs`, and `Ctx::child`/`Ctx::isolate` cloning the handle so a tree
      shares one word. `Self::deadline` is the worked example for every one of those decisions.
- [ ] **Then hoist the handle** — `crates/nvs-codegen/src/emit.rs:@emit_safepoint` loads it from the
      value bound beside `ctx_p` in the ABI entry block, not from `Ctx` at each poll. Confirm with
      `nvs run --dump-asm` on a bare `while`: the back edge must hold one load, one test and one
      branch, and no context re-materialization.
- [ ] **Do not fold the word back into `Ctx`** however tempting one fewer indirection looks. The
      allocator writing into a live `&mut Ctx` is the aliasing problem the move exists to solve, and
      it is invisible in a build that happens to work.
- [ ] **Stage 3 and stage 4 are the next group after this one**, and they share
      `budget.rs`/`abi.rs`/`string.rs`/`array.rs`. Do not start stage 4's refusal before stage 3's
      flag exists — the refusal reports through the flag, so half of it cannot be tested alone.
