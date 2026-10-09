Checks if an account needs a home directory. The result is `true` or `false`.

A home directory is a folder on a server where a user keeps their own files. When this flag is set,
the account must have a home directory. Most accounts do not have this flag.

`homeDirectoryRequired` reads the bit `0x8` of `userAccountControl`. A normal account with this flag
is `520`.
