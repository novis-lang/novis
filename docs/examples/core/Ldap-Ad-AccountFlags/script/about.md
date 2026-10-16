Checks if a logon script runs for an account. The result is `true` or `false`.

A logon script is a program that Windows runs when the user logs in. Active Directory sets this
flag on every account that has a logon script. Most accounts do not have one.

`script` reads the bit `0x1` of `userAccountControl`. A normal account with this flag is `513`.
