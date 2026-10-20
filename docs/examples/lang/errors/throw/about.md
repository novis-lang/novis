`throw` reports an error and stops the work in progress. It takes an object of a class under
`Throwable`, such as `LogicError`, `IOError`, or a class you wrote yourself. The nearest `catch`
clause whose class matches takes over, and the lines after the `throw` never run.

`throw new RuntimeError("the reason");` is the usual form. The message is the first argument, and a
class may add more. If no `catch` matches, the program ends with status 1.

`throw` is a statement, and it is also an expression. You can write it where a value is expected:
after `??` when a value may be missing, in the `default` arm of a `match`, and in a ternary
arm. In each of those places the program either has a value or throws, so
every line after it can use the value.

**The examples below** show `throw` after `??` for a missing field, in the `default` arm of a
`match` over order statuses, and in the checks a program runs before it ships an order. The last
one shows a method that throws for a value it cannot use, and a `catch` around the call.
