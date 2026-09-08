# Handoff

## State

**Goal 18, stages 1 and 2 are complete; stage 3's hydration walk is landed and the two members that
already had a type-argument door use it.** `Core\Json::decodeAs<{n: int}>("…")` and
`Core\Request::jsonAs<{…}>()` hydrate an inline shape today: the walk reads slot 2's
`nvs_runtime::ShapeCodec` with `Value::as_shape_codec`, and where it is `Some` the field list, the
nested-class lookup and the build door all come from the contract instead of the descriptor.
`crates/nvs-stdlib/src/json.rs`'s module doc § *A shape is a second contract, not a second walk* is
that fork's home, and `decode_field`'s own doc owns the two answers a shape gives differently — an
absent optional key is `Value::unset()`, and a `?T` field is `as ?T`
(`rule:expressions/nullable-conversion`), which answers `null` where a class contract would report an
issue. Everything else is one walk and one collected `ParseError`.

**No rule changed and no ABI changed**, so ADR 0159 is still unopened: the three leading constants
landed last session and this one only read the third. `Core\Json::encode` now skips a never-written
slot when it spells a shape, so a hydrated `{s?: string}` round-trips as an absent key rather than
refusing.

**What stage 3 still owes is the member itself** — `Core\Arr::shapeAs<T>` — and the door's diagnostic,
which still says a class is the only thing a type argument may name. The failing acceptance check
(`examples/input-shapes.nvs`) is stage 5's fixture, an unwritten artefact rather than a regression.

## Next group

**Stage 3: the converter member, and the door it comes through** — one file set:
`crates/nvs-stdlib/src/arr.rs`, `crates/nvs-stdlib/src/registry.rs`,
`crates/nvs-types/src/expr/args.rs`. The diagnostic is first because it is independent and cheap; the
member is last because the coverage gate fails the moment a registry row has no conformance case.

- [ ] **`E_TYPE_ARG_NOT_A_CLASS` still says a class is the only answer** — the message and help at
      `crates/nvs-types/src/expr/args.rs:1560` were written when a type argument was a class or an
      `array<C>` of one, and the arm above them (`crates/nvs-types/src/expr/args.rs:1546`) accepts an
      inline shape for every row of `WRITTEN_CLASS_MEMBERS`. Amend both, and with them
      `rule:types/arrays`'s type-argument-door sentence, which loop-goal.md § *Stage 3* item 1 says
      names three members rather than two.
- [ ] **`Core\Arr::shapeAs<T>` — the row, the card, the `address()` arm, the roster entry and three
      cases** — `crates/nvs-stdlib/src/arr.rs:92` is the class, `crates/nvs-stdlib/src/arr.rs:2161`
      the `address()` arm, and `crates/nvs-stdlib/src/registry.rs:2389` is `WRITTEN_CLASS_MEMBERS`,
      which is the whole of what opens the type-argument door for a member. The body is one call —
      `crate::json::hydrate(ctx, class, shape, document, list, "Core\\Arr::shapeAs")` over the
      subject itself, since the document that walk takes is an `NvsArray` and an `array<mixed>`
      already is one, so nothing is decoded on the way in. `args: [N]` is three more than the row's
      arity. loop-goal.md § *Stage 3* items 2 to 4 are the specification, and
      `tests/conformance/core/json-decodes-into-an-inline-shape.nvst` is the question set to vary
      rather than repeat.

## Backlog

- Stage 4's two wrappers, `postAs<T>` and `queryAs<T>` — loop-goal.md § *Stage 4*.
- Stage 5's fixture `examples/input-shapes.nvs`, which is the failing acceptance check — loop-goal.md.
- `Core\Db\Row`'s hydration has no shape door at all (`crates/nvs-stdlib/src/db/row.rs:122`); it is a
  second walk over a row rather than a document, and nothing has asked it for one yet.
- An inline shape reached as a *field* of another shape or class is still `CodecTy::Opaque` —
  `crates/nvs-stdlib/src/json.rs`'s known gap 2 and `nvs_types::derive::codec_ty` own it.
