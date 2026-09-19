`throw` reports an error and stops the work in progress. It takes an object of a class under
`Throwable`, such as `LogicError`, `IOError`, or a class you wrote yourself. The nearest `catch`
clause whose class matches takes over, and the lines after the `throw` never run.

`throw` is a statement, and it is also an expression. You can write it where a value is expected:
after `??` when a value may be missing, in a `match` arm for a subject the program has no answer
for, and in a ternary arm. In each of those places the program either has a value or throws, so
every line after it can use the value.

**The examples below** show `throw` after `??` for a missing field, in the `default` arm of a
`match` over order statuses, and in the checks a program runs before it ships an order.
