`isAutoSubmitted()` checks whether a message has the header `Auto-Submitted: auto-generated`. In a
test, `Core\Test::sentMail()` returns one `Core\Test\SentMail` for each message, and
`isAutoSubmitted()` is one of its methods.

The header says that a program sent the message, not a person. A mail server does not send an
out-of-office reply to such a message. So an automatic message does not start a loop of automatic
replies.

The result is the `autoSubmitted` option that your program gave to `Core\Mail::send`. The default
is `true`, because most mail that a web application sends is automatic: a receipt, a password reset
or an alert. Give `autoSubmitted: false` for a message that a person wrote, such as a reply from
a support agent.

Use `isAutoSubmitted()` to check that a message a person wrote is sent as a normal mail.

**The examples below** show an automatic order mail, a helpdesk reply that a person wrote, and a
request that sends both kinds.
