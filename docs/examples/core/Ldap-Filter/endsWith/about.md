Builds a filter that finds the directory entries where a value of an attribute ends with this text.

A common use is to find every person whose mail address is in one domain, such as all addresses that
end with `@example.test`. The text is sent to the server as data. A `*` in it is a normal character and
not a wildcard, so a value from a form cannot widen the search.

An empty text throws a `LogicError`, and so does a name with a character that an attribute name cannot
have.

**Good to know:** a search like this is often slower than `equals` or `startsWith`, because many
servers cannot use an index for it.
