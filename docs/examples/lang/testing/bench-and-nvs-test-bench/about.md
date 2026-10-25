`#[Bench]` marks a method as a bench. A bench measures how much work a piece of code does.
`nvs test --bench main.nvs` runs every bench in the program, one at a time. It runs no test, and a
plain `nvs test` runs no bench.

A bench method has the same shape as a `#[Test]` method. Write `use Core\Test\Bench;` at the top of
the file, and `#[Bench]` above the method.

The report prints a `count:` line under each bench. It counts statements, calls, allocations,
allocated bytes and runs of the garbage collector. These counts are the same on every run and on
every computer. The line also prints the time the bench took, marked `(advisory)`, because that
time changes on every run.

A bench passes without an assertion. With `--format json`, a build server can compare the counts
with the last run.

**Good to know:** a new version of Novis can give the same program different counts.

The examples show a bench beside a test, two ways to write the same work, and benches that a build
server checks.
