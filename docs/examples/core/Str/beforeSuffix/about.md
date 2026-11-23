Returns the start of a string before a given end, or `null` if the string does not end with it.

`Core\Str::beforeSuffix` checks whether the string ends with the suffix. If it does, the result is
everything before the suffix. If it does not, the result is `null`. So one call does the check and
the cut. If the suffix is the whole string, the result is an empty string. An empty suffix gives the
whole string back.

The check is case-sensitive: `".CSV"` does not match `".csv"`. The suffix must also start at a whole
character, the way a person reads it. For example, `"\r\n"` is one line ending, and it counts as one
character. The suffix `"\n"` starts inside that character, so the result is `null`.
`Core\Str::endsWith` uses the same rule, so it returns `false` for these two strings.
`Core\Str::afterPrefix` does the same at the start of a string.

**The examples below** take the name of a file without its extension, remove a trailing `/` from a
path, and show that a suffix cannot start inside a character.
