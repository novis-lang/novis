Counts the jobs of one queue that were pushed inside a transaction of another database.

`Core\Queue::stats` returns a `Core\Queue\Stats`, and `nonTransactional()` is one of its numbers.
`Core\Queue` saves its jobs in the database that `[queue] connection` names. Your program can push a
job while a transaction is open on a different database. That job is saved at once, and it is not
part of the transaction. If the transaction is rolled back, the job stays in the queue.

`nonTransactional()` counts these jobs in the jobs table. A push outside any transaction is not
counted. A push inside a transaction on the queue's own database is not counted either, because a
rollback deletes that job too. Jobs in the dead-letter table (the table of jobs that failed too often)
are not counted.

A number above `0` means that some jobs may belong to work that was rolled back. To keep a job and its
data together, push the job inside a transaction on the queue's database.

**The examples below** count a push inside another database's transaction, show that a rollback keeps
the job, and warn about it in a health check.
