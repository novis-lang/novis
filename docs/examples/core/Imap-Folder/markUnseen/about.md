Marks one message as not read. It removes the `\Seen` flag of the message.

After `markUnseen`, a mail program shows the message as new again, and `search({ unseen: true })`
finds it. A script uses this when it took a message but could not finish the work, so the next run
tries the message again. A person can also use it to keep a message as a reminder.

Only `\Seen` is removed. Other flags of the message, such as `\Flagged`, stay as they are. If the
message is not read, `markUnseen` changes nothing. If no message has that UID, it throws an
`IOError`.
