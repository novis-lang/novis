Marks one message as not read. It removes the `\Seen` flag of the message.

After `markUnseen`, a mail program shows the message as new again, and `search({ unseen: true })`
finds it. A script uses this when it took a message but could not finish the work, so the next run
tries the message again. A person can also use it for a message they want to read again later. The
message then shows as new in their mail program.

Only `\Seen` is removed. Other flags of the message, such as `\Flagged`, do not change. If the
message is not read, `markUnseen` changes nothing. You name the message by its UID (the number the
server gives each message in a folder). If no message has that UID, `markUnseen` throws an
`IOError`.
