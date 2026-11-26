Reads one message of a folder and returns it as a `Core\Mail\Message`.

Pass a UID (the number the server gives each message in a folder) that `search` returned. The
message has the same methods as one that `Core\Mail\Message::parse` returns: `subject`, `from`,
`date`, `text`, `attachments`, `kind` and the rest. Its `uid()` is the UID you passed. Every part of
the message comes from its sender, so each text is a `tainted string`. A tainted string is text from
outside your program.

`fetch` does not mark the message as read. Call `markSeen` when your program is done with it.

**Good to know:** if no message has that UID, `fetch` throws an `IOError`. It also throws an
`IOError` when the message is larger than the memory the request has left. The body of such a
message is not read.
