- **`int as float` is checked and refuses past 2^53, not past `int`'s own range.** `nvs_runtime`'s
  `int_to_float` is `value.unsigned_abs() <= F64_EXACT_INT_LIMIT`, so `Core\Math::INT_MIN as float`
  throws `cannot convert `int` -9223372036854775808 to `float`` even though -2^63 is exactly
  representable. A case sweeping a `float`-typed member over an `int` table is bounded at
  ±9007199254740992; reaching `int`'s extremes on the float side needs a `float` *literal* (`0.0 -
  9223372036854775808.0`). [until: gone crates/nvs-runtime/src/helpers.rs:F64_EXACT_INT_LIMIT]
