Every arm checks the error that the expression on the left threw. In `f() catch (LogicError $a) => x
catch (IOError $b) => y`, the `IOError` arm does not check `x`.

An arm for a parent class that is written before an arm for its subclass matches first. The later
arm never runs. This compiles without a warning.

`catch` covers everything to its left up to an assignment. In `$x = $a / $b catch (ArithmeticError
$e) => 0`, it covers the whole division. It also covers a whole `?:` and a whole `??` chain.

The variable is optional: `catch (IOError) => 0` is allowed. An arm names one class.
`catch (IOError | LogicError $e)` does not compile: it is the error `E0245`.

An arm's variable can only be used inside that arm. After the arm, you may declare the same name
again. The name must not already be a variable of the function when the arm starts: that is the
error `E0406`.

The type of the result includes the type of the expression and the type of every arm. If `get`
returns `int` and an arm gives `null`, the result is `?int`. Writing that result into an `int`
variable does not compile: it is the error `E0401`. An arm that throws adds nothing to the type.

`return`, `break` and `continue` in an arm do not compile: that is the error `E0126`.

This form has no `finally`. When the error leaves the expression, the `finally` block of the `try`
around it runs as usual.

The warning for `catch (Throwable) => value` is `W1006`. An arm with a variable does not get it.
