`Exception` and `Error` are not declared. `class ConfigError extends Exception`, `catch (Error $e)` and
`new Exception("…")` are each the error `E0303`. Write `Throwable`, `LogicError` or `RuntimeError`.

`Throwable` is a class. You can create one with `new Throwable("…")`, and you can extend it directly.

`ArithmeticError` extends `Throwable`, not `RuntimeError`. A `catch (RuntimeError $e)` clause does not
catch it. These are the messages it has:

- Division by zero gives `Division by zero`.
- `%` by zero gives `Modulo by zero`.
- An `int` addition past the largest `int` gives `Integer addition overflowed`.
- `2.5 as int` gives ``cannot convert `float` 2.5 to `int` ``. A `float` with a fraction does not
  convert to `int`.

Your own class can extend any class in the tree, `ArithmeticError` and `ParseError` too. A class that
extends `ParseError` has `issues`, and the list is empty. A class in the tree may be `final` or
`abstract`.

`Core\Test\Failure` is the class of a failed assertion. It extends `Throwable` directly, and your own
class cannot extend it: that is the error `E0303`.

`Core\Script::finish()` ends the program, and no `catch` catches it. A `finally` block still runs.
