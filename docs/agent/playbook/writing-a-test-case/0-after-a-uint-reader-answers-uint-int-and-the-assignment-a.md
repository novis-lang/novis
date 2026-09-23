- **`?? 0` after a `?uint` reader answers `uint|int`, and the assignment a line later is what
  fails.** `Core\Db\Row::uint` answers `?uint` while a bare `0` literal is an `int`, so
  `uint $total = $total + ($row->uint('size') ?? 0)` is `E0401 expected uint, found mixed` — the
  same line over `->int` compiles, which is why it reads as a bug in the member. Declare a
  `uint $zero = 0;` and write `?? $zero`, or read into a `?uint` and narrow it with
  `if ($size != null)`. [until: reviewed 2026-09-21]
