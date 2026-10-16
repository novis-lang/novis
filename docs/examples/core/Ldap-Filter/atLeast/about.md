Builds a filter that finds the directory entries where a value of an attribute is equal to or greater
than this value.

The server compares by the attribute's own order. For a number, `10` is greater than `9`. For a time,
a later time is greater. For text, the order is alphabetical. The value is sent as data, so it may come
from a form.

LDAP has no "greater than" filter. To find values greater than `5`, combine `Core\Ldap\Filter::not`
with `Core\Ldap\Filter::atMost` and the value `5`.

A name with a character that an attribute name cannot have throws a `LogicError`.

**Good to know:** some attributes have no order at all. For those, the server finds nothing.
