- **An `--ORACLE--` case must never `echo` a `NAN`.** PHP 8.4 and later emit *"Warning: unexpected
  NAN value was coerced to string"* onto the same stream as the output, so the oracle's expectation
  carries a warning Novis's side cannot print and the case fails on a row that agrees; `INF` is
  fine. Render the value through a guard — `$v == $v` is false for exactly one `float`, on both
  sides — and echo a sentinel, as `math-int-div-and-mod-match-intdiv-and-fmod`'s `Show::real` does.
  [until: gone tests/differential/core/math-int-div-and-mod-match-intdiv-and-fmod.nvst:Show::real]
