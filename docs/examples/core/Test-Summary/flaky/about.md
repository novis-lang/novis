`Summary::flaky()` returns how many tests of the class failed first and then passed on a retry. Such
a test is called flaky.

A test runs again after a failure only when its `#[Test]` attribute has `retries:`. A flaky test is
not counted in `passed()` or in `failed()`.

You get a `Summary` as the parameter of a `public static` method with `#[AfterClass]`. The test runner
creates it after the last test of the class. You cannot create a `Summary` yourself.

**In plain words:** a flaky test passes only sometimes. It often depends on time, on the network or on
the order of the tests. It is worth fixing before it starts to fail.

**The examples below** show a count of flaky tests, a warning, and a limit for the number of flaky
tests.
