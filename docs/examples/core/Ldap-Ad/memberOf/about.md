Returns a filter that matches the members of a group. Use it with `search` on a connection, to
find the users in a group.

By default the filter matches only the direct members. With `{nested: true}`, it also matches the
members of a group that is itself in the group, at any depth. Only Active Directory understands the
nested filter.

**Good to know:** the group is a `Core\Ldap\Dn` or a string. A string cannot be `tainted`. When a
part of the DN comes from a user, build it with `Core\Ldap\Dn::of`.
