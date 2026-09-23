- **A `?T` local does not narrow past the `if` that tested it, and `&&` narrows neither operand.**
  A guard clause — `if ($x == null) { echo "missing\n"; return; }` — leaves every statement after it
  still seeing `null|T`, and `if ($a != null && $b != null)` leaves both nullable inside the block,
  so a `.nvst` case written in the PHP habit fails `E0401`/`E0459` on its first run rather than at
  the line that looks wrong. Write `if ($x != null) { T $narrowed = $x; … }` and rebind once per
  value, nesting the `if`s where there are two, which also keeps the `--EXPECT--` block honest about
  interleaving. [until: reviewed 2026-09-17]
