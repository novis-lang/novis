# Handoff

## State

**Stage 10 item 38 is closed: all three of ADR 0125 § 4's dynamic sites lower.** `$x instanceof $cls`
joins `new $cls(...)` and `$cls::f(...)`, and the goal's `conformance (the class reference)` check
passes — its four cases are on disk and green, which is what the driver's last acceptance failure was
naming.

**`InstKind::InstanceOf` now takes a `TestedClass`** (`crates/nvs-ir/src/ir.rs:1408`) rather than a
`class: String`: `Named` for every written site — both `catch` ladders, `as`'s downcast, a closure's
parameter check — and `Descriptor` for the dynamic one. That enum's own doc comment is the home for why
the two forms are one instruction and why the written form keeps its label rather than folding into an
`InstKind::ClassDescConst`.

**Nothing new is recorded in the typed-expression table for the dynamic site**, and
`Lowering::lower_instanceof`'s doc comment owns why: `ExprInfo::InstanceOf` exists to carry a *resolved
name*, and here there is no name — which of the two forms a site takes is decided by the shape of the
right-hand side, and the checker's `E0496` is what leaves only those two shapes.

**Known gap, unchanged and recorded on `ExprInfo::ClassRefCall`:** `$cls::f(...)` written as ADR 0027's
first-class callable still records `ExprInfo::CallableRef`, so the `Closure` names `T`'s method rather
than the implementor's. It needs a refusal or a descriptor-carrying closure, not a lowering.

**`as ?class<T>` still has no row** and still needs the representation decision ADR 0125 § 2 promises;
`Lowering::lower_class_reference`'s *Known gaps* names the shape.

**`orient.py`'s `[context]` gaps.** The next group is documentation, so the standing one now bites: no
field selects `docs/reference/lang/*.md` or `docs/reference/tools/*.md`, and the next session needs
both. Also standing: `docs/adr/README.md` and `ground-rules.md` are not in `modules`, and the pack
prints the goal item but not the `[[check]]` grading it.

## Next group

**Item 39's second half — the reference and the tables, file set `docs/reference/lang/20-types.md`,
`docs/reference/lang/50-classes.md`, `docs/reference/tools/30-php-differences.md`,
`docs/reference/findings.md` and two ADRs.** The corpus half is done; every item below is prose over
landed behaviour, and `python tools/reference.py` runs the examples.

- [ ] **The type atom and its `as` row.** `class<T>` beside `callable` and the class names at
      `docs/reference/lang/20-types.md:185`, and the conversion row at
      `docs/reference/lang/20-types.md:511` — the string door, the folded `Foo::class` door, and the
      throw ADR 0125 § 2 specifies. ADR 0125 §§ 2, 4.
- [ ] **`instanceof`'s operand and the `new` paragraph beside it** at
      `docs/reference/lang/50-classes.md:476`: all three sites accept a class reference and nothing
      else, and § 5's refusal is what a `new` over one costs. ADR 0125 §§ 4, 5.
- [ ] **The four tables that still say there is no spelling.** The differences row at
      `docs/reference/tools/30-php-differences.md:31`, `docs/reference/findings.md:61`, ADR 0007 § 2's
      `as` table plus § 7 row 14's tail at `docs/adr/0007-explicit-type-system.md:396`, and ADR 0019's
      DI-container sentence at
      `docs/adr/0019-reflection-and-ast-parsing-are-core-features.md:120`.

## Backlog

- `$cls::f(...)` as a first-class callable names `T`'s method — `nvs_types::expr_table::ExprInfo`'s
  `ClassRefCall` doc owns it.
- `as ?class<T>` has no row — `nvs_ir::lower::convert::Lowering::lower_class_reference`'s *Known gaps*.
- A `-p nvs-ir` lowering test for `InstanceOf`'s descriptor form; the four `.nvst` cases pin the
  behaviour end to end today.
- ADR 0017's freeing of executable memory — `docs/plan/m6.md` carries it, no consumer until goal 6.
- `python tools/bench.py --warm-start --max-ms 10` — the goal's own § *The harness this goal owes*.
