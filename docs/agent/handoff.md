# Handoff

## State

**Goal 1. Stage 0 — ADR 0061 § 3's enumeration — is closed.** All three named acceptance tests are
on disk and pass: `an_interface_enumeration_is_sorted_by_qualified_name` and
`an_abstract_class_is_not_enumerated` in `crates/nvs-hir/src/hierarchy.rs`, and
`an_implementor_without_a_no_argument_constructor_is_named` in `crates/nvs-types/src/program.rs`.

**The third one moved crate, and the goal files moved with it.** `E0744` is reported by the checking
pass, so no `-p nvs-hir` test can reach it; both `docs/agent/loop-goal.toml` and
`docs/agent/goals/1-core-depth.toml` now list it under the `nvs-types` check, and the two files are
byte-identical again. The playbook's *Tooling* section holds the trap.

**The sort case is not the pair the last handoff named.** `App\Sub\A` against `App\Beta` sorts
identically under the segment key and a rendered-string key, so it separates no implementation; the
fixture uses `App\Sub\A` against `App\SubA`, where `\` (0x5C) sorting above `A` makes the two keys
disagree. The test's own doc comment owns why the segment key is the answer, and the playbook's
*Writing a test case* section holds the general trap.

**`python tools/verify.py` is still red at `crates/nvs-ir/tests/refusals.rs:179`** — the same 17
unattributed lowering refusals the goal switch orphaned, untouched by this session and unrelated to
it. It is the first failure, so the gate stops there and never reaches the `.nvst` trees or clippy.
`cargo test -p nvs-hir` and `cargo test -p nvs-types` are both green, which is what stage 0 runs.

**The two frozen fixtures are what fails next**, unchanged from the last session's reading:
`examples/routes.nvs` is `E0405` (`Core\Router` has no `url`/`urlAbsolute` — there is no
`nvs-stdlib` router module at all), `examples/commands.nvs` is `E0726` ×4.

## Next group

**One file set: `crates/nvs-types/src/attributes.rs`, `crates/nvs-types/src/derive.rs`,
`crates/nvs-stdlib/src/cli.rs`.** The third slice leaves it — take it only with room to spare.

- [ ] **`Core\Command` and `Core\Option` become shape-typed `type` aliases**, which is exactly
      `examples/commands.nvs`'s four `E0726`s. The refusal is reported at
      `crates/nvs-types/src/attributes.rs:200`; the closed roster it is checked against is
      `ATTRIBUTES`, whose rule is `crates/nvs-types/src/derive.rs:15`'s module doc ("nothing else is
      ever matched by name"). ADR 0046 § 2, ADR 0086 § 6.
- [ ] **Run `target/debug/nvs.exe examples/commands.nvs` against its frozen `want`** (the `exact`
      check in `docs/agent/loop-goal.toml`) and close whatever the aliases alone do not. The fixture
      reaches the aliases through `Core\Program::implementing<App\Command>()`, so the scan half is
      already landed and green.
- [ ] **`Core\Router::url`/`urlAbsolute`** — a whole new `Core` class, not a row: nothing under
      `crates/nvs-stdlib/src/` declares one, so it is a new module plus a `CLASSES` entry at
      `crates/nvs-stdlib/src/registry.rs:710`, and the four edits in
      [conventions.md](conventions.md). ADR 0077 § 4, ADR 0102 § 6. **Different file set** — a
      session that has spent its budget on the two above leaves this whole.

## Backlog

- `crates/nvs-ir/tests/refusals.rs:179` — 17 lowering refusals with no attribution, orphaned by the
  goal switch; `verify.py` stops there. Owner: `nvs-ir`'s own module doc.
- Reading either compile-time table back at run time is a member nobody has specified — ADR 0077 § 4
  and ADR 0086 § 6 expose no enumeration, and both frozen fixtures work around it.
- `Core\Router::match`, `Core\Command::run` and the terminal are out of goal 1's scope by its own
  § *Standing decisions*.
- M4's residue: the 1000-case corpus count, met as the suite grows — `docs/implementation-plan.md`.
