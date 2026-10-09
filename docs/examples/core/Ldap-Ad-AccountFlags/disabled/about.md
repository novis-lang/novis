Checks if an account is disabled. The result is `true` or `false`.

Nobody can log in with a disabled account, even with the right password. An administrator disables
an account, for example when a person leaves the company. The account and its groups stay in the
directory, so it can be enabled again later.

`disabled` reads the bit `0x2` of `userAccountControl`. A normal account is `512`, and the same
account when it is disabled is `514`.
