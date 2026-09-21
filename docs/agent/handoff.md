# Handoff

## State

Goal `core-db-rows-and-1-more` — the generated dossier goal over `Core\Db\Rows` and `Core\Db\Schema` —
has 9 of its 11 items landed. All six `Core\Db\Rows` members carry their feature proofs, and so do
`Core\Db\Schema::fromArray`, `::toArray` and `::planAgainst`; `python tools/dossier.py --id '<feature>'`
prints `complete.` for every one of them. Nothing is blocked. What is left is `applySafe` and
`applyIncludingRisky`.

Three things the next session should not re-derive. **A test proof is often a `covers:` marker on a case
that already exists**: `planAgainst` owed two and wrote none, because
`tests/conformance/core/db-schema-plans-every-difference-and-apply-safe-closes-them.nvst` and the Rust
`plan_against_needs_only_the_db_connect_a_program_already_holds` already pinned it and only needed
attributing. The two members left have the same neighbours —
`db-schema-apply-safe-refuses-what-apply-including-risky-runs.nvst`,
`db-schema-reports-a-table-it-does-not-declare-and-never-drops-it.nvst`, and the Rust
`applying_without_the_db_schema_capability_throws_naming_it` and
`apply_safe_refuses_a_plan_holding_a_step_that_is_not_safe` — so read those four before writing a case.

**`nvs.toml`'s `Core\Db\Schema` block now carries planAgainst's five programs**, each with
`connect = ["notes"]`, which is the whole of what planning needs. A program that *applies* holds
`schema = ["notes"]` beside it, as `fromArray`'s third example does. `[db.notes]` is `:memory:`, so
every program creates the tables it wants to find.

**A plan against a table a proof wrote by hand is not empty unless the types match**, and the playbook
bullet under *Writing a test case* is that trap in full.

## Next group

**One file set for both:** `crates/nvs-stdlib/src/db/schema.rs` (`mod tests`, for a `covers:` marker),
`tests/conformance/core/db-schema-apply-*.nvst` (the same), `nvs.toml` where a program applies a schema,
and each feature's own proof trees. One slice is one feature with all of `rule:testing/feature-proofs`.

- [ ] **`Core\Db\Schema::applySafe`** — owes examples, hostile, perf, tests. The member is
      `crates/nvs-stdlib/src/db/schema.rs:710` and the card `crates/nvs-stdlib/src/db/registry.rs:1636`;
      the claims worth pinning are that the whole plan is refused when one step it would run is not
      `Safe`, that the refusal names that step and the entry point which would run it, and that
      `db.schema` is asked for by name.
- [ ] **`Core\Db\Schema::applyIncludingRisky`** — owes examples, hostile, perf, tests. The member is
      `crates/nvs-stdlib/src/db/schema.rs:726` and the card `crates/nvs-stdlib/src/db/registry.rs:1645`;
      the claim worth pinning is that it runs the steps `applySafe` refuses and still runs no reported
      drop, so a table the schema does not declare survives it.

## Backlog

- Members other than `Core\Db\Schema::fromArray` that turn a Novis array into a Rust tree have not been
  probed for the unbounded recursion `DEPTH_LIMIT` fixed — `crates/nvs-stdlib/src/db/schema.rs:34`.
- `Core\Db\Plan::steps` and the four `Core\Db\Plan\Step` readers are attributed by no `covers:` marker
  either, though `tests/conformance/core/db-plan-*.nvst` pins all of them.
