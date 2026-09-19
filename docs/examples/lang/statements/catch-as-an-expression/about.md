`catch` written after an expression gives that expression a value to fall back on when it throws an
error. It saves writing a whole `try` block for one value.

Write the expression, then `catch (SomeError $e) => fallback`. If the expression runs normally, its
own value is the result. If it throws an error the arm matches, the arm's value is the result. You
may write several arms after one expression; they are tried in order, and the first class that
matches wins. An error no arm matches keeps travelling, as it would without the `catch`. An arm
holds an expression, so `throw` is allowed there and `return` is not.

**Good to know:** `catch (Throwable) => value` with no variable and no `throw` gives a warning. It
hides every error, including the ones you never thought about.

**The examples below** show a fallback for a row that is missing, then several arms tried in order,
then a report line whose division is guarded and a low-level error replaced by a clear one.
