Checks if an account is locked out. The result is `true` or `false`.

Active Directory locks an account after too many wrong passwords. Nobody can log in with it until
the lockout time ends or an administrator unlocks it.

Active Directory computes this flag. It does not store it in `userAccountControl`. A search that
selects `userAccountControl` also reads `msDS-User-Account-Control-Computed`, and `lockedOut` reads
the bit `0x10` from there. So `bits` never includes it, and `with` cannot set or clear it.
