Checks if a group is a security group. A security group can be given permissions, for example to a
folder or an application. The result is `true` or `false`.

A group where `security` returns `false` is a distribution group. Active Directory uses it only as a
mailing list, so a program that decides what a person may do should ignore it.

`security` reads the bit `0x80000000` of `groupType`. That bit is the highest one, so the number that
`bits` returns is negative when `security` returns `true`.
