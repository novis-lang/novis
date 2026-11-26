With `[debug] store` on, a scheduled job, a queue job and a command run are each recorded as one `run`
line, and its events, dumps and logs carry its trace id, so the viewer shows a run as it shows a
request.

A `run` line has `of` (`schedule`, `queue` or `command`), `name`, `start_us`, `duration_us` and
`outcome` (`ok`, `failed` or `cancelled`), and optional `memory_peak`, `memory_limit` and
`exception`, whose shape is the request's
(`rule:testing/a-recorded-exception-keeps-its-frames-and-never-a-value`). Its `name` is the
schedule's name, the queue job's script, or the command (`run` or `test`) and its script. A run
draws a trace id where it starts, as a request does, so every line it writes joins it. The field
table is `rule:testing/the-debug-stream-reaches-the-service-through-the-disk`'s.

**Where each kind of run writes it.** Each is a point where the runtime already knows the run's
outcome, so no new branch runs while `store` is off:

- a scheduled run, where its outcome already counts in `nvs_schedule_runs_total`
  (`crates/nvs-server/src/metrics.rs`);
- a queue job, where the worker settles an attempt as succeeded, failed or dead-lettered
  (`crates/nvs-cli/src/worker.rs`), one line per attempt;
- a command run, where `nvs run` and `nvs test` seal their spool at exit
  (`crates/nvs-cli/src/main.rs`, `sealed`). A run stopped by a signal is `cancelled`.

**The Runs view.** The viewer has a Runs view beside Requests: the same list, filters and paging,
and the same page with its tabs, without the Request and Response tabs, because a run has neither.
The index keeps runs in a `runs` table of their own
(`rule:testing/the-debug-index-holds-what-the-viewer-reads-for-a-bounded-time`).

- **Memory:** nothing while `store` is off. With it on, one `run` line built per run, where today
  that run writes none.

**Declined:**
- **A run written as a `request` line with a made-up method.** Every list, filter, route view and
  count would then need to know the fake method and leave it out, and a run has no status, path or
  route to fill the request's required fields with.
- **A `run` line for each scheduled tick that does not start a job.** It records no work, and a
  schedule that ticks every second would fill the spool cap with nothing.
