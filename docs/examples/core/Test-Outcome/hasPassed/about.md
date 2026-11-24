`Outcome::passed()` checks whether this run of the test passed. The result is `true` when it
passed, and `false` when it failed.

You get an `Outcome` as the parameter of a method with `#[AfterEachTest]`. That method runs after each
test, also when the test failed. You cannot create an `Outcome` yourself.

A common use is to clean up after a test that passed, and to keep files of a test that failed. You
can then open those files and see what went wrong.

**Good to know:** a test with `retries:` runs again after a failed run. The result is about one run,
so the first run can be `false` and the second `true`.

**The examples below** show a file kept only for a failed test, a status line for each test, and a
report line for a build server.
