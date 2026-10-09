Checks if a service on this account may act for any user. The result is `true` or `false`.

Delegation lets a service connect to another service in the name of a user. When this flag is set,
a service that runs under this account may do that for every user who logs in to it. This is a
strong right, so few accounts have it.

`trustedForDelegation` reads the bit `0x80000` of `userAccountControl`. A normal account with this
flag is `524800`.
