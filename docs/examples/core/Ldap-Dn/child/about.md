Returns the DN of an entry directly below this one. The new part, `attribute=value`, is added at
the start.

Use it to build the DN of a new user or group below a base you know, such as
`OU=Staff,DC=example,DC=test`. The value can come from user input. Each special character in it is
escaped, so it stays one value.

**Good to know:** the DN you call it on does not change, and you get a new DN back. An empty value
throws a `LogicError`, and so does an attribute that is not a name.
