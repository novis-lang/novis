`Summary::failedTests()` returns the names of the tests of the class that failed, as an array of
strings. When no test failed, the array is empty.

Each name is the name of the test method, the same way the test report prints it. A test with
`#[TestWith]` rows has the row number after the name, such as `itRoundsThePrice#1`.

You get a `Summary` as the parameter of a `public static` method with `#[AfterClass]`. The test runner
creates it after the last test of the class. You cannot create a `Summary` yourself.

**The examples below** show the failed tests one per line, a list for a chat message, and a file that
the next run can read.
