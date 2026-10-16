Deletes one entry from the directory. It returns nothing.

You give it the DN of the entry, as a `Dn` or as text. The connection must log in as an account
that may delete the entry.

`delete` deletes only one entry. An entry that has other entries below it cannot be deleted, and
`delete` throws `Core\Ldap\LdapError`. Delete the entries below it first, then the entry itself.
When there is no entry with the DN, `delete` throws `Core\Ldap\LdapError` with `$kind` set to
`NoSuchObject`.

**Good to know:** Active Directory keeps a deleted entry for some time, with most of its
attributes removed. `read` and `search` do not find it, unless the search sets `showDeleted`.
