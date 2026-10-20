A `return` inside a `finally` block does not compile: it is the error `E0250`. Change the result in
the `try` or in a `catch`, or after the whole block.

A `break` or `continue` that would leave a `finally` block does not compile: it is the error `E0251`.
This is also true for `break 2` from a loop inside the `finally`. A loop that is written completely
inside the `finally` block may use `break` and `continue`.

A `yield` inside a `finally` block does not compile: it is the error `E0255`. A generator cannot
pause there. Write the `yield` in the `try` block or after the whole block.

A `finally` block may contain its own `try` and `catch`. An error caught there does not leave the
block.

When the `try` returns a value, the value is computed first. The `finally` block runs after that, so
a change it makes does not change the value that the function returns.

When no `catch` handles an error, the `finally` blocks still run. Then the error leaves the function,
or ends the program at the top level.

A `finally` block can throw while an error is leaving the block. The handler outside then gets the
new error, and its `previous` is the first error. A method called in the `finally` that throws does
the same. If the new error already has a `previous`, it keeps that `previous`, and the first error is
lost.
