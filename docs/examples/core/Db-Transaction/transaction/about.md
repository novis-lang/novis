Runs a function as one step inside a transaction that is already open.

`Core\Db\Transaction::transaction` works like the method with the same name on a connection. It
takes a function, runs it, and returns what the function returned. The inner transaction is a
savepoint. When you call `rollBack` on it, only its own changes are undone. The outer transaction
continues and can still save its own work. When nothing is undone, all changes are saved together
when the outer transaction ends.

**In plain words:** the inner transaction is one step of a bigger job. When one step fails, you
can undo that step and keep the job.

This is useful for a function that needs a transaction for its own writes. The function can open
one without checking whether its caller already has one open.

**Good to know:** the isolation level and `readOnly` are set for the whole transaction. An inner
call that sets one of them throws a `LogicError`. Set them on the outermost call.

**The examples below** undo one step and keep the others, let a function open its own transaction,
and keep an order when its coupon cannot be used.
