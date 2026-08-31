# Handoff

## State

**ADR 0086 § 4's five prompts are all on disk.** `multiSelect<T>` is a row on `CLASS`
(`crates/nvs-stdlib/src/cli.rs:163`) beside `select`, sharing its menu through `menu_of`
(`crates/nvs-stdlib/src/cli.rs:1370`) and differing only in the parse: `chosen_of`
(`crates/nvs-stdlib/src/cli.rs:1397`) reads a line of numbers into one flag per option, so a choice
named twice is chosen once and the answer comes back in the **menu's** order rather than the
typing's. An empty line is the empty set — which is why § 4's table gives this member no `default`
and why an unattended run throws `Core\Cli\NotInteractive` where the other four answer one. Anything
off the menu (`0`, `1-3`, `all`) is asked again rather than guessed at; a trailing separator is
forgiven. The member's own doc comment is the home of those three decisions.

**ADR 0059 § 3's cap is asserted as memory, not as bookkeeping.**
`the_local_tiers_memory_is_charged_to_the_core_and_capped` (`crates/nvs-stdlib/src/cache.rs:1023`)
closes the driver's last failing acceptance check. It joins the two halves of § 3's sentence:
`local_cap` reads `[cache.local] max_size` per write because the directive's row is `Apply::Reload`,
and a sweep writing thirty-two times the cap leaves *this thread's* `nvs_runtime::budget::live_bytes`
up by about the cap — so eviction gave the bytes back. The charge is per-thread, and a second core
writing the same key pays for its own copy, which is § 3's O(cores × working set) measured.

**Still owed on `Core\Cli`**: `arguments`, `write`, `displayWidth` and § 4's scripted answer queue
for `nvs test` — `cli.rs`'s gaps 1 and 2 own them. `Text + Text` still needs a row in `nvs_types`'
operator table (`crates/nvs-types/src/expr/operators.rs`). Stage 2 owes `truncate`, `lock` and
`Core\IO::stdin`/`stdout`/`stderr`.

**The orientation pack still does not print `docs/spec/01-core-library.md`**, which is in no
`[context]` field; § 15's `Core\Cli` list is what these signatures are written against.

## Next group

**§ 4's answer queue and § 3's two remaining members, over one file set:
`crates/nvs-stdlib/src/cli.rs`, `crates/nvs-runtime/src/terminal.rs` and `crates/nvs-test/src/`.**

- [ ] **A scripted answer queue under `nvs test`** — ADR 0086 § 4's last paragraph. Today every
      prompt takes the not-interactive path under the runner, because a test's output is a buffer so
      `watched` (`crates/nvs-stdlib/src/cli.rs:1093`) answers `false`. The queue is drained by
      `ask_terminal` (`crates/nvs-stdlib/src/cli.rs:1104`) ahead of that check, so all five prompts
      gain it at once and none grows a path of its own; where it is *filled* from is the open
      question — a `Core\Test` member is the cheap answer and the one this group should decide.
- [ ] **`write` and `displayWidth`** — ADR 0086 § 3, `cli.rs`'s gap 1. `write` is a row on `CLASS`
      (`crates/nvs-stdlib/src/cli.rs:163`) with an `address` arm
      (`crates/nvs-stdlib/src/cli.rs:906`) and a `Cli\Stream` argument over the sink `echo` already
      is. `displayWidth` belongs beside `Cli\Style` and additionally owes a UAX #11 table this tree
      does not carry — take `write` first and say in the handoff whether the table is worth a slice.
- [ ] **`arguments(): array<tainted string>`** — spec § 15, `cli.rs`'s gap 2. A row on `CLASS`
      (`crates/nvs-stdlib/src/cli.rs:163`) and an `address` arm
      (`crates/nvs-stdlib/src/cli.rs:906`), over what `nvs_runtime` already holds of the process's
      argv; ADR 0012 § 1 is why it replaces `$argv`/`$argc` rather than joining them.

## Backlog

- `Text + Text` needs an operator-table row — `crates/nvs-types/src/expr/operators.rs`.
- Stage 2 owes `Core\IO::truncate`, `::lock` and the three standard streams — `io.rs`'s own gaps.
- § 5's region does not repaint on a resize; `terminal.rs`'s `Region` doc owns the gap.
- `docs/spec/01-core-library.md` is in no `[context]` field of `docs/agent/loop-goal.toml`.
- A `Core` generic's `T` does not bind from an inline array literal — playbook, *Writing a test case*.
