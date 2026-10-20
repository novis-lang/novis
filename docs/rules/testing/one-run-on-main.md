A run on `main` that a newer run of the same workflow replaces is cancelled, so `CI` and `Website`
each have one run going on `main` at a time. `bun nv ci --push` cancels every run still going on an
older commit once its push is through, and `bun nv ci --watch` cancels, on each poll, every run going
beside a newer one of its workflow, such as a push run the nightly deep lane started after. A pull
request's runs are cancelled by the workflow's own concurrency group.

The newest run tests every commit the cancelled one did, and the loop pushes only at a goal's end and
after a fix, so a run is seldom cut short. What a cancel spends: the cancelled commit has no verdict of
its own, so a bisect between two pushes is local, and a cancelled job saves no cache, so the next run
of that job starts from the cache before it. A nightly deep run that a push cuts short leaves that
night without Miri and the fuzzers.

The concurrency group is keyed by event as well, so a push run never waits behind a nightly: `--push`
and `--watch` cancel the older of the two instead.

**Declined:** `cancel-in-progress` for `main` in `ci.yml`. The group is keyed by event, so it would
never cancel a push run beside a nightly, and a group without the event would queue each push behind a
nightly's two hours.
