`#[Test(...)]` takes options that change how one test runs. Every option is optional, and a test that
needs none is written as plain `#[Test]`.

`skip:` states a reason, and the test is reported as skipped without being run. `at:` fixes the clock
at one instant, so a test of dates and times reads the same time on every run. `Core\Test::advance`
moves that fixed clock forward. `seed:` fixes the numbers `Core\Random` draws, so the test draws the
same sequence on every run. `retries:` allows more attempts after a failure, and needs `because:`
beside it to state why. A test that fails and then passes is reported as flaky, never as passed.

**Good to know:** an option name the compiler does not know does not compile, so a mistyped option is
found while you build.

**The examples below** show a skipped test, a fixed clock, and a test class that uses several options
together.
