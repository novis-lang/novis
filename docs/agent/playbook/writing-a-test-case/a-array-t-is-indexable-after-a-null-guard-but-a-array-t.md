- **A `?array<T>` is indexable after a `!= null` guard, but a `?array<T>` *answer* indexed directly
  is still `E0482`.** `narrow` drops `null` and `Lowering::untag_narrowed` untags the read at the
  variable, so every read inside the guard works; `Core\Arr::first($rows)` indexed inline has no
  test to narrow. Bind it first (`?array<string> $row = Core\Arr::first($rows);`); under `??` every
  level of the chain is guarded and `$a["nope"]["j"] ?? "d"` needs no test.
  [until: reviewed 2026-09-06]
