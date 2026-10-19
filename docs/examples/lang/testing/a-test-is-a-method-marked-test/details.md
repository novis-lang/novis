`#[Test]` and `#[Core\Test]` are the same attribute. `use Core\Test;` makes the short name work for the
attribute and for the assertions, such as `Test::assertSame`.

A `#[Test]` method that is `static`, `private` or `protected`, or that returns a value, is `E0733`.
The message names the rule the method breaks.

A test method may declare parameters. A fixture or a data row fills each one. A parameter that
nothing fills is `E0736`.

The report lists the classes in name order. Inside a class, the tests run in the order they are
written. Each test line starts with a mark: `✓` passed, `✗` failed, `-` skipped and `!` flaky.

A test that calls no assertion fails with the message `it asserted nothing`. To test that a call
finishes without an error, write `Core\Test::assertDoesNotThrow`.

A test that throws an error fails, and the error's message is printed under the test.

What a test prints with `echo` is not shown when the test passes. When the test fails, its output
is printed under the failure, after `output:`. `nvs test --show-output` prints the output of every
test.

A `#[Test]` method in an `abstract` class runs only in each class that extends it. A `#[Test]` on a
method of an interface does nothing.

A file with no `#[Test]` method runs no test. The summary is `0 failed, 0 passed, 0 skipped, 0
flaky`, and the exit status is `0`.

`nvs run` on a file with tests runs the statements outside the classes and runs no test.
