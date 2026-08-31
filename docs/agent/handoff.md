# Handoff

## State

**Goal 4 — `Core`'s capability-bearing half — is running, and ADR 0086 § 6's dispatching half is
open at its last member.** `Core\Command::run(): uint` matches this process's command line against
the table `#[Command]` built while compiling and calls the handler it names, so § 6's one deliberate
divergence from ADR 0077 — this table *dispatches* — is on disk with four conformance cases over it.

**How a handler is reached was the decision, and it needed nothing new.**
`nvs_runtime::call_static` (`crates/nvs-runtime/src/dispatch.rs:128`) splits the row's
`Class::method` label, asks `Ctx::class_desc` for the class and its descriptor's method table for the
address: `nvs_types::layout::ClassLayout::methods` lists every method with a body, `static` ones
included, so the address was already there and no label map had to be installed beside the table.
That function's own doc is the home of the reasoning, including the rejected compile-time expansion.

**Two things had to cross that had not.** `nvs_types::commands::ArgConv` — a closed set of five
decided by `conversion_of` (`crates/nvs-types/src/commands.rs:158`) in the same walk that already
answers § 6's third compile error — is which conversion a parameter needs, and that module's gap 1
now states why the conversion crosses and the type does not. And the argument vector is `Ctx`'s,
written before the program runs: `nvs run <file> [args...]` collects the trailing words
(`trailing_var_arg`, so a program's own `--verbose` is never answered by `nvs run`), a bundled
executable passes `std::env::args().skip(1)`, and § 13's `Core\Cli::arguments` should read
`Ctx::command_line` rather than `std::env`.

**`.nvst`'s `--ARGS--` is honoured now**, one argument per line and no shell splitting
(`crates/nvs-test/src/case.rs`'s `Case::args`) — its `NOT_YET` entry named argv as the blocker, and
argv is what this session landed. `--INI--` and `--ENV--` are still deferred.

**Three gaps are named where they bite:** `crates/nvs-stdlib/src/command.rs`'s gap 3 — an option that
is not a flag is *required*, because a declared default is folded at the call site and the row
carries none — its gap 1, `completions`, and `nvs_runtime::commands`' gap 1, the four `ArgConv`
variants that are `Unconverted` and throw a `LogicError` when a command line reaches one.

**The driver's acceptance failure is not a regression.** `examples/crypto.nvs` fails on
`Core\Password::hash`, which is stage 4's fixture waiting on stage 4; a program leg runs before every
cargo-named check, so it will mask the rest of the list until crypto lands. Three fixtures still owe
configuration their stage must write, unchanged: `examples/http.nvs` needs stage 5's
`http://127.0.0.1:8099` origin, `examples/logging.nvs` an `[[app]]` block naming
`examples/logging/handler.nvs`, and every remaining fixture that reaches the world its `net.connect`
grant. `orient.py` still prints two dead `[context] modules` selectors —
`crates/nvs-host/src/pool.rs` and `crates/nvs-host/src/stream.rs` — and wants
`crates/nvs-runtime/src/commands.rs`, `crates/nvs-runtime/src/dispatch.rs` and
`crates/nvs-test/src/case.rs` added. Nothing is blocked.

## Next group

**§ 6's last member and the two holes under the one that landed.** One file set:
`crates/nvs-stdlib/src/command.rs`, `crates/nvs-stdlib/src/cli.rs`,
`crates/nvs-runtime/src/commands.rs` and `crates/nvs-types/src/commands.rs`.

- [ ] **`command_run_dispatches_through_the_compiled_table`** — stage 3's named check, and the one
      thing this session did not reach: the dispatch is asserted by conformance cases and by no Rust
      test. It has to build the table by hand — the shape is
      `crates/nvs-stdlib/tests/allocation_policy.rs:286`'s `closure_of` (a leaked `ClassTable`, a
      `MethodRow` whose `code` is a plain `unsafe extern "C" fn`) plus `Ctx::set_runtime_error_class`,
      since `crates/nvs-runtime/src/ctx.rs:2466`'s `class_desc` reads the table through *that*
      handle. The helper is `crates/nvs-stdlib/src/command.rs:215` and is crate-private, so the test
      belongs in that module's own `mod tests`. ADR 0086 § 6.
- [ ] **Let a declared default cross, and drop gap 3** — `#[Option] uint $retries = 3` is refused
      today when it is not written. The row needs a folded constant per argument beside
      `crates/nvs-types/src/commands.rs:158`'s `conv` (`crate::defaults::literal_default` folds one),
      its twin in `crates/nvs-runtime/src/commands.rs:36`, and the arm in
      `crates/nvs-stdlib/src/command.rs:334`'s `fill` that fills an unwritten option from it.
      ADR 0086 § 6.
- [ ] **`completions(Cli\Shell $shell): string` and `Cli\Shell`** — four shells generated from the
      same table `help` reads, so what it waits on is the enum, beside the two in
      `crates/nvs-stdlib/src/cli.rs:70`. Closes
      `help_and_completions_are_generated_from_the_same_table`. ADR 0086 § 6.

## Backlog

- Stage 4 is what the driver's acceptance check is stopped on — `docs/agent/loop-goal.toml` § 4.
- `Core\Cli::arguments` (§ 13) should read `Ctx::command_line` — `crates/nvs-stdlib/src/cli.rs`'s gap 3.
- A usage page goes to the diagnostic channel until § 3's `write` lands — `command.rs`'s `usage`.
- `examples/http.nvs`, `examples/logging.nvs` and the `net.connect` grants — the plan's *Open now*.
