Every run of `ci.yml` runs every job, whatever its event and whatever its diff touched.

A push to `main`, a pull request, the nightly, a manual dispatch and a release's call of the workflow
run the same jobs, and no job carries an `if:` that reads the diff. One workflow file holds every job,
and a job is never defined twice: a second copy of a suite drifts, and nothing compares the copies.

**The nightly runs the same jobs**, to catch what changes outside the tree on a day with no push: a new
advisory, a new nightly toolchain, a download that moved. The fuzz corpus is cached between runs, so
each run's fuzzing starts from everything the earlier runs found. Miri, the fuzz smoke runs, the
unsafe audit, the `valgrind` leak check over every `examples/` program and the comparison's `flood`
row in its smoke form run in every run. The `valgrind` job is the one run that looks for leaks from end
to end, since the `asan` job runs with leak detection off. The unsafe audit's counts and the `flood`
row's results gate nothing.

**A green run of a commit on `main` is the release's proof.** `bun nv release --tested` accepts a green
`ci.yml` run of that commit from a push, the nightly or a dispatch, and `release.yml`'s `test` job then
skips beside it. A pull request's run does not count: it tests a merge, not the commit.

**Every run builds the release binaries** through `build.yml`, with no version, and keeps the archives
as its artifacts, so a tester can take any commit's binaries from its run. Its `release-version` job
runs the release's version step on the next patch version in its own working copy and writes nothing
back. A green run of a push to `main` publishes the `dev` image through `image.yml`, from that run's
Linux archives, so the image never names a commit CI failed. A newer push to `main` cancels the older
push's run still going (`rule:testing/one-run-on-main`), and the newer run tests every commit the
cancelled one did.

What it spends: about 1,030 to 1,280 runner minutes per run, where a push run that skipped the jobs its
diff could not reach took about 620. The extra jobs run on Linux. A run may wait behind GitHub's limit
of 60 concurrent jobs. It spends nothing from security, latency or the simplicity of the language, and
no table of paths has to be kept in step with what each job reads.

`tools/nv/test/ci-push-builds.test.ts` guards the build, the version step and the `dev` image.

**Declined:**
- **Path lanes, which run only the jobs a diff reaches.** A lane table that missed an input let a red
  commit through, and a green push run that skipped jobs did not prove a release.
- **A separate workflow for the development build.** It is a second workflow and a second trigger for
  the same build `ci.yml` already runs.
- **A workflow-level path filter.** A workflow it filters out never reports, so a required status check
  on it waits forever.
