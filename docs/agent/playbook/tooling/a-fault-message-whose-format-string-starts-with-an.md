- **A `Fault::` message whose format string *starts* with an interpolation is invisible to the
  error-path coverage gate, which then passes by accident.** `conformance_coverage.rs`'s
  `fault_sites` takes the stem before the first `{` and drops any under 14 characters, so
  `format!("{MEMBER} refused {arg:?}")` is never owed a case; and the match is
  `corpus.contains(&stem)` over raw `.nvst` text, so a stem holding `Core\IO::` needs a backslash no
  Novis string writes unescaped. Open a refusal with a backslash-free sentence naming the rule and
  put the member name in the interpolated tail.
  [until: gone crates/nvs-stdlib/tests/conformance_coverage.rs:OWED_A_CASE]
