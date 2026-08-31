# Handoff

## State

**Goal 4 — `Core`'s capability-bearing half — is running. Stage 3's process half is closed and
its terminal half has not started.** `Core\Process::run` is the five-edit shape over
`nvs_runtime::capability::exec` (`crates/nvs-stdlib/src/process.rs:78`): the door is asked for
`process.exec` first and the target's kind second, and the member's whole body is the wait and
the capture. ADR 0044 § 1's `ProcessResult` is an instance class, `Core\Process\Result`
(`crates/nvs-stdlib/src/process.rs:166`) — three zero-argument members over three slots, with
`stdout`/`stderr` as `bytes` per that section and `exitCode` as `int`. That module's own docs own
why it is an instance and not a shape, and carry its two known gaps: the wait blocks the calling
worker thread (ADR 0044 § 5 wants the blocking pool), and `[limits] max_output` bounds nothing
yet (only the request's memory limit does, exactly as for `Core\IO::read`).

`examples/process.nvs` runs green against its three frozen lines, over an `[[app]]` block in
`nvs.toml` that scopes `process.exec` to `target/debug` and `examples/process` — both, because
the door asks the capability first and a grant naming only the binary would turn the fixture's
`.bat` line into a capability denial that still printed the same word.

Three fixtures still owe configuration their stage must write, unchanged: `examples/http.nvs`
names `http://127.0.0.1:8099` and stage 5 owes that origin; `examples/logging.nvs` needs an
`[[app]]` block naming `examples/logging/handler.nvs`; and every remaining fixture that reaches
the world still owes its `net.connect` grant.

`orient.py` still prints two dead `[context] modules` selectors — `crates/nvs-host/src/pool.rs`
and `crates/nvs-host/src/stream.rs` match no module. Nothing is blocked.

## Next group

**Stage 3's named checks over the member that now exists.** One file set:
`crates/nvs-stdlib/src/process.rs` (a `mod tests` it does not yet have),
`tests/conformance/reject/`, and `crates/nvs-types/src/core_lib.rs`.

- [ ] **`there_is_no_shell_string_form_of_run_or_spawn` and
      `a_windows_batch_or_powershell_target_is_refused`**, the two `-p nvs-stdlib` names the
      goal's stage 3 lists. Both are assertions over rows and over the door: the first reads
      `CLASS`'s `params` at `crates/nvs-stdlib/src/process.rs:78` and holds that no member of it
      takes a lone `CoreTy::Text` where a command line could go, the second drives
      `shell_target` at `crates/nvs-runtime/src/capability.rs:379` across the three extensions
      and their upper-case spellings. ADR 0044 §§ 1 and 4.
- [ ] **`tests/conformance/reject/there-is-no-shell-string-form-of-process-run.nvst`**, which
      the goal's stage 10 conformance list names by path. An `--EXPECTF-ERROR--` case:
      `Core\Process::run("sh -c 'echo hi'")` is an arity diagnostic and
      `Core\Process::run("sh", "-c echo hi")` a type one, against the row at
      `crates/nvs-stdlib/src/process.rs:78`. Freeze the diagnostic from a real run — its
      indentation widens with the line number.
- [ ] **`a_tainted_path_or_argv_element_is_a_compile_time_diagnostic` and
      `process_exec_is_deny_by_default`**, the two `-p nvs-types` names. The first belongs
      beside the qualifier sweeps at `crates/nvs-types/src/core_lib.rs:587`; the second reads
      `nvs_stdlib::registry::CAPABILITIES`' new row at
      `crates/nvs-stdlib/src/registry.rs:1132` against `nvs_config::Cap::ProcessExec`'s
      `RuntimeTighten` default. ADR 0044 § 6.

## Backlog

- `Core\Process::spawn` — ADR 0044 §§ 2-3's streamed half, over the same door.
- The wait moves to goal 2's blocking pool — ADR 0044 § 5, `crates/nvs-stdlib/src/process.rs`'s
  known gap 1.
- `[limits] max_output` bounds a capture — ADR 0044 § 1; nothing in the tree reads that
  directive yet.
- `Core\Cli` and `Core\Command` — stage 3's terminal half, ADR 0086, a different file set.
- The two dead `[context] modules` selectors in `docs/agent/loop-goal.toml`.
