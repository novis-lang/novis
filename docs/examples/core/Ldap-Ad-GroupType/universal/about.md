Checks if a group is a universal group. The result is `true` or `false`.

A universal group can have members from any domain, and it can be given permissions in any domain.
`Enterprise Admins` and `Schema Admins` are universal groups.

A group has exactly one scope: global, domain local or universal. `universal` reads the bit `0x8` of
`groupType`.
