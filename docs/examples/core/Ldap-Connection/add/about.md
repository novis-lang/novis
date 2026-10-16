Adds a new entry to the directory. It returns nothing.

You give it the DN of the new entry and its attributes. The attributes are an array keyed by the
attribute name, such as `["objectClass" => "organizationalUnit", "description" => "Sales"]`. A list
as the value writes one value per element, such as `"objectClass" => ["top", "group"]`.

A value is written in the same form the readers of `Core\Ldap\Entry` return it. A `string` is
written as it is. An `int`, a `bool` and a `Core\Ldap\Dn` are written as the text the server
expects, so a value you read can be written back. A `float` has no LDAP form, and `add` throws a
`LogicError` for it.

If the server does not accept the entry, `add` throws `Core\Ldap\LdapError`. Its `$kind` says why:
`AlreadyExists` when an entry has that DN, `NoSuchObject` when the parent entry does not exist, and
`InsufficientAccess` when the account may not add it.

**Good to know:** the connection must log in as an account that may add entries below the parent.
Every entry needs an `objectClass`, and the server adds attributes of its own, such as
`objectGUID` and `whenCreated`.
