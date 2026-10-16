Returns the user part of an address, which is the text between `//` and `@`. For
`ftp://ann@files.example.com/`, `userInfo()` returns `ann`.

Some addresses also have a password after a `:`, such as `ann:secret`. The user and the password
are returned as one text, and `userInfo` does not split them. Writing a password in an address is
not safe, because addresses are often saved in logs.

Escapes such as `%40` stay in the result. Use `Core\Uri::decodeComponent` to decode them.

The result is `null` when the address has no `@` before its host. It is `""` when nothing is written
before the `@`.

**The examples below** read the user part, decode an escaped user part, and check that a database
address in the settings has no password in it.
