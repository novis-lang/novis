# Handoff

## State

**Goal 18, stage 2 is complete — checker and lowering both.** A shape field carries a required bit,
`{name?: T}` writes it, assignability lets an optional key be absent, `tainted {…}` distributes, every
subscript and shape-property level of a `??`, `isset` or `empty` operand is marked guarded, and that
mark now reaches the machine: `InstKind::SlotGet` carries an `AbsentKey`
(`crates/nvs-ir/src/ir.rs:736`) filled from `ExprInfo::ShapeProperty::guarded`, the guarded read takes
`Ty::Tagged`, and `nvs-codegen` picks `nvs_object_slot_optional_get` off it over the one `RuntimeSig`.
`$p->a ?? "d"` and `isset($p->a)` over an absent optional field answer instead of throwing; the two
refusals that are not about presence — a non-object receiver, a `Tag::Unset` slot — still throw under
both answers. [ADR 0157](../decisions/0157.md) § 2 is the reasoning and `rule:types/shape-type`'s
optional-field paragraph the specification.

**Stages 3, 4 and 5 have not started.** The failing acceptance check (`examples/input-shapes.nvs`) is
stage 5's fixture and is still an unwritten artefact, not a regression.

The pack does not print the *next* stage's paragraph from `docs/agent/loop-goal.md`, which is what
choosing the next group reads — a `[context]` gap worth closing if a later session pays for it twice.

## Next group

**Stage 3: the converter** — one file set: `crates/nvs-stdlib/src/arr.rs`,
`crates/nvs-stdlib/src/registry.rs`, `crates/nvs-stdlib/src/json.rs`,
`crates/nvs-types/src/expr/args.rs`, `tests/conformance/core/`.

- [ ] **`Core\Arr::shapeAs<T>` is one row and one hydration walk** — the row goes at the tail of the
      `Core\Arr` roster, `crates/nvs-stdlib/src/arr.rs:694`, with the five edits a new member owes
      (the playbook's *a new core member owes* bullet is the recipe and its third conformance case).
      What is a failure is the goal's stage 3 § 4: an absent *required* key and a value `as` refuses,
      never an absent optional key, an extra key, or a `?T` field `as ?T` answers `null` for.
      `rule:types/shape-type` and `rule:types/conversion` are what it cites.
- [ ] **The failure is collected, not the first one** — one `ParseError` carrying every field that
      failed with its dotted path (`user.address.city`), built the way
      `rule:core-classes/derive-reports-every-field` already builds a derived decoder's, whose shape is
      at `crates/nvs-stdlib/src/json.rs:976`. Two field-error conventions in one runtime is the thing
      to avoid, so this reuses that one rather than opening a second.
- [ ] **The type-argument door gains its third member** — `rule:types/arrays`'s list is
      `Json::decodeAs<T>` and `Db::queryAs<T>` today; a written argument is bound at
      `crates/nvs-types/src/expr/args.rs:1484`, and `shapeAs` records no decode or row site there for
      the reason the comment above that line already gives for `decodeAs`.

## Backlog

- Stage 4: `Core\Request::postAs<T>` and `queryAs<T>`, `crates/nvs-stdlib/src/request.rs:255` —
  `docs/agent/loop-goal.md` § *Stage 4*.
- Stage 5: `examples/input-shapes.nvs`, the three reference pages, and stage 2's diagnostic corpus —
  `docs/agent/loop-goal.md` § *Stage 5*.
- `Json::decodeAs<T>` over an inline shape should call stage 3's walk rather than keep its own —
  `docs/agent/loop-goal.md` § *Stage 3* item 2.
- Goal 16's `json(): tainted mixed` against `rule:security/tainted-qualifier`'s grammar is still
  undecided in that rule's body — the goal's § *Standing decisions* says it is settled here.
