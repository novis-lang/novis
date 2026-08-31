# Handoff

## State

**ADR 0086 § 4's prompts now hold both halves of "it never blocks".** The four members are rows on
`CLASS` (`crates/nvs-stdlib/src/cli.rs:160`) reading the controlling terminal, and the read itself is
under `nvs_runtime::terminal::ANSWER_DEADLINE` — five minutes, with no parameter on any member that
lengthens it. `ask` runs whole on its own thread and `answer_within`
(`crates/nvs-runtime/src/terminal.rs:512`) waits on the channel, because a cooked-mode line read has
no portable timed form: `poll`/`WaitForSingleObject` answers "an input record arrived", never "the
line is complete". Both of the driver's § 4 checks pass.

**A silence is one class, one opening clause and two endings.** `Core\Cli\NotInteractive` covers both
— a program catching it is asking whether the question could be answered at all — and `unanswered`
(`crates/nvs-stdlib/src/cli.rs:877`) picks the ending, so a deadline that elapsed never claims a
terminal that does not exist. Both messages open `no answer for Core\Cli::<member>:`, which is what
lets one `.nvst` case discharge both for `conformance_coverage.rs`'s error-path gate — the timeout
half is unreachable from a piped case by construction. ADR 0086 § 4's body carries the rule.

**Still owed on `Core\Cli`**: `arguments`, `write`, `displayWidth`, `multiSelect`, § 5's `live<T>`
and `progress<T>`, and § 4's scripted answer queue for `nvs test` — `cli.rs`'s gap 2 owns them.
`Text + Text` still needs a row in `nvs_types`' operator table
(`crates/nvs-types/src/expr/operators.rs`). Stage 2 owes `truncate`, `lock` and
`Core\IO::stdin`/`stdout`/`stderr`.

**The orientation pack still does not print `docs/spec/01-core-library.md`**, which is in no
`[context]` field; § 15's `Core\Cli` list is what these signatures are written against and § 10's
exception tree is what `Cli\NotInteractive` was added to.

## Next group

**§ 5's live region and § 4's fifth prompt, over one file set:
`crates/nvs-stdlib/src/cli.rs`, `crates/nvs-runtime/src/terminal.rs`, and
`crates/nvs-stdlib/src/instance.rs` for the handle class.**

- [ ] **`live<T>` is scoped and restores the terminal on a panic** — ADR 0086 §§ 5 and 8 with ADR
      0020 § 5. A row beside the four prompts (`crates/nvs-stdlib/src/cli.rs:160`) with its `address`
      arm (`crates/nvs-stdlib/src/cli.rs:631`); the frame writer, the cursor hide/restore and § 5's
      "renders nothing at all when the stream is not a terminal" belong beside `prompt`
      (`crates/nvs-runtime/src/terminal.rs:478`), and the handle `$body` receives is a
      `CoreTy::Instance` built the way `crates/nvs-stdlib/src/instance.rs` builds one. § 8 makes
      restoration an obligation on *every* exit path — throw, fatal, panic, signal — so a scope guard
      in Rust is the design and a Novis `finally` is not. Closes
      `a_live_region_is_scoped_and_restores_the_terminal_on_a_panic`.
- [ ] **`progress<T>` over that same region** — § 5's second row,
      `advance({by?: uint, label?: string})` on the writer `live<T>` just built
      (`crates/nvs-stdlib/src/cli.rs:160`). Take it only if `live<T>` landed with room left; it is
      the cheap half of § 5 and none of it is new mechanism.
- [ ] **`multiSelect<T>` and § 4's scripted answer queue** — the fifth prompt beside the four
      (`crates/nvs-stdlib/src/cli.rs:1106` is the shape a prompt body takes now, and
      `crates/nvs-stdlib/src/cli.rs:487` the card beside it), and § 4's last paragraph, which is what
      makes an interactive flow assertable under `nvs test` rather than untestable.

## Backlog

- `Core\Cli::arguments`, `write` and `displayWidth` — `cli.rs`'s gaps 1 and 2.
- `Text + Text` owes an operator row — `crates/nvs-types/src/expr/operators.rs`.
- Stage 2 owes `Core\IO::truncate`, `lock` and `stdin`/`stdout`/`stderr` — `io.rs`'s own gaps.
- `docs/spec/01-core-library.md` is in no `[context]` field of `docs/agent/loop-goal.toml`.
