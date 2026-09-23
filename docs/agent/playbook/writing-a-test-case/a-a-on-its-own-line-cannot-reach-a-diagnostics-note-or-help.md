- **A `%A` on its own line cannot reach a diagnostic's `= note:` or `= help:` continuation, because
  the newline the wildcard ends on has to match one in the output.** Pinning `PHP 8.5's |> applies a
  callable` as its own expected line fails against `  = note: PHP 8.5's …` — the wildcard absorbs the
  indentation happily, but the pattern still demands a line break immediately before the literal, and
  the diff then prints two blocks that read alike. Write the wildcard inline instead, `%A= note: …`,
  which matches and also survives the gutter widening when the case's line number reaches two digits.
  [until: reviewed 2026-09-07]
