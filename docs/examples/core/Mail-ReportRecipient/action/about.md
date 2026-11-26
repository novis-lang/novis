Returns what the mail server did with the message for one recipient of a delivery report.

A delivery report is the standard part of a bounce that lists each recipient and what happened. A
bounce is a message from a mail server that says your mail could not be delivered. The `Action`
line of the report says what the server did for one recipient.

The result is a `Core\Mail\ReportRecipient::Action` case. `Failed` means the message was not
delivered. `Delayed` means the message is late and the server still tries. `Delivered` means the
server put the message in the mailbox. `Relayed` means the message was passed on to a server that
sends no report. `Expanded` means the message was sent on to the members of a mailing list.

One report may give a different action for each recipient. A program that handles bounces checks
the action of each recipient before it changes anything. Only `Failed` means the address did not
get the message.

**Good to know:** some recipients have no `Action` line, or have a word that the delivery report
standard (RFC 3464) does not define. Such a recipient is not in `recipients()`, so `action()` never
returns a value for it.
