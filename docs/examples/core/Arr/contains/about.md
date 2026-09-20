Tests whether an array holds a value. It replaces PHP's `in_array`.

The result is `true` as soon as one entry is the value you are looking for, and `false` when no
entry is. The empty array gives `false`.

The comparison is always exact. There is no second way to compare and no extra argument to pass, so
a text is only found by the same text, letter for letter, and `"1"` never finds the number `1`.
Numbers are the one thing compared across types: a whole number and the same number written with a
fraction are equal, so `1` finds `1.0`.

**Good to know:** this looks at the values, never at the keys. Use `Core\Arr::hasKey` to test a key,
and `Core\Arr::keyOf` when you need the key of the entry that was found.
