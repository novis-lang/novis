Checks if Active Directory created the group itself. The result is `true` or `false`.

The groups under `CN=Builtin`, such as `Administrators`, are created together with the domain, so
`isSystemGroup` returns `true` for them. A group that a person or a program created returns `false`.

`isSystemGroup` reads the bit `0x1` of `groupType`.
