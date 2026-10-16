Wherever the `{preserveKeys: …}` option appears — `slice`, `chunk`, `reverse`, the sorts —
**`false` means the result is a list**, keys renumbered from `"0"`, and `true` means every key is kept.

The option has one meaning over every array shape. Renumbering integer keys while keeping string ones
would be the key-type-dependent rule `rule:types/array-combination` removes, arriving through an
option name.

`false` is the default because these members are overwhelmingly called with a list. For an argument
with non-numeric keys, `false` drops the keys and `{preserveKeys: true}` keeps them. PHP's
`array_slice` and `array_reverse` keep string keys whatever the option says, so here the result
differs from PHP.

**Declined:**
- **A default of `true`.** Slicing a list from offset 2 would give keys `"2"` and `"3"`, which is not a
  list, breaking the common case and every `Core\Json::encode` after it.
