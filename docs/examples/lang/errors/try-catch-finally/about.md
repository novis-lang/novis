`try` guards a block of code that can throw. Each `catch` clause beside it names one class from the
throwable tree, and `finally` runs whatever the outcome is.

A clause matches its own class and every class under it, so `catch (RuntimeError $e)` also takes an
`IOError` and a `TimeoutError`, and `catch (Throwable $e)` takes everything. The clauses are tried
from top to bottom, so the narrow classes go first.

A `catch` clause may throw as well. What it throws leaves the whole `try`: the clauses beside it
never see it, and the next `try` further out handles it. This is how one layer reports an error of
its own and keeps the caught one as the cause.

`finally` runs on every way out of the block, and nested `finally` blocks run from the innermost one
outwards.

**The examples below** show two clauses picked by class, a `catch` that throws a new error with a
cause, and the clean-up around a job that reads rows.
