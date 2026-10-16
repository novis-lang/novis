Checks if an account is a domain controller. The result is `true` or `false`.

A domain controller is a server that runs Active Directory. Each domain controller has an account
of this type, and it uses the account to work with the other domain controllers. No other account
has this flag.

`serverTrustAccount` reads the bit `0x2000` of `userAccountControl`. An account with only this flag
is `8192`.
