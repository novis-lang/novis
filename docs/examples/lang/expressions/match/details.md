The subject goes in brackets after `match`. Without the brackets, the code does not compile.
The subject runs once, before any arm.

The arms are tested in the order they are written. An arm's values run one by one, and the test
stops at the first match. The arms after it do not run. When two arms have the same value, the first
one gives the result.

An arm may have several values, separated by commas. This works in `match (true)` too:
`$n > 5, $n < 0 => "far"` matches when one of the two conditions is true. A comma after the last
arm is allowed.

Each value is compared with the subject by `==`. A `mixed` subject that contains the string `"5"`
does not match the arm `5`. A value whose type can never be equal to the subject is the error `E0466`.
A string against an `int` is one such case. `null` against an `int` subject is another: the subject
must be a `?int` for that arm. In `match (true)`, each condition must be a `bool`, and an `int`
condition is `E0466` too.

In `match (true)`, a condition such as `$m is int` narrows the variable in that arm's result.
`match (true) { $m is int => $m + 1, default => 0 }` gives `6` when `$m` is `5`.

The type of a `match` is the union of its results. With the results `"a"` and `2`, the type is
`string|int`. Assigning that to a `string` variable is the error `E0401`.

A result is one expression. A block of statements in braces does not compile. A result may be a
`throw`, such as `default => throw new RuntimeError("bad")`.

A `match` with no arms is the error `E0476`. When no arm matches and there is no `default`, the
`LogicError` has the message ``no `match` arm matched the subject``. This also happens for an enum
case that has no arm, because the compiler does not check that the arms list every case.
