# Handoff

## State

**Stage 10 item 37 is closed — the checker knows `class<T>` end to end.** `Ty::ClassRef(TypeId)`
(`crates/nvs-types/src/ty.rs:83`) is the interned type; `as` is its only source
(`ConvKind::ClassRef`, `crates/nvs-types/src/expr/operators.rs:1719`); it widens with its argument
(`crates/nvs-types/src/expr/assign.rs:173`). ADR 0125 § 4's three sites now take the value:
`check_new_target`'s `NewTarget::Expr` arm answers with `T` so `infer_new` types the arguments
against `T`'s constructor, `infer_static_call` resolves the member on `T`'s roster, and
`infer_instanceof` accepts the operand without consulting `T` at all. § 5's `E0794` is
`reject_divergent_implementor_constructor` (`crates/nvs-types/src/expr/calls.rs:587`), which walks
`nvs_hir::implementors` and reports the first subclass whose own constructor is not substitutable.
`reject_dynamic_class_name` keeps `E0496` for everything else, its help naming `as class<Base>`.

**The seam item 38 must close: the checker records *nothing* in the expr table for any of the three
dynamic sites, and that is deliberate.** `ExprInfo::New`, `ExprInfo::Call` and `ExprInfo::InstanceOf`
each name a *written* class — the one `nvs-codegen` bakes an address in for — so recording `T` there
would lower an allocation of the base and a direct call to its body. Both record sites are guarded
and say so in full. `nvs-ir` therefore finds no entry for such a span and panics; that is unreachable
today because `lower_decl_type` (`crates/nvs-ir/src/lower/mod.rs:2748`) refuses the atom first, and
item 38 lifts both together or not at all. No `.nvst` case can name `class<T>` until it does.

**`orient.py`'s `[context]` gaps.** Standing and unchanged: the pack prints the goal item but not the
`[[check]]` grading it, so this session re-read `loop-goal.toml` for the stage 10 block. No field
selects `docs/reference/lang/*.md` (item 39 needs it); `docs/adr/README.md` and
`ground-rules.md` are not in `modules`. In `adrs`: **0125 §§ 4-5** — every remaining slice of this
stage pays for them, and this session sliced 0125 again. The `crates/nvs-host/src/budget.rs` warning
is the documented forward anchor and is not a bug.

## Next group

**Item 38, the lowering — file set `crates/nvs-ir/src/lower/` plus `crates/nvs-ir/src/ir.rs` and
`crates/nvs-types/src/expr_table.rs`.** Nothing below the checker knows the type, so the first slice
is what unblocks every other one, including item 39's four `.nvst` cases.

- [ ] **The atom lowers, and so does the conversion.** `lower_decl_type`
      (`crates/nvs-ir/src/lower/mod.rs:2748`) answers for `Ty::ClassRef` — a descriptor is one word,
      so the representation is the pointer `new static` already carries — and `lower_conversion`
      (`crates/nvs-ir/src/lower/convert.rs:629`) grows § 2's two rows: the `string` door is a
      hierarchy walk that throws, and `class<U> → class<T>` is the same walk over a descriptor in
      hand. The AST side is `Type::ClassRef` (`crates/nvs-syntax/src/ast.rs:153`). ADR 0125 §§ 1-2.
- [ ] **The three sites' entries, and the instructions that read them.**
      `crates/nvs-types/src/expr_table.rs:615`'s
      `ExprInfo::InstanceOf` needs a descriptor-valued twin and `New`/`Call` a dynamic one; the
      checker's two guards (`crates/nvs-types/src/expr/calls.rs` at the `ExprInfo::New` and
      `ExprInfo::Call` records) come off in the same slice. `InstKind::NewDynamic`
      (`crates/nvs-ir/src/ir.rs:547`) and `InstanceOf` (`crates/nvs-ir/src/ir.rs:750`) are what they
      lower to. ADR 0125 § 4's table. Closes `a_new_through_a_class_reference_lowers_to_new_dynamic`
      and `a_folded_class_constant_lowers_to_a_descriptor_constant`.
- [ ] **Item 39, the corpus and the reference.** The four `.nvst` cases the stage 10 check names
      (`docs/agent/loop-goal.toml:2193`), and the `class<T>` row beside `array<T>` in
      `docs/reference/lang/20-types.md:1`. The atom the reference has to describe is
      `crates/nvs-syntax/src/ast.rs:153`. Only reachable once both slices above are green.

## Backlog

- ADR 0125 § 5 is checked per `new` site; a program with many of them re-walks `implementors` each
  time. Cheap today, and the memo belongs in `nvs-hir`'s graph if it ever is not.
- `E0404` (`E_INCOMPATIBLE_OVERRIDE`) is declared and unused: no override-compatibility check exists,
  which is why § 5's comparison is written out in `constructor_accepts_everything` rather than shared.
- Stage 8: conformance 1087, differential 206 of 210, migration 37% over its 36% floor
  (`docs/implementation-plan.md`).
- `docs/reference/findings.md` § *Triage* holds Stage 0c's verdicts; nothing implementable left.
