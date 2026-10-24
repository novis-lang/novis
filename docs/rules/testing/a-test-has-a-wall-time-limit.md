Each test runs under a wall-time limit: `[limits] wall_time` applies to every test on its own, and
`#[Test(timeout: "5s")]` replaces it for one test.

The limit is counted from the moment the test's class is constructed to the moment the test method
returns, so a test's fixtures and hooks count toward it and the tests that ran before it do not. A test
that passes its limit is stopped the way a request past its wall time is stopped
(`rule:errors/on-limit`), and it fails with a message that names the limit and where it came from: the
configuration or the attribute. One limit covers every attempt of a test together, since the attempts
share the test's isolate (`rule:testing/runner-is-strict`): a test stopped at its limit is a failure,
and a retry gives it no new time.

`timeout` is written in the duration syntax `nvs.toml` uses, and a value that does not parse does not
compile, and neither does a zero. It may be longer or shorter than `[limits] wall_time`, and
`[limits.hard]`'s `wall_time` bounds it as it bounds a request's own `Core\Config::set`: a test whose
`timeout` is above that bound fails without running. Where neither is written, a test has no wall-time
limit, which is what `[limits] wall_time` already means when it is absent.

Reusing the request's key is the point: a test of a handler is held to the time the handler gets in
production, and one setting says it for both. The override is per test because a slow test is a fact
about that test, not about the run.

- **Memory:** one deadline per test in flight, the same one a request carries, and the thread `nvs run`
  times its own wall time with, for each test in flight that has a limit.

**Declined:**
- **A run-wide `--timeout` flag.** It is a second setting for what `[limits] wall_time` already says,
  and it would hold a fast test and a slow one to one number.
- **A default limit for tests when none is written.** A test that is meant to be slow would fail on a
  number nobody chose, and `nvs run` has no default either.
