`$failure is RuntimeError` and `$failure is LogicError` are both `false`. Only a `catch` for
`Core\Test\Failure` or for `Throwable` takes a failed assertion.

An assertion that passes returns nothing and prints nothing.

You can create a failure yourself with `new Core\Test\Failure("…")`. Your own class cannot extend it.

A failure that nothing catches ends the program with status 1, like any uncaught error. The report on
standard error has the class `Core\Test\Failure` and the message.
