A run on `main` that a newer run of the same workflow replaces is cancelled, so `CI` and `Website`
each have one run going on `main` at a time. `ci.yml`'s concurrency group cancels an older push's run
when a newer push arrives, as it does for a pull request's. `bun nv ci --push` cancels every run still
going on an older commit once its push is through, and `bun nv ci --watch` cancels, on each poll, every
run going beside a newer one of its workflow, such as a push run the nightly deep lane started after.
`bun nv ci --dispatch dry-run` and `--dispatch release` cancel the push runs of `ci.yml` and
`dev-build.yml` still going on the commit their release run tests, since that run's full suite and
release build cover both, and never a run on another commit, a scheduled or dispatched run, or a
release run.

The newest run tests every commit the cancelled one did, because a push run diffs against the newest
commit whose `CI` run went green, not against the push's `before` (`rule:testing/ci-lanes`). What a
cancel spends: the cancelled commit has no verdict of its own, so a bisect between two green runs is
local, and a cancelled job saves no cache, so the next run of that job starts from the cache before it.
A nightly deep run that a push cuts short leaves that night without Miri and the fuzzers. A commit
whose push runs a release run replaced gets no `dev` image.

The concurrency group is keyed by event as well, so a push run never waits behind a nightly, and the
group never cancels a scheduled or dispatched run: `--push` and `--watch` cancel the older of a push
and a nightly instead.

**Declined:** one concurrency group for every event of `ci.yml`. A push would queue behind a nightly's
two hours, or cancel the nightly's Miri and fuzzers.
