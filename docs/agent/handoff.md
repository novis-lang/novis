# Handoff

## State

**Stage 0 items 1, 2, 3 and 9 are done.** ADR 0094 is built: a property, class constant or method with no
`public`/`protected`/`private` is `E0122`, in a `class`, `interface` or anonymous-class body. The walk that
reports it is `mwl_syntax::check_declarations` — **renamed from `check_casing`**, because it now answers
two questions off one visit; `casing.rs`'s module doc owns why they share a walk, and the file keeps its
name so ADRs 0029/0043/0049 still point at it. A bare `private(set)` names the pair it is missing (§ 3), a
class-body `var` is redirected in the parser (§ 4, `parse_class_body_var`), a plain constructor parameter
is exempt (§ 2), and an `enum` body still reports only `E0220` because `check_enum_decl` never walks its
members.

**The corpus rewrite was eleven fixtures, not sixty**, all in `casing.rs`'s own tests plus two `.mwlt`
methods — only `mwl-cli` and `mwl_hir::requires` call that walk, so a `<?mwl` snippet in a `mwl-types` or
parser test never reaches it. That is now a playbook bullet; do not budget a large rewrite for the next
rule added there without checking its callers.

`python tools/verify.py` is green (1327 tests) and `mwl test tests/` is 422 passed / 0 failed.

**Still open from earlier items:** ADR 0090 § 2 makes `int`/`uint`/`float`/`decimal` one domain, so the
checker accepts `$n == $f` — but `mwl-codegen` refuses it at
[emit.rs:861](../../crates/mwl-codegen/src/emit.rs#L861) (*"a binary operator over mismatched
representations"*). Those rows are pinned in `crates/mwl-types/tests/equality.rs` and deliberately left out
of the conformance case, which says so in a comment. It travels with the group below.

**`orient.py` did not print three things.** `[context] modules` in `loop-goal.toml` is missing
`mwl-types/src/ty.rs` (the `Ty` enum) and the whole of `mwl-hir`; `[context] shapes` has no entry for a
`mwl-types` integration test (harness: `crates/mwl-types/tests/common/mod.rs`). This session also needed
`crates/mwl-syntax/src/ast.rs` (`Modifier`, `ClassMemberKind`, the three member structs) and
`crates/mwl-syntax/src/parser/ty.rs` (`token_starts_type`), neither of which `[context] modules` names.

## Next group — ADR 0090 § 3, equality at runtime (Stage 0 item 4)

**Shared file set:** `crates/mwl-runtime/src/identity.rs`, `crates/mwl-ir/src/lower/expr.rs`,
`crates/mwl-codegen/src/emit.rs`, then the two test files. The helper exists and is unwired; [1] wires it,
[2] widens the one representation mismatch that blocks the numeric rows, [3] puts the pinned rows back.

- [ ] **Item 4a — wire `==`/`!=` to the three non-scalar rows** (ADR 0090 § 3). `value_identical` at
      [identity.rs:108](../../crates/mwl-runtime/src/identity.rs#L108) is written and reaches nothing.
      Lowering point is `lower_binary` at [expr.rs:1946](../../crates/mwl-ir/src/lower/expr.rs#L1946),
      dispatch at [expr.rs:162](../../crates/mwl-ir/src/lower/expr.rs#L162). Strings compare as text and
      never as numbers, arrays ordered and element-wise and recursive, objects by identity; § 5's `mixed`
      pairing answers `false` and never throws. Tests named by `loop-goal.toml`'s stage 0 block:
      `equal_strings_compare_as_text_and_never_as_numbers`, `equal_arrays_compare_ordered_and_element_wise`,
      `equal_objects_compare_by_identity`, all in `crates/mwl-runtime`.
- [ ] **Item 4b — one side widened before a cross-representation numeric compare.** `emit_binop` at
      [emit.rs:850](../../crates/mwl-codegen/src/emit.rs#L850) refuses two representations outright;
      ADR 0090 § 2's table makes `int`/`uint`/`float`/`decimal` one domain, so the widen belongs here
      rather than in a checker that already accepts the pairing.
- [ ] **Item 4c — restore the dropped conformance rows.** The numeric rows removed from
      `tests/conformance/lang/equality-across-overlapping-types-still-compiles.mwlt` go back once 4b lands;
      the comment in that case says why they left, and `crates/mwl-types/tests/equality.rs` keeps pinning
      the static half either way.

## Backlog

- **ADR 0047 § 4** — the literal and enum-case type atoms are checked, not refused by name (Stage 0
  item 5; test `a_literal_type_atom_is_checked`).
- **`private`/`protected` are enforced** — the *access* half of visibility, keyed on the accessing class
  (item 6). The declaration half landed above; neither waits on the other.
- **`Comparable`/`Stringable` carry their member signatures** — `$s->toString()` is `E0405` today, and
  `instanceof Stringable` panics `mwl-ir` (item 7).
- **ADR 0061** — `autoload` grammar plus the name-to-file fixpoint over `mwl_hir::requires` (item 8).
- **`E_BAD_MODIFIER` (E0106) is still defined and never emitted** — ADR 0094 answered the *missing*
  modifier, not the nonsensical one (`abstract` on a property, two visibilities at once); `docs/adr/0039`
  § 1 owns order, nothing owns combination.
- **`examples/collect.mwl`** is the next Stage 3 fixture, needing spec §§ 7-9 and 11-12 at once
  (`docs/implementation-plan.md` § *Open now*); **ADR 0069's combination members** are M4S work in the
  same stage — the *refusal* landed, the members did not.
