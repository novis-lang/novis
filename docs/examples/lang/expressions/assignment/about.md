`=` writes a value into a local variable, a property, a static property or an array element. It is
also an expression. Its value is what was just written, so `$a = $b = 3` sets both.

Every compound form, such as `+=` or `.=`, is a short way to write `$x = $x op e`. The target is
worked out once, so `$totals[next()] += 1` runs `next()` once. A compound form cannot change the
type of its target, so `$n /= 2` on an `int` does not compile.

`??+=`, `??-=` and `??.=` update a value that may not exist yet. `$count["paid"] ??+= 1` starts
from `0` when the key is missing or the value is `null`. A string starts from `""`.

**The examples below** write and update values, update arrays and objects, and tally a report with
`??+=`. The last two show every compound operator, and the error for `/=` on an `int`.
