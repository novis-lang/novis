# Handoff

## State

**Goal `unowned-sweep`, stage 4, first half — landed.** All three `-p nvs-runtime` names the check
lists are on disk in `crates/nvs-runtime/src/floor.rs` and pass. `floor::install_panic_hook` is the
hook and `floor::report_panic` is the half a test can ask a question of, because a `PanicHookInfo`
cannot be constructed; `crates/nvs-cli/src/main.rs:849` installs it as `main`'s first statement, before
the bundle footer read, so a bundled application carries it too. `crates/nvs-runtime/src/lib.rs`'s
known gap 4 is struck — the list keeps its holes rather than renumbering, as it already did at 3.

The hook reaches its context through `ctx::current`'s thread-local, the door
`crate::object::dismantle` already uses: a hook's signature carries no context, and that thread-local
is the only one there is. The claim test is `Ctx::inbound`, which is exactly what `stamp_envelope`
gates on — so a `nvs run` panic falls through to the hook installed before this one and still reaches
stderr, and `rule:http-server/containment-does-not-end-at-the-helper` is untouched: the hook runs
before the unwind starts and returns, so `run_helper` and `run_task` contain a panic as they did.

**Stage 4's second check has neither of its names on disk, and `max_output` has no reader anywhere in
the tree** — `peek.py --locate max_output` finds it only in two decision records, so what is missing
first is the directive reader beside `Ctx::memory_limit`, not a call site that forgot to ask.

## Next group

**Stage 4: `[limits] max_output` bounds a capture, at both readers, `rule:core-classes/process-run`** —
one file set: `crates/nvs-stdlib/src/process.rs` over `crates/nvs-stdlib/src/io.rs`, with the
directive's reader in `crates/nvs-runtime/src/ctx/limits.rs`. ADR 0044 § 1 is already in the pack and
says the shape: the directive is reused rather than a cap added, and a child that writes past it is
killed with `run()` throwing, "the same shape an over-large response body already gets".

- [ ] **The directive gets a reader.** `crates/nvs-runtime/src/ctx/limits.rs:50` is `memory_limit`,
      the shape a second limit takes, and `crates/nvs-runtime/src/ctx/limits.rs:60` its setter. Nothing
      reads `[limits] max_output` today, so settle there whether it is bytes-per-capture or
      bytes-per-request before either call site asks — the two members share one directive and
      `rule:config/three-changeability-classes` owns which class it is.
- [ ] **`a_child_whose_output_exceeds_limits_max_output_is_bounded_rather_than_unbounded`** —
      `crates/nvs-stdlib/src/process.rs:325` is `nvs_core_process_run`, and that module's known gap 1
      at `crates/nvs-stdlib/src/process.rs:59` is what closes with it.
      `rule:core-classes/process-run`.
- [ ] **`core_io_read_is_bounded_by_the_same_directive_and_the_same_signature`** —
      `crates/nvs-stdlib/src/io.rs:1969` is `nvs_core_io_read`. The gap above says the same signature
      closes both, so both land here or neither does.

## Backlog

- Stage 5 rewrites `docs/agent/carried-gaps.md` § *Unowned* to what survives — `docs/agent/loop-goal.md`
  § *Stage 5* owns what is expected to.
- `carried-gaps.md:63`'s panic-hook row is stale now that the gap is struck; stage 5's rewrite is where
  it goes.
- `[context] modules` names `crates/nvs-runtime/src/lib.rs` for stage 4 but not
  `crates/nvs-runtime/src/floor.rs`, which is where the hook actually landed.
