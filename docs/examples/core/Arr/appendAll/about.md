Joins arrays end to end, and numbers the result from 0.

You give a first array and as many more as you like. The result holds every value of the first array,
then every value of the second, and so on, in that order. The keys the arrays had are not kept: the
result is always a list. Nothing is compared and nothing is dropped, so a value that is in two of the
arrays is in the result twice. It replaces PHP's `array_merge` over lists, including the
`array_merge(...$arrays)` form.

**Good to know:** use `Core\Arr::overlay` when the keys matter and a later value should replace an
earlier one with the same key.
