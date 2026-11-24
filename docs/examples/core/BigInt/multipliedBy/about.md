Returns the product of this number and another one, as a new `Core\BigInt`.

The product is always exact, however large the numbers are. When the product of two `int` values is
too large for an `int`, the multiplication throws an error. Use `Core\BigInt` when a product may
grow that far.

The sign follows the usual rule: two numbers with the same sign give a positive product, and two
with different signs give a negative one.

`multipliedBy` does not change either number. It returns the result as a new number, so keep that
result in a variable.

**Good to know:** `$a * $b` does not work on two `Core\BigInt` values, because `*` is only for plain
numbers. `multipliedBy` is how you write it.
