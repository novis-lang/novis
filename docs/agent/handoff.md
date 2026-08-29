# Handoff

## State

**ADR 0072's roster runs end to end, and `examples/tasks.nvs` prints all four frozen lines.**
The acceptance check that had been failing (`limit held at 1` against `limit held at 2`) is
closed: `Core\Time::sleep` parks through `nvs_runtime::host::Host::sleep` instead of blocking
the thread, so children under a `limit` overlap and the gauge sees the peak.

**A cancelled task standing on script frames is resumed, not unwound.** That is the mechanism
under the line above and its home is `crates/nvs-host/src/scheduler.rs`'s module doc § *The task
tree, and what cancelling one costs*. `nvs_runtime::HelperFrame` counts an `extern "C"` frame
for as long as a helper runs (two thread-local accesses per helper call, priced in its own doc);
`yield_on` reads the count at each suspension and `Task::unwindable` remembers it; a cancelled
task that is not clear of them gets `Resume::Cancelled` and answers with `Ctx::cancel`, which is
`nvs_safepoint`'s own status. ADR 0072 § 4's last row is now `Outcome::Cancelled` rather than an
unwind through the call, and `nvs_runtime::host`'s module doc carries why.

**Three residues, all deliberate and all documented at their site.** A cancellation is delivered
**once**, so a park site that ignores it is left parked (`net`, `channel` and `blocking` all
answer it, so only a future site could); `Drop for Scheduler` leaks a task it may not unwind
rather than aborting, which is ADR 0106's rule bought for one stack mapping per in-flight task at
the death of a worker; and `NvsStream`'s cancelled park answers `ErrorKind::ConnectionAborted`,
a policy no member reaches yet and which goal 4's `Core\Http\Client` may want to revisit.

**Orientation gaps.** `[context] adrs` still carries ADR 0072 §§ 4 and 5 only. `[context]
modules` has no pattern for `nvs-runtime/src/abi.rs`, `ctx.rs` or `host.rs`, all three of which
this session read again to find the helper ABI, the safepoint and the seam.

## Next group

**ADR 0072 § 4's remaining rows, as `.nvst` cases.** File set: `tests/conformance/core/` (the
new `a-cancelled-task-dies-where-it-sleeps.nvst` is the shape to copy), against
`crates/nvs-host/src/group.rs:246` (the second throw's diagnostic channel) and
`crates/nvs-stdlib/src/task.rs:262` (the `TimeoutError` row).

- [ ] **Two children throw: the first by completion order propagates and the second is
      written, never swallowed.** ADR 0072 § 4 row 3. `group.rs:246` is the `write_diagnostic`
      that has to show up, and no case asserts it today — a sibling that throws while a
      cancellation is in flight is the only way to reach it.
- [ ] **A `limit` asserted on both sides.** ADR 0072 § 3. `examples/tasks.nvs` pins the peak at
      2; what no case pins is that a `limit` of 1 serialises (peak 1, and the answers still come
      back in key order) and that a limit larger than the input is not a bound at all.
- [ ] **`Core\Task::all` over a field that throws**, so § 1's shape result and § 4's throw row
      meet: the sibling fields are cancelled, the shape is not built, and the throw the caller
      sees is the child's own class rather than a message copied out of it
      (`task.rs:258`, `ctx.raise`).

## Backlog

- `Core\Task\Channel` has no row in `nvs_stdlib::registry`; `crates/nvs-host/src/channel.rs` is
  the whole mechanism and item 11's language surface is what is left (`docs/plan/m5.md`).
- A write to a field of `Core\Task::all`'s result is checked against the *argument*'s field tag;
  the fix is `nvs-ir` recording the result shape's representations at the call site
  (`crates/nvs-stdlib/src/task.rs`'s module doc, gap 1).
- `[context] modules` in `docs/agent/loop-goal.toml` needs `nvs-runtime/src/{abi,ctx,host}.rs`
  and an `nvs-cli` pattern (this handoff's *Orientation gaps*).
- ADR 0072 § 6's `afterResponse` re-parenting is unwritten; `scheduler.rs`'s module doc names it
  as the one shape that outlives its spawning call.
