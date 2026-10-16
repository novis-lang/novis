Finds every match of a pattern in a text and replaces it with another text. It returns the new text.
If there is no match, it returns the same text.

The replacement text can contain parts of the match. `$0` is the whole match. `$1`, `$2` and so on
are the groups of the pattern, counted from the left. A named group is written `${name}`. Write
`${1}0` when a digit comes right after a group, and `$$` for one dollar sign. A group that the
pattern does not have inserts nothing. The `limit` option replaces only the first matches.
`\1` is not a group in the replacement text.

**Good to know:** the replacement is the same text for every match. To build a different text
for each match with your own code, use `Core\Regex::replaceWith`.

The examples show a simple replacement and `limit`, the use of groups, and hiding card numbers in a
log line.
