Checks if an account is a trust with another domain. The result is `true` or `false`.

A trust lets the users of one domain use the servers and files of another domain. Active Directory
creates an account of this type for each trust, and the two domains use it to log in to each other.
No person logs in with it.

`interdomainTrustAccount` reads the bit `0x800` of `userAccountControl`. An account with only this
flag is `2048`.
