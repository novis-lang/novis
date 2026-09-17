# Handoff

## State

**Goal `decided-closures`, stage 4 — the library.** `crates/nvs-stdlib/src/reflect.rs`'s gap 1 is
half built: `Core\Reflect\ConstantInfo` is registered and its line is gone from `NOT_YET_BUILT`, so
the gap's prose now names `AttributeInfo` alone. `python tools/owners.py --closes decided-closures`
still names 16, because the item is the whole of gap 1 and the second half is the next group.

**A class constant travels as a folded value, not as a spelling.** `nvs_types::layout::ClassConstant`
(name, `public`, `protected`, `secret`, value) rides `nvs_ir::ir::Class::constants` and
`nvs-codegen`'s `constant_desc` to `nvs_runtime::ConstantDesc` on the descriptor itself — a roster
beside the field ones, keyed by name and aligned to no slot order, because a constant belongs to the
class and claims no storage. The fold is `nvs_types::consts::fold_const`'s, reused rather than
rewritten, so the four literals `rule:types/constant-in-type-position` folds are exactly the four a
reflective read hands back and an `array` constant is `ConstantValue::Opaque` in both places.

**The roster is metadata and the value is acting**, which is `rule:core-classes/reflect` § 2 applied
one member over. `ClassInfo::constants()` names every constant including the `private` ones and
carries `name`/`isPublic`/`hasValue`; `ClassInfo::constant(name)` is the acting door and faces
`Ctx::constant_is_visible_from`, the third method of `crates/nvs-runtime/src/ctx/visibility.rs`. It
refuses a `secret`-typed constant at **every** site including the declaring class's own, because it
answers `mixed`, which carries no qualifier — `rule:security/secret-sinks-refuse` would never fire
downstream, so the bit travels and the member fails closed.

**What is left in this file is gap 1b**, and it needs descriptor data of a different shape: an
attribute is an attach site with a payload, not a member of a class body.

## Next group

**Stage 4: `crates/nvs-stdlib/src/reflect.rs`'s last gap** — one file set, the same channel this
session widened: `crates/nvs-types/src/retrieval.rs`, `crates/nvs-types/src/layout.rs`,
`crates/nvs-ir/src/ir.rs`, `crates/nvs-codegen/src/lib.rs`, `crates/nvs-runtime/src/object.rs`,
`crates/nvs-stdlib/src/reflect.rs` and the cases under `tests/conformance/core/`.

- [ ] **`crates/nvs-stdlib/src/reflect.rs:156` — gap 1b: `Core\Reflect\AttributeInfo`.** The join is
      this session's exactly, one station earlier: the source is
      `crates/nvs-types/src/retrieval.rs`'s `AttributeTable` rather than a class-body walk, because
      `rule:attributes/structural-retrieval` indexes an attach site and not a member. Add the roster
      beside `crates/nvs-types/src/layout.rs:95`'s neighbours in `ClassLayout`, beside
      `crates/nvs-ir/src/ir.rs:205` in `ir::Class`, a converter at
      `crates/nvs-codegen/src/lib.rs:1552` next to `constant_desc`, and a list on
      `crates/nvs-runtime/src/object.rs:476`'s descriptor beside `constants`. **Decide what a
      payload is carried as and state what it spends** (`rule:programs/memory-priority`) — the
      payload is `rule:attributes/payload-is-a-compile-time-constant`'s constant set, so
      `nvs_runtime::ConstantValue` may already be the currency. Then the five edits of
      `docs/agent/conventions.md` § *A `Core` member* and three `.nvst` cases.
- [ ] **`crates/nvs-stdlib/src/lib.rs:116` — gap 1: §§ 13–20 of the registry.** Takes the same file
      set's tail (`reflect.rs`, `registry.rs`) and is the gap that says what the roster still owes;
      read it after the one above, since `AttributeInfo` is one of the rows it counts.

## Backlog

- `crates/nvs-types/src/defaults.rs:58` gap 1 — a named constant as a parameter default; the fold
  this session made reusable (`consts::fold_const`) is what it was waiting on. Owner:
  `decided-closures`.
- `crates/nvs-stdlib/src/test.rs:94` and `:102` — two gaps, one file, one session. Owner:
  `decided-closures`.
- `crates/nvs-stdlib/src/db/mod.rs:231` and `:257` — same shape, same file. Owner:
  `decided-closures`.
- `crates/nvs-syntax/src/lib.rs:96` gap 1 — a bare inline shape type on a local declaration; the
  only front-end gap left on the register.
