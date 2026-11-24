Returns the result of taking another number away from this one, as a new `Core\BigInt`.

The difference is always exact, however large the numbers are. Two numbers that each fit in an `int`
can have a difference that does not. Use `Core\BigInt` when a value may grow that large.

The result can be negative. When `$other` is the larger of the two numbers, the result is negative.

`minus` does not change either number. It returns the result as a new number, so keep that result
in a variable.

**Good to know:** `$a - $b` does not work on two `Core\BigInt` values, because `-` is only for plain
numbers. `minus` is how you write it. `$a->minus($b)` and `$a->plus($b->negated())` give the same
result.
