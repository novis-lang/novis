Returns the whole text that a pattern matched.

This is the same text as group `0`. The result is never `null`, because every match has a whole
text. It can be an empty string: a pattern such as `\d*` can match zero characters.

Use `text` when you need the matched part and not its groups. To read one part of the match, use
`group` with the number or the name of the group.

The examples show reading a match, an empty match, and collecting every tag in a message.
