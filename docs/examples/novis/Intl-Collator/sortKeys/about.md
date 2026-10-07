Gives each string a sort key: bytes that compare in the order of a language and region.

`Collator::sortKeys` returns one key for each string, in the same order as the list. Compare two keys
byte by byte, for example with `Core\Bytes::compare`. The result is the same order that
`Collator::sort` gives the two strings for the same locale and options. Equal strings have equal keys.

You can store a key in a database column and sort rows by that column. The database then sorts in
the order of the language, and it does not need to know the language rules.

The options are the same as for `Collator::sort`. With `strength: Strength::Primary`, `"Apple"` and
`"apple"` have the same key.

`Collator::sortKeys` throws a `LogicError` when the locale tag is not valid.

**Good to know:** a key belongs to one locale, one set of options and one version of Novis. Compare
only keys that were made the same way, and make them again after an upgrade.
