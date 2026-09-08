# Handoff

## State

**Goal 18. Stage 2's group is closed — all three items landed and verified, and both of the stage's
acceptance checks now name tests that exist.** Nothing in stage 2 needed designing: the optional marker
and the shape qualifier were already implemented, and every item was a test over landed behaviour.

`crates/nvs-syntax/src/parser/tests/ty.rs` gained the four names the grammar check lists. `{a?: int}`
and `{a: ?int}` are asserted to differ in *both* halves of the parse — `ShapeField::required` and the
field's `TypeKind` — with `{a?: ?int}` saying both at once, so neither half is derivable from the other.
`tainted {…}` is pinned on both sides of its bound: accepted over a shape carrying text, refused
(`E_TAINTED_SHAPE_HAS_NO_TEXT`) over one carrying none, and its distribution asserted through a nested
shape, a nullable and an `array<T>` element with a non-text field left exactly as written.

**Three `nvs-types` tests were renamed rather than duplicated**, having existed under other spellings:
`a_source_with_extra_fields_still_satisfies_the_shape`, `a_source_missing_a_required_field_does_not`
and `a_source_missing_an_optional_field_satisfies_the_shape` in `objects_and_shapes.rs`, plus
`a_tainted_shape_naming_no_text_field_is_refused` in `tainted.rs`. No other `[[check]]` in
`loop-goal.toml` named an old spelling — checked before renaming.

**The one name with no test was `a_tainted_shape_is_not_assignable_to_the_same_shape_unqualified`, and
the relation already refused it** — no code changed for it. The distribution makes it fall out of two
rules that already stand: the qualified shape's fields are `tainted string`, shape assignability is
field-by-field ordinary assignability, and that axis only widens one way. A shape is not a laundering
route, and no rule of its own says so.

**Stage 3 appears already green and stage 4 is where the next work is.** All seven `nvs-stdlib` names
stage 3 lists exist (`crates/nvs-stdlib/src/arr.rs:7306`–`7488`) and its three conformance cases are on
disk. Stage 4's members are implemented — `postAs` at `crates/nvs-stdlib/src/request.rs:342`, `queryAs`
at `:252` — but **none** of its six test names exists and **none** of its four conformance cases is on
disk.

## Next group

**Stage 4: the boundary — the six names and four cases over landed `postAs`/`queryAs`** — one file set:
`crates/nvs-stdlib/src/request.rs` (its inline `mod tests`, `:3685`) and `tests/conformance/core/`.
`rule:security/tainted-qualifier` and `rule:types/shape-type` are the specification, and the goal's
stage 4 prose is `docs/agent/loop-goal.md:"## Stage 4"`. As in stage 2, verify what is landed before
writing a name: the members are implemented, so most items are tests.

- [ ] **The four hydration names** — `crates/nvs-stdlib/src/request.rs:5676`, beside
      `post_reads_the_fields_a_files_walk_buffered`, which is the fixture shape these copy:
      `post_as_hydrates_the_whole_form_into_the_shape_it_was_given`,
      `post_as_with_a_name_hydrates_one_bracket_subtree`, `query_as_reads_the_query_string_the_same_way`
      and `post_as_may_follow_another_buffering_reader`. The code they ask about is
      `crates/nvs-stdlib/src/request.rs:2235` (`postAs`, over `form_of`) and
      `crates/nvs-stdlib/src/request.rs:2096` (`queryAs`, over `Core\Arr::shapeAs`'s one walk).
- [ ] **The body-claim pair** — `post_as_refuses_after_a_streaming_reader_has_consumed_the_body`, at
      `crates/nvs-stdlib/src/request.rs:2235`, where the content-type claim is read. Goal 16's
      body-read rule is unchanged and `postAs` takes the claim as one buffering reader; the playbook's
      `-p nvs-stdlib` bullets say what a fixture can and cannot build.
- [ ] **`an_unqualified_shape_at_a_request_call_site_is_diagnosed_naming_the_shape`** — the check files
      it under `-p nvs-stdlib`, but it is a *checker* diagnostic; the site that would raise it is
      `crates/nvs-types/src/expr/args.rs:1554`, where the written type argument's owner is decided.
      **Verify it is landed before writing the name**, and if it is not, that is the group's one piece
      of real work rather than a test.
- [ ] **The four stage-4 conformance cases** — the shape to copy is the neighbour already on disk,
      `tests/conformance/core/a-request-hydrates-its-body-into-a-declared-type.nvst:1`, and none of
      these four is written yet:
      `a-request-hydrates-its-form-into-a-declared-shape.nvst`,
      `a-request-shape-names-every-field-the-peer-got-wrong.nvst`,
      `a-request-shape-ignores-a-field-it-did-not-name.nvst` and
      `a-query-string-hydrates-into-a-declared-shape.nvst`. Never `--ORACLE--` in this suite.

## Backlog

- Stage 5's remaining checks, once stage 4 is green — `docs/agent/loop-goal.toml`.
- `docs/agent/handoff.md`'s stage-2 line numbers had drifted by one test; anchors age, names do not.
- `E0811` is still the next free type-band diagnostic code — `crates/nvs-diagnostics/src/lib.rs`.
