A `deadline` bounds when cancellation is **requested**, not when the call returns.

A child blocked mid-statement cannot simply be abandoned: a database connection with an unread
result set is unusable, so the statement is cancelled through the driver's own mechanism and the
call waits until that connection is back in a known state. That drain is bounded in turn by the
connection's own `timeout`, after which the connection is **closed rather than returned to the
pool**.

So `deadline: 2s` can return at two seconds plus one connection timeout in the pathological case.
Overrunning a stated deadline by a bounded amount is the correct trade against handing a poisoned
connection back to a pool, where it would fail some later, unrelated request. A deadline is a bound
on when the work stops being asked for; it is not a hard wall-clock guarantee on return.
