Returns a new array with every `null` value removed.

`Core\Arr::withoutNull` never changes the array you give it. Every entry that stays keeps its own
key. Use `Core\Arr::values` when you want the keys numbered from `"0"` again.

The type of the result has no `null`. An `array<?string>` gives an `array<string>`, so you can use
the values without a cast. An `array<mixed>` gives an `array<mixed>`.

**Good to know:** only `null` is removed. The values `0`, `''`, `false` and `[]` stay in the result.
An array of only `null` values gives an empty array, and that is not an error.
