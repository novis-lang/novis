Runs one step of your work inside the transaction you already have open.

`Core\Db\Transaction::transaction` is the same method a connection has, and it works the same way: it
takes a function, runs it, and returns what that function returned. What is different is where it
runs. A transaction opened inside another transaction is a savepoint, so the inner one has an end of
its own. Giving up on it undoes only what it wrote, and the transaction around it can still save its
own work.

**In plain words:** the inner transaction is one step of a bigger job. A step that goes wrong can be
dropped without dropping the job.

This is what lets a function open a transaction for its own writes without knowing whether the
caller has one open already. Both sides write the same code, and everything either of them wrote is
saved together at the end.

An isolation level and `readOnly` are settled for the whole transaction, so a step that asks for
either throws `LogicError`. Ask for them on the outermost call instead.

The examples undo one step and keep the rest, let a function open its own transaction, and keep an
order whose coupon cannot be used.
