A `try` needs at least one `catch` clause or a `finally` block. A `try` with neither does not compile:
it is the error `E0242`.

A `catch` clause may name an interface. It then matches every class that implements that interface.

A clause for a parent class that is written before a clause for its subclass catches that error
first. The later clause never runs. This compiles without a warning.

`catch (LogicError | IOError $e)` does not compile: it is the error `E0245`. Two clauses that declare
the same variable in one function do not compile either: that is the error `E0406`.

When a `catch` clause throws, the other clauses of the same `try` do not catch the new error. The
`finally` block of that `try` runs first. Then the next `try` outside handles the error.

When one `try` is inside another, the inner `finally` block runs before the outer one.
