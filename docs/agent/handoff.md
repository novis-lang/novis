# Handoff

## State

**Goal `m4-refusals` — Stage 6 is closed, all three halves; Stage 7 is the next group.**
`python tools/holes.py` reports **3** refusal sites, `UNATTRIBUTED: 0`, **15** guarded, and
`crates/nvs-ir/tests/refusals.rs`'s `CEILING` is **3** to match.

- `crates/nvs-ir/src/lower/operator.rs:664`'s `emit_equality` is the one equality lowering in
  this crate: the `decimal`, tagged, `?class<T>`-against-`null`, enum and mixed-numeric rows,
  then the `BinOp` a matched pair is. A written `==`/`!=`, a `switch` label and a `match` arm
  all reach it, so `rule:expressions/switch-match-equality`'s "one comparison in the language"
  is one function rather than three tables.
- A pair no single value inhabits is that function's `guarded_by!` naming `E0466`, which
  `nvs_types::expr::operators::reject_disjoint_equality` raises at the comparison, at the label
  (`crates/nvs-types/src/locals.rs:1230`) and at the arm alike.
- `crates/nvs-ir/src/lower/convert.rs:510`'s truthy table is exhaustive: `Ty::Void` is guarded by
  `E0719` and `Ty::Ref` is an engine invariant naming `InstKind::RefLoad`.
- Ownership stays with each caller, which is the only side that knows whether an operand aliases a
  slot. The `switch` chain now stages a fresh label on the owned-temporaries stack the way the
  `match` chain already did, because the rows a cross-representation pair takes carry
  `rule:errors/propagation`'s error edge. Nothing is blocked.

## Next group

**Stage 7: `is` answers for every type** — one file set: `crates/nvs-ir/src/lower/expr.rs`, whose
`test_shape` and `lower_is` sit 500 lines apart in it. `docs/agent/loop-goal.md` § *Stage 7* is the
spec and `rule:types/type-test`'s "Nothing else is refused" is the rule; every row below is one the
`as` walk already performs, called in answering mode.

- [ ] **`is` over a union and over an intersection** — the members' own tests, short-circuiting on
      the first `true` and on the first `false` respectively —
      `crates/nvs-ir/src/lower/expr.rs:4986`, `rule:types/type-test`. No new diagnostic: `E0711`
      refusing `as array<Foo>` is a fact about `as`, and that rule is explicit `is` does not
      inherit it.
- [ ] **`is` over a shape, and over an `array<T>` whose element type no tag decides** — the field
      walk `as` performs for the same type, given a per-element test instead of a tag word, so
      there is one walk and not two — `crates/nvs-ir/src/lower/expr.rs:5458` (`test_shape`), whose
      `None` and known gap go with it.
- [ ] **`is` over `iterable` and over `callable`** — an `array`, or an object whose class
      implements `Iterable` or `Iterator` (`rule:iteration/two-interfaces`); a closure
      (`rule:types/callable-is-a-closure`) — `crates/nvs-ir/src/lower/expr.rs:4986`.

## Backlog

- `rule:security/secret-comparison-is-constant-time`'s row is read from what the checker recorded at
  a written `==`'s span, so a `switch`/`match` over a `secret` subject takes the ordinary `BinOp`
  instead — whether the rule wants the label too is unasked.
- `crates/nvs-ir/src/lib.rs` gap 1's remaining half: a `for` condition clause holding more than one
  comma-separated expression.
