Returns the delivery report inside a bounce or a delay, as a `Core\Mail\Report`.

A bounce is a message from a mail server that says your mail could not be delivered. A delivery
report is the standard part of a bounce that lists each recipient and what happened. A server also
sends a delivery report when the delivery is late. The report lists each recipient with what the
server did, the status code and the server's own message. `recipients()` on the result returns that
list.

The result is `null` for every message that is not a delivery report: a reply, an auto-reply, a spam
complaint or a normal mail. Use `?->` to read it, or check `kind()` first.

**Good to know:** a report that says every recipient got the mail still has a report. `kind()`
returns the kind of message, and `Other` means it is not a bounce, a delay, a complaint, a reply or an
automatic reply. The kind of this report is `Other`, because nothing failed and nothing is late. Only the
first 1,000 recipients of a report are read. The rest are not read.

**The examples below** list the recipients of a bounce, show the messages that have no report, and
handle a report where one address failed and one is late.
