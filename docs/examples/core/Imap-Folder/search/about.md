Finds messages in a folder and returns their UIDs, the oldest first. A UID is the number the server
gives each message in a folder.

Set one or more options, and `search` returns the messages that match all of them. `unseen: true`
finds the messages that are not read yet. `since` and `before` take a `Core\Time\Date`. `since`
includes that day, and `before` does not. `from` and `subject` find the messages whose `From`
header or subject contains the text. Upper and lower case do not matter. `limit` returns at most that many
UIDs. With no option, `search` returns every message in the folder.

Pass each UID to `fetch` to read the message, or to `move`, `markSeen` or `delete` to change it.

**Good to know:** `search` does not read the messages and does not mark them as read. A
`directory` block is a folder on disk that works like a mailbox, for tests and local work. In such
a block, `since` and `before` use the `Date` header of each message.
