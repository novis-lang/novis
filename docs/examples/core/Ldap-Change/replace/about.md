Returns a `Core\Ldap\Change` that replaces every value of one attribute of a directory entry. You
give the attribute's name and the new value or list of values.

The change does nothing on its own. You put it in a list and pass the list to a connection's
`modify` method. `modify` sends every change in the list in one request, so the server makes all of
them or none of them.

**Good to know:** `replace` works whether the attribute has values or not. With `null` or an empty
list, the attribute is removed. A value is written in the form that `Core\Ldap\Entry` returns it,
such as an `int`, a `bool`, a `Core\Uuid` or an `Instant`. A name that is not an attribute name
throws a `LogicError` at once.
