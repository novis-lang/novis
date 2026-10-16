Checks if an account is a read-only domain controller. The result is `true` or `false`.

A read-only domain controller is a server with a copy of the directory that it cannot change. It
keeps the passwords of only some users. Companies use it in places that are less safe, such as a
small branch office.

`partialSecretsAccount` reads the bit `0x4000000` of `userAccountControl`. The account of a
read-only domain controller usually has the value `83890176`.
