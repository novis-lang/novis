Returns the text of every group of a match in one array.

The first entry is group `0`, the whole match. The other groups follow in the order they start in
the pattern. A group with a name is in the array twice: first under its name, then under its
number. This is the same order as PHP's `$matches` array after `preg_match`.

A group that this match did not use is still in the array, and its value is `null`. So the array
always has one entry for every group in the pattern, and you can tell an unused group from a group
that matched an empty text.

**Good to know:** to read one group, `group` is shorter and throws an error for a group that does
not exist.

The examples show the whole array, a named group that appears twice, and turning a line of text
into named fields.
