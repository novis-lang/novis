Gives up on a transaction. Nothing the transaction wrote is saved, and the reason you give reaches
whoever catches the failure.

`Core\Db\Transaction::rollBack` takes one argument, the reason, and throws `Core\Db\RolledBack`. The
reason becomes that error's `reason` property and its message, so write it for the person who reads
the error. Nothing after the call runs, and none of the statements the transaction ran are kept.

Because the method throws, no other code has to check whether the transaction was given up. A
function that your work calls can give up on the whole transaction by itself. The functions between
it and `Core\Db\Connection::runTransaction` do not need to return anything for this.

Catch `Core\Db\RolledBack` around the call to `Core\Db\Connection::runTransaction` to handle it. Catching
it inside the work does not save anything: the transaction is already given up by then.

The examples give up when a check fails, and give up from inside a function. The last one moves money
between two accounts only when the first one can pay.
