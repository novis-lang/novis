# Handoff

## State

**Goal `decided-closures`, stage 3 — the checker and the front end.** Stages 1 and 2 are closed and
their checks pass. Stage 3's own three checks are red only because their tests are not written yet;
one of the two tests the `nvs-types` check names now exists.

`crates/nvs-runtime/src/record.rs` gap 1 is **built and its item deleted** — the register is empty
and `python tools/owners.py --closes decided-closures` no longer names that file.
`rule:security/secret-qualifier` now has a container axis: a `secret` value may be written into an
array element or a shape field only where that position's own type carries the qualifier, `E0824`
where it does not. The refusal is `crates/nvs-types/src/expr/quals.rs`'s
`reject_secret_into_container`, reached from the array literal, the object literal and both
spellings of an element write; `array<secret string>` and `{token: secret string}` were already
expressible and are what a program writes instead. **An argument list steps aside**, through
`Env::in_call_argument` (`crates/nvs-types/src/lib.rs`), because the three positions
`rule:security/secret-sinks-refuse` leaves open — a bound database parameter, a process argv, an
outbound request — are each written as an `array<mixed>` argument. That carve-out is guarded by
`a_secret_bound_as_a_database_parameter_is_still_accepted`.

The rule fragment was amended in the same slice and `python tools/rules.py --render` run. Nothing is
blocked.

## Next group

**Stage 4 of the goal prose, stage `3 the checker` of the checks: the front end's own four gaps** —
one file set: `crates/nvs-syntax/src/`, with `crates/nvs-types/tests/` and `docs/rules/` for the
first item's test and fragments.

- [ ] **`crates/nvs-syntax/src/casing.rs:66` gap 1 — a `type` alias's name is PascalCase like a
      class.** The reporter to call is `check_type_name` at `crates/nvs-syntax/src/casing.rs:224`,
      with a `"type alias"` category; the two sites are the file-scope form, which needs a
      `StmtKind::TypeAliasDecl` arm beside `crates/nvs-syntax/src/casing.rs:379`, and the member
      form at `crates/nvs-syntax/src/casing.rs:544`, which is an explicit empty arm today. This is a
      rule change: `rule:core-api/identifier-casing`'s scope table gains the row and
      `rule:types/type-alias` states it, both amended in the same slice with no record opened, then
      `python tools/rules.py --render`. The acceptance check names the test
      `a_type_alias_that_is_not_pascal_case_is_refused` under `cargo test -p nvs-types`, and no
      `nvs-types` fixture runs the casing pass — see the playbook bullet under *Writing a test
      case*, which is this session's.
- [ ] **`crates/nvs-syntax/src/lib.rs:103` gap 2 and `crates/nvs-syntax/src/lib.rs:109` gap 3 — M1's
      two.** `use function` / `use const` get the refusal naming
      `rule:classes/no-free-functions-or-constants` rather than a generic parse error, and every
      keyword spelling as a name segment past the first gets conformance coverage instead of a
      spot check. The acceptance check names both tests:
      `use_function_and_use_const_get_the_targeted_refusal` and
      `every_keyword_spelling_is_a_name_segment_past_the_first`, in `nvs-syntax`.
- [ ] **`crates/nvs-syntax/src/lib.rs:92` gap 1 — a local declared with a bare inline shape type
      gets the targeted error** pointing at `type Point = {x: int}; Point $p;`, which is the decided
      answer: the alias form is kept and the message is what changes.

## Backlog

- `Core\Json::encode(["token" => $s])` written inline is still not refused —
  `crates/nvs-types/src/expr/quals.rs`'s `reject_secret_encoded_argument` reads the argument's type,
  which is `array<mixed>`; the enqueue and log sinks walk the written literal and it could too.
- A failed `Core\Test` assertion renders its operands into a record, and no sink rule asks whether
  one carries `secret` (`rule:security/secret-sinks-refuse`'s terminal bullet names a test report).
- Stage 3's remaining items: `embedded.rs` gap 1, `hierarchy.rs` gap 1, `requires.rs` gap 1,
  `defaults.rs` gap 1, `response.rs` gap 1 — the goal file's stage 3 list.
- Stage 4 is `crates/nvs-stdlib/src/` and is the goal's largest, the prepared-pattern channel being
  its one ADR slot.
