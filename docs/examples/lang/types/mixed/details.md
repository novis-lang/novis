You can use a `mixed` value in arithmetic, as a condition, with `[]`, with `->`, with `.` and with
`is` or `as`. Each operation checks the value while the program runs. If the value has the wrong
type, it throws an error.

Arithmetic needs a number. `"abc" + 1` throws a `RuntimeError`, and so does `"5" + 1`. Convert text
first: `($m as int) + 1`. An `int` and a `float` add as they do without `mixed`: `5 + 1.5` is `6.5`.

`==` never converts a value. A `mixed` `"5"` is not equal to `5`.

`.` joins a number or a string. An array with `.` throws a `RuntimeError`. `[]` on a value that is
not an array throws a `RuntimeError`, and so does `->` for a field the object does not have.
Calling a method on a value that is not an object throws a `LogicError`.

A `mixed` value is never assigned to a narrower type without `as`. `int $i = $m;` is the error
`E0401`, and so is a `mixed` argument for a `string` parameter.
