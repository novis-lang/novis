Checks if a group is a global group. The result is `true` or `false`.

A global group can only have members from its own domain. Other domains can still use it, for example
to give it permissions. A new group in Active Directory is global unless you choose another scope.

A group has exactly one scope: global, domain local or universal. `global` reads the bit `0x2` of
`groupType`.
