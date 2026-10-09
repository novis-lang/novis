Checks if the user must log in with a smart card. The result is `true` or `false`.

A smart card is a small card with a chip. The user puts it into a card reader to log in. When this
flag is set, the user cannot log in with a password. An administrator sets this flag on accounts
that need more protection.

`smartcardRequired` reads the bit `0x40000` of `userAccountControl`. A normal account with this
flag is `262656`.
