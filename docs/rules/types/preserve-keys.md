Wherever the `{preserveKeys: …}` option appears — `slice`, `chunk`, `reverse`, the sorts —
**`false` means the result is a list**, keys renumbered from `"0"`, and `true` means every key is kept.

PHP renumbers integer keys and silently keeps string ones, which is the key-type-dependent rule
`rule:types/array-combination` removes, arriving through an option name. Here the option has one
meaning over every array shape.

`false` stays the default, matching PHP for a list argument, which is what these members are
overwhelmingly called with. For an argument with non-numeric keys the result differs from PHP's — the
keys are gone rather than kept — and `{preserveKeys: true}` is the faithful rewrite where that
mattered.
