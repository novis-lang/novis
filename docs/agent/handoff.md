# Handoff

## State

**ADR 0072's two members run end to end.** `nvs run` executes its program inside a task
(`crates/nvs-cli/src/main.rs:619`) — one `nvs_host::Scheduler`, a reactor installed over it,
`TaskRoot::Request`, and the `Ctx` read back off `take_finished()` — so § 1's "each is a child
of the calling task" has a calling task under a CLI program. `crates/nvs-stdlib/src/task.rs`
holds both bodies; that module's doc is the one home for the three steps they share, for why a
job's captures are *borrowed* rather than retained, and for the one known gap (a write to a
field of `all`'s result is checked against the argument's field tag, and the fix is `nvs-ir`
recording the result shape's representations at the call site).

**`examples/tasks.nvs` prints three of its four frozen lines.** `all=3`, `map=1,4,9,16` and
`deadline hit` are right; the fourth is `limit held at 1` against a frozen `limit held at 2`.
That is not a bug in the `limit` — it is `Core\Time::sleep` blocking the thread, so no two
children ever overlap and the peak gauge cannot exceed 1. **The acceptance check still fails
there**, and the next group is what closes it.

**The seam grew its second method and it has no caller yet.** `nvs_runtime::host::Host::sleep`
is right, `nvs_host::group::SchedulerHost` implements it in one line over `crate::timer::sleep`,
and `Core\Time::sleep` still calls `std::thread::sleep` — because a member that parks is a
parked task the scheduler tears down with a **forced unwind**, and a `nvs_helper!` frame is
`extern "C"`, which aborts. Both that method's doc and `nvs_stdlib::time`'s gap 3 carry the
slice. `nvs_runtime::run_helper` now re-raises a forced unwind instead of reporting it as a
helper `FATAL` (`crates/nvs-runtime/src/abi.rs:457`), which is what turned a silent corosensei
panic into a legible one.

**Orientation gaps.** `[context] adrs` still carries ADR 0072 §§ 4 and 5 only; §§ 1, 2 and 3
were sliced by hand again. `[context] modules` has no `nvs-cli` pattern and none for
`nvs-runtime/src/host.rs`, `nvs-runtime/src/abi.rs` or `nvs-runtime/src/object.rs`, all of
which this session had to read for the shape representation and the helper ABI.

## Next group

**Make a cancelled task with script frames die by status, then let `Core\Time::sleep` park.**
File set: `crates/nvs-host/src/scheduler.rs` (`tear_down:732`, `Waiting:152`),
`crates/nvs-host/src/timer.rs` (`park_until:249`), `crates/nvs-runtime/src/ctx.rs`
(`nvs_safepoint:1593`) and `crates/nvs-stdlib/src/time.rs` (`nvs_core_time_sleep:1722`, gap 3).

- [ ] **A parked task that cannot be unwound is resumed instead.** ADR 0072 § 5, ADR 0002.
      `tear_down` is the only `force_unwind` site; a task whose park happened under a helper
      frame has to be woken and told it is cancelled, and `park_until:249`'s
      `suspend_current(Waiting::Parked)` is where it learns. What it then answers is
      `nvs_safepoint`'s own cancel status, so no `catch` can see it (§ 5 runs no user code).
- [ ] **`Core\Time::sleep` parks.** Two lines — `nvs_runtime::host::with_current(|host|
      host.sleep(d))` with the `std::thread::sleep` fallback for a thread with no host —
      plus deleting gap 3. `examples/tasks.nvs`'s last frozen line is the test.
- [ ] **A `.nvst` case per member**, which is the item this session did not reach.
      `tests/conformance/core/`, and the six existing cases naming `Core\Task` are all
      `--EXPECTF-ERROR--`, so these are the first that *run* one. `all` over one field, `map`
      preserving a string key, and `limit: 0` refused as a `LogicError` are the three shapes.

## Backlog

- The shape-tag known gap: `nvs-ir` records the result shape's reprs at the call site —
  `crates/nvs-stdlib/src/task.rs`'s module doc owns the statement of it.
- `Core\Task\Channel` has no registry row; `crates/nvs-host/src/channel.rs` is the mechanism.
- ADR 0072 § 4's second-throw-to-`Core\Log` row is written by nobody —
  `crates/nvs-host/src/group.rs` drops it instead.
- Stage 5's graph copy reached twice (`docs/agent/loop-goal.toml`, stage 5).
- The crate-dependency-graph gap in `[context]`, still open from two sessions back.
