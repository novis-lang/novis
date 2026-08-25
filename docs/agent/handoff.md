# Handoff

## State

**Spec § 9's two identity-keyed collections run end to end.** `Core\ObjectSet<T>` (nine rows) and
`Core\ObjectMap<K, V>` (nine rows) are `crates/mwl-stdlib/src/objset.rs` and `objmap.rs`, both over
one store module — `identity_store.rs`, whose module doc owns the layout decision (an `MwlArray`
keyed by `"<hex identity hash>#<ordinal>"`, chain kept dense so a lookup stops at the first gap).
`Core\Heap` and the `Iterable` all three rows declare are what § 9 still owes.

**`new` on a `Core` class now constructs and lowers.** The roster is
`mwl_stdlib::registry::CONSTRUCTORS` (`registry.rs:621`); `mwl_types::expr::calls` carves it out of
the "a `Core` class has no constructor" refusal (`calls.rs:258`), and `mwl-ir` lowers `new` on a name
it holds to an `InstKind::CoreCall` rather than an `InstKind::New` (`lower/expr.rs:2365`).
`crates/mwl-stdlib/src/instance.rs`'s second `# Decision` section owns why, and its `set_slot` is the
one thing that mutates a built instance.

**A `Core` generic class is now nameable in type position too**, not only as a `new` target —
`lower.rs`'s `generic_params` and `expr/args.rs`'s `substitute_receiver_args` each consult
`registry::class_type_params` beside `mwl_hir::interfaces`. That is what makes
`Core\ObjectMap<Tag, int> $w` a declarable type and `$w->keys()` an `array<Tag>`.

**`python tools/loop.py --goal-only` still stops at `collect.mwl`, now purely on spec § 7.** Lines
13–23 resolve and run; the first report is `Core\Encoding::toHex`/`Core\Hash::of` at
`collect.mwl:25`. Conformance is 372 of 600, differential 86 of 150,
`every_part_one_spec_member_is_registered` still does not exist.

`Tag::Bytes` still blocks `Random::bytes`, `Core\Encoding`, `Core\Bytes` and `Hash::*`:
`mwl_ir::Ty::Bytes` and `registry::CoreTy::Bytes` exist, but `mwl_runtime::Tag` has no `Bytes` row,
so `value.rs:49` spends `Tag::Str` on both. It is pre-authorized (`loop-goal.md` § *Standing
decisions*), and it is the next real decision on the gate's path.

## Next group — spec § 7, the section `collect.mwl` now stops on

**Shared file set:** a new `crates/mwl-stdlib/src/encoding.rs` (`uuid.rs:147`'s `instance:`/`slots:`
block is still the shape to copy, and `objset.rs:105` the shape of an `address` arm),
`crates/mwl-runtime/src/value.rs:49` and its `Tag` enum, `crates/mwl-stdlib/src/registry.rs`
(`CLASSES` at `:590`, `CoreTy::Bytes` at `:109`), `crates/mwl-stdlib/src/lib.rs`'s `mod` list and
`address_of` chain (`:260`), and `tests/conformance/core/`. Spec rows:
`docs/spec/01-core-library.md` § 7.

- [ ] **`mwl_runtime::Tag` gains a `Bytes` row**, so something can construct a fresh `bytes` value —
      `value.rs:49` currently spends `Tag::Str` on both. Decide the heap shape (a second `MwlStr`
      without the UTF-8 promise is the cheap answer) and record it in `mwl-runtime`'s module doc, per
      the standing decision on design calls. Nothing else in § 7 can land first.
- [ ] **`Core\Encoding`'s hex and base64 rows** — `toHex`/`fromHex`/`toBase64`/`fromBase64`, with the
      dependency picked under ADR 0051 § 4 and its reasoning in the module's own doc comment. One
      `.mwlt` case per row.
- [ ] **`Core\Encoding::encodeText`/`decodeText` and `Core\Charset`**, which `collect.mwl:26` writes.
- [ ] **`Core\Hash::of`/`hmac`/`equals` and `Core\Digest`** (§ 11's remaining half), which
      `collect.mwl:25` writes and which needs the same `bytes` row.

## Backlog

- `Core\Heap<T>` and § 9's `Iterable` — `docs/spec/01-core-library.md:660-662`; `Heap` also needs
  ADR 0013's `Comparable` reachable from a `Core` class.
- **Constructor property promotion does not create a property**: `public readonly string $name` in a
  `constructor` parameter compiles, and `$obj->name` is then `E0405`. Owner: `mwl-types`' own gaps.
- `every_part_one_spec_member_is_registered` — the loop's definition of done, `loop-goal.md` Stage 4.
- `do`/`while` is the one M4 control-flow statement that does not lower — `mwl-ir` gap 1.
- ADR 0088's registry-wide qualifier classification for member rows — `implementation-plan.md`.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
