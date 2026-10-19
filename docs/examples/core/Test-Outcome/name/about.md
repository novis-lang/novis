`Outcome::name()` returns the name of the test method that ran. For a test method
`itAddsAnItem`, the result is `itAddsAnItem`.

You get an `Outcome` as the parameter of a method with `#[AfterEachTest]`. The test runner creates it
after each run of each test. You cannot create one yourself.

A test with `#[TestWith]` rows runs once per row. Each run has the row number after the name, the
same way the test report prints it: `itRoundsThePrice#0`, `itRoundsThePrice#1` and so on.

**The examples below** show a file named after the test, the name of a test with rows, and a file
that lists the tests that ran.
