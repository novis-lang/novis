Builds a filter that finds the directory entries where a value of an attribute is equal to or less
than this value.

The server compares by the attribute's own order. For a number, `9` is less than `10`. For a time, an
earlier time is less. For text, the order is alphabetical. The value is sent as data, so it may come
from a form.

LDAP has no "less than" filter. To find values less than `5`, combine `Core\Ldap\Filter::not` with
`Core\Ldap\Filter::atLeast` and the value `5`.

A name with a character that an attribute name cannot have throws a `LogicError`.

**Good to know:** some attributes have no order at all. For those, the server finds nothing.
