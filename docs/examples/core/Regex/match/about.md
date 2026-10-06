Finds the first place where a pattern matches a text, and returns the match with its parts.

The result is a `Core\Regex\Match`, or `null` if the pattern matches nowhere. A `Match` has the
matched text, the position where it starts, and the text of each group in the pattern. A group is a
part of the pattern in round brackets, and you can read it by its number or by its name. The `from`
option sets the position where the search starts. Positions count characters, not bytes.

The pattern must be text from your program. A pattern from a user does not compile here. To search
for text a user typed, put it in the pattern with `Core\Regex::quote`.

**Good to know:** always check for `null` before you read the match. To find every match instead of
the first one, use `Core\Regex::matchAll`. To only check whether the pattern matches, use
`Core\Regex::matches`.

related: Core\Regex::matches, Core\Regex::matchAll, Core\Regex::quote
