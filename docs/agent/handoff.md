# Handoff

## State

**Goal 18, stages 1 and 2 are complete, and stage 3's hydration walk plus its type-argument door are
landed.** `Core\Json::decodeAs<{n: int}>("…")` and `Core\Request::jsonAs<{…}>()` hydrate an inline shape
today: the walk reads slot 2's `nvs_runtime::ShapeCodec` with `Value::as_shape_codec`, and where it is
`Some` the field list, the nested-class lookup and the build door all come from the contract instead of
the descriptor. `crates/nvs-stdlib/src/json.rs`'s module doc § *A shape is a second contract, not a
second walk* is that fork's home, and `decode_field`'s own doc owns the two answers a shape gives
differently.

**The door now says so out loud.** `E0465`'s message and help name the inline shape beside the class, and
`rule:types/arrays`'s type-argument-door sentence names the three members that have the door and the
three things that may be written at it. ADR 0159 is the record behind that — it is the goal's authorized
number, and it is now spent, so a further rule change in this goal has no record left to hang on.

**What stage 3 still owes is the member itself** — `Core\Arr::shapeAs<T>`. The failing acceptance check
(`examples/input-shapes.nvs`) is stage 5's fixture, an unwritten artefact rather than a regression.

## Next group

**Stage 3: the converter member** — one file set: `crates/nvs-stdlib/src/arr.rs`,
`crates/nvs-stdlib/src/registry.rs`. The five edits and the tests are one slice, because the coverage
gate fails the moment a registry row has no conformance case; the test scaffold is a slice of its own
only if it grows past the fixture below.

- [ ] **The scaffold a shape test needs, in `crates/nvs-stdlib/src/arr.rs`'s test module** — a
      `-p nvs-stdlib` test cannot borrow one from anywhere: nothing in the crate builds a
      `nvs_runtime::ShapeCodec` today. Both halves come off the `ClassTable` —
      `crates/nvs-runtime/src/object.rs:1335` is `define_shape_codec(fields, classes)`, which takes one
      `nvs_types::CodecField` per field in **sorted field-name order** and one `*const ClassDesc` per
      field (null where the field names no class), and the shape's own `ClassDesc` is an ordinary
      definition whose name starts `$shape{` (`crates/nvs-runtime/src/object.rs:777`) with one slot per
      field and **no constructor** — `crates/nvs-stdlib/src/json.rs:1563`'s `build_shape` writes slots
      directly, which is why. `crates/nvs-stdlib/src/request.rs:5542` is the nearest existing fixture, and
      it builds a *class*, so it is a model for the `Ctx` half only.
- [ ] **`Core\Arr::shapeAs<T>` — the row, the card, the `address()` arm, the roster entry and the cases**
      — `crates/nvs-stdlib/src/arr.rs:92` is the class, `crates/nvs-stdlib/src/arr.rs:2161` the
      `address()` arm, and `crates/nvs-stdlib/src/registry.rs:2389` is `WRITTEN_CLASS_MEMBERS`, which is
      the whole of what opens the type-argument door. Row: `names: &["a"]`,
      `params: &[CoreTy::Array(&CoreTy::Mixed)]`, `return_ty: CoreTy::Written("T")`, so the helper is
      `args: [4]` — three more than the row's arity. The body reads slots 0/1/2 exactly as
      `crates/nvs-stdlib/src/request.rs:2384` does, calls `crate::json::check_codec` first, then
      `crate::json::hydrate(ctx, class, shape, document, list, "Core\\Arr::shapeAs")`
      (`crates/nvs-stdlib/src/json.rs:1239`) — and **`args[3]` must be retained before it is passed**,
      because `hydrate` releases the document it is handed and the subject is the caller's array, not one
      this member built. loop-goal.md § *Stage 3* items 2 to 4 are the specification, the seven test
      names in `docs/agent/loop-goal.toml:5746` are the question set, and
      `tests/conformance/core/json-decodes-into-an-inline-shape.nvst` is the case to vary rather than
      repeat.
- [ ] **`rule:types/arrays`'s roster gains its fourth member** once the row lands —
      `docs/rules/types/arrays.md:42` names three today. No new record: ADR 0159 § 2 decides that the
      roster is the compiler's table and a member joining it is a row, not a decision. Re-render with
      `python tools/rules.py --render`.

## Backlog

- Stage 4's two wrappers, `postAs<T>` and `queryAs<T>` — loop-goal.md § *Stage 4*.
- Stage 5's fixture `examples/input-shapes.nvs`, which is the failing acceptance check — loop-goal.md.
- Spec §§ 6 and 15's rosters still do not list `shapeAs` — loop-goal.md § *Standing decisions*.
- `docs/reference/core/Arr.md`, if that page exists, owes `shapeAs` a paragraph — playbook, *Tooling*.
