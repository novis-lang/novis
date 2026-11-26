Returns the `UIDVALIDITY` number of a folder. The number tells you if the UIDs you saved for this
folder are still correct.

Each message in a folder has a UID (the number the server gives each message in a folder). It stays
the same while the message is in the folder. A script that runs again later can save the highest UID it has read, and then read only
the newer messages. Save the `UIDVALIDITY` with it. Sometimes a server numbers the messages of a
folder again, for example after a restore. Then it gives the folder a new `UIDVALIDITY`, and a UID
you saved may now belong to another message.

**In plain words:** the UIDs are like seat numbers, and the `UIDVALIDITY` is the date of the event.
A seat number from another event is no help.

**Good to know:** a `directory` block is a folder on disk that works like a mailbox, for tests and
local work. In such a block, the number is always 1.
