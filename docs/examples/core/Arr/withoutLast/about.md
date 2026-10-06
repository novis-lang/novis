Returns a new array with every entry except the last one.

`Core\Arr::withoutLast` never changes the array you give it. Every entry that stays keeps its own
key, so a list keeps the numbering it had and only its highest key is gone.

`Core\Arr::last` returns the value this method leaves out. Use the two together when you want to read
the newest value and then continue with the ones before it.

**Good to know:** an array with one entry gives an empty array, and an empty array gives an empty
array. Both are normal results, and neither is an error.
