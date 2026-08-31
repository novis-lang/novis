# Handoff

## State

**ADR 0086 § 4's prompts have landed.** `Core\Cli::ask`, `::confirm`, `::select<T>` and `::secret`
are rows on `CLASS` (`crates/nvs-stdlib/src/cli.rs:151`) over a reader that opens the controlling
terminal by name — `/dev/tty`, or `CONIN$`/`CONOUT$` — in `nvs_runtime::terminal`
(`crates/nvs-runtime/src/terminal.rs:413`), so `cat data.csv | myprog` can still ask. Nothing in
the prompt path reads standard input, and the driver's failing check
`a_prompt_reads_the_controlling_terminal_and_not_stdin` now exists and holds that over both
modules' own sources.

**A prompt asks two questions before it opens anything**, and that pair is what makes prompts
testable at all: `nvs_runtime::terminal::is_interactive()` (does the *process* have a terminal) and
`Ctx::output_reaches_the_terminal()` (does *this request's* output reach it). A `.nvst` case runs
as a child with its output piped, so every prompt in one takes the not-interactive path by
construction, and four cases freeze what it answers there. `cli.rs`'s module doc § *The prompts*
owns the reasoning.

**`Core\Cli\NotInteractive` is a class in `nvs_hir::errors::TREE`**, under `RuntimeError` — the
second namespaced entry beside `Core\Test\Failure`. `CoreTy::SecretTaintedStr` is new beside it:
`secret` answers `secret tainted string`, both qualifiers at once, and `echo` of it is `E0790`.

**Two spellings moved, both folded into ADR 0086's own body.** A shape-literal field name may now
be a keyword, because § 4 spells an option `default`; and `select`'s list is `$choices`, because
ADR 0063 R2 reserves `options`. Both are playbook bullets under *Writing Novis itself*.

**Still owed on `Core\Cli`**: `arguments`, `write`, `displayWidth`, `multiSelect`, and § 4's
scripted answer queue for `nvs test` — `cli.rs`'s gap 2 owns all of them. `Text + Text` still needs
a row in `nvs_types`' operator table (`crates/nvs-types/src/expr/operators.rs`). Stage 2 owes
`truncate`, `lock` and `Core\IO::stdin`/`stdout`/`stderr`.

**The orientation pack still does not print `docs/spec/01-core-library.md`**, which is in no
`[context]` field; § 15's `Core\Cli` list is what these signatures were written against and § 10's
exception tree is what the new class was added to.

## Next group

**§ 4's second rule and § 5's live region, over the same two files. Shared file set:
`crates/nvs-stdlib/src/cli.rs`, `crates/nvs-runtime/src/terminal.rs`.**

- [ ] **No prompt blocks without a deadline** — ADR 0086 § 4's second paragraph, over the four
      bodies that now exist (`crates/nvs-stdlib/src/cli.rs:880`) and the read beneath them
      (`crates/nvs-runtime/src/terminal.rs:425`). What is pinned today is the *unattended* half: a
      prompt with a terminal still waits as long as a person takes, and a read with no bound is
      what ADR 0074's rule forbids on the other surface. Closes
      `no_prompt_blocks_without_a_deadline`.
- [ ] **`live<T>` is scoped and restores the terminal on a panic** — ADR 0086 §§ 5 and 8 with ADR
      0020 § 4, as a row on `CLASS` (`crates/nvs-stdlib/src/cli.rs:151`) whose body brackets a
      callable, over the same handle the prompts open
      (`crates/nvs-runtime/src/terminal.rs:462`). Closes
      `a_live_region_is_scoped_and_restores_the_terminal_on_a_panic`.
- [ ] **`multiSelect<T>` and § 4's scripted answer queue** — the fifth row beside its four
      siblings (`crates/nvs-stdlib/src/cli.rs:200`) and a queue the reader drains ahead of the
      device (`crates/nvs-runtime/src/terminal.rs:413`), which is what makes an *interactive* flow
      assertable rather than only its defaults. Worth taking beside one of the two above; alone it
      is a session's fixed cost for one row.

## Backlog

- `Core\Cli::arguments`, `write` and `displayWidth` — `crates/nvs-stdlib/src/cli.rs` gaps 1 and 2.
- `Text + Text` needs an operator-table row — `crates/nvs-types/src/expr/operators.rs`.
- Stage 2's `truncate`, `lock` and `Core\IO::stdin`/`stdout`/`stderr` — `docs/plan/m8.md`.
- The `[context]` manifest has no `docs/spec/01-core-library.md` selector — `docs/agent/loop-goal.toml`.
