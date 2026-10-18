An `int` is a 64-bit signed whole number. A `uint` is a 64-bit unsigned whole number, from `0` to
`18446744073709551615`. A result outside that range throws an `ArithmeticError`, such as
`Integer addition overflowed`. Below zero, a `uint` throws `Integer subtraction overflowed`.

`-1` is never a `uint`, because the `-` is an operator. `uint $u = -1;` is the error `E0401`.
Beside a `uint` variable, a plain number is a `uint` too, so `$u + 1` compiles.

`$i + $u` with an `int` and a `uint` is the error `E0407`. An `int` variable does not assign to a
`uint` variable either: that is the error `E0401`. Convert one side with `as int` or `as uint`.

When one side of a sum is a `float`, the other side is converted to `float`. A `decimal` and a
`float` in one sum is the error `E0455`. A `decimal` and an `int` or a `uint` give a `decimal`.

A `float` prints without a trailing `.0`, so `2.0` prints `2`. `0.1 + 0.2` prints `0.3`, but it is
not equal to `0.3`.

A `decimal` keeps its decimal places: `1.10 * 3` is `3.30`. Division rounds at the 28th place, so
`1 as decimal / 3` is `0.3333333333333333333333333333`.

`bool` has no arithmetic: `$b + 1` is the error `E0716`. A `bool` converts only to `string` and to
`bool`. `true as int` is the error `E0708`, so write `$b ? 1 : 0`. `true as string` is `"1"` and
`false as string` is `""`.

`true` and `false` are also types of their own, with one value each.
