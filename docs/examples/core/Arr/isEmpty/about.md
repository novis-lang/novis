Tests whether an array holds no entries at all.

The result is `true` when the array has no entries, and `false` when it has at least one. A value of
`null`, `0` or an empty text is still an entry, so an array holding one of those is not empty. What
counts is how many entries are stored, never what is stored in them.

`Core\Arr::isEmpty` reads a number the array already keeps, so it does not look at the entries. The
answer costs the same for three entries and for three million.

**Good to know:** this is the way to tell an empty array from one whose first entry is `null`.
`Core\Arr::first` returns `null` for both.
