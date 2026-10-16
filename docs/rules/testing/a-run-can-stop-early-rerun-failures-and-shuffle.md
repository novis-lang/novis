`nvs test --fail-fast` stops starting tests after the first failure, `--rerun-failed` runs only the
tests the last run failed, and `--shuffle` starts tests in a random order and prints the seed.

**`--fail-fast`.** After the first test fails, no new test starts. Tests already running finish and
are reported, and the summary counts the tests that never started as not run. A flaky test is not a
failure (`rule:testing/runner-is-strict`), so it does not stop the run. The exit status is the failed
run's.

**`--rerun-failed`.** Every run of a program writes the names of its failed tests to a small file in the
data folder, keyed by the program's entry path. `--rerun-failed` runs exactly those tests, and a later
run overwrites the file. When the last run failed nothing, or no run was recorded, it runs the whole
selection and says so in its first line. It combines with `--filter` and the tag flags
(`rule:testing/tags-and-shards-select-what-runs`), which narrow it further.

**`--shuffle`.** Tests start in an order drawn from a seed, and the summary prints the seed.
`--shuffle=<seed>` repeats that order. Report order stays declaration order
(`rule:testing/runner-is-strict`), so a shuffled run and a plain one print the same report when both
pass. Every test runs in its own isolate (`rule:testing/isolate-per-test`), so a random order finds
nothing inside the program. It finds what two tests share outside it, a database or a file, which
`rule:testing/tests-run-across-every-core` leaves to the program to keep apart. `--shuffle` is off by
default.

- **Memory:** the failed-test file is one line per failed test, on disk. Nothing per test in flight.

**Not on disk.** `nvs test` has none of the three flags (`crates/nvs-cli/src/main.rs:574`).

**Declined:**
- **Shuffling by default.** A run that changes order on its own makes a shared-state failure look like
  a flaky test, and a developer who did not ask for it has no seed in mind to repeat it.
- **The failed-test file in the project folder.** It would appear in every working tree and in
  `git status`, for a fact about one machine's last run.
- **A `--fail-fast` that kills tests already running.** Their result is already being paid for, and a
  killed test would report nothing useful.
