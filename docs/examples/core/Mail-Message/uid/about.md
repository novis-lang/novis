Returns the UID of a message that was read from a mailbox folder, or `null` for a message that
`Core\Mail\Message::parse` read.

A UID is the number of a message in its mailbox folder. It does not change while the message stays
in that folder. `Core\Imap\Folder::search` returns the UIDs of a folder, and
`Core\Imap\Folder::fetch` reads the message with one UID. That message's `uid()` is the same number.

Pass the UID to the other methods of the same folder, such as `move`, `markSeen` or `delete`. A
UID belongs to one folder. When a message is moved, it gets a new UID in the other folder.

**The examples below** show the UID of each message in a folder, a parsed message with no UID, and
how to keep the UIDs of the bounces in a folder. A bounce is a message from a mail server that says
your mail could not be delivered.
