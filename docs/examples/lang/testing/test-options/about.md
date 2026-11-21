`#[Test(...)]` takes options that change how one test runs. Every option is optional. A test that
needs none is plain `#[Test]`.

`skip:` gives a reason, and the test is reported as skipped and does not run. `at:` sets the clock
to one time that does not change, so every run reads the same time. `Core\Test::advance` moves that
clock forward. `seed:` makes `Core\Random` give the same numbers on every run. `retries:` allows
more attempts after a failure, and needs `because:` to say why. A test that passes only on a retry
is reported as flaky. It does not count as passed.

`timeout:` sets a time limit for one test. `tags:` labels a test, and `nvs test --tag` runs only
the tests with that label. `env:` sets environment variables for the test, such as
`['API_KEY' => 'test']`. The requests the test sends with `Core\Test::request` and the programs it
runs with `Core\Test::run` get them too.

**Good to know:** an option with a wrong name does not compile.

**The examples below** show a skipped test, a fixed clock, several options together, a flaky test,
retries that need a reason, a time limit, and tags.
