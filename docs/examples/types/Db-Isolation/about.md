How much of other transactions' unfinished work your own transaction is allowed to see.

A database runs many transactions at once, and the isolation level is the promise it makes to yours
about the others while they are still going. The five cases run from `ReadUncommitted`, where a
statement may read a row nobody has committed yet, to `Serializable`, where the result has to match
some order in which the transactions ran one at a time. You pass one as the `isolation` option of
`transaction`, and leaving it out is the usual thing to do: the transaction then runs at whatever
level the database was set up with.

**Good to know:** the cases are a list and not a ranking. `Snapshot` is not simply stronger or weaker
than the levels written either side of it, because backends put it in different places, so pick the
one that describes the work rather than reaching for the next one along. A database that cannot offer
the level you asked for says so, instead of quietly running your work at a weaker one.
