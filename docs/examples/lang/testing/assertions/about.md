An assertion checks one thing about a value and reports it when the check does not hold.

Every assertion is a static method of `Core\Test`. You write the value your code produced first,
and the value you expect second. An assertion that holds returns and does nothing. One that does
not hold records the failure and throws `Core\Test\Failure`, and the error message names both
sides. You may add `{message: "..."}` at the end of any assertion to put your own sentence in front
of that report.

Catching a `Core\Test\Failure` does not make a test pass. The runner reads its verdict from the
record, so a test that caught its own failure still fails, and every failed assertion of a test is
reported.

**Good to know:** both sides of an assertion have the same type, so `assertSame(1, "1")` does not
compile.
