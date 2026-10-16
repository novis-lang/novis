Checks if an account is a local account for a user from another domain. The result is `true` or
`false`.

Such an account is called a temporary duplicate account. The user's main account is in another
domain, and this account gives the user access to this domain only. Few directories have such
accounts. An ordinary user account has `normalAccount` instead.

`temporaryDuplicateAccount` reads the bit `0x100` of `userAccountControl`. An account with only this
flag is `256`.
