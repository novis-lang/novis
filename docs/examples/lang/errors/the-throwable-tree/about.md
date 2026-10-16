Every error in Novis is an object of a class under `Throwable`, and the whole tree is short. There
is no `Exception` and no `Error`.

`LogicError` means a bug in your own program, such as a `match` with no arm for the value it was
given. `RuntimeError` means the world said no: a missing array key, a value that cannot be
converted, a file you are not allowed to read. `IOError`, `ParseError`, `TimeoutError` and
`RecursionError` are kinds of `RuntimeError`. `ArithmeticError` is its own branch, for division by
zero and for a number that does not fit.

A `catch` arm also catches every class under the one it names, so `catch (Throwable $e)` catches
everything.

**The examples below** show which class each common failure throws, how one arm takes a whole
branch, and a job that treats a bug and a bad input differently.
