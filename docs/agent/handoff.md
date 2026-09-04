# Handoff

## State

**Goal 6, M7. ADR 0102 § 2 is closed on all three of its sides** — the runtime walk, the
program-visible reader, and now the acceptance check.
`no_methods_for_a_path_is_404_and_some_is_405_with_allow` lives in
`crates/nvs-runtime/src/routes.rs` and its check is `-p nvs-runtime`: the `404`/`405` decision is a
computation the table performs and the *program* sends, which ADR 0102 § 1 settles in one clause, so
the test writes the sender's arithmetic out as a closure and asserts the answer under it.
`crates/nvs-server/src/route.rs` keeps § 1's half and nothing of § 2. The playbook bullet under
*Tooling* is the general shape.

**`nvs_runtime::routes` gap 2 has split, and only its `Core\Uuid` half is open.** A `decimal`
capture converts through `crate::decimal::Decimal::parse` and refuses a segment that is not one, so
the § 5 narrowing a `decimal` route declares is now a property of the table. The `Core\Uuid` half is
**a placement decision, not a missing arm**: that parse is `nvs_stdlib::uuid`, one crate above
`nvs-runtime`, which cannot depend on it — and writing the `8-4-4-4-12` grammar a second time here
is what gap 4 refuses. The two candidates are stated in the module doc's own gap 2, which is that
decision's home.

**Unchanged and still triaged**: neither half of
`the_csrf_check_and_the_route_label_read_the_match_rather_than_matching_again` exists. ADR 0102 § 8
puts CSRF enforcement on the *server*, and `Core\Csrf::verify`'s seal/open primitive is
`nvs_stdlib::crypto`, which `nvs-server` does not depend on — so that half carries the same
placement question the `Core\Uuid` half does, while the `route` label half is ADR 0076 § 1's, in a
different subsystem. Its `_and_` makes it the playbook's conjunction shape.

**`orient.py`'s carried gap is closed**: `[context] adrs` gained ADR 0102 §§ 4 and 8 and ADR 0096
§ 4, and dropped § 2 with the stage it closed. `docs/agent/goals/6-server.toml` is byte-identical to
the live goal again — it had drifted by one hunk.

## Next group

**Two placements, each a rule stated in one crate whose reader lives in another.** The file set is
`crates/nvs-runtime/src/routes.rs`, `crates/nvs-stdlib/src/uuid.rs`,
`crates/nvs-stdlib/src/router.rs` and `crates/nvs-cli/src/main.rs`.

- [ ] **Decide where a `Core\Uuid` capture's acceptance lives, and close gap 2** (ADR 0102 § 5) —
      the arm is `crates/nvs-runtime/src/routes.rs:345`, the variant it would replace is
      `crates/nvs-runtime/src/routes.rs:115`, the producer is `crates/nvs-cli/src/main.rs:873`, and
      the crossing that would carry the value is `crates/nvs-stdlib/src/router.rs:841`. The two
      candidates are in that module's gap 2: move the 16-byte parse down beside
      `crates/nvs-runtime/src/decimal.rs:1`, or leave the conversion where it cannot be performed
      and say so. `decimal` landed this session and is the shape to copy.
- [ ] **`Core\Router\Match` answers no `Core\Uuid`** — `crates/nvs-stdlib/src/router.rs:841`'s
      `capture_value` gains the arm the moment the item above decides. Take it in the same session
      as item 1 or not at all; alone it is a one-line edit with nothing to test.
- [ ] **Split `the_csrf_check_and_the_route_label_read_the_match_rather_than_matching_again` into
      one name per half** (ADR 0102 § 8, ADR 0096 § 4) — the check block is
      `docs/agent/loop-goal.toml:3447`, the door is `crates/nvs-server/src/route.rs:57`, and the
      primitive the CSRF half needs is `crates/nvs-stdlib/src/csrf.rs:339`. The conjunction reports
      one open item where there are two, and the halves land in different crates.

## Backlog

- Gap 1: the walk is a linear scan, not ADR 0077 § 2's trie — `crates/nvs-runtime/src/routes.rs:52`.
- Gap 4: a capture's value is still percent-encoded; decoding is `nvs_stdlib::uri`'s —
  `crates/nvs-runtime/src/routes.rs:79`.
- `crates/nvs-runtime/src/commands.rs:69` gap 1 is the same `decimal` arm one table along, and is
  now the only place that conversion is still missing.
- ADR 0076 § 1's `route` metric label is emitted nowhere — the second half of the conjunction above.
