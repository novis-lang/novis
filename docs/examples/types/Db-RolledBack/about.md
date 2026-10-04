The error raised when a program gives up on a transaction it started.

A transaction here runs a callable. If the callable returns, the work is kept. If it throws, the
work is undone.
`rollBack` is how the code inside says the work should not stand — a basket that no longer adds up, a
stock count that would go negative, a check only the last step could make. It records the reason,
throws this error carrying it, and the error leaves the call that owned the transaction whether or
not anything in between caught it.

So the caller outside always learns both halves: that nothing was written, and why. The reason is a
sentence the program chose, and it is readable as `reason` and as the error's message, so a log line
and a branch read the same words.

**Good to know:** this is the program's own decision. A refusal the database made is
`Core\Db\DbError`, which sits beside this one rather than above it, so a `catch` written for either
one never quietly takes the other.
