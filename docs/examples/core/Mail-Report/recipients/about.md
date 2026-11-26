Returns each recipient of a delivery report, in the order the report lists them.

A bounce is a message from a mail server that says your mail could not be delivered. A delivery
report is the standard part of a bounce that lists each recipient and what happened.
`Core\Mail\Message::report()` returns the report, and `recipients()` returns one
`Core\Mail\ReportRecipient` for each address in it.

One report can be about several addresses. Some of them may have failed while others are only
late, so check each recipient on its own.

**Good to know:** at most 1,000 recipients are read from one report. A recipient without a
`Final-Recipient` line (the address the server tried) is not in the list. The `Action` line
says what the server did. RFC 3464, the standard for delivery reports, lists its values. A
recipient with another value is not in the list either.

Every text in the list is `tainted`, because the server that sent the report wrote it. Tainted
means that the value comes from outside your program, so you must check or escape it before you
use it.
