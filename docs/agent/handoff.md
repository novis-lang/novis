# Handoff

## State

**Goal `m4-refusals` — Stage 7's `iterable` and `callable` rows are landed; one row is open.**
`python tools/holes.py` still reports **3** refusal sites, `UNATTRIBUTED: 0`, **15** guarded, and
`crates/nvs-ir/tests/refusals.rs`'s `CEILING` is **3** to match: `lower_type_test`'s panic still stands,
naming a shape, an element type no tag decides and `rule:types/callable-signature`'s written signature.

- `is iterable` is `TestShape::Any` of `Tag(Ty::Array)`, `Class("Iterable")` and `Class("Iterator")`
  (`crates/nvs-ir/src/lower/expr.rs:5592`) — the three rows `rule:iteration/foreach-subjects` accepts,
  each of which already existed, so the row added no mechanism. The array tag goes first, being the one
  member that reaches no descriptor.
- `is callable` is the descriptor walk against `crate::lower::CLOSURE_MARKER` (`$closure`,
  `crates/nvs-ir/src/lower/mod.rs:3562`): one `conforms` edge on each of the two environment classes
  `lower/closure.rs` synthesizes, and one field-less descriptor pushed into every program whether or not
  the file writes a closure. `$` cannot start an identifier, so nothing can implement the marker by hand.
- Nothing is blocked.

## Next group

**Stage 7: `is` over a shape, and over an `array<T>` whose element type no tag decides** — one file set:
`crates/nvs-ir/src/lower/expr.rs` and `crates/nvs-ir/src/lower/mod.rs`, plus `crates/nvs-stdlib/src/arr.rs`
and `crates/nvs-runtime/src/helpers.rs` for the walk the shape row calls. `docs/agent/loop-goal.md`
§ *Stage 7* is the spec and `rule:types/type-test` is the rule.

- [ ] **The shape row — decide which walk answers it, then call that one** — stage 7's prose says "the
      field walk `as` performs for the same type", and **`as` performs none**:
      `crates/nvs-ir/src/lower/convert.rs:1158`'s `lower_checked_downcast` has no shape arm, and the walk
      that exists is `Core\Arr::shapeAs`'s (`crates/nvs-stdlib/src/arr.rs:5932`, which throws, over
      `crates/nvs-runtime/src/helpers.rs:1534`'s per-field rows). An answering mode is a second entry
      point to that one walk, not a second walk — `rule:types/type-test`, `rule:core-api/shape-rules`.
      The arm goes beside the `iterable` one at `crates/nvs-ir/src/lower/expr.rs:5592`.
- [ ] **The element row — `array<Foo>`, an array of shapes, an array of unions** —
      `crates/nvs-ir/src/lower/mod.rs:3831`'s `array_element_tags` answers `None` for an element no tag
      decides, which is what makes `TestShape::ArrayOf`'s word unbuildable; the row is that walk given a
      per-element *test* instead of a tag word, so it composes with the arm above.
      `crates/nvs-ir/src/lower/expr.rs:5126` is the `ArrayOf` arm it joins — `rule:types/type-test`.
- [ ] **Then the panic and the ceiling** — with both rows in, `crates/nvs-ir/src/lower/expr.rs:4985`'s
      `panic!` covers only the written callable signature below, so either that row lands too and the
      panic goes with `CEILING` at 2, or the panic keeps it and its doc comment names the backlog item
      that owns it (`crates/nvs-ir/tests/refusals.rs`'s `CEILING`).

## Backlog

- `$x is callable(int): string` — `rule:types/callable-signature`'s written signature is a row in
  `rule:types/type-test`'s table that stage 7's list omits, and it **cannot be answered correctly
  today**: a closure carries `FN_ARITY` and `FN_PARAM_TAGS` but nothing that names its return type, so
  the marker walk would answer `true` for a closure of any signature. A decision, not a slice.
- `is Iterable<int>` and `is Iterable<string>` answer the same, the class row comparing the label alone
  — `rule:iteration/concrete-generic-implements`, and true of `instanceof` before this goal.
- The stale prose in `docs/plan/m4.md` (its 1000-case figure and `done*`) belongs to goal `plan-truth`.
