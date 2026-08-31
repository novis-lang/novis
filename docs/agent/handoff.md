# Handoff

## State

**Goal 4 — `Core`'s capability-bearing half — is running, and ADR 0086 § 6's dispatching half is
closed.** `Core\Command::run(): uint` matches this process's command line against the table
`#[Command]` built while compiling and calls the handler it names, and stage 3's named check
`command_run_dispatches_through_the_compiled_table` now asserts that in Rust, over a class table the
test builds by hand rather than through a compiler. Beside it,
`a_handler_the_program_does_not_declare_is_a_logic_error` pins `call_static`'s `None` arm — the one
`run` failure that is a mistake in the program rather than in its command line.

**How a native test reaches a compiled handler is the lore that came out of it**, and it is a
playbook bullet now: `Ctx::class_desc` reads the class table through
`Ctx::set_runtime_error_class`'s handle, so a dispatch test installs an `ErrorClass` over a class
that has nothing to do with spec § 10. `crates/nvs-stdlib/src/command.rs:845`'s `dispatching` is
the fixture and needs no leak, since the handle owns the `Rc`.

**Three gaps are still named where they bite:** `crates/nvs-stdlib/src/command.rs`'s gap 3 — an
option that is not a flag is *required*, because a declared default is folded at the call site and
the row carries none — its gap 1, `completions`, and `nvs_runtime::commands`' gap 1, the four
`ArgConv` variants that are `Unconverted` and throw a `LogicError` when a command line reaches one.

**The table's crossing point is a fifth file the last handoff's file set did not name.**
`crates/nvs-cli/src/main.rs:682`'s `runtime_commands` is where `nvs_types::commands::CommandTable`
becomes `nvs_runtime::commands::CommandTable`; `nvs-ir` never sees a command row at all. Any field
added to a row crosses there and nowhere else.

**The driver's acceptance failure is not a regression.** `examples/crypto.nvs` fails on
`Core\Password::hash`, which is stage 4's fixture waiting on stage 4; a program leg runs before
every cargo-named check, so it will mask the rest of the list until crypto lands. Three fixtures
still owe configuration their stage must write, unchanged: `examples/http.nvs` needs stage 5's
`http://127.0.0.1:8099` origin, `examples/logging.nvs` an `[[app]]` block naming
`examples/logging/handler.nvs`, and every remaining fixture that reaches the world its `net.connect`
grant. `orient.py` still prints two dead `[context] modules` selectors —
`crates/nvs-host/src/pool.rs` and `crates/nvs-host/src/stream.rs` — and wants
`crates/nvs-runtime/src/commands.rs`, `crates/nvs-runtime/src/dispatch.rs`,
`crates/nvs-test/src/case.rs` and `crates/nvs-cli/src/main.rs` added. Nothing is blocked.

## Next group

**§ 6's two remaining holes.** The first spans one more crate than the last handoff said — the file
set is `crates/nvs-types/src/commands.rs`, `crates/nvs-cli/src/main.rs`,
`crates/nvs-runtime/src/commands.rs` and `crates/nvs-stdlib/src/command.rs`; the second stays in
`crates/nvs-stdlib/src/command.rs` and `crates/nvs-stdlib/src/cli.rs`.

- [ ] **Let a declared default cross, and drop gap 3.** § 6's own example writes
      `#[Option] uint $retries = 3` and today the row carries no default, so `fill` makes the option
      *required*: `crates/nvs-stdlib/src/command.rs:391`'s `(true, _)` arm is the refusal to
      replace. The row is built twice in one walk —
      `crates/nvs-types/src/commands.rs:492` (positional) and
      `crates/nvs-types/src/commands.rs:527` (option) — so that is where the parameter's default
      expression is in scope; it crosses at `crates/nvs-cli/src/main.rs:682`'s `runtime_commands`
      into `crates/nvs-runtime/src/commands.rs:78`'s `CommandArg`. **Decide and record** whether a
      non-literal default is a compile error or simply not carried: carrying the *text* and running
      it back through `crates/nvs-stdlib/src/command.rs:411`'s `convert` is the shape that needs no
      second copy of the type. ADR 0086 § 6.
- [ ] **`completions(Cli\Shell $shell): string` and `Cli\Shell`** — four shells generated from the
      same table `help` renders, which is `crates/nvs-stdlib/src/command.rs:517`'s `overview` and
      `crates/nvs-stdlib/src/command.rs:493`'s `page_for` read a second way. The enum belongs beside the two `Cli` already has:
      `crates/nvs-stdlib/src/cli.rs:194`'s `Cli\Stream` and `crates/nvs-stdlib/src/cli.rs:228`'s
      `Cli\ColorDepth` are the `CoreEnum` + `EnumDoc` shape. The member's five edits land in
      `crates/nvs-stdlib/src/command.rs:76` (the rows), `:111` (the cards) and `:151`
      (`address`). ADR 0086 § 6, and the module's own gap 1.

## Backlog

- The four `ArgConv::Unconverted` variants — `decimal`, an enum, a literal union, `Core\Uuid` —
  `crates/nvs-runtime/src/commands.rs`' gap 1.
- `--INI--` and `--ENV--` in `.nvst`, still `NOT_YET` — `crates/nvs-test/src/case.rs`.
- `Core\Cli::arguments` should read `Ctx::command_line`, not `std::env` — ADR 0086 § 13.
- Stage 4 (`Core\Password`, `Core\Crypto`) is what unblocks the acceptance list — `docs/plan/m8.md`.
- `examples/http.nvs` and `examples/logging.nvs` owe the configuration their stages write.
- Two dead `[context] modules` selectors in `docs/agent/loop-goal.toml` — `nvs-host`'s `pool.rs`
  and `stream.rs`.
