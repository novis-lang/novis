A condition can test a value of any type. The value is either truthy or falsy.

The conditions are the test of an `if`, a `while` and the middle part of a `for`. The test of a
ternary `? :` and of `?:` is a condition too, and so are the values beside `&&`, `||` and `!`.
These values are falsy: `false`, `0`, `0.0`, `""`, `"0"`, an empty array, an empty `bytes` and
`null`. Every other value is truthy. This includes `"0.0"`, `" "` and every object.

**Good to know:** only a condition tests a value this way. A `bool` variable, a `bool` parameter and
a `bool` return value need `as bool`, which uses the same table. `empty($x)` is `true` when `$x` is
falsy. `isset($x)` is `true` when `$x` is not `null`.

**The examples below** test the fields of a form, show each kind of condition, build the header of
an email from settings that may be empty, and convert values with `as bool`.
