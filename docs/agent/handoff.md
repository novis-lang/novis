# Handoff

## State

**Goal 4 — `Core`'s capability-bearing half — is running, and ADR 0086 § 6's one remaining hole is
`completions`.** `Core\Cli\Shell` is now on disk and registered: four cases — `Bash`, `Zsh`, `Fish`,
`Pwsh`, § 6's roster and nothing else — with its ADR 0117 card and a test holding both halves
(`crates/nvs-stdlib/src/cli.rs:278`). It is declared in `cli.rs` rather than beside the member that takes
it, which is an exception to `crate::registry::ENUMS`' own rule; that module's new *Why `Core\Cli\Shell`
is here* section is the one home for why.

**`completions` no longer waits on the enum — it waits on a *name*, and that changes what the next slice
is.** Every one of the four scripts registers itself against the program's own name (`complete -F …
myprog`, `complete -c myprog`, `Register-ArgumentCompleter -CommandName myprog`), and nothing in a
running process has one: `Ctx::command_line` is the words *past* the program, the table carries command
names rather than the program's, and ADR 0118 § 2 forbids `nvs-stdlib` reading `argv[0]` itself. So the
member owes a `Ctx` accessor filled at the one site that already fills `set_command_line`. That fact's
home is `crates/nvs-stdlib/src/command.rs`'s gap 1, which now states it; this paragraph is a pointer.

**The driver's acceptance failure is unchanged and is not a regression.** `examples/crypto.nvs` fails on
`Core\Password::hash`, stage 4's fixture waiting on stage 4, and a program leg runs before every
cargo-named check, so it masks the rest of the list until crypto lands. Three fixtures still owe
configuration their stage must write: `examples/http.nvs` needs stage 5's `http://127.0.0.1:8099` origin,
`examples/logging.nvs` an `[[app]]` block naming `examples/logging/handler.nvs`, and every remaining
fixture that reaches the world its `net.connect` grant. `nvs_runtime::commands`' gap 1 — the four
`ArgConv` variants that are `Unconverted` — is unchanged, and so is `command.rs`'s gap 2.

**`orient.py`'s manifest is unchanged and still wrong in the same two ways**: two dead `[context] modules`
selectors — `crates/nvs-host/src/pool.rs` and `crates/nvs-host/src/stream.rs` — and it wants
`crates/nvs-runtime/src/commands.rs`, `crates/nvs-types/src/defaults.rs`, `crates/nvs-test/src/case.rs`
and `crates/nvs-cli/src/main.rs` added. This session needed `crates/nvs-runtime/src/ctx.rs` as well, to
learn that no program name exists there; add it to `[context] modules` too. Nothing is blocked.

## Next group

**`completions`, and the program name it turns out to need.** The file set is
`crates/nvs-runtime/src/ctx.rs`, `crates/nvs-cli/src/main.rs` and `crates/nvs-stdlib/src/command.rs`, plus
`.nvst` files under `tests/conformance/core/`. Do them in this order — the second cannot be written
without the first.

- [ ] **`Ctx::program_name`, the name a completion script registers against.** An accessor and its setter
      beside the pair that already carry the argument vector — `crates/nvs-runtime/src/ctx.rs:1190` is
      `command_line`, `crates/nvs-runtime/src/ctx.rs:1195` its setter — filled from the invocation at
      `crates/nvs-cli/src/main.rs:843`, the one site that calls `set_command_line`. Decide there what the
      name is for `nvs run script.nvs` (the script's own stem, not `nvs`, or the run would generate
      completions for the toolchain) versus for ADR 0048's single-file executable, and say so in the
      accessor's doc comment — it is the whole of what the member can be wrong about. ADR 0086 § 6,
      ADR 0118 § 2 for why `nvs-stdlib` may not read it itself.
- [ ] **`completions(Cli\Shell $shell): string`, generated from the same table `help` reads.** Row, card,
      body and `address` arm: `crates/nvs-stdlib/src/command.rs:83` is `CLASS`,
      `crates/nvs-stdlib/src/command.rs:153` is `address`, and `crates/nvs-stdlib/src/command.rs:507`'s
      `page_for` is the shape to copy — one generator per case, reading `row.name`, `arg.param`,
      `arg.spellings` and `arg.is_option()`, exactly the fields the page reads. The enum is
      `crates/nvs-stdlib/src/cli.rs:278`, its ordinals decoded as
      `crates/nvs-stdlib/src/cli.rs:331`'s `stream_of` decodes its own. The four scripts' *content* is
      this module's call the way the usage page's layout is; the card at
      `crates/nvs-stdlib/src/cli.rs:285` already commits each case to an idiom. Replace gap 1 at
      `crates/nvs-stdlib/src/command.rs:49` when it closes. ADR 0086 § 6.
- [ ] **A `.nvst` case per shell's script, and the conformance-coverage case.** One file under
      `tests/conformance/core/` asserting each generated script names the declared command — count the
      shells rather than freezing four scripts byte-for-byte, which is *invariance over a sweep* from
      `docs/agent/conventions.md`. `crates/nvs-stdlib/tests/conformance_coverage.rs:1` is what fails
      `cargo test -p nvs-stdlib` until a case calls the member.

## Backlog

- A page names no declared type — `crates/nvs-stdlib/src/command.rs`'s gap 2 and `nvs_types::commands`' gap 1 own it.
- The four `ArgConv::Unconverted` variants — `crates/nvs-runtime/src/commands.rs`' gap 1.
- `Cli\Style`, `Cli\Color`, prompts and `escape` — `crates/nvs-stdlib/src/cli.rs`' gap 3, `docs/plan/m8.md` owns when.
- ADR 0086 § 1's substitution table, which `echo` does not apply — `crates/nvs-stdlib/src/cli.rs`' gap 1.
- Stage 4's `Core\Password`, which `examples/crypto.nvs` waits on and which masks the acceptance list.
- `docs/agent/loop-goal.toml`'s `[context]` manifest — two dead selectors, five wanted additions.
