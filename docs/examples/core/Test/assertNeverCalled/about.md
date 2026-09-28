`Core\Test::assertNeverCalled()` checks that your code did not call a method on a test double. It
is the opposite of `Core\Test::assertCalled`, and it reads the same list of calls.

You write the method as `Mailer::send`, and the compiler checks the name. One call to the method is
enough to fail. The check then throws a `Core\Test\Failure`. The message gives the number of calls
and the arguments of the first one. An object that is not a double throws a `LogicError`.

Use it for code that must not do something: a dry run that must not delete files, or a failed
payment that must not send a confirmation.

**The examples below** show a dry run, the message of a failed check, and an order that must not
send an email.
