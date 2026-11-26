Returns each recipient of a delivery report, in the order the report lists them.

A delivery report is the message a mail server sends back when it could not deliver a mail, or
when the delivery is late. `Core\Mail\Message::report()` returns the report, and `recipients()`
returns one `Core\Mail\ReportRecipient` for each address in it. Each recipient has its address,
what the server did, the status code and the server's own message.

One report can be about several addresses. Some of them may have failed while others are only
late, so check each recipient on its own.

**Good to know:** at most 1,000 recipients are read from one report. A recipient without an
address, or with an action that the standard does not define, is not in the list. Every text in
the list is `tainted`, because the server that sent the report wrote it.
