`Core\Test::assertTrue()` checks that a value is `true`. The value must be a `bool`. A call with a
number, a string or an array does not compile. Write a comparison instead, for example
`$count > 0`.

When the value is `true`, the check passes and nothing happens. When the value is `false`, the check
throws a `Core\Test\Failure`. The message only says that the value is `false`. The option `message`
adds your own text at the start, so you can say what you expected.

Use it for a condition that no other check tests, for example a rule about passwords or a date that
must be in the future.

**The examples below** show checks that pass, the message of a failed check, and a test of a
password rule.
