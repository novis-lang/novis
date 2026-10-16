`echo`, `print`, `unset`, `exit` and `yield` are five short statements: they write output, remove an
array entry, end the program, and produce values one at a time.

`echo a, b, c;` writes several values in order with nothing between them. `print $value;` writes one
value, and its result is `1`. Both turn a value into text the same way `.` does: `true` writes `1`,
and `false` and `null` write nothing.

`unset($a["k"]);` removes that entry from the array `$a`, and nothing else. Every variable and every
property in Novis always has a value, so `unset($x)` and `unset($o->p)` do not compile. Assign `null`
instead, where the type allows it.

`exit;` ends the program with status 0. `exit(3);` ends it with status 3, and `exit("bye")` writes the
text and then ends with status 0.

`yield $value;` goes inside a method that returns `Iterator<T>`. It sends one value to the loop that
reads it. A `yield` is a statement, so `$x = yield 1;` does not compile.
