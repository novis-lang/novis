# Handoff

## State

**Stage 4's seam is decided, recorded and on disk; both ends of it are still empty.**
`crates/nvs-runtime/src/host.rs` is the route a `Core` member reaches its host through — a
`&'static dyn Host` in a thread-local, `install`/`with_current`/`is_installed`, and one method
`Host::run_group` (`host.rs:154`) whose `Outcome` is ADR 0072 § 4's table. That module's doc is the
decision's one home and carries all three parts of it: why the edge is inverted through
`nvs-runtime` instead of `nvs-stdlib` depending on `nvs-host`, why a thread-local beats a second
opaque pointer in `Ctx`, and why what crosses is a whole group rather than a `spawn`/`wait`/`cancel`
the member sequences. `scheduler.rs`'s module doc points at it; the plan's *Open now* summarises it.

**Nothing implements `Host` and nothing installs one.** The trait has no implementor at this commit
by design — the seam was this slice, the body is the next. Three tests in `host.rs` pin the route
itself (absent answers `None`, an installed host is reachable from a free function with no argument
carrying it, installing nests and restores).

**`nvs-cli` does not depend on `nvs-host` at all.** No shipped binary links the host crate, so
`nvs run examples/tasks.nvs` has no scheduler beneath it and would find no host installed even with
both bodies written. That is a real, separate slice and it is in the group below.

**`examples/tasks.nvs` still aborts at the placeholder** (`crates/nvs-stdlib/src/task.rs:135`), whose
message now names the seam rather than claiming no route exists. The acceptance check fails there and
will keep failing until the group below is through.

**Orientation gaps.** `[context] adrs` still carries ADR 0072 §§ 4 and 5 only; §§ 1, 2 and 3 are what
the remaining Stage 4 items are specified by and are still sliced by hand. Separately, this session
needed the **crate dependency graph** — which crates depend on `nvs-stdlib` and on `nvs-host` — and
the pack has no selector that prints it; `[context] modules` gives module first sentences, not edges.
That question decided part 1 of the seam, so a `Cargo.toml`-reading selector would earn its place.

## Next group

**Fill the seam in, from the host end to the member.** File set: `crates/nvs-runtime/src/host.rs`
(`Host:145`, `run_group:154`, `with_current:199`), `crates/nvs-host/src/scheduler.rs`
(`Scheduler::run:631`, `spawn_child:893`, `install_tree:323`), `crates/nvs-cli/src/runner.rs:189`,
and `crates/nvs-stdlib/src/task.rs` (`CLASS:81`, `address:118`, the placeholder at `:135`).

- [ ] **`impl Host` on the host side, and install it in `Scheduler::run`.** ADR 0072 §§ 1, 3, 4.
      Install beside `install_tree` (`scheduler.rs:323`) so the two guards have the same life. The
      one open design question is **what a child gets for a `Ctx`**: `spawn_child` (`:893`) takes an
      owned one, and § 1's children share the request. Decide it in `scheduler.rs`'s module doc, which
      already owns the task tree's design.
- [ ] **`nvs run` runs its program inside a task.** `nvs-cli` gains a `nvs-host` dependency and
      `runner.rs:189`'s `Ctx` is built inside a `Scheduler` rather than beside one. Without this the
      acceptance check's own program has no host no matter what the members do.
- [ ] **`nvs_core_task_all`'s body** — § 1's fields become `host::Job`s, one per field, and the
      answer is a shape with the same field names. `nvs_runtime::call_closure`
      (`crates/nvs-runtime/src/closure.rs:132`) is what a job calls.
- [ ] **`nvs_core_task_map`'s body** — § 2's array, "preserving the input's keys and order", over one
      shared callback. Same seam call, different job construction.

## Backlog

- No `Core\Task\Channel` row in `nvs_stdlib::registry` — item 11's residue (`docs/agent/loop-goal.md`).
- § 6's `afterResponse` re-parenting onto the request tree — `scheduler.rs`'s module doc names it.
- `[deferred]`/`[limits]`/`script.spawn` enforcement is goal 3's, not this one.
- M4's 1000-case conformance corpus count, as the suite grows (`docs/plan/m4.md`).
