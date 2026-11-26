Returns the name of every folder in the mailbox, such as `INBOX`, `Sent` and `Archive`.

Pass a name from this list to `folder` to open that folder. The mail server writes the names, so
each one is a `tainted string` (text from outside your program). You can print a name and pass it to
`folder`. A tainted string cannot be a plain `string`, so it does not compile as a file path. The
compiler error names the method that makes it safe, such as `Core\IO::within` for a path.

`INBOX` is the folder for new mail on every server. The names of the other folders differ from one
server to the next, so a script that needs `Archive` or `Junk` checks the list first. The names are
in the order the server sends them.

**Good to know:** a `directory` block is a folder on disk that works like a mailbox, for tests and
local work. In such a block, each subdirectory is one folder. The names are in alphabetical order,
with capital letters before small letters.
