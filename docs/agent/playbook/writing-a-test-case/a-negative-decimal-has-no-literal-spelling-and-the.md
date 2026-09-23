- **A negative `decimal` has no literal spelling, and the diagnostic is `E0401: expected decimal,
  found int`.** `rule:types/numeric-literal-placement` target-types the literal, and a unary minus
  in front of one is an ordinary operator over an `int`, so `decimal $d = -4;` does not compile.
  Build it by subtraction from a `decimal` that does, `decimal $four = 4; decimal $minusFour = 0 -
  $four;`, as `tests/conformance/lang/decimal-arithmetic-is-exact-and-keeps-its-scale.nvst` does
  with `0 - $price`. [until: reviewed 2026-09-06]
