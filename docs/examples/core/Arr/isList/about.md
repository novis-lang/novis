Tests whether an array is a plain list. It replaces PHP's `array_is_list`.

An array is a list when its keys are exactly `0`, `1`, `2` and so on, with no key missing and no key
out of order. An array you build with `[]`, or by adding entries with `$a[] = ...`, is a list. An
array with names as keys is not, and neither is one whose numbers start at `1` or skip a number.

An array with no entries is a list.

**Good to know:** this is the test that says what an array will look like in JSON. A list is written
as a JSON array, and every other array is written as a JSON object.
