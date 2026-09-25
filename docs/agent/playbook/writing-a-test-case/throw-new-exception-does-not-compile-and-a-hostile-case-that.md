- **`throw new Exception` does not compile, and a hostile case that does not compile is a failure
  rather than a pass.** Novis declares `RuntimeError`, `LogicError`, `ParseError`, `IOError`,
  `TimeoutError` and `ArithmeticError`; `Exception` is PHP's name for the root and E0303 says only
  `no matching declaration`, which reads like a missing import. Take the spelling from
  `grep -rho "throw new [A-Za-z\\\\]*" tests/conformance` before writing one.
  [until: gone crates/nvs-diagnostics/src/lib.rs:E0303]
