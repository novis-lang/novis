# Handoff

## State

**ADR 0096 is landed whole.** § 4's opt-out was the last open part and is now `check_csrf_opt_out`
(`crates/nvs-types/src/routes.rs:410`), reporting `E0764` where a `csrf: false` sits beside a method
whose every `#[Route]` names a verb outside `UNSAFE_VERBS` (`routes.rs:114`). The verbs are read from
the method's whole attribute list, not from the row, because § 1a gives one `#[Access]` to every
`#[Route]` a method carries; only `false` is refused, and a verb `verb_of` cannot read counts as
unsafe so the roster walk's diagnostic is not doubled.

**`routes.rs`'s known gap 2 is closed**: `check_class_routes` (`routes.rs:351`) collects every
`#[Route]` on a method and calls `collect_route` once per attribute, so ADR 0046 § 3's repetition is
one method serving several verbs. ADR 0110 § 1's exception rides in `check_table` (`routes.rs:993`) —
a repeated `name` is allowed when the prior row shares both `handler` and `path`, and the
duplicate-*route* rule above it is untouched.

**The one gap left in that module is gap 1** (`routes.rs:75`): a `#[Query]` outside a `#[Route]`
method binds nothing and is not refused.

**The acceptance check that fails is item 6's**,
`a_command_table_is_built_from_the_program_enumeration` — ADR 0086 § 6's command table, untouched by
this group and open by design, not a regression.

**Item 15 in `docs/agent/loop-goal.md` still owns M4's seventeen `nvs-ir` lowering refusals**,
unchanged and standing by design; the ratchet is `CEILING` at `crates/nvs-ir/tests/refusals.rs:66`.

## Next group

**One file set: `crates/nvs-types/src/attributes.rs`, `crates/nvs-types/src/routes.rs`,
`crates/nvs-types/src/commands.rs` and `crates/nvs-diagnostics/src/lib.rs`** — the two closed-roster
markers that mean nothing away from the declaration they mark, plus the walk that sees every method.
No acceptance check names these. Next free diagnostic code is `E0765`, after
`crates/nvs-diagnostics/src/lib.rs:2081`.

- [ ] **A `#[Query]` outside a `#[Route]` method is refused.** `routes.rs`'s gap 1 (`routes.rs:75`),
      ADR 0102 § 3. `query_params` (`routes.rs:845`) is reached only from a route method, so the
      refusal belongs to the per-method walk in `attributes.rs:68` — the one that already calls
      `check_one_access` and holds `m` and therefore its parameter list. `crate::derive::QUERY` is
      `crates/nvs-types/src/derive.rs:151`.
- [ ] **A `#[Command]`'s own gap 2, the same shape.** `check_class_commands`
      (`crates/nvs-types/src/commands.rs:91`) reads its marker only on `#[Command]` methods; read
      that crate's `# Known gaps` for the exact wording before writing the refusal, because the two
      markers should share one diagnostic or plainly not share one.
- [ ] **A `.nvst` case for each refusal**, `--EXPECTF-ERROR--` under `tests/conformance/`, once both
      diagnostics exist — the indentation widens with the line number, so write it against the real
      output.

## Backlog

- Gap 1's sibling question: whether `#[Access]` on a method with no `#[Route]` is also refused — ADR
  0096 § 1a says one per method and stops there (`docs/adr/0096-…md` § 1a).
- ADR 0086 § 6's command table, item 6's failing acceptance check (`docs/agent/loop-goal.md`).
- M4's seventeen `nvs-ir` lowering refusals, item 15 (`docs/agent/loop-goal.md`).
- `Core\Router::match` and `::url` reverse the table; the rows now exist for them
  (`docs/adr/0102-…md` § 6, `crates/nvs-types/src/links.rs`).
- The 1000-case conformance corpus count, M4's residue (`docs/implementation-plan.md`).
