`Summary::passed()` returns how many tests of the class passed on their first run.

You get a `Summary` as the parameter of a `public static` method with `#[AfterClass]`. The test runner
creates it after the last test of the class. You cannot create a `Summary` yourself.

A test that failed first and then passed on a retry is not counted here. `flaky()` counts it. A test
with `#[TestWith]` rows counts once for each row.

**The examples below** show a count of passed tests, a check that every test passed, and a line for
a status page.
