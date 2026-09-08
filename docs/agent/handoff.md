# Handoff

## State

**Goal 18, stages 1 to 3 are complete.** `Core\Arr::shapeAs<T>(array<mixed> $a): T` is a registry
row, a card, a body and an `address()` arm in `crates/nvs-stdlib/src/arr.rs`, and
`crates/nvs-stdlib/src/registry.rs`'s `WRITTEN_CLASS_MEMBERS` opens the type-argument door for it.
`rule:types/arrays`'s roster names four members now.

**The walk is `crate::json`'s and there is exactly one.** `shapeAs` calls `crate::json::hydrate`,
which `Core\Json::decodeAs` and `Core\Request::jsonAs` already reach, so the field list, the
presence column, the dotted paths and the one collected `ParseError` are shared. What forked is a
single bit — `crate::json::Reading`, a field on `Contract` — because an array's entries carry no
wire types: under `Reading::Values` a scalar field converts through `nvs_runtime::to_int`,
`to_uint`, `to_float` and `value_to_string`, which are the rows `$mixed as int` already goes
through, and under `Reading::Wire` a JSON document's own types are matched exactly as before.
`Reading`'s own doc comment in `crates/nvs-stdlib/src/json.rs` is that fork's home. Nothing new
entered `rule:types/conversion`'s table, and no row lands on `bool`, so a checkbox's `"1"` is a
failed field rather than a `true`.

`crate::json::scalar` now answers with an **owned** reference — the retain moved inside it from
its two callers — because `Reading::Values`'s `→ string` row builds a string where the strict read
refused outright, so "did this arm allocate" stopped being a question a caller could answer.

The failing acceptance check (`examples/input-shapes.nvs`) is stage 5's fixture: an unwritten
artefact, not a regression.

## Next group

**Stage 4: the two members at the boundary** — one file set: `crates/nvs-stdlib/src/request.rs`,
`crates/nvs-stdlib/src/uri.rs`. loop-goal.md § *Stage 4* is the specification and the four check
names in `docs/agent/loop-goal.toml` are the question set; stage 3's member is the whole mechanism,
so each of these is a row plus one call.

- [ ] **`Core\Request::postAs<T>` — the row, the card, the body and the `address()` arm** —
      `crates/nvs-stdlib/src/request.rs:324` is `post`'s row, `crates/nvs-stdlib/src/request.rs:1991`
      its body and `crates/nvs-stdlib/src/request.rs:1340` the `address()` arm. Signature is
      `postAs<T>({name?: string}): T` — `rule:core-api/shape-rules` R15's one row, not two — so the
      helper is `args: [4]`: the three written-type constants, then the options bag's one option.
      `crates/nvs-stdlib/src/arr.rs`'s `nvs_core_arr_shape_as` is the body to copy: read slots 0/1/2,
      `crate::json::check_codec`, retain the subject, then `crate::json::hydrate(..., Reading::Values,
      "Core\\Request::postAs")`. The whole-form read is the array `post` already indexes into, and
      **this does not open a public `post(): array<mixed>`**. Add `(r"Core\Request", "postAs")` to
      `crates/nvs-stdlib/src/registry.rs:2389`.
- [ ] **`Core\Request::queryAs<T>` — the same four edits over the query string** —
      `crates/nvs-stdlib/src/request.rs:243` is `query`'s row and
      `crates/nvs-stdlib/src/request.rs:1928` its body; `crates/nvs-stdlib/src/uri.rs:2348` is
      `parse_query`, which already answers the entire array. Roster entry beside `postAs`.
- [ ] **Three `.nvst` cases each**, over goal 16's request sections and goal 17's builder —
      `tests/conformance/core/an-array-becomes-the-shape-it-was-asked-for.nvst` is the shape to
      vary, and `crates/nvs-stdlib/src/arr.rs:6016` is `shape_of`, the fixture a `-p nvs-stdlib`
      test borrows for both halves of an inline shape's contract.

## Backlog

- Stage 5's fixture `examples/input-shapes.nvs`, which is the failing acceptance check — loop-goal.md.
- Spec §§ 6 and 15's rosters still do not list `shapeAs` — loop-goal.md § *Standing decisions*.
- `docs/reference/core/Arr.md`, if that page exists, owes `shapeAs` a paragraph — playbook, *Tooling*.
- `Reading::Values` over a *nested* class field is threaded but has no case; an inline shape nested
  in a shape still erases to `CodecTy::Opaque` — `crates/nvs-stdlib/src/json.rs` § *Known gaps*.
