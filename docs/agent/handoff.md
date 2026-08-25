# Handoff

## State

**Stage 0 items 1, 2 and 8 are done.** `===`/`!==` no longer exist (`E0232` at the lexer); two statically
disjoint operands are **`E0466`** from `mwl_types::expr::operators::reject_disjoint_equality`, which
`==`/`!=`, a `switch` label and a `match` arm all reach — ADR 0090 § 6 makes the last two the same check
against the subject; and `+`/`+=` with an array operand is **`E0467`** naming `Core\Arr::underlay`
(ADR 0069 § 2), recovering to the array type so `$a += $b` reports once rather than twice.

The disjointness predicate is **one-sided on purpose** — it refuses only where disjointness is provable
from the two types alone, so `mixed`, `iterable`, an intersection, a union with any overlapping member and
any class name this compilation did not declare all pass. `types_are_disjoint`'s own doc comment owns why;
do not "tighten" it without reading that first.

`python tools/verify.py` is green (1321 tests) and `mwl test tests/` is 421 passed / 0 failed.

**One gap this uncovered:** ADR 0090 § 2 makes `int`/`uint`/`float`/`decimal` **one domain**, so the checker
now accepts `$n == $f` — but `mwl-codegen` refuses it at
[emit.rs:861](../../crates/mwl-codegen/src/emit.rs#L861) (*"a binary operator over mismatched
representations"*). Those rows are pinned in `crates/mwl-types/tests/equality.rs` and deliberately left out
of the conformance case, which says so in a comment. That is item [2] below.

**`orient.py` did not print three things I needed.** `[context] modules` in `loop-goal.toml` is missing
`mwl-types/src/ty.rs` (the `Ty` enum — every rule over types starts there) and the whole of `mwl-hir`
(`symbol.rs`'s `SymbolKind`, `hierarchy.rs`'s `implements_interface`); `[context] shapes` has no entry for
a `mwl-types` integration test, whose harness is `crates/mwl-types/tests/common/mod.rs`.

## Next group — ADR 0090 § 3 at runtime (Stage 0 item 3, then its lowering)

**Shared file set:** `crates/mwl-runtime/src/identity.rs`, `crates/mwl-ir/src/lower/expr.rs` and
`crates/mwl-codegen/src/emit.rs`. One ADR section, one lowering path, the three `mwl-runtime` tests
`loop-goal.toml` names. In this order — [1] defines the helpers, [2] is the representation question their
call sites raise.

- [ ] **Item 3 — ADR 0090 § 3's three non-scalar rows** (M3/M4). A helper each for strings (text, never
      numeric), arrays (ordered, element-wise, recursive) and objects (identity), plus § 5's `mixed`
      pairing answering `false` and never throwing. `mwl_runtime::identity::value_identical` at
      [identity.rs:108](../../crates/mwl-runtime/src/identity.rs#L108) is the comparison already defined
      and *not yet wired to `==`* — that module doc says so in its first paragraph. The string row may
      already be done: [emit.rs:869](../../crates/mwl-codegen/src/emit.rs#L869) routes `Ty::Str`/`Ty::Bytes`
      equality through `mwl_str_eq` rather than an `icmp`. Wiring point is
      [expr.rs:162](../../crates/mwl-ir/src/lower/expr.rs#L162)'s `Eq | NotEq` dispatch and
      [expr.rs:1946](../../crates/mwl-ir/src/lower/expr.rs#L1946) `lower_binary`. Tests:
      `equal_strings_compare_as_text_and_never_as_numbers`,
      `equal_arrays_compare_ordered_and_element_wise`, `equal_objects_compare_by_identity`.
- [ ] **Item 3b — cross-representation numeric equality lowers.** `emit_binop` at
      [emit.rs:850](../../crates/mwl-codegen/src/emit.rs#L850) rejects two representations outright;
      ADR 0090 § 2's numeric row and § 3's "mathematically equal across the whole domain" both need one
      side widened first. When it lands, restore the dropped rows to
      `tests/conformance/lang/equality-across-overlapping-types-still-compiles.mwlt` and delete the comment
      that explains their absence.

## Backlog

- **ADR 0047 § 4** — the literal and enum-case type atoms are checked, not refused by name
  (`docs/agent/loop-goal.md` § *Stage 0* item 4; test `a_literal_type_atom_is_checked`).
- **`private`/`protected` are enforced** — nothing enforces them on a class member (same list, item 5).
- **`Comparable`/`Stringable` carry their member signatures** — `$s->toString()` is `E0405` today, and
  `instanceof Stringable` panics `mwl-ir` (same list, item 6).
- **ADR 0061** — `autoload` grammar plus the name-to-file fixpoint over `mwl_hir::requires` (item 7).
- **ADR 0069's combination members** — `overlay`/`underlay`/`appendAll`/`overlayDeep` in `Core\Arr`; the
  *refusal* landed, the members are M4S (that ADR's *Verification*).
- **`examples/collect.mwl`** is the next Stage 3 fixture, needing spec §§ 7-9 and 11-12 at once
  (`docs/implementation-plan.md` § *Open now*).
