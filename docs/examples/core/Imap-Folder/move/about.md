Moves one message to another folder of the same mailbox, such as `Archive`.

Pass the UID of the message (the number the server gives each message in a folder) and the name of
the folder. The message keeps its flags, so a read
message is still read in the new folder. In the new folder, the message gets a new UID, and its old
UID no longer works in the old folder. If you need the message again, search for it in the new
folder.

Only the message you name is moved. If the target folder does not exist, `move` throws an
`IOError`.

**Good to know:** the server must support moving one message by its UID. Every common mail server
does. On a server that does not, `move` throws an `IOError`, and no message is moved.

**The examples below** move a message to the archive, show its new UID, and move each bounce to its
own folder. Each example moves the messages back at the end, so it prints the same every time.
