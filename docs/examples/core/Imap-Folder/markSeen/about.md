Marks one message as read. It sets the `\Seen` flag of the message.

A mail program shows a read message in normal letters and a new one in bold. `search({ unseen:
true })` finds only the messages without `\Seen`. A script that handles each new message once
calls `markSeen` after it handles the message. On its next run, `search` no longer finds that
message.

`fetch` does not mark a message as read, so your program decides when a message counts as read.
If the message is read already, `markSeen` changes nothing. If no message has that UID, it throws
an `IOError`.

**Good to know:** call `markSeen` after the work is done. If the work fails first, the message is
still new, and the next run tries it again.
