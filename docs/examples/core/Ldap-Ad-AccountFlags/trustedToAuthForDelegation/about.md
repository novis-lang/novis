Checks if a service on this account may act for a user without that user's password. The result is
`true` or `false`.

Delegation lets a service connect to another service in the name of a user. With this flag, the
service may do that even when the user did not log in with Kerberos, for example through a web
form. An administrator also chooses which services it may connect to.

`trustedToAuthForDelegation` reads the bit `0x1000000` of `userAccountControl`. A normal account
with this flag is `16777728`.
