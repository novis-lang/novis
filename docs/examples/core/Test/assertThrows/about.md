`Core\Test::assertThrows()` checks that a piece of code throws an error of a given class. You pass
the code as a function with no parameters, and the name of the class, for example
`LogicError::class`. The check calls the function once.

The check passes when the function throws that class or a class that extends it. The error is caught,
so your test continues after the check.

When the function throws no error, or an error of another class, the check throws a
`Core\Test\Failure`. The message names the class that was thrown and its message. The option
`message` adds your own text in front of it.

**The examples below** show a check that passes, the messages of failed checks, and a test of a
payment that is larger than the balance.
