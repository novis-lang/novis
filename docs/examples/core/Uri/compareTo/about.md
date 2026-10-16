Compares two addresses and returns `-1`, `0` or `1`, so you can sort addresses or check if two of them
are the same.

`$a->compareTo($b)` returns `0` when both are the same address, even when they are written in a
different way. Before it compares them, it makes three changes to a copy of each address. The scheme
and the host become lower case. An escape such as `%7E` becomes the plain character, here `~`, when that
character does not need an escape. The `.` and `..` parts are removed from a path that starts
with `/`. Neither address itself changes. In all other cases, the result is `-1` when `$a` comes first
and `1` when `$b` comes first.

**Good to know:** `http://example.com:80/` and `http://example.com/` are different here, because
`compareTo` does not know which port each scheme uses by default. The path is case-sensitive, so `/A`
and `/a` are different too.

**The examples below** check two ways to write one address, sort a list of addresses, and skip pages a
program has already visited.
