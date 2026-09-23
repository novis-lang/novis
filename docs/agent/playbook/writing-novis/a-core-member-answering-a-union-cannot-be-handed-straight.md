- **A `Core` member answering a union cannot be handed straight back to a parameter declared `T`.**
  `Core\Arr::append($a, Core\Arr::sum($empty))` is `E0401: expected int, found int|float|decimal`,
  because `sum`/`product`/`average` answer the whole union whatever their subject's element type
  was, and `as int` over a union does not lower. A case that wants to feed a fold's answer back into
  the array writes the literal and asserts separately that the member answers it; rendering the
  union is fine, since `echo` and `as string` both take it. [until: reviewed 2026-09-06]
