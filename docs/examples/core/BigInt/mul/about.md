Returns the product of this number and another one, as a new `Core\BigInt`.

There is no size at which the product stops being exact. Multiplying two `int` values whose product
is too large for an `int` throws an error, and `Core\BigInt` is what you use instead when a product
may grow that far.

The sign follows the usual rule: two numbers with the same sign give a positive product, and two
with different signs give a negative one.

Both numbers stay as they were. `mul` returns a new number, so a running product is the number you
get back from it, not the one you called it on.

**Good to know:** `$a * $b` does not work on two `Core\BigInt` values, because `*` is only for plain
numbers. `mul` is how you write it.
