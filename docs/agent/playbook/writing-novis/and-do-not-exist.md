- **`===` and `!==` do not exist**, and reaching for one in a `.nvst` is `E0232` on the operator
  rather than a type error you can read past: Novis keeps exactly one equality operator, `==`, which
  never converts either operand. A null test is `$x == null`.
  [until: gone crates/nvs-diagnostics/src/lib.rs:E_IDENTITY_OPERATOR_UNSUPPORTED]
