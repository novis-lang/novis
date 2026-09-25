- **`Core\Math::INT_MAX as float` throws at run time, so a case wanting a huge finite float reaches
  for `Core\Math::FLOAT_MAX`.** `int as float` is one of `rule:types/conversion`'s checked
  conversions and 2^63-1 is not representable in an `f64`, so the row is `Uncaught Exception: cannot
  convert `int` 9223372036854775807 to `float`` with nothing said at compile time. `Core\Math` has
  `FLOAT_MAX`, `FLOAT_MIN` (the smallest positive normal, not `f64::MIN`), `INFINITY` and `NAN`; a
  derived infinity is `Core\Math::FLOAT_MAX * 10.0` or `Core\Math::log(0.0)`.
  [until: gone crates/nvs-stdlib/src/math.rs:FLOAT_MAX]
