Reads the one entry at a DN. It returns a `Core\Ldap\Entry`, or `null` when there is no entry with
that DN.

You give the DN as a `Core\Ldap\Dn` or as text. Text cannot be `tainted`. When part of the DN comes
from a request, build the DN with `Dn::of` or `child`. They escape the special characters, so the
name cannot point to another entry.

The option `select` names the attributes the entry should have. Without it, the entry has every
attribute the account of the connection may read. Ask only for what you need: each value is sent
over the network.

**Good to know:** the values of the entry are `tainted`, because the directory stores what people
typed. A name in `select` that is not an attribute name throws `LogicError`. Any server error other
than "no such entry" throws `Core\Ldap\LdapError`.
