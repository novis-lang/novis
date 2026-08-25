# Handoff

## State

**`examples/collect.mwl` is past its parser hole.** `new Core\ObjectSet<Tag>()` parses: `ExprKind::New`
carries a `type_args: Vec<Type>` read by the same checkpointed trial parse a call site's own `<...>` goes
through (`parse_call_type_args`, reused verbatim), and the checker refuses the list on every target,
because no target has a type parameter to bind yet. ADR 0007 § 3 now names both expression positions.

**`python tools/loop.py --goal-only` passes Stages 0, 1, 2 and six of Stage 3's seven fixtures.** Running
`mwl run examples/collect.mwl` by hand now reports the work behind the hole, in source order: `E0441` at
`collect.mwl:13` and `:19` (nothing generic to bind), then `Core\Encoding::toHex`/`Core\Hash::of` at `:25`.
So the whole of § 9 plus a `bytes` producer is what stands between the loop and Stage 4; the counts behind
*that* are conformance **370 of 600** and differential **86 of 150** (differential has not moved this run),
and `every_part_one_spec_member_is_registered` still does not exist.

`Tag::Bytes` still blocks `Random::bytes`, `Core\Encoding`, `Core\Bytes` and `Hash::*`, and it is a design
call the loop is pre-authorized to settle (`loop-goal.md` § *Standing decisions*): `mwl_ir::Ty::Bytes` and
`registry::CoreTy::Bytes` both exist, and what is missing is a runtime producer — `mwl_runtime::Tag` has no
`Bytes` row, so `value.rs:49` spends `Tag::Str` on both. Spec § 12's `Core\Uri` is its percent-encoding
half only; `uri.rs`' own docs own why `isValid` waits on the RFC 3986 dependency pick.

## Next group — spec § 9's collections, and the type arguments that bind to them

The previous group's slices 2-4 have nothing to bind against until § 9 exists, so they are folded in here.

**Shared file set:** `crates/mwl-stdlib/src/registry.rs` (`CoreTy::Instance` at `registry.rs:234`,
`CoreClass::instance`/`slots` at `registry.rs:228`, `WRITTEN_CLASS_MEMBERS` at `registry.rs:670`),
`crates/mwl-stdlib/src/uuid.rs` (`instance:` at `uuid.rs:147` is the whole shape to copy, and its module
doc at `uuid.rs:34-89` says what a `Core`-owned instance may hold), a new `crates/mwl-stdlib/src/objset.rs`
alongside it, `crates/mwl-types/src/core_lib.rs` (`is_registered` at `core_lib.rs:146`, `CLASSES` walked at
`core_lib.rs:41`), `crates/mwl-types/src/expr/calls.rs` (`check_new_type_args` at `calls.rs:277` refuses
every list today; `check_new_target` at `calls.rs:413`; `infer_new` at `calls.rs:208`), and
`tests/conformance/core/`. Spec rows: `docs/spec/01-core-library.md:650-673`.

- [ ] **A `Core`-owned generic class is a roster with an arity**, in `registry.rs` beside
      `WRITTEN_CLASS_MEMBERS` — the same table shape, class name to its parameter names. `check_new_type_args`
      (`calls.rs:277`) consults it instead of refusing unconditionally, reporting `E0442` (`E_TYPE_ARG_COUNT`)
      on a wrong count and keeping `E0441` for everything off the roster. `ObjectMap<K, V>` against
      `ObjectSet<T>` is the case that proves the list is positional.
- [ ] **`Core\ObjectSet<T>`'s nine rows** (`01-core-library.md:660`) over the `CoreTy::Instance` shape
      `uuid.rs:147` models, keyed on the strict identity `mwl_runtime::identity` already defines.
      Registering the rows and writing their `.mwlt` cases is **one slice** — `mwl-stdlib`'s
      `tests/conformance_coverage.rs` fails the moment a row has no case calling it.
- [ ] **`Core\ObjectMap<K, V>`'s ten rows** (`01-core-library.md:659`), same file set, same slice rule.
      `get` returns `?V` (`01-core-library.md:663`), which is the row that needs `?T` and has it.
- [ ] **Conformance cases for the binding itself**, under `tests/conformance/lang/`: one constructing each
      arity, and the pair already on disk for the refusals
      (`tests/conformance/reject/a-new-target-with-no-type-parameter-refuses-a-written-list.mwlt`).

`lower_new` (`mwl-ir/src/lower/expr.rs:2303`) needs no change for any of this: it never sees `type_args`,
which is ADR 0047 § 5's erasure rule holding by construction rather than by code.

## Backlog

Ordered by what the acceptance test blocks on, not by section number:

- **`Core\Heap<T>`** — § 9's third collection, ordered by ADR 0013's `Comparable`
  (`01-core-library.md:661-668`). Same file set as the group above; split off only for size.
- **A `bytes` producer** — the `Tag::Bytes` decision above, then § 7's seven rows and `Hash::*`.
  `collect.mwl:25-26` is on the acceptance path, not in this backlog for long.
- **`Uri::parse`/`isValid`, `Csv`, `Validate`, `Out::capture`** — the rest of `collect.mwl`'s roster;
  `Uri`'s own gap 1 owns the dependency pick.
- **Stage 4's counts are their own work, not a side effect.** 230 conformance and 64 differential cases
  are owed, and this run produces ~2 and 0 per session. A case over an *already registered* member costs
  no new code and shares one file set, so these belong in dedicated groups of a dozen.
- **`every_part_one_spec_member_is_registered`** (`loop-goal.toml:339`) — reads the member rows out of
  `docs/spec/01-core-library.md` and checks each against the registry.
- **`do`/`while` is the one M4 control-flow statement that does not lower** (`mwl-ir`'s own module doc).
