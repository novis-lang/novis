`#[BeforeClass]` takes no parameters. `#[AfterClass]` may take one `Core\Test\Summary`, and
`#[AfterEachTest]` one `Core\Test\Outcome`. A hook with another parameter is `E0862`. So is a hook
that returns a value or is not `public`. A class hook must be `static`, and `#[AfterEachTest]` must
not be. Each of those is `E0862` too.

A second hook of one kind in a class is `E0304`.

`#[BeforeClass]` and `#[AfterClass]` may not write a `static` property. That is `E0863`. Each test
reads the declared value, so a fixture is the way to give the tests a value. `#[AfterEachTest]`
runs with the test, and it may write one.

The class hooks run in this order: `#[BeforeClass]`, then the fixtures, then the tests, then
`#[AfterClass]`. They run when every test of the class is skipped. They do not run for a class with
no test, or for a class that `--filter` leaves out.

What a class hook prints with `echo` is part of the report, under the class name. What
`#[AfterEachTest]` prints belongs to the test, so it is shown only when the test fails.

`#[AfterEachTest]` runs after every attempt. It sees the properties the test set on the object.

`Core\Test\Outcome` has `class()`, `name()`, `attempt()`, `passed()` and `failures()`, the messages
of the failed assertions. `Core\Test\Summary` has `class()`, `passed()`, `failed()`, `skipped()`,
`flaky()` and `failedTests()`, the names of the failed tests.

When `#[BeforeClass]` throws an error, each test fails. The message names the hook and gives the
error's message. `#[AfterClass]` still runs.

When `#[AfterClass]` throws an error, the report has a line marked `✗` with the hook's name, and
the exit status is `1`. When `#[AfterEachTest]` throws an error, that attempt fails.

A subclass runs the tests, the fixtures and the hooks of its parent under its own name. The parent
also runs them under its name, unless it is `abstract`. A method that overrides a hook or a test
without its attribute is `E0862` or `E0733`.

There is no `#[BeforeEachTest]`. The constructor of the test class runs before each test.
