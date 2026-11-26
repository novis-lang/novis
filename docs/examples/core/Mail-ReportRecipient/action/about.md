Returns what the mail server did with the message for one recipient of a delivery report.

The result is a `Core\Mail\ReportRecipient::Action` case. `Failed` means the message was not
delivered. `Delayed` means the message is late and the server still tries. `Delivered`, `Relayed`
and `Expanded` mean the message went on: to the mailbox, to a server that sends no report, or to
a mailing list.

One report may give a different action for each recipient. A program that handles bounces checks
the action of each recipient before it changes anything. Only `Failed` means the address did not
get the message.

**Good to know:** a recipient whose `Action` line is missing, or is a word the standard does not
define, is not in the report's list.
