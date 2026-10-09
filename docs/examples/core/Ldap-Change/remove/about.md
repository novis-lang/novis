Returns a `Core\Ldap\Change` that removes values from one attribute of a directory entry. The
other values of the attribute stay.

The change does nothing on its own. You put it in a list and pass the list to a connection's
`modify` method. `modify` sends every change in the list in one request, so the server makes all of
them or none of them. A `remove` and an `add` in one list replace one value with another.

**Good to know:** the entry must have the value. If it does not, `modify` throws an `LdapError` and
the server changes nothing. To remove every value of an attribute, use `removeAll`. A name that is
not an attribute name throws a `LogicError` at once.
