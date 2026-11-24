Which differences between two strings change their order when `Collator` sorts them.

`Collator::Strength` is declared in the class `Novis\Intl\Collator`.

`Collator` sorts strings by the rules of a language. You pass a `Collator::Strength` to
`Collator::sort` or `Collator::sortKeys` as `strength`. Each level compares more than the one before
it:

- `Primary` compares the letters only. "resume", "Résumé" and "RESUME" are equal.
- `Secondary` also compares accents, so "resume" and "résumé" are different.
- `Tertiary` also compares upper and lower case. This is the default.
- `Quaternary` also compares punctuation when the option `ignorePunctuation` is `true`. In Japanese,
  it also compares Hiragana and Katakana, the two Japanese syllable alphabets.
- `Identical` also compares the characters themselves when everything else is equal. It is the
  slowest level.

`ignorePunctuation` is another option of `Collator::sort` and `Collator::sortKeys`. When it is `true`,
`Primary`, `Secondary` and `Tertiary` do not compare punctuation.

Strings that are equal at the chosen strength keep the order they had in the list.

**Good to know:** use `Primary` for a search that should ignore case and accents.
