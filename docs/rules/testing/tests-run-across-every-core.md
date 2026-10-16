`nvs test` runs test isolates in parallel across every core, as many at once as `--jobs` allows, and
`--jobs` defaults to the machine's hardware threads as it already does for `.nvst` cases. Every test
already runs in an isolate of its own (`rule:testing/isolate-per-test`), so running them at once is a
scheduling question and not an isolation one. The runner places each test isolate with the placement a
program uses (`rule:concurrency/on-worker-runs-the-child-on-another-core`). `--jobs 1` runs every test
on the runner's own core, one at a time, and so does a program that loads an extension: the extension
host belongs to the thread that loaded it.

The runner guarantees order and nothing about data. The order it guarantees is the one
`rule:testing/class-hooks` gives: `#[BeforeClass]` returns before the class's first test starts,
the class's fixtures are built before its first test, and `#[AfterClass]` starts after its last test has
finished. Tests of one class, and tests of different classes, run at the same time. What two of them
share outside their isolates, a database or a file, is the program's to keep apart.

The report is in declaration order, whatever order the tests finish in (`rule:testing/runner-is-strict`),
so a parallel run and a run on one core report the same thing. Coverage counts each test into a table
of its own and adds it to the run's when the test finishes, so the lines a test reached are still that
test's.

**A test's output is captured.** What a test, its hooks and its fixtures print while it runs is held for
that test. A test that passes discards it, and a test that fails prints it under its failure, in the
report's place for that test. What a class's `#[BeforeClass]` and fixtures print is held too, and
printed under the class's header whatever its tests did. `--show-output` turns capture off: output then
goes to the terminal as it is printed and is not attributed, so two tests running at once may mix their
lines. A test that reads the terminal reads end of input, so no test stops the run to wait for a person.

- **Memory:** each test in flight holds a parent context of its own, its isolate, its fixtures as bytes
  and as copies, its captured output and its own coverage table when coverage is on, all released when
  it is reported. A failed test's output is held until its report line is printed. That is O(`--jobs`),
  the most tests running at once, plus the results of finished tests waiting for an earlier one, and
  never the output of a test that passed.

**Declined:**
- **A second setting for the number of tests at once.** `--jobs` already says how many `.nvst` cases run
  at once, and a `#[Test]` run reads the same flag.
- **A runner that keeps two classes' data apart.** It would have to know what a test touches outside
  its isolate, and it cannot.
- **Output streamed by default.** Two tests running at once mix their lines, and a passing test's
  output is noise in every run.
