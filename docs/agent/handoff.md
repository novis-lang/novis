# Handoff

## State

**ADR 0035's condition end is closed for a value that is not one.** A call
returning `void` tested for truth is `E0719` where it is written, from the one
site `nvs_types` now has for the question
(`nvs_types::expr::check_condition`, whose doc comment is the rule's home) plus
`!`'s and `empty()`'s own arms; `&&`, `||` and `??` keep `E0718`, being ADR
0007 § 4's operands rather than ADR 0035's test. The plan's `Open now` carries
the split and the two `.nvst` cases that pin it.

- **The five catch-all rosters now have their agreement case.**
  `tests/conformance/lang/an-operator-agrees-whether-its-operand-is-typed-or-tagged.nvst`
  asks 45 questions of a typed and a tagged spelling of one value and counts
  the agreements. It found one hole rather than a disagreement, below.
- **A `match` whose subject is `mixed` panics** at
  `crates/nvs-ir/src/lower/expr.rs:1521` — the label chain asserts each label
  lowered to the subject's own representation instead of widening it, and a
  tagged subject's `Eq` would need `Helper::Identical` rather than the
  `InstKind::BinOp` emitted there. It is the next group's first slice.
- **A `catch` binding still has no callable members** — `$e->getMessage()`
  panics `nvs-ir` at `lower/expr.rs:2281`. Unchanged.
- **`orient.py`'s pack was complete for this item.** The two standing manifest
  gaps are unchanged — `[context] modules` has no `nvs-runtime` and no
  `nvs-diagnostics` entry, and `crates/nvs-diagnostics/src/lib.rs` was again a
  file this session edited.

## Next group

**The tagged `match` subject and the two rosters below it, over the one
lowering file plus the backend one this session did not open.** The files:
`crates/nvs-ir/src/lower/expr.rs` and `crates/nvs-codegen/src/emit.rs`.

- [ ] **A `match` over a tagged subject** — `crates/nvs-ir/src/lower/expr.rs:1521`
      asserts `cond_ty == subj_ty`. ADR 0007 § 4's tagged rows are chosen in
      `nvs-ir` from the operand's representation everywhere else (`lower_binary`
      rewrites `Eq` over `Ty::Tagged` into `Helper::Identical`); do the same
      here — coerce each label to the subject's representation and emit the
      helper — and the assert becomes an internal-consistency check with a
      roster like its neighbours'. The agreement case above gains the `match`
      row it is missing.
- [ ] **A `foreach` whose subject is `void`** — the same shape one storage kind
      along, unchecked: `nvs_types::expr::iteration::foreach_source` is where a
      subject's type is asked for, and `crates/nvs-ir/src/lower/control.rs` is
      where an unlowerable one would arrive. One scratch run says whether it is
      a hole at all.
- [ ] **`emit_binop`'s residue roster, asserted** — its comment claims what is
      left is the three representations no source expression has (`ClassDesc`,
      `Ref`, `Void`). The `Void` third is now unreachable through a condition
      too; fold that into the comment when the `match` slice is landing in the
      same file set.

## Backlog

- A `catch` binding's members — `docs/plan/m4.md`, `nvs-ir` known gap.
- `nvs-ir` gap 22: a `require`'s value form needs a frame per file.
- `array<T> as array<U>` inside a `mixed` element — the nested-shape hole the
  playbook's `Core\Json::encode` bullet works around.
- `[context] modules` in `docs/agent/loop-goal.toml` names neither
  `nvs-runtime` nor `nvs-diagnostics`, both of which sessions keep editing.
