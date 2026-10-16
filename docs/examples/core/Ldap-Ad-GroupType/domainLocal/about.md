Checks if a group is a domain local group. The result is `true` or `false`.

A domain local group can have members from any domain. It can only be given permissions in its own
domain. The groups under `CN=Builtin`, such as `Administrators`, are domain local.

A group has exactly one scope: global, domain local or universal. `domainLocal` reads the bit `0x4` of
`groupType`.
