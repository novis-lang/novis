Returns the rest of a string after a given start, or `null` if the string does not start with it.

`Core\Str::afterPrefix` checks whether the string starts with the prefix. If it does, the result is
everything after the prefix. If it does not, the result is `null`. So one call does the check and
the cut. If the prefix is the whole string, the result is an empty string. An empty prefix gives the
whole string back.

The check is case-sensitive: `"Bearer "` does not match `"bearer "`. The prefix must also end after a
whole character, the way a person reads it. For example, `"e\u{301}"` is an "e" with a combining
accent, and a person reads it as one character: "é". The prefix `"e"` ends inside that character, so
the result is `null`. `Core\Str::startsWith` uses the same rule, so it returns `false` for these two
strings. `Core\Str::beforeSuffix` does the same at the end of a string.

**The examples below** read a day number out of form field names, read the token out of an
`Authorization` header, and show that a prefix cannot end inside a character.
