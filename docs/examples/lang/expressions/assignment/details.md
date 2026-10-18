The compound forms are `+=`, `-=`, `*=`, `/=`, `%=`, `**=`, `.=`, `<<=`, `>>=`, `&=`, `|=` and `^=`.
There is no `<=>=` and no `|>=`.

A compound form needs a value that fits the type of the target. Each of these is the error `E0401`:

- `$n /= 2` on an `int`, because `/` gives `int|float`
- `$n += 1.5` on an `int`
- `$n .= "x"` on an `int`

`$f += 1` on a `float` works, because the `int` is widened. A bitwise form on a `float`, such as
`$f |= 1`, is the error `E0706`. On an `?int`, `$a += 1` is the error `E0716`, even when the value is
not `null`. Use `??+=` or test the value first.

A compound form throws the errors of its operator. On an `int` or a `uint`, a result out of range
throws an `ArithmeticError`, and so does `$n %= 0`. `$n **= -1` on an `int` throws too.

A compound form on an array element reads the element first. `$a["k"] += 1` throws a `RuntimeError`
when the key `"k"` is missing. `??+=` starts a missing key at `0`.

The left side must be a place to store a value. Writing to a constant or to a call, such as `P::X =
2`, is the error `E0105`. Writing a `readonly` property after the constructor is the error `E0782`.
A write to a variable that was never declared is the error `E0301`. A parameter is a variable, so a
method may write to it.

`if ($a = 0)` compiles. It writes `0` and tests the value written, so the `else` block runs.
