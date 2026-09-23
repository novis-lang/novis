The database where this deployment stores its background jobs.

Novis keeps queued jobs in an ordinary database table. This setting names the connection for that
table, which is one of the database blocks in the same file. There is no default. If the file has a
`[queue]` block without this setting, the server does not start.

**In plain words:** use the application's own database. Then a job is added in the same transaction
as the work that caused it. If that transaction is rolled back, the job is never created.

A program cannot change this setting. The server applies a new value while it runs. It first checks
the new database, the same way it does when it starts. Workers on the old connection put their
current job back and stop, and new workers take jobs from the new connection.

The example prints where this deployment stores its jobs. Then it tries to change that, and
`Core\Config::set` returns `false`.
