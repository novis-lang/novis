Checks if Kerberos uses only DES keys for this account. The result is `true` or `false`.

Kerberos is the system that Windows uses to log users in. DES is an old type of encryption, and
today it is easy to break. When this flag is set, Kerberos uses only DES for this account, so the
account is not safe. New versions of Windows do not support DES at all.

`useDesKeyOnly` reads the bit `0x200000` of `userAccountControl`. A normal account with this flag
is `2097664`.
