Wherever the `{preserveKeys: …}` option appears — `slice`, `chunk`, `reverse`, the sorts —
**`false` means the result is a list**, keys renumbered from `"0"`, and `true` means every key is kept.

The option has one meaning over every array shape. Renumbering integer keys while keeping string ones
would be the key-type-dependent rule `rule:types/array-combination` removes, arriving through an
option name.

`false` is the default because these members are overwhelmingly called with a list. For an argument
with non-numeric keys, `false` drops the keys and `{preserveKeys: true}` keeps them.
