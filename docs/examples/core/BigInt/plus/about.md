Returns the sum of this number and another one, as a new `Core\BigInt`.

The sum is always exact, however large the numbers are. When the sum of two `int` values is too
large for an `int`, the addition throws an error. Use `Core\BigInt` when a total may grow that far.

`plus` does not change either number. It returns the result as a new number, so keep that result in
a variable.

**Good to know:** `$a + $b` does not work on two `Core\BigInt` values, because `+` is only for plain
numbers. `plus` is how you write it. To take a number away, use `minus`, or add a negative number.
