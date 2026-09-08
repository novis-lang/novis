# Handoff

## State

**Goal 19 — stage 5's reference pages are done.** `docs/reference/lang/50-classes.md` counts six
global interfaces, says what `Parses` is for and carries a `Parses` section beside `Comparable`'s;
`docs/reference/lang/90-attributes.md`'s two prose rosters end at "a class implementing `Parses`"
rather than at `Core\Uuid`; `docs/novis.md` is regenerated. Every example in both chapters runs —
`reference.py --examples-only` reports 46 of 46 for the classes chapter and 11 of 11 for attributes.

**The earliest failing acceptance check is stage 3's, and it is six tests that do not exist rather
than a feature that does not work.** The behaviour they name is on disk: a class capture reaches the
`Parses` predicate at `crates/nvs-types/src/routes.rs:1750`'s diagnostic and a command argument at
`crates/nvs-types/src/commands.rs:249`, both verified by hand this session against
`target/debug/nvs.exe`.

**Still open and not code this stage needed:** `tryParse` is unreachable on a user implementor (the
playbook bullet has the shape), nothing binds a `#[Query]` parameter, and a class-typed `#[Query]`
cannot carry a default (`E0451` wants a literal of the declared type). Stage 4's `cargo-named` checks
still name five `-p nvs-runtime` tests that do not exist, one of which states the answer this goal's
§ *Standing decisions* settled the other way — check names to amend, not work to do.

## Next group

**Stage 3: the predicate's own tests — one file set: `crates/nvs-types/tests/routes.rs`,
`crates/nvs-types/tests/commands.rs`, `crates/nvs-stdlib/src/registry.rs`.** All six names are
`docs/agent/loop-goal.toml`'s stage-3 `cargo-named` lists, verbatim.

- [ ] **The three binding sites** — `a_user_class_implementing_parses_may_be_a_route_capture` and
      `a_user_class_implementing_parses_may_be_a_query_parameter` go at
      `crates/nvs-types/tests/routes.rs:1088`, after the `#[Query]` marker test;
      `a_user_class_implementing_parses_may_be_a_command_argument` at
      `crates/nvs-types/tests/commands.rs:423`. `rule:security/route-capture-is-laundered-by-its-type`
      and `rule:routing/a-query-parameter-is-declared-like-a-capture` are what they pin.
- [ ] **The refusal and the two streamlining assertions** —
      `a_class_without_the_interface_is_refused_naming_parses_as_the_fix`,
      `core_uuid_reaches_the_roster_through_the_interface_and_not_its_name` and
      `a_parses_capture_names_no_closed_set_and_changes_no_route_rank`, all at
      `crates/nvs-types/tests/routes.rs:1088`. The diagnostic the first reads is
      `crates/nvs-types/src/routes.rs:1750` and prints `or a class implementing \`Parses\``; the
      closed set the third asserts nothing about is `crates/nvs-types/src/routes.rs:1768`.
- [ ] **The stdlib half** — `implements_parses_is_true_for_core_uuid_and_false_for_a_class_without_both_members`
      and `implements_parses_requires_try_parse_to_answer_the_nullable_self`, in the test module at
      `crates/nvs-stdlib/src/registry.rs:3388`, over the predicate at
      `crates/nvs-stdlib/src/registry.rs:2591`.

## Backlog

- `Parses` has no rule fragment of its own; it is described in `crates/nvs-hir/src/interfaces.rs:61`
  and inside `rule:security/route-capture-is-laundered-by-its-type` — `docs/rules/classes/`.
- `tryParse` unreachable on a user implementor — the playbook bullet, `crates/nvs-types/src/iter_lib.rs:123`.
- Stage 4's five `-p nvs-runtime` check names to amend — `docs/agent/loop-goal.toml`.
- A `#[Query]` parameter binds nothing yet, and a class-typed one cannot carry a default.
