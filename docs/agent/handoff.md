# Handoff

## State

**ADR 0096 is landed except for where the decision goes.** `#[Access]` is `nvs_types::derive::ACCESS` on
`ATTRIBUTES`; `routes::check_access` holds one payload to § 1a's two rules a roster cannot state — `allow`
is required, and its value is an enum case or a class constant, which are one `ExprKind::ClassConstAccess`
— and `routes::check_access_declared` refuses a `#[Route]` whose method carries no sibling (§§ 1 and 3).
Three new codes: `E0760`, `E0761`, `E0762`.

**`allow` is `OptionTy::Mixed`**, the roster type's fifth row and the first with no type behind it: § 1a
declares the field `mixed` because § 2 is a promise *not* to know what a decision means, so `intern`
answers `None` and the value is checked as ADR 0046 § 2's constant and placed at nothing.

**`Core\Audience::Public` exists** — `nvs_stdlib::router::AUDIENCE`, one case, on `registry::ENUMS`. It is
what makes the fix for `E0762` a name that already resolves, and § 1a says it will not grow a second case.

**Every route fixture in the tree declares a decision now.** In `crates/nvs-types/tests/routes.rs` that is
the `with_access` helper (`tests/routes.rs:38`), which supplies it for every fixture that is about
something else; the two `core/` `.nvst` cases, the one `reject/` case and `examples/routes/Users.nvs`
carry it written out, and the reject case's `--EXPECTF-ERROR--` line numbers moved with the inserted lines.

**Three parts of ADR 0096 are not landed and are the next group**: the decision does not ride on the
`Route` row, a second `#[Access]` on one method is not refused, and `csrf: false` on a route whose every
verb is safe is accepted. `routes.rs`' module doc names the last two as the questions it does not ask yet.

**The acceptance check that fails is item 6's**, `a_command_table_is_built_from_the_program_enumeration` —
ADR 0086 § 6's command table, untouched by this group and open by design, not a regression.

**Item 15 in `docs/agent/loop-goal.md` still owns M4's seventeen `nvs-ir` lowering refusals**, unchanged
and standing by design; the ratchet is `CEILING` at `crates/nvs-ir/tests/refusals.rs:66`.

## Next group

**One file set: `crates/nvs-types/src/routes.rs`, `src/expr_table.rs`, `tests/routes.rs` and
`crates/nvs-diagnostics/src/lib.rs`.** All three finish ADR 0096; no acceptance check names them.

- [ ] **The decision lands on the `Route` row.** ADR 0102 § 8: the *dispatcher* enforces the access
      decision, so the declared name has to cross into `nvs-ir` on the row the way `query` already does —
      as the resolved string, since § 2 never asks what it means. `Route` at
      `crates/nvs-types/src/routes.rs:198`, filled in `collect_route` at `routes.rs:345`, crossing through
      `crate::expr_table`; `check_access_declared` at `routes.rs:313` is where the sibling is already
      found.
- [ ] **A second `#[Access]` on one method is refused, naming both.** ADR 0096 § 1a's last bullet: two
      decisions are two readings — conjunction or disjunction — and choosing silently is what § 3 exists
      to prevent. `check_access_declared` (`routes.rs:313`) holds the method's attribute groups already,
      but `crate::testing::attribute_named` answers with *one*, so this counts them itself. Next free
      code is `E0763`.
- [ ] **`csrf: false` on a route whose every verb is safe does not compile.** ADR 0096 §§ 1a and 4 — it
      needs the row's verb, so it follows the first slice. The safe/unsafe split is the contiguous tail
      from `Post` in `nvs_stdlib::router::METHOD` (`crates/nvs-stdlib/src/router.rs:83`), which that
      const's own doc says is a bound to take rather than a list to copy.

## Backlog

- ADR 0077 § 1's example (`docs/adr/0077-compile-time-routing.md:80`) writes a `#[Route]` with no sibling
  `#[Access]`, which no longer compiles; `docs/agent/doc-cleanup.md` owns that pass.
- A `#[Query]` or `#[Access]` outside a `#[Route]` method is not refused — `nvs_types::routes`' module
  doc gap 1, which is `nvs_types::commands`' gap 2 exactly.
- Only the first `#[Route]` on a method becomes a row — `routes.rs`' gap 2; ADR 0110 § 1 has nothing to
  except until it does.
- Item 6, the command table (ADR 0086 § 6), is the acceptance check still open — `docs/agent/loop-goal.md`.
- Item 15 owns M4's seventeen `nvs-ir` lowering refusals — `docs/agent/loop-goal.md`.
- `[context] adrs` now carries ADR 0096 §§ 1, 1a, 4 and ADR 0102 § 8, in `docs/agent/loop-goal.toml` and
  in `docs/agent/goals/1-core-depth.toml`; the next session should get them printed rather than sliced.
