`nvs test --fail-fast` stops starting tests after the first failure, `--rerun-failed` runs only the
tests the last run failed, and `--shuffle` starts tests in a random order and prints the seed.

**`--fail-fast`.** Once a failed test is reported, no new test starts, and no later class's
`#[BeforeClass]` runs. Tests already running finish and are reported, and the class whose test failed
still runs its `#[AfterClass]`. The summary counts the tests that never started as not run. Under
`--jobs 1` the run stops right after the first failure. With more jobs a failure is seen when it is
reported in declaration order, so a few more tests may start first. A flaky test is not a failure
(`rule:testing/runner-is-strict`), so it does not stop the run. The exit status is the failed run's.

**`--rerun-failed`.** Every run of a program writes the `Class::label` of each failed test, a data
row's `#N` included, to `failed-tests/<hash>.txt` in the data folder, where the hash is of the
program's canonical entry path. `--rerun-failed` runs exactly those tests, and a later run overwrites
the file, with an empty one when nothing failed. When the last run failed nothing, or no run was
recorded, it runs the whole selection and says so in its first line. It combines with `--filter` and
the tag flags (`rule:testing/tags-and-shards-select-what-runs`), which narrow it further.

**`--shuffle`.** The tests of each class start in an order drawn from a seed, and the summary prints
the seed. `--shuffle=<seed>` repeats that order: the draw is a SplitMix64 stream, so a seed gives one
order on every platform. Classes keep their order, because a class's `#[BeforeClass]` and
`#[AfterClass]` bracket its tests in the report. Report order stays declaration order
(`rule:testing/runner-is-strict`), so a shuffled run and a plain one print the same report when both
pass. Every test runs in its own isolate (`rule:testing/isolate-per-test`), so a random order finds
nothing inside the program. It finds what two tests share outside it, a database or a file, which
`rule:testing/tests-run-across-every-core` leaves to the program to keep apart. `--shuffle` is off by
default.

Under `--format json` and `--format junit` the documents keep their schema, and the not-run count and
the seed are printed on stderr. A `.nvst` tree refuses all three flags: a case is a process of its
own, judged on its whole output, and has no class or seed to shuffle by.

- **Memory:** the failed-test file is one line per failed test, on disk. A shuffled run holds, per
  class, one slot per selected test and the result of each finished test that waits for one written
  above it to start. A run without `--shuffle` holds nothing more.

**Declined:**
- **Shuffling by default.** A run that changes order on its own makes a shared-state failure look like
  a flaky test, and a developer who did not ask for it has no seed in mind to repeat it.
- **The failed-test file in the project folder.** It would appear in every working tree and in
  `git status`, for a fact about one machine's last run.
- **A `--fail-fast` that kills tests already running.** Their result is already being paid for, and a
  killed test would report nothing useful.
