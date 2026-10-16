Returns how many parameters a method declares.

`parameterCount()` returns a `uint`. `$this` is not counted, so a method written as
`total(int $net, int $tax)` returns `2`. A method with no parameters returns `0`.

`Core\Reflect\ClassInfo::call` needs at least this many arguments. With fewer, it throws a
`LogicError` and the method does not run. Extra arguments are ignored.

The number is also correct for a method the compiler adds, such as the constructor of
`LogicError`. For those methods `parameters()` returns an empty array, and `parameterCount()` is
the way to know what the method takes.

**The examples below** count the parameters of each method, show the count for an error class,
and run only the tasks that need no arguments.
