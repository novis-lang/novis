# Handoff

## State

**Goal 4 — `Core`'s capability-bearing half — is running. Stage 3's process half is closed, including
the four checks the goal names for it.** `Core\Process::run` is the five-edit shape over
`nvs_runtime::capability::exec`; `crates/nvs-stdlib/src/process.rs`'s module doc owns why the result
is an instance and carries its two known gaps. The four checks are green:
`there_is_no_shell_string_form_of_run_or_spawn` and `a_windows_batch_or_powershell_target_is_refused`
in that module's new `mod tests`, `a_tainted_path_or_argv_element_is_a_compile_time_diagnostic` and
`process_exec_is_deny_by_default` beside the qualifier sweeps in `crates/nvs-types/src/core_lib.rs`,
plus `tests/conformance/reject/there-is-no-shell-string-form-of-process-run.nvst`.

**Stage 3's terminal half has not started, and `examples/cli.nvs` is now honest about what it needs.**
The fixture had never compiled for a reason unrelated to its stage — the playbook's new bullet owns
that — and now fails on exactly `Core\Command::help` and `Core\Cli::width`. The compiled command
table it reads from already exists: `nvs_types::commands::CommandTable`, recorded on `ExprTypeTable`.

Three fixtures still owe configuration their stage must write, unchanged: `examples/http.nvs` names
`http://127.0.0.1:8099` and stage 5 owes that origin; `examples/logging.nvs` needs an `[[app]]` block
naming `examples/logging/handler.nvs`; every remaining fixture that reaches the world still owes its
`net.connect` grant. `orient.py` still prints two dead `[context] modules` selectors —
`crates/nvs-host/src/pool.rs` and `crates/nvs-host/src/stream.rs` match no module. Nothing is blocked.

## Next group

**Stage 3's terminal half, worked inward from the fixture's two missing members.** One file set:
`crates/nvs-stdlib/src/cli.rs`, `crates/nvs-stdlib/src/registry.rs` and
`crates/nvs-types/src/expr_table.rs`.

- [ ] **`Core\Cli::width`, and § 3's once-per-process terminal facts under it** — stream, tty-ness,
      colour depth and width resolved once for the process and cached, so two reads are the same
      number by construction rather than by luck. The class goes beside the carrier at
      `crates/nvs-stdlib/src/cli.rs:55`, its rows into `crates/nvs-stdlib/src/registry.rs:984`.
      Closes `tty_colour_depth_and_width_resolve_once_per_process` and the third frozen line of
      `examples/cli.nvs`. ADR 0086 § 3.
- [ ] **`Core\Command::help(?string $name): Cli\Text`, off the table that already exists** —
      `nvs_types::commands::CommandTable` at `crates/nvs-types/src/commands.rs:163`, reached through
      `crates/nvs-types/src/expr_table.rs:1075`'s `commands()`. **First decide the direction**:
      `nvs-types` depends on `nvs-stdlib`, so a native member cannot read that table, and § 6's
      "generated from the table" therefore has to be a compile-time fold (ADR 0057) or a lowered
      constant. Closes `help_and_completions_are_generated_from_the_same_table` and the fixture's
      second line. ADR 0086 § 6.
- [ ] **`terminal_output_substitutes_a_control_sequence_visibly` and
      `styling_is_a_value_type_and_never_a_grammar`**, over the carrier at
      `crates/nvs-stdlib/src/cli.rs:55` — assertions over the sink both members above write through,
      once they exist. ADR 0086 §§ 1-2.

## Backlog

- `Core\Process::spawn` — ADR 0044 §§ 2-3's streamed half, over the same door.
- The wait moves to goal 2's blocking pool — ADR 0044 § 5, `crates/nvs-stdlib/src/process.rs`'s
  known gap 1; it is the one stage 3 process check still open.
- `[limits] max_output` bounds a capture — ADR 0044 § 1; nothing in the tree reads that directive.
- `Core\Command::run`'s dispatch and § 4's prompts — the four stage 3 `Core\Cli` names left over.
- The two dead `[context] modules` selectors in `docs/agent/loop-goal.toml`.
