Tests whether an array has an entry stored under a key.

The result is `true` when the key is present, whatever is stored under it. A key whose value is
`null` is still present. Use `Core\Arr::hasKey` to tell a stored `null` from a key that is not in
the array.

The key is an `int`, a `uint` or a `string`. `1` and `"1"` are the same key. In a plain list the keys are the positions, so `0` names the first entry.

**Good to know:** this looks at the keys, never at the values. Use `Core\Arr::contains` to search
the values instead.
