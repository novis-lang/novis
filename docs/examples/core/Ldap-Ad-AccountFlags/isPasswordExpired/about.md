Checks if the password of an account has expired. The result is `true` or `false`.

The domain sets how long a password is valid, for example six weeks. After that time, the user must
choose a new password before they can log in again. When `hasNonExpiringPassword` returns `true`,
the password does not expire this way.

Active Directory computes this flag. It does not store it in `userAccountControl`. A search that
selects `userAccountControl` also reads `msDS-User-Account-Control-Computed`, and
`isPasswordExpired` reads the bit `0x800000` from there. So `bits` never includes it, and `with`
cannot set or clear it.
