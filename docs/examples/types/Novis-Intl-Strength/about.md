Which differences between two strings change their order when `Collator` sorts them.

You pass a `Strength` to `Collator::sort` or `Collator::sortKeys` as `strength`. Each level compares
more than the one before it:

- `Primary` compares the letters only. "resume", "Résumé" and "RESUME" are equal.
- `Secondary` also compares accents, so "resume" and "résumé" are different.
- `Tertiary` also compares upper and lower case. This is the default.
- `Quaternary` and `Identical` find even smaller differences. With `Identical`, two strings are
  equal only when they are the same text.

Strings that are equal keep the order they had in the list.

**Good to know:** use `Primary` for a search that should ignore case and accents.
