`Core\Math::max` returns the larger of two values. To find the largest value
in an array, use `Core\Arr::max`.

It compares numbers, strings, `bool` values and `null`. The two values must be of the same type.
Strings are compared by their characters, so `"pear"` is larger than `"apple"`. When the two
values are equal, the result is the first one. `NaN` (a value that means "not a number") is
smaller than every other `float`.

When the two values cannot be compared, `Core\Math::max` throws a `RuntimeError`. Two arrays, two
objects, or a number and a string in two `mixed` variables are examples.

**The examples below** show numbers and strings, then `NaN` and values that cannot be compared,
then a stock level that must never go below zero.
