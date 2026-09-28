`Core\Test::assertCalled()` checks that your code called a method on a test double. You make the
double with `Core\Test::double` or `Core\Test::partial`. The double keeps a list of every call it
gets, and this check reads that list after your code has run.

You write the method as `Mailer::send`. The compiler checks this name, so a renamed method breaks the
test at compile time. The option `times` gives the exact number of calls. Without it, one call or
more passes. The option `with` gives the arguments that one of the calls must have.

When the check fails, it throws a `Core\Test\Failure`. The message lists every call the double got.
An object that is not a double throws a `LogicError`.

**The examples below** show a check that an email was sent, a check of the number of calls, and a
test that a checkout charges the customer once.
