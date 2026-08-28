# Handoff

## State

**M4's frontier is Stage 5, and item 32 is one spelling from done.** ADR 0046 § 1's
attach grammar, § 2's payload rule, § 1's shape check, ADR 0033 § 4's fifth sink and
now §§ 4-5's structural retrieval have all landed. The plan's *Open now* paragraph is
the one home for why each landed piece is shaped the way it is, and
`crates/nvs-types/src/retrieval.rs`'s module doc is the retrieval's own.

- **The retrieval is a fold, not a call.** `nvs_types::retrieval::fold_retrieval`
  (`crates/nvs-types/src/retrieval.rs:196`) records the answer as an
  `ExprInfo::CoreConst` against the call's own span and `infer_static_call`
  (`crates/nvs-types/src/expr/calls.rs:196`) then records **no** `ExprInfo::Call` — one
  span carries one entry, and `nvs-ir` would otherwise lower the call it was told to
  replace. `nvs_stdlib::attributes` registers the two rows and no implementation.
- **`Foo::constructor(...)` needs a written constructor**, which ADR 0046 § 4 says it
  should not: `nvs_hir::members::check_member_ref` (`crates/nvs-hir/src/members.rs:936`)
  reports `E0309` for a class that declares none. See the plan's *Open now* paragraph
  for why the naive widening opens a `nvs-ir` panic, and the next group for the shape
  the fix has to take.
- **ADR 0046 still has no *Verification* section**, and M4's acceptance paragraph names
  one for it alongside 0014, 0023, 0028 and 0069. Every member it would list now
  exists, so this is the next session's to write rather than a section that would be
  written twice.

## Next group

**Item 32's last spelling, then ADR 0046's own *Verification*.** The file set is
`crates/nvs-hir/src/members.rs`, `crates/nvs-types/src/expr/calls.rs` and
`docs/adr/0046-attributes-shape-literal-metadata.md`.

- [ ] **A first-class-callable reference to a synthesized `constructor` resolves** —
      ADR 0046 § 4 plus ADR 0022 § 2 ("every class has one, definitely"). Exempt the
      name `constructor` in `member_declared`'s caller
      (`crates/nvs-hir/src/members.rs:936`) **only** where the reference is
      `CallArgs::FirstClassCallable`, and refuse a written `Foo::constructor()` call on
      a class that declares none in `nvs_types` beside it — unrefused it reaches
      `nvs-ir` with no resolved target and panics, which is the whole reason the
      exemption is not one line. Then drop the `public function constructor() {}` from
      both cases below and the run case still passes.
- [ ] **ADR 0046's *Verification* section**, written once the above lands — M4's
      acceptance paragraph (`docs/plan/m4.md`) names one fixture per rule for this ADR,
      and the two cases that exist are
      `tests/conformance/core/an-attribute-is-retrieved-by-the-shape-it-satisfies.nvst`
      and
      `tests/conformance/reject/an-attribute-retrieval-is-refused-where-it-cannot-be-folded.nvst`,
      beside § 2's and § 1's three under `tests/conformance/reject/`.

## Backlog

- A payload holding a user-declared class constant or an enum case is `E0731` at the
  *retrieval* — closing it wants `nvs_types::signatures`' unmodeled-constant gap, that
  crate's own known gaps.
- A `require` whose path is not a string literal runs nothing at all, silently, in both
  forms — `nvs_hir::requires`' own known gap.
- ADR 0024 § 4's sink list and ADR 0033's `Core\Log` inspection wait on `Core` classes
  that arrive at M7/M8 — `docs/agent/loop-goal.md` § *Standing decisions*.
- `nvs-ir` gap 1: a case cannot name one callback and hand it to several members —
  `crates/nvs-ir/src/lib.rs`'s own gap list.
