Returns `true` when the delivery to this recipient failed permanently, and `false` when the problem may be temporary.

A bounce is a message from a mail server that says your mail could not be delivered. A delivery
report is the standard part of a bounce that lists each recipient and what happened. This method
reads the status code of one recipient in that report.

The result is `true` when the status code starts with `5`, such as `5.1.1` for an address that does
not exist. The server will not deliver to that address if you send the mail again. A code that
starts with `4` is temporary. `4.2.2` is an example: it means that the mailbox is full. For such a
code, the result is `false`. A recipient with no status code also gives `false`.

**In plain words:** a permanent failure means "stop sending to this address". A temporary one means
"try again later".

**Good to know:** use this to decide when to stop mailing an address. Many mail providers block a
sender that keeps sending to addresses that do not exist.
