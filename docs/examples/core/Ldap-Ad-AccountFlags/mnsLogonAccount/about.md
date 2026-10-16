Checks if an account is an MNS logon account. The result is `true` or `false`.

MNS is short for Majority Node Set, a type of Windows server cluster. A cluster of this type uses an
account with this flag to log in. Most directories have no such account.

`mnsLogonAccount` reads the bit `0x20000` of `userAccountControl`. A normal account with this flag
is `131584`.
