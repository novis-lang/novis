- **A `?T` from a `Core\Db\Row` reader narrows through `if ($x != null)` and through nothing else —
  not a declared local, and not an `&&` chain.** `Core\Time\Date $born = $row->date("born");` is
  `E0401` (expected `Core\Time\Date`, found `null|Core\Time\Date`) even though the scalar `string
  $unwrapped = $named->string("owner");` next door compiles, and `if ($a != null && $b != null)`
  still raises `E0459` on both receivers in the body. Write one `if` per object receiver; a
  `?decimal` or `?bool` needs none, since `echo` and a ternary condition take them.
  [until: reviewed 2026-09-06]
