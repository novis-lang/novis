# Handoff

## State

**Goal `unowned-sweep`, stage 2 — landed.** ADR 0147's mechanism is spent end to end and both levels
of `rule:core-classes/uri-removable-components` are on disk: `with` clears a component, and the new
pair `queryParameter` / `withQueryParameter` edits one query parameter by a name chosen at run time.
All eight `-p nvs-stdlib` names the stage-2 check lists exist and pass; the other two stage-2 checks'
names were already on disk. The rule's status is now `shipped`, and `docs/rules/`, `docs/novis.md`
and `docs/ground-rules.md` are re-rendered.

`crates/nvs-stdlib/src/uri.rs` has **no `# Known gaps` section left** — gap 1 was exactly this pair.
The pair composes that module's own `query` slot, `parse_query` and `build`, so the query string
gained no second canonicalization; it inherits `buildQuery`'s one divergence, where a name spelling
brackets and an array value write the same bytes, and the writer's doc comment is that fact's home.

`Core\Queue`'s gap 1 stays closed as a **decision** in `crates/nvs-stdlib/src/queue.rs`'s module doc:
`limits` and `grants` wait on enforcement in `nvs_types::expr::isolate`, not on a spelling. Nothing
is blocked. **Stage 3 has none of its three `-p nvs-types` names on disk**, and is the next group.

## Next group

**Stage 3: `array<T>` accepts a covariant read, `rule:types/unions-and-mixed`'s assignability
relation** — one file set: `crates/nvs-types/tests/arrays.rs`, over
`crates/nvs-types/src/expr/assign.rs`. The user's decision, recorded and not re-argued
(`docs/agent/loop-goal.md` § *Standing decisions*).

- [ ] **`an_array_of_int_satisfies_an_array_of_int_or_string_parameter`** — read
      `crates/nvs-types/src/expr/assign.rs:172` **before writing any code**: that arm already
      recurses `is_assignable` on the element type, so the widening may already hold and the item is
      then the test that pins it rather than a change. The test's home is the end of
      `crates/nvs-types/tests/arrays.rs:153`. `rule:types/unions-and-mixed`.
- [ ] **`the_widening_accepts_strictly_more_programs_and_breaks_none_that_compile_today`** — the
      other side of the same bound, over `crates/nvs-types/src/expr/assign.rs:60`'s entry point: an
      `array<int|string>` argument still does *not* satisfy an `array<int>` parameter, so the
      relation is one-way. Same file set. `rule:types/unions-and-mixed`.
- [ ] **`a_write_through_the_widened_parameter_does_not_reach_the_callers_array`** — the check files
      this under `-p nvs-types`, which cannot observe a runtime write; decide from
      `crates/nvs-runtime/src/array.rs:853`, where `set` calls `make_unique`, whether the claim a
      type test can carry is that an array parameter is a *value* parameter, and split the check if
      it is not (`docs/agent/playbook.md` § *Tooling*, the `loop-goal.toml`-check bullets).

## Backlog

- Stage 4's two small ones — the panic hook's request id, and `max_output` over a capture —
  `docs/agent/loop-goal.toml`, stage `4 hook and cap`.
- `docs/reference/core/Uri.md`'s example is at ten lines of output; a further member added there
  wants a second block rather than an eleventh line — `docs/reference/README.md`.
- Whether any other `Core` member should spend the three-state bag now that `Uri` has —
  `rule:core-api/omission-is-not-a-written-null`, and each member's own spec row.
