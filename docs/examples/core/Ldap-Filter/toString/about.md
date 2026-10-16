Returns a filter as LDAP filter text, such as `(&(objectClass=user)(cn=Ann))`.

Use it to write a filter to a log, or to see what a filter you built will find. A search does not use
this text. The server gets the filter in its own binary form, so the text is only for people to read.

A `*`, `(`, `)` or `\` in a value is written as `\` and two hex digits, so `Ann*` is written as
`Ann\2a`. That way the text shows that the `*` is part of the value.

**Good to know:** the text is `tainted`, because a value in the filter may come from a user. Treat it
as you treat any other user input.
