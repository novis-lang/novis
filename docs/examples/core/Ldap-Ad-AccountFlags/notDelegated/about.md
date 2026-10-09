Checks if no service may act for this user. The result is `true` or `false`.

Delegation lets a service connect to another service in the name of a user. When this flag is set,
no service may do that for this user, even a service that is trusted for delegation. Administrators
set this flag on accounts with many rights, such as their own accounts.

`notDelegated` reads the bit `0x100000` of `userAccountControl`. A normal account with this flag is
`1049088`.
