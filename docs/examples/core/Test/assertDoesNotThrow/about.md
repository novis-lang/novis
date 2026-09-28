`Core\Test::assertDoesNotThrow()` checks that a piece of code runs without an error. You pass the
code as a function with no parameters. The check calls it once. The value it returns is not used.

When the function throws an exception, the check throws a `Core\Test\Failure` in its place. The
message contains the message of the exception. The option `message` adds your own text in front of
it. A failed check inside the function also counts as an error, so this check fails too.

Use it for a test whose whole point is that a call works, for example an import that must skip bad
input. A test that checks a result uses `Core\Test::assertEquals` on the result.

**The examples below** show a call that works, the message of a failed check, and a test of an import
with broken lines.
