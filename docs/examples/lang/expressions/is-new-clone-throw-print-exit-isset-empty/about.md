Eight small expressions that test a value, build one, copy one or stop the program.

`$x is T` is true when the value in `$x` is a `T`. Where the test is true, `$x` has the type
`T`. `$x is $cls` tests against a `class<T>` variable. `new C(...)` builds an object.

`clone $o` makes a shallow copy. An object in a property is
shared with the original. `isset($x)` is true when `$x` is there and is not `null`. `empty($x)` is
true when `$x` is false by the truth table. Both work on a variable, a property or an array element.

**Good to know:** `throw`, `print` and `exit` are expressions too, so each can be the right side of
`??`. `print` writes one value and gives `1`.

**The examples below** show a type test, an object and its copy, and settings that may be missing.
The last two show all of these, and a `string` after `is`, which does not compile.
