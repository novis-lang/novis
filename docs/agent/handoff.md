# Handoff

## State

**Stage 0 items 1, 2 and 9 are done, and the list gained a new item 3.** ADR 0094 was decided with the user
after the previous session ended: every member declaration writes a visibility, `E0122`, no implicit
`public`. It sits first among the undone because it rewrites the corpus, so every item below it shifted by
one — what was item 3 is now item 4, and so on. The ADR is on disk, the code is claimed in
`mwl-diagnostics`, and **nothing of it is built**.

From the two items before it: `===`/`!==` no longer exist (`E0232` at the lexer); two statically disjoint
operands are **`E0466`** from `mwl_types::expr::operators::reject_disjoint_equality`, which `==`/`!=`, a
`switch` label and a `match` arm all reach — ADR 0090 § 6 makes the last two the same check against the
subject; and `+`/`+=` with an array operand is **`E0467`** naming `Core\Arr::underlay` (ADR 0069 § 2),
recovering to the array type so `$a += $b` reports once rather than twice.

The disjointness predicate is **one-sided on purpose** — it refuses only where disjointness is provable
from the two types alone, so `mixed`, `iterable`, an intersection, a union with any overlapping member and
any class name this compilation did not declare all pass. `types_are_disjoint`'s own doc comment owns why;
do not "tighten" it without reading that first.

`python tools/verify.py` is green (1321 tests) and `mwl test tests/` is 421 passed / 0 failed.

**One gap the equality work uncovered:** ADR 0090 § 2 makes `int`/`uint`/`float`/`decimal` **one domain**,
so the checker now accepts `$n == $f` — but `mwl-codegen` refuses it at
[emit.rs:861](../../crates/mwl-codegen/src/emit.rs#L861) (*"a binary operator over mismatched
representations"*). Those rows are pinned in `crates/mwl-types/tests/equality.rs` and deliberately left out
of the conformance case, which says so in a comment. It travels with item 4, in the backlog below.

**`orient.py` did not print three things the previous session needed.** `[context] modules` in
`loop-goal.toml` is missing `mwl-types/src/ty.rs` (the `Ty` enum — every rule over types starts there) and
the whole of `mwl-hir` (`symbol.rs`'s `SymbolKind`, `hierarchy.rs`'s `implements_interface`); `[context]
shapes` has no entry for a `mwl-types` integration test, whose harness is
`crates/mwl-types/tests/common/mod.rs`.

## Next group — ADR 0094, visibility at every member declaration (Stage 0 item 3)

**Shared file set:** `crates/mwl-syntax/src/casing.rs`, `crates/mwl-syntax/src/parser/decl.rs`,
`crates/mwl-diagnostics/src/lib.rs`, then the corpus. One ADR, one post-parse walk, one mechanical rewrite —
[1] is the check, [2] is every file that has to change because of it, and they land in the same slice so no
fixture is left written against the old rule.

- [ ] **Item 3 — the check** (M1). `E0122` on a property, class constant or method carrying no
      `public`/`protected`/`private`, in a `class`, `interface` or anonymous-class body. The declaration
      path is [decl.rs:442](../../crates/mwl-syntax/src/parser/decl.rs#L442)
      `parse_class_member_with_attrs` — one function behind all three bodies — and the modifiers it takes
      permissively come from [decl.rs:78](../../crates/mwl-syntax/src/parser/decl.rs#L78)
      `parse_modifiers`, whose doc comment says the legality check is "a later check": this is it. The walk
      to extend is [casing.rs:66](../../crates/mwl-syntax/src/casing.rs#L66) `check_casing`, already called
      from [main.rs:191](../../crates/mwl-cli/src/main.rs#L191), main.rs:234 and
      [requires.rs:187](../../crates/mwl-hir/src/requires.rs#L187) — reuse it and there is no new wiring.
      `E_MISSING_VISIBILITY` is already reserved at
      [lib.rs:169](../../crates/mwl-diagnostics/src/lib.rs#L169) and marked not-yet-emitted; delete that
      sentence when it fires. Watch three edges: a plain constructor parameter must **not** report (ADR 0094
      § 2 — visibility is the promotion marker), a bare `private(set)` must (§ 3), and an `enum` body must
      keep reporting only `E0220` rather than both. Tests: `a_member_without_visibility_is_a_compile_error`,
      `a_bare_set_visibility_is_a_compile_error`, `a_class_body_var_names_the_missing_visibility`,
      `a_plain_constructor_parameter_needs_no_visibility`, plus
      `tests/conformance/reject/a-member-must-declare-its-visibility.mwlt`.
- [ ] **Item 3b — the corpus rewrite, same slice.** Four member declarations across `.mwlt`/`.mwl` and
      roughly sixty inline snippets in Rust tests omit a visibility today. Find them with
      `grep -rEn "^\s+(final |abstract |static )*function [a-zA-Z_]" --include=*.mwlt --include=*.mwl .`
      and the `{ function`/`; function` shapes under `crates/`. `public` is the answer everywhere — these
      were written against PHP's default, and the rewrite must not change what any of them tests.

## Backlog

- **ADR 0090 § 3 at runtime** — a helper each for strings, arrays and objects, plus § 5's `mixed` pairing
  answering `false`; `mwl_runtime::identity::value_identical` at
  [identity.rs:108](../../crates/mwl-runtime/src/identity.rs#L108) exists and is *not yet wired to `==`*;
  wiring point [expr.rs:162](../../crates/mwl-ir/src/lower/expr.rs#L162) and `lower_binary` at expr.rs:1946.
  Travelling with it: `emit_binop` at [emit.rs:850](../../crates/mwl-codegen/src/emit.rs#L850) needs one
  side widened before cross-representation numeric equality lowers, and the rows dropped from
  `tests/conformance/lang/equality-across-overlapping-types-still-compiles.mwlt` go back when it does
  (loop-goal.md § *Stage 0* item 4).
- **ADR 0047 § 4** — the literal and enum-case type atoms are checked, not refused by name (same list,
  item 5; test `a_literal_type_atom_is_checked`).
- **`private`/`protected` are enforced** — the *access* half of visibility, keyed on the accessing class
  (same list, item 6). Item 3 above is the declaration half; neither waits on the other.
- **`Comparable`/`Stringable` carry their member signatures** — `$s->toString()` is `E0405` today, and
  `instanceof Stringable` panics `mwl-ir` (same list, item 7).
- **ADR 0061** — `autoload` grammar plus the name-to-file fixpoint over `mwl_hir::requires` (item 8).
- **`examples/collect.mwl`** is the next Stage 3 fixture, needing spec §§ 7-9 and 11-12 at once
  (`docs/implementation-plan.md` § *Open now*); **ADR 0069's combination members**
  (`overlay`/`underlay`/`appendAll`/`overlayDeep`) are M4S work in the same stage — the *refusal* landed,
  the members did not.
