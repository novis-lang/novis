Each type that is not a number gives a compile error with an arithmetic operator:

- `"3" * 2` and `true + 1` are the error `E0716`. A `bool` becomes a number with `$b ? 1 : 0`.
- A value that can be `null`, such as a `?float`, is the error `E0716` too. Test it first with
  `if ($a != null)`, or give it a default with `??`: `($a ?? 0.0) + 1.5`.
- `+"3"` and `-"3"` are the error `E0705`.
- Two arrays with `+` are the error `E0467`.

A `mixed` value is checked when the program runs. A `mixed` that is `"3"` in `$m + 1` throws a
`RuntimeError`.

`int / int` has the type `int|float`. So `float $avg = 7 / 2;` compiles, and `int $n = 7 / 2;` is
the error `E0401`. `6 / 3` is the `int` `2` when the program runs. `Core\Math::intDiv(7, 2)` is
integer division, and it gives `3`.

`%` works on `int`, `uint` and `decimal`. On a `float` it is the error `E0717`, and
`Core\Math::mod(5.5, 2.0)` gives the remainder `1.5`. The result has the sign of the number on the
left: `(-7) % 3` is `-1`, and `7 % -3` is `1`.

`int` with `uint` is the error `E0407`, and so is a shift or a bitwise operator between them. An
integer literal next to a `uint` is a `uint`, so `$u + 3` compiles.

`decimal` with `float` is the error `E0455`, also for a `float` literal. Write
`$d + (0.2 as decimal)`. `decimal` with `int` or `uint` gives a `decimal`. `**` on a `decimal` is the
error `E0455` too, and `Core\Decimal::pow` is the power. A `decimal` division keeps up to 28 digits
after the point: `1.0 / 3.0` is `0.3333333333333333333333333333`.

`**` with whole numbers gives a whole number. A negative exponent throws, except with the base `1`
or `-1`: `(-1) ** -3` is `-1`. `2 ** 0.5` is a `float`.

Each overflow of a whole number throws an `ArithmeticError` with its own message: `Integer addition
overflowed`, `Integer subtraction overflowed`, `Integer multiplication overflowed`, `Integer
exponentiation overflowed` and `Integer negation overflowed`. `0 - 1` on a `uint` overflows too. A
`float` that is too large becomes `INF` and does not throw.

A division by zero throws an `ArithmeticError` with the message `Division by zero`. This is also
true for a `float`, such as `1.0 / 0`. For a `decimal`, the error has another message, the same one
as a result that is too large. `%` by zero gives `Modulo by zero`, and `Core\Math::intDiv(1, 0)`
throws as well.

`++` and `--` work on an `int`, `uint`, `float` or `decimal` variable, property or array element.
On a string they are the error `E0474`, and on a value such as `3++` the error `E0105`.

The bitwise operators `& | ^ ~ << >>` work on `int` and `uint`, and give the same type. On a `float`
they are the error `E0706`. A shift does not check for overflow, so `1 << 63` is
`-9223372036854775808`.
