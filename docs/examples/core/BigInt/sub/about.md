Returns the result of taking another number away from this one, as a new `Core\BigInt`.

There is no size at which the difference stops being exact. Two numbers that each fit in an `int` can
have a difference that does not, and `Core\BigInt` is what you use when a value may reach that far.

The result carries a sign. When `$other` is the larger of the two numbers, the result is negative.

Both numbers stay as they were. `sub` returns a new number, so a balance you draw down is the number
you get back from it, not the one you called it on.

**Good to know:** `$a - $b` does not work on two `Core\BigInt` values, because `-` is only for plain
numbers. `sub` is how you write it. `$a->sub($b)` and `$a->add($b->neg())` give the same result.
