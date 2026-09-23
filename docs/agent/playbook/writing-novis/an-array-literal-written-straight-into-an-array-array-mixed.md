- **An array literal written straight into an `array<array<mixed>>` element reads as `array<mixed>`,
  and does not satisfy an `array<array<T>>` parameter.** `Core\Arr::flatten([$s, $s])` inside an
  `array<array<mixed>>` literal is `E0401: expected array<array<mixed>>, found array<mixed>` at the
  inner literal, because the inner literal is checked against the outer's element type. Bind it
  first (`array<array<string>> $pair = [$s, $s];`) and pass the binding.
  [until: reviewed 2026-09-06]
