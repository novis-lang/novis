Checks if an account may have an empty password. The result is `true` or `false`.

When this flag is set, Active Directory accepts an empty password for the account. Anybody who knows
the account name can then log in with it. This is a security risk, so it is good to check which
accounts have this flag.

`passwordNotRequired` reads the bit `0x20` of `userAccountControl`. A normal account with this flag
is `544`.
