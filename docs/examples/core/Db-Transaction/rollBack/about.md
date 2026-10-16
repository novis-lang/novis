Gives up on a transaction. Nothing the transaction wrote is saved, and the reason you give reaches
whoever catches the failure.

`Core\Db\Transaction::rollBack` takes one argument, the reason, and throws `Core\Db\RolledBack`. The
reason becomes that error's `reason` property and its message, so write it for the person who reads
the failure rather than for the program. Nothing after the call runs, and none of the statements the
transaction ran are kept.

Because it throws, there is no state to remember and no flag for a later layer to check. A function
the work calls can give up on the whole transaction by itself, and the code between that function and
`Core\Db\Connection::transaction` needs to pass nothing back up.

Catch `Core\Db\RolledBack` around the call to `Core\Db\Connection::transaction` to handle it. Catching
it inside the work does not save anything: the transaction is already given up by then.

The examples give up when a check fails, give up from inside a function, and move money between two
accounts only when the first one can pay.
