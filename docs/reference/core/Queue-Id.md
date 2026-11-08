---
summary: the id of one background job. `Core\Queue::push` returns it, and `status`, `cancel` and `delete` take it
keywords: job id, queue job, background job, job state, job status, cancel a job, delete a job, track a job, duplicate job
---

`Core\Queue\Id` identifies one background job and the queue that the job is in. `Core\Queue::push`
returns it. You pass it to `Core\Queue::status`, `Core\Queue::cancel` and `Core\Queue::delete`. An
`Id` has no methods, and it cannot be printed or converted to a `string`.

When you call `push` with a `key`, and a pending job already has that key, `push` adds no job. It
returns an `Id` for the pending job, so `status`, `cancel` and `delete` work on that job.

`status` reads the state of the job from the database on every call. `cancel` returns `false` when
the job is no longer waiting. After `delete` removes the job, `status` throws a `RuntimeError` for its
`Id`.

The example below creates the two queue tables itself, because its database is in memory. On a
server, `nvs queue migrate` creates them.

```toml file=nvs.toml
[db.jobs]
driver = "sqlite"
path = ":memory:"

[capabilities.db]
connect = ["jobs"]

[queue]
connection = "jobs"
workers = 0
```
```nvs
<?nvs
use Core\Queue;

var $db = Core\Db::connect('jobs');
$db->execute(<<<'SQL'
create table nvs_jobs (id integer primary key autoincrement, queue varchar(255) not null,
    script text not null, args text, state smallint not null, attempts integer not null,
    max_attempts integer not null, backoff_ms bigint not null, run_at bigint not null,
    tag varchar(255), dedupe_key varchar(255), dedupe_pending varchar(255),
    created_at bigint not null, claimed_at bigint, errors text, grants text, limits text,
    non_transactional smallint, token text, constraint nvs_jobs_dedupe unique (dedupe_pending))
SQL, []);
$db->execute(<<<'SQL'
create table nvs_dead_jobs (id bigint not null, queue varchar(255) not null,
    script text not null, args text, attempts integer not null, max_attempts integer not null,
    backoff_ms bigint not null, run_at bigint not null, tag varchar(255),
    dedupe_key varchar(255), created_at bigint not null, failed_at bigint not null,
    errors text not null, token text, primary key (id))
SQL, []);

Core\Queue\Id $first = Queue::push('jobs/send-invoice.nvs', { args: ['order' => 7], key: 'invoice-7' });
echo Queue::status($first) == Queue::State::Pending ? 'pending' : 'not pending', "\n";

// The key is the same, so this returns an `Id` for the first job.
Core\Queue\Id $again = Queue::push('jobs/send-invoice.nvs', { args: ['order' => 7], key: 'invoice-7' });
echo Queue::cancel($again) ? 'cancelled' : 'not cancelled', "\n";
echo Queue::status($first) == Queue::State::Cancelled ? 'cancelled' : 'not cancelled', "\n";

// The job is not waiting any more, so `cancel` returns `false`.
echo Queue::cancel($first) ? 'cancelled' : 'not cancelled', "\n";
```
```output
pending
cancelled
cancelled
not cancelled
```
