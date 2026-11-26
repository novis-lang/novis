Returns what a received message is: a bounce, a delay, a complaint, an auto-reply, a reply or another message.

The result is a `Core\Mail\Message::Kind` case. The message is checked when it is parsed, and the
first rule that matches gives the kind. A delivery report with a failed recipient is a `Bounce`. A
report with only late recipients is a `Delay`. A spam report is a `Complaint`. A message with an
`Auto-Submitted` header, or a header that an out-of-office program writes, is an `AutoReply`. A
message with `In-Reply-To` is a `Reply`. Everything else is `Other`.

**Good to know:** only the standard report formats count. A message with the subject "Undelivered"
that is not a delivery report is `Other`. The kind never depends on the subject or the sender, so a
stranger cannot make a normal message look like a bounce by its words.
