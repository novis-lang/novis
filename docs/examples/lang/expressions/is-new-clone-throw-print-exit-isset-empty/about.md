Eight small expressions that test a value, build one, copy one or stop the program.

`$x is T` is true when the value in `$x` is a `T` right now, and in the branch where it is true the
compiler knows `$x` is a `T`, so you can use it there. `new C(...)` builds an object. `clone $o`
makes a shallow copy: numbers, text and arrays are copied, and an object in a property is shared
with the original. `isset($x)` is true when `$x` is there and its value is not `null`, and
`empty($x)` is true when `$x` is false by the truth table. Both take a variable, a property or an
array element, and a key that is not there gives `false`.

**Good to know:** `throw`, `print` and `exit` are expressions as well, so each one can sit on the
right of `??`. `print` writes one value and gives `1`, and `exit` ends the program.
