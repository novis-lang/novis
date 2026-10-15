`nvs test` runs test isolates in parallel across every core, as many at once as the runner chooses,
with no setting. Every test already runs in an isolate of its own (`rule:testing/isolate-per-test`), so
running them at once is a scheduling question and not an isolation one. The runner places each test
isolate with the placement a program uses (`rule:concurrency/on-worker-runs-the-child-on-another-core`).

The runner guarantees order and nothing about data. The order it guarantees is the one
`rule:testing/class-hooks` gives: `#[BeforeClass]` returns before the class's first test starts,
the class's fixtures are built before its first test, and `#[AfterClass]` starts after its last test has
finished. Tests of one class, and tests of different classes, run at the same time. What two of them
share outside their isolates, a database or a file, is the program's to keep apart.

The report is in declaration order, whatever order the tests finish in (`rule:testing/runner-is-strict`),
so a parallel run and a run on one core report the same thing. Coverage counts each test into a table
of its own and adds it to the run's when the test finishes, so the lines a test reached are still that
test's.

- **Memory:** each test in flight holds its isolate, its fixture copies and its own coverage table when
  coverage is on, all released when it finishes. That is O(cores), the most tests running at once, and
  never O(tests in the suite).

**Not on disk.** The runner makes and finishes the isolates one at a time
(`crates/nvs-cli/src/runner.rs:781`), and the coverage counts of a test are the growth of one shared
table while it runs (`crates/nvs-cli/src/runner.rs:378`).

**Declined:**
- **A setting for the number of tests at once.** The runner fills the cores it has. A program that
  needs two tests apart keeps them apart in what they share.
- **A runner that keeps two classes' data apart.** It would have to know what a test touches outside
  its isolate, and it cannot.
