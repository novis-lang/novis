A `try` needs at least one `catch` clause or a `finally` block. A `try` with neither does not compile:
it is the error `E0242`.

A `catch` clause may name an interface. It then matches every class that implements that interface.

A clause for a parent class that is written before a clause for its subclass catches that error
first. The later clause never runs. This compiles without a warning.

`catch (LogicError | IOError $e)` does not compile: it is the error `E0245`. Write two clauses.

The variable of a `catch` clause is a variable of the whole function. Two clauses in one function may
use the same name when they catch the same type. When the types are different, the second clause does
not compile: that is the error `E0406`.

A `catch` clause may throw as well. The other clauses of the same `try` do not catch the new error.
The `finally` block of that `try` runs first, and then the next `try` outside handles the error. This
is how one layer reports an error of its own and keeps the caught one as its `previous`.

When one `try` is inside another, the inner `finally` block runs before the outer one.

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
