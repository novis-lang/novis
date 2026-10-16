`Core\Test::double()` builds a test double: an object that you use in a test in place of a real
one. You write the interface in angle brackets, `Core\Test::double<Mailer>(...)`, and give one
callable for each method of that interface. The result is a `Mailer`. You can pass it to any code
that takes a `Mailer`.

When your code calls a method on the double, the callable for that method runs and its result is
the return value. The compiler checks the callables. A missing method, an extra method or a callable
with the wrong parameter types does not compile. The double also keeps a list of its calls, which
`Core\Test::assertCalled` and `Core\Test::assertNeverCalled` read.

**The examples below** show a fixed clock, two doubles that test both results of a payment, and a
report tested with a fake order store.
