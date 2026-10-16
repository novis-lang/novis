Returns a `Core\Ldap\Change` that adds values to one attribute of a directory entry. You give the
attribute's name and one value or a list of values.

The change does nothing on its own. You put it in a list and pass the list to a connection's
`modify` method. `modify` sends every change in the list in one request, so the server makes all of
them or none of them.

**Good to know:** the entry must not have the value yet. If it has, `modify` throws an `LdapError`
and the server changes nothing. The attribute name must start with a letter and contain only
letters, digits and `-`. Any other name throws a `LogicError` at once. A value is written in the
form that `Core\Ldap\Entry` returns it, such as an `int`, a `Core\Uuid` or an `Instant`.
