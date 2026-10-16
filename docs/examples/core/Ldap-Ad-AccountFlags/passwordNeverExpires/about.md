Checks if the password of an account never expires. The result is `true` or `false`.

Active Directory normally makes a user change the password after some time. The domain sets this
time, for example six weeks. When this flag is set, the password of the account does not expire.
Administrators often set it for service accounts, which programs use to log in.

`passwordNeverExpires` reads the bit `0x10000` of `userAccountControl`. A normal account with this
flag is `66048`.
