Deletes one message from a folder at once. The message cannot be brought back.

Pass the UID of the message (the number the server gives each message in a folder). Only that
message is deleted. Other mail programs can mark messages for deletion too. `delete` does not
remove those messages. If no message has that UID, `delete` throws an `IOError`.

To keep a copy, move the message to a folder such as `Trash` with `move` instead.

**Good to know:** the server must support deleting one message by its UID. Every common mail
server does. If it does not, `delete` throws an `IOError` before it changes anything.

**The examples below** first add the message they delete. They use a `directory` block (a folder
on disk that works like a mailbox, for tests and local work). In such a block, a message is a file,
so `Core\IO::write` adds one. Each example then prints the same every time it runs.
