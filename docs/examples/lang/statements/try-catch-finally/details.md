A `try` needs at least one `catch` clause or a `finally` block. A `try` with neither does not compile:
it is the error `E0242`.

A `catch` clause may name an interface. It then matches every class that implements that interface.

A clause for a parent class that is written before a clause for its subclass catches that error
first. The later clause never runs. This compiles without a warning.

`catch (LogicError | IOError $e)` does not compile: it is the error `E0245`.

The variable of a `catch` clause is a variable of the whole function. Two clauses in one function may
use the same name when they catch the same type. When the types are different, the second clause does
not compile: that is the error `E0406`.

When a `catch` clause throws, the other clauses of the same `try` do not catch the new error. The
`finally` block of that `try` runs first. Then the next `try` outside handles the error.

When one `try` is inside another, the inner `finally` block runs before the outer one.
