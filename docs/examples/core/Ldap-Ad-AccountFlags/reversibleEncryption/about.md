Checks if the password of an account is stored with an encryption that can be reversed. The result
is `true` or `false`.

Active Directory normally stores only a hash of a password, and a hash cannot be turned back into
the password. When this flag is set, the server stores the password in a form it can decrypt. Some
old programs need this. It is less safe, so most accounts do not have this flag.

`reversibleEncryption` reads the bit `0x80` of `userAccountControl`. A normal account with this flag
is `640`.
