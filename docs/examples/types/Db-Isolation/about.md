How much of other transactions' unfinished work your own transaction is allowed to see.

A database runs many transactions at the same time. The isolation level says what your transaction
sees of the others while they are still running. The five cases go from `ReadUncommitted` to
`Serializable`. With `ReadUncommitted`, a statement may read a row that nobody has committed yet.
With `Serializable`, the result is the same as if the transactions ran one after another.

You pass a case as the `isolation` option of `runTransaction`. Most programs leave it out. The
transaction then uses the level the database is configured with.

**Good to know:** the order of the cases is not a ranking. Databases do not agree on where
`Snapshot` belongs, so it is not stronger or weaker than the cases next to it. Pick the case that
describes the work you need. A database that does not support the level you choose throws an error.
It does not run your work at a weaker level.
