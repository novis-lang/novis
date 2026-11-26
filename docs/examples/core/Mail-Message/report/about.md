Returns the delivery report inside a bounce or a delay, as a `Core\Mail\Report`.

A mail server sends a delivery report when it cannot deliver a mail, or when the delivery is late.
The report lists each recipient with what the server did, the status code and the server's own
message. `recipients()` on the result returns that list.

The result is `null` for every message that is not a delivery report: a reply, an auto-reply, a spam
complaint or a normal mail. Use `?->` to read it, or check `kind()` first.

**Good to know:** a report that says every recipient got the mail still has a report. Its kind is
`Other`, because nothing failed and nothing is late. A report gives at most 1,000 recipients.

**The examples below** list the recipients of a bounce, show the messages that have no report, and
handle a report where one address failed and one is late.
