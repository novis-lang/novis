`try` guards a block of code that can throw. Write `try { … }`, then `catch` clauses, a `finally`
block, or both. Each `catch` clause names one class from the throwable tree.

A clause matches its own class and every class under it, so `catch (RuntimeError $e)` also takes an
`IOError`. The clauses are tried from top to bottom, so the narrow classes go first. You may leave
the variable out when you do not need the error itself.

`finally` runs on every way out of the block: after the body, after a `catch`, and on `return`,
`break`, `continue` or an error that nothing caught.

**The examples below** show two clauses picked by class, a `catch` that throws a new error with a
cause, and the clean-up around a job that reads rows. Then they show a loop that carries on after a
row that fails, two clauses with one variable name, which does not compile, and a `finally` that
runs on `continue`.
