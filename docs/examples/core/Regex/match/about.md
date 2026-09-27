Finds the first place where a pattern matches a text, and returns the match with its parts.

The result is a `Core\Regex\Match`, or `null` if the pattern matches nowhere. A `Match` has the
matched text, the position where it starts, and the text of each group in the pattern. A group is a
part of the pattern in round brackets, and you can read it by its number or by its name. The `from`
option sets the position where the search starts. Positions count characters, not bytes. This
replaces PHP's `preg_match` with its `$matches` array.

**Good to know:** always check for `null` before you read the match. To find every match instead of
the first one, use `Core\Regex::matchAll`.
