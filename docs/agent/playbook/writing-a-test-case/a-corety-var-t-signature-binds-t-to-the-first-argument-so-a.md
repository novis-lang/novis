- **A `CoreTy::Var("T")` signature binds `T` to the first argument, so a mixed-type pair never
  reaches the runtime.** `Core\Math::min`, `max` and `clamp` are all `Var("T")`, so
  `Core\Math::min(0, "a")` and even `Core\Math::min(2, 1.5)` are `E0401` at the *second* argument.
  Declare the union on the bindings (`int|string $zero = 0;`); a `float` parameter does not widen an
  `int` literal either, so `Core\Math::mod(7, 2.0)` needs `7 as float`. [until: gone crates/nvs-stdlib/src/math.rs:Core\Math::min]
