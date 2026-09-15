# Handoff

## State

Goal `m8-stdlib-depth`. **Stages 0, 2, 3 and 4 are done; stage 5 is half landed.** A `decimal` and a
`Core\Time\Instant` both cross the JSON wire as strings at both ends — `"19.99"` and RFC 3339 — and
`crates/nvs-stdlib/src/json.rs`'s *A value type crosses as text* section is that decision's one home.
What is left of stage 5 is the **inline shape reached as a field**, which is still gap 1 there and still
`CodecTy::Opaque` in `crates/nvs-types/src/derive.rs:551`. Nothing is blocked.

Of the stage's three checks, the `.nvst` one is green (the case carries the name the check asks for) and
the two Rust ones each want one more test: `decode_as_fills_an_inline_shape_field` in `nvs-stdlib`, and
the shape third of `a_decimal_instant_or_shape_field_erases_to_a_codec_it_can_decode` in `nvs-types` —
whose decimal and `Instant` thirds already exist at `crates/nvs-types/tests/derive.rs:853` under the
older name.

## Next group

**Stage 5: an inline shape on the JSON wire** — one file set: `crates/nvs-runtime/src/object.rs`,
`crates/nvs-types/src/derive.rs`, `crates/nvs-ir/src/lower/mod.rs`, `crates/nvs-codegen/src/lib.rs` and
`crates/nvs-stdlib/src/json.rs`. `rule:core-classes/derive-field-list`. The design below was worked out
against all five files this session; it is a proposal, not a landed fact.

- [ ] **A shape field erases to a wire type that carries its contract** —
      `crates/nvs-types/src/derive.rs:551` is the `_ => Opaque` arm that swallows `Ty::Shape`, and
      `crates/nvs-runtime/src/object.rs:669` is the `CodecTy::Opaque` it lands on. A shape field needs
      **two** resolved pointers and the erasure carries the label for each: the shape *class*
      (`nvs_types::derive::shape_class_label`, in `CodecField::class`, which
      `crates/nvs-codegen/src/lib.rs:1280`'s `nested_descs` already resolves) and the shape's own
      *contract*, which is a `nvs_runtime::ShapeCodec` and has nowhere to live today — a `ClassDesc`
      holds one field list per class and `crates/nvs-runtime/src/object.rs:768`'s `ShapeCodec` is
      addressed by a call site, not by a field. Two knots to expect: `codec_ty` holds the interner
      immutably and building a nested contract needs `&mut` for `without_null`
      (`crates/nvs-types/src/derive.rs:407`'s `shape_codec` is the call it would make), and the shape
      class has to exist in the unit even when no literal of it is written, which is
      `crates/nvs-ir/src/lower/mod.rs:3274`'s label registered from the field rather than from a site.
- [ ] **`decodeAs` fills a shape field** — `crates/nvs-stdlib/src/json.rs:1324`'s `Contract` gains a
      `shape_at(index)` beside `class_at`, and `crates/nvs-stdlib/src/json.rs:2131`'s `decode_nested`
      builds `Contract::new(class, shape_at(index))` where it now passes `None`; everything under that
      — `decode_fields`, `build_shape`, the `?T` and absent-key columns — already answers for a shape.
      The check names `decode_as_fills_an_inline_shape_field`; the `codec_class` helper at
      `crates/nvs-stdlib/src/json.rs:2764` is what the two landed tests build their descriptors with.
- [ ] **The erasure test takes the check's name and its third case** —
      `crates/nvs-types/tests/derive.rs:853` is `a_decimal_an_instant_and_bytes_field_erase_to_their_own_codec_type`,
      which the check spells `a_decimal_instant_or_shape_field_erases_to_a_codec_it_can_decode`. Rename
      it and add the shape row; `bytes` stays asserted, since it is the one erasure with no JSON door.

## Backlog

- A `#[Json\Derive]` class holding an `array<Instant>` has no case of its own — `decode_list` reaches
  `scalar` per element, so it should already work; unasserted (`crates/nvs-stdlib/src/json.rs:2371`).
- `crates/nvs-stdlib/src/db/mod.rs:283`'s skipped-field default now points at `crate::json`'s gap 2,
  which is where a decoder with no call site reaches a constant; goal `m8-db-queue` owns closing it.
- `docs/agent/carried-gaps.md:246` still describes json gap 1 as the whole roster; it is the inline
  shape alone now.
