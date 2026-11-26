Opens a mailbox and signs in. The server and the sign-in are set in an `[imap.<name>]` block of
`nvs.toml`, and your program passes only the name of the block.

The block for a mail server has `host`, `user`, and `password` or `password_file`. The connection
always uses TLS, so the password is never sent as plain text. The name must also be listed in
`[capabilities.imap] connect`. Otherwise `open` throws a `RuntimeError`.

A block can also read a folder on disk instead of a server: `directory = "mailbox"`. Each
subdirectory is a mail folder, and each `<uid>.eml` file in it is one message. The examples on this
page use such a block, so they run without a mail server.

**Good to know:** the connection closes when the request or the script ends. You do not close it
yourself.
