# Handoff

## State

**Goal 4 — `Core`'s capability-bearing half — is running, and ADR 0086 § 6's matching half is closed
but for `completions`.** A parameter's declared default now crosses on the command row as the *text* a
command line would have written for it (`crates/nvs-types/src/commands.rs:125`'s `CommandArg::default`),
so `Core\Command::run` fills an argument nobody wrote by running that text through the same `convert`
a written word takes. `crates/nvs-stdlib/src/command.rs`'s gap 3 is gone with it.

**The decision the item asked for is recorded in ADR 0086 § 6 and there was no third option**: a
parameter default is *already* a literal of its declared type everywhere in the language
(`E_PARAM_DEFAULT_NOT_LITERAL`, `crates/nvs-types/src/defaults.rs`' module doc), so a `#[Command]`
parameter has no non-literal default to refuse a second time. The row reads the folded constant back off
`MethodSig::defaults` rather than folding the expression again — a second fold could disagree with the
one every ordinary call site uses. `default_text`'s `None` arm is defensive only: every constant it
declines belongs to a type § 6's third compile error already refuses as an `#[Option]`.

**One rule outranks the default and is asserted on both sides**: § 6 gives a `bool` option by its being
*written*, so an unwritten `bool $loud = true` is `false`. That is the flag arm in
`crates/nvs-stdlib/src/command.rs:387`, ahead of the default, and it is pinned natively and in
`tests/conformance/core/command-run-fills-an-unwritten-option-from-its-declared-default.nvst`.

**Two gaps are still named where they bite:** `crates/nvs-stdlib/src/command.rs`'s gap 1,
`completions`, and its gap 2 — a page names no declared type, because the row deliberately carries none
(`nvs_types::commands`' gap 1); the default is on the row now and is still not rendered, which that gap
says why. `nvs_runtime::commands`' gap 1 — the four `ArgConv` variants that are `Unconverted` — is
unchanged.

**The driver's acceptance failure is not a regression.** `examples/crypto.nvs` fails on
`Core\Password::hash`, which is stage 4's fixture waiting on stage 4; a program leg runs before every
cargo-named check, so it will mask the rest of the list until crypto lands. Three fixtures still owe
configuration their stage must write, unchanged: `examples/http.nvs` needs stage 5's
`http://127.0.0.1:8099` origin, `examples/logging.nvs` an `[[app]]` block naming
`examples/logging/handler.nvs`, and every remaining fixture that reaches the world its `net.connect`
grant. `orient.py` still prints two dead `[context] modules` selectors — `crates/nvs-host/src/pool.rs`
and `crates/nvs-host/src/stream.rs` — and wants `crates/nvs-runtime/src/commands.rs`,
`crates/nvs-types/src/defaults.rs`, `crates/nvs-test/src/case.rs` and `crates/nvs-cli/src/main.rs`
added. Nothing is blocked.

## Next group

**§ 6's last hole, and the `Cli` enum it waits on.** The file set is `crates/nvs-stdlib/src/cli.rs`,
`crates/nvs-stdlib/src/command.rs` and `crates/nvs-stdlib/src/registry.rs`, plus one `.nvst` under
`tests/conformance/core/`.

- [ ] **`Cli\Shell`, the four-case enum `completions` takes.** ADR 0086 § 6 names `bash`, `zsh`, `fish`
      and `pwsh` and nothing else, so it is a closed roster like
      `crates/nvs-stdlib/src/router.rs:34`'s `Core\Http\Method` — read that one first, it is the shape
      to copy, including how a `Core`-owned enum is registered and how a member reads a case back.
      `crates/nvs-stdlib/src/cli.rs:1` is the module that owns it. ADR 0086 §§ 3, 6.
- [ ] **`completions(Cli\Shell $shell): string`, generated from the same table `help` reads.** The row,
      card, body and `address()` arm go beside `help`'s in
      `crates/nvs-stdlib/src/command.rs:198`'s `CLASS`, and the generator walks
      `crates/nvs-runtime/src/commands.rs:78`'s rows exactly as `page_for` does — one command name per
      row, one spelling per option. Drops that module's gap 1. ADR 0086 § 6.
- [ ] **A `.nvst` case per shell's script, and the member's conformance-coverage case.** One file under
      `tests/conformance/core/`, asserting the four scripts differ and each names every command — the
      *agreement* shape, not four frozen blobs.
      `crates/nvs-stdlib/tests/conformance_coverage.rs:104` fails without a case calling the member at
      all, and `crates/nvs-stdlib/src/command.rs:198` is the row it counts.

## Backlog

- Render the declared default on a usage page, or record that a page names neither it nor the type —
  `crates/nvs-stdlib/src/command.rs`'s gap 2 owns the question.
- The four `Unconverted` conversions — `decimal`, an enum, a literal union, `Core\Uuid` —
  `crates/nvs-runtime/src/commands.rs`' gap 1.
- Stage 4's `Core\Password::hash`, which `examples/crypto.nvs` waits on — `docs/plan/m8.md`.
- `examples/http.nvs`, `examples/logging.nvs` and the remaining fixtures' configuration —
  `docs/agent/loop-goal.toml`.
