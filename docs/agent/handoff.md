# Handoff

## State

**Goal 24 — stage 3 is closed.** `Core\Os` is on disk with all five of the spec's § 16 facts —
`pid`, `hostname`, `cpuCount`, `residentBytes`, `loadAverage` — and the stage-3 `cargo-named`
check's four `-p nvs-stdlib` names are green. `examples/os-facts.nvs` runs, and three `.nvst` cases
ask each member a different question.

**The syscalls are `nvs_runtime::os`'s, not this class's.** ADR 0118 § 2 forbids `nvs-stdlib` from
reaching the operating system itself and `nvs_stdlib_reaches_the_os_only_through_the_gate` holds it
there, so `crates/nvs-runtime/src/os.rs` is the platform half — the same move `terminal.rs` already
made. The root `Cargo.toml` gained one `windows-sys` feature for it, `Win32_System_ProcessStatus`.

Two decisions this landed, both recorded in `crates/nvs-stdlib/src/os.rs`'s module doc.
**`Core\Os` has no row in `registry::CAPABILITIES` at all** rather than five `None` rows: a class is
capability-bearing in that table's reading the moment any row names it, so five `None`s would claim
this class is a door. `Core\Path` is the precedent the closure test's own doc names. And
**`residentBytes` is a high-water mark** — `getrusage`'s `ru_maxrss`, the peak working set on
Windows — which is the reading ADR 0148 § 12 names, so it only ever grows and is never a request's
held bytes.

Stage 5 is part done: `§16 Core\Os` and the three `Core\Os::*` migration keys are struck, because a
line naming a registered class fails the same test as a missing one. Nothing is blocked.

## Next group

**Stage 4: `Core\Signal`, graceful shutdown and nothing else** — one file set:
`crates/nvs-stdlib/src/signal.rs` (new), `crates/nvs-stdlib/src/lib.rs`,
`crates/nvs-stdlib/src/registry.rs`, `crates/nvs-runtime/src/drain.rs`.

- [ ] **The class, its members and its registration.** `rule:core-api/tier-roster` places it
      (ADR 0051 § 3, "leaving only a narrow `Core\Signal` for graceful shutdown") and the goal's
      § Stage 4 is the surface: a handler for the terminating signals, no `kill`, no `alarm`, no
      signal number as an integer. Five edits per member, `docs/agent/conventions.md` § *A `Core`
      member*: the module joins the list at `crates/nvs-stdlib/src/lib.rs:237` and the address chain
      at `crates/nvs-stdlib/src/lib.rs:390`, the class the roster at
      `crates/nvs-stdlib/src/registry.rs:1730`. `crates/nvs-stdlib/src/os.rs:53` is the neighbour to
      copy a handle-free class's shape from, and it is also the answer to whether this one owes
      capability rows — read its module doc's second section before writing any.
- [ ] **The handler enters the drain that exists rather than a second state machine.**
      `rule:concurrency/a-drain-closes-a-connection-cleanly` is the machine;
      `crates/nvs-runtime/src/drain.rs:81` is `Drain::begin` and `:98` the process-wide reader
      `Core\Server::isDraining` already answers from. The delivery sets a flag a safepoint reads —
      `crates/nvs-runtime/src/ctx/safepoint.rs:173` is `nvs_safepoint`, which already polls for
      cancellation and is where a second flag belongs.
- [ ] **The three `-p nvs-stdlib` names the stage-4 check wants**, listed at
      `docs/agent/loop-goal.toml:6200`. They are the specification for the shape: a handler at a
      safepoint and never in a signal context, one drain rather than two, and no job-control
      surface.

## Backlog

- Stage 5's remaining strikes: `§16 Core\Net` and `§16 Core\Signal`, and the `socket_*`,
  `stream_socket_*`, `fsockopen`, `posix_*` and `pcntl_*` migration keys —
  `crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt`.
- Stage 6: `Core\Budget`'s three numbers and the recorded peak — ADR 0148 §§ 11-15, now reachable
  from the goal's `[context.stage.3]` overlay and owed a `[context.stage.6]` one.
- `Core\Os::loadAverage` has no `.nvst` case pinning its Windows message, because a frozen
  expectation cannot hold on both platforms; `load_average_throws_on_windows_with_a_message_naming_the_platform`
  is what asserts it instead.
