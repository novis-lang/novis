- **An array literal written directly as a `Core` argument or a `foreach` subject is `array<mixed>`
  and is refused.** `Core\Arr::replaceRange($a, 1, 2, ["X"])` is `E0401: expected array<string>,
  found array<mixed>` and `foreach (["a", "b"] as string $t)` is `E0401: expected string, found
  mixed`, because the receiving type is never pushed into the literal and `var` refuses to infer an
  element type (`E0414`). Declare a typed local one line above — `array<string> $rows = [...]` — and
  pass or iterate that. [until: reviewed 2026-09-06]
