# Handoff

## State

**Goal 4 — `Core`'s capability-bearing half — is running. Stage 3's terminal *profile* is closed;
its sink half and `Core\Command` are not.** ADR 0086 § 3's four facts are `Core\Cli::isTty`,
`::width`, `::height` and `::colorDepth`, four registry rows over `nvs_runtime::terminal`, which
resolves stream tty-ness, the window size and the colour depth into one `OnceLock` and hands back
reads of it. That module's own doc comment owns the caching, what it spends, and why **no
capability gates it** — the streams are open before the program runs a line, so reporting how wide
one is grants nothing. `crates/nvs-stdlib/src/cli.rs` holds the surface and the ordinal mapping
between the runtime's Rust `ColorDepth` and `Core\Cli\ColorDepth`; its gaps 2 and 3 own what § 13
still owes.

`tty_colour_depth_and_width_resolve_once_per_process` is green in that module's `mod tests`, beside
three `.nvst` cases under `tests/conformance/core/cli-*.nvst`. `libc` and `windows-sys` are now
`nvs-runtime` dependencies for the two window-size calls — no new crate entered the graph, only the
`Win32_System_Console` feature.

**`examples/cli.nvs` now fails on `Core\Command::help` alone**; its third frozen line's members all
resolve. The compiled table `help` reads exists but is **not public**:
`nvs_types::commands` is `pub(crate)` (`crates/nvs-types/src/lib.rs:201`).

Three fixtures still owe configuration their stage must write, unchanged: `examples/http.nvs` names
`http://127.0.0.1:8099` and stage 5 owes that origin; `examples/logging.nvs` needs an `[[app]]`
block naming `examples/logging/handler.nvs`; every remaining fixture that reaches the world still
owes its `net.connect` grant. `orient.py` still prints two dead `[context] modules` selectors —
`crates/nvs-host/src/pool.rs` and `crates/nvs-host/src/stream.rs` match no module; add
`crates/nvs-runtime/src/terminal.rs` to that manifest, since this session had to name it blind.
Nothing is blocked.

## Next group

**`Core\Command`, worked outward from the table the compiler already builds.** One file set:
`crates/nvs-types/src/commands.rs`, `crates/nvs-types/src/expr_table.rs`,
`crates/nvs-stdlib/src/registry.rs` and a new `crates/nvs-stdlib/src/command.rs`.

- [ ] **`Core\Command::help(?string $name): Cli\Text`, off the table that already exists** — the
      `CommandTable` at `crates/nvs-types/src/commands.rs:163`, read back through
      `crates/nvs-types/src/expr_table.rs:1075`, has to become reachable from a running program
      first: `crates/nvs-types/src/lib.rs:201` declares the module `pub(crate)`. Register the class
      beside `crate::cli::CLASS` at `crates/nvs-stdlib/src/registry.rs:984`. Closes the second
      frozen line of `examples/cli.nvs`, `usage: greet <name> [--loud]`. ADR 0086 § 6.
- [ ] **`command_run_dispatches_through_the_compiled_table`** — `Core\Command::run()` is the
      dispatching entry point, unlike `Core\Router`, because a CLI has one. Same table at
      `crates/nvs-types/src/commands.rs:163`; the row goes in the same `CLASS` the slice above
      declares, registered at `crates/nvs-stdlib/src/registry.rs:984`. ADR 0086 § 6.
- [ ] **`help_and_completions_are_generated_from_the_same_table`** — the check that the usage page
      and the shell completions are two renderings of one table rather than two writers that agree
      today. Add the three `.nvst` cases each new member owes while the renderings are open:
      `crates/nvs-stdlib/tests/conformance_coverage.rs:268`'s `BELOW_THE_FLOOR` is empty, so a
      member with fewer than three fails `cargo test -p nvs-stdlib`. ADR 0086 § 6.

## Backlog

- Stage 3's sink half — `Core\Cli::write`, `displayWidth`, and § 1's substitution table on `echo`,
  which is `crates/nvs-runtime/src/helpers.rs:1649`'s `value_to_string`. `crates/nvs-stdlib/src/cli.rs`'s
  gap 2 owns the split and gap 1 the sink's own debt.
- `styling_is_a_value_type_and_never_a_grammar` — `Cli\Style`/`Cli\Color` beside the carrier at
  `crates/nvs-stdlib/src/cli.rs:84`. ADR 0086 § 2.
- § 4's prompts and § 5's scoped regions — four more named checks in stage 3's `cargo-named` list.
  ADR 0086 §§ 4, 5, 8.
- `displayWidth` owes a UAX #11 table this tree does not carry; ADR 0051 § 4 pre-authorizes picking
  the crate. `docs/agent/loop-goal.md` § *Standing decisions*.
- The three fixture configuration debts in `## State`, each owed by its own stage.
- `docs/agent/loop-goal.toml`'s `[context] modules` has two dead selectors and is missing
  `nvs-runtime`'s new `terminal` module.
