# Handoff

## State

**ADR 0086 § 2 has landed whole.** `Core\Cli\Color` is a value type with two constructors and
sixteen named class constants (`crates/nvs-stdlib/src/cli.rs:681`), `Core\Cli\Style` is one
`of({color?, background?, bold?, dim?, italic?, underline?, strikethrough?})` over R2's trailing
shape (`crates/nvs-stdlib/src/cli.rs:891`), and `Core\Cli\Text::styled` substitutes its text
exactly as `plain` does before wrapping it in the style's own SGR sequence — so the only control
bytes a `Text` can carry are still the ones a `Style` put there. The driver's failing check
`styling_is_a_value_type_and_never_a_grammar` now exists and passes, with three conformance cases
beside it.

**The mechanism three sessions recorded as missing was never missing.**
`registry::Const::Built { symbol, args }` — a constant whose value is the *call* that builds it —
has been in the tree since `Core\Time\Zone::UTC`, and reaches `nvs-ir` intact. One stale sentence
on `CoreConst::ty` said the roster was scalar-only; it is fixed, ADR 0086 § 2's body now names the
mechanism, and the playbook bullet under *Writing Novis itself* owns the trap.

**Styling is rendered when the `Text` is built, not at the write.** § 3's profile is resolved once
per process, so the bytes are identical either way — except per-stream: one `Text` cannot be plain
on a redirected stderr and styled on a terminal stdout in the same run. ADR 0086 § 2's body and
`cli.rs`'s gap 3 own that limit and what closing it would cost (a `Text` of runs, against ADR 0088
§ 5's captured-bytes carrier).

**`Text + Text` is still owed and is not a stdlib slice**: `+` over two objects needs a row in
`nvs-types`' operator table (`crates/nvs-types/src/expr/operators.rs`) before this file can express
it. Stage 2 still owes `truncate`, `lock` and `Core\IO::stdin`/`stdout`/`stderr`.

**The orientation pack still does not print `docs/spec/01-core-library.md`**, which is in no
`[context]` field; § 13's `Core\Cli` rows are what these signatures were written against.

## Next group

**The rest of `[3 process and terminal]`'s `Core\Cli` list — ADR 0086 § 4's prompts, then § 5's
live region. Shared file set: `crates/nvs-stdlib/src/cli.rs`,
`crates/nvs-runtime/src/terminal.rs`.**

- [ ] **§ 4's prompts read the controlling terminal, not stdin** — `ask`, `confirm`, `select<T>`
      and `secret` as rows on `CLASS` (`crates/nvs-stdlib/src/cli.rs:126`), their bodies and their
      `address` arms (`crates/nvs-stdlib/src/cli.rs:371`), over a reader that opens the terminal
      itself rather than reading `Stream::In` — `crates/nvs-runtime/src/terminal.rs:139` is the
      profile beside which it belongs, and ADR 0118 § 2 says the OS side may not live in
      `nvs-stdlib`. Closes `a_prompt_reads_the_controlling_terminal_and_not_stdin`.
- [ ] **No prompt blocks without a deadline** — § 4 again, over the same rows on `CLASS`
      (`crates/nvs-stdlib/src/cli.rs:126`) and the same reader beside
      `crates/nvs-runtime/src/terminal.rs:139`: every prompt carries a bounded wait rather than an
      optional one, which is the shape ADR 0074's *"no spelling for an unbounded outbound wait"*
      already gives the other direction. Closes `no_prompt_blocks_without_a_deadline`, and is a
      second question about the rows above rather than a second set of them.
- [ ] **`live<T>` is scoped and restores the terminal on a panic** — ADR 0086 §§ 5 and 8 with
      ADR 0020 § 4, over `crates/nvs-stdlib/src/cli.rs:371` and the raw-mode half of
      `crates/nvs-runtime/src/terminal.rs:139`. Closes
      `a_live_region_is_scoped_and_restores_the_terminal_on_a_panic`.

## Backlog

- `Text + Text`, which is a `nvs-types` operator-table row first — ADR 0086 § 2.
- `Cli::write` and `Cli::displayWidth` — ADR 0086 § 3, `cli.rs`'s gap 1.
- `Core\IO::truncate`, `lock`, and `stdin`/`stdout`/`stderr` — `docs/plan/m8.md` stage 2.
- A per-stream `Text` of runs, if `Cli::write` ever takes a stream — ADR 0086 § 2's body.
- `docs/spec/01-core-library.md` § 13 is in no `[context]` field of `docs/agent/loop-goal.toml`.
