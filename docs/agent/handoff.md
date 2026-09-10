# Handoff

## State

**Goal `unowned-sweep`, stage 3 — landed.** All three `-p nvs-types` names the stage-3 check lists
are on disk in `crates/nvs-types/tests/arrays.rs` and pass. Nothing in the checker changed: the
element-covariant arm at `crates/nvs-types/src/expr/assign.rs:172` already recursed `is_assignable`
on the element type, and a union target is already satisfied member-wise at `:103`, so
`array<int>` → `array<int|string>` held before this session and the slice was the three tests that
pin it. Stages 1 and 2 stay landed; nothing is blocked.

The third test is the one worth knowing about. A `-p nvs-types` case reads `Diagnostics` back from a
compiled source string, so it cannot observe copy-on-write at run time; what it pins instead is the
compile-time half of the same soundness argument — the callee's write is checked against the
*parameter's* element type, and the caller's binding keeps the element type it declared, so
`int $n = $a[0];` still checks after the call and `string $s = $a[0];` is still exactly one mismatch.

**Stage 4 has none of its five names on disk.** Its first check is the panic hook, and no
`panic::set_hook` exists anywhere in `crates/nvs-runtime/src` outside `abi.rs`'s own `mod tests`
(grep over that tree, hits at `abi.rs:964`–`1024` only), so that hook is unwritten rather than
mislocated.

## Next group

**Stage 4: the panic hook writes to the request log, `rule:errors/helper-abi` and
`rule:http-server/containment`** — one file set: `crates/nvs-runtime/src/abi.rs` over
`crates/nvs-runtime/src/ctx/output.rs`. Containment does not change and the goal's § *Standing
decisions* freezes that: what the hook changes is where the message is written, never whether a
panic can be recovered.

- [ ] **`a_panic_on_a_served_request_is_written_to_the_request_log_with_its_request_id`** — the
      envelope filler that already owns `request_id` is
      `crates/nvs-runtime/src/ctx/output.rs:287` (`stamp_envelope`, and its doc says `request_id`
      *is* the trace id), and the record path it feeds is
      `crates/nvs-runtime/src/ctx/output.rs:229` (`write_log_record`). The hook has a `Ctx` to
      reach or it has nothing to stamp — settle that before writing the hook.
      `rule:errors/helper-abi`.
- [ ] **`a_cli_scripts_panic_still_reaches_stderr_through_the_default_hook`** — the containment
      point and the existing quiet-hook fixtures are `crates/nvs-runtime/src/abi.rs:964` and the
      `quietly` helper at `crates/nvs-runtime/src/abi.rs:1018`; a test that installs a hook must
      restore the previous one, since every test in the binary shares the process's.
      `rule:errors/helper-abi`.
- [ ] **`the_hook_changes_presentation_and_never_lets_a_panic_be_recovered`** — the task-root case
      it sits beside is `crates/nvs-runtime/src/abi.rs:1032`; assert the `FATAL` still arrives and
      the context is still ended, so the test fails if a hook ever swallows one.
      `rule:http-server/containment`.

## Backlog

- Stage 4's second check — `max_output` bounds a capture at both readers, `-p nvs-stdlib`, over
  `crates/nvs-stdlib/src/process.rs` and `crates/nvs-stdlib/src/io.rs`; ADR 0044 § 1 owns the cap.
- Stage 5 — `python tools/owners.py --unowned --check`, then the conformance and differential
  suites; `docs/agent/loop-goal.toml:6909`.
- Whether the *runtime* half of `array<T>` covariance has a `.nvst` case of its own is **not
  checked**; `rule:types/arrays` owns the value semantics if one is missing.
