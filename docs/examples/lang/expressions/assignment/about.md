`=` writes a value into a local variable, a property, a static property or an array element. It is
an expression as well as a statement: its value is what was just written, so `$a = $b = 3` sets both
and `($a = 5) + 1` gives 6.

Every compound form, such as `+=`, `-=`, `*=` or `.=`, is a short way to write `$x = $x op e`. The
target is worked out a single time, so `$totals[next()] += 1` runs `next()` once. A compound form
cannot change the type of what it writes into, so `$n /= 2` on an `int` does not compile.

`??+=`, `??-=` and `??.=` update a value that may not exist yet. `$count["paid"] ??+= 1` starts
from `0` when the key is missing or the value is `null`. A string starts from `""`. Use the plain
form when the target always has a value.

**The examples below** take these in turn: writing and updating a value, updating what is inside an
array and an object, and tallying a report with `??+=` first and the same update written out after.
