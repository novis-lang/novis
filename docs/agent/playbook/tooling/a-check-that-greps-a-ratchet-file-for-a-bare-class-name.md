- **A `[[check]]` that greps a ratchet file for a bare class name matches the header comment that
  explains the key shape, not a key.** `crates/nvs-stdlib/tests/spec-members-compiler-facing-outstanding.txt`
  names `Core\BigInt` twice while stating what a key looks like, so an `exit = "nonzero"` gate over a
  bare `BigInt` goes red the day the last key is struck rather than green. Match a line that is not a
  comment — `git grep -E "^[^#]*<Class>"` — which is why the `Core\Test` gate beside it greps `Test::`.
  [until: gone crates/nvs-stdlib/tests/spec-members-compiler-facing-outstanding.txt]
