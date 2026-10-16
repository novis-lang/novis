Returns the sum of this number and another one, as a new `Core\BigInt`.

There is no size at which the sum stops being exact. Adding two `int` values whose total is too large
for an `int` throws an error, and `Core\BigInt` is what you use instead when a total may grow that
far.

Both numbers stay as they were. `add` returns a new number, so a running total is the number you get
back from it, not the one you called it on.

**Good to know:** `$a + $b` does not work on two `Core\BigInt` values, because `+` is only for plain
numbers. `add` is how you write it. To take a number away, use `sub`, or add a negative number.
