Checks if a group is an application group whose members come from a query. The result is `true` or
`false`.

This kind of group belongs to Authorization Manager, a Windows tool that keeps the roles of one
application. The group has no list of members. The application runs a query each time it checks who
is in the group. Most directories have no such groups, so `appQuery` usually returns `false`.

`appQuery` reads the bit `0x20` of `groupType`. An application group with a list of members has the bit
`0x10` instead, and `appBasic` reads it.
