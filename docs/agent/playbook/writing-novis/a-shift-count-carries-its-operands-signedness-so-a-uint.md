- **A shift count carries its operand's signedness, so a `uint` shift needs a `uint` count.** `$u <<
  64` is `E0407: int and uint have no representable common type in arithmetic`, because
  `nvs_types::expr::operators::bitwise_result` refuses a mixed-signedness pair for all five binary
  bitwise rows and a count is just the right-hand operand. Declare `uint $width = 64;` and shift by
  that; on the `int` arm a negative count is `ArithmeticError: Bit shift by negative number` and a
  count of 64 or more answers `0` (or all-sign for `>>`). [until: reviewed 2026-09-06]
