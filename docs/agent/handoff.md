# Handoff

## State

**M8 goal 5, stage 10 — the corpus.** The driver's acceptance failure was the stage-10 `nvs-suite`
check: its `cases` list named `tests/conformance/db/…`, a directory this corpus never adopted, which
is the playbook's own trap about a drafted directory layout. The list now names the corpus's flat
`core/db-…` paths and is six entries rather than nine, with a comment above it saying where the other
three went.

**Three of the six are new and green.** ADR 0067 § 5's two binding spellings, including a `:name`
written once and read twice; § 6's three typed-reader refusals, each asserted to *name the column*
rather than to have a frozen wording; and § 7's rollback-only flag surviving a `catch (Throwable)`
between `rollBack` and the closure's own return. All three run against a live `:memory:` SQLite
connection — the one backend a conformance case can really reach.

**Three drafted names were dropped rather than repathed**, each already measured by an earlier stage
of the same goal file: stage 9's `a_two_statement_literal_query_is_a_diagnostic`, and stage 8's
`an_enqueue_commits_with_the_write_that_made_it` and
`an_exhausted_job_reaches_the_dead_letter_table_and_is_not_discarded`. `docs/agent/goals/5-database.toml`
carries the identical edit, so the next `goal-switch.py` does not restore the old list.

**The `Core\Taint::assertTrusted` gap is untouched**, and it is now confirmed to be a whole class and
not a missing row: `grep` finds no `Core\Taint` anywhere in `nvs-stdlib`, so a diagnostic already names
a member the tree cannot resolve.

## Next group

**One file set: `crates/nvs-stdlib/src/registry.rs` with a new `taint.rs` beside it, widening to
`crates/nvs-types/src/core_lib.rs` on the third.**

- [ ] **Decide whether `Core\Taint::assertTrusted` earns a registry row, and record the decision.**
      ADR 0067 § 3 names it as the only way through `Settings.host`, which has no launderer, and
      `nvs_types::expr::args::check_shape_field`'s diagnostic now names it too. ADR 0024 § 3 owns the
      axis. The mark is the real question: `crates/nvs-stdlib/src/registry.rs:152` is `Qual`, whose
      `Launder` doc says a launderer names *the sink it launders for*, and this member launders for
      every sink — the `tainted` twin of `Qual::Reveal`. `crates/nvs-stdlib/src/secret.rs:41` is
      `Core\Secret`, the same escape hatch one axis over and the model to copy, `$reason` included.
- [ ] **If it earns one, write the five edits** — the row, the card, the body, the `address()` arm and
      three `.nvst` cases, all in a new `crates/nvs-stdlib/src/taint.rs` modelled on
      `crates/nvs-stdlib/src/secret.rs:41`, registered beside its siblings in
      `crates/nvs-stdlib/src/registry.rs:152`'s crate.
- [ ] **Then hold the roster the way the `secret` axis is held.** `crates/nvs-types/src/core_lib.rs:979`
      is where `Qual::Reveal`'s roster is closed and `crates/nvs-types/src/core_lib.rs:1007` is the
      predicate that reads it; `crates/nvs-types/src/expr/quals.rs:319` is the contagion half. The
      `tainted` axis needs the same three, or the new mark is a rule held nowhere near its row.

## Backlog

- Stage 10's `min_passing = 1350` is well under the corpus's real count — `docs/agent/loop-goal.toml`.
- `crates/nvs-types/src/intrinsics.rs` known gaps 5-7 still stand as written.
- `E0618`'s host-grant check is still untouched — `docs/adr/0067-core-db.md` § 3.
- The migration floor is 94% against 90%/74% on disk — `tools/check-migration.py`.
- `tests/conformance/reject/` has no `Core\Db` case at all; every refusal above is a runtime one.
