# Handoff

## State

**Goal 2 of the parity program. ADR 0106 § 5 is complete — all three halves.** The deadline flag
lives in `Ctx`'s hot line (`DEADLINE_OFFSET`, `Ctx::deadline_expired`/`expire_deadline`); the poll
is supplied by `nvs_runtime::bounded_loop`, which owns the batch (`DEADLINE_POLL_BATCH`, 256) and
what a fired poll returns; and `Core\Arr::map` is the first real O(input) member to walk through it.
A fired poll is `Fault::Fatal` — a deadline is a cancellation, and ADR 0072 § 5 makes cancellation
uncatchable, the same standing `nvs_safepoint` already gives `SafepointFlags::CANCEL`.

**The ADR's one number is measured, not asserted.**
`benches/abi-probe/tests/perf_guards.rs`'s `an_amortised_deadline_poll_costs_less_than_the_check_it_rides_beside`
measures both sides in the same run — 0.044 ns amortised per iteration against a 0.237 ns hot-line
check on this box, 5.4x headroom — so no ceiling constant can go stale on another machine.

**Stage 0 is closed.** §§ 1–3's containment boundary and its three named invariants are on disk;
§ 4's engine-side depth bound already exists in both places it names (`nvs-stdlib`'s
`json::DEFAULT_MAX_DEPTH` at PHP's 512, `nvs-syntax`'s `parser::MAX_RECURSION_DEPTH`); § 5 is above.
The next group is Stage 2, which the goal says everything else waits on.

**`crates/nvs-host` still does not exist**, so `orient.py` warns that the `[context] modules` pattern
`crates/nvs-host/src/*.rs` matches nothing. Expected until the group below lands it, not a manifest bug.

**The acceptance check on `examples/tasks.nvs` (`Core\Task` has no member named `all`) is not a
regression and cannot be closed yet** — loop-goal.md § *Stage 2* says no `Core\Task` member is
written until that stage is green, and `Task::all` is Stage 4's. The four goal fixtures each stop at
their own item's first missing member; that is this goal's ordinary state.

**The one rule every session of this goal holds:** the runtime is ours and it is not `async`, and
`nvs-host`'s socket implements plain `std::io::Read`/`Write` while parking its coroutine.
`docs/plan/design.md` § *Thread-per-core, shared-nothing runtime* is its only home.

## Next group

**Stage 2's keystone — the crate everything above is a consumer of.** One new file set:
`crates/nvs-host/src/*.rs`, the workspace `Cargo.toml` members list, and `docs/adr/0115-*.md`.
Re-check the next free ADR number before creating the file.

- [ ] **`crates/nvs-host` exists, and one core runs one scheduler.** loop-goal.md § *Stage 2* item 3:
      a per-core thread pinned to a CPU, a run queue of coroutines, and a `Ctx` carrying the yielder.
      The shape is already modelled — `benches/abi-probe/src/lib.rs:105` (`pub struct Ctx`, the one
      its own doc calls "deliberately shaped like the real `Ctx` will be"), `:90` (`enum Waiting`),
      `:686` (`in_coroutine`, the `corosensei` driver). Refcounts stay non-atomic; that is
      `design.md`'s decisive structural choice and what this crate must not break.
- [ ] **ADR 0115 — the reactor and the parking stream.** loop-goal.md § *Standing decisions* names
      this as one of exactly two pre-authorized ADR slots and says it is the **first** slice of item
      4, not a follow-up: what readiness mechanism each platform uses, what the parking contract is,
      what a `WouldBlock` costs, and how a coroutine's stack is accounted. Shape and the six rules
      `python tools/adr.py` enforces are in conventions.md § *An ADR*.
- [ ] **`NvsTcp` implements `std::io::Read`/`Write` over the reactor**, parking the coroutine rather
      than blocking the core — ADR 0106 § 6's second paragraph, which is why a synchronous-looking
      read is safe here at all.

## Backlog

- Timers on the same reactor, and they are what will call `Ctx::expire_deadline` — loop-goal.md § *Stage 2* item 5.
- The blocking pool, bounded at twice the core count — ADR 0106 § 6.
- The watchdog reading the in-flight deadline each worker already maintains — ADR 0106 § 7.
- More O(input) `Core` members adopt `bounded_loop` (`Arr::filter`, `Arr::reduce`, `Arr::mapKeys`) — ADR 0106 § 5.
- `Core\Task::all`/`::map` and the roster — loop-goal.md § *Stage 4*; the `examples/tasks.nvs` check stays red until then.
- M4's residual 1000-case conformance corpus count — docs/implementation-plan.md.
