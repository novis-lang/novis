The full order, from the operators that run first to the ones that run last:

- `->`, `?->`, `::`, `[…]`, a call `(…)`, `$x++`, `$x--` and `as T`, left to right
- `new` and `clone`
- `**`, right to left
- a prefix `-`, `+`, `~`, `++` and `--`
- `|>`, left to right
- `is`, left to right
- `!`
- `*`, `/` and `%`, left to right
- `+` and `-`, left to right
- `<<` and `>>`, left to right
- `.`, left to right
- `<`, `<=`, `>` and `>=`, left to right
- `==`, `!=` and `<=>`, left to right
- `&`, then `^`, then `|`, each left to right
- `&&`, then `||`, each left to right
- `??`, right to left
- `? :`, right to left
- `catch (T $e) => …` in an expression
- `=` and every other assignment, such as `+=` and `??=`, right to left
- `print`, `throw` and `yield`, which take everything to their right

`as` runs before every prefix and binary operator. `-$s as int` is `-($s as int)`, and
`$s as int ** 2` is `($s as int) ** 2`. To convert a result, write `($a / $b) as int`.

`!` runs after `is`. `!$o is User` is `!($o is User)`.

`|>` runs before every binary operator and after a prefix operator. `-$a |> Core\Math::abs($_)` is
`Core\Math::abs(-$a)`. In `"x=" . $a |> Core\Str::upper($_)`, only `$a` goes into the pipeline. In
`$x = $a |> Core\Str::trim($_)`, `$x` gets the result of the whole pipeline.

`new User()->name()` calls the method on the new object, with no parentheses around `new`.
`clone $a->b` copies the object in the property `$a->b`.

The compiler gives warning `W1022` in three cases:

- a `??` whose default is an arithmetic or `.` expression without parentheses, such as `$n ?? 0 + 10`
- a `? :` whose condition is an arithmetic or `.` expression without parentheses, such as
  `"n=" . $ok ? "x" : "y"`
- a `?:` or `? :` whose condition is a `??` without parentheses, such as `$m ?? 1 ?: 2`

The warning shows two ways to add the parentheses. The first keeps what the line does now. The
second is the order the line most likely meant. A warning does not stop the program.

A `catch` in an expression runs after the ternary and before the assignment. So in
`$x = $a / $b catch (ArithmeticError) => 0`, the `catch` covers the whole division, and `$x` gets
the result. A second `catch` after it is another case of the same `catch` expression.
