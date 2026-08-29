# Handoff

## State

**ADR 0086 § 6's command table is built.** `CommandTable` and its `Command` row live in
`crates/nvs-types/src/commands.rs:145`, filled by `check_class_commands` out of the same walk that
already checked each method's options, threaded through `Env` (`crates/nvs-types/src/lib.rs:359`)
exactly as `RouteTable` is, and crossing to `nvs-ir` on `ExprTypeTable::commands()`
(`crates/nvs-types/src/expr_table.rs:976`). All three of § 6's compile errors are now reported: a
duplicate command name is `E0768` from `check_table` over the finished table, and the two decidable
from one parameter list were already there.

**Two decisions the ADR left open are recorded in that module's docs**, which is their home: `name`
is **required** (`E0767`, because a command line selects a command by it — unlike a route, which is
reached by its path), and **every** `#[Command]` on a method becomes a row, which is ADR 0046 § 3's
repetition read as `check_class_routes` reads it and makes two names for one implementation an
alias. `commands.rs`' known-gap list is down to one: a row carries no parameter *types*, by design —
the consumer holds the signature the handler names.

**Nothing reads the table back.** `Core\Command::run`/`::help`/`::completions` are goal 4's, and the
standing decisions keep them out of scope. Stage 2's acceptance check (`nvs-types (the command
table)`) now names four tests that all exist and pass.

**Item 15 still owns M4's seventeen `nvs-ir` lowering refusals**, unchanged and standing by design;
the ratchet is `CEILING` at `crates/nvs-ir/tests/refusals.rs:66`.

**Orientation gap, second session running:** the pack still does not print ADR 0086 § 6, which is
where this whole group came from. `[context] adrs` needs `0086 § 6`; for the group below it needs
`0057 §§ 1, 4` (§ 1 is the closed list, § 4 is "preparation never changes behaviour", which is the
soundness rule the stage's own comment states).

## Next group

**ADR 0057's intrinsic folding — stage 3, the next acceptance check in run order.** One file set:
`crates/nvs-types/src/intrinsics.rs` (new), `crates/nvs-types/src/expr/calls.rs`, and
`crates/nvs-types/tests/intrinsics.rs` (new). Nothing exists yet — a `grep` for `0057` across
`nvs-types/src` returns nothing, so this is greenfield in the checker and every parser it needs is
already written in `nvs-stdlib`. The stage's own comment in `docs/agent/loop-goal.toml:1222` is the
specification: **the prepared path and the runtime path are one implementation**, so a fold produces
an earlier answer and never a different one. Next free diagnostic code is `E0769`, after
`crates/nvs-diagnostics/src/lib.rs:2128`.

- [ ] **The closed list, and the hook that reads it.** ADR 0057 § 1. A roster of `Core` members whose
      literal argument is folded, consulted where a call's target is already resolved —
      `infer_static_call` (`crates/nvs-types/src/expr/calls.rs:182`) for `Core\Str::format(...)`,
      `infer_method_call` (`crates/nvs-types/src/expr/calls.rs:51`) for `$when->format(...)`, both
      of which reach `check_args` (`crates/nvs-types/src/expr/calls.rs:930`). Test:
      `a_member_outside_the_intrinsic_list_is_never_folded`.
- [ ] **`Core\Str::format`'s template.** § 4 over `nvs_stdlib::format`'s existing grammar
      (`crates/nvs-stdlib/src/format.rs:46` says out loud that this is what it is waiting for) —
      give that parser an entry point rather than writing a second one. Test:
      `a_literal_format_template_checks_its_placeholder_count_and_types`.
- [ ] **`Core\Regex`'s pattern.** Same shape over `crates/nvs-stdlib/src/regex.rs:29`. Test:
      `a_literal_regex_pattern_is_prepared_while_checking`.
- [ ] **The date pattern.** `crates/nvs-stdlib/src/time.rs:73` and
      `crates/nvs-stdlib/src/cldr.rs:55`. Test: `a_literal_date_format_is_validated_while_checking`.

## Backlog

- `a_literal_uri_is_validated_while_checking` — stage 3's fourth intrinsic, `docs/agent/loop-goal.toml:1238`.
- `a_prepared_literal_and_its_runtime_twin_share_one_implementation` — the same group's `-p nvs-stdlib`
  half, `docs/agent/loop-goal.toml:1248`.
- Stage 4 is ADR 0085's OpenAPI document, four tests in `-p nvs-cli`, over the route table this goal
  finished; `docs/agent/loop-goal.toml:1274`.
- `every_core_class_has_a_conformance_floor_of_three` and the other stage 5 depth gates —
  `python tools/gaps.py` is the worklist, never re-derived by hand.
- A method carrying two `#[Command]`s that name the same command is refused as a duplicate, which is
  right but reports at the second `name:` field; if that reads badly in practice, `commands.rs`'
  `check_table` is where a same-method pair would get its own sentence.
