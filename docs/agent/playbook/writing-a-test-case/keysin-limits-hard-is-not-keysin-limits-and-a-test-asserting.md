- **`keys_in("limits.hard")` is not `keys_in("limits")`, and a test asserting the two blocks agree
  key for key fails on the design rather than on a defect.** The ceiling block is its own struct in
  `crates/nvs-config/src/tree.rs` carrying only the six budgets a request may write, while
  `[limits]` also carries the reserve pair, the recursion cap, the two decompression keys and `hard`
  itself — a ceiling over a value no request can move would bound nothing. Subtract the keys some
  `limits.*` row carves out of the block before comparing the pair, which is what
  `every_budget_with_a_ceiling_answers_the_request_and_the_operator_differently` does.
  [until: gone crates/nvs-config/tests/directives.rs:every_budget_with_a_ceiling_answers_the_request_and_the_operator_differently]
