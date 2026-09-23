- **An `int` literal does not reach an `array<float>`'s element type, so a case about the one
  numeric domain declares `array<int|float>`.** `Core\Arr::contains($floats, 1)` is `E0401: expected
  float, found int` at the argument: the needle is typed `T`, and arguments do not widen.
  `array<int|float> $numeric = [1.0, 2.5];` makes `T` the union, and `contains($numeric, 1)` answers
  `true`, the `rule:expressions/equality-semantics` row worth pinning. [until: reviewed 2026-09-06]
