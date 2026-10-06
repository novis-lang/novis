Returns the text of one group of a match, by the group's number or by its name.

A group is a part of the pattern in round brackets. Groups are numbered from `1`, left to right,
and group `0` is the whole match. A group written as `(?<name>...)` also has a name, and you can
read it by that name.

The result is `null` when the pattern has the group, but this match did not use it. This happens
with an optional group, such as `(\d+)?`. If the pattern has no group with that number or name,
`group` throws a `RuntimeError`. So a typing mistake in a group name is an error, and not an empty
value.

The examples show reading groups by number, a group that did not match, and reading the parts of a
log line by name.
