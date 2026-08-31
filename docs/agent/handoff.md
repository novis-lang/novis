# Handoff

## State

**ADR 0086 § 4 is closed, prompts and queue.** `Core\Test::scriptAnswers(array<string>)`
(`crates/nvs-stdlib/src/test.rs:280`) appends to `Ctx::script_answers`, and `ask_terminal`
(`crates/nvs-stdlib/src/cli.rs:1102`) takes its oldest line **ahead of** `watched`, so all five
prompts drain it and none grew a path of its own. `select` and `multiSelect` decide before they ask —
they render a menu first — so those two read `answerable` rather than `watched`; the playbook's
*Writing Novis itself* bullet is the home of that trap. The queue lives on `nvs_runtime::Ctx` beside
the fixed clock and is scoped by ADR 0079 § 2's per-test isolate rather than by a `thread_local`; the
`scripted_answers` field doc (`crates/nvs-runtime/src/ctx.rs:620`) owns that reasoning, and the ADR's
§ 4 last paragraph now names the member. A scripted line wins over a terminal that is there, and an
empty queue is the ordinary unattended answer — so scripting too few answers is assertable rather
than a hang. Three `.nvst` cases under `tests/conformance/core/test-script-answers-*` ask three
different questions of it, which is `conformance_coverage.rs`'s floor and not a habit.

**ADR 0075 § 2's approximate tier is asserted as behaviour.**
`shed_is_per_core_and_approximate_and_says_so` (`crates/nvs-stdlib/src/ratelimit.rs:1012`) closes the
driver's last failing acceptance check: two threads each admit their own arrival for one key under a
limit of one, ADR 0059 § 3's cap forgetting the arrival admits a burst GCRA alone refuses, and
`SHED_DOC` states both — the member is driven through `nvs_runtime::call`, since what is at stake is
where the timestamp goes rather than the arithmetic on it.

**Still owed on `Core\Cli`**: `arguments`, `write` and `displayWidth` — `cli.rs`'s gaps 1 and 2 own
them, and the gap list no longer names the answer queue. `Text + Text` still needs a row in
`nvs_types`' operator table (`crates/nvs-types/src/expr/operators.rs`). Stage 2 owes `truncate`,
`lock` and `Core\IO::stdin`/`stdout`/`stderr`.

**The orientation pack still does not print `docs/spec/01-core-library.md`**, which is in no
`[context]` field; § 15's `Core\Cli` list is what the next group's signatures are written against, and
its `Core\Test` roster row is a place a new member has to be added by hand.

## Next group

**§ 3's two output members and § 15's `arguments`, over one file set:
`crates/nvs-stdlib/src/cli.rs` and `crates/nvs-runtime/src/ctx.rs`.**

- [ ] **`write`** — ADR 0086 § 3, `cli.rs`'s gap 1. A row on `CLASS`
      (`crates/nvs-stdlib/src/cli.rs:167`), its card, an `address` arm
      (`crates/nvs-stdlib/src/cli.rs:910`) and a `.nvst` case. It is a second spelling of the sink
      `echo` already is, so what it owes is the stream argument — `Cli\Stream` is already an enum
      here — and not a second rendering rule; `question_of` (`crates/nvs-stdlib/src/cli.rs:1081`) is
      the shape for reading its text argument.
- [ ] **`arguments(): array<tainted string>`** — spec § 15, `cli.rs`'s gap 2. The same four edits,
      reading `Ctx::command_line` (`crates/nvs-runtime/src/ctx.rs:1272` is its writer, and
      `program_name` at `crates/nvs-runtime/src/ctx.rs:1298` is the accessor beside it). Empty for
      every served request, which that field's doc says is the state the member has to answer for.
- [ ] **`displayWidth`** — ADR 0086 § 3, the same four edits at `crates/nvs-stdlib/src/cli.rs:167`
      and `crates/nvs-stdlib/src/cli.rs:910`, and the one of the three with a dependency: it owes a
      UAX #11 east-asian-width table this tree does not carry, and belongs beside `Cli\Style`. A new
      crate is pre-authorized under ADR 0051 § 4 and owes the `[workspace.dependencies]` line with
      its reason, `cargo deny check` and `python tools/gen-attribution.py`.

## Backlog

- `Text + Text` needs a row in `crates/nvs-types/src/expr/operators.rs` — `cli.rs`'s § 2 gap.
- Stage 2 owes `truncate`, `lock` and `Core\IO::stdin`/`stdout`/`stderr` — `docs/plan/m8.md`.
- `orient.py` prints no `docs/spec/01-core-library.md`; the manifest wants a `[context]` selector for
  § 15 and for the `Core\Test` roster row — `docs/agent/loop-goal.toml`.
- A `Cli\Text` renders once and cannot be plain on one stream and styled on another — `cli.rs` gap 3.
