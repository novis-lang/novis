Builds a filter that finds the directory entries where a value of an attribute starts with this text.

Use it for a search box that shows matches while a person types, or to find every account whose
name starts with the same letters. The text is sent to the server as data. A `*` in it is a normal
character and not a wildcard, so a value from a form cannot widen the search.

An empty text throws a `LogicError`. To find every entry that has the attribute at all, use
`Core\Ldap\Filter::present`. A name with a character that an attribute name cannot have throws a
`LogicError` too.
