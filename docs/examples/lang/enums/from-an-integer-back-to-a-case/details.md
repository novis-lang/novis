The value is first converted to the backing type, with the rules of `as int` or `as uint`. Then the
enum is searched for a case with that number. Each step can fail:

- A number that no case has throws a `RuntimeError`. The message lists every case of the enum.
- Text that is not a whole number throws a `RuntimeError`. Text with a space, such as `" 1"`, and text
  with a fraction, such as `"2.0"`, are not whole numbers.
- A `float` with a fraction, such as `1.5`, throws an `ArithmeticError`. A `float` without one, such
  as `2.0`, finds the case with the number `2`.
- A negative number for a `uint` enum throws an `ArithmeticError`.
- `null` throws a `RuntimeError`.

`as ?Level` gives `null` in each of these cases, and throws nothing.

`ArithmeticError` is not a kind of `RuntimeError`. A `catch (RuntimeError $e)` does not catch it.

When two cases have the same number, the result is equal to both of them.

A `mixed` value and a `tainted string` convert the same way. `as Level` does not convert a `bool`:
`true as Level` is the error `E0708`.

An enum has no `from` or `tryFrom` method. `Level::from(1)` is the error `E0309`. Write `1 as Level` or
`1 as ?Level`.
