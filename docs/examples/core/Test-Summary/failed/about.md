`Summary::failed()` returns how many tests of the class failed. When every test passed, the result is
`0`.

You get a `Summary` as the parameter of a `public static` method with `#[AfterClass]`. The test runner
creates it after the last test of the class. That method also runs when a test failed. You cannot
create a `Summary` yourself.

A test with `retries:` counts as failed only when its last run failed too. A test that ended the
program with `exit()` also counts as failed.

**The examples below** show a warning when a test failed, a status file for a build server, and the
subject line of a report email.
