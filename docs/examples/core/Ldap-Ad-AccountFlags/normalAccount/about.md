Checks if an account is an ordinary user account. The result is `true` or `false`.

Every account has one type. The account of a person is an ordinary user account. A computer, a
domain controller and a trust with another domain have their own types, and for them
`normalAccount` is `false`. Use it to keep only the accounts of people.

`normalAccount` reads the bit `0x200` of `userAccountControl`. An ordinary account is `512`.
