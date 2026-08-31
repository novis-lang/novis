# Handoff

## State

**The failing acceptance check is closed.** `nvs-types (the open-typed argument that is still
checked)` named a test that did not exist. `Core\Log::write` is ADR 0033 § 4's log sink now, refused
at the **call site** because `fields` stays `array<mixed>` *by design* — the ADR says so in the
bullet — so no parameter type can carry the question:
`nvs_types::expr::quals::reject_secret_logged_argument` (`crates/nvs-types/src/expr/quals.rs:658`)
is the fourth rule of `reject_secret_debug_argument`'s shape, under the new `E0797`. It is scoped to
the `fields` argument, and `is_fields_argument` owns why the other two positions are left to their
own declared types. It also reads the *elements* of a written literal by name off the `LocalScope`,
which is § 4's "including inside a `fields` array literal" and which no type could answer: a literal
checked against an `array<mixed>` expectation *is* that expectation.

**`Core\Cli::arguments` has landed** — spec § 15's `arguments(): array<tainted string>`, the first
row of the class in spec order. It reads no OS: ADR 0118 § 2 keeps `argv` out of `nvs-stdlib`, so it
answers `Ctx::command_line()`, which only `nvs-cli`'s `main` writes — a served request therefore
reads an empty array, and the module's gap 2 now says that rather than naming a missing member.
`$argv[0]` has no spelling: the invoked name is `Ctx::program_name`'s question.

**Still owed on `Core\Cli`**: `displayWidth`, the module's own gap 1, which owes a UAX #11 table this
tree does not carry. Stage 7 still owes reading `[log] target`.

`orient.py` printed no section of ADR 0086 or ADR 0033 although both items were sections of them:
`[context] adrs` needs `0086:1`, `0086:3` and `0033:4`, and `[context] spec` still misses
`docs/spec/01-core-library.md` § 15.

## Next group

**`Core\Cli`'s last member and the case that pins this session's other half, over
`crates/nvs-stdlib/src/cli.rs` and `tests/conformance/core/`.**

- [ ] **`Core\Cli::displayWidth`** — ADR 0086 § 3's `displayWidth(string $value): uint`, UAX #11
      columns rather than `Core\Str::length`'s graphemes. The five edits of a `Core` member, at
      `crates/nvs-stdlib/src/cli.rs:213` (the rows — it goes after `colorDepth`, spec order),
      `crates/nvs-stdlib/src/cli.rs:362` (the cards, in row order beside `WIDTH_DOC`),
      `crates/nvs-stdlib/src/cli.rs:1020` (the `address()` arm) and
      `crates/nvs-stdlib/src/cli.rs:1286` (`nvs_core_cli_width`, the body to write beside). The
      decision to make first is where the width table comes from: a dependency is pre-authorized
      under ADR 0051 § 4 and owes the `[workspace.dependencies]` comment, `cargo deny check` and
      `python tools/gen-attribution.py`.
- [ ] **A `.nvst` case for `E0797`** — the refusal above has a `-p nvs-types` test and no case a
      program reaches. `tests/conformance/core/cli-arguments-elements-are-tainted-and-a-sink-refuses-one.nvst`
      is the `--EXPECTF-ERROR--` shape to copy, and `crates/nvs-types/src/expr/quals.rs:658` is what
      it asks about; both halves are worth pinning, the bag whose own type carries the qualifier and
      the value written inside the literal.

## Backlog

- Stage 7's last item: reading `[log] target` — `docs/adr/0020-error-escalation-ladder.md` § 4.
- `Core\IO`'s `truncate` and `lock` — `crates/nvs-stdlib/src/io.rs`'s own module doc.
- `Cli\Text` cannot be plain on one stream and styled on another — `crates/nvs-stdlib/src/cli.rs` gap 3.
- ADR 0033's container axis: an array literal joins nothing, so a qualifier on an element is lost at
  the binding — `crates/nvs-types/src/expr/quals.rs`'s `reject_secret_encoded_argument` names it.
