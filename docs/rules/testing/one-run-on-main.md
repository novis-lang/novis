A push run on `main` that a newer run of the same workflow replaces is cancelled, so `CI` and `Website`
each have one push run going on `main` at a time. `ci.yml`'s concurrency group cancels an older push's
run when a newer push arrives, as it does for a pull request's. `bun nv ci --push` cancels every push run
still going on an older commit once its push is through, and `bun nv ci --watch` cancels, on each poll,
every push run going beside a newer one of its workflow, such as a push run the nightly started after.
`bun nv ci --dispatch dry-run` and `--dispatch release` cancel the push runs of `ci.yml` still going on
the commit their release run tests, since that run's full suite and release build cover them, and never
a run on another commit or a release run.

**A scheduled or dispatched run is never cancelled**, by the group or by a command. It runs the jobs a
push run leaves out, miri over every crate and the unsafe audit, so no push run replaces it. A push that
lands while the nightly runs leaves the two side by side, and the nightly's verdict stays its own.

The newest run tests every commit the cancelled push run did, because every push run runs every job of a
push run (`rule:testing/every-run-runs-every-job`). What a cancel spends: the cancelled commit has no
verdict of its own, so a bisect between two green runs is local, and a cancelled job saves no cache, so
the next run of that job starts from the cache before it. A commit whose push run was cancelled gets no
`dev` image: the next green push run publishes it. What a kept nightly spends: runners for a push run and
a nightly at once, for as long as both go.

The concurrency group is keyed by event as well, so a push run never waits behind a nightly, and the
group never cancels a scheduled or dispatched run.

**Declined:**

- One concurrency group for every event of `ci.yml`. A push would queue behind a nightly's two hours, or
  a nightly would be cancelled by every push.
- `--push` cancelling a nightly on an older commit. Its miri and unsafe-audit jobs run in no push run, so
  every push during the nightly's two hours left that night with none of them, and the cancelled jobs
  read as red.
