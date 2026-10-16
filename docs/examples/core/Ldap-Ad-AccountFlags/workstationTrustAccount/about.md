Checks if an account is a computer that is a member of the domain. The result is `true` or `false`.

When a Windows computer joins a domain, Active Directory creates an account for it. The computer
uses this account to log in to the domain. The name of the account usually ends with `$`.

`workstationTrustAccount` reads the bit `0x1000` of `userAccountControl`. An account with only this
flag is `4096`.
