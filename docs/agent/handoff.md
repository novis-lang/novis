# Handoff

## State

**ADR 0102 § 3's `#[Query]` and ADR 0086 § 6's `#[Option]` are now held to the declaration that
reads them.** `check_stray_query` (`crates/nvs-types/src/routes.rs:254`) reports `E0765` for a
`#[Query]` on a method carrying no `#[Route]`, `check_stray_options`
(`crates/nvs-types/src/commands.rs:125`) reports `E0766` for an `#[Option]` on a method carrying no
`#[Command]`, and both run from the per-method walk in `crates/nvs-types/src/attributes.rs:63` — the
one that already calls `check_one_access`. Neither owning pass could ask it: each walks only the
methods its own attribute selects, so a stray marker is invisible to the pass that would refuse it.
Reported once per parameter, so an author who wrote two is told about both.

**`routes.rs` has no known gaps left.** `commands.rs` is down to two, renumbered: the table itself
(gap 1) and whether `name` is required on `#[Command]` (gap 2, which the table pass is what should
answer).

**The acceptance check that fails is item 6's**, `a_command_table_is_built_from_the_program_enumeration`
— the test does not exist in `crates/nvs-types/tests/commands.rs`, which holds seven and none of them
that. It is `commands.rs`' gap 1 stated as a test name, open by design and not a regression; the next
group below is what closes it.

**Item 15 in `docs/agent/loop-goal.md` still owns M4's seventeen `nvs-ir` lowering refusals**,
unchanged and standing by design; the ratchet is `CEILING` at `crates/nvs-ir/tests/refusals.rs:66`.

**Orientation gap:** the pack printed ADR 0096 §§ 1/1a/4 and ADR 0102 § 8, but not ADR 0102 § 3 or
ADR 0086 § 6 — the two sections this group's markers come from. The module docs restated enough to
work from; `[context] adrs` should gain `0102 § 3` and `0086 § 6` before the next group, which is
squarely inside § 6.

## Next group

**One file set: `crates/nvs-types/src/commands.rs`, `crates/nvs-types/src/check.rs`,
`crates/nvs-types/src/lib.rs` and `crates/nvs-types/tests/commands.rs`** — ADR 0086 § 6's command
table, built the way ADR 0077's route table already is, which is the model to copy rather than
re-derive. This closes the failing acceptance check. Next free diagnostic code is `E0767`, after
`crates/nvs-diagnostics/src/lib.rs:2111`.

- [ ] **The table is collected.** ADR 0086 § 6 over ADR 0061 § 3's scan. A `CommandTable` shaped like
      `RouteTable` (`routes.rs:349`, its row `Route` at `routes.rs:292`) and threaded exactly as that
      one is: created at `check.rs:110`, carried through `Env` at `lib.rs:359`, recorded through
      `expr_table.rs:960`. `check_class_commands` (`commands.rs:90`) is what fills it, and it already
      walks every `#[Command]` method.
- [ ] **A duplicate command name is § 6's third compile error.** `check_table` (`routes.rs:1015`) is
      the shape — the whole-program question asked once, from `check.rs:150`. Whether `name` is
      required (`commands.rs`' gap 2) is answered here too: the row is what needs one.
- [ ] **`a_command_table_is_built_from_the_program_enumeration`**, in
      `crates/nvs-types/tests/commands.rs`, asked over **two files** through
      `common::check_program_table` (`crates/nvs-types/tests/common/mod.rs:192`) — the only shape that
      tells "the program's table" from "this file's". The acceptance check names the test by that
      exact spelling.

## Backlog

- A `#[Query]` or `#[Option]` on a **free function's** parameter is refused by nothing:
  `crates/nvs-types/src/check.rs` has no `StmtKind::FunctionDecl` arm, so `check_declaration` never
  reaches one and ADR 0046 § 2's constant rule is unchecked there too. Owner: `attributes.rs`' module
  doc, which should carry it as a gap.
- ADR 0085's OpenAPI document over the finished route table — `docs/adr/0085`, untouched.
- The conformance floor is per class, not per corpus — `python tools/gaps.py` is what says so.
- `Core\Router::match` and `Core\Command::run` stay out of scope by standing decision.
