- **A `reject` case's `--EXPECTF-ERROR--` block lists the *later* passes' diagnostics too, because a
  parse-band refusal does not stop the compile.** A case pinning `E0250` for a `return` in a
  `finally` also collected an `E0301` for an undeclared local in its own scaffolding, between the two
  errors it meant to pin and inside the `aborting due to N errors` count. Declare every local a
  reject case writes (`var $n = 1;`), and take the expected block from the runner's own `actual:`
  dump rather than from what the refusal alone would print. [until: reviewed 2026-09-07]
