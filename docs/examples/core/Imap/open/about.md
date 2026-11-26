Opens a mailbox and signs in. The server and the sign-in are set in an `[imap.<name>]` block of
`nvs.toml`, and your program passes only the name of the block.

The block for a mail server has `host`, `user`, and `password` or `password_file`. The connection
always uses TLS (the encryption that `https` uses), so the password is never sent as plain text.
The name must also be listed in `[capabilities.imap] connect`. Otherwise `open` throws a
`RuntimeError`.

A block can also be a `directory` block (a folder on disk that works like a mailbox, for tests and
local work). You write it as `directory = "mailbox"`. Each subdirectory is a mail folder. Each
`<uid>.eml` file in it is one message, and its UID (the number the server gives each message in a
folder) is the number in the file name. The examples on this page use such a block, so they run
without a mail server.

**Good to know:** the connection closes when the request or the script ends. You do not close it
yourself.
