`Summary::skipped()` returns how many tests of the class were skipped. A test is skipped when its
`#[Test]` attribute has `skip:` with a reason.

You get a `Summary` as the parameter of a `public static` method with `#[AfterClass]`. The test runner
creates it after the last test of the class. You cannot create a `Summary` yourself.

A skipped test does not run, so it is not counted in `passed()`, `failed()` or `flaky()`. Add the four
counts to get the number of tests in the class.

**The examples below** show a count of skipped tests, a reminder before a release, and the total
number of tests in a class.
