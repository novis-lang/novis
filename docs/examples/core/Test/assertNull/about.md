`Core\Test::assertNull()` checks that a value is `null`. The value may have any type. When it is
`null`, the check passes and nothing happens.

When the value is not `null`, the check throws a `Core\Test\Failure`. The message shows the value.
The option `message` adds your own text in front of it.

The check only tests for `null`. The values `0`, `""`, `false` and an empty array are not `null`, so
the check fails for each of them.

Use it to test code that returns `null` when it finds nothing, for example a search for a user.

**The examples below** show a check that passes, the message of a failed check, and a test of a
search with no result.
