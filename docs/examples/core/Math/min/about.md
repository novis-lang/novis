`Core\Math::min` returns the smaller of two values. This replaces PHP's `min` when you give it two
values. To find the smallest value in an array, use `Core\Arr::min`.

It compares numbers, strings, `bool` values and `null`. The two values must be of the same type.
Strings are compared by their characters, so `"apple"` is smaller than `"pear"`. When the two
values are equal, the result is the first one. `NaN` (a value that means "not a number") is
smaller than every other `float`, so a `NaN` value is always the result.

When the two values cannot be compared, `Core\Math::min` throws a `RuntimeError`. Two arrays, two
objects, or a number and a string in two `mixed` variables are examples.

**The examples below** show numbers and strings, then `NaN` and values that cannot be compared,
then a discount that must never be larger than the price.
