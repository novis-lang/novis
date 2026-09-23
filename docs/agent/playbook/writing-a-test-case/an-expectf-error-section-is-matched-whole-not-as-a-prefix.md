- **An `--EXPECTF-ERROR--` section is matched whole, not as a prefix.** A sweep that ends at its
  last `error[...]` line fails against a compiler that then prints `error: aborting due to N
  errors`. End the section with `%A` and that line, count included — a second assertion that no
  *extra* diagnostic crept in; `tests/conformance/lang/a-void-call-is-not-an-operand.nvst` has the
  shape and the conventions' skeleton does not. [until: reviewed 2026-09-06]
