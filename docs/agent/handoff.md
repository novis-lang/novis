# Handoff

## State

**ADR 0086 § 1's raw path has landed: the terminal sink substitutes everything except its own
carrier.** `echo` of a `Ty::Object` or `Ty::Tagged` operand now lowers to `Helper::EchoValue`
with no conversion in front of it (`crates/nvs-ir/src/lower/expr.rs:521`), because a carrier is
a *class* and a conversion throws the class away; `nvs_echo_value`
(`crates/nvs-runtime/src/helpers.rs:2504`) asks `is_carrier_value` and only then renders through
the same `stringify` `nvs-ir` would have called. Both spellings reach the stream through one
`write_rendered`, so they cannot come to write different bytes. A scalar or `Ty::Str` operand
still takes `EchoStr` and one helper call less. `is_carrier_value`'s doc comment owns why the
question is asked of the class rather than of a `raw` bit riding on a `Tag::Str`.

**`Core\Cli\Text::plain` is the constructor that keeps that path from being a hole**
(`crates/nvs-stdlib/src/cli.rs:577`): it substitutes over its own argument, so the only control
bytes a `Text` can carry are the ones `styled` will put there from a `Cli\Style`. `built`
(`crates/nvs-stdlib/src/cli.rs:557`) transfers and neutralizes nothing; its doc comment names
both callers and what each owes. `the_sink_writes_a_carrier_raw_and_everything_else_substituted`
pins the disagreement over one payload — either half alone passes while the rule is broken.

**`Cli\Style`, `Cli\Color` and `Text::styled` are blocked on one mechanism**, which is the next
group's first item and is *not* what ADR 0086 § 2 says it is: a `Core` class constant cannot be
an instance today. The playbook bullet under *Writing Novis itself* and `cli.rs`'s gap 3 both
own it. That is what the driver's failing check
`styling_is_a_value_type_and_never_a_grammar` waits on; the rest of `[3 process and terminal]`'s
`Core\Cli` list is `a_prompt_reads_the_controlling_terminal_and_not_stdin`,
`no_prompt_blocks_without_a_deadline` and
`a_live_region_is_scoped_and_restores_the_terminal_on_a_panic`. Stage 2 still owes `truncate`,
`lock` and `Core\IO::stdin`/`stdout`/`stderr`.

**`Text + Text` is still owed and is not a stdlib slice**: `+` over two objects needs a row in
`nvs-types`' operator table before any of this file can express it.

**The orientation pack still does not print `docs/spec/01-core-library.md`**, which is in no
`[context]` field; § 13's `Core\Cli\Text` rows are what `plain`'s signature came from.

## Next group

**ADR 0086 § 2's styling half, which is one mechanism and then two slices of rows over it.
Shared file set: `crates/nvs-stdlib/src/registry.rs`, `crates/nvs-types/src/expr_table.rs` and
`crates/nvs-stdlib/src/cli.rs`.**

- [ ] **A `Core` class constant that is an instance — decide the spelling and where it is
      built** — ADR 0086 § 2's *"the sixteen named colours are class constants"*. `CoreConst`
      (`crates/nvs-stdlib/src/registry.rs:956`) carries a `Const`
      (`crates/nvs-stdlib/src/registry.rs:604`) whose roster is scalar only, and
      `nvs_types::expr::members` folds one to `ExprInfo::CoreConst`
      (`crates/nvs-types/src/expr/members.rs:156`). The shape to copy is its documented sibling
      `ExprInfo::ProgramInstances` (`crates/nvs-types/src/expr_table.rs:714`), where the fold's
      answer is an *allocation* `nvs-ir` emits at the use site — so `Color::RED` becomes a
      per-request instance like any other object and nothing is shared across isolates. Decide
      and record it in ADR 0086 § 2's own body, which is currently wrong about this tree.
- [ ] **`Cli\Color` and `Cli\Style::of`, over that mechanism** — ADR 0086 § 2's two bullets:
      sixteen named constants plus `Color::rgb(uint, uint, uint)` and `Color::index(uint)`, and
      `Style::of({color?, background?, bold?, dim?, italic?, underline?, strikethrough?})` as
      R5's `of` with R2's one trailing shape. Rows and cards beside `Core\Cli`'s at
      `crates/nvs-stdlib/src/cli.rs:112`, the `address()` arm at
      `crates/nvs-stdlib/src/cli.rs:357`, and `crate::registry::CLASSES` needs both names.
- [ ] **`Cli\Text::styled(string, Style)` and
      `styling_is_a_value_type_and_never_a_grammar`** — the acceptance check the driver is
      failing on. The row goes in `TEXT` (`crates/nvs-stdlib/src/cli.rs:514`) beside `plain`'s,
      the body beside `crates/nvs-stdlib/src/cli.rs:577`, and the test beside
      `crates/nvs-stdlib/src/cli.rs:685`, which is the raw path's own case and the one that
      already proves `styled`'s ESC bytes will survive the sink. The grammar half of the name is
      assertable off the registry: no `Cli` row takes a template or a pattern, so ADR 0063 R11's
      count stays four.

## Backlog

- `Text + Text` — an operator over two objects, and so a `nvs-types` slice first
  (`docs/adr/0086-core-cli-terminal-is-a-sink.md` § 2).
- `Core\Cli::write(Cli\Stream $stream, …)` — spec § 15's roster, the second spelling of the sink
  (`crates/nvs-stdlib/src/cli.rs`'s gap 1).
- `Core\Cli::displayWidth` — owes a UAX #11 table this tree does not carry (same gap).
- An `ESC` in a source literal is still not a compile error, so `"\e"` compiles — ADR 0086 § 1's
  second mechanism.
- Stage 2 still owes `Core\IO::truncate`, `::lock` and `Core\IO::stdin`/`stdout`/`stderr`
  (`docs/plan/m8.md`).
- `docs/spec/01-core-library.md` is in no `[context]` field of `docs/agent/loop-goal.toml`.
