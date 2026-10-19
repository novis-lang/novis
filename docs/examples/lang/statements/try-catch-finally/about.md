`try` runs a block of code and handles the errors it raises. Write `try { … }`, then `catch`
clauses, a `finally` block, or both.

A `catch` clause names one class. It matches that class and every class that extends it. The clauses
are tried from top to bottom, and the first one that matches runs. Write the narrow classes first.
A clause names one class only, so two classes need two clauses. Each clause declares its own
variable. Two clauses in one function that catch different classes need two different names. You may leave the variable out
when you do not need the error itself.

`finally` always runs. It runs after the body, after a `catch`, and on the way out through `return`,
`break`, `continue` or an error nothing caught. Put your clean-up work there.

**The examples below** show the block form around a call that fails, then two `catch` clauses picked
by class, then a `finally` that closes a job whatever happens. The last two show two clauses with
one variable name, which does not compile, and a `finally` that runs on `continue`.
