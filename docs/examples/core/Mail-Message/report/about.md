Returns the delivery report inside a bounce or a delay, as a `Core\Mail\Report`.

A bounce is a message from a mail server that says your mail could not be delivered. A delivery
report is the standard part of a bounce that lists each recipient and what happened. A server also
sends one when the delivery is late. For each recipient, the report has what the server did, the
status code and the server's own message. `recipients()` on the result returns that list.

The result is `null` for every message that is not a delivery report: a reply, an auto-reply, a spam
complaint or a normal mail. Read it with `?->`, which calls a method only when the value is not
`null`.

**Good to know:** a report that says every recipient got the mail still has a report. Its `kind()`
is `Other`, because nothing failed and nothing is late. Only the first 1,000 recipients of a report
are read.

**The examples below** list the recipients of a bounce, show the messages that have no report, and
handle a report where one address failed and one is late.
