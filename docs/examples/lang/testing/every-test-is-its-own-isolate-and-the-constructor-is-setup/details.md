Each row of a `#[TestWith]` test runs on its own too. A `static` value one row writes is back at
its declared value in the next row.

The attempts of one test share their `static` values. With `retries:`, a counter that the first
attempt raised still has that value in the second attempt.

A constructor that throws an error makes the test fail, with the error's message.

A fixture is built outside the tests. A `static` value that a fixture method writes is back at its
declared value in every test.

A test that starts a script with `spawn script` and does not `await` it fails. The message says the
test left a task still running. `Core\Task::all` and `Core\Task::map` wait for every task they start,
so a test that uses them does not fail this way.
