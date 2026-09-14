# Handoff

## State

**Goal `m4-refusals` — Stage 7's written-`callable` row is decided, and the checker half is landed.**
The decision, whose home is `crates/nvs-types/src/callables.rs`' module doc: a closure carries **no**
signature at run time and is given none. Its object holds a parameter count and one tag nibble per
parameter for the dynamic call path, and a nibble names no class and no return type; the class
`nvs-ir` synthesizes per literal *does* name the signature exactly. So the row is one **marker
supertype per tested signature**, conformed to by every literal whose own signature is assignable to
it — the descriptor walk `$x is callable` already answers with, one step more specific, and the
`TestShape::Class` row costs exactly what that one costs.

- **The relation is `nvs_types::expr::is_assignable` itself**, not a second reading of it, so `is`
  admits what a binding of that type would: a wider parameter and a narrower return both conform
  (`crates/nvs-types/src/expr/assign.rs:231`), and so does a literal declaring *fewer* parameters.
  This is why the conformance cannot be computed in `nvs-ir`: that relation needs a `ClassGraph`, a
  `SignatureTable` and a mutable interner, and lowering holds none of the three.
- **Both spellings that make a closure record their signature** —
  `crates/nvs-types/src/expr/calls.rs`'s `fn` literal and its two first-class-callable sites — keyed
  by the literal's span, because a first-class callable's class label is `nvs-ir`'s own name for a
  site and never reaches the checker.
- **`ExprTypeTable` carries the answer across the crate seam**: `callable_sig_marker(TypeId)` for the
  row, `callable_sig_markers()` for the descriptors to emit, `callable_markers_at(Span)` for the
  edges. **Nothing in `nvs-ir` reads them yet** — that is the next group, and until it lands
  `$f is callable(int): string` still reaches `crates/nvs-ir/src/lower/expr.rs:4986`'s panic.
- `python tools/holes.py` is unchanged at **3** refusal sites, `UNATTRIBUTED: 0`: this slice closed no
  site, and the panic keeps every word it had.
- Stopped here on the context gate, not on a problem.
- Nothing is blocked.

## Next group

**Stage 7: the `callable(...)` row over the landed decision** — one file set:
`crates/nvs-ir/src/lower/expr.rs`, `crates/nvs-ir/src/lower/mod.rs`, `crates/nvs-ir/src/lower/closure.rs`
and `tests/conformance/lang/an-is-test-against-iterable-or-callable-answers-by-what-the-value-holds.nvst`.
`rule:types/type-test` is the rule and `rule:types/callable-signature` owns the type;
`crates/nvs-types/src/callables.rs` is the design and needs no re-deciding.

- [ ] **The descriptors, and the edge on each closure class** — emit one field-less `ir::Class` per
      `exprs.callable_sig_markers()` beside the marker every program already gets
      (`crates/nvs-ir/src/lower/mod.rs:805`), *before* the sort that keeps the class list
      reproducible (`crates/nvs-ir/src/lower/mod.rs:823`). Then push
      `exprs.callable_markers_at(<literal span>)` onto `conforms` at both synthesis sites — the `fn`
      literal's (`crates/nvs-ir/src/lower/closure.rs:296`) and the first-class callable's
      (`crates/nvs-ir/src/lower/closure.rs:748`).
- [ ] **The row** — thread `exprs: &ExprTypeTable` through `test_shape`
      (`crates/nvs-ir/src/lower/expr.rs:5885`, four recursive sites) and answer
      `CheckedTy::CallableSig` with `TestShape::Class(exprs.callable_sig_marker(tested)?)` beside the
      bare-`callable` arm at `crates/nvs-ir/src/lower/expr.rs:5940`. A signature the checker recorded
      no marker for stays `None`, which is the safe direction: it reaches the panic rather than a walk
      against a label the unit never declared.
- [ ] **The stage's case, and the two prose homes the row closes** — drop the callable clause from
      `test_shape`'s `# Known gaps` (`crates/nvs-ir/src/lower/expr.rs:5875`) and from the panic's
      message (`crates/nvs-ir/src/lower/expr.rs:4986`), keeping its `only lowers` wording so
      `tools/holes.py`'s count does not move; then extend
      `tests/conformance/lang/an-is-test-against-iterable-or-callable-answers-by-what-the-value-holds.nvst`
      with a written signature answering `true`, one answering `false` on the return type alone, and a
      first-class callable.

## Backlog

- `test_shape`'s `# Known gaps` also still lists the `array<Foo>` element walk that goal
  `m4-refusals` stage 7 already landed — `crates/nvs-ir/src/lower/expr.rs:5875`.
- Stage 8's declared-type probes are the goal's next stage and the driver's earliest red check —
  `docs/agent/loop-goal.toml:9698`.
