- **`Core\Arr::count` answers `uint`, and an `int` binding or a bare `0` beside one does not compile.**
  `int $n = Core\Arr::count($rows)` is `E0401: expected int, found uint`, and
  `uint $d = $ok ? ($v as uint) : 0` is `uint|int` because the literal is an `int` — both read as a
  mistake in the member rather than in the binding. Bind counts as `uint`, convert the other side with
  `as uint`, and write the zero as a `uint $d = 0;` on its own line before the `if`.
  [until: gone crates/nvs-stdlib/src/arr.rs:Core\Arr::count]
