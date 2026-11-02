A full re-analysis of a ~1,000-line document stays under a named bound, and the guard has the shape
`benches/abi-probe/tests/perf_guards.rs` already uses: a soft bound that warns and a hard bound, ten
times it, that fails (`crates/nvs-lsp/tests/latency.rs`).

A full analysis is the cold and invalidated path: the first analysis of a file with no valid index entry,
and an analysis after the session's interner is rebuilt. A keystroke's cost is the counted one of
`rule:ide/the-index-is-incremental-and-persistent`, the files parsed and the bodies checked, and the
counted tests guard it rather than a clock. `rule:ide/one-grammar-one-tree` traded incremental reparse
away, so the edited file is still reparsed whole; this measurement is what says the trade still holds. If
the guard ever fails, the answer is real work — item-level caching over the `SyntaxIndex`, then a decision
to revisit with a number in hand — and never a smaller number in the test. Architecture assumptions are
tested, not remembered, and nothing else in the tree protects this one.
