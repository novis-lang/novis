# Handoff

## State

**Goal 4 — `Core`'s capability-bearing half — is running, and stage 3 is closed.** ADR 0086 § 6's
`Core\Command::help(?string $name): Cli\Text` is a registry row over the table the compiler already
built, so `examples/cli.nvs` runs green against all three of its frozen lines.

**The channel the table crosses is the new thing, and it is what § 6's other two members will use.**
`nvs_types::commands` is now `pub`; `nvs-cli`'s `run_run` reads `checked.exprs.commands()`, converts
the rows to `nvs_runtime::commands::CommandTable` — strings only, no compiler type below the
boundary — and installs it with `Ctx::set_commands` before the program starts, exactly as the
configuration snapshot is. That module's own doc owns why the rows cross at run time instead of
folding while checking (`help`'s `?string` and `run`'s argv are both runtime values), and
`nvs_stdlib::command`'s doc owns the page's layout, which § 6 leaves undecided.

**Only `nvs run` installs a table.** `nvs test`'s runner (`crates/nvs-cli/src/runner.rs:248`) does
not, so a `#[Test]` calling `help` sees the "declares no command" page; `.nvst` cases are unaffected
because `nvs test` runs each one through the `nvs run` path. Gaps 1 and 2 of
`crates/nvs-stdlib/src/command.rs` own what § 6 still owes: `run`, `completions`, and why a page
carries no defaults or types.

Three fixtures still owe configuration their stage must write, unchanged: `examples/http.nvs` names
`http://127.0.0.1:8099` and stage 5 owes that origin; `examples/logging.nvs` needs an `[[app]]`
block naming `examples/logging/handler.nvs`; every remaining fixture that reaches the world still
owes its `net.connect` grant. `orient.py` still prints two dead `[context] modules` selectors —
`crates/nvs-host/src/pool.rs` and `crates/nvs-host/src/stream.rs` — and now wants
`crates/nvs-runtime/src/terminal.rs` and `crates/nvs-runtime/src/commands.rs` added to that
manifest. Nothing is blocked.

## Next group

**§ 6's dispatching half, over the table that now reaches the runtime.** One file set:
`crates/nvs-stdlib/src/command.rs`, `crates/nvs-runtime/src/commands.rs`,
`crates/nvs-cli/src/main.rs` and whichever of `crates/nvs-ir/src/lower/expr.rs` the first item
decides on.

- [ ] **Decide how a handler is reached, then write `Core\Command::run(): uint`** — a native helper
      cannot reach a *static* method: `crates/nvs-runtime/src/dispatch.rs:42` is receiver-keyed, and
      the only label→address lookup is the unit's own, used once at
      `crates/nvs-cli/src/main.rs:711`. So the two candidates are a compile-time expansion in
      `nvs-ir` beside `Core\Program::implementing`'s, or a label lookup installed on the `Ctx` next
      to the table at `crates/nvs-runtime/src/ctx.rs:1167`. It also needs the process argument
      vector, which nothing exposes yet — `crates/nvs-stdlib/src/cli.rs:64`'s gap 3 is where § 13's
      `arguments` would go. Register the member beside `help` at
      `crates/nvs-stdlib/src/command.rs:72`. ADR 0086 § 6.
- [ ] **`command_run_dispatches_through_the_compiled_table`** — stage 3's named check over the
      member above, plus the `.nvst` that runs a command by name. The table lookup it asserts over
      is `crates/nvs-runtime/src/commands.rs:108`.
- [ ] **`help_and_completions_are_generated_from_the_same_table`** — `completions(Cli\Shell $shell):
      string` for bash, zsh, fish and pwsh, and the check that it and the usage page read one table.
      The `Cli\Shell` enum goes beside the two at `crates/nvs-stdlib/src/cli.rs:181`; the renderers
      go beside `crates/nvs-stdlib/src/command.rs:225`.

## Backlog

- `Core\Cli`'s sink half — `write`, `displayWidth`, `Text::plain`/`styled`, `Cli\Style`/`Color`:
  `crates/nvs-stdlib/src/cli.rs`'s gaps 2 and 3.
- `examples/http.nvs` and `examples/logging.nvs` still owe their configuration —
  `docs/agent/loop-goal.toml` stages 5 and 6.
- A `#[Test]` cannot see a command table: `crates/nvs-cli/src/runner.rs:248` installs none.
- Spec § 13's `arguments`, `escape` and the prompt members — `docs/plan/m8.md`.
