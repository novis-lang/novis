# Handoff

## State

**A `new` target's type-argument list now binds.** `mwl_stdlib::registry::GENERIC_CLASSES`
(`registry.rs:703`) is the roster of `Core`-owned generic classes — `ObjectMap<K, V>`, `ObjectSet<T>`,
`Heap<T>` — and `check_new_type_args` (`calls.rs:293`) reads it positionally: the declared arity or
`E0442`, and `E0441` for every target off the roster. An accepted list is interned onto the target
(`Core\ObjectMap<Tag, int>`, not a bare class), so the receiver-driven substitution in
`expr/args.rs` has something to zip against once § 9's members exist. An entry need not be registered
in `CLASSES` yet; that is deliberate and its doc comment says why.

**`python tools/loop.py --goal-only` still stops at Stage 3's `collect.mwl`, but one section further
along.** Its first report is now `Core\Encoding::toHex`/`Core\Hash::of` at `collect.mwl:25` — spec § 7
— because lines 13 and 19 resolve. Conformance is 370 of 600, differential 86 of 150,
`every_part_one_spec_member_is_registered` still does not exist.

**Two things stand between § 9 and a running `collect.mwl`, and both are decisions the next session
makes.** (1) `infer_new` reports "no constructor" for every registered `Core` class
(`calls.rs:250`, `core_lib::is_registered`) — the § 9 collections are the one carve-out, since the spec
writes `new Core\ObjectSet<Tag>()`. (2) `mwl-ir` lowers `ExprInfo::New` to a program descriptor
(`lower/expr.rs:2311`), and a `Core` instance's `ClassDesc` comes from `mwl-stdlib` instead
(`instance::build`, `uuid.rs:183`). The cheap answer is to lower `new` on a roster class to that
class's own native constructor helper, so nothing below `mwl-ir` learns that `Core` owns a class —
which is exactly the promise `crates/mwl-stdlib/src/instance.rs`'s module doc already makes. Today an
unregistered `Core\ObjectSet` type-checks and would reach codegen with no descriptor at all.

`Tag::Bytes` still blocks `Random::bytes`, `Core\Encoding`, `Core\Bytes` and `Hash::*`: `mwl_ir::Ty::Bytes`
and `registry::CoreTy::Bytes` exist, but `mwl_runtime::Tag` has no `Bytes` row, so `value.rs:49` spends
`Tag::Str` on both. It is pre-authorized (`loop-goal.md` § *Standing decisions*).

## Next group — spec § 9's collections, now that the binding is there

**Shared file set:** a new `crates/mwl-stdlib/src/objset.rs` (`uuid.rs:147`'s `instance:`/`slots:` block
is the whole shape to copy) and its `mod` line in `crates/mwl-stdlib/src/lib.rs`,
`crates/mwl-stdlib/src/registry.rs` (`CLASSES`, `GENERIC_CLASSES` at `:703`),
`crates/mwl-stdlib/src/instance.rs` (`build`/`receiver`/`slot`),
`crates/mwl-types/src/expr/calls.rs:250` (the `new` carve-out),
`crates/mwl-ir/src/lower/expr.rs:2311` (the `New` arm), `crates/mwl-runtime/src/identity.rs` (the
identity hash the keying needs), and `tests/conformance/core/`. Spec rows:
`docs/spec/01-core-library.md:658-673`.

- [ ] **`new Core\ObjectSet<T>()` constructs and lowers**, with `add`, `has`, `count`, `isEmpty` and
      `clear` — the carve-out at `calls.rs:250`, the lowering decision above, the `slots:` layout
      (one slot holding the identity-keyed store), and one `.mwlt` case per row. Record the lowering
      choice in `instance.rs`'s module doc, per the standing decision on design calls.
- [ ] **`ObjectSet`'s set algebra** — `remove`, `union`, `intersect`, `diff` (`01-core-library.md:660`),
      same files, plus its own `.mwlt` case. `diff` is spelled as on `Core\Arr`, deliberately.
- [ ] **`Core\ObjectMap<K, V>`'s ten rows** (`01-core-library.md:659`) over the same store, with
      `get` returning `?V` — the spec paragraph under the table says why there is no throwing read.
- [ ] **The positive half of the binding's conformance coverage**, under `tests/conformance/lang/`:
      the refusals are pinned by `a-core-collection-takes-the-type-arguments-it-declares.mwlt`, and a
      case that *constructs* each collection cannot run until the slice above lands.

## Backlog

- `Core\Heap<T>` — on `GENERIC_CLASSES` already; needs ADR 0013's `Comparable` or a constructor
  comparator (`docs/spec/01-core-library.md` § 9).
- `mwl_runtime::Tag::Bytes` and a `bytes` producer — blocks spec § 7 whole (`mwl-runtime`'s module doc).
- `Core\Encoding`/`Core\Hash`/`Csv` each need a dependency picked under ADR 0051 § 4.
- `every_part_one_spec_member_is_registered` — the loop's own definition of done; does not exist
  (`docs/agent/loop-goal.md` § *Stage 4*).
- `Uri::parse` and its instance still owe the RFC 3986 dependency pick (`crates/mwl-stdlib/src/uri.rs`).
- `docs/spec/02-php-migration.md` is 31% classified (`python tools/check-migration.py`).
