What the mail server did with a message for one recipient of a delivery report.

`Core\Mail\ReportRecipient::action()` returns one of the five cases. `Failed` means the message
could not be delivered. `Delayed` means the message is late, and the server still tries to deliver
it. `Delivered` means the message reached the mailbox. `Relayed` means the message was passed to a
server that sends no report. `Expanded` means the message went to a mailing list, which sends it on
to its members.

**Good to know:** a message is a bounce when at least one recipient is `Failed`. It is a delay when
no recipient failed and at least one is `Delayed`.
