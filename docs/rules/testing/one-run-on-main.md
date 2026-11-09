A run on `main` that a newer run of the same workflow replaces is cancelled, so `CI` and `Website`
each have one run going on `main` at a time. `ci.yml`'s concurrency group cancels an older push's run
when a newer push arrives, as it does for a pull request's. `bun nv ci --push` cancels every run still
going on an older commit once its push is through, and `bun nv ci --watch` cancels, on each poll, every
run going beside a newer one of its workflow, such as a push run the nightly started after.
`bun nv ci --dispatch dry-run` and `--dispatch release` cancel the push runs of `ci.yml` still going on
the commit their release run tests, since that run's full suite and release build cover them, and never
a run on another commit, a scheduled or dispatched run, or a release run.

The newest run tests every commit the cancelled one did, because every run runs every job
(`rule:testing/every-run-runs-every-job`). What a cancel spends: the cancelled commit has no verdict of
its own, so a bisect between two green runs is local, and a cancelled job saves no cache, so the next
run of that job starts from the cache before it. A nightly run that a push cuts short leaves that night
without its run on an unchanged tree. A commit whose push run was cancelled gets no `dev` image: the
next green push run publishes it.

The concurrency group is keyed by event as well, so a push run never waits behind a nightly, and the
group never cancels a scheduled or dispatched run: `--push` and `--watch` cancel the older of a push
and a nightly instead.

**Declined:** one concurrency group for every event of `ci.yml`. A push would queue behind a nightly's
two hours, or a nightly would be cancelled by every push.
