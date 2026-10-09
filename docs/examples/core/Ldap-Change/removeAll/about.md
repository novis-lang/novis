Returns a `Core\Ldap\Change` that removes an attribute and every value it has from a directory
entry. You give only the attribute's name.

The change does nothing on its own. You put it in a list and pass the list to a connection's
`modify` method. `modify` sends every change in the list in one request, so the server makes all of
them or none of them.

**Good to know:** the entry must have the attribute. If it does not, `modify` throws an `LdapError`.
`replace` with `null` also removes an attribute, and it works when the attribute is missing. A name
that is not an attribute name throws a `LogicError` at once.
