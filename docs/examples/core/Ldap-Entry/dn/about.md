Returns the DN of an entry as a `Core\Ldap\Dn`.

A DN (distinguished name) is the full path of an entry in the directory, such as
`CN=Anna Berg,OU=Company,DC=example,DC=test`. Every entry has a DN, so `dn` never returns `null`.

The result is a `Core\Ldap\Dn` object. `toString` returns the whole DN as text. `rdnValue` returns
the entry's own name, such as `Anna Berg`. `parent` returns the DN of the folder the entry is in.
You can also pass the `Dn` to `read` to read the same entry again.

**Good to know:** the DN is not an attribute. `toArray` does not contain it, and `string("dn")`
returns `null`.
