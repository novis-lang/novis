# Handoff

## State

**The host end of ADR 0072's seam is on disk and green; the other end is still empty.**
`crates/nvs-host/src/group.rs` is the one implementor of `nvs_runtime::host::Host`, and
`Scheduler::run` installs it beside the task tree's guard (`scheduler.rs:@Scheduler::run`).
That module's doc is the one home of the two decisions only an implementor can make — what a
child gets for a `Ctx`, and the four-step sequence that makes § 4's "nothing still running" a
property of the call. `nvs-host` is 101 tests, up from 93.

**A child's context aliases the request's statics** (`nvs_runtime::Ctx::child`, an `unsafe fn`
whose obligation is discharged by the group runner's structure). Output and the assertion
ledger are the child's own and are spliced back in **job** order; the origin, debug flags,
error class and deadline word are copied. `Ctx::statics_base` is new and is how a test asserts
the alias.

**Two shapes on the seam changed while filling it in**, both in
`crates/nvs-runtime/src/host.rs`: `Outcome::Threw` carries a `Thrown` (`:136`) rather than a
`Value`, because that is what `take_thrown` produces and `raise` takes; and `Job` (`:97`) now
documents that it **may be dropped without ever being called** — a cancelled or deadline-ended
group drops its unstarted jobs, so whatever a job captured must be released by that capture's
own `Drop`. That obligation lands on slices 2 and 3 below.

**`examples/tasks.nvs` still aborts at the placeholder** (`crates/nvs-stdlib/src/task.rs:135`)
and the acceptance check still fails there. Nothing in `nvs-stdlib` changed this session.

**Orientation gaps.** `[context] adrs` still carries ADR 0072 §§ 4 and 5 only; §§ 1, 2 and 3
specify every remaining Stage 4 item and are still sliced by hand. `[context] modules` has no
`nvs-cli` pattern, so the `nvs run` execution site — `crates/nvs-cli/src/main.rs:606` — had to
be grepped for; the handoff's own anchor pointed at `runner.rs:189`, which is `nvs test`'s
context and not `nvs run`'s. The crate-dependency-graph gap the previous session reported is
still open.

## Next group

**Fill the member end in, from `nvs run` down to the two bodies.** File set:
`crates/nvs-cli/src/main.rs` (`:606`), `crates/nvs-cli/Cargo.toml:15`,
`crates/nvs-stdlib/src/task.rs` (`CLASS:81`, `address:118`, the placeholder at `:135`), and
`crates/nvs-runtime/src/host.rs` (`Job:97`, `Bounds:106`, `Outcome:121`) as the contract both
ends read.

- [ ] **`nvs run` runs its program inside a task.** ADR 0072 §§ 1, 3. `nvs-cli` gains
      `nvs-host.workspace = true` (`Cargo.toml:15`), and `main.rs:606`'s
      `nvs_runtime::call(entry, &mut ctx, &[])` becomes a `Scheduler::spawn` of that call plus
      `reactor::install` and `run_until_idle`. Two things have to come back out of the task
      and neither does today: the call's status (record it in a `Cell` the body captures) and
      the `Ctx` itself, which arrives in `Scheduler::take_finished()`'s `Finished` rather than
      staying borrowed — `main.rs:620-660` reads `ctx.flush_output`, `ctx.exit_code` and
      `ctx.take_thrown` off it afterwards. A `TaskRoot::Request` is the right root: ADR 0106
      § 2 fails one request rather than retiring the worker.
- [ ] **`nvs_core_task_all`'s body.** ADR 0072 §§ 1, 3, 4. § 1's shape fields become one
      `host::Job` each, in field order, calling `nvs_runtime::call_closure`; the answers come
      back as `Outcome::Completed` and are assembled into the shape. `Threw` is
      `ctx.raise(thrown)` plus the helper's `THROWN`; `TimedOut` builds `TimeoutError`, which
      the host deliberately does not know. **Each job captures a `callable` `Value` and must
      release it from a wrapper's own `Drop`**, per `host.rs:97` — a job dropped unrun is the
      ordinary cancelled path, not an error. Delete `unimplemented_scheduler_half` only once
      both rows have a body.
- [ ] **`nvs_core_task_map`'s body.** ADR 0072 § 2. One `Job` per element over the single
      shared callback, and the result array **preserves the input's keys and order** — the
      group already answers in job order, so what is left is carrying the keys alongside the
      index. Same capture-release obligation, once per element.
- [ ] **A `.nvst` case per member**, which `conformance_coverage.rs` requires anyway. The
      shapes worth taking are *a bound asserted on both sides* for `limit` and *agreement*
      between `all` and `map` on what a deadline does.

## Backlog

- `Scheduler::finished` accumulates a `Finished` per child until `take_finished` drains it; a
  group of N children leaves N behind. Whoever owns the request boundary has to drain — worth
  a line in `scheduler.rs`'s module doc if slice 1 above does not force it.
- `Core\Task\Channel` has no row in `nvs_stdlib::registry`; item 11's language surface.
- Whether `{limit: 0}` deserves a diagnostic is the member's question — `group.rs`'s module
  doc clamps it to 1 and says so.
- § 4's second throw goes to the failing child's diagnostic channel until `Core\Log` exists
  (`group.rs`, `Child::run`).
- M4's residue: the 1000-case corpus count, met as the suite grows.
- `docs/agent/loop-goal.toml` `[context]`: add ADR 0072 §§ 1–3 and an `nvs-cli` module pattern.
