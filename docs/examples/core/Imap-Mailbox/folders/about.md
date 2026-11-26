Returns the name of every folder in the mailbox, such as `INBOX`, `Sent` and `Archive`.

Pass a name from this list to `folder` to open that folder. The mail server writes the names, so
each one is a `tainted string`. You can print a name and pass it to `folder`. Check it before you
use it in a file path or a database query.

`INBOX` is the folder for new mail on every server. The names of the other folders differ from one
server to the next, so a script that needs `Archive` or `Junk` checks the list first.

**Good to know:** in a `directory` block, each subdirectory is one folder, and the names are in
alphabetical order.
