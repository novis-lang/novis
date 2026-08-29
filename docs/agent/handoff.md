# Handoff

## State

**Goal 2's item 11 is closed end to end, and `examples/channel.nvs` prints its three frozen
lines.** The acceptance check that had been failing (`Core\Task\Channel` is not declared) is
closed: the class is registered, constructible, generic in `T` and iterable, and its `send`
suspends at the bound rather than growing the queue.

**Its queue is Novis values in the instance's own slots, not a handle into the host.** That is the
decision under everything else and its home is `crates/nvs-stdlib/src/channel.rs`'s module doc,
along with the two beside it: the object is its own iterator (`iterate()` answers the receiver,
`advance()` is where a consumer waits), and a state change wakes every waiter on the channel
rather than one. `nvs_runtime::host::Host` gained `waker()` and `park()` for it — a `Waker` is a
one-shot boxed closure, taken *before* the waited-on state is released, and `None` from `waker()`
is how a member learns there is no task beneath it and reports instead of blocking a core.

**One residue, deliberate.** A `send` on a closed channel is a `Fault::fatal` rather than a
throw, because there is no `Core` exception class this crate reaches for and ADR 0020's ladder
has not been applied to this surface; a session that gives channels a throwable failure mode
should decide it at the ladder rather than here.

**Orientation gaps.** `[context] adrs` still carries ADR 0072 §§ 4 and 5 only. `[context] modules`
has no pattern for `nvs-runtime/src/host.rs`, `nvs-stdlib/src/instance.rs`, `cursor.rs`, `heap.rs`
or `registry.rs` — the five files a new `Core` instance class is assembled from, all of which this
session read to find the four rosters (`CLASSES`, `CONSTRUCTORS`, `GENERIC_CLASSES`, `ITERABLES`)
plus `instance.rs`'s dispatch roster.

## Next group

**ADR 0072 § 4's remaining rows, as `.nvst` cases.** File set: `tests/conformance/core/`
(`a-channel-of-one-loses-no-value-and-reorders-none.nvst` is the newest `Task::all` shape to
copy), against `crates/nvs-host/src/group.rs:279` and `:454` (the two `write_diagnostic` calls)
and `crates/nvs-stdlib/src/task.rs:263` (the `TimeoutError` row).

- [ ] **Two children throw: the first by completion order propagates and the second is written,
      never swallowed.** ADR 0072 § 4 row 3. `crates/nvs-host/src/group.rs:279` and `:454` are the
      two `write_diagnostic` calls that have to show up, and no case asserts either today — a
      sibling that throws while a cancellation is in flight is the only way to reach them.
- [ ] **A `limit` asserted on both sides.** ADR 0072 § 3. `examples/tasks.nvs` pins the peak at
      2 from one side only; the case that pins it from the other is a `limit` of *n* over *n+1*
      children, where the (n+1)th cannot have started before one of the first *n* returned.
- [ ] **`Core\Task::all` over a field that throws**, so § 1's shape result and § 4's throw row are
      asserted by one case: `crates/nvs-stdlib/src/task.rs:263`'s neighbourhood is the member end.

## Backlog

- A `send` on a closed channel has no throwable spelling — ADR 0020's ladder, if it should.
- `Core\Task\Channel` has no `.nvst` case for a *cancelled* waiter; `nvs-host`'s own channel does.
- ADR 0072 § 4's deadline row over a channel park (a consumer waiting past a group's deadline).
- M5's `Core\Task::map` near-linear speedup benchmark — `docs/plan/m5.md` § *Verify*.
- The 1000-case conformance corpus count — M4's residue, `docs/implementation-plan.md`.
- `docs/spec/01-core-library.md` has no § naming `Core\Task\Channel`; the registry is its only home.
