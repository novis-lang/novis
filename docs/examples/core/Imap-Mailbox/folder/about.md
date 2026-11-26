Opens one folder of the mailbox, such as `INBOX`, and returns it as a `Core\Imap\Folder`.

You search, read, move, flag and delete messages through the folder. Pass a name that `folders`
returns. `INBOX` exists on every server. If the folder does not exist, `folder` throws an
`IOError`.

You can open several folders of the same mailbox, and use them one after the other. Each message in
a folder has a UID (the number the server gives each message in a folder). A UID belongs to one
folder, so UID 1 in `INBOX` and UID 1 in `Sent` are two different messages.

**Good to know:** the name is sent to the server as text, so it cannot add a command. A `directory`
block is a folder on disk that works like a mailbox, for tests and local work. In such a block, a
name with `/`, `\`, `:` or `..` throws a `RuntimeError`.
