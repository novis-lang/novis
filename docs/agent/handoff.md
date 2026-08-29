# Handoff

## State

**Goal 2, Stage 2. `crates/nvs-host` exists and one core runs one scheduler.** A `Worker` is one OS
thread pinned to one CPU; a `Scheduler` is its run queue; a task is a `corosensei` stackful coroutine
that suspends by reaching its yielder through `Ctx`. 12 tests, all green. The crate's own module docs
own the reasoning — `lib.rs` for the layering, `scheduler.rs` for why a task never migrates,
`affinity.rs` for why a refused pin is never fatal.

**The yielder travels in the real `Ctx` as an opaque `*const ()`** (`ctx.rs`: the `yielder` field,
`Ctx::yielder`/`set_yielder`, cold, below the hot line). Opaque so `nvs-runtime` — the crate every
compiled unit links — needs no coroutine dependency; `nvs-host` owns the single `unsafe` that turns
it back into a `&Yielder`, which is why that crate declares `unsafe_code = "deny"` rather than
inheriting the workspace's `forbid`. `scheduler::suspend(&Ctx, Waiting)` is the seam, and it returns
`false` rather than pretending when there is no scheduler beneath.

**ADR 0115 is written and indexed** — readiness on all three platforms via `mio` (not `tokio`, and
§ 1 says why that is not the standing decisions' `BLOCKED`), the five-rule parking contract, try-the-
syscall-then-park with the cost table that decides the order, and a 1 MiB reserved / resident-narrow /
pooled-per-worker stack. Its § 4 also names what closes `ctx.rs`'s recorded known gap: a task on a
host-allocated stack can arm its recursion limit from real bounds.

**No reactor yet, so nothing parks for a real reason.** `Waiting::Parked` + `Scheduler::wake(TaskId)`
is the whole seam and it is tested by hand-waking. That is the next group.

**The acceptance check on `examples/tasks.nvs` (`Core\Task` has no member named `all`) is still not a
regression** — loop-goal.md § *Stage 2* writes no `Core\Task` member until that stage is green, and
`Task::all` is Stage 4's. Unchanged by this session.

**`core_affinity` is a new dependency** (+`num_cpus`, +`hermit-abi`, all MIT/Apache-2.0); the
workspace `Cargo.toml` comment above it answers ADR 0051 § 4's two questions.
`THIRD-PARTY-LICENSES.txt` regenerated to no diff — `nvs-host` is not yet in `nvs-cli`'s graph, so
add a `gen-attribution.py` run to the session that wires it in.

## Next group

**The reactor, now that ADR 0115 specifies it.** One file set: `crates/nvs-host/src/` (a new
`reactor.rs` and `net.rs`, plus `lib.rs:41` where the modules are declared and `lib.rs:44` where they
are re-exported), and `crates/nvs-host/Cargo.toml:12` for `mio`.

- [ ] **`reactor.rs` — `mio`-backed readiness, keyed by `TaskId`.** ADR 0115 §§ 1–2: register-then-
      suspend, a wake is a hint, deregister on `Finished`, and poll with a zero timeout while the run
      queue is non-empty. It hangs off `Scheduler::run`'s exit condition —
      `scheduler.rs:245` (`pub fn run`) returns when `ready` is empty, and `scheduler.rs:288`
      (`parked_count`) is the "block in the reactor now" test. Add `mio` to the workspace
      `Cargo.toml` beside `core_affinity` with the same two-questions comment.
- [ ] **`net.rs` — `NvsTcp: std::io::Read + Write` over the reactor.** ADR 0115 § 3, in that order:
      syscall first, register-and-park only on `WouldBlock`, keep the registration while the stream
      is held. It calls `scheduler::suspend` at `scheduler.rs:299`; the test that matters asserts
      through `std::io::Read` alone, with no scheduler-aware call in the body.
- [ ] **Arm the recursion limit from the task's own stack.** ADR 0115 § 4's last bullet closes the
      known gap recorded in `crates/nvs-runtime/src/ctx.rs:62` — `Scheduler::spawn`
      (`scheduler.rs:196`) calls `Ctx::arm_stack_limit` with the bounds `corosensei` handed it,
      instead of leaving `Ctx::new`'s `STACK_CEILING` guess in place. Edit that module doc in the
      same commit; it currently says "at M6".

## Backlog

- Stage 2 item 4's blocking pool — filesystem, name resolution, waiting on a child; ADR 0106 § 6 owns
  the bound (twice the core count) and this crate is where it goes.
- A stack pool per worker, ADR 0115 § 4 — `Scheduler` allocates a fresh `corosensei` default stack per
  task today.
- `nvs-host` is in no binary's dependency graph yet; wiring it into `nvs-cli` is what makes
  `gen-attribution.py` see `core_affinity`.
- `Waiting` is `#[non_exhaustive]` with two unit variants; ADR 0115's I/O registration is what gives
  it a third, per `scheduler.rs:63`.
- M4's residue, the 1000-case corpus count — `docs/plan/m4.md`.
- `benches/abi-probe`'s `Ctx` and the real one have now diverged in purpose; the probe's doc still
  says "deliberately shaped like the real `Ctx` will be" (`benches/abi-probe/src/lib.rs:100`).
