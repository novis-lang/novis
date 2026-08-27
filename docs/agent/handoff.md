# Handoff

## State

**M4 — language completeness.** One slice landed, and it closes the live crash the previous
group found rather than a worklist item: **reading an absent array key throws.**

`mwl_ir::InstKind::ArrayGet` is a *fallible* instruction now. It carries ADR 0002's error
edge, `mwl-codegen` emits it through `emit_helper` against one runtime entry point
(`mwl_array_required_get`, in `crates/mwl-runtime/src/helpers.rs`), and the `Ty::Str`
versus `Ty::Int` key split that used to pick between two borrowing primitives is decided
there by the key's own tag — so `Signatures::array_get`/`array_get_index` and their two
`RuntimeSig` rows are gone. The divergence is ADR 0007 § 7 **row 11**; that helper's doc
comment owns why the old answer was a null dereference below the language rather than an
error inside it, and `InstKind::ArrayGet`'s owns the instruction's half.

An absent key is told from a stored `null` by an `Option` out of the table
(`mwl_runtime::array::entry`/`entry_at_index`, both new), so an `array<?T>` reads its own
`null` back. The write side is unchanged and deliberately answers the opposite —
`Helper::ArrayRowForWrite` vivifies where a read throws.

`verify.py` 6 of 6 green — conformance **577**, differential 162. `tools/leak-check.sh` is
green over a fixture that throws with a rendered `uint` key and a temporary base live, which
is the one new refcount edge: the key is staged on the owned-temporaries stack now rather
than released inline. `holes.py` is unchanged at **35 sites, 9 items** — this was never a
panic site, which is why no item named it.

## Next group

**All three share `crates/mwl-types/src/locals.rs`'s `narrow` (line 303) and
`crates/mwl-ir/src/lower/expr.rs`'s `lower_index` (line 3661) / `lower_coalesce`
(line 1271).** The third is new and is the one with a user-visible wrong answer.

- [ ] **`$a["k"] ?? "d"` throws where PHP yields the default.** `lower_coalesce`
      (`crates/mwl-ir/src/lower/expr.rs:1271`) short-circuits to the left operand whenever
      its representation is not `Ty::Tagged`, and an `array<string>` element is a `Ty::Str`
      — so the `??` never runs, and the read under it now throws instead of crashing.
      PHP's `??` is exactly *"absent or null, without the warning"*, so priority 2 says the
      guarded read must not throw. The shape: `mwl_types` records the index expression under
      a `??` (and under `isset`, once that lowers) as coalesce-guarded, and `lower_index`
      emits the non-throwing read for one — `mwl_array_get`'s borrowing answer is still
      there and is what it wants. ADR 0007 § 7 row 11 is the rule the exception is carved
      out of, so say so in its cell.
- [ ] **`?array<T>` does not narrow out of `null`** — `crates/mwl-types/src/locals.rs:303`,
      whose residue check accepts only `Ty::Class(..)`. **Relaxing that check is not the
      slice**, and this session's change is why the temptation is now real: the helper ABI
      is tag-dispatched, so a narrowed `?array<T>` would read *through a subscript* with no
      untag at all — and then meet `InstKind::ArrayNextSlot` (`foreach`), `ArraySet` (an
      element write) and every other consumer that takes a raw `Ty::Array` operand, each of
      which gets a `Ty::Tagged` one and fails in codegen. The slice is to record the
      narrowing on the *variable read* the way a receiver's already is (`untag_receiver`,
      `crates/mwl-ir/src/lower/expr.rs`) and untag in `lower_expr`, which is what that
      function's doc comment at `locals.rs:290-302` has been waiting for.
- [ ] **Re-word E0482's nullable-array help and retire the playbook's `?array<T>` trap** —
      both are only true while the slice above is open. `E0482` is declared in
      `crates/mwl-diagnostics/src/lib.rs` and reported from
      `crates/mwl-types/src/expr/mod.rs`.

## Backlog

- `isset(...)`/`empty(...)` parse (`crates/mwl-syntax/src/ast.rs:845`) and lower nowhere —
  the same guarded-read question as `??` above (`mwl_types::expr`'s module doc).
- `mwl-ir` gap 1's tagged arithmetic is what gates narrowing a `?int` at all
  (`crates/mwl-types/src/locals.rs:299`).
- The two unattributed `holes.py` sites are `lower_decl_type`/`lower_checked_ty`'s
  catch-alls — `decimal`, `never`, `iterable`, `self`/`static`/`parent`, a shape type and an
  intersection as a *declared* type (docs/implementation-plan.md § Open now).
- `docs/spec/02-php-migration.md` is 31% classified (`python tools/check-migration.py`).
- 20 of the 32 named `.mwlt` cases are still to write (`python tools/holes.py --cases`).

**Orientation gap:** `[context] modules` in `docs/agent/loop-goal.toml` names no
`mwl-runtime` pattern at all, and no `mwl-codegen/src/lib.rs` — this slice lived in
`crates/mwl-runtime/src/{array,helpers,abi}.rs` and in `Signatures`, all of them read from
scratch. Any slice that changes what an instruction *costs at runtime* lands there.
