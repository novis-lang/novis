A full re-analysis of a ~1,000-line document stays under a named bound, and the guard has the shape
`benches/abi-probe/tests/perf_guards.rs` already uses.

`rule:ide/one-grammar-one-tree` traded incremental reparse away, so every analysis reparses the whole
document; this measurement is what says the trade still holds. If the guard ever fails, the answer is real
work — item-level caching over the `SyntaxIndex`, then a decision to revisit with a number in hand — and
never a smaller number in the test. Architecture assumptions are tested, not remembered, and this is the
one thing M4B built nothing to protect.
