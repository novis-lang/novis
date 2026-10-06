Tests whether an array has an entry stored under a key.

The result is `true` when the key is present, whatever is stored under it. A key whose value is
`null` is still a key that is present, so `Core\Arr::hasKey` is how you tell a stored `null` from a
key that is not in the array at all.

The key is an `int` or a `string`, and both name the same entry: `1` and `"1"` are one key. In a
plain list the keys are the positions, so `0` names the first entry.

**Good to know:** this looks at the keys, never at the values. Use `Core\Arr::contains` to search
the values instead.
