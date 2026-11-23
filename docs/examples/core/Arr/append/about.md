Adds one or more values to the end of an array.

`Core\Arr::append` never changes the array you give it. It returns a new array: first every entry of
your array, under the key it already had, then each value you added.

Every key in Novis is a string. An added value gets the same key that `$a[] = $v` would give it.
For a list of three entries, that is the key `"3"`, then `"4"`. An array with no number key at all
starts at `"0"`. The array remembers its next number key, and `unset` does not lower it. So when
`"9"` is the largest number key, the next key after `unset($a["9"])` is still `"10"`.

You can pass as many values as you like in one call. They are added in the order you write them. If
you pass no value, the result is your array, entry for entry.
