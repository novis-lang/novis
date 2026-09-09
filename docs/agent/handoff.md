# Handoff

## State

**Goal 24 — stage 4 is closed, and stage 5 with it.** `Core\Signal` is registered with one member,
`onShutdown(callable $handler): void`, and the stage-4 `cargo-named` check's three `-p nvs-stdlib`
names are green. `§16 Core\Signal` left `spec-classes-part-two-outstanding.txt`, which was the last
key stage 5 owed: `every_part_two_spec_class_is_registered` and `every_migration_member_is_registered`
both pass, and no outstanding file still names `Core\Net`, `Core\Os` or `Core\Signal`. Stage 6 —
`Core\Budget` — is the first open stage.

**The delivery is a store, and the handler is a safepoint.** `SafepointFlags::SHUTDOWN` is the new
bit; `nvs_safepoint` runs `Ctx::run_shutdown_handler` in the one branch there that is not a stop, so
the closure is entered between two Novis statements on the request's own stack. The delivery's whole
effect on state is `Drain::process().begin()` — `rule:concurrency/a-drain-closes-a-connection-cleanly`'s
drain and no second state machine — which is why the handler observes `Core\Server::isDraining()`
already `true`.

**Nothing installs an operating-system handler yet**, so nothing raises the bit outside tests. That
slice is `crates/nvs-cli/src/serve.rs:730`'s already-named one and belongs with the control socket
beside it; the backlog carries it. `crates/nvs-stdlib/src/signal.rs`'s module doc is the home of why
the class has no `isShuttingDown` of its own and no signal number anywhere in a signature.

## Next group

**Stage 6: `Core\Budget`'s three numbers** — one file set: `crates/nvs-stdlib/src/budget.rs` (new),
`crates/nvs-stdlib/src/lib.rs`, `crates/nvs-stdlib/src/registry.rs`, `crates/nvs-runtime/src/budget.rs`.

- [ ] **The class, its three members and its registration.** `memoryHeld`, `memoryPeak` and
      `memoryLimit` over the counter `crates/nvs-runtime/src/budget.rs:180` already keeps in every
      build ([0148](../decisions/0148.md) §§ 11-12, and `rule:programs/memory-priority` for what a
      reading may cost). Five edits per member, `docs/agent/conventions.md` § *A `Core` member*: the
      module joins the list at `crates/nvs-stdlib/src/lib.rs:191` and the address chain at
      `crates/nvs-stdlib/src/lib.rs:362`, the class the roster at
      `crates/nvs-stdlib/src/registry.rs:1497`. `crates/nvs-stdlib/src/signal.rs:75` is the neighbour
      to copy a handle-free, capability-free class's shape from, and `Core\Os::residentBytes` is the
      process reading that stays where it is — 0148 § 12 is why the two are not one member.
- [ ] **The high-water mark, recorded in `budget::add`.** `crates/nvs-runtime/src/budget.rs:180`,
      inside the branch that already tests for a positive delta, and a nested `Ctx` restores
      `max(enclosing, reached)` on drop rather than clobbering the mark of the request that spawned
      it (0148 § 15). The cost claim is measured in `benches/abi-probe`, never asserted.
- [ ] **Three `.nvst` cases, each a different question.** One allocates a known-size buffer, drops
      it, and reads a peak above the drop against a held figure below it; one asserts a parent's peak
      survives a child that allocated less; one asks `memoryLimit` with nothing configured.
      `tests/conformance/core/signal-registering-a-handler-begins-no-shutdown.nvst:1` is the shape,
      and `crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt:18` is the `§16 Core\Budget`
      line the same slice strikes.

## Backlog

- The operating-system half of the shutdown: nothing installs a signal handler or raises
  `SafepointFlags::SHUTDOWN` on running requests — `crates/nvs-cli/src/serve.rs:730` names the slice,
  beside the control socket that shares it.
- Stage 6's third item — `Script\ExitReport::memoryPeak`, the `nvs_request_memory_peak_bytes`
  histogram, and the `[limits] memory_high_water` fraction (0148 § 14) — is its own group after the
  class lands.
- `Core\Server`'s rest — the request's own environment and `traceId()` — waits on a served request
  carrying it; `crates/nvs-stdlib/src/server.rs`'s module doc owns that gap.
