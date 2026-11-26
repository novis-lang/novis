What the mail server did with a message for one recipient of a delivery report.

A bounce is a message from a mail server that says your mail could not be delivered. A delivery
report is the standard part of a bounce that lists each recipient and what happened.

`Core\Mail\ReportRecipient::action()` returns one of the five cases. `Failed` means the message
could not be delivered. `Delayed` means the message is late, and the server still tries to deliver
it. `Delivered` means the server put the message in the mailbox. `Relayed` means the message was
passed to another server, which sends no report. So you do not learn whether the mail arrived.
`Expanded` means the message was sent on to the members of a mailing list.

**Good to know:** a message is a bounce when at least one recipient is `Failed`. It is a delay when
no recipient failed and at least one is `Delayed`.
