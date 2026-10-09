Checks if a group is an application group. The result is `true` or `false`.

An application group belongs to Authorization Manager, a Windows tool that keeps the roles of one
application. Most directories have no application groups, so `appBasic` usually returns `false`.

`appBasic` reads the bit `0x10` of `groupType`. An application group whose members come from a query
has the bit `0x20` instead, and `appQuery` reads it.
