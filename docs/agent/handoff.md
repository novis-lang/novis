# Handoff

## State

**Goal 2 of the parity program, and ADR 0106 § 2's containment rule has landed.** The outer boundary is
`nvs_runtime::run_task` in `crates/nvs-runtime/src/abi.rs` — one `catch_unwind` per *task*, applied by
whatever spawns it rather than per call site, with `TaskRoot::{Request, Worker}` carrying § 2's split and
`TaskPanic::retires_worker` answering it. The `nvs_helper!` wrapper is untouched and is the **inner**
boundary; a fault it catches still becomes `FATAL` on the `Ctx` and never reaches the outer one.

The two rules that travel with § 3 are *held*, not newly written: teardown is iterative rather than
recursive (`crates/nvs-runtime/src/release.rs`'s module doc is that rule's home, pinned by
`object.rs:2693` and `array.rs:2486`), and nothing on that path is fallible. `run_task`'s own doc comment
states both and points at them.

**`crates/nvs-host` still does not exist.** Coroutines live only in the M0 spike,
`benches/abi-probe/src/lib.rs`, whose invariants now include the two that pin the outer boundary across a
stack switch — a panic on a coroutine stack with the JIT frames already returned is contained at the task
root, and the thread goes on creating and driving coroutines.

**The tree was red on arrival and is green now, in something no slice of this goal touches.** The goal
switch at `84c294e` dropped the item list that attributed M4's seventeen `nvs-ir` lowering refusals, so
`every_refusal_is_a_diagnostic_or_decided` failed on sites nobody had edited — the second switch running
to the same result. That inventory now lives in `docs/agent/carried-refusals.md`, which `holes.py` reads
as a second item source, so no goal inherits it by hand and no switch can drop it. The playbook bullet is
the short version.

**The one rule every session of this goal holds:** the runtime is ours and it is not `async`, and
`nvs-host`'s socket implements plain `std::io::Read`/`Write` while parking its coroutine.
`docs/plan/design.md` § *Thread-per-core, shared-nothing runtime* is its only home.

## Next group

**ADR 0106 § 5 — time is bounded *inside* a helper, not only between helpers.** The deadline flag goes in
the hot cache line the stack check already loads, so a poll is a compare rather than a load, and the poll
is supplied by a combinator rather than remembered per helper. This is the second half of the group the
last session named; § 4's remaining half moved to the backlog because it is a different file set.

One file set: `crates/nvs-runtime/src/ctx.rs`, `crates/nvs-runtime/src/abi.rs`,
`benches/abi-probe/tests/perf_guards.rs`.

- [ ] **The request's deadline flag lives in `Ctx`'s existing hot line.** ADR 0106 § 5. It goes beside
      `safepoint`/`stack_limit` and gets an `offset_of!` constant like its neighbours —
      `crates/nvs-runtime/src/ctx.rs:576` (`SAFEPOINT_OFFSET`), `:583` (`STACK_LIMIT_OFFSET`), `:748`
      (`arm_stack_limit`, the one place a bound is computed), `:1439` (`nvs_stack_check`).
- [ ] **A bounded-loop combinator supplies the poll.** ADR 0106 § 5's first constraint, whose argument is
      `nvs_helper!`'s: a helper adopts a shape and the shape carries the obligation. It belongs next to
      `run_task` in `crates/nvs-runtime/src/abi.rs:404`. Its second constraint is the one to design
      against — a poll site must be a point at which abandoning leaves the value consistent, and where the
      operation has none, the bound goes on the *input* instead (ADR 0056's precedent).
- [ ] **The cost bound the ADR owns.** The amortised poll must stay under the stack check's own per-call
      cost, held in `benches/abi-probe/tests/perf_guards.rs` — `:69`
      (`a_checked_return_frame_stays_cheap`) and `:181` are the two shapes to copy.

## Backlog

- § 4's engine-side depth counter for the **parser**, plus its `nvs.toml` directive per ADR 0064 — a
  different file set (`nvs-syntax`). `Core\Json` already carries its half: `json.rs:241` (`max_depth`),
  `json.rs:492` (`DEPTH_CEILING_U32`).
- § 6's blocking pool, bounded at twice the core count — needs `nvs-host` to exist first.
- The reactor and parking-stream ADR, this goal's Stage 2 item 4 and one of its two ADR slots
  (`docs/agent/loop-goal.md` § *Standing decisions*).
- `orient.py`'s `[context] modules` pattern `crates/nvs-host/src/*.rs` matches nothing and will keep
  warning until that crate exists; nothing else the pack left out was needed this session.
