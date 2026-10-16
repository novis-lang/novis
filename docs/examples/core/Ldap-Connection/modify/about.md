Changes the attributes of one entry in the directory. It returns nothing.

You give it the DN of the entry and a list of changes. Each change is made with one of four
functions of `Core\Ldap\Change`:

- `Change::add` adds values to an attribute.
- `Change::remove` removes some values from an attribute.
- `Change::removeAll` removes an attribute with all of its values.
- `Change::replace` removes the old values and writes the new ones.

The server makes the changes in the order of the list. If one change fails, the server makes none
of them, and `modify` throws `Core\Ldap\LdapError`. Its `$kind` says why, such as
`ConstraintViolation` for a value that is not there to remove.

The connection must log in as an account that may change the entry. A value is written in the
same form the readers of `Core\Ldap\Entry` return it, so a value you read can be written back.

**Good to know:** the list cannot be empty. Some attributes belong to the server, such as
`objectGUID` and `objectSid`, and a change to one of them throws an error.
