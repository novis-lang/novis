Which part of an entry is compared when you ask what two arrays have in common, or what only one of
them has.

`Core\Arr::SetOn` is declared in the class `Core\Arr`. `Core\Arr::difference` and `Core\Arr::intersection`
take it as the `on` option, and compare the values by default. `Values` compares values
alone, `Keys` compares keys alone, and `Both` counts an entry as matching only when the key and the
value both match.

The result always keeps the keys of the first array and their order.

**Good to know:** the `by` option changes the part that `on` selects before it is compared. Under
`Keys`, the `by` function changes the key, not the value.
