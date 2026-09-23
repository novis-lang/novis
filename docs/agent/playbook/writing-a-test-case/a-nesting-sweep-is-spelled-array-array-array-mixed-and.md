- **A nesting sweep is spelled `array<array<array<mixed>>>`, and `flatten` cannot be iterated to a
  fixed point.** `Core\Arr::flatten`'s parameter is `array<array<T>>`, so `Core\Arr::flatten($x)`
  over its own `array<mixed>` answer is `E0401`. Write one step over many shapes:
  `array<array<array<mixed>>> $rows` with `foreach ($rows as array<array<mixed>> $row)` makes
  nestings data, and `Core\Str::countOf($json, "[") == 1` asserts an answer holds no nested array.
  [until: reviewed 2026-09-06]
