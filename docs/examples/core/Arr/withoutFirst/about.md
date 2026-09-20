Returns a new array with every entry except the first one. It replaces what PHP's `array_shift`
leaves behind.

`Core\Arr::withoutFirst` never changes the array you give it. Every entry that stays keeps its own
key. A list numbered from `"0"` gives a result numbered from `"1"`. Use `Core\Arr::values` when you
want the keys numbered from `"0"` again.

`Core\Arr::first` returns the value this method leaves out. Use the two together when you want to
read the first value and then continue with the rest.

**Good to know:** an array with one entry gives an empty array, and an empty array gives an empty
array. Both are normal results, and neither is an error.
