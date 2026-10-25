An assertion checks one thing about a value. When the check is false, the test fails.

Every assertion is a static method of `Core\Test`. You write the value your code produced first,
and the value you expect second. An assertion that passes returns and does nothing. One that fails
records the failure and throws `Core\Test\Failure`, and the error message names both values. You
may add `{ message: "..." }` at the end of any assertion to put your own sentence in front of that
message.

Catching a `Core\Test\Failure` does not make a test pass. The runner keeps a record of every failed
assertion, so a test that caught its own failure still fails. The report lists every failed
assertion of a test.

**Good to know:** both sides of an assertion have the same type, so `assertSame(1, "1")` does not
compile.
