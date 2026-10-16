Counts the entries in an array.

The result is a whole number, and `0` for an array with nothing in it. `Core\Arr::count` counts the
entries at the top level only. An entry that is itself an array counts as one, so counting the
values inside that entry is a second call.

The keys make no difference to the result. An array whose two keys are `0` and `1000000` has two
entries, so its count is `2`.

**Good to know:** `Core\Arr::isEmpty` tests whether an array has anything in it, and needs no
comparison with `0`. To count the characters of a text, use `Core\Str::length`.
