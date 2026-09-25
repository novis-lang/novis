- **A fault raised inside a `Core` member reaches a `catch` one frame up with an empty `location`,
  so a case that prints it pins the emptiness rather than a site.** The site is seeded on the
  catchable edge of the frame that raised it — a `finally` seeds none — which is what
  `a-helper-fault-names-the-frame-it-was-raised-in-when-it-is-caught-there.nvst` is written around,
  while `backtrace` is filled either way. Print `Core\Arr::count($e->backtrace)` when the claim is
  that the throw unwound, and leave `location` to the case that owns it.
  [until: gone tests/conformance/error/a-helper-fault-names-the-frame-it-was-raised-in-when-it-is-caught-there.nvst:backtrace]
