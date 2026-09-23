- **A new compile-time refusal can break a green `.nvst` written for the *runtime* half of the same
  rule sentence.** `rule:routing/an-absolute-link-takes-a-configured-origin` makes a leftover
  `$params` key the query string *and* an undeclared one a compile error; landing the second half
  turned `tests/conformance/core/router-url-turns-a-leftover-params-key-into-a-query-string.nvst`
  red with `E0759`. Before adding a refusal, `grep -rl` the corpus for what it refuses.
  [until: reviewed 2026-09-06]
