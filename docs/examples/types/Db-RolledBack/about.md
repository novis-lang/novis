The error that is thrown when a program gives up on a transaction it started.

`Core\Db\Connection::runTransaction` runs a function. If the function returns, the work is saved. If
it throws, the work is undone.

Code inside the function calls `rollBack` when the work should not be saved. Examples are a basket
whose total is wrong, or a stock count that would go below zero. `rollBack` records the reason and
throws this error with it. The `runTransaction` call throws the error again, even when code inside the
function caught it.

So the code that called `runTransaction` always learns two things: nothing was written, and why. The
reason is a sentence your program wrote. You read it as the `reason` property or as the error's
message, so a log line and an `if` see the same words.

**Good to know:** this error is your program's own decision. When the database rejects a statement,
the error is `Core\Db\DbError`. Neither class extends the other, so a `catch` for one never catches
the other.
